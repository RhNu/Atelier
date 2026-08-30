use atelier_downloadable_resources::{
    DownloadableResourceCatalog, DownloadableResourceDescriptor, DownloadableResourceFile,
    DownloadableResourceGroup,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogDocument {
    pub format: String,
    pub schema_version: u32,
    pub catalog_version: String,
    pub resources: Vec<ResourceDocument>,
    pub groups: Vec<GroupDocument>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceDocument {
    pub id: String,
    pub version: String,
    pub contract_version: u32,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub files: Vec<FileDocument>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileDocument {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub urls: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupDocument {
    pub id: String,
    pub resources: Vec<String>,
}

impl From<CatalogDocument> for DownloadableResourceCatalog {
    fn from(value: CatalogDocument) -> Self {
        Self {
            format: value.format,
            schema_version: value.schema_version,
            catalog_version: value.catalog_version,
            resources: value.resources.into_iter().map(Into::into).collect(),
            groups: value.groups.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ResourceDocument> for DownloadableResourceDescriptor {
    fn from(value: ResourceDocument) -> Self {
        Self {
            id: value.id,
            version: value.version,
            contract_version: value.contract_version,
            dependencies: value.dependencies,
            files: value.files.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<FileDocument> for DownloadableResourceFile {
    fn from(value: FileDocument) -> Self {
        Self {
            path: value.path,
            size_bytes: value.size_bytes,
            sha256: value.sha256,
            urls: value.urls,
        }
    }
}

impl From<GroupDocument> for DownloadableResourceGroup {
    fn from(value: GroupDocument) -> Self {
        Self {
            id: value.id,
            resources: value.resources,
        }
    }
}
