use super::{
    AppResult, CompiledPrompt, CompiledPromptDto, DanbooruCategory, LexiconBootstrap,
    LexiconBootstrapDto, LexiconCapabilityStatusDto, LexiconCategoryDto, LexiconContentRating,
    LexiconContentRatingDto, LexiconEntityDetail, LexiconEntityDetailDto, LexiconEntityKind,
    LexiconEntityKindDto, LexiconFacetDto, LexiconGroupSummaryDto, LexiconRelatedEntityDto,
    LexiconSearchFilters, LexiconSearchItem, LexiconSearchItemDto, LexiconSearchMode,
    LexiconSearchModeDto, LexiconSearchPage, LexiconSearchPageDto, LexiconSearchQuery,
    LexiconSearchRequestDto, LexiconStatsDto, LocalizedLexiconTextDto, PromptChunk, PromptChunkDto,
    PromptChunkId, PromptChunkKey, PromptFunctionTraceEntry, PromptFunctionTraceEntryDto,
    PromptPreset, PromptPresetBehavior, PromptPresetBehaviorDto, PromptPresetDto, PromptPresetId,
    PromptPresetKind, PromptPresetKindDto, PromptTrace, PromptTraceDto, UpsertPromptChunkRequest,
    UpsertPromptChunkRequestDto, UpsertPromptPresetRequest, UpsertPromptPresetRequestDto,
    image_model_to_domain, image_model_to_dto, quality_preset_to_domain, quality_preset_to_dto,
    resource_ref_from_dto, resource_ref_to_dto,
};
pub fn prompt_chunk_to_dto(chunk: &PromptChunk) -> PromptChunkDto {
    PromptChunkDto {
        chunk_id: chunk.id.as_str().to_owned(),
        key: chunk.key.as_str().to_owned(),
        content: chunk.content.clone(),
        category: chunk.category.clone(),
        description: chunk.description.clone(),
        preview: chunk.preview_thumb.as_ref().map(resource_ref_to_dto),
        models: chunk
            .models
            .iter()
            .copied()
            .map(image_model_to_dto)
            .collect(),
        created_at_ms: chunk.created_at_ms,
        updated_at_ms: chunk.updated_at_ms,
    }
}

pub fn prompt_preset_to_dto(preset: &PromptPreset) -> PromptPresetDto {
    PromptPresetDto {
        preset_id: preset.id.as_str().to_owned(),
        kind: prompt_preset_kind_to_dto(preset.kind),
        name: preset.name.clone(),
        category: preset.category.clone(),
        description: preset.description.clone(),
        order: preset.order,
        prompt_behavior: prompt_preset_behavior_to_dto(&preset.prompt_behavior),
        uc_behavior: prompt_preset_behavior_to_dto(&preset.uc_behavior),
        quality_override: preset.quality_override.map(quality_preset_to_dto),
        uc_preset_override: preset.uc_preset_override.clone(),
        preview: preset.preview_thumb.as_ref().map(resource_ref_to_dto),
        models: preset
            .models
            .iter()
            .copied()
            .map(image_model_to_dto)
            .collect(),
        created_at_ms: preset.created_at_ms,
        updated_at_ms: preset.updated_at_ms,
    }
}

pub fn upsert_prompt_preset_to_domain(
    request: UpsertPromptPresetRequestDto,
) -> UpsertPromptPresetRequest {
    UpsertPromptPresetRequest {
        preset_id: request.preset_id.map(PromptPresetId::new),
        kind: prompt_preset_kind_to_domain(request.kind),
        name: request.name,
        category: request.category,
        description: request.description,
        order: request.order,
        prompt_behavior: prompt_preset_behavior_to_domain(request.prompt_behavior),
        uc_behavior: prompt_preset_behavior_to_domain(request.uc_behavior),
        quality_override: request.quality_override.map(quality_preset_to_domain),
        uc_preset_override: request.uc_preset_override,
        preview_thumb: request.preview.map(resource_ref_from_dto),
        models: request
            .models
            .into_iter()
            .map(image_model_to_domain)
            .collect(),
    }
}

fn prompt_preset_behavior_to_domain(value: PromptPresetBehaviorDto) -> PromptPresetBehavior {
    match value {
        PromptPresetBehaviorDto::Surround { before, after } => {
            PromptPresetBehavior::Surround { before, after }
        }
        PromptPresetBehaviorDto::Replace { text } => PromptPresetBehavior::Replace { text },
    }
}

fn prompt_preset_behavior_to_dto(value: &PromptPresetBehavior) -> PromptPresetBehaviorDto {
    match value {
        PromptPresetBehavior::Surround { before, after } => PromptPresetBehaviorDto::Surround {
            before: before.clone(),
            after: after.clone(),
        },
        PromptPresetBehavior::Replace { text } => {
            PromptPresetBehaviorDto::Replace { text: text.clone() }
        }
    }
}

pub const fn prompt_preset_kind_to_domain(value: PromptPresetKindDto) -> PromptPresetKind {
    match value {
        PromptPresetKindDto::Main => PromptPresetKind::Main,
        PromptPresetKindDto::Character => PromptPresetKind::Character,
    }
}

pub const fn prompt_preset_kind_to_dto(value: PromptPresetKind) -> PromptPresetKindDto {
    match value {
        PromptPresetKind::Main => PromptPresetKindDto::Main,
        PromptPresetKind::Character => PromptPresetKindDto::Character,
    }
}

pub fn upsert_prompt_chunk_to_domain(
    request: UpsertPromptChunkRequestDto,
) -> AppResult<UpsertPromptChunkRequest> {
    Ok(UpsertPromptChunkRequest {
        chunk_id: request.chunk_id.map(PromptChunkId::new),
        key: PromptChunkKey::parse(&request.key)?,
        content: request.content,
        category: request.category,
        description: request.description,
        preview_thumb: request.preview.map(resource_ref_from_dto),
        models: request
            .models
            .into_iter()
            .map(image_model_to_domain)
            .collect(),
    })
}

pub fn compiled_prompt_to_dto(value: &CompiledPrompt) -> CompiledPromptDto {
    CompiledPromptDto {
        expanded_prompt: value.expanded_prompt.clone(),
        trace: prompt_trace_to_dto(&value.trace),
    }
}

pub fn prompt_trace_to_dto(value: &PromptTrace) -> PromptTraceDto {
    PromptTraceDto {
        raw_prompt: value.raw_prompt.clone(),
        expanded_prompt: value.expanded_prompt.clone(),
        function_calls: value
            .function_calls
            .iter()
            .map(trace_entry_to_dto)
            .collect(),
    }
}

fn trace_entry_to_dto(value: &PromptFunctionTraceEntry) -> PromptFunctionTraceEntryDto {
    PromptFunctionTraceEntryDto {
        function_name: value.function_name.clone(),
        raw_call: value.raw_call.clone(),
        resolved_arguments: value.resolved_arguments.clone(),
        result_text: value.result_text.clone(),
        depth: value.depth,
        call_chain: value.call_chain.clone(),
    }
}

pub fn lexicon_bootstrap_to_dto(value: LexiconBootstrap) -> LexiconBootstrapDto {
    LexiconBootstrapDto {
        bundle_version: value.bundle_version,
        status: LexiconCapabilityStatusDto {
            lexical_available: value.status.lexical_available,
            semantic_available: value.status.semantic_available,
            message: value.status.message,
        },
        stats: LexiconStatsDto {
            total_entities: value.stats.total_entities,
            tag_entities: value.stats.tag_entities,
            artist_entities: value.stats.artist_entities,
            sensitive_entities: value.stats.sensitive_entities,
            translation_count: value.stats.translation_count,
            group_count: value.stats.group_count,
        },
        categories: value
            .categories
            .into_iter()
            .map(|facet| LexiconFacetDto {
                value: facet.value,
                label: facet.label,
                count: facet.count,
            })
            .collect(),
        groups: value
            .groups
            .into_iter()
            .map(|group| LexiconGroupSummaryDto {
                id: group.id,
                name: group.name,
                member_count: group.member_count,
            })
            .collect(),
    }
}

pub fn lexicon_search_query_to_domain(value: LexiconSearchRequestDto) -> LexiconSearchQuery {
    LexiconSearchQuery {
        text: value.query,
        mode: match value.mode {
            LexiconSearchModeDto::Lexical => LexiconSearchMode::Lexical,
            LexiconSearchModeDto::Semantic => LexiconSearchMode::Semantic,
        },
        filters: LexiconSearchFilters {
            entity_kinds: value
                .filters
                .entity_kinds
                .into_iter()
                .map(entity_kind_to_domain)
                .collect(),
            categories: value
                .filters
                .categories
                .into_iter()
                .map(category_to_domain)
                .collect(),
            group_ids: value.filters.group_ids,
            ratings: value
                .filters
                .ratings
                .into_iter()
                .map(rating_to_domain)
                .collect(),
        },
        selected_entity_ids: value.selected_entity_ids,
        offset: value.offset,
        limit: value.limit,
    }
}

pub fn lexicon_page_to_dto(value: LexiconSearchPage) -> LexiconSearchPageDto {
    LexiconSearchPageDto {
        items: value.items.into_iter().map(lexicon_item_to_dto).collect(),
        total: value.total,
        offset: value.offset,
        limit: value.limit,
    }
}

pub fn lexicon_item_to_dto(value: LexiconSearchItem) -> LexiconSearchItemDto {
    LexiconSearchItemDto {
        entity_id: value.entity_id,
        canonical_name: value.canonical_name,
        primary_translation: value.primary_translation,
        kind: entity_kind_to_dto(value.kind),
        category: category_to_dto(value.category),
        post_count: value.post_count,
        rating: rating_to_dto(value.rating),
        matched_text: value.matched_text,
        match_reason: value.match_reason.as_str().to_owned(),
        score: value.score,
    }
}

pub fn lexicon_detail_to_dto(value: LexiconEntityDetail) -> LexiconEntityDetailDto {
    LexiconEntityDetailDto {
        entity: lexicon_item_to_dto(value.entity),
        translations: value
            .translations
            .into_iter()
            .map(|text| LocalizedLexiconTextDto {
                locale: text.locale,
                text: text.text,
            })
            .collect(),
        aliases: value.aliases,
        wiki: value
            .wiki
            .into_iter()
            .map(|text| LocalizedLexiconTextDto {
                locale: text.locale,
                text: text.text,
            })
            .collect(),
        groups: value
            .groups
            .into_iter()
            .map(|group| LexiconGroupSummaryDto {
                id: group.id,
                name: group.name,
                member_count: group.member_count,
            })
            .collect(),
        related: value
            .related
            .into_iter()
            .map(|related| LexiconRelatedEntityDto {
                entity: lexicon_item_to_dto(related.entity),
                relation: related.relation,
                score: related.score,
            })
            .collect(),
    }
}

const fn entity_kind_to_domain(value: LexiconEntityKindDto) -> LexiconEntityKind {
    match value {
        LexiconEntityKindDto::Tag => LexiconEntityKind::Tag,
        LexiconEntityKindDto::Artist => LexiconEntityKind::Artist,
    }
}

const fn entity_kind_to_dto(value: LexiconEntityKind) -> LexiconEntityKindDto {
    match value {
        LexiconEntityKind::Tag => LexiconEntityKindDto::Tag,
        LexiconEntityKind::Artist => LexiconEntityKindDto::Artist,
    }
}

const fn category_to_domain(value: LexiconCategoryDto) -> DanbooruCategory {
    match value {
        LexiconCategoryDto::General => DanbooruCategory::General,
        LexiconCategoryDto::Copyright => DanbooruCategory::Copyright,
        LexiconCategoryDto::Character => DanbooruCategory::Character,
        LexiconCategoryDto::Artist => DanbooruCategory::Artist,
    }
}

const fn category_to_dto(value: DanbooruCategory) -> LexiconCategoryDto {
    match value {
        DanbooruCategory::General => LexiconCategoryDto::General,
        DanbooruCategory::Copyright => LexiconCategoryDto::Copyright,
        DanbooruCategory::Character => LexiconCategoryDto::Character,
        DanbooruCategory::Artist => LexiconCategoryDto::Artist,
    }
}

const fn rating_to_domain(value: LexiconContentRatingDto) -> LexiconContentRating {
    match value {
        LexiconContentRatingDto::Safe => LexiconContentRating::Safe,
        LexiconContentRatingDto::Sensitive => LexiconContentRating::Sensitive,
        LexiconContentRatingDto::Unknown => LexiconContentRating::Unknown,
    }
}

const fn rating_to_dto(value: LexiconContentRating) -> LexiconContentRatingDto {
    match value {
        LexiconContentRating::Safe => LexiconContentRatingDto::Safe,
        LexiconContentRating::Sensitive => LexiconContentRatingDto::Sensitive,
        LexiconContentRating::Unknown => LexiconContentRatingDto::Unknown,
    }
}
