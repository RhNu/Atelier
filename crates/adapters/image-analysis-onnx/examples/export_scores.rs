use std::path::{Path, PathBuf};

use atelier_adapter_downloadable_resources_fs::FileSystemDownloadableResourceManager;
use atelier_adapter_image_analysis_onnx::{OnnxImageAnalysisRuntime, initialize_ort_runtime};
use atelier_downloadable_resources::DownloadableResourceManager;
use atelier_image_analysis::{
    AnalysisOutputSelection, ImageAnalysisInput, ImageAnalysisModelId, ImageAnalyzer,
    ImageRatingScores,
};
use atelier_resource_catalog::{ResourceId, ResourceRef};
use atelier_safety::anime_rating_policy;
use futures_executor::block_on;

struct ScoredImage {
    primary: ImageRatingScores,
    primary_fused: f32,
    review: Option<ImageRatingScores>,
    review_fused: Option<f32>,
    final_sensitive: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 4 {
        return Err("usage: export_scores <dataset> <output.csv> <model-root> \
             <onnx-runtime-library> [--force-wd] [--limit=N]"
            .into());
    }
    let force_wd = args.iter().any(|value| value == "--force-wd");
    let limit = args
        .iter()
        .find_map(|value| value.strip_prefix("--limit="))
        .map(str::parse)
        .transpose()?;
    block_on(export(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
        force_wd,
        limit,
    ))
}

async fn export(
    dataset: &Path,
    output: &Path,
    model_root: &Path,
    runtime_path: &Path,
    force_wd: bool,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = initialize_ort_runtime(runtime_path)?;
    let catalog_url = std::env::var("ATELIER_RESOURCE_CATALOG_URL")?;
    let resources = FileSystemDownloadableResourceManager::new(model_root, catalog_url, "")?;
    resources.install("anime-dbrating", None).await?;
    resources.install("wd-swinv2-tagger-v3", None).await?;
    let analyzer = OnnxImageAnalysisRuntime::new(runtime, runtime_path, resources)?;

    let mut images = image_paths(dataset)?;
    images.sort();
    if let Some(limit) = limit {
        images.truncate(limit);
    }
    let mut writer = csv::Writer::from_path(output)?;
    writer.write_record([
        "path",
        "label",
        "dbrating_general",
        "dbrating_sensitive",
        "dbrating_questionable",
        "dbrating_explicit",
        "primary_fused",
        "review_called",
        "wd_general",
        "wd_sensitive",
        "wd_questionable",
        "wd_explicit",
        "review_fused",
        "final_sensitive",
    ])?;

    let mut review_count = 0_usize;
    for (index, path) in images.iter().enumerate() {
        let scored = score_image(&analyzer, path, index, force_wd).await?;
        if scored.review.is_some() {
            review_count += 1;
        }
        let relative = path
            .strip_prefix(dataset)?
            .to_string_lossy()
            .replace('\\', "/");
        let label = path
            .strip_prefix(dataset)?
            .components()
            .next()
            .and_then(|value| value.as_os_str().to_str())
            .ok_or("image is not below a label directory")?;
        writer.write_record([
            relative,
            label.to_owned(),
            scored.primary.general.value().to_string(),
            scored.primary.sensitive.value().to_string(),
            scored.primary.questionable.value().to_string(),
            scored.primary.explicit.value().to_string(),
            scored.primary_fused.to_string(),
            scored.review.is_some().to_string(),
            scored
                .review
                .map(|scores| scores.general.value().to_string())
                .unwrap_or_default(),
            scored
                .review
                .map(|scores| scores.sensitive.value().to_string())
                .unwrap_or_default(),
            scored
                .review
                .map(|scores| scores.questionable.value().to_string())
                .unwrap_or_default(),
            scored
                .review
                .map(|scores| scores.explicit.value().to_string())
                .unwrap_or_default(),
            scored
                .review_fused
                .map(|score| score.to_string())
                .unwrap_or_default(),
            scored.final_sensitive.to_string(),
        ])?;
        if (index + 1) % 25 == 0 || index + 1 == images.len() {
            eprintln!("scored {}/{}", index + 1, images.len());
        }
    }
    writer.flush()?;
    let review_percent = if images.is_empty() {
        0.0
    } else {
        f64::from(u32::try_from(review_count)?) / f64::from(u32::try_from(images.len())?) * 100.0
    };
    eprintln!(
        "review calls: {review_count}/{} ({:.4}%)",
        images.len(),
        review_percent
    );
    Ok(())
}

async fn score_image(
    analyzer: &OnnxImageAnalysisRuntime,
    path: &Path,
    index: usize,
    force_wd: bool,
) -> Result<ScoredImage, Box<dyn std::error::Error>> {
    let input = ImageAnalysisInput {
        resource: ResourceRef::base(ResourceId::new(format!("benchmark-{index}"))),
        bytes: std::fs::read(path)?,
        mime_type: None,
    };
    let primary = analyzer
        .analyze(
            ImageAnalysisModelId::AnimeDbRating,
            input.clone(),
            AnalysisOutputSelection::ratings_only(),
        )
        .await?
        .ratings
        .ok_or("dbrating did not return ratings")?;
    let policy = anime_rating_policy();
    let primary_fused = policy.primary_score(primary);
    let review = if force_wd || policy.should_review(primary_fused) {
        analyzer
            .analyze(
                ImageAnalysisModelId::WdSwinv2TaggerV3,
                input,
                AnalysisOutputSelection::ratings_only(),
            )
            .await?
            .ratings
    } else {
        None
    };
    let review_fused = review.map(|scores| policy.review_score(scores));
    Ok(ScoredImage {
        primary,
        primary_fused,
        review,
        review_fused,
        final_sensitive: primary_fused >= policy.primary_threshold
            || review_fused.is_some_and(|score| score >= policy.review_threshold),
    })
}

fn image_paths(dataset: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut images = Vec::new();
    for label in ["sfw", "nsfw"] {
        for entry in std::fs::read_dir(dataset.join(label))? {
            let path = entry?.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| {
                        matches!(
                            value.to_ascii_lowercase().as_str(),
                            "png" | "jpg" | "jpeg" | "webp"
                        )
                    })
            {
                images.push(path);
            }
        }
    }
    Ok(images)
}
