use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use atelier_downloadable_resources::{
    DownloadableResourceError, DownloadableResourceFile, DownloadableResourceResult,
};
use futures_util::StreamExt;
use reqwest::header::{CONTENT_RANGE, RANGE};
use sha2::{Digest, Sha256};

pub async fn download_file(
    client: &reqwest::Client,
    spec: &DownloadableResourceFile,
    destination: &Path,
    cancelled: &AtomicBool,
    mut report: impl FnMut(u64),
) -> DownloadableResourceResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(operation)?;
    }
    let file_name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            DownloadableResourceError::Operation("invalid destination file name".to_owned())
        })?;
    let partial = destination.with_file_name(format!("{file_name}.part"));
    let mut last_error = None;
    for url in &spec.urls {
        match download_from(client, url, spec, &partial, cancelled, &mut report).await {
            Ok(()) => {
                if let Err(error) = verify(&partial, spec) {
                    last_error = Some(error.to_string());
                    continue;
                }
                if destination.exists() {
                    fs::remove_file(destination).map_err(operation)?;
                }
                fs::rename(partial, destination).map_err(operation)?;
                return Ok(());
            }
            Err(DownloadableResourceError::Cancelled) => {
                return Err(DownloadableResourceError::Cancelled);
            }
            Err(error) => last_error = Some(error.to_string()),
        }
    }
    Err(DownloadableResourceError::Operation(
        last_error.unwrap_or_else(|| "all resource mirrors failed".to_owned()),
    ))
}

async fn download_from(
    client: &reqwest::Client,
    url: &str,
    spec: &DownloadableResourceFile,
    partial: &Path,
    cancelled: &AtomicBool,
    report: &mut impl FnMut(u64),
) -> DownloadableResourceResult<()> {
    let existing = resumable_bytes(partial, spec.size_bytes)?;
    let mut request = client.get(url);
    if existing > 0 {
        request = request.header(RANGE, format!("bytes={existing}-"));
    }
    let response = request.send().await.map_err(operation)?;
    let resume = existing > 0
        && response.status() == reqwest::StatusCode::PARTIAL_CONTENT
        && response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with(&format!("bytes {existing}-")));
    let response = response.error_for_status().map_err(operation)?;
    let mut file = if resume {
        OpenOptions::new()
            .append(true)
            .open(partial)
            .map_err(operation)?
    } else {
        File::create(partial).map_err(operation)?
    };
    let mut downloaded = if resume { existing } else { 0 };
    report(downloaded);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if cancelled.load(Ordering::Acquire) {
            return Err(DownloadableResourceError::Cancelled);
        }
        let chunk = chunk.map_err(operation)?;
        file.write_all(&chunk).map_err(operation)?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        if downloaded > spec.size_bytes {
            return Err(DownloadableResourceError::Operation(
                "download exceeded declared file size".to_owned(),
            ));
        }
        report(downloaded);
    }
    file.sync_all().map_err(operation)
}

pub fn verify(path: &Path, spec: &DownloadableResourceFile) -> DownloadableResourceResult<()> {
    let mut file = File::open(path).map_err(operation)?;
    if file.metadata().map_err(operation)?.len() != spec.size_bytes {
        return Err(DownloadableResourceError::Operation(format!(
            "size mismatch for {}",
            spec.path
        )));
    }
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(operation)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    if digest_hex(digest.finalize()) != spec.sha256.to_ascii_lowercase() {
        return Err(DownloadableResourceError::Operation(format!(
            "SHA-256 mismatch for {}",
            spec.path
        )));
    }
    Ok(())
}

fn digest_hex(digest: impl IntoIterator<Item = u8>) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing a SHA-256 digest to String cannot fail");
    }
    output
}

fn resumable_bytes(path: &Path, expected: u64) -> DownloadableResourceResult<u64> {
    let existing = fs::metadata(path).map_or(0, |metadata| metadata.len());
    if existing >= expected {
        fs::remove_file(path).map_err(operation)?;
        Ok(0)
    } else {
        Ok(existing)
    }
}

fn operation(error: impl std::fmt::Display) -> DownloadableResourceError {
    DownloadableResourceError::Operation(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::sync::Arc;

    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn resumes_with_range_and_falls_back_when_range_is_ignored() {
        let directory = tempdir().unwrap();
        for supports_range in [true, false] {
            let destination = directory.path().join(format!("range-{supports_range}.bin"));
            fs::write(
                destination.with_file_name(format!(
                    "{}.part",
                    destination.file_name().unwrap().to_string_lossy()
                )),
                b"abc",
            )
            .unwrap();
            let (url, request) = serve_once(move |request| {
                assert!(request.contains("Range: bytes=3-") || request.contains("range: bytes=3-"));
                if supports_range {
                    response(
                        "206 Partial Content",
                        &["Content-Range: bytes 3-5/6"],
                        b"def",
                    )
                } else {
                    response("200 OK", &[], b"abcdef")
                }
            });
            download_file(
                &reqwest::Client::new(),
                &file_spec(url, b"abcdef"),
                &destination,
                &AtomicBool::new(false),
                |_| {},
            )
            .await
            .unwrap();
            request.join().unwrap();
            assert_eq!(fs::read(destination).unwrap(), b"abcdef");
        }
    }

    #[tokio::test]
    async fn switches_mirrors_and_rejects_corrupt_content() {
        let directory = tempdir().unwrap();
        let (failed_url, failed) = serve_once(|_| response("503 Unavailable", &[], b""));
        let (corrupt_url, corrupt) = serve_once(|_| response("200 OK", &[], b"abcdeg"));
        let (good_url, good) = serve_once(|_| response("200 OK", &[], b"abcdef"));
        let destination = directory.path().join("mirrored.bin");
        let mut spec = file_spec(failed_url, b"abcdef");
        spec.urls.push(corrupt_url);
        spec.urls.push(good_url);
        download_file(
            &reqwest::Client::new(),
            &spec,
            &destination,
            &AtomicBool::new(false),
            |_| {},
        )
        .await
        .unwrap();
        failed.join().unwrap();
        corrupt.join().unwrap();
        good.join().unwrap();
        assert_eq!(fs::read(destination).unwrap(), b"abcdef");

        let (corrupt_url, corrupt) = serve_once(|_| response("200 OK", &[], b"abcdeg"));
        let corrupt_destination = directory.path().join("corrupt.bin");
        let error = download_file(
            &reqwest::Client::new(),
            &file_spec(corrupt_url, b"abcdef"),
            &corrupt_destination,
            &AtomicBool::new(false),
            |_| {},
        )
        .await
        .unwrap_err();
        corrupt.join().unwrap();
        assert!(error.to_string().contains("SHA-256 mismatch"));
        assert!(!corrupt_destination.exists());
    }

    fn file_spec(url: String, expected: &[u8]) -> DownloadableResourceFile {
        DownloadableResourceFile {
            path: "payload.bin".to_owned(),
            size_bytes: expected.len() as u64,
            sha256: digest_hex(Sha256::digest(expected)),
            urls: vec![url],
        }
    }

    fn serve_once(
        handler: impl FnOnce(&str) -> Vec<u8> + Send + 'static,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handler = Arc::new(std::sync::Mutex::new(Some(handler)));
        let join = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0_u8; 4096];
            let read = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..read]);
            let response = handler.lock().unwrap().take().unwrap()(&request);
            stream.write_all(&response).unwrap();
        });
        (format!("http://{address}/payload.bin"), join)
    }

    fn response(status: &str, headers: &[&str], body: &[u8]) -> Vec<u8> {
        let extra = if headers.is_empty() {
            String::new()
        } else {
            format!("{}\r\n", headers.join("\r\n"))
        };
        format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n",
            body.len()
        )
        .bytes()
        .chain(body.iter().copied())
        .collect()
    }
}
