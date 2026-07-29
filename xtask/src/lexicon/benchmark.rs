use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use atelier_adapter_lexicon_bundle::{LexiconBundle, LexiconBundleManifest};
use atelier_prompt_lexicon::{
    LexiconEngine, LexiconSearchFilters, LexiconSearchMode, LexiconSearchQuery,
};
use serde::Deserialize;

const MAX_BUNDLE_BYTES: u64 = 300 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct LexiconBenchmarkConfig {
    pub queries: PathBuf,
    pub candidate_run: PathBuf,
    pub baseline_run: PathBuf,
    pub bundle: PathBuf,
    pub runtime_library: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexiconBenchmarkSummary {
    pub query_count: usize,
    pub candidate_ndcg_10: f64,
    pub baseline_ndcg_10: f64,
    pub relative_quality: f64,
    pub bundle_bytes: u64,
    pub completion_p95: Duration,
    pub lexical_p95: Duration,
    pub semantic_first: Duration,
    pub semantic_p95: Duration,
}

#[derive(Deserialize)]
struct JudgedQuery {
    id: String,
    query: String,
    locale: String,
    slice: String,
    #[serde(default)]
    entity_kind: Option<String>,
    relevance: HashMap<String, u8>,
}

#[derive(Deserialize)]
struct SearchRun {
    id: String,
    results: Vec<String>,
}

/// Compares a compact semantic model run with the pinned BGE-M3 baseline.
///
/// # Errors
/// Returns an error if inputs are incomplete or the 95% NDCG@10 gate fails.
pub fn benchmark_lexicon(
    config: &LexiconBenchmarkConfig,
) -> Result<LexiconBenchmarkSummary, String> {
    let queries = read_jsonl::<JudgedQuery>(&config.queries)?;
    let candidate = read_jsonl::<SearchRun>(&config.candidate_run)?
        .into_iter()
        .map(|run| (run.id, run.results))
        .collect::<HashMap<_, _>>();
    let baseline = read_jsonl::<SearchRun>(&config.baseline_run)?
        .into_iter()
        .map(|run| (run.id, run.results))
        .collect::<HashMap<_, _>>();
    if queries.is_empty() {
        return Err("benchmark query set is empty".to_owned());
    }
    validate_language_mix(&queries)?;
    let (candidate_ndcg_10, baseline_ndcg_10, relative_quality) =
        score_quality(&queries, &candidate, &baseline)?;
    let performance = benchmark_runtime(config, &queries)?;
    Ok(LexiconBenchmarkSummary {
        query_count: queries.len(),
        candidate_ndcg_10,
        baseline_ndcg_10,
        relative_quality,
        bundle_bytes: performance.bundle_bytes,
        completion_p95: performance.completion_p95,
        lexical_p95: performance.lexical_p95,
        semantic_first: performance.semantic_first,
        semantic_p95: performance.semantic_p95,
    })
}

fn score_quality(
    queries: &[JudgedQuery],
    candidate: &HashMap<String, Vec<String>>,
    baseline: &HashMap<String, Vec<String>>,
) -> Result<(f64, f64, f64), String> {
    let mut candidate_total = 0.0;
    let mut baseline_total = 0.0;
    let mut slices: HashMap<String, (f64, f64, u32)> = HashMap::new();
    let mut canonical_hits = 0_u32;
    let mut canonical_count = 0_u32;
    let mut alias_hits = 0_u32;
    let mut alias_count = 0_u32;
    for query in queries {
        let candidate_results = candidate
            .get(&query.id)
            .ok_or_else(|| format!("candidate run is missing query {}", query.id))?;
        let baseline_results = baseline
            .get(&query.id)
            .ok_or_else(|| format!("baseline run is missing query {}", query.id))?;
        let candidate_score = ndcg_at_10(candidate_results, &query.relevance);
        let baseline_score = ndcg_at_10(baseline_results, &query.relevance);
        candidate_total += candidate_score;
        baseline_total += baseline_score;
        record_slice(
            &mut slices,
            format!("locale:{}", query.locale),
            candidate_score,
            baseline_score,
        );
        record_slice(
            &mut slices,
            format!("slice:{}", query.slice),
            candidate_score,
            baseline_score,
        );
        if let Some(kind) = &query.entity_kind {
            record_slice(
                &mut slices,
                format!("entity:{kind}"),
                candidate_score,
                baseline_score,
            );
        }
        if query.slice.contains("canonical") {
            canonical_count += 1;
            canonical_hits += u32::from(recall_hit(candidate_results, &query.relevance, 1));
        }
        if query.slice.contains("alias") || query.slice.contains("translation") {
            alias_count += 1;
            alias_hits += u32::from(recall_hit(candidate_results, &query.relevance, 5));
        }
    }
    let count = f64::from(
        u32::try_from(queries.len())
            .map_err(|_| "benchmark query set is too large to score".to_owned())?,
    );
    let candidate_ndcg_10 = candidate_total / count;
    let baseline_ndcg_10 = baseline_total / count;
    let relative_quality = if baseline_ndcg_10 <= f64::EPSILON {
        1.0
    } else {
        candidate_ndcg_10 / baseline_ndcg_10
    };
    if relative_quality + f64::EPSILON < 0.95 {
        return Err(format!(
            "semantic quality gate failed: candidate {:.4}, baseline {:.4}, relative {:.2}%",
            candidate_ndcg_10,
            baseline_ndcg_10,
            relative_quality * 100.0
        ));
    }
    validate_slices(&slices)?;
    validate_recall(
        "canonical exact Recall@1",
        canonical_hits,
        canonical_count,
        1.0,
    )?;
    validate_recall("alias/translation Recall@5", alias_hits, alias_count, 0.99)?;
    Ok((candidate_ndcg_10, baseline_ndcg_10, relative_quality))
}

struct RuntimeSummary {
    bundle_bytes: u64,
    completion_p95: Duration,
    lexical_p95: Duration,
    semantic_first: Duration,
    semantic_p95: Duration,
}

fn benchmark_runtime(
    config: &LexiconBenchmarkConfig,
    queries: &[JudgedQuery],
) -> Result<RuntimeSummary, String> {
    LexiconBundleManifest::read(&config.bundle).map_err(|error| error.to_string())?;
    let bundle_bytes = bundle_size(&config.bundle)?;
    if bundle_bytes > MAX_BUNDLE_BYTES {
        return Err(format!(
            "bundle size gate failed: {bundle_bytes} bytes exceeds {MAX_BUNDLE_BYTES}"
        ));
    }
    atelier_adapter_onnx_runtime::initialize(&config.runtime_library)
        .map_err(|error| error.to_string())?;
    let lexicon = LexiconBundle::open(&config.bundle).map_err(|error| error.to_string())?;
    let mut completion_times = Vec::with_capacity(queries.len());
    let mut lexical_times = Vec::with_capacity(queries.len());
    for query in queries {
        let started = Instant::now();
        lexicon
            .complete(&query.query, 20)
            .map_err(|error| error.to_string())?;
        completion_times.push(started.elapsed());
        let started = Instant::now();
        timed_search(&lexicon, &query.query, LexiconSearchMode::Lexical)?;
        lexical_times.push(started.elapsed());
    }
    let started = Instant::now();
    timed_search(&lexicon, &queries[0].query, LexiconSearchMode::Semantic)?;
    let semantic_first = started.elapsed();
    let mut semantic_times = Vec::with_capacity(queries.len().saturating_sub(1));
    for query in &queries[1..] {
        let started = Instant::now();
        timed_search(&lexicon, &query.query, LexiconSearchMode::Semantic)?;
        semantic_times.push(started.elapsed());
    }
    let completion_p95 = percentile_95(&mut completion_times);
    let lexical_p95 = percentile_95(&mut lexical_times);
    let semantic_p95 = percentile_95(&mut semantic_times);
    check_duration(
        "completion warmed p95",
        completion_p95,
        Duration::from_millis(50),
    )?;
    check_duration(
        "lexical warmed p95",
        lexical_p95,
        Duration::from_millis(120),
    )?;
    check_duration(
        "first semantic query",
        semantic_first,
        Duration::from_secs(3),
    )?;
    check_duration(
        "semantic warmed p95",
        semantic_p95,
        Duration::from_millis(400),
    )?;
    Ok(RuntimeSummary {
        bundle_bytes,
        completion_p95,
        lexical_p95,
        semantic_first,
        semantic_p95,
    })
}

fn timed_search(
    lexicon: &LexiconBundle,
    text: &str,
    mode: LexiconSearchMode,
) -> Result<(), String> {
    lexicon
        .search(&LexiconSearchQuery {
            text: text.to_owned(),
            mode,
            filters: LexiconSearchFilters::default(),
            selected_entity_ids: Vec::new(),
            offset: 0,
            limit: 10,
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn bundle_size(bundle: &PathBuf) -> Result<u64, String> {
    fs::read_dir(bundle)
        .map_err(|error| error.to_string())?
        .try_fold(0_u64, |total, entry| {
            let entry = entry.map_err(|error| error.to_string())?;
            let metadata = entry.metadata().map_err(|error| error.to_string())?;
            total
                .checked_add(metadata.len())
                .ok_or_else(|| "bundle size overflow".to_owned())
        })
}

fn validate_language_mix(queries: &[JudgedQuery]) -> Result<(), String> {
    if queries.len() < 2 {
        return Err("benchmark requires at least two judged queries".to_owned());
    }
    let count =
        u32::try_from(queries.len()).map_err(|_| "benchmark query set is too large".to_owned())?;
    let chinese = u32::try_from(
        queries
            .iter()
            .filter(|query| query.locale.starts_with("zh") || query.locale == "mixed")
            .count(),
    )
    .map_err(|_| "benchmark query set is too large".to_owned())?;
    let ratio = f64::from(chinese) / f64::from(count);
    if !(0.5..=0.7).contains(&ratio) {
        return Err(format!(
            "benchmark language mix must be approximately 60% Chinese/mixed; found {:.2}%",
            ratio * 100.0
        ));
    }
    Ok(())
}

fn record_slice(
    slices: &mut HashMap<String, (f64, f64, u32)>,
    key: String,
    candidate: f64,
    baseline: f64,
) {
    let entry = slices.entry(key).or_default();
    entry.0 += candidate;
    entry.1 += baseline;
    entry.2 += 1;
}

fn validate_slices(slices: &HashMap<String, (f64, f64, u32)>) -> Result<(), String> {
    for (name, (candidate, baseline, count)) in slices {
        if *count == 0 || *baseline <= f64::EPSILON {
            continue;
        }
        let relative = candidate / baseline;
        if relative + f64::EPSILON < 0.9 {
            return Err(format!(
                "{name} quality slice is {:.2}% of BGE-M3; minimum is 90%",
                relative * 100.0
            ));
        }
    }
    Ok(())
}

fn recall_hit(results: &[String], relevance: &HashMap<String, u8>, limit: usize) -> bool {
    results
        .iter()
        .take(limit)
        .any(|item| relevance.get(item).copied().unwrap_or(0) > 0)
}

fn validate_recall(label: &str, hits: u32, count: u32, minimum: f64) -> Result<(), String> {
    if count == 0 {
        return Err(format!("benchmark has no queries for {label}"));
    }
    let recall = f64::from(hits) / f64::from(count);
    if recall + f64::EPSILON < minimum {
        return Err(format!(
            "{label} gate failed: {:.2}% is below {:.2}%",
            recall * 100.0,
            minimum * 100.0
        ));
    }
    Ok(())
}

fn percentile_95(values: &mut [Duration]) -> Duration {
    if values.is_empty() {
        return Duration::ZERO;
    }
    values.sort_unstable();
    let index = values
        .len()
        .saturating_mul(95)
        .div_ceil(100)
        .saturating_sub(1);
    values[index]
}

fn check_duration(label: &str, actual: Duration, maximum: Duration) -> Result<(), String> {
    if actual > maximum {
        return Err(format!(
            "{label} gate failed: {:.2} ms exceeds {:.2} ms",
            actual.as_secs_f64() * 1_000.0,
            maximum.as_secs_f64() * 1_000.0
        ));
    }
    Ok(())
}

fn ndcg_at_10(results: &[String], relevance: &HashMap<String, u8>) -> f64 {
    let dcg = results
        .iter()
        .take(10)
        .enumerate()
        .map(|(index, item)| {
            let relevance = f64::from(*relevance.get(item).unwrap_or(&0));
            (relevance.exp2() - 1.0) / ndcg_discount(index)
        })
        .sum::<f64>();
    let mut ideal = relevance.values().copied().collect::<Vec<_>>();
    ideal.sort_unstable_by(|left, right| right.cmp(left));
    let idcg = ideal
        .into_iter()
        .take(10)
        .enumerate()
        .map(|(index, relevance)| (f64::from(relevance).exp2() - 1.0) / ndcg_discount(index))
        .sum::<f64>();
    if idcg <= f64::EPSILON {
        0.0
    } else {
        dcg / idcg
    }
}

fn ndcg_discount(index: usize) -> f64 {
    f64::from(u32::try_from(index.saturating_add(2)).unwrap_or(u32::MAX)).log2()
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Result<Vec<T>, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line)
                .map_err(|error| format!("{} line {}: {error}", path.display(), index + 1))
        })
        .collect()
}
