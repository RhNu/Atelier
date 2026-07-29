use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
pub use atelier_adapter_onnx_runtime::OrtRuntime;
use atelier_safety::{
    SafetyAssessment, SafetyError, SafetyModelScore, SafetyResult, SafetyScanInput, SafetyScanner,
};
use image::{DynamicImage, ImageFormat, RgbImage, codecs::jpeg::JpegEncoder, imageops};
use ort::{session::Session, value::Tensor};

pub const OPEN_NSFW_MODEL_ID: &str = "open_nsfw@onnx";

const PREPROCESS_RESIZE: u32 = 256;
const MODEL_INPUT_SIZE: u32 = 224;
const JPEG_QUALITY: u8 = 75;
const BGR_MEAN: [f32; 3] = [104.0, 117.0, 123.0];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NsfwRuntimeAssets {
    pub model_path: PathBuf,
    pub runtime_library_path: PathBuf,
}

/// Initializes the process-global ONNX Runtime from a host-selected library.
///
/// Repeated initialization with the same canonical path is idempotent. A
/// different path is rejected because `ort` cannot replace a committed runtime
/// in the same process.
///
/// # Errors
/// Returns an error when the library is missing, cannot be loaded, conflicts
/// with the committed runtime, or another caller configured `ort` first.
pub fn initialize_ort_runtime(
    runtime_library_path: impl AsRef<Path>,
) -> SafetyResult<&'static OrtRuntime> {
    atelier_adapter_onnx_runtime::initialize(runtime_library_path)
        .map_err(|error| SafetyError::scanner(error.to_string()))
}

/// Builds a scanner from host-provided ONNX assets and an initialized runtime.
///
/// # Errors
/// Returns an error when the asset runtime does not match the initialized
/// runtime, or when ONNX Runtime cannot load the model.
pub fn build_safety_scanner(
    assets: &NsfwRuntimeAssets,
    runtime: &'static OrtRuntime,
) -> SafetyResult<Arc<dyn SafetyScanner>> {
    runtime
        .for_path(&assets.runtime_library_path)
        .map_err(|error| SafetyError::scanner(error.to_string()))?;
    OrtNsfwScanner::load(&assets.model_path)
        .map(|scanner| Arc::new(scanner) as Arc<dyn SafetyScanner>)
}

struct OrtNsfwScanner {
    session: Mutex<Session>,
}

impl OrtNsfwScanner {
    /// Loads the `OpenNSFW` model after the host has initialized ONNX Runtime.
    ///
    /// # Errors
    /// Returns an error when the model is missing or ONNX Runtime cannot create
    /// a session from it.
    fn load(model_path: &Path) -> SafetyResult<Self> {
        if !model_path.exists() {
            return Err(SafetyError::scanner(format!(
                "NSFW model file missing: {}",
                model_path.display()
            )));
        }
        let session = Session::builder()
            .map_err(ort_error_to_safety)?
            .commit_from_file(model_path)
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
        let pixel_count = (MODEL_INPUT_SIZE * MODEL_INPUT_SIZE) as usize;
        let mut tensor = Vec::with_capacity(pixel_count * 3);
        for pixel in normalized.pixels() {
            let [r, g, b] = pixel.0;
            tensor.push(f32::from(b) - BGR_MEAN[0]);
            tensor.push(f32::from(g) - BGR_MEAN[1]);
            tensor.push(f32::from(r) - BGR_MEAN[2]);
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
                MODEL_INPUT_SIZE as usize,
                MODEL_INPUT_SIZE as usize,
                3_usize,
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
    resource: atelier_resource_catalog::ResourceRef,
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
    use atelier_resource_catalog::{ResourceId, ResourceRef};

    #[test]
    fn missing_runtime_library_returns_scanner_error() {
        let temp = tempfile::tempdir().unwrap();
        let error = initialize_ort_runtime(temp.path().join(runtime_library_file_name()))
            .expect_err("missing runtime should not initialize");

        assert_eq!(error.kind(), atelier_safety::SafetyErrorKind::Scanner);
    }

    #[test]
    fn missing_model_file_returns_scanner_error() {
        let temp = tempfile::tempdir().unwrap();
        let Err(error) = OrtNsfwScanner::load(&temp.path().join("missing.onnx")) else {
            panic!("missing model should not load");
        };

        assert_eq!(error.kind(), atelier_safety::SafetyErrorKind::Scanner);
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
    fn preprocess_uses_nhwc_bgr_interleaved_order() {
        let tensor = OrtNsfwScanner::preprocess(&solid_png([20, 40, 80])).unwrap();

        assert!((tensor[3] - tensor[0]).abs() < 1.0);
        assert!(tensor[0] > tensor[1] + 20.0);
        assert!(tensor[1] > tensor[2] + 10.0);
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
    #[ignore = "loads the native ONNX Runtime; run in the dedicated safety smoke job"]
    fn bundled_onnx_smoke_test() {
        let Some(assets) = smoke_test_assets() else {
            return;
        };
        let runtimes = std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..8 {
                let runtime_library_path = &assets.runtime_library_path;
                handles.push(
                    scope.spawn(move || initialize_ort_runtime(runtime_library_path).unwrap()),
                );
            }
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        let runtime = runtimes[0];
        assert!(
            runtimes
                .iter()
                .all(|candidate| std::ptr::eq(*candidate, runtime))
        );

        let alternate_dir = tempfile::tempdir().unwrap();
        let alternate_runtime = alternate_dir.path().join(runtime_library_file_name());
        std::fs::copy(&assets.runtime_library_path, &alternate_runtime).unwrap();
        let conflict = initialize_ort_runtime(&alternate_runtime)
            .expect_err("a different runtime path must not replace the committed runtime");
        assert!(conflict.to_string().contains("already initialized"));

        let scanner = build_safety_scanner(&assets, runtime).unwrap();
        let assessment = futures_executor::block_on(scanner.scan_image(SafetyScanInput {
            resource: ResourceRef::base(ResourceId::new("resource:smoke")),
            bytes: sample_png(),
            mime_type: Some("image/png".to_owned()),
        }))
        .unwrap();
        assert!((0.0..=1.0).contains(&assessment.score.value()));
    }

    fn smoke_test_assets() -> Option<NsfwRuntimeAssets> {
        if std::env::var_os("ATELIER_RUN_SAFETY_ONNX_SMOKE").is_some() {
            let model_path = std::env::var_os("ATELIER_SAFETY_ONNX_MODEL")
                .map(PathBuf::from)
                .expect("ATELIER_SAFETY_ONNX_MODEL must point to open-nsfw.onnx");
            let runtime_library_path = std::env::var_os("ATELIER_ONNX_RUNTIME")
                .map(PathBuf::from)
                .expect("ATELIER_ONNX_RUNTIME must point to the ONNX Runtime library");
            return Some(NsfwRuntimeAssets {
                model_path,
                runtime_library_path,
            });
        }

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            let safety_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../apps/desktop/src-tauri/resources/safety");
            let model_path = safety_dir.join("open_nsfw.onnx");
            let runtime_library_path = safety_dir.join("onnxruntime.dll");
            if model_path.exists() && runtime_library_path.exists() {
                return Some(NsfwRuntimeAssets {
                    model_path,
                    runtime_library_path,
                });
            }
        }

        None
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
