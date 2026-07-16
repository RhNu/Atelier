use super::{
    AppResult, CompiledPrompt, CompiledPromptDto, PromptChunk, PromptChunkDto, PromptChunkId,
    PromptChunkKey, PromptFunctionTraceEntry, PromptFunctionTraceEntryDto, PromptLexiconCatalog,
    PromptLexiconCatalogDto, PromptLexiconCategorySummaryDto, PromptLexiconEntry,
    PromptLexiconEntryDto, PromptLexiconListPage, PromptLexiconListQuery,
    PromptLexiconListQueryDto, PromptLexiconMatchField, PromptLexiconMatchRank,
    PromptLexiconPageDto, PromptLexiconStatsDto, PromptLexiconSubcategorySummaryDto, PromptPreset,
    PromptPresetDto, PromptPresetId, PromptPresetKind, PromptPresetKindDto, PromptTrace,
    PromptTraceDto, UpsertPromptChunkRequest, UpsertPromptChunkRequestDto,
    UpsertPromptPresetRequest, UpsertPromptPresetRequestDto, resource_ref_from_dto,
    resource_ref_to_dto,
};
use atelier_app_api::prompt::{
    PromptAnalysisDto, PromptDiagnosticDto, PromptDiagnosticSeverityDto, PromptHighlightSpanDto,
    PromptSyntaxNodeDto, PromptSyntaxProfileDto, PromptTextRangeDto, PromptTokenDto,
};
use atelier_prompt::{
    FunctionRegistry, PromptDiagnosticKind, PromptSpan, PromptSyntaxProfile, PromptTokenKind,
    parse_prompt,
};

pub fn analyze_prompt_to_dto(
    text: String,
    profile_dto: PromptSyntaxProfileDto,
) -> PromptAnalysisDto {
    let profile = syntax_profile_to_domain(profile_dto);
    let parsed = parse_prompt(&text);
    let ast = parsed.ast();
    let diagnostics =
        parsed.diagnostics_with_functions(&profile, &FunctionRegistry::atelier_defaults());

    let mut nodes = Vec::new();
    let mut highlights = Vec::new();
    for span in ast.strengthening() {
        nodes.push(node("strengthening", *span, None));
        highlights.push(highlight("strengthening", *span, Some(1050)));
    }
    for span in ast.weakening() {
        nodes.push(node("weakening", *span, None));
        highlights.push(highlight("weakening", *span, Some(952)));
    }
    for item in ast.numeric_emphasis() {
        nodes.push(node(
            "numeric_emphasis",
            item.span,
            Some(item.weight.clone()),
        ));
        let weight_milli = parse_weight_milli(&item.weight);
        highlights.push(highlight("numeric_emphasis", item.span, weight_milli));
    }
    for item in ast.randomizers() {
        nodes.push(node("randomizer", item.span, None));
        highlights.push(highlight("randomizer", item.span, None));
    }
    for item in ast.pipes() {
        nodes.push(node("pipe", item.span, None));
        highlights.push(highlight("pipe", item.span, None));
    }
    for item in ast.extension_calls() {
        nodes.push(node("extension_call", item.span, Some(item.name.clone())));
        highlights.push(highlight("extension_call", item.span, None));
    }

    PromptAnalysisDto {
        source_text: text,
        profile: profile_dto,
        tokens: parsed
            .tokens()
            .iter()
            .map(|token| PromptTokenDto {
                kind: token_kind_name(token.kind).to_owned(),
                text: token.text.clone(),
                range: range(token.span),
            })
            .collect(),
        nodes,
        highlights,
        diagnostics: diagnostics
            .into_iter()
            .map(|diagnostic| PromptDiagnosticDto {
                code: diagnostic_code(diagnostic.kind).to_owned(),
                severity: diagnostic_severity(diagnostic.kind),
                message: diagnostic.message,
                hint: diagnostic_hint(diagnostic.kind).map(str::to_owned),
                range: range(diagnostic.span),
            })
            .collect(),
    }
}

fn syntax_profile_to_domain(value: PromptSyntaxProfileDto) -> PromptSyntaxProfile {
    match value {
        PromptSyntaxProfileDto::NovelaiV3 => PromptSyntaxProfile::novelai_v3(),
        PromptSyntaxProfileDto::NovelaiV4 => PromptSyntaxProfile::novelai_v4(),
        PromptSyntaxProfileDto::NovelaiV45 => PromptSyntaxProfile::novelai_v45(),
    }
}

fn node(kind: &str, span: PromptSpan, detail: Option<String>) -> PromptSyntaxNodeDto {
    PromptSyntaxNodeDto {
        kind: kind.to_owned(),
        range: range(span),
        detail,
    }
}

fn highlight(
    kind: &str,
    span: PromptSpan,
    effective_weight_milli: Option<i32>,
) -> PromptHighlightSpanDto {
    PromptHighlightSpanDto {
        kind: kind.to_owned(),
        range: range(span),
        effective_weight_milli,
    }
}

fn parse_weight_milli(value: &str) -> Option<i32> {
    let (negative, unsigned) = value
        .strip_prefix('-')
        .map_or((false, value), |rest| (true, rest));
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let whole = whole.parse::<i32>().ok()?;
    let fraction = format!("{fraction:0<3}")
        .chars()
        .take(3)
        .collect::<String>()
        .parse::<i32>()
        .ok()?;
    let milli = whole.checked_mul(1000)?.checked_add(fraction)?;
    Some(if negative { -milli } else { milli })
}

const fn range(span: PromptSpan) -> PromptTextRangeDto {
    PromptTextRangeDto {
        start_byte: span.start,
        end_byte: span.end,
    }
}

const fn token_kind_name(kind: PromptTokenKind) -> &'static str {
    match kind {
        PromptTokenKind::Whitespace => "whitespace",
        PromptTokenKind::Text => "text",
        PromptTokenKind::Identifier => "identifier",
        PromptTokenKind::Number => "number",
        PromptTokenKind::InvalidNumber => "invalid_number",
        PromptTokenKind::String => "string",
        PromptTokenKind::UnterminatedString => "unterminated_string",
        PromptTokenKind::Escaped => "escaped",
        PromptTokenKind::LBrace => "l_brace",
        PromptTokenKind::RBrace => "r_brace",
        PromptTokenKind::LBracket => "l_bracket",
        PromptTokenKind::RBracket => "r_bracket",
        PromptTokenKind::LParen => "l_paren",
        PromptTokenKind::RParen => "r_paren",
        PromptTokenKind::Comma => "comma",
        PromptTokenKind::Pipe => "pipe",
        PromptTokenKind::DoublePipe => "double_pipe",
        PromptTokenKind::Colon => "colon",
        PromptTokenKind::DoubleColon => "double_colon",
        PromptTokenKind::Dollar => "dollar",
        PromptTokenKind::Equals => "equals",
        PromptTokenKind::Error => "error",
    }
}

const fn diagnostic_severity(kind: PromptDiagnosticKind) -> PromptDiagnosticSeverityDto {
    match kind {
        PromptDiagnosticKind::UnclosedStrengthening
        | PromptDiagnosticKind::UnclosedWeakening
        | PromptDiagnosticKind::UnclosedNumericEmphasis
        | PromptDiagnosticKind::UnclosedRandomizer
        | PromptDiagnosticKind::UnclosedFunctionCall => PromptDiagnosticSeverityDto::Warning,
        _ => PromptDiagnosticSeverityDto::Error,
    }
}

const fn diagnostic_code(kind: PromptDiagnosticKind) -> &'static str {
    match kind {
        PromptDiagnosticKind::UnclosedStrengthening => "unclosed_strengthening",
        PromptDiagnosticKind::UnclosedWeakening => "unclosed_weakening",
        PromptDiagnosticKind::UnmatchedStrengtheningClose => "unmatched_strengthening_close",
        PromptDiagnosticKind::UnmatchedWeakeningClose => "unmatched_weakening_close",
        PromptDiagnosticKind::UnclosedNumericEmphasis => "unclosed_numeric_emphasis",
        PromptDiagnosticKind::UnclosedRandomizer => "unclosed_randomizer",
        PromptDiagnosticKind::EmptyRandomizerOption => "empty_randomizer_option",
        PromptDiagnosticKind::UnclosedFunctionCall => "unclosed_function_call",
        PromptDiagnosticKind::InvalidNumericWeight => "invalid_numeric_weight",
        PromptDiagnosticKind::UnterminatedString => "unterminated_string",
        PromptDiagnosticKind::UnsupportedCapability => "unsupported_capability",
        PromptDiagnosticKind::AmbiguousPipe => "ambiguous_pipe",
        PromptDiagnosticKind::UnknownFunction => "unknown_function",
        PromptDiagnosticKind::InvalidFunctionArity => "invalid_function_arity",
        PromptDiagnosticKind::InvalidFunctionArgument => "invalid_function_argument",
    }
}

const fn diagnostic_hint(kind: PromptDiagnosticKind) -> Option<&'static str> {
    match kind {
        PromptDiagnosticKind::UnclosedStrengthening => Some("close the block with `}` or `::`"),
        PromptDiagnosticKind::UnclosedWeakening => Some("close the block with `]` or `::`"),
        PromptDiagnosticKind::UnclosedNumericEmphasis => Some("close numeric emphasis with `::`"),
        PromptDiagnosticKind::UnclosedRandomizer => Some("close the randomizer with `||`"),
        PromptDiagnosticKind::UnclosedFunctionCall => Some("close the extension call with `)`"),
        PromptDiagnosticKind::EmptyRandomizerOption => {
            Some("add text between adjacent pipe separators")
        }
        _ => None,
    }
}

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

pub fn prompt_preset_to_dto(preset: &PromptPreset) -> PromptPresetDto {
    PromptPresetDto {
        preset_id: preset.id.as_str().to_owned(),
        kind: prompt_preset_kind_to_dto(preset.kind),
        name: preset.name.clone(),
        category: preset.category.clone(),
        description: preset.description.clone(),
        order: preset.order,
        enabled: preset.enabled,
        before: preset.before.clone(),
        after: preset.after.clone(),
        replace: preset.replace.clone(),
        uc_before: preset.uc_before.clone(),
        uc_after: preset.uc_after.clone(),
        uc_replace: preset.uc_replace.clone(),
        quality_override: preset.quality_override.clone(),
        uc_preset_override: preset.uc_preset_override.clone(),
        preview: preset.preview_thumb.as_ref().map(resource_ref_to_dto),
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
        enabled: request.enabled,
        before: request.before,
        after: request.after,
        replace: request.replace,
        uc_before: request.uc_before,
        uc_after: request.uc_after,
        uc_replace: request.uc_replace,
        quality_override: request.quality_override,
        uc_preset_override: request.uc_preset_override,
        preview_thumb: request.preview.map(resource_ref_from_dto),
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
