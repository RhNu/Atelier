//! `reqwest` adapter for Danbooru discovery.

mod explore;
pub use explore::DanbooruExploreSource;

use async_trait::async_trait;
use atelier_danbooru::{
    DANBOORU_PAGE_SIZE, DanbooruClient, DanbooruCredentials, DanbooruError, DanbooruErrorKind,
    DanbooruMedia, DanbooruMediaVariant, DanbooruPost, DanbooruPostPage, DanbooruProfile,
    DanbooruRating, DanbooruResult, DanbooruSearchRequest,
};
use futures_timer::Delay;
use futures_util::StreamExt;
use reqwest::{StatusCode, Url, header};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex as AsyncMutex, Semaphore};

const DEFAULT_BASE_URL: &str = "https://danbooru.donmai.us/";
const API_RATE_PER_SECOND: f64 = 8.0;
const API_BURST: f64 = 2.0;
const SEARCH_CACHE_LIMIT: usize = 64;
const SEARCH_CACHE_TTL: Duration = Duration::from_mins(2);
const POST_CACHE_LIMIT: usize = 500;
const POST_CACHE_TTL: Duration = Duration::from_mins(30);
const MAX_MEDIA_BYTES: usize = 12 * 1024 * 1024;

pub struct ReqwestDanbooruClient {
    client: reqwest::Client,
    base_url: Url,
    api_gate: AsyncMutex<()>,
    rate: AsyncMutex<RateState>,
    cache: Mutex<CacheState>,
    media_slots: Semaphore,
}

impl ReqwestDanbooruClient {
    /// Creates the production Danbooru client.
    ///
    /// # Errors
    /// Returns an error if the fixed base URL or HTTP client cannot be built.
    pub fn new() -> DanbooruResult<Self> {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    /// Creates a client with an injectable base URL for focused adapter tests.
    ///
    /// # Errors
    /// Returns an error if the base URL or HTTP client cannot be built.
    pub fn with_base_url(base_url: &str) -> DanbooruResult<Self> {
        let base_url = Url::parse(base_url).map_err(|error| {
            DanbooruError::new(
                DanbooruErrorKind::InvalidRequest,
                format!("invalid Danbooru base URL: {error}"),
            )
        })?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| DanbooruError::unavailable(error.to_string()))?;
        Ok(Self {
            client,
            base_url,
            api_gate: AsyncMutex::new(()),
            rate: AsyncMutex::new(RateState::new()),
            cache: Mutex::new(CacheState::default()),
            media_slots: Semaphore::new(6),
        })
    }

    fn search_cache_key(
        provider_tags: &str,
        before_id: Option<u64>,
        credentials: Option<&DanbooruCredentials>,
    ) -> String {
        format!(
            "{}|{}|{}",
            credential_scope(credentials),
            before_id.map_or_else(|| "first".to_owned(), |value| value.to_string()),
            provider_tags
        )
    }

    fn cached_search(&self, key: &str) -> Option<DanbooruPostPage> {
        let mut cache = self.cache.lock().ok()?;
        cache.expire();
        cache.search.get(key).map(|entry| entry.value.clone())
    }

    fn cached_post(
        &self,
        post_id: u64,
        credentials: Option<&DanbooruCredentials>,
    ) -> Option<DanbooruPost> {
        let mut cache = self.cache.lock().ok()?;
        cache.expire();
        cache
            .posts
            .get(&(credential_scope(credentials), post_id))
            .map(|entry| entry.value.clone())
    }

    fn store_page(
        &self,
        key: String,
        page: DanbooruPostPage,
        credentials: Option<&DanbooruCredentials>,
    ) {
        let Ok(mut cache) = self.cache.lock() else {
            log::warn!("Danbooru cache is unavailable");
            return;
        };
        let now = Instant::now();
        for post in &page.posts {
            cache.insert_post(post.clone(), credential_scope(credentials), now);
        }
        cache.insert_search(key, page, now);
    }

    fn store_post(&self, post: DanbooruPost, credentials: Option<&DanbooruCredentials>) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert_post(post, credential_scope(credentials), Instant::now());
        }
    }

    async fn send_json<T>(
        &self,
        url: Url,
        credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<T>
    where
        T: DeserializeOwned,
    {
        let mut transient_attempt = 0_u8;
        let mut rate_attempt = 0_u8;
        loop {
            self.acquire_rate_token().await;
            let started = Instant::now();
            let mut request = self
                .client
                .get(url.clone())
                .header(header::USER_AGENT, user_agent(credentials));
            if let Some(credentials) = credentials {
                request = request.basic_auth(
                    &credentials.username,
                    Some(credentials.api_key.expose_secret()),
                );
            }
            let response = request
                .send()
                .await
                .map_err(|error| DanbooruError::unavailable(error.to_string()))?;
            let status = response.status();
            log::debug!(
                "Danbooru API request completed: status={} duration_ms={} authenticated={}",
                status.as_u16(),
                started.elapsed().as_millis(),
                credentials.is_some()
            );
            if status.is_success() {
                return response.json::<T>().await.map_err(|error| {
                    DanbooruError::new(DanbooruErrorKind::InvalidResponse, error.to_string())
                });
            }
            if status == StatusCode::TOO_MANY_REQUESTS && rate_attempt == 0 {
                rate_attempt += 1;
                let retry_after = retry_after(&response).unwrap_or(Duration::from_secs(1));
                Delay::new(retry_after).await;
                continue;
            }
            if matches!(
                status,
                StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE
            ) && transient_attempt < 2
            {
                let delay = Duration::from_millis(500 * (1_u64 << transient_attempt));
                transient_attempt += 1;
                Delay::new(delay).await;
                continue;
            }
            return Err(status_error(status, retry_after(&response)));
        }
    }

    async fn acquire_rate_token(&self) {
        loop {
            let wait_seconds = {
                let mut state = self.rate.lock().await;
                state.replenish();
                if state.tokens >= 1.0 {
                    state.tokens -= 1.0;
                    return;
                }
                (1.0 - state.tokens) / API_RATE_PER_SECOND
            };
            Delay::new(Duration::from_secs_f64(wait_seconds)).await;
        }
    }

    async fn download_media(
        &self,
        url: Url,
        credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruMedia> {
        validate_media_url(&url, &self.base_url)?;
        let _permit = self
            .media_slots
            .acquire()
            .await
            .map_err(|_| DanbooruError::unavailable("Danbooru media queue is unavailable"))?;
        let response = self
            .client
            .get(url)
            .header(header::USER_AGENT, user_agent(credentials))
            .send()
            .await
            .map_err(|error| DanbooruError::unavailable(error.to_string()))?;
        if !response.status().is_success() {
            return Err(status_error(response.status(), retry_after(&response)));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_MEDIA_BYTES as u64)
        {
            return Err(DanbooruError::new(
                DanbooruErrorKind::MediaRejected,
                "Danbooru media exceeds the 12 MiB limit",
            ));
        }
        let mime_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .unwrap_or_default()
            .to_owned();
        if !mime_type.starts_with("image/") {
            return Err(DanbooruError::new(
                DanbooruErrorKind::MediaRejected,
                "Danbooru media is not an image",
            ));
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| DanbooruError::unavailable(error.to_string()))?;
            if bytes.len().saturating_add(chunk.len()) > MAX_MEDIA_BYTES {
                return Err(DanbooruError::new(
                    DanbooruErrorKind::MediaRejected,
                    "Danbooru media exceeds the 12 MiB limit",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(DanbooruMedia { mime_type, bytes })
    }
}

#[async_trait]
impl DanbooruClient for ReqwestDanbooruClient {
    async fn search(
        &self,
        request: DanbooruSearchRequest,
        credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruPostPage> {
        let provider_tags = request.provider_tags()?;
        let key = Self::search_cache_key(&provider_tags, request.before_id, credentials);
        if let Some(page) = self.cached_search(&key) {
            return Ok(page);
        }
        let _gate = self.api_gate.lock().await;
        if let Some(page) = self.cached_search(&key) {
            return Ok(page);
        }
        let mut url = self
            .base_url
            .join("posts.json")
            .map_err(|error| DanbooruError::unavailable(error.to_string()))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("tags", &provider_tags);
            query.append_pair("limit", &DANBOORU_PAGE_SIZE.to_string());
            if let Some(before_id) = request.before_id {
                query.append_pair("page", &format!("b{before_id}"));
            }
        }
        let raw = self.send_json::<Vec<RawPost>>(url, credentials).await?;
        let posts: Vec<DanbooruPost> = raw
            .into_iter()
            .map(TryInto::try_into)
            .collect::<DanbooruResult<Vec<_>>>()?;
        let next_before_id = (posts.len() == DANBOORU_PAGE_SIZE)
            .then(|| posts.last().map(|post| post.id))
            .flatten();
        let page = DanbooruPostPage {
            posts,
            next_before_id,
            authenticated: credentials.is_some(),
        };
        self.store_page(key, page.clone(), credentials);
        Ok(page)
    }

    async fn post(
        &self,
        post_id: u64,
        credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruPost> {
        if let Some(post) = self.cached_post(post_id, credentials) {
            return Ok(post);
        }
        let _gate = self.api_gate.lock().await;
        if let Some(post) = self.cached_post(post_id, credentials) {
            return Ok(post);
        }
        let url = self
            .base_url
            .join(&format!("posts/{post_id}.json"))
            .map_err(|error| DanbooruError::unavailable(error.to_string()))?;
        let post: DanbooruPost = self
            .send_json::<RawPost>(url, credentials)
            .await?
            .try_into()?;
        self.store_post(post.clone(), credentials);
        Ok(post)
    }

    async fn media(
        &self,
        post_id: u64,
        variant: DanbooruMediaVariant,
        credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruMedia> {
        let post = self.post(post_id, credentials).await?;
        let url = post.media_url(variant).ok_or_else(|| {
            DanbooruError::new(
                DanbooruErrorKind::NotFound,
                "Danbooru post does not have the requested media variant",
            )
        })?;
        let url = Url::parse(url).map_err(|error| {
            DanbooruError::new(DanbooruErrorKind::InvalidResponse, error.to_string())
        })?;
        self.download_media(url, credentials).await
    }

    async fn profile(&self, credentials: &DanbooruCredentials) -> DanbooruResult<DanbooruProfile> {
        let _gate = self.api_gate.lock().await;
        let url = self
            .base_url
            .join("profile.json")
            .map_err(|error| DanbooruError::unavailable(error.to_string()))?;
        let profile = self.send_json::<RawProfile>(url, Some(credentials)).await?;
        Ok(DanbooruProfile {
            username: profile.name,
            level: profile.level_string,
        })
    }
}

// Private cache namespace: rotated keys and anonymous access cannot share post data.
fn credential_scope(credentials: Option<&DanbooruCredentials>) -> String {
    credentials.map_or_else(
        || "anonymous".to_owned(),
        |credentials| {
            let mut hash = Sha256::new();
            hash.update(credentials.username.as_bytes());
            hash.update([0]);
            hash.update(credentials.api_key.expose_secret().as_bytes());
            base64::Engine::encode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                hash.finalize(),
            )
        },
    )
}

#[derive(Debug)]
struct RateState {
    tokens: f64,
    last: Instant,
}

impl RateState {
    fn new() -> Self {
        Self {
            tokens: API_BURST,
            last: Instant::now(),
        }
    }

    fn replenish(&mut self) {
        let now = Instant::now();
        self.tokens = now
            .duration_since(self.last)
            .as_secs_f64()
            .mul_add(API_RATE_PER_SECOND, self.tokens)
            .min(API_BURST);
        self.last = now;
    }
}

#[derive(Default)]
struct CacheState {
    search: HashMap<String, CacheEntry<DanbooruPostPage>>,
    search_order: VecDeque<String>,
    posts: HashMap<(String, u64), CacheEntry<DanbooruPost>>,
    post_order: VecDeque<(String, u64)>,
}

struct CacheEntry<T> {
    stored_at: Instant,
    value: T,
}

impl CacheState {
    fn expire(&mut self) {
        self.search
            .retain(|_, entry| entry.stored_at.elapsed() <= SEARCH_CACHE_TTL);
        self.search_order
            .retain(|key| self.search.contains_key(key));
        self.posts
            .retain(|_, entry| entry.stored_at.elapsed() <= POST_CACHE_TTL);
        self.post_order.retain(|id| self.posts.contains_key(id));
    }

    fn insert_search(&mut self, key: String, value: DanbooruPostPage, now: Instant) {
        self.search_order.retain(|current| current != &key);
        self.search_order.push_back(key.clone());
        self.search.insert(
            key,
            CacheEntry {
                stored_at: now,
                value,
            },
        );
        while self.search.len() > SEARCH_CACHE_LIMIT {
            if let Some(oldest) = self.search_order.pop_front() {
                self.search.remove(&oldest);
            }
        }
    }

    fn insert_post(&mut self, value: DanbooruPost, scope: String, now: Instant) {
        let id = (scope, value.id);
        self.post_order.retain(|current| *current != id);
        self.post_order.push_back(id.clone());
        self.posts.insert(
            id,
            CacheEntry {
                stored_at: now,
                value,
            },
        );
        while self.posts.len() > POST_CACHE_LIMIT {
            if let Some(oldest) = self.post_order.pop_front() {
                self.posts.remove(&oldest);
            }
        }
    }
}

#[derive(Deserialize)]
struct RawPost {
    id: u64,
    created_at: String,
    rating: String,
    image_width: u32,
    image_height: u32,
    score: i64,
    fav_count: u64,
    file_ext: String,
    file_size: u64,
    source: String,
    preview_file_url: Option<String>,
    large_file_url: Option<String>,
    tag_string_artist: String,
    tag_string_copyright: String,
    tag_string_character: String,
    tag_string_general: String,
    tag_string_meta: String,
}

impl TryFrom<RawPost> for DanbooruPost {
    type Error = DanbooruError;

    fn try_from(value: RawPost) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            created_at: value.created_at,
            rating: parse_rating(&value.rating)?,
            width: value.image_width,
            height: value.image_height,
            score: value.score,
            favorite_count: value.fav_count,
            file_extension: value.file_ext,
            file_size: value.file_size,
            source_url: nonempty(value.source),
            preview_url: value.preview_file_url,
            sample_url: value.large_file_url,
            artist_tags: tags(&value.tag_string_artist),
            copyright_tags: tags(&value.tag_string_copyright),
            character_tags: tags(&value.tag_string_character),
            general_tags: tags(&value.tag_string_general),
            meta_tags: tags(&value.tag_string_meta),
        })
    }
}

#[derive(Deserialize)]
struct RawProfile {
    name: String,
    #[serde(default)]
    level_string: Option<String>,
}

fn parse_rating(value: &str) -> DanbooruResult<DanbooruRating> {
    match value {
        "g" => Ok(DanbooruRating::General),
        "s" => Ok(DanbooruRating::Sensitive),
        "q" => Ok(DanbooruRating::Questionable),
        "e" => Ok(DanbooruRating::Explicit),
        other => Err(DanbooruError::new(
            DanbooruErrorKind::InvalidResponse,
            format!("unknown Danbooru rating `{other}`"),
        )),
    }
}

fn tags(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_owned).collect()
}

fn nonempty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn user_agent(credentials: Option<&DanbooruCredentials>) -> String {
    credentials.map_or_else(
        || {
            format!(
                "Atelier/{} (https://github.com; anonymous)",
                env!("CARGO_PKG_VERSION")
            )
        },
        |credentials| {
            format!(
                "Atelier/{} (https://github.com; by {} on Danbooru)",
                env!("CARGO_PKG_VERSION"),
                credentials.username
            )
        },
    )
}

fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    response
        .headers()
        .get(header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

fn status_error(status: StatusCode, retry_after: Option<Duration>) -> DanbooruError {
    let kind = match status {
        StatusCode::UNAUTHORIZED => DanbooruErrorKind::Unauthorized,
        StatusCode::FORBIDDEN => DanbooruErrorKind::Forbidden,
        StatusCode::NOT_FOUND => DanbooruErrorKind::NotFound,
        StatusCode::TOO_MANY_REQUESTS => DanbooruErrorKind::RateLimited,
        status if status.is_server_error() => DanbooruErrorKind::Unavailable,
        _ => DanbooruErrorKind::InvalidRequest,
    };
    DanbooruError::new(kind, format!("Danbooru request failed with HTTP {status}"))
        .with_retry_after(retry_after.map(|duration| duration.as_secs()))
}

fn validate_media_url(url: &Url, base_url: &Url) -> DanbooruResult<()> {
    if url.scheme() != "https" && url.scheme() != base_url.scheme() {
        return Err(DanbooruError::new(
            DanbooruErrorKind::MediaRejected,
            "Danbooru media URL must use HTTPS",
        ));
    }
    let production = base_url.as_str() == DEFAULT_BASE_URL;
    if production && url.host_str() != Some("cdn.donmai.us") {
        return Err(DanbooruError::new(
            DanbooruErrorKind::MediaRejected,
            "Danbooru media URL host is not allowed",
        ));
    }
    if !production && url.host_str() != base_url.host_str() {
        return Err(DanbooruError::new(
            DanbooruErrorKind::MediaRejected,
            "Danbooru media URL host is not allowed",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
