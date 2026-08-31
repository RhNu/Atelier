use super::*;
use atelier_app_api::{explore::*, novelai_explore::*};
use atelier_explore::{
    ExploreCursor, ExploreMedia, ExploreMediaVariant, ExplorePage, ExploreResult, ExploreSource,
    novelai::{self, NovelAiExplorePost, NovelAiExploreQuery},
};

const POST_ID: &str = "00000000-0000-0000-0000-000000000001";

#[test]
fn public_explore_needs_neither_workspace_nor_secrets_and_binds_cursors() {
    block_on(async {
        let source = Arc::new(FakeExplore(Mutex::new(Vec::new())));
        let host = AtelierRuntime::with_dependencies(NoSecretAccess, RecordingFactory::default())
            .with_novelai_explore_source(source.clone());
        assert_eq!(host.list_explore_sources().len(), 2);
        let query = ExploreQueryDto::NovelaiExploreGallery(NovelAiExploreQueryDto {
            tags: vec!["blue sky".into()],
            sort: NovelAiExploreSortDto::New,
            period: None,
            creator_id: None,
            random_salt: None,
        });
        let first = host
            .search_explore_posts(ExploreSearchRequestDto {
                query: query.clone(),
                cursor: None,
            })
            .await
            .unwrap();
        assert!(!first.authenticated);
        assert!(
            matches!(&first.items[0], ExplorePostSummaryDto::NovelaiExploreGallery(post) if post.id == POST_ID)
        );
        let mut changed = query.clone();
        if let ExploreQueryDto::NovelaiExploreGallery(query) = &mut changed {
            query.tags.push("forest".into());
        }
        assert_eq!(
            host.search_explore_posts(ExploreSearchRequestDto {
                query: changed,
                cursor: first.next_cursor.clone()
            })
            .await
            .unwrap_err()
            .code,
            "explore_invalid_request"
        );
        host.search_explore_posts(ExploreSearchRequestDto {
            query,
            cursor: first.next_cursor,
        })
        .await
        .unwrap();
        assert_eq!(
            *source.0.lock().unwrap(),
            vec![None, Some(ExploreCursor::Offset(40))]
        );
        let item = ExploreItemRefDto {
            source_id: ExploreSourceIdDto::NovelaiExploreGallery,
            item_id: POST_ID.into(),
        };
        let detail = host.get_explore_post_detail(item.clone()).await.unwrap();
        let ExplorePostDetailDto::NovelaiExploreGallery(detail) = detail else {
            panic!("wrong source");
        };
        assert_eq!(detail.metadata.prompt.as_deref(), Some("  1.2::sky::\n"));
        assert_eq!(
            detail.page_url,
            format!("https://novelai.net/explore/image/{POST_ID}")
        );
        let media = host
            .get_explore_media(ExploreMediaRequestDto {
                item,
                variant: ExploreMediaVariantDto::Thumbnail,
            })
            .await
            .unwrap();
        assert_eq!(media.mime_type.as_deref(), Some("image/png"));
        assert_eq!(media.image_base64, "AQID");
    });
}

#[derive(Clone)]
struct NoSecretAccess;
#[async_trait]
impl SecretStore for NoSecretAccess {
    async fn write_secret(&self, _: &SecretRecordId, _: SecretValue) -> SecretsResult<()> {
        panic!("Explore must not write secrets")
    }
    async fn read_secret(&self, _: &SecretRecordId) -> SecretsResult<SecretValue> {
        panic!("Explore must not read secrets")
    }
    async fn delete_secret(&self, _: &SecretRecordId) -> SecretsResult<bool> {
        panic!("Explore must not delete secrets")
    }
}
struct FakeExplore(Mutex<Vec<Option<ExploreCursor>>>);
#[async_trait]
impl ExploreSource for FakeExplore {
    type Query = NovelAiExploreQuery;
    type Post = NovelAiExplorePost;
    async fn search(
        &self,
        query: Self::Query,
        cursor: Option<ExploreCursor>,
    ) -> ExploreResult<ExplorePage<Self::Post>> {
        query.validate()?;
        self.0.lock().unwrap().push(cursor);
        Ok(ExplorePage {
            items: vec![post()],
            next_cursor: Some(ExploreCursor::Offset(40)),
            total: Some(80),
            authenticated: false,
        })
    }
    async fn detail(&self, id: &str) -> ExploreResult<Self::Post> {
        assert_eq!(id, POST_ID);
        Ok(post())
    }
    async fn media(&self, id: &str, variant: ExploreMediaVariant) -> ExploreResult<ExploreMedia> {
        assert_eq!(id, POST_ID);
        assert_eq!(variant, ExploreMediaVariant::Thumbnail);
        Ok(ExploreMedia {
            mime_type: "image/png".into(),
            bytes: vec![1, 2, 3],
        })
    }
}
fn post() -> NovelAiExplorePost {
    NovelAiExplorePost {
        id: POST_ID.into(),
        title: "Synthetic".into(),
        description: String::new(),
        creator_id: None,
        creator_name: None,
        created_at: String::new(),
        width: 832,
        height: 1216,
        like_count: None,
        metadata: novelai::NovelAiExploreMetadata {
            status: novelai::ExploreMetadataStatus::Available,
            prompt: Some("  1.2::sky::\n".into()),
            negative_prompt: None,
            characters: vec![],
            negative_characters: vec![],
            use_coords: None,
            use_order: None,
            negative_use_coords: None,
            negative_use_order: None,
            parameters: vec![],
            raw: None,
            warnings: vec![],
        },
    }
}
