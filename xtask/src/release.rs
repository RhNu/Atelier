use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use atelier_downloadable_resources::{DownloadableResourceCatalog, validate_catalog};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct PackageManifest {
    version: String,
}

#[derive(Deserialize)]
struct Catalog {
    resources: Vec<ResourceDescriptor>,
}

#[derive(Deserialize)]
struct ResourceDescriptor {
    id: String,
    version: String,
    files: Vec<ResourceFile>,
}

#[derive(Deserialize)]
struct ResourceFile {
    path: String,
    size_bytes: u64,
    sha256: String,
}

/// Updates the single desktop application version source.
///
/// # Errors
/// Returns an error for invalid/non-increasing versions or unreadable manifests.
pub fn prepare_app_release(root: &Path, version: &str) -> Result<(), String> {
    let next =
        Version::parse(version).map_err(|error| format!("invalid release version: {error}"))?;
    if !next.pre.is_empty() || !next.build.is_empty() {
        return Err("application releases must use a stable SemVer".to_owned());
    }
    let path = package_path(root);
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
    println!("Prepared Atelier {next}; commit the version change before tagging.");
    Ok(())
}

/// Creates an annotated application tag from a clean worktree.
///
/// # Errors
/// Returns an error for dirty state, invalid versions, or failed Git operations.
pub fn tag_app_release(root: &Path) -> Result<(), String> {
    require_clean(root)?;
    let version = app_version(root)?;
    create_tag(root, &format!("v{version}"))
}

/// Validates a resource descriptor and any repository-owned payload.
///
/// # Errors
/// Returns an error for unknown resources, missing payloads, or size/hash mismatches.
pub fn validate_resource(root: &Path, id: &str) -> Result<String, String> {
    let catalog: Catalog = serde_json::from_slice(
        &fs::read(root.join("resources/catalog/catalog-v1.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let descriptor = catalog
        .resources
        .into_iter()
        .find(|resource| resource.id == id)
        .ok_or_else(|| format!("unknown resource: {id}"))?;
    Version::parse(&descriptor.version)
        .map_err(|error| format!("invalid resource version: {error}"))?;
    let payload = root
        .join("resources/payloads")
        .join(id)
        .join(&descriptor.version);
    if payload.is_dir() {
        for file in &descriptor.files {
            let path = payload.join(&file.path);
            if !path.is_file() {
                return Err(format!("missing payload file: {}", path.display()));
            }
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            if bytes.len() as u64 != file.size_bytes {
                return Err(format!("payload size mismatch: {}", path.display()));
            }
            let hash = digest_hex(Sha256::digest(&bytes));
            if hash != file.sha256 {
                return Err(format!("payload hash mismatch: {}", path.display()));
            }
        }
    } else if descriptor
        .files
        .iter()
        .any(|file| file.path.starts_with("lexicon") || id.starts_with("lexicon-"))
    {
        return Err(format!(
            "missing local payload directory: {}",
            payload.display()
        ));
    }
    println!("Resource {id}@{} is valid.", descriptor.version);
    Ok(descriptor.version)
}

fn digest_hex(digest: impl IntoIterator<Item = u8>) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing a SHA-256 digest to String cannot fail");
    }
    output
}

/// Validates the checked-in stable resource catalog.
///
/// # Errors
/// Returns an error when the catalog cannot be read or violates its domain contract.
pub fn validate_resource_catalog(root: &Path) -> Result<(), String> {
    let path = root.join("resources/catalog/catalog-v1.json");
    let repository = repository_name(root)?;
    let source = fs::read_to_string(&path)
        .map_err(|error| error.to_string())?
        .replace("__GITHUB_REPOSITORY__", &repository);
    let catalog: DownloadableResourceCatalog =
        serde_json::from_slice(source.as_bytes()).map_err(|error| error.to_string())?;
    validate_catalog(&catalog).map_err(|error| error.to_string())?;
    println!("Downloadable resource catalog is valid: {}", path.display());
    Ok(())
}

/// Validates that a resource release tag exactly matches its stable catalog descriptor.
///
/// # Errors
/// Returns an error for malformed tags, unknown IDs, or version mismatches.
pub fn validate_resource_tag(root: &Path, tag: &str) -> Result<(String, String), String> {
    let body = tag
        .strip_prefix("resource-")
        .ok_or_else(|| "resource tag must start with resource-".to_owned())?;
    let (id, version) = body
        .rsplit_once("-v")
        .ok_or_else(|| "resource tag must end with -v<SemVer>".to_owned())?;
    Version::parse(version).map_err(|error| format!("invalid resource tag version: {error}"))?;
    let declared = validate_resource(root, id)?;
    if declared != version {
        return Err(format!(
            "resource tag version {version} does not match {id}@{declared}"
        ));
    }
    Ok((id.to_owned(), version.to_owned()))
}

/// Validates a resource and creates its annotated release tag from a clean worktree.
///
/// # Errors
/// Returns an error for dirty state, invalid payloads, or failed Git operations.
pub fn tag_resource_release(root: &Path, id: &str) -> Result<(), String> {
    require_clean(root)?;
    let version = validate_resource(root, id)?;
    create_tag(root, &format!("resource-{id}-v{version}"))
}

fn app_version(root: &Path) -> Result<Version, String> {
    let package: PackageManifest =
        serde_json::from_slice(&fs::read(package_path(root)).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    Version::parse(&package.version).map_err(|error| error.to_string())
}

fn package_path(root: &Path) -> std::path::PathBuf {
    root.join("apps/desktop/package.json")
}

fn repository_name(root: &Path) -> Result<String, String> {
    if let Ok(repository) = std::env::var("GITHUB_REPOSITORY")
        && repository.split('/').count() == 2
    {
        return Ok(repository);
    }
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    let remote = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let path = remote
        .trim()
        .strip_prefix("git@github.com:")
        .or_else(|| remote.trim().strip_prefix("https://github.com/"))
        .ok_or_else(|| "GITHUB_REPOSITORY or a GitHub origin remote is required".to_owned())?;
    Ok(path.trim_end_matches(".git").to_owned())
}

fn require_clean(root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() || !output.stdout.is_empty() {
        return Err("release tags require a clean working tree".to_owned());
    }
    Ok(())
}

fn create_tag(root: &Path, tag: &str) -> Result<(), String> {
    let status = Command::new("git")
        .args(["tag", "--annotate", tag, "--message", tag])
        .current_dir(root)
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!("could not create tag {tag}"));
    }
    println!("Created local tag {tag}; push it explicitly when ready.");
    Ok(())
}
