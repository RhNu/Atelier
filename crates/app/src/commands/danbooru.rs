use std::collections::HashMap;

use atelier_app_api::danbooru::{
    DanbooruAccountDto, DanbooruAccountStateDto, DanbooruAuthModeDto, DanbooruMediaRequestDto,
    DanbooruMediaVariantDto, DanbooruPostDetailDto, DanbooruPostDetailRequestDto,
    DanbooruPostPageDto, DanbooruPostSummaryDto, DanbooruRatingDto, DanbooruSearchRequestDto,
    DanbooruTagCategoryDto, DanbooruTagDto, SaveDanbooruAccountRequestDto,
};
use atelier_app_api::error::ErrorEnvelopeDto;
use atelier_app_api::resource::ResourceImageDto;
use atelier_danbooru::{
    DanbooruCredentials, DanbooruErrorKind, DanbooruMediaVariant, DanbooruPost, DanbooruRating,
    DanbooruSearchRequest, DanbooruTagCategory,
};
use atelier_secrets::{SecretRecordId, SecretStore, SecretValue, SecretsErrorKind};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::{AtelierRuntime, CommandResult};
use crate::AppError;

const DANBOORU_SECRET_ID: &str = "danbooru-api-key:default";

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: Send + Sync,
    E: Send + Sync,
{
    /// Reads application-level Danbooru account metadata and keyring availability.
    ///
    /// # Errors
    /// Returns an error envelope when global settings cannot be read.
    pub async fn get_danbooru_account(&self) -> CommandResult<DanbooruAccountDto> {
        let settings = self
            .global_settings
            .get_global_settings()
            .await
            .map_err(AppError::from)
            .map_err(|error| error.envelope())?;
        let Some(username) = settings.integrations.danbooru_username else {
            return Ok(anonymous_account());
        };
        match self.secrets.read_secret(&danbooru_secret_id()).await {
            Ok(_) => Ok(DanbooruAccountDto {
                configured: true,
                state: DanbooruAccountStateDto::Configured,
                username: Some(username),
                level: None,
            }),
            Err(_) => Ok(DanbooruAccountDto {
                configured: true,
                state: DanbooruAccountStateDto::KeyringUnavailable,
                username: Some(username),
                level: None,
            }),
        }
    }

    /// Verifies and stores the application-level Danbooru credentials.
    ///
    /// # Errors
    /// Returns an error envelope for invalid credentials or storage failures.
    pub async fn save_danbooru_account(
        &self,
        request: SaveDanbooruAccountRequestDto,
    ) -> CommandResult<DanbooruAccountDto> {
        let _gate = self.danbooru_account_gate.lock().await;
        let username = request.username.trim().to_owned();
        if username.is_empty() {
            return Err(AppError::new(
                "danbooru_invalid_account",
                "Danbooru username must not be empty",
            )
            .envelope());
        }
        let secret_id = danbooru_secret_id();
        let previous_secret = match self.secrets.read_secret(&secret_id).await {
            Ok(secret) => Some(secret),
            Err(error) if error.kind == SecretsErrorKind::MissingSecret => None,
            Err(error) => return Err(AppError::from(error).envelope()),
        };
        let api_key = request
            .api_key
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .map(SecretValue::new)
            .or_else(|| previous_secret.clone())
            .ok_or_else(|| {
                AppError::new(
                    "danbooru_invalid_account",
                    "Danbooru API key is required for initial setup",
                )
                .envelope()
            })?;
        let credentials = DanbooruCredentials {
            username: username.clone(),
            api_key: api_key.clone(),
        };
        let profile = match self.danbooru.profile(&credentials).await {
            Ok(profile) => Some(profile),
            Err(error)
                if matches!(
                    error.kind,
                    DanbooruErrorKind::Unauthorized | DanbooruErrorKind::Forbidden
                ) =>
            {
                return Err(danbooru_error_envelope(error));
            }
            Err(error) if error.kind == DanbooruErrorKind::Unavailable => None,
            Err(error) => return Err(danbooru_error_envelope(error)),
        };

        if let Err(error) = self.secrets.write_secret(&secret_id, api_key).await {
            rollback_secret(&self.secrets, &secret_id, previous_secret.clone()).await;
            return Err(AppError::from(error).envelope());
        }
        let stored_username = profile
            .as_ref()
            .map_or_else(|| username.clone(), |profile| profile.username.clone());
        if let Err(error) = self
            .global_settings
            .update_danbooru_username(Some(stored_username.clone()))
            .await
        {
            rollback_secret(&self.secrets, &secret_id, previous_secret).await;
            return Err(AppError::from(error).envelope());
        }

        Ok(profile.map_or_else(
            || DanbooruAccountDto {
                configured: true,
                state: DanbooruAccountStateDto::Configured,
                username: Some(stored_username),
                level: None,
            },
            |profile| DanbooruAccountDto {
                configured: true,
                state: DanbooruAccountStateDto::Verified,
                username: Some(profile.username),
                level: profile.level,
            },
        ))
    }

    /// Verifies the currently configured Danbooru credentials.
    ///
    /// # Errors
    /// Returns an error envelope when configuration or Danbooru is unavailable.
    pub async fn probe_danbooru_account(&self) -> CommandResult<DanbooruAccountDto> {
        let Some(credentials) = self.danbooru_credentials().await? else {
            return Ok(anonymous_account());
        };
        match self.danbooru.profile(&credentials).await {
            Ok(profile) => Ok(DanbooruAccountDto {
                configured: true,
                state: DanbooruAccountStateDto::Verified,
                username: Some(profile.username),
                level: profile.level,
            }),
            Err(error)
                if matches!(
                    error.kind,
                    DanbooruErrorKind::Unauthorized | DanbooruErrorKind::Forbidden
                ) =>
            {
                Ok(DanbooruAccountDto {
                    configured: true,
                    state: DanbooruAccountStateDto::Invalid,
                    username: Some(credentials.username),
                    level: None,
                })
            }
            Err(error) => Err(danbooru_error_envelope(error)),
        }
    }

    /// Removes Danbooru account metadata and its keyring secret.
    ///
    /// # Errors
    /// Returns an error envelope when either storage operation fails.
    pub async fn delete_danbooru_account(&self) -> CommandResult<DanbooruAccountDto> {
        let _gate = self.danbooru_account_gate.lock().await;
        let secret_id = danbooru_secret_id();
        let previous_secret = match self.secrets.read_secret(&secret_id).await {
            Ok(secret) => Some(secret),
            Err(error) if error.kind == SecretsErrorKind::MissingSecret => None,
            Err(error) => return Err(AppError::from(error).envelope()),
        };
        if let Err(error) = self.secrets.delete_secret(&secret_id).await {
            rollback_secret(&self.secrets, &secret_id, previous_secret.clone()).await;
            return Err(AppError::from(error).envelope());
        }
        if let Err(error) = self.global_settings.update_danbooru_username(None).await {
            if let Some(secret) = previous_secret {
                let _ = self.secrets.write_secret(&secret_id, secret).await;
            }
            return Err(AppError::from(error).envelope());
        }
        Ok(anonymous_account())
    }

    /// Searches Danbooru posts using anonymous or configured authentication.
    ///
    /// # Errors
    /// Returns an error envelope for invalid queries, account failures, or HTTP failures.
    pub async fn search_danbooru_posts(
        &self,
        request: DanbooruSearchRequestDto,
    ) -> CommandResult<DanbooruPostPageDto> {
        let credentials = self.danbooru_credentials().await?;
        let page = self
            .danbooru
            .search(
                DanbooruSearchRequest {
                    query: request.query,
                    ratings: request.ratings.into_iter().map(rating_to_domain).collect(),
                    before_id: request.before_id,
                },
                credentials.as_ref(),
            )
            .await
            .map_err(danbooru_error_envelope)?;
        Ok(DanbooruPostPageDto {
            items: page.posts.iter().map(post_summary_to_dto).collect(),
            next_before_id: page.next_before_id,
            auth_mode: if page.authenticated {
                DanbooruAuthModeDto::Authenticated
            } else {
                DanbooruAuthModeDto::Anonymous
            },
        })
    }

    /// Loads one Danbooru post and enriches its tags with the local lexicon.
    ///
    /// # Errors
    /// Returns an error envelope when account or post loading fails.
    pub async fn get_danbooru_post_detail(
        &self,
        request: DanbooruPostDetailRequestDto,
    ) -> CommandResult<DanbooruPostDetailDto> {
        let credentials = self.danbooru_credentials().await?;
        let post = self
            .danbooru
            .post(request.post_id, credentials.as_ref())
            .await
            .map_err(danbooru_error_envelope)?;
        let names = post
            .ordered_tags()
            .into_iter()
            .take(500)
            .map(|(_, name)| name.to_owned())
            .collect::<Vec<_>>();
        let translations = self
            .lexicon
            .lookup_canonical_names(&names)
            .unwrap_or_default()
            .into_iter()
            .map(|item| {
                (
                    item.canonical_name,
                    (item.primary_translation, item.post_count),
                )
            })
            .collect::<HashMap<_, _>>();
        let tags = post
            .ordered_tags()
            .into_iter()
            .map(|(category, name)| {
                let enrichment = translations.get(name);
                DanbooruTagDto {
                    canonical_name: name.to_owned(),
                    category: tag_category_to_dto(category),
                    translation: enrichment
                        .map(|(translation, _)| translation)
                        .filter(|translation| !translation.trim().is_empty())
                        .cloned(),
                    post_count: enrichment.map(|(_, post_count)| *post_count),
                }
            })
            .collect();
        Ok(DanbooruPostDetailDto {
            post: post_summary_to_dto(&post),
            created_at: post.created_at.clone(),
            file_size: post.file_size,
            source_url: post.source_url.clone(),
            danbooru_url: format!("https://danbooru.donmai.us/posts/{}", post.id),
            tags,
        })
    }

    /// Loads an allowlisted preview or sample for a Danbooru post.
    ///
    /// # Errors
    /// Returns an error envelope when account, post, or media validation fails.
    pub async fn get_danbooru_media(
        &self,
        request: DanbooruMediaRequestDto,
    ) -> CommandResult<ResourceImageDto> {
        let credentials = self.danbooru_credentials().await?;
        let media = self
            .danbooru
            .media(
                request.post_id,
                match request.variant {
                    DanbooruMediaVariantDto::Preview => DanbooruMediaVariant::Preview,
                    DanbooruMediaVariantDto::Sample => DanbooruMediaVariant::Sample,
                },
                credentials.as_ref(),
            )
            .await
            .map_err(AppError::from)
            .map_err(|error| error.envelope())?;
        Ok(ResourceImageDto {
            image_base64: STANDARD.encode(media.bytes),
            mime_type: Some(media.mime_type),
        })
    }

    async fn danbooru_credentials(&self) -> CommandResult<Option<DanbooruCredentials>> {
        let settings = self
            .global_settings
            .get_global_settings()
            .await
            .map_err(AppError::from)
            .map_err(|error| error.envelope())?;
        let Some(username) = settings.integrations.danbooru_username else {
            return Ok(None);
        };
        let api_key = self
            .secrets
            .read_secret(&danbooru_secret_id())
            .await
            .map_err(|error| {
                AppError::new(
                    "danbooru_keyring_unavailable",
                    format!("configured Danbooru API key is unavailable: {error}"),
                )
                .envelope()
            })?;
        Ok(Some(DanbooruCredentials { username, api_key }))
    }
}

const fn anonymous_account() -> DanbooruAccountDto {
    DanbooruAccountDto {
        configured: false,
        state: DanbooruAccountStateDto::Anonymous,
        username: None,
        level: None,
    }
}

fn danbooru_error_envelope(error: atelier_danbooru::DanbooruError) -> ErrorEnvelopeDto {
    let retry_after_seconds = error.retry_after_seconds;
    let mut envelope = AppError::from(error).envelope();
    if let Some(seconds) = retry_after_seconds {
        envelope = envelope.with_details(serde_json::json!({ "retry_after_seconds": seconds }));
    }
    envelope
}

fn danbooru_secret_id() -> SecretRecordId {
    SecretRecordId::new(DANBOORU_SECRET_ID)
}

async fn rollback_secret<S: SecretStore>(
    secrets: &S,
    id: &SecretRecordId,
    previous: Option<SecretValue>,
) {
    if let Some(secret) = previous {
        let _ = secrets.write_secret(id, secret).await;
    } else {
        let _ = secrets.delete_secret(id).await;
    }
}

const fn rating_to_domain(value: DanbooruRatingDto) -> DanbooruRating {
    match value {
        DanbooruRatingDto::General => DanbooruRating::General,
        DanbooruRatingDto::Sensitive => DanbooruRating::Sensitive,
        DanbooruRatingDto::Questionable => DanbooruRating::Questionable,
        DanbooruRatingDto::Explicit => DanbooruRating::Explicit,
    }
}

const fn rating_to_dto(value: DanbooruRating) -> DanbooruRatingDto {
    match value {
        DanbooruRating::General => DanbooruRatingDto::General,
        DanbooruRating::Sensitive => DanbooruRatingDto::Sensitive,
        DanbooruRating::Questionable => DanbooruRatingDto::Questionable,
        DanbooruRating::Explicit => DanbooruRatingDto::Explicit,
    }
}

const fn tag_category_to_dto(value: DanbooruTagCategory) -> DanbooruTagCategoryDto {
    match value {
        DanbooruTagCategory::Artist => DanbooruTagCategoryDto::Artist,
        DanbooruTagCategory::Copyright => DanbooruTagCategoryDto::Copyright,
        DanbooruTagCategory::Character => DanbooruTagCategoryDto::Character,
        DanbooruTagCategory::General => DanbooruTagCategoryDto::General,
        DanbooruTagCategory::Meta => DanbooruTagCategoryDto::Meta,
    }
}

fn post_summary_to_dto(post: &DanbooruPost) -> DanbooruPostSummaryDto {
    DanbooruPostSummaryDto {
        id: post.id,
        rating: rating_to_dto(post.rating),
        width: post.width,
        height: post.height,
        score: post.score,
        favorite_count: post.favorite_count,
        file_extension: post.file_extension.clone(),
        tag_count: post.ordered_tags().len(),
        has_preview: post.preview_url.is_some(),
        has_sample: post.sample_url.is_some(),
    }
}
