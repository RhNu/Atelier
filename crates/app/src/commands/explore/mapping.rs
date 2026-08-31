use atelier_app_api::novelai_explore as dto;
use atelier_explore::novelai as domain;

pub(super) fn query(query: dto::NovelAiExploreQueryDto) -> domain::NovelAiExploreQuery {
    domain::NovelAiExploreQuery {
        tags: query.tags,
        creator_id: query.creator_id,
        random_salt: query.random_salt,
        sort: match query.sort {
            dto::NovelAiExploreSortDto::New => domain::NovelAiExploreSort::New,
            dto::NovelAiExploreSortDto::Top => domain::NovelAiExploreSort::Top,
            dto::NovelAiExploreSortDto::Hot => domain::NovelAiExploreSort::Hot,
            dto::NovelAiExploreSortDto::Random => domain::NovelAiExploreSort::Random,
        },
        period: query.period.map(|p| match p {
            dto::NovelAiExplorePeriodDto::Day => domain::NovelAiExplorePeriod::Day,
            dto::NovelAiExplorePeriodDto::Week => domain::NovelAiExplorePeriod::Week,
            dto::NovelAiExplorePeriodDto::Month => domain::NovelAiExplorePeriod::Month,
        }),
    }
}

pub(super) fn summary(post: &domain::NovelAiExplorePost) -> dto::NovelAiExplorePostSummaryDto {
    dto::NovelAiExplorePostSummaryDto {
        id: post.id.clone(),
        title: post.title.clone(),
        creator_id: post.creator_id.clone(),
        creator_name: post.creator_name.clone(),
        width: post.width,
        height: post.height,
        like_count: post.like_count,
    }
}

pub(super) fn detail(post: domain::NovelAiExplorePost) -> dto::NovelAiExplorePostDetailDto {
    dto::NovelAiExplorePostDetailDto {
        post: summary(&post),
        description: post.description,
        created_at: post.created_at,
        page_url: format!("https://novelai.net/explore/image/{}", post.id),
        metadata: metadata(post.metadata),
    }
}

fn metadata(value: domain::NovelAiExploreMetadata) -> dto::NovelAiExploreMetadataDto {
    dto::NovelAiExploreMetadataDto {
        status: match value.status {
            domain::ExploreMetadataStatus::Available => dto::ExploreMetadataStatusDto::Available,
            domain::ExploreMetadataStatus::Missing => dto::ExploreMetadataStatusDto::Missing,
            domain::ExploreMetadataStatus::Partial => dto::ExploreMetadataStatusDto::Partial,
            domain::ExploreMetadataStatus::Invalid => dto::ExploreMetadataStatusDto::Invalid,
        },
        prompt: value.prompt,
        negative_prompt: value.negative_prompt,
        characters: value.characters.into_iter().map(caption).collect(),
        negative_characters: value.negative_characters.into_iter().map(caption).collect(),
        use_coords: value.use_coords,
        use_order: value.use_order,
        negative_use_coords: value.negative_use_coords,
        negative_use_order: value.negative_use_order,
        parameters: value
            .parameters
            .into_iter()
            .map(|p| dto::ExploreGenerationParameterDto {
                name: p.name,
                value: p.value,
            })
            .collect(),
        raw: value.raw,
        warnings: value.warnings,
    }
}

fn caption(value: domain::ExploreCharacterCaption) -> dto::ExploreCharacterCaptionDto {
    dto::ExploreCharacterCaptionDto {
        text: value.text,
        centers: value
            .centers
            .into_iter()
            .map(|p| dto::ExploreCharacterCenterDto { x: p.x, y: p.y })
            .collect(),
    }
}
