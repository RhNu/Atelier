use super::{
    AppResult, CompiledPrompt, CompiledPromptDto, PromptChunk, PromptChunkDto, PromptChunkId,
    PromptChunkKey, PromptFunctionTraceEntry, PromptFunctionTraceEntryDto, PromptLexiconCatalog,
    PromptLexiconCatalogDto, PromptLexiconCategorySummaryDto, PromptLexiconEntry,
    PromptLexiconEntryDto, PromptLexiconListPage, PromptLexiconListQuery,
    PromptLexiconListQueryDto, PromptLexiconMatchField, PromptLexiconMatchRank,
    PromptLexiconPageDto, PromptLexiconStatsDto, PromptLexiconSubcategorySummaryDto, PromptTrace,
    PromptTraceDto, UpsertPromptChunkRequest, UpsertPromptChunkRequestDto, resource_ref_from_dto,
    resource_ref_to_dto,
};

pub fn prompt_chunk_to_dto(chunk: &PromptChunk) -> PromptChunkDto {
    PromptChunkDto {
        chunk_id: chunk.id.as_str().to_owned(),
        key: chunk.key.as_str().to_owned(),
        content: chunk.content.clone(),
        category: chunk.category.clone(),
        description: chunk.description.clone(),
        preview: chunk.preview_thumb.as_ref().map(resource_ref_to_dto),
        created_at_ms: chunk.created_at_ms,
        updated_at_ms: chunk.updated_at_ms,
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
    })
}

pub fn compiled_prompt_to_dto(value: &CompiledPrompt) -> CompiledPromptDto {
    CompiledPromptDto {
        expanded_prompt: value.expanded_prompt.clone(),
        trace: prompt_trace_to_dto(&value.trace),
    }
}

fn prompt_trace_to_dto(value: &PromptTrace) -> PromptTraceDto {
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

pub fn lexicon_catalog_to_dto(value: PromptLexiconCatalog) -> PromptLexiconCatalogDto {
    PromptLexiconCatalogDto {
        stats: PromptLexiconStatsDto {
            total_tags: value.stats.total_tags,
            categorized_tags: value.stats.categorized_tags,
            uncategorized_tags: value.stats.uncategorized_tags,
            matched_weights: value.stats.matched_weights,
            total_translations: value.stats.total_translations,
            tags_with_aliases: value.stats.tags_with_aliases,
            max_aliases_per_tag: value.stats.max_aliases_per_tag,
            source_count: value.stats.source_count,
            manifest_version: value.stats.manifest_version,
            primary_from_category_json: value.stats.primary_from_category_json,
            primary_from_manifest_sources: value.stats.primary_from_manifest_sources,
            primary_fallback_to_tag: value.stats.primary_fallback_to_tag,
        },
        categories: value
            .categories
            .into_iter()
            .map(|category| PromptLexiconCategorySummaryDto {
                name: category.name,
                tag_count: category.tag_count,
                subcategory_count: category.subcategory_count,
                subcategories: category
                    .subcategories
                    .into_iter()
                    .map(|subcategory| PromptLexiconSubcategorySummaryDto {
                        name: subcategory.name,
                        tag_count: subcategory.tag_count,
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn lexicon_query_to_domain(value: PromptLexiconListQueryDto) -> PromptLexiconListQuery {
    PromptLexiconListQuery {
        query: value.query,
        category: value.category,
        subcategory: value.subcategory,
        limit: value.limit,
        offset: value.offset,
    }
}

pub fn lexicon_page_to_dto(value: PromptLexiconListPage) -> PromptLexiconPageDto {
    PromptLexiconPageDto {
        items: value.items.into_iter().map(lexicon_entry_to_dto).collect(),
        total: value.total,
        offset: value.offset,
        limit: value.limit,
    }
}

pub fn lexicon_search_to_page(
    value: atelier_prompt_lexicon::PromptLexiconSearchResult,
    limit: usize,
) -> PromptLexiconPageDto {
    PromptLexiconPageDto {
        items: value.items.into_iter().map(lexicon_entry_to_dto).collect(),
        total: value.total,
        offset: 0,
        limit,
    }
}

fn lexicon_entry_to_dto(value: PromptLexiconEntry) -> PromptLexiconEntryDto {
    PromptLexiconEntryDto {
        tag: value.tag,
        weight: value.weight,
        category: value.category,
        subcategory: value.subcategory,
        primary_translation: value.primary_translation,
        matched_translation: value.matched_translation,
        match_field: match_field_as_str(value.match_field).to_owned(),
        match_rank: match_rank_as_str(value.match_rank).to_owned(),
    }
}

const fn match_field_as_str(value: PromptLexiconMatchField) -> &'static str {
    match value {
        PromptLexiconMatchField::Tag => "tag",
        PromptLexiconMatchField::PrimaryTranslation => "primary_translation",
        PromptLexiconMatchField::Alias => "alias",
    }
}

const fn match_rank_as_str(value: PromptLexiconMatchRank) -> &'static str {
    match value {
        PromptLexiconMatchRank::Exact => "exact",
        PromptLexiconMatchRank::Prefix => "prefix",
        PromptLexiconMatchRank::Substring => "substring",
    }
}
