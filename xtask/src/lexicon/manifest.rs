use super::{Deserialize, Ordering, Path, PathBuf, fs, strip_bom};

pub(super) fn load_source_manifest(path: &Path) -> Result<SourceManifest, String> {
    let raw_text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let raw: SourceManifestRaw =
        serde_json::from_str(&strip_bom(&raw_text)).map_err(|error| error.to_string())?;
    let manifest_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut sources = Vec::new();
    for (manifest_index, source) in raw.sources.iter().enumerate() {
        sources.push(ManifestSource::from_raw(
            source,
            manifest_dir,
            manifest_index,
        )?);
    }
    Ok(SourceManifest {
        version: raw.version.unwrap_or(1),
        sources,
    })
}

#[derive(Deserialize)]
struct SourceManifestRaw {
    pub(super) version: Option<u32>,
    pub(super) sources: Vec<ManifestSourceRaw>,
}

#[derive(Deserialize)]
struct ManifestSourceRaw {
    pub(super) id: String,
    pub(super) path: String,
    pub(super) parser: String,
    pub(super) priority: i64,
    #[serde(default)]
    pub(super) alias_only: bool,
    pub(super) allow_primary: Option<bool>,
}

#[derive(Clone)]
pub(super) struct SourceManifest {
    pub(super) version: u32,
    pub(super) sources: Vec<ManifestSource>,
}

#[derive(Clone)]
pub(super) struct ManifestSource {
    pub(super) id: String,
    pub(super) relative_path: String,
    pub(super) path: PathBuf,
    pub(super) parser: SourceParser,
    pub(super) priority: i64,
    pub(super) alias_only: bool,
    pub(super) allow_primary: bool,
    pub(super) manifest_index: usize,
}

impl ManifestSource {
    fn from_raw(
        raw: &ManifestSourceRaw,
        manifest_dir: &Path,
        manifest_index: usize,
    ) -> Result<Self, String> {
        let id = raw.id.trim().to_owned();
        let relative_path = raw.path.trim().to_owned();
        let parser = SourceParser::parse(raw.parser.trim())?;
        let allow_primary = raw.allow_primary.unwrap_or(!raw.alias_only);
        if id.is_empty() || relative_path.is_empty() {
            return Err("prompt lexicon source id and path must be non-empty".to_owned());
        }
        if raw.alias_only && allow_primary {
            return Err(format!(
                "prompt lexicon source `{id}` cannot be alias_only and allow_primary"
            ));
        }
        Ok(Self {
            id,
            path: manifest_dir.join(&relative_path),
            relative_path,
            parser,
            priority: raw.priority,
            alias_only: raw.alias_only,
            allow_primary,
            manifest_index,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum SourceParser {
    Weighted,
    Simple,
    Reversed,
    Github,
    Alias,
}

impl SourceParser {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "weighted_csv" => Ok(Self::Weighted),
            "simple_csv" => Ok(Self::Simple),
            "reversed_csv" => Ok(Self::Reversed),
            "github_csv" => Ok(Self::Github),
            "alias_csv" => Ok(Self::Alias),
            _ => Err(format!("unsupported prompt lexicon parser `{value}`")),
        }
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Weighted => "weighted_csv",
            Self::Simple => "simple_csv",
            Self::Reversed => "reversed_csv",
            Self::Github => "github_csv",
            Self::Alias => "alias_csv",
        }
    }
}

pub(super) fn compare_manifest_source(left: &ManifestSource, right: &ManifestSource) -> Ordering {
    right
        .priority
        .cmp(&left.priority)
        .then_with(|| left.manifest_index.cmp(&right.manifest_index))
        .then_with(|| left.id.cmp(&right.id))
}
