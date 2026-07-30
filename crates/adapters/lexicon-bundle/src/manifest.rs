use std::fs::{self, File};
use std::io::{Read, Take};
use std::path::{Path, PathBuf};

use atelier_prompt_lexicon::{LexiconError, LexiconResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const BUNDLE_FORMAT: &str = "atelier.lexicon.bundle";
pub const BUNDLE_SCHEMA_VERSION: u32 = 2;
pub const DATABASE_SCHEMA_VERSION: u32 = 1;
const MAX_TOKENIZER_CONTENT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LexiconBundleManifest {
    pub format: String,
    pub schema_version: u32,
    pub bundle_version: String,
    pub database: BundleFile,
    #[serde(default)]
    pub semantic: Option<SemanticManifest>,
    #[serde(default)]
    pub enrichment: Option<EnrichmentManifest>,
    #[serde(default)]
    pub ranking: RankingManifest,
    #[serde(default)]
    pub sources: Vec<SourceManifest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnrichmentManifest {
    pub mode: String,
    pub endpoint: String,
    pub model: String,
    pub prompt_hash: String,
    pub entity_count: usize,
    pub input_sha256: String,
    pub output_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleFile {
    pub file: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenizerFile {
    #[serde(flatten)]
    pub bundle: BundleFile,
    pub encoding: TokenizerEncoding,
    pub content_sha256: String,
    pub content_size_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokenizerEncoding {
    ZstdJson,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticManifest {
    pub model: BundleFile,
    pub tokenizer: TokenizerFile,
    pub license: BundleFile,
    pub identity_vectors: BundleFile,
    pub knowledge_vectors: BundleFile,
    pub dimensions: usize,
    pub entity_count: usize,
    #[serde(default = "default_max_length")]
    pub max_length: usize,
    #[serde(default)]
    pub model_contract: SemanticModelContract,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticModelContract {
    #[serde(default = "default_input_ids")]
    pub input_ids: String,
    #[serde(default = "default_attention_mask")]
    pub attention_mask: String,
    #[serde(default)]
    pub token_type_ids: Option<String>,
    #[serde(default)]
    pub output_name: Option<String>,
    #[serde(default = "default_pooling")]
    pub pooling: String,
    #[serde(default = "default_true")]
    pub normalize: bool,
    #[serde(default = "default_query_prefix")]
    pub query_prefix: String,
    #[serde(default = "default_passage_prefix")]
    pub passage_prefix: String,
}

impl Default for SemanticModelContract {
    fn default() -> Self {
        Self {
            input_ids: default_input_ids(),
            attention_mask: default_attention_mask(),
            token_type_ids: Some("token_type_ids".to_owned()),
            output_name: None,
            pooling: default_pooling(),
            normalize: true,
            query_prefix: default_query_prefix(),
            passage_prefix: default_passage_prefix(),
        }
    }
}

impl SemanticManifest {
    pub(crate) fn verify_checksums(
        &self,
        root: &Path,
        bundle: &LexiconBundleManifest,
    ) -> LexiconResult<()> {
        for file in [
            &self.model,
            &self.tokenizer.bundle,
            &self.license,
            &self.identity_vectors,
            &self.knowledge_vectors,
        ] {
            bundle.verify_checksum(root, file)?;
        }
        Ok(())
    }
}

impl TokenizerFile {
    /// Decompresses and verifies the tokenizer JSON payload.
    ///
    /// # Errors
    /// Returns an error when the zstd stream is invalid or its decoded size or checksum differs
    /// from the manifest.
    pub fn decode(&self, root: &Path) -> LexiconResult<Vec<u8>> {
        let path = root.join(&self.bundle.file);
        let file = File::open(&path).map_err(|error| {
            LexiconError::invalid_bundle(format!("failed to open {}: {error}", path.display()))
        })?;
        let decoder = zstd::stream::read::Decoder::new(file).map_err(|error| {
            LexiconError::invalid_bundle(format!(
                "failed to decode tokenizer {}: {error}",
                path.display()
            ))
        })?;
        let limit = self.content_size_bytes.saturating_add(1);
        let mut limited: Take<_> = decoder.take(limit);
        let capacity = usize::try_from(self.content_size_bytes).map_err(|_| {
            LexiconError::invalid_bundle("tokenizer content size exceeds platform capacity")
        })?;
        let mut bytes = Vec::with_capacity(capacity);
        limited.read_to_end(&mut bytes).map_err(|error| {
            LexiconError::invalid_bundle(format!(
                "failed to decompress tokenizer {}: {error}",
                path.display()
            ))
        })?;
        if bytes.len() as u64 != self.content_size_bytes {
            return Err(LexiconError::invalid_bundle(format!(
                "tokenizer content size mismatch for {}",
                path.display()
            )));
        }
        if format!("{:x}", Sha256::digest(&bytes)) != self.content_sha256.to_ascii_lowercase() {
            return Err(LexiconError::invalid_bundle(format!(
                "tokenizer content SHA-256 mismatch for {}",
                path.display()
            )));
        }
        Ok(bytes)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankingManifest {
    #[serde(default = "default_semantic_weight")]
    pub semantic_weight: f32,
    #[serde(default = "default_context_weight")]
    pub context_weight: f32,
    #[serde(default = "default_popularity_weight")]
    pub popularity_weight: f32,
}

impl Default for RankingManifest {
    fn default() -> Self {
        Self {
            semantic_weight: default_semantic_weight(),
            context_weight: default_context_weight(),
            popularity_weight: default_popularity_weight(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceManifest {
    pub id: String,
    pub url: String,
    pub snapshot: String,
    pub sha256: String,
    pub license: String,
}

impl LexiconBundleManifest {
    /// Reads and structurally validates a bundle manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest cannot be read, decoded, or validated.
    pub fn read(root: &Path) -> LexiconResult<Self> {
        let path = root.join("manifest.json");
        let bytes = fs::read(&path).map_err(|error| {
            LexiconError::invalid_bundle(format!("failed to read {}: {error}", path.display()))
        })?;
        let manifest: Self = serde_json::from_slice(&bytes).map_err(|error| {
            LexiconError::invalid_bundle(format!("failed to parse {}: {error}", path.display()))
        })?;
        manifest.validate(root)?;
        Ok(manifest)
    }

    /// Validates the schema, paths, sizes, and semantic model contract.
    ///
    /// # Errors
    ///
    /// Returns an error when any bundle invariant is violated.
    pub fn validate(&self, root: &Path) -> LexiconResult<()> {
        if self.format != BUNDLE_FORMAT || self.schema_version != BUNDLE_SCHEMA_VERSION {
            return Err(LexiconError::invalid_bundle(format!(
                "unsupported format {} schema {}",
                self.format, self.schema_version
            )));
        }
        if self.bundle_version.trim().is_empty() {
            return Err(LexiconError::invalid_bundle(
                "bundle_version must not be empty",
            ));
        }
        if let Some(enrichment) = &self.enrichment
            && (enrichment.mode != "batch"
                || enrichment.endpoint != "/v1/chat/completions"
                || enrichment.model.trim().is_empty()
                || enrichment.entity_count == 0
                || !is_sha256(&enrichment.prompt_hash)
                || !is_sha256(&enrichment.input_sha256)
                || !is_sha256(&enrichment.output_sha256))
        {
            return Err(LexiconError::invalid_bundle(
                "invalid LLM enrichment provenance",
            ));
        }
        validate_file(root, &self.database)?;
        if let Some(semantic) = &self.semantic {
            if semantic.dimensions == 0 || semantic.entity_count == 0 {
                return Err(LexiconError::invalid_bundle(
                    "semantic dimensions and entity_count must be positive",
                ));
            }
            validate_file(root, &semantic.model)?;
            validate_tokenizer_file(root, &semantic.tokenizer)?;
            validate_file(root, &semantic.license)?;
            validate_vector_file(root, &semantic.identity_vectors, semantic)?;
            validate_vector_file(root, &semantic.knowledge_vectors, semantic)?;
            if semantic.model_contract.pooling != "mean" || !semantic.model_contract.normalize {
                return Err(LexiconError::invalid_bundle(
                    "schema v2 semantic contract requires mean pooling and normalization",
                ));
            }
        }
        let total = self.ranking.semantic_weight
            + self.ranking.context_weight
            + self.ranking.popularity_weight;
        if !total.is_finite() || (total - 1.0).abs() > 0.001 {
            return Err(LexiconError::invalid_bundle(
                "semantic ranking weights must sum to 1",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn path_for(&self, root: &Path, file: &BundleFile) -> PathBuf {
        root.join(&file.file)
    }

    /// Verifies one bundle asset without trusting its file name or size alone.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or its digest does not match.
    pub fn verify_checksum(&self, root: &Path, file: &BundleFile) -> LexiconResult<()> {
        let path = self.path_for(root, file);
        let mut stream = File::open(&path).map_err(|error| {
            LexiconError::invalid_bundle(format!("failed to open {}: {error}", path.display()))
        })?;
        let mut digest = Sha256::new();
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            let read = stream.read(&mut buffer).map_err(|error| {
                LexiconError::invalid_bundle(format!("failed to read {}: {error}", path.display()))
            })?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
        if format!("{:x}", digest.finalize()) != file.sha256.to_ascii_lowercase() {
            return Err(LexiconError::invalid_bundle(format!(
                "SHA-256 mismatch for {}",
                path.display()
            )));
        }
        Ok(())
    }
}

fn validate_tokenizer_file(root: &Path, tokenizer: &TokenizerFile) -> LexiconResult<()> {
    validate_file(root, &tokenizer.bundle)?;
    if !tokenizer.bundle.file.ends_with(".json.zst")
        || tokenizer.content_size_bytes == 0
        || tokenizer.content_size_bytes > MAX_TOKENIZER_CONTENT_BYTES
        || !is_sha256(&tokenizer.content_sha256)
    {
        return Err(LexiconError::invalid_bundle(
            "invalid zstd JSON tokenizer metadata",
        ));
    }
    Ok(())
}

fn validate_file(root: &Path, file: &BundleFile) -> LexiconResult<()> {
    if file.file.contains("..") || Path::new(&file.file).is_absolute() {
        return Err(LexiconError::invalid_bundle(format!(
            "bundle path is not relative: {}",
            file.file
        )));
    }
    let path = root.join(&file.file);
    let metadata = fs::metadata(&path).map_err(|error| {
        LexiconError::invalid_bundle(format!("missing bundle file {}: {error}", path.display()))
    })?;
    if metadata.len() != file.size_bytes {
        return Err(LexiconError::invalid_bundle(format!(
            "bundle file size mismatch for {}",
            path.display()
        )));
    }
    if !is_sha256(&file.sha256) {
        return Err(LexiconError::invalid_bundle(format!(
            "invalid SHA-256 for {}",
            file.file
        )));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_vector_file(
    root: &Path,
    file: &BundleFile,
    semantic: &SemanticManifest,
) -> LexiconResult<()> {
    validate_file(root, file)?;
    let expected = semantic
        .entity_count
        .checked_mul(semantic.dimensions)
        .and_then(|count| count.checked_mul(2))
        .ok_or_else(|| LexiconError::invalid_bundle("semantic vector size overflow"))?;
    if file.size_bytes != expected as u64 {
        return Err(LexiconError::invalid_bundle(format!(
            "{} has {} bytes; expected {expected}",
            file.file, file.size_bytes
        )));
    }
    Ok(())
}

const fn default_max_length() -> usize {
    512
}

fn default_query_prefix() -> String {
    "query: ".to_owned()
}

fn default_passage_prefix() -> String {
    "passage: ".to_owned()
}

fn default_input_ids() -> String {
    "input_ids".to_owned()
}

fn default_attention_mask() -> String {
    "attention_mask".to_owned()
}

fn default_pooling() -> String {
    "mean".to_owned()
}

const fn default_true() -> bool {
    true
}

const fn default_semantic_weight() -> f32 {
    0.85
}

const fn default_context_weight() -> f32 {
    0.10
}

const fn default_popularity_weight() -> f32 {
    0.05
}
