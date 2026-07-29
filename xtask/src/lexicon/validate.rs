use std::fs;
use std::path::Path;

use atelier_adapter_lexicon_bundle::{LexiconBundle, LexiconBundleManifest};
use atelier_prompt_lexicon::LexiconEngine;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconValidationSummary {
    pub entity_count: u64,
    pub semantic_available: bool,
}

/// Fully validates manifest checksums, database structure, and semantic dimensions.
///
/// # Errors
/// Returns an error when any runtime asset is stale, malformed, or an LFS pointer.
pub fn validate_lexicon_bundle(root: &Path) -> Result<LexiconValidationSummary, String> {
    reject_lfs_pointers(root)?;
    let manifest = LexiconBundleManifest::read(root).map_err(|error| error.to_string())?;
    verify_file(root, &manifest.database.file, &manifest.database.sha256)?;
    if let Some(semantic) = &manifest.semantic {
        verify_file(root, &semantic.model.file, &semantic.model.sha256)?;
        verify_file(root, &semantic.tokenizer.file, &semantic.tokenizer.sha256)?;
        verify_file(root, &semantic.license.file, &semantic.license.sha256)?;
        verify_file(
            root,
            &semantic.identity_vectors.file,
            &semantic.identity_vectors.sha256,
        )?;
        verify_file(
            root,
            &semantic.knowledge_vectors.file,
            &semantic.knowledge_vectors.sha256,
        )?;
    }
    let engine = LexiconBundle::open(root).map_err(|error| error.to_string())?;
    let bootstrap = engine.bootstrap().map_err(|error| error.to_string())?;
    Ok(LexiconValidationSummary {
        entity_count: bootstrap.stats.total_entities,
        semantic_available: bootstrap.status.semantic_available,
    })
}

fn reject_lfs_pointers(root: &Path) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if !path.is_file() {
            continue;
        }
        let bytes = fs::read(&path).map_err(|error| error.to_string())?;
        if bytes.starts_with(b"version https://git-lfs.github.com/spec/v1") {
            return Err(format!(
                "{} is a Git LFS pointer; run `git lfs pull`",
                path.display()
            ));
        }
    }
    Ok(())
}

fn verify_file(root: &Path, relative: &str, expected: &str) -> Result<(), String> {
    let path = root.join(relative);
    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    if bytes.starts_with(b"version https://git-lfs.github.com/spec/v1") {
        return Err(format!(
            "{} is a Git LFS pointer; run `git lfs pull`",
            path.display()
        ));
    }
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "SHA-256 mismatch for {}: expected {expected}, got {actual}",
            path.display()
        ))
    }
}
