//! Atelier-only client for the undocumented public `NovelAI` Explore gallery.
//! Deliberately has no dependency on credentials or novelai-bridge.

mod metadata;
mod wire;

use async_trait::async_trait;
use atelier_explore::{
    ExploreCursor, ExploreError, ExploreErrorKind, ExploreMedia, ExploreMediaVariant, ExplorePage,
    ExploreResult, ExploreSource,
    novelai::{NovelAiExplorePost, NovelAiExploreQuery, validate_post_id},
};
use futures_timer::Delay;
use futures_util::StreamExt;
use reqwest::{Client, Response, StatusCode, Url, header};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::Cursor,
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, Semaphore};
use wire::{PAGE_SIZE, RawPage, decode_post, search_body};

const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;
const MAX_MEDIA_BYTES: usize = 12 * 1024 * 1024;
struct CachedPost {
    stored_at: Instant,
    post: NovelAiExplorePost,
    complete: bool,
}

const CACHE_TTL: Duration = Duration::from_secs(120);

pub struct NovelAiExploreClient {
    client: Client,
    base: Url,
    request_gate: AsyncMutex<Instant>,
    cooldown: Mutex<Option<Instant>>,
    media_slots: Semaphore,
    posts: Mutex<HashMap<String, CachedPost>>,
}

impl NovelAiExploreClient {
    /// Constructs an anonymous client for the fixed public host.
    ///
    /// # Errors
    /// Returns an error when HTTP client construction fails.
    pub fn new() -> ExploreResult<Self> {
        Self::build("https://explore.novelai.net/")
    }

    fn build(base: &str) -> ExploreResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!(
                "Atelier/",
                env!("CARGO_PKG_VERSION"),
                " (public Explore reader)"
            ))
            .build()
            .map_err(|_| unavailable("could not initialize Explore HTTP client"))?;
        Ok(Self {
            client,
            base: Url::parse(base).map_err(|_| unavailable("invalid Explore host"))?,
            request_gate: AsyncMutex::new(Instant::now()),
            cooldown: Mutex::new(None),
            media_slots: Semaphore::new(4),
            posts: Mutex::new(HashMap::new()),
        })
    }

    async fn send(&self, path: &str, body: Option<Value>) -> ExploreResult<Response> {
        {
            let mut next = self.request_gate.lock().await;
            if let Some(wait) = next.checked_duration_since(Instant::now()) {
                Delay::new(wait).await;
            }
            let cooldown = *self
                .cooldown
                .lock()
                .map_err(|_| unavailable("Explore rate state unavailable"))?;
            if let Some(until) = cooldown
                && let Some(wait) = until.checked_duration_since(Instant::now())
            {
                let mut error = ExploreError::new(
                    ExploreErrorKind::RateLimited,
                    "NovelAI Explore is cooling down",
                );
                error.retry_after_seconds = Some(wait.as_secs().saturating_add(1));
                return Err(error);
            }
            *next = Instant::now() + Duration::from_millis(350);
        }
        let url = self
            .base
            .join(path)
            .map_err(|_| ExploreError::invalid("invalid Explore endpoint"))?;
        let request = body.map_or_else(
            || self.client.get(url.clone()),
            |body| self.client.post(url.clone()).json(&body),
        );
        let response = request
            .send()
            .await
            .map_err(|_| unavailable("NovelAI Explore request failed or timed out"))?;
        if !response.status().is_success() {
            let mut error = status_error(response.status());
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                let seconds = response
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5);
                error.retry_after_seconds = Some(seconds);
                *self
                    .cooldown
                    .lock()
                    .map_err(|_| unavailable("Explore rate state unavailable"))? =
                    Instant::now().checked_add(Duration::from_secs(seconds));
            }
            return Err(error);
        }
        Ok(response)
    }

    async fn json(&self, path: &str, body: Option<Value>) -> ExploreResult<Value> {
        let response = self.send(path, body).await?;
        let bytes = bounded_bytes(response, MAX_JSON_BYTES).await?;
        serde_json::from_slice(&bytes).map_err(|_| {
            ExploreError::new(
                ExploreErrorKind::InvalidResponse,
                "NovelAI Explore returned invalid JSON",
            )
        })
    }

    fn cache_post(&self, post: &NovelAiExplorePost, complete: bool) {
        if let Ok(mut posts) = self.posts.lock() {
            posts.retain(|_, entry| entry.stored_at.elapsed() < CACHE_TTL);
            if !complete && posts.get(&post.id).is_some_and(|entry| entry.complete) {
                return;
            }
            if posts.len() >= 128
                && let Some(oldest) = posts
                    .iter()
                    .min_by_key(|(_, entry)| entry.stored_at)
                    .map(|(id, _)| id.clone())
            {
                posts.remove(&oldest);
            }
            posts.insert(
                post.id.clone(),
                CachedPost {
                    stored_at: Instant::now(),
                    post: post.clone(),
                    complete,
                },
            );
        }
    }
}

#[async_trait]
impl ExploreSource for NovelAiExploreClient {
    type Query = NovelAiExploreQuery;
    type Post = NovelAiExplorePost;

    async fn search(
        &self,
        query: Self::Query,
        cursor: Option<ExploreCursor>,
    ) -> ExploreResult<ExplorePage<Self::Post>> {
        query.validate()?;
        let offset = match cursor {
            None => 0,
            Some(ExploreCursor::Offset(offset)) if offset <= 1_000_000 => offset,
            _ => return Err(ExploreError::invalid("invalid Explore offset cursor")),
        };
        let value = self
            .json("post/search", Some(search_body(&query, offset)))
            .await?;
        let page: RawPage = serde_json::from_value(value).map_err(|_| {
            ExploreError::new(
                ExploreErrorKind::InvalidResponse,
                "NovelAI Explore search shape changed",
            )
        })?;
        if page.pagination.offset != offset
            || page.pagination.limit != PAGE_SIZE
            || page.results.len() as u64 > PAGE_SIZE
        {
            return Err(ExploreError::new(
                ExploreErrorKind::InvalidResponse,
                "unexpected Explore pagination",
            ));
        }
        let received = page.results.len() as u64;
        let mut items = Vec::new();
        for value in page.results {
            match decode_post(value) {
                Ok(post) => {
                    self.cache_post(&post, false);
                    items.push(post);
                }
                Err(error) if error.kind == ExploreErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        // Advance by upstream rows, including filtered rows, rather than visible cards.
        let next = offset.saturating_add(received);
        Ok(ExplorePage {
            total: (received == items.len() as u64).then_some(page.pagination.total),
            items,
            next_cursor: (received > 0 && next < page.pagination.total)
                .then_some(ExploreCursor::Offset(next)),
            authenticated: false,
        })
    }

    async fn detail(&self, item_id: &str) -> ExploreResult<Self::Post> {
        validate_post_id(item_id)?;
        let cached = self
            .posts
            .lock()
            .map_err(|_| unavailable("Explore cache unavailable"))?
            .get(item_id)
            .filter(|entry| entry.complete && entry.stored_at.elapsed() < CACHE_TTL)
            .map(|entry| entry.post.clone());
        if let Some(post) = cached {
            return Ok(post);
        }
        let post = decode_post(self.json(&format!("post/{item_id}"), None).await?)?;
        if post.id != item_id {
            return Err(ExploreError::new(
                ExploreErrorKind::InvalidResponse,
                "Explore returned another post",
            ));
        }
        self.cache_post(&post, true);
        Ok(post)
    }

    async fn media(
        &self,
        item_id: &str,
        variant: ExploreMediaVariant,
    ) -> ExploreResult<ExploreMedia> {
        validate_post_id(item_id)?;
        let _slot = self
            .media_slots
            .acquire()
            .await
            .map_err(|_| unavailable("Explore media queue unavailable"))?;
        // Check public visibility before serving an ID supplied through IPC.
        let approved = self
            .posts
            .lock()
            .map_err(|_| unavailable("Explore cache unavailable"))?
            .get(item_id)
            .is_some_and(|entry| entry.stored_at.elapsed() < CACHE_TTL);
        if !approved {
            self.detail(item_id).await?;
        }
        let endpoint = match variant {
            ExploreMediaVariant::Thumbnail => "thumbnail",
            ExploreMediaVariant::Preview => "blob",
        };
        let response = self
            .send(&format!("post/{endpoint}/{item_id}"), None)
            .await?;
        let mime_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .unwrap_or_default()
            .trim()
            .to_owned();
        if !matches!(
            mime_type.as_str(),
            "image/webp" | "image/png" | "image/jpeg"
        ) {
            return Err(ExploreError::new(
                ExploreErrorKind::MediaRejected,
                "unsupported Explore media type",
            ));
        }
        let bytes = bounded_bytes(response, MAX_MEDIA_BYTES).await?;
        let reader = image::ImageReader::new(Cursor::new(&bytes))
            .with_guessed_format()
            .map_err(|_| media_invalid())?;
        let format_mime = reader.format().map(|format| format.to_mime_type());
        if format_mime != Some(mime_type.as_str()) {
            return Err(media_invalid());
        }
        let (width, height) = reader.into_dimensions().map_err(|_| media_invalid())?;
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 40_000_000 {
            return Err(media_invalid());
        }
        Ok(ExploreMedia { mime_type, bytes })
    }
}

async fn bounded_bytes(response: Response, limit: usize) -> ExploreResult<Vec<u8>> {
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(ExploreError::new(
            ExploreErrorKind::MediaRejected,
            "Explore response exceeds size limit",
        ));
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| unavailable("Explore response interrupted"))?;
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(ExploreError::new(
                ExploreErrorKind::MediaRejected,
                "Explore response exceeds size limit",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn unavailable(message: &str) -> ExploreError {
    ExploreError::new(ExploreErrorKind::Unavailable, message)
}
fn media_invalid() -> ExploreError {
    ExploreError::new(
        ExploreErrorKind::MediaRejected,
        "invalid or oversized Explore image",
    )
}

fn status_error(status: StatusCode) -> ExploreError {
    let kind = match status {
        StatusCode::UNAUTHORIZED => ExploreErrorKind::Unauthorized,
        StatusCode::FORBIDDEN => ExploreErrorKind::Forbidden,
        StatusCode::NOT_FOUND => ExploreErrorKind::NotFound,
        StatusCode::TOO_MANY_REQUESTS => ExploreErrorKind::RateLimited,
        status if status.is_server_error() => ExploreErrorKind::Unavailable,
        _ => ExploreErrorKind::InvalidResponse,
    };
    ExploreError::new(kind, format!("NovelAI Explore returned HTTP {status}"))
}

#[cfg(test)]
mod tests;
