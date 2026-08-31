use std::fs::File;
use std::io::Read;
use std::path::Path;

use atelier_prompt_lexicon::{LexiconError, LexiconResult};
use sha2::{Digest, Sha256};

use crate::manifest::{BundleFile, digest_hex, validate_file_metadata};

const MAX_LICENSE_BYTES: u64 = 1024 * 1024;

/// Accept only byte-identical text or LF/CRLF equivalents of the recorded digest.
/// Legacy manifests hashed Windows line endings; release catalogs hashed LF bytes.
/// Do not accept arbitrary readable text or strip meaningful whitespace.
pub fn verify_license(root: &Path, file: &BundleFile) -> LexiconResult<()> {
    let metadata = validate_file_metadata(root, file)?;
    if metadata.len() > MAX_LICENSE_BYTES {
        return Err(LexiconError::invalid_bundle(
            "model license exceeds size limit",
        ));
    }
    let path = root.join(&file.file);
    let mut bytes = Vec::new();
    File::open(&path)
        .and_then(|stream| stream.take(MAX_LICENSE_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|error| LexiconError::invalid_bundle(format!("{}: {error}", path.display())))?;
    if bytes.len() as u64 > MAX_LICENSE_BYTES {
        return Err(LexiconError::invalid_bundle(
            "model license exceeds size limit",
        ));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| LexiconError::invalid_bundle(format!("invalid license UTF-8: {error}")))?;
    let expected = file.sha256.to_ascii_lowercase();
    if digest_hex(Sha256::digest(&bytes)) == expected {
        return Ok(());
    }
    let lf = text.replace("\r\n", "\n");
    if digest_hex(Sha256::digest(lf.as_bytes())) == expected
        || digest_hex(Sha256::digest(lf.replace('\n', "\r\n").as_bytes())) == expected
    {
        return Ok(());
    }
    Err(LexiconError::invalid_bundle(format!(
        "model license content SHA-256 mismatch for {} (checked LF and CRLF)",
        path.display()
    )))
}
