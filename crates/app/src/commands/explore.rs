mod cursor;
mod mapping;

use atelier_adapter_danbooru::DanbooruExploreSource;
use atelier_app_api::{
    error::ErrorEnvelopeDto,
    explore::{
        ExploreItemRefDto, ExploreMediaRequestDto, ExploreMediaVariantDto, ExplorePageDto,
        ExplorePostDetailDto, ExplorePostSummaryDto, ExploreQueryDto, ExploreSearchRequestDto,
        ExploreSourceDescriptorDto, ExploreSourceIdDto,
    },
    resource::ResourceImageDto,
};
use atelier_explore::{
    DanbooruExploreQuery, ExploreError, ExploreErrorKind, ExploreMediaVariant, ExploreSource,
    novelai::{NovelAiExplorePost, NovelAiExploreQuery},
};
use atelier_secrets::SecretStore;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::sync::atomic::Ordering;

use super::{
    AtelierRuntime, CommandResult,
    danbooru::{post_summary_to_dto, rating_to_domain},
};

pub type NovelAiExploreSource =
    dyn ExploreSource<Query = NovelAiExploreQuery, Post = NovelAiExplorePost>;

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: Send + Sync,
    E: Send + Sync,
{
    #[must_use]
    pub fn list_explore_sources(&self) -> Vec<ExploreSourceDescriptorDto> {
        vec![
            ExploreSourceDescriptorDto {
                id: ExploreSourceIdDto::DanbooruDatabase,
                name: "Danbooru Database".into(),
                experimental: false,
                supports_account: true,
                available: true,
            },
            ExploreSourceDescriptorDto {
                id: ExploreSourceIdDto::NovelaiExploreGallery,
                name: "NovelAI Explore Gallery".into(),
                experimental: true,
                supports_account: false,
                available: self.novelai_explore.is_some(),
            },
        ]
    }

    /// Searches a source using a query-bound continuation token.
    ///
    /// # Errors
    /// Returns validation, account, or source errors without requiring a workspace.
    pub async fn search_explore_posts(
        &self,
        request: ExploreSearchRequestDto,
    ) -> CommandResult<ExplorePageDto> {
        let source_id = match &request.query {
            ExploreQueryDto::DanbooruDatabase(_) => ExploreSourceIdDto::DanbooruDatabase,
            ExploreQueryDto::NovelaiExploreGallery(_) => ExploreSourceIdDto::NovelaiExploreGallery,
        };
        let revision = if source_id == ExploreSourceIdDto::DanbooruDatabase {
            self.explore_identity_revision.load(Ordering::SeqCst)
        } else {
            0
        };
        let fingerprint = cursor::fingerprint(&request.query, revision)?;
        let cursor = cursor::decode(request.cursor.as_deref(), &fingerprint)
            .map_err(|e| envelope(source_id, e))?;
        let (items, next, total, authenticated) = match request.query {
            ExploreQueryDto::DanbooruDatabase(query) => {
                let source = self.danbooru_explore_source().await?;
                let page = source
                    .search(
                        DanbooruExploreQuery {
                            query: query.query,
                            ratings: query.ratings.into_iter().map(rating_to_domain).collect(),
                        },
                        cursor,
                    )
                    .await
                    .map_err(|e| envelope(source_id, e))?;
                (
                    page.items
                        .iter()
                        .map(|post| {
                            ExplorePostSummaryDto::DanbooruDatabase(post_summary_to_dto(post))
                        })
                        .collect(),
                    page.next_cursor,
                    page.total,
                    page.authenticated,
                )
            }
            ExploreQueryDto::NovelaiExploreGallery(query) => {
                let page = self
                    .novelai_explore_source()?
                    .search(mapping::query(query), cursor)
                    .await
                    .map_err(|e| envelope(source_id, e))?;
                (
                    page.items
                        .iter()
                        .map(|post| {
                            ExplorePostSummaryDto::NovelaiExploreGallery(mapping::summary(post))
                        })
                        .collect(),
                    page.next_cursor,
                    page.total,
                    page.authenticated,
                )
            }
        };
        Ok(ExplorePageDto {
            items,
            next_cursor: next.map(|next| cursor::encode(next, &fingerprint)),
            total,
            authenticated,
        })
    }

    /// Loads a detail payload without erasing source-specific information.
    ///
    /// # Errors
    /// Returns account, invalid ID, missing post, or source errors.
    pub async fn get_explore_post_detail(
        &self,
        item: ExploreItemRefDto,
    ) -> CommandResult<ExplorePostDetailDto> {
        match item.source_id {
            ExploreSourceIdDto::DanbooruDatabase => {
                let post = self
                    .danbooru_explore_source()
                    .await?
                    .detail(&item.item_id)
                    .await
                    .map_err(|e| envelope(item.source_id, e))?;
                Ok(ExplorePostDetailDto::DanbooruDatabase(
                    self.enrich_danbooru_post(&post),
                ))
            }
            ExploreSourceIdDto::NovelaiExploreGallery => {
                let post = self
                    .novelai_explore_source()?
                    .detail(&item.item_id)
                    .await
                    .map_err(|e| envelope(item.source_id, e))?;
                Ok(ExplorePostDetailDto::NovelaiExploreGallery(
                    mapping::detail(post),
                ))
            }
        }
    }

    /// Reads a bounded, source-validated media variant; never accepts a URL.
    ///
    /// # Errors
    /// Returns account, source, or media validation errors.
    pub async fn get_explore_media(
        &self,
        request: ExploreMediaRequestDto,
    ) -> CommandResult<ResourceImageDto> {
        let variant = match request.variant {
            ExploreMediaVariantDto::Thumbnail => ExploreMediaVariant::Thumbnail,
            ExploreMediaVariantDto::Preview => ExploreMediaVariant::Preview,
        };
        let source_id = request.item.source_id;
        let media = match source_id {
            ExploreSourceIdDto::DanbooruDatabase => {
                self.danbooru_explore_source()
                    .await?
                    .media(&request.item.item_id, variant)
                    .await
            }
            ExploreSourceIdDto::NovelaiExploreGallery => {
                self.novelai_explore_source()?
                    .media(&request.item.item_id, variant)
                    .await
            }
        }
        .map_err(|e| envelope(source_id, e))?;
        Ok(ResourceImageDto {
            image_base64: STANDARD.encode(media.bytes),
            mime_type: Some(media.mime_type),
        })
    }

    async fn danbooru_explore_source(&self) -> CommandResult<DanbooruExploreSource> {
        Ok(DanbooruExploreSource::new(
            self.danbooru.clone(),
            self.danbooru_credentials().await?,
        ))
    }

    fn novelai_explore_source(&self) -> CommandResult<&NovelAiExploreSource> {
        self.novelai_explore.as_deref().ok_or_else(|| {
            envelope(
                ExploreSourceIdDto::NovelaiExploreGallery,
                ExploreError::new(
                    ExploreErrorKind::Unavailable,
                    "NovelAI Explore is unavailable",
                ),
            )
        })
    }
}

fn envelope(source: ExploreSourceIdDto, error: ExploreError) -> ErrorEnvelopeDto {
    let code = match error.kind {
        ExploreErrorKind::InvalidRequest => "explore_invalid_request",
        ExploreErrorKind::Unauthorized => "explore_unauthorized",
        ExploreErrorKind::Forbidden => "explore_forbidden",
        ExploreErrorKind::NotFound => "explore_not_found",
        ExploreErrorKind::RateLimited => "explore_rate_limited",
        ExploreErrorKind::Unavailable => "explore_unavailable",
        ExploreErrorKind::InvalidResponse => "explore_invalid_response",
        ExploreErrorKind::MediaRejected => "explore_media_rejected",
    };
    ErrorEnvelopeDto::new(code, error.message).with_details(
        serde_json::json!({"source_id":source,"retry_after_seconds":error.retry_after_seconds}),
    )
}
