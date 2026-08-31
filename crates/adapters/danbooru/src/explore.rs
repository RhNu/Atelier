use std::sync::Arc;

use async_trait::async_trait;
use atelier_danbooru::{
    DanbooruClient, DanbooruCredentials, DanbooruMediaVariant, DanbooruPost, DanbooruSearchRequest,
};
use atelier_explore::{
    DanbooruExploreQuery, ExploreCursor, ExploreError, ExploreMedia, ExploreMediaVariant,
    ExplorePage, ExploreResult, ExploreSource,
};

/// Binds one resolved identity to the existing Danbooru client for an app operation.
pub struct DanbooruExploreSource {
    client: Arc<dyn DanbooruClient>,
    credentials: Option<DanbooruCredentials>,
}

impl DanbooruExploreSource {
    #[must_use]
    pub fn new(client: Arc<dyn DanbooruClient>, credentials: Option<DanbooruCredentials>) -> Self {
        Self {
            client,
            credentials,
        }
    }
}

#[async_trait]
impl ExploreSource for DanbooruExploreSource {
    type Query = DanbooruExploreQuery;
    type Post = DanbooruPost;

    async fn search(
        &self,
        query: Self::Query,
        cursor: Option<ExploreCursor>,
    ) -> ExploreResult<ExplorePage<Self::Post>> {
        let before_id = match cursor {
            None => None,
            Some(ExploreCursor::BeforeId(id)) if id > 0 => Some(id),
            _ => return Err(ExploreError::invalid("invalid Danbooru cursor")),
        };
        let page = self
            .client
            .search(
                DanbooruSearchRequest {
                    query: query.query,
                    ratings: query.ratings,
                    before_id,
                },
                self.credentials.as_ref(),
            )
            .await?;
        Ok(ExplorePage {
            items: page.posts,
            total: None,
            next_cursor: page.next_before_id.map(ExploreCursor::BeforeId),
            authenticated: page.authenticated,
        })
    }

    async fn detail(&self, item_id: &str) -> ExploreResult<Self::Post> {
        Ok(self
            .client
            .post(post_id(item_id)?, self.credentials.as_ref())
            .await?)
    }

    async fn media(
        &self,
        item_id: &str,
        variant: ExploreMediaVariant,
    ) -> ExploreResult<ExploreMedia> {
        let variant = match variant {
            ExploreMediaVariant::Thumbnail => DanbooruMediaVariant::Preview,
            ExploreMediaVariant::Preview => DanbooruMediaVariant::Sample,
        };
        let media = self
            .client
            .media(post_id(item_id)?, variant, self.credentials.as_ref())
            .await?;
        Ok(ExploreMedia {
            mime_type: media.mime_type,
            bytes: media.bytes,
        })
    }
}

fn post_id(id: &str) -> ExploreResult<u64> {
    id.parse::<u64>()
        .ok()
        .filter(|value| *value > 0 && value.to_string() == id)
        .ok_or_else(|| ExploreError::invalid("invalid Danbooru post ID"))
}
