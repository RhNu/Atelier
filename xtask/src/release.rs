use std::fs;
use std::path::Path;

use atelier_downloadable_resources::{DownloadableResourceCatalog, validate_catalog};
use semver::Version;
use serde::Deserialize;

#[derive(Deserialize)]
struct PackageManifest {
    version: String,
}

/// Updates the single desktop application version source without committing or publishing.
///
/// # Errors
/// Returns an error for invalid/non-increasing versions or unreadable manifests.
pub fn prepare_app_release(root: &Path, version: &str) -> Result<(), String> {
    let next =
        Version::parse(version).map_err(|error| format!("invalid release version: {error}"))?;
    if !next.pre.is_empty() || !next.build.is_empty() {
        return Err("application releases must use a stable SemVer".to_owned());
    }
    let path = root.join("apps/desktop/package.json");
    let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let package: PackageManifest =
        serde_json::from_str(&source).map_err(|error| error.to_string())?;
    let current = Version::parse(&package.version).map_err(|error| error.to_string())?;
    if next <= current {
        return Err(format!(
            "release version {next} must be newer than {current}"
        ));
    }
    let needle = format!("\"version\": \"{current}\"");
    let replacement = format!("\"version\": \"{next}\"");
    if source.matches(&needle).count() != 1 {
        return Err("desktop package version field is ambiguous".to_owned());
    }
    fs::write(path, source.replacen(&needle, &replacement, 1))
        .map_err(|error| error.to_string())?;
    println!("Prepared Atelier {next}; commit, push to main, then run Release application.");
    Ok(())
}

/// Checks the source catalog using the same contract as the runtime consumer.
/// Payload bytes are checked once during resource staging, not by this command.
///
/// # Errors
/// Returns an error when the catalog cannot be read or violates its domain contract.
pub fn validate_resource_catalog(root: &Path) -> Result<(), String> {
    let path = root.join("resources/catalog/catalog-v1.json");
    let catalog: DownloadableResourceCatalog =
        serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    validate_catalog(&catalog).map_err(|error| error.to_string())?;
    println!("Downloadable resource catalog is valid: {}", path.display());
    Ok(())
}
