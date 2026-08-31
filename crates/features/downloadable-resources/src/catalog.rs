use std::collections::{HashMap, HashSet};
use std::path::Path;

use semver::Version;
use url::Url;

use crate::{
    CATALOG_FORMAT, CATALOG_SCHEMA_VERSION, DownloadableResourceCatalog, DownloadableResourceError,
    DownloadableResourceResult,
};

/// Validates the complete catalog before it reaches installers or consumers.
///
/// # Errors
/// Returns an invalid-catalog error for unsupported contracts, malformed identifiers, unsafe
/// paths, duplicate entries, invalid URLs, or dependency/group cycles.
pub fn validate_catalog(catalog: &DownloadableResourceCatalog) -> DownloadableResourceResult<()> {
    if catalog.format != CATALOG_FORMAT || catalog.schema_version != CATALOG_SCHEMA_VERSION {
        return invalid("unsupported format or schema version");
    }
    Version::parse(&catalog.catalog_version).map_err(|error| {
        DownloadableResourceError::InvalidCatalog(format!("invalid catalog version: {error}"))
    })?;
    let mut ids = HashSet::new();
    for resource in &catalog.resources {
        validate_id(&resource.id)?;
        if !ids.insert(resource.id.as_str()) {
            return invalid(format!("duplicate resource id: {}", resource.id));
        }
        Version::parse(&resource.version).map_err(|error| {
            DownloadableResourceError::InvalidCatalog(format!(
                "invalid version for {}: {error}",
                resource.id
            ))
        })?;
        if resource.contract_version != 1 || resource.files.is_empty() {
            return invalid(format!("unsupported or empty resource: {}", resource.id));
        }
        let mut paths = HashSet::new();
        for file in &resource.files {
            validate_file(
                &resource.id,
                file.path.as_str(),
                file.size_bytes,
                &file.sha256,
                &file.urls,
            )?;
            if !paths.insert(file.path.as_str()) {
                return invalid(format!(
                    "duplicate file path in {}: {}",
                    resource.id, file.path
                ));
            }
        }
    }
    let resources = catalog
        .resources
        .iter()
        .map(|resource| (resource.id.as_str(), resource.dependencies.as_slice()))
        .collect::<HashMap<_, _>>();
    for (id, dependencies) in &resources {
        for dependency in *dependencies {
            if !resources.contains_key(dependency.as_str()) {
                return invalid(format!("{id} references missing dependency {dependency}"));
            }
        }
    }
    for id in resources.keys().copied() {
        detect_cycle(id, &resources, &mut HashSet::new(), &mut HashSet::new())?;
    }
    let mut group_ids = HashSet::new();
    for group in &catalog.groups {
        validate_id(&group.id)?;
        if !group_ids.insert(group.id.as_str()) || group.resources.is_empty() {
            return invalid(format!("duplicate or empty group: {}", group.id));
        }
        for resource in &group.resources {
            if !resources.contains_key(resource.as_str()) {
                return invalid(format!(
                    "group {} references missing resource {resource}",
                    group.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_file(
    resource: &str,
    path: &str,
    size: u64,
    sha256: &str,
    urls: &[String],
) -> DownloadableResourceResult<()> {
    let path_value = Path::new(path);
    if path.is_empty()
        || path_value.is_absolute()
        || path_value
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return invalid(format!("invalid file path in {resource}: {path}"));
    }
    if size == 0 || sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return invalid(format!("invalid file metadata in {resource}: {path}"));
    }
    if urls.is_empty()
        || urls.iter().any(|value| {
            Url::parse(value).map_or(true, |url| {
                url.scheme() != "https"
                    || url.host_str().is_none()
                    || !url.username().is_empty()
                    || url.password().is_some()
            })
        })
    {
        return invalid(format!("resource URLs must use HTTPS: {resource}/{path}"));
    }
    Ok(())
}

fn validate_id(id: &str) -> DownloadableResourceResult<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return invalid(format!("invalid identifier: {id}"));
    }
    Ok(())
}

fn detect_cycle<'a>(
    id: &'a str,
    resources: &HashMap<&'a str, &'a [String]>,
    visiting: &mut HashSet<&'a str>,
    visited: &mut HashSet<&'a str>,
) -> DownloadableResourceResult<()> {
    if visited.contains(id) {
        return Ok(());
    }
    if !visiting.insert(id) {
        return invalid(format!("resource dependency cycle at {id}"));
    }
    let Some(dependencies) = resources.get(id) else {
        return invalid(format!("resource dependency {id} is missing"));
    };
    for dependency in *dependencies {
        detect_cycle(dependency, resources, visiting, visited)?;
    }
    visiting.remove(id);
    visited.insert(id);
    Ok(())
}

fn invalid<T>(message: impl Into<String>) -> DownloadableResourceResult<T> {
    Err(DownloadableResourceError::InvalidCatalog(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DownloadableResourceDescriptor, DownloadableResourceFile, DownloadableResourceGroup,
    };

    fn catalog() -> DownloadableResourceCatalog {
        DownloadableResourceCatalog {
            format: CATALOG_FORMAT.to_owned(),
            schema_version: 1,
            catalog_version: "1.0.0".to_owned(),
            resources: vec![DownloadableResourceDescriptor {
                id: "lexicon-core".to_owned(),
                version: "1.0.0".to_owned(),
                contract_version: 1,
                dependencies: vec![],
                files: vec![DownloadableResourceFile {
                    path: "lexicon.sqlite".to_owned(),
                    size_bytes: 10,
                    sha256: "0".repeat(64),
                    urls: vec!["https://example.invalid/lexicon.sqlite".to_owned()],
                }],
            }],
            groups: vec![DownloadableResourceGroup {
                id: "starter".to_owned(),
                resources: vec!["lexicon-core".to_owned()],
            }],
        }
    }

    #[test]
    fn accepts_a_valid_catalog() {
        validate_catalog(&catalog()).unwrap();
    }

    #[test]
    fn rejects_path_traversal_and_missing_groups() {
        let mut value = catalog();
        value.resources[0].files[0].path = "../model.onnx".to_owned();
        assert!(validate_catalog(&value).is_err());
        let mut value = catalog();
        value.groups[0].resources[0] = "missing".to_owned();
        assert!(validate_catalog(&value).is_err());
    }

    #[test]
    fn rejects_duplicate_ids_paths_and_invalid_semver() {
        let mut value = catalog();
        value.resources.push(value.resources[0].clone());
        assert!(validate_catalog(&value).is_err());

        let mut value = catalog();
        let duplicate = value.resources[0].files[0].clone();
        value.resources[0].files.push(duplicate);
        assert!(validate_catalog(&value).is_err());

        let mut value = catalog();
        value.resources[0].version = "latest".to_owned();
        assert!(validate_catalog(&value).is_err());
    }

    #[test]
    fn rejects_unknown_contract_and_dependency_cycles() {
        let mut value = catalog();
        value.resources[0].contract_version = 2;
        assert!(validate_catalog(&value).is_err());

        let mut value = catalog();
        let mut second = value.resources[0].clone();
        second.id = "lexicon-semantic".to_owned();
        second.dependencies = vec!["lexicon-core".to_owned()];
        value.resources[0].dependencies = vec!["lexicon-semantic".to_owned()];
        value.resources.push(second);
        assert!(validate_catalog(&value).is_err());
    }
}
