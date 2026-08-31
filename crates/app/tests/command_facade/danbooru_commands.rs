use atelier_app_api::danbooru::{
    DanbooruAccountStateDto, DanbooruRatingDto, SaveDanbooruAccountRequestDto,
};
use atelier_app_api::explore::{
    DanbooruExploreQueryDto, ExplorePostSummaryDto, ExploreQueryDto, ExploreSearchRequestDto,
};
use atelier_danbooru::{
    DanbooruClient, DanbooruCredentials, DanbooruError, DanbooruErrorKind, DanbooruMedia,
    DanbooruMediaVariant, DanbooruPost, DanbooruPostPage, DanbooruProfile, DanbooruResult,
    DanbooruSearchRequest,
};

use super::*;

#[test]
fn verified_account_is_shared_with_search_and_can_be_deleted() {
    block_on(async {
        let host = test_host_with_global_settings(GlobalSettings::default())
            .with_danbooru_client(Arc::new(FakeDanbooruClient::verified()));

        let saved = host
            .save_danbooru_account(SaveDanbooruAccountRequestDto {
                username: " alice ".to_owned(),
                api_key: Some("secret".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(saved.state, DanbooruAccountStateDto::Verified);
        assert_eq!(saved.username.as_deref(), Some("alice"));
        assert_eq!(saved.level.as_deref(), Some("Member"));

        let page = host
            .search_explore_posts(ExploreSearchRequestDto {
                query: ExploreQueryDto::DanbooruDatabase(DanbooruExploreQueryDto {
                    query: "blue_eyes".to_owned(),
                    ratings: vec![DanbooruRatingDto::General, DanbooruRatingDto::Sensitive],
                }),
                cursor: None,
            })
            .await
            .unwrap();
        assert!(page.authenticated);
        assert!(
            matches!(&page.items[0], ExplorePostSummaryDto::DanbooruDatabase(post) if post.id == 42)
        );

        assert_eq!(
            host.probe_danbooru_account().await.unwrap().state,
            DanbooruAccountStateDto::Verified
        );
        assert_eq!(
            host.delete_danbooru_account().await.unwrap().state,
            DanbooruAccountStateDto::Anonymous
        );
        assert!(!host.get_danbooru_account().await.unwrap().configured);
    });
}

#[test]
fn invalid_credentials_do_not_replace_anonymous_configuration() {
    block_on(async {
        let host = test_host_with_global_settings(GlobalSettings::default()).with_danbooru_client(
            Arc::new(FakeDanbooruClient::failing(DanbooruErrorKind::Unauthorized)),
        );

        let error = host
            .save_danbooru_account(SaveDanbooruAccountRequestDto {
                username: "alice".to_owned(),
                api_key: Some("bad-key".to_owned()),
            })
            .await
            .unwrap_err();
        assert_eq!(error.code, "danbooru_unauthorized");
        assert_eq!(
            host.get_danbooru_account().await.unwrap().state,
            DanbooruAccountStateDto::Anonymous
        );
    });
}

#[test]
fn transient_probe_failure_saves_an_unverified_configuration() {
    block_on(async {
        let host = test_host_with_global_settings(GlobalSettings::default()).with_danbooru_client(
            Arc::new(FakeDanbooruClient::failing(DanbooruErrorKind::Unavailable)),
        );

        let saved = host
            .save_danbooru_account(SaveDanbooruAccountRequestDto {
                username: "alice".to_owned(),
                api_key: Some("secret".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(saved.state, DanbooruAccountStateDto::Configured);
        assert_eq!(
            host.get_danbooru_account().await.unwrap().state,
            DanbooruAccountStateDto::Configured
        );
    });
}

#[test]
fn partial_keyring_write_is_rolled_back() {
    block_on(async {
        let store = PartiallyFailingSecretStore::fail_next_write();
        let host = test_host_with_secret_store(store.clone(), GlobalSettings::default());

        let error = host
            .save_danbooru_account(SaveDanbooruAccountRequestDto {
                username: "alice".to_owned(),
                api_key: Some("secret".to_owned()),
            })
            .await
            .unwrap_err();

        assert_eq!(error.code, "secret_store");
        assert!(store.values().is_empty());
        assert_eq!(
            host.get_danbooru_account().await.unwrap().state,
            DanbooruAccountStateDto::Anonymous
        );
    });
}

#[test]
fn partial_keyring_delete_restores_the_configured_secret() {
    block_on(async {
        let store = PartiallyFailingSecretStore::fail_next_delete_with_secret();
        let mut settings = GlobalSettings::default();
        settings.integrations.danbooru_username = Some("alice".to_owned());
        let host = test_host_with_secret_store(store.clone(), settings);

        let error = host.delete_danbooru_account().await.unwrap_err();

        assert_eq!(error.code, "secret_store");
        assert_eq!(
            store.values(),
            vec![("danbooru-api-key:default".to_owned(), "secret".to_owned())]
        );
        assert_eq!(
            host.get_danbooru_account().await.unwrap().state,
            DanbooruAccountStateDto::Configured
        );
    });
}

struct FakeDanbooruClient {
    profile_error: Option<DanbooruErrorKind>,
}

impl FakeDanbooruClient {
    const fn verified() -> Self {
        Self {
            profile_error: None,
        }
    }

    const fn failing(kind: DanbooruErrorKind) -> Self {
        Self {
            profile_error: Some(kind),
        }
    }
}

#[async_trait]
impl DanbooruClient for FakeDanbooruClient {
    async fn search(
        &self,
        _request: DanbooruSearchRequest,
        credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruPostPage> {
        Ok(DanbooruPostPage {
            posts: vec![post()],
            next_before_id: None,
            authenticated: credentials.is_some(),
        })
    }

    async fn post(
        &self,
        _post_id: u64,
        _credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruPost> {
        Ok(post())
    }

    async fn media(
        &self,
        _post_id: u64,
        _variant: DanbooruMediaVariant,
        _credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruMedia> {
        Ok(DanbooruMedia {
            mime_type: "image/jpeg".to_owned(),
            bytes: vec![1, 2, 3],
        })
    }

    async fn profile(&self, credentials: &DanbooruCredentials) -> DanbooruResult<DanbooruProfile> {
        if let Some(kind) = self.profile_error {
            return Err(DanbooruError::new(kind, "profile failed"));
        }
        Ok(DanbooruProfile {
            username: credentials.username.clone(),
            level: Some("Member".to_owned()),
        })
    }
}

#[derive(Clone)]
struct PartiallyFailingSecretStore {
    state: Arc<Mutex<Vec<(String, String)>>>,
    fail_write: Arc<Mutex<bool>>,
    fail_delete: Arc<Mutex<bool>>,
}

impl PartiallyFailingSecretStore {
    fn fail_next_write() -> Self {
        Self {
            state: Arc::default(),
            fail_write: Arc::new(Mutex::new(true)),
            fail_delete: Arc::default(),
        }
    }

    fn fail_next_delete_with_secret() -> Self {
        Self {
            state: Arc::new(Mutex::new(vec![(
                "danbooru-api-key:default".to_owned(),
                "secret".to_owned(),
            )])),
            fail_write: Arc::default(),
            fail_delete: Arc::new(Mutex::new(true)),
        }
    }

    fn values(&self) -> Vec<(String, String)> {
        self.state.lock().unwrap().clone()
    }
}

#[async_trait]
impl SecretStore for PartiallyFailingSecretStore {
    async fn write_secret(&self, id: &SecretRecordId, secret: SecretValue) -> SecretsResult<()> {
        let mut state = self.state.lock().unwrap();
        state.retain(|(candidate, _)| candidate != id.as_str());
        state.push((id.as_str().to_owned(), secret.expose_secret().to_owned()));
        drop(state);
        if std::mem::take(&mut *self.fail_write.lock().unwrap()) {
            Err(atelier_secrets::SecretsError::secret_store(
                "partial keyring write",
            ))
        } else {
            Ok(())
        }
    }

    async fn read_secret(&self, id: &SecretRecordId) -> SecretsResult<SecretValue> {
        self.state
            .lock()
            .unwrap()
            .iter()
            .find(|(candidate, _)| candidate == id.as_str())
            .map(|(_, value)| SecretValue::new(value.clone()))
            .ok_or_else(|| atelier_secrets::SecretsError::missing_secret(id.as_str()))
    }

    async fn delete_secret(&self, id: &SecretRecordId) -> SecretsResult<bool> {
        let mut state = self.state.lock().unwrap();
        let before = state.len();
        state.retain(|(candidate, _)| candidate != id.as_str());
        if std::mem::take(&mut *self.fail_delete.lock().unwrap()) {
            Err(atelier_secrets::SecretsError::secret_store(
                "partial keyring delete",
            ))
        } else {
            Ok(state.len() != before)
        }
    }
}

fn test_host_with_secret_store(
    store: PartiallyFailingSecretStore,
    settings: GlobalSettings,
) -> AtelierRuntime<PartiallyFailingSecretStore, RecordingFactory> {
    AtelierRuntime::with_global_settings_dependencies_extractor_and_safety_scanner(
        GlobalSettingsService::new(Arc::new(MemoryGlobalSettingsRepository {
            settings: Mutex::new(settings),
        })),
        store,
        RecordingFactory::default(),
        NovelAiEmbeddedVibeExtractor,
        None,
    )
    .with_danbooru_client(Arc::new(FakeDanbooruClient::verified()))
}

fn post() -> DanbooruPost {
    DanbooruPost {
        id: 42,
        created_at: "2026-07-30T00:00:00Z".to_owned(),
        rating: atelier_danbooru::DanbooruRating::General,
        width: 1024,
        height: 768,
        score: 9,
        favorite_count: 3,
        file_extension: "jpg".to_owned(),
        file_size: 1024,
        source_url: None,
        preview_url: None,
        sample_url: None,
        artist_tags: vec!["artist_a".to_owned()],
        copyright_tags: Vec::new(),
        character_tags: Vec::new(),
        general_tags: vec!["blue_eyes".to_owned()],
        meta_tags: Vec::new(),
    }
}
