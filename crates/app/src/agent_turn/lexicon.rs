use atelier_agent::{AgentError, AgentResult, AgentToolSpec};
use atelier_app_api::prompt::{
    LexiconSearchFiltersDto, LexiconSearchModeDto, LexiconSearchRequestDto,
};
use serde::Deserialize;
use serde_json::json;

use super::{
    schemas::{object, spec},
    tools::{AgentTools, parse_args},
};

impl<S, F, E> AgentTools<S, F, E> {
    pub(super) fn get_lexicon_context(&self) -> AgentResult<String> {
        let value = self
            .lexicon
            .bootstrap()
            .map(crate::mapping::lexicon_bootstrap_to_dto)
            .map_err(|error| AgentError::runtime(error.to_string()))?;
        Ok(json!(value).to_string())
    }

    pub(super) fn search_lexicon(&self, arguments: &str) -> AgentResult<String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Args {
            query: String,
            mode: Option<LexiconSearchModeDto>,
            #[serde(default)]
            filters: LexiconSearchFiltersDto,
            #[serde(default)]
            selected_entity_ids: Vec<u64>,
            #[serde(default)]
            offset: usize,
            limit: Option<usize>,
        }
        let args: Args = parse_args(arguments)?;
        let limit = args.limit.unwrap_or(30);
        if !(1..=100).contains(&limit) {
            return Err(AgentError::validation("limit must be between 1 and 100"));
        }
        let query = crate::mapping::lexicon_search_query_to_domain(LexiconSearchRequestDto {
            query: args.query,
            mode: args.mode.unwrap_or(LexiconSearchModeDto::Lexical),
            filters: args.filters,
            selected_entity_ids: args.selected_entity_ids,
            offset: args.offset,
            limit,
        });
        let page = self
            .lexicon
            .search(&query)
            .map(crate::mapping::lexicon_page_to_dto)
            .map_err(|error| AgentError::runtime(error.to_string()))?;
        Ok(json!(page).to_string())
    }

    pub(super) fn get_lexicon_entity(&self, arguments: &str) -> AgentResult<String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Args {
            entity_id: u64,
        }
        let args: Args = parse_args(arguments)?;
        let value = self
            .lexicon
            .entity(args.entity_id)
            .map(crate::mapping::lexicon_detail_to_dto)
            .map_err(|error| AgentError::runtime(error.to_string()))?;
        Ok(json!(value).to_string())
    }
}

pub fn specs() -> Vec<AgentToolSpec> {
    vec![
        spec(
            "get_lexicon_context",
            "Read installed lexicon capabilities and available categories/groups. Semantic search may be unavailable.",
            object(json!({}), &[]),
        ),
        spec(
            "search_lexicon",
            "Search the installed tag/artist lexicon for canonical terms, translations and semantic suggestions. Defaults to lexical search. Optional filters require all four arrays; use empty arrays to leave facets unrestricted.",
            object(
                json!({
                    "query":{"type":"string"},"mode":{"enum":["lexical","semantic"]},
                    "filters":object(json!({"entity_kinds":{"type":"array","items":{"enum":["tag","artist"]}},"categories":{"type":"array","items":{"enum":["general","copyright","character","artist"]}},"group_ids":{"type":"array","items":{"type":"string"}},"ratings":{"type":"array","items":{"enum":["safe","sensitive","unknown"]}}}), &["entity_kinds","categories","group_ids","ratings"]),
                    "selected_entity_ids":{"type":"array","items":{"type":"integer","minimum":0}},"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":100}
                }),
                &["query"],
            ),
        ),
        spec(
            "get_lexicon_entity",
            "Read a lexicon entity's aliases, translations, wiki and related terms. This returns text only.",
            object(
                json!({"entity_id":{"type":"integer","minimum":0}}),
                &["entity_id"],
            ),
        ),
    ]
}
