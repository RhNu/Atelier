use std::fs;
use std::path::{Path, PathBuf};

use semver::Version;
use serde::{Deserialize, Serialize};

use super::{GitSnapshot, request_id};

const STATE_FORMAT: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReleaseState {
    format: u32,
    pub selector: String,
    pub version: String,
    pub request_id: String,
    pub base_sha: Option<String>,
    pub source_sha: Option<String>,
    pub ci_run: Option<u64>,
    pub release_run: Option<u64>,
    pub release_url: Option<String>,
    pub completed: bool,
}

impl ReleaseState {
    pub fn new(selector: String, version: String) -> Self {
        Self {
            format: STATE_FORMAT,
            selector,
            request_id: request_id(&version),
            version,
            base_sha: None,
            source_sha: None,
            ci_run: None,
            release_run: None,
            release_url: None,
            completed: false,
        }
    }

    pub fn reconcile(
        &mut self,
        git: &GitSnapshot,
        current: &Version,
        target: &Version,
    ) -> Result<(), String> {
        if current > target {
            return Err(format!(
                "workspace version {current} is newer than unfinished release {target}"
            ));
        }
        self.base_sha.get_or_insert_with(|| git.origin_sha.clone());
        if current == target
            && git.clean
            && self.release_run.is_none()
            && (self.source_sha.is_none() || git.head_sha == git.origin_sha)
        {
            self.source_sha = Some(git.head_sha.clone());
        }
        Ok(())
    }

    pub fn restart_request(&mut self) {
        self.request_id = request_id(&self.version);
        self.release_run = None;
        self.release_url = None;
    }
}

pub fn load_state(root: &Path) -> Result<Option<ReleaseState>, String> {
    let path = state_path(root)?;
    if !path.is_file() {
        return Ok(None);
    }
    let state: ReleaseState =
        serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
            .map_err(|error| format!("invalid release checkpoint: {error}"))?;
    if state.format != STATE_FORMAT {
        return Err(format!(
            "unsupported release checkpoint format {}",
            state.format
        ));
    }
    Ok(Some(state))
}

pub fn save_state(root: &Path, state: &ReleaseState) -> Result<(), String> {
    let path = state_path(root)?;
    fs::create_dir_all(path.parent().expect("release state has a parent"))
        .map_err(|error| error.to_string())?;
    let mut bytes = serde_json::to_vec_pretty(state).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn state_path(root: &Path) -> Result<PathBuf, String> {
    let git_dir = super::git::capture(root, "git", &["rev-parse", "--git-dir"])?;
    let git_dir = PathBuf::from(git_dir);
    let git_dir = if git_dir.is_absolute() {
        git_dir
    } else {
        root.join(git_dir)
    };
    Ok(git_dir.join("atelier/release-state.json"))
}
