use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use atelier_image_analysis::{ImageAnalysisError, ImageAnalysisResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::spec::{ModelFileSpec, ModelSpec};

const MANIFEST_FORMAT: &str = "atelier.image-analysis.model-pack";
const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize, Serialize)]
struct InstalledManifest {
    format: String,
    schema_version: u32,
    model_id: String,
    revision: String,
    files: Vec<InstalledFile>,
}

#[derive(Debug, Deserialize, Serialize)]
struct InstalledFile {
    path: String,
    sha256: String,
    size_bytes: u64,
}

pub fn write_manifest(path: &Path, spec: &ModelSpec) -> ImageAnalysisResult<()> {
    let manifest = InstalledManifest {
        format: MANIFEST_FORMAT.to_owned(),
        schema_version: MANIFEST_SCHEMA_VERSION,
        model_id: spec.id.as_str().to_owned(),
        revision: spec.revision.to_owned(),
        files: spec
            .files
            .iter()
            .map(|file| InstalledFile {
                path: file.relative_path.to_owned(),
                sha256: file.sha256.to_owned(),
                size_bytes: file.size_bytes,
            })
            .collect(),
    };
    let parent = path
        .parent()
        .ok_or_else(|| ImageAnalysisError::model_install("manifest path has no parent"))?;
    fs::create_dir_all(parent).map_err(manifest_error)?;
    let temporary = path.with_extension("json.part");
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(manifest_error)?;
    let mut file = File::create(&temporary).map_err(manifest_error)?;
    file.write_all(&bytes).map_err(manifest_error)?;
    file.write_all(b"\n").map_err(manifest_error)?;
    file.sync_all().map_err(manifest_error)?;
    if path.exists() {
        fs::remove_file(path).map_err(manifest_error)?;
    }
    fs::rename(temporary, path).map_err(manifest_error)
}

pub fn read_and_validate_manifest(path: &Path, spec: &ModelSpec) -> ImageAnalysisResult<()> {
    let bytes = fs::read(path).map_err(manifest_error)?;
    let manifest: InstalledManifest = serde_json::from_slice(&bytes).map_err(manifest_error)?;
    if manifest.format != MANIFEST_FORMAT
        || manifest.schema_version != MANIFEST_SCHEMA_VERSION
        || manifest.model_id != spec.id.as_str()
        || manifest.revision != spec.revision
        || manifest.files.len() != spec.files.len()
    {
        return Err(ImageAnalysisError::model_unavailable(
            "installed model manifest does not match the pinned model",
        ));
    }
    let root = path
        .parent()
        .ok_or_else(|| ImageAnalysisError::model_unavailable("manifest path has no parent"))?;
    for expected in spec.files {
        let Some(record) = manifest
            .files
            .iter()
            .find(|record| record.path == expected.relative_path)
        else {
            return Err(ImageAnalysisError::model_unavailable(
                "installed model manifest is missing a required file",
            ));
        };
        if record.sha256 != expected.sha256 || record.size_bytes != expected.size_bytes {
            return Err(ImageAnalysisError::model_unavailable(
                "installed model manifest contains unexpected file metadata",
            ));
        }
        let metadata = fs::metadata(root.join(expected.relative_path)).map_err(manifest_error)?;
        if metadata.len() != expected.size_bytes {
            return Err(ImageAnalysisError::model_unavailable(
                "installed model file has an unexpected size",
            ));
        }
        verify_file(root.join(expected.relative_path).as_path(), expected)?;
    }
    Ok(())
}

pub fn verify_file(path: &Path, spec: &ModelFileSpec) -> ImageAnalysisResult<()> {
    let mut file = File::open(path).map_err(manifest_error)?;
    if file.metadata().map_err(manifest_error)?.len() != spec.size_bytes {
        return Err(ImageAnalysisError::model_install(
            "model file size does not match its manifest",
        ));
    }
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(manifest_error)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    if format!("{:x}", digest.finalize()) != spec.sha256 {
        return Err(ImageAnalysisError::model_install(
            "model file SHA-256 does not match its manifest",
        ));
    }
    Ok(())
}

fn manifest_error(error: impl std::fmt::Display) -> ImageAnalysisError {
    ImageAnalysisError::model_install(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_image_analysis::ImageAnalysisModelId;

    #[test]
    fn validation_detects_same_size_file_corruption() {
        let temp = tempfile::tempdir().unwrap();
        let bytes = b"verified model bytes";
        let digest = format!("{:x}", Sha256::digest(bytes));
        let digest = Box::leak(digest.into_boxed_str());
        let files = Box::leak(
            vec![ModelFileSpec {
                relative_path: "model.onnx",
                url: "https://example.invalid/model.onnx",
                sha256: digest,
                size_bytes: bytes.len() as u64,
            }]
            .into_boxed_slice(),
        );
        let spec = ModelSpec {
            id: ImageAnalysisModelId::AnimeDbRating,
            revision: "test-revision",
            required: true,
            files,
        };
        let model_path = temp.path().join("model.onnx");
        let manifest_path = temp.path().join("manifest.json");
        fs::write(&model_path, bytes).unwrap();
        write_manifest(&manifest_path, &spec).unwrap();
        read_and_validate_manifest(&manifest_path, &spec).unwrap();

        fs::write(&model_path, vec![b'x'; bytes.len()]).unwrap();

        assert!(read_and_validate_manifest(&manifest_path, &spec).is_err());
    }
}
