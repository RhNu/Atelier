use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use image::{DynamicImage, ImageFormat, RgbImage, codecs::jpeg::JpegEncoder, imageops};
use nai_atelier_safety::{
    SafetyAssessment, SafetyError, SafetyModelScore, SafetyResult, SafetyScanInput, SafetyScanner,
};
use ort::{session::Session, value::Tensor};

pub const OPEN_NSFW_MODEL_ID: &str = "open_nsfw@onnx";

const PREPROCESS_RESIZE: u32 = 256;
const MODEL_INPUT_SIZE: u32 = 224;
const JPEG_QUALITY: u8 = 75;
const BGR_MEAN: [f32; 3] = [104.0, 117.0, 123.0];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NsfwRuntimeAssets {
    pub model_path: PathBuf,
    pub runtime_library_path: Option<PathBuf>,
}

/// Builds a scanner from host-provided ONNX assets.
///
/// # Errors
/// Returns an error when a configured model or runtime path is missing, or when
/// ONNX Runtime cannot load the model.
pub fn build_safety_scanner(
    assets: Option<NsfwRuntimeAssets>,
) -> SafetyResult<Option<Arc<dyn SafetyScanner>>> {
    let Some(assets) = assets else {
        return Ok(None);
    };
    OrtNsfwScanner::load(assets).map(|scanner| Some(Arc::new(scanner) as Arc<dyn SafetyScanner>))
}

pub struct OrtNsfwScanner {
    session: Mutex<Session>,
}

impl OrtNsfwScanner {
    /// Loads the `OpenNSFW` ONNX model using a host-provided ONNX Runtime library.
    ///
    /// # Errors
    /// Returns an error when paths are missing or ONNX Runtime initialization
    /// fails.
    pub fn load(assets: NsfwRuntimeAssets) -> SafetyResult<Self> {
        if !assets.model_path.exists() {
            return Err(SafetyError::scanner(format!(
                "NSFW model file missing: {}",
                assets.model_path.display()
            )));
        }
        let runtime_library_path = assets.runtime_library_path.ok_or_else(|| {
            SafetyError::scanner("ONNX Runtime library missing for NSFW detector")
        })?;
        if !runtime_library_path.exists() {
            return Err(SafetyError::scanner(format!(
                "ONNX Runtime library missing: {}",
                runtime_library_path.display()
            )));
        }

        let _ = ort::init_from(&runtime_library_path)
            .map_err(ort_error_to_safety)?
            .commit();
        let session = Session::builder()
            .map_err(ort_error_to_safety)?
            .commit_from_file(&assets.model_path)
            .map_err(ort_error_to_safety)?;
        Ok(Self {
            session: Mutex::new(session),
        })
    }

    fn preprocess(bytes: &[u8]) -> SafetyResult<Vec<f32>> {
        let decoded = image::load_from_memory(bytes)
            .map_err(|error| SafetyError::scanner(format!("failed to decode image: {error}")))?;
        let resized = imageops::resize(
            &decoded.to_rgb8(),
            PREPROCESS_RESIZE,
            PREPROCESS_RESIZE,
            imageops::FilterType::Triangle,
        );
        let offset = (PREPROCESS_RESIZE - MODEL_INPUT_SIZE) / 2;
        let cropped =
            imageops::crop_imm(&resized, offset, offset, MODEL_INPUT_SIZE, MODEL_INPUT_SIZE)
                .to_image();
        let normalized = jpeg_roundtrip(cropped)?;
        let plane_len = (MODEL_INPUT_SIZE * MODEL_INPUT_SIZE) as usize;
        let mut tensor = vec![0.0; plane_len * 3];
        for (idx, pixel) in normalized.pixels().enumerate() {
            let [r, g, b] = pixel.0;
            tensor[idx] = f32::from(b) - BGR_MEAN[0];
            tensor[plane_len + idx] = f32::from(g) - BGR_MEAN[1];
            tensor[(plane_len * 2) + idx] = f32::from(r) - BGR_MEAN[2];
        }
        Ok(tensor)
    }
}

#[async_trait]
impl SafetyScanner for OrtNsfwScanner {
    async fn scan_image(&self, input: SafetyScanInput) -> SafetyResult<SafetyAssessment> {
        let tensor = Self::preprocess(&input.bytes)?;
        let input_tensor = Tensor::from_array((
            [
                1_usize,
                3_usize,
                MODEL_INPUT_SIZE as usize,
                MODEL_INPUT_SIZE as usize,
            ],
            tensor.into_boxed_slice(),
        ))
        .map_err(ort_error_to_safety)?;
        let mut session_guard = self
            .session
            .lock()
            .map_err(|_| SafetyError::scanner("failed to lock ONNX safety session"))?;
        let outputs = session_guard
            .run(ort::inputs![input_tensor])
            .map_err(ort_error_to_safety)?;
        let (_, output_values) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(ort_error_to_safety)?;
        let values = output_values.to_vec();
        drop(outputs);
        drop(session_guard);
        scores_to_assessment(input.resource, &values)
    }
}

fn scores_to_assessment(
    resource: nai_atelier_resource_catalog::ResourceRef,
    values: &[f32],
) -> SafetyResult<SafetyAssessment> {
    let nsfw = values
        .get(1)
        .copied()
        .or_else(|| values.last().copied())
        .ok_or_else(|| SafetyError::scanner("NSFW model returned an empty output tensor"))?;
    let mut raw_scores = Vec::new();
    if values.len() > 1 {
        let safe = values[0];
        raw_scores.push(SafetyModelScore::new("safe", safe.clamp(0.0, 1.0))?);
    }
    raw_scores.push(SafetyModelScore::new("nsfw", nsfw.clamp(0.0, 1.0))?);
    SafetyAssessment::from_model_scores(resource, raw_scores).map(|assessment| {
        assessment.with_scorer(OPEN_NSFW_MODEL_ID, Some(env!("CARGO_PKG_VERSION")))
    })
}

fn jpeg_roundtrip(image: RgbImage) -> SafetyResult<RgbImage> {
    let mut jpeg_buffer = Vec::new();
    {
        let mut jpeg_writer = JpegEncoder::new_with_quality(&mut jpeg_buffer, JPEG_QUALITY);
        jpeg_writer
            .encode_image(&DynamicImage::ImageRgb8(image))
            .map_err(|error| {
                SafetyError::scanner(format!("failed to encode JPEG buffer: {error}"))
            })?;
    }
    image::load_from_memory_with_format(&jpeg_buffer, ImageFormat::Jpeg)
        .map(|decoded| decoded.to_rgb8())
        .map_err(|error| SafetyError::scanner(format!("failed to decode JPEG buffer: {error}")))
}

fn ort_error_to_safety(error: impl std::fmt::Display) -> SafetyError {
    SafetyError::scanner(format!("ONNX Runtime error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nai_atelier_resource_catalog::{ResourceId, ResourceRef};

    #[test]
    fn build_scanner_returns_none_without_assets() {
        assert!(build_safety_scanner(None).unwrap().is_none());
    }

    #[test]
    fn missing_model_file_returns_scanner_error() {
        let temp = tempfile::tempdir().unwrap();
        let Err(error) = OrtNsfwScanner::load(NsfwRuntimeAssets {
            model_path: temp.path().join("missing.onnx"),
            runtime_library_path: Some(temp.path().join(runtime_library_file_name())),
        }) else {
            panic!("missing model should not load");
        };

        assert_eq!(error.kind(), nai_atelier_safety::SafetyErrorKind::Scanner);
    }

    #[test]
    fn preprocess_produces_open_nsfw_input_tensor() {
        let tensor = OrtNsfwScanner::preprocess(&sample_png()).unwrap();

        assert_eq!(
            tensor.len(),
            (MODEL_INPUT_SIZE * MODEL_INPUT_SIZE * 3) as usize
        );
        assert!(tensor.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn preprocess_uses_nchw_bgr_plane_order() {
        let tensor = OrtNsfwScanner::preprocess(&solid_png([20, 40, 80])).unwrap();
        let plane_len = (MODEL_INPUT_SIZE * MODEL_INPUT_SIZE) as usize;

        assert!((tensor[1] - tensor[0]).abs() < 1.0);
        assert!(tensor[0] > tensor[plane_len] + 20.0);
        assert!(tensor[plane_len] > tensor[plane_len * 2] + 10.0);
    }

    #[test]
    fn score_mapping_preserves_safe_and_nsfw_outputs() {
        let assessment = scores_to_assessment(
            ResourceRef::base(ResourceId::new("resource:1")),
            &[0.12, 0.88],
        )
        .unwrap();

        assert_score(assessment.safe_score.unwrap().value(), 0.12);
        assert_score(assessment.score.value(), 0.88);
        assert_eq!(assessment.raw_scores.len(), 2);
        assert_eq!(assessment.scorer_label.as_deref(), Some(OPEN_NSFW_MODEL_ID));
    }

    #[test]
    fn score_mapping_does_not_invent_safe_score_for_single_output_models() {
        let assessment =
            scores_to_assessment(ResourceRef::base(ResourceId::new("resource:1")), &[0.88])
                .unwrap();

        assert!(assessment.safe_score.is_none());
        assert_score(assessment.score.value(), 0.88);
        assert_eq!(assessment.raw_scores.len(), 1);
        assert_eq!(assessment.raw_scores[0].label, "nsfw");
    }

    fn assert_score(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn bundled_onnx_smoke_test() {
        if std::env::var_os("NAI_ATELIER_RUN_SAFETY_ONNX_SMOKE").is_none() {
            return;
        }
        let model_path = std::env::var_os("NAI_ATELIER_SAFETY_ONNX_MODEL")
            .map(PathBuf::from)
            .expect("NAI_ATELIER_SAFETY_ONNX_MODEL must point to open-nsfw.onnx");
        let runtime_library_path = std::env::var_os("NAI_ATELIER_ONNX_RUNTIME")
            .map(PathBuf::from)
            .expect("NAI_ATELIER_ONNX_RUNTIME must point to the ONNX Runtime library");
        let scanner = OrtNsfwScanner::load(NsfwRuntimeAssets {
            model_path,
            runtime_library_path: Some(runtime_library_path),
        })
        .unwrap();
        let assessment = futures_executor::block_on(scanner.scan_image(SafetyScanInput {
            resource: ResourceRef::base(ResourceId::new("resource:smoke")),
            bytes: sample_png(),
            mime_type: Some("image/png".to_owned()),
        }))
        .unwrap();
        assert!((0.0..=1.0).contains(&assessment.score.value()));
    }

    fn sample_png() -> Vec<u8> {
        solid_png([224, 196, 190])
    }

    fn solid_png(rgb: [u8; 3]) -> Vec<u8> {
        let image = image::RgbImage::from_pixel(16, 16, image::Rgb(rgb));
        let mut output = Vec::new();
        DynamicImage::ImageRgb8(image)
            .write_to(&mut std::io::Cursor::new(&mut output), ImageFormat::Png)
            .unwrap();
        output
    }

    #[cfg(target_os = "windows")]
    const fn runtime_library_file_name() -> &'static str {
        "onnxruntime.dll"
    }

    #[cfg(target_os = "linux")]
    const fn runtime_library_file_name() -> &'static str {
        "libonnxruntime.so"
    }

    #[cfg(target_os = "macos")]
    const fn runtime_library_file_name() -> &'static str {
        "libonnxruntime.dylib"
    }
}
