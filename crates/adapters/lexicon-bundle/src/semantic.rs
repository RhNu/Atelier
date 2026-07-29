use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Mutex;

use atelier_prompt_lexicon::{
    LexiconError, LexiconResult, LexiconSearchFilters, LexiconSearchItem,
};
use half::f16;
use memmap2::{Mmap, MmapOptions};
use ort::{
    session::{Session, SessionInputValue, builder::GraphOptimizationLevel},
    value::Tensor,
};
use rayon::prelude::*;
use tokenizers::{Tokenizer, utils::truncation::TruncationParams};

use crate::manifest::{RankingManifest, SemanticManifest, SemanticModelContract};

pub struct SemanticEngine {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    identity: Mmap,
    knowledge: Mmap,
    rows: Vec<LexiconSearchItem>,
    dimensions: usize,
    max_length: usize,
    query_prefix: String,
    ranking: RankingManifest,
    contract: SemanticModelContract,
}

impl SemanticEngine {
    pub fn load(
        root: &Path,
        manifest: &SemanticManifest,
        ranking: RankingManifest,
        rows: Vec<LexiconSearchItem>,
    ) -> LexiconResult<Self> {
        if rows.len() != manifest.entity_count {
            return Err(LexiconError::invalid_bundle(format!(
                "semantic row count {} does not match manifest {}",
                rows.len(),
                manifest.entity_count
            )));
        }
        let mut tokenizer = Tokenizer::from_file(root.join(&manifest.tokenizer.file))
            .map_err(|error| LexiconError::SemanticUnavailable(error.to_string()))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: manifest.max_length,
                ..TruncationParams::default()
            }))
            .map_err(|error| LexiconError::SemanticUnavailable(error.to_string()))?;
        let session = Session::builder()
            .map_err(semantic_error)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(semantic_error)?
            .with_intra_threads(4)
            .map_err(semantic_error)?
            .commit_from_file(root.join(&manifest.model.file))
            .map_err(semantic_error)?;
        let identity = map_file(&root.join(&manifest.identity_vectors.file))?;
        let knowledge = map_file(&root.join(&manifest.knowledge_vectors.file))?;
        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            identity,
            knowledge,
            rows,
            dimensions: manifest.dimensions,
            max_length: manifest.max_length,
            query_prefix: manifest.model_contract.query_prefix.clone(),
            ranking,
            contract: manifest.model_contract.clone(),
        })
    }

    pub fn search(
        &self,
        text: &str,
        filters: &LexiconSearchFilters,
        context: &HashMap<u64, f32>,
        max_results: usize,
    ) -> LexiconResult<Vec<LexiconSearchItem>> {
        let query = self.embed(text)?;
        let dimensions = self.dimensions;
        let mut matches = self
            .rows
            .par_iter()
            .enumerate()
            .filter(|(_, item)| matches_filters(item, filters))
            .map(|(index, item)| {
                let identity = dot_f16(&self.identity, index, dimensions, &query);
                let knowledge = dot_f16(&self.knowledge, index, dimensions, &query);
                let semantic = identity.max(knowledge).mul_add(0.5, 0.5).clamp(0.0, 1.0);
                let context = context.get(&item.entity_id).copied().unwrap_or(0.0);
                let popularity = f32::from(
                    u16::try_from(item.post_count.checked_ilog2().unwrap_or(0)).unwrap_or(u16::MAX),
                ) / 26.0;
                let score = self.ranking.popularity_weight.mul_add(
                    popularity.clamp(0.0, 1.0),
                    self.ranking.context_weight.mul_add(
                        context.clamp(0.0, 1.0),
                        self.ranking.semantic_weight * semantic,
                    ),
                );
                let mut result = item.clone();
                result.score = score;
                result
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| right.post_count.cmp(&left.post_count))
                .then_with(|| left.canonical_name.cmp(&right.canonical_name))
        });
        matches.truncate(max_results);
        Ok(matches)
    }

    fn embed(&self, text: &str) -> LexiconResult<Vec<f32>> {
        let input = format!("{}{}", self.query_prefix, text.trim());
        let encoding = self
            .tokenizer
            .encode(input, true)
            .map_err(|error| LexiconError::SemanticUnavailable(error.to_string()))?;
        let length = encoding.len().min(self.max_length);
        let input_ids = encoding.get_ids()[..length]
            .iter()
            .map(|value| i64::from(*value))
            .collect::<Vec<_>>();
        let attention_values = encoding.get_attention_mask()[..length]
            .iter()
            .map(|value| i64::from(*value))
            .collect::<Vec<_>>();
        let token_type_ids = encoding.get_type_ids()[..length]
            .iter()
            .map(|value| i64::from(*value))
            .collect::<Vec<_>>();
        let input_ids = Tensor::from_array(([1, length], input_ids.into_boxed_slice()))
            .map_err(semantic_error)?;
        let attention_mask =
            Tensor::from_array(([1, length], attention_values.clone().into_boxed_slice()))
                .map_err(semantic_error)?;
        let token_type_ids = Tensor::from_array(([1, length], token_type_ids.into_boxed_slice()))
            .map_err(semantic_error)?;
        let mut session = self.session.lock().map_err(|_| {
            LexiconError::SemanticUnavailable("semantic session lock is unavailable".to_owned())
        })?;
        let mut inputs: Vec<(String, SessionInputValue<'_>)> = vec![
            (self.contract.input_ids.clone(), input_ids.into()),
            (self.contract.attention_mask.clone(), attention_mask.into()),
        ];
        if let Some(name) = &self.contract.token_type_ids {
            inputs.push((name.clone(), token_type_ids.into()));
        }
        let outputs = session.run(inputs).map_err(semantic_error)?;
        let output = self
            .contract
            .output_name
            .as_ref()
            .and_then(|name| outputs.get(name))
            .unwrap_or(&outputs[0]);
        let (shape, values) = output.try_extract_tensor::<f32>().map_err(semantic_error)?;
        let mut pooled = if shape.len() == 3 {
            mean_pool(values, length, self.dimensions, &attention_values)
        } else {
            values.iter().copied().take(self.dimensions).collect()
        };
        drop(outputs);
        drop(session);
        normalize(&mut pooled)?;
        Ok(pooled)
    }
}

fn mean_pool(values: &[f32], length: usize, dimensions: usize, mask: &[i64]) -> Vec<f32> {
    let mut pooled = vec![0.0; dimensions];
    let mut count = 0.0_f32;
    for (token, attention) in mask.iter().copied().enumerate().take(length) {
        if attention == 0 {
            continue;
        }
        count += 1.0;
        let start = token * dimensions;
        for (target, value) in pooled
            .iter_mut()
            .zip(values[start..start + dimensions].iter())
        {
            *target += *value;
        }
    }
    if count > 0.0 {
        for value in &mut pooled {
            *value /= count;
        }
    }
    pooled
}

fn normalize(values: &mut [f32]) -> LexiconResult<()> {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if !norm.is_finite() || norm <= f32::EPSILON {
        return Err(LexiconError::SemanticUnavailable(
            "semantic model returned an invalid embedding".to_owned(),
        ));
    }
    for value in values {
        *value /= norm;
    }
    Ok(())
}

fn dot_f16(bytes: &[u8], row: usize, dimensions: usize, query: &[f32]) -> f32 {
    let start = row * dimensions * 2;
    query
        .iter()
        .enumerate()
        .map(|(index, query_value)| {
            let offset = start + index * 2;
            let bits = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            f16::from_bits(bits).to_f32() * query_value
        })
        .sum()
}

fn matches_filters(item: &LexiconSearchItem, filters: &LexiconSearchFilters) -> bool {
    (filters.entity_kinds.is_empty() || filters.entity_kinds.contains(&item.kind))
        && (filters.categories.is_empty() || filters.categories.contains(&item.category))
        && (filters.ratings.is_empty() || filters.ratings.contains(&item.rating))
}

fn map_file(path: &Path) -> LexiconResult<Mmap> {
    let file = File::open(path).map_err(|error| {
        LexiconError::SemanticUnavailable(format!("failed to open {}: {error}", path.display()))
    })?;
    // SAFETY: files are immutable bundled resources and the mapping is retained for the engine
    // lifetime. The desktop never writes or replaces them while the process is running.
    unsafe { MmapOptions::new().map(&file) }.map_err(|error| {
        LexiconError::SemanticUnavailable(format!("failed to map {}: {error}", path.display()))
    })
}

fn semantic_error(error: impl std::fmt::Display) -> LexiconError {
    LexiconError::SemanticUnavailable(format!("ONNX semantic search error: {error}"))
}
