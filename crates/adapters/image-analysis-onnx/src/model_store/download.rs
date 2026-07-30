use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::Ordering;

use atelier_image_analysis::{
    ImageAnalysisError, ImageAnalysisModelId, ImageAnalysisResult, ModelInstallProgressSink,
};
use futures_util::StreamExt;
use reqwest::header::{CONTENT_RANGE, RANGE};

use crate::manifest::verify_file;
use crate::spec::{ModelFileSpec, ModelSpec};

use super::{
    OnnxImageAnalysisRuntime, blocking_task_error, install_error, report_progress, set_last_error,
    total_bytes,
};

struct DownloadReporting<'a> {
    completed_before: u64,
    total_bytes: u64,
    sink: Option<&'a dyn ModelInstallProgressSink>,
}

impl OnnxImageAnalysisRuntime {
    pub(super) async fn download_file(
        &self,
        spec: &ModelSpec,
        file: &ModelFileSpec,
        completed_before: u64,
        progress: Option<&dyn ModelInstallProgressSink>,
    ) -> ImageAnalysisResult<()> {
        let root = self.revision_root(spec);
        fs::create_dir_all(&root).map_err(install_error)?;
        let final_path = root.join(file.relative_path);
        let partial_path = root.join(format!("{}.part", file.relative_path));
        let final_verified = if final_path.is_file() {
            let final_path = final_path.clone();
            let file = *file;
            tokio::task::spawn_blocking(move || verify_file(&final_path, &file).is_ok())
                .await
                .map_err(blocking_task_error)?
        } else {
            false
        };
        if final_verified {
            report_progress(
                progress,
                spec.id,
                completed_before + file.size_bytes,
                total_bytes(spec),
            );
            return Ok(());
        }

        for _attempt in 0..3 {
            if self
                .download_file_attempt(
                    spec,
                    file,
                    &partial_path,
                    &final_path,
                    completed_before,
                    progress,
                )
                .await?
            {
                return Ok(());
            }
        }
        Err(ImageAnalysisError::model_install(format!(
            "failed to download and verify {}",
            file.relative_path
        )))
    }

    async fn download_file_attempt(
        &self,
        spec: &ModelSpec,
        file: &ModelFileSpec,
        partial_path: &Path,
        final_path: &Path,
        completed_before: u64,
        progress: Option<&dyn ModelInstallProgressSink>,
    ) -> ImageAnalysisResult<bool> {
        self.ensure_install_active(spec.id)?;
        let existing = resumable_bytes(partial_path, file.size_bytes)?;
        let mut request = self.client.get(file.url);
        if existing > 0 {
            request = request.header(RANGE, format!("bytes={existing}-"));
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                set_last_error(self.slot(spec.id), error.to_string());
                return Ok(false);
            }
        };
        if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            remove_file_if_present(partial_path)?;
            set_last_error(
                self.slot(spec.id),
                format!(
                    "server rejected the saved download range for {}; restarting",
                    file.relative_path
                ),
            );
            return Ok(false);
        }
        if !response.status().is_success() {
            set_last_error(
                self.slot(spec.id),
                format!("model download returned HTTP {}", response.status()),
            );
            return Ok(false);
        }
        let resume = response_can_resume(&response, existing);
        let reporting = DownloadReporting {
            completed_before,
            total_bytes: total_bytes(spec),
            sink: progress,
        };
        if !self
            .write_response_body(spec, file, partial_path, response, resume, &reporting)
            .await?
        {
            return Ok(false);
        }
        let runtime = self.clone();
        let model_id = spec.id;
        let file = *file;
        let partial_path = partial_path.to_owned();
        let final_path = final_path.to_owned();
        tokio::task::spawn_blocking(move || {
            runtime.activate_download(model_id, &file, &partial_path, &final_path)
        })
        .await
        .map_err(blocking_task_error)?
    }

    async fn write_response_body(
        &self,
        spec: &ModelSpec,
        file: &ModelFileSpec,
        partial_path: &Path,
        response: reqwest::Response,
        resume: bool,
        reporting: &DownloadReporting<'_>,
    ) -> ImageAnalysisResult<bool> {
        let mut output = open_partial_file(partial_path, resume)?;
        let mut downloaded = if resume {
            fs::metadata(partial_path).map_or(0, |metadata| metadata.len())
        } else {
            0
        };
        let mut stream = response.bytes_stream();
        let mut failed = false;
        let mut oversized = false;
        while let Some(chunk) = stream.next().await {
            self.ensure_install_active(spec.id)?;
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    set_last_error(self.slot(spec.id), error.to_string());
                    failed = true;
                    break;
                }
            };
            if downloaded.saturating_add(chunk.len() as u64) > file.size_bytes {
                set_last_error(
                    self.slot(spec.id),
                    format!(
                        "model download exceeded the pinned size for {}",
                        file.relative_path
                    ),
                );
                failed = true;
                oversized = true;
                break;
            }
            output.write_all(&chunk).map_err(install_error)?;
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            report_progress(
                reporting.sink,
                spec.id,
                reporting.completed_before + downloaded,
                reporting.total_bytes,
            );
        }
        output.flush().map_err(install_error)?;
        output.sync_all().map_err(install_error)?;
        drop(output);
        if oversized {
            remove_file_if_present(partial_path)?;
        }
        Ok(!failed)
    }

    fn ensure_install_active(&self, id: ImageAnalysisModelId) -> ImageAnalysisResult<()> {
        if self.slot(id).cancel.load(Ordering::Acquire) {
            return Err(ImageAnalysisError::model_install(
                "model installation was cancelled",
            ));
        }
        Ok(())
    }

    fn activate_download(
        &self,
        id: ImageAnalysisModelId,
        file: &ModelFileSpec,
        partial_path: &Path,
        final_path: &Path,
    ) -> ImageAnalysisResult<bool> {
        if verify_file(partial_path, file).is_err() {
            set_last_error(
                self.slot(id),
                format!(
                    "downloaded model file failed verification: {}",
                    file.relative_path
                ),
            );
            remove_file_if_present(partial_path)?;
            return Ok(false);
        }
        if final_path.exists() {
            fs::remove_file(final_path).map_err(install_error)?;
        }
        fs::rename(partial_path, final_path).map_err(install_error)?;
        Ok(true)
    }
}

fn response_can_resume(response: &reqwest::Response, existing: u64) -> bool {
    existing > 0
        && response.status() == reqwest::StatusCode::PARTIAL_CONTENT
        && response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with(&format!("bytes {existing}-")))
}

fn open_partial_file(path: &Path, resume: bool) -> ImageAnalysisResult<File> {
    if resume {
        OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(install_error)
    } else {
        File::create(path).map_err(install_error)
    }
}

fn resumable_bytes(partial_path: &Path, expected_size: u64) -> ImageAnalysisResult<u64> {
    let existing = fs::metadata(partial_path).map_or(0, |metadata| metadata.len());
    if existing >= expected_size {
        remove_file_if_present(partial_path)?;
        return Ok(0);
    }
    Ok(existing)
}

fn remove_file_if_present(path: &Path) -> ImageAnalysisResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(install_error(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_or_oversized_partial_is_discarded_before_resuming() {
        let temp = tempfile::tempdir().unwrap();
        let partial = temp.path().join("model.onnx.part");
        fs::write(&partial, b"four").unwrap();

        assert_eq!(resumable_bytes(&partial, 4).unwrap(), 0);
        assert!(!partial.exists());

        fs::write(&partial, b"oversized").unwrap();
        assert_eq!(resumable_bytes(&partial, 4).unwrap(), 0);
        assert!(!partial.exists());
    }

    #[test]
    fn incomplete_partial_can_resume() {
        let temp = tempfile::tempdir().unwrap();
        let partial = temp.path().join("model.onnx.part");
        fs::write(&partial, b"two").unwrap();

        assert_eq!(resumable_bytes(&partial, 4).unwrap(), 3);
        assert!(partial.exists());
    }
}
