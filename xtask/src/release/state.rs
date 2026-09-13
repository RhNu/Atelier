use std::fs::{self, OpenOptions};
use std::io::Write;
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
        if current == target && git.clean && self.release_run.is_none() && self.source_sha.is_none()
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

    pub fn restart_from_main(&mut self, git: &GitSnapshot) {
        self.base_sha = Some(git.head_sha.clone());
        self.source_sha = Some(git.head_sha.clone());
        self.ci_run = None;
        self.completed = false;
        self.restart_request();
    }

    pub fn complete(&mut self, release_url: String) {
        self.release_url = Some(release_url);
        self.completed = true;
    }
}

pub fn load_state(root: &Path) -> Result<Option<ReleaseState>, String> {
    let path = state_path(root)?;
    load_state_path(&path)
}

fn load_state_path(path: &Path) -> Result<Option<ReleaseState>, String> {
    if !path.is_file() {
        let backup = backup_path(path);
        return if backup.is_file() {
            read_state(&backup).map(Some)
        } else {
            Ok(None)
        };
    }
    match read_state(path) {
        Ok(state) => Ok(Some(state)),
        Err(primary_error) => {
            let backup = backup_path(path);
            if backup.is_file() {
                read_state(&backup).map(Some).map_err(|backup_error| {
                    format!("{primary_error}; backup checkpoint is also invalid: {backup_error}")
                })
            } else {
                Err(primary_error)
            }
        }
    }
}

fn read_state(path: &Path) -> Result<ReleaseState, String> {
    let state: ReleaseState = serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("could not read release checkpoint: {error}"))?,
    )
    .map_err(|error| format!("invalid release checkpoint: {error}"))?;
    if state.format != STATE_FORMAT {
        return Err(format!(
            "unsupported release checkpoint format {}",
            state.format
        ));
    }
    validate_state(&state)?;
    Ok(state)
}

pub fn save_state(root: &Path, state: &ReleaseState) -> Result<(), String> {
    validate_state(state)?;
    let path = state_path(root)?;
    fs::create_dir_all(path.parent().expect("release state has a parent"))
        .map_err(|error| error.to_string())?;
    let mut bytes = serde_json::to_vec_pretty(state).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    replace_state_file(&path, &bytes)
}

fn replace_state_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let backup = backup_path(path);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("could not create release checkpoint: {error}"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("could not write release checkpoint: {error}"))?;
    drop(file);

    if path.is_file() {
        if read_state(path).is_ok() {
            fs::copy(path, &backup)
                .map_err(|error| format!("could not back up release checkpoint: {error}"))?;
        } else if !backup.is_file() {
            return Err(
                "cannot replace an invalid release checkpoint without a valid backup".to_owned(),
            );
        }
        fs::remove_file(path)
            .map_err(|error| format!("could not replace release checkpoint: {error}"))?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if backup.is_file() && !path.exists() {
            let _ = fs::copy(&backup, path);
        }
        return Err(format!("could not install release checkpoint: {error}"));
    }
    if backup.is_file() {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

fn validate_state(state: &ReleaseState) -> Result<(), String> {
    let version = Version::parse(&state.version)
        .map_err(|error| format!("invalid checkpoint version: {error}"))?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err("release checkpoint version must be a stable SemVer".to_owned());
    }
    if state.selector.trim().is_empty() {
        return Err("release checkpoint selector is empty".to_owned());
    }
    if state.request_id.trim().is_empty() {
        return Err("release checkpoint request ID is empty".to_owned());
    }
    for (name, sha) in [
        ("base", state.base_sha.as_deref()),
        ("source", state.source_sha.as_deref()),
    ] {
        if let Some(sha) = sha
            && (sha.len() != 40 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(format!(
                "release checkpoint {name} SHA must be 40 hexadecimal characters"
            ));
        }
    }
    if (state.ci_run.is_some() || state.release_run.is_some()) && state.source_sha.is_none() {
        return Err("release checkpoint has a workflow run without a source SHA".to_owned());
    }
    Ok(())
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
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

#[cfg(test)]
mod tests {
    use std::fs;

    use semver::Version;
    use tempfile::tempdir;

    use super::{
        GitSnapshot, ReleaseState, backup_path, load_state_path, replace_state_file, validate_state,
    };

    fn state() -> ReleaseState {
        let mut state = ReleaseState::new("patch".to_owned(), "0.6.0".to_owned());
        state.base_sha = Some("a".repeat(40));
        state.source_sha = Some("b".repeat(40));
        state
    }

    #[test]
    fn validates_checkpoint_invariants() {
        let mut state = state();
        assert!(validate_state(&state).is_ok());

        state.version = "0.6.0-beta.1".to_owned();
        assert!(validate_state(&state).is_err());
        state.version = "0.6.0".to_owned();
        state.source_sha = Some("short".to_owned());
        assert!(validate_state(&state).is_err());
        state.source_sha = None;
        state.ci_run = Some(42);
        assert!(validate_state(&state).is_err());
    }

    #[test]
    fn falls_back_to_backup_after_an_interrupted_replacement() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("release-state.json");
        let bytes = serde_json::to_vec_pretty(&state()).unwrap();
        replace_state_file(&path, &bytes).unwrap();
        fs::copy(&path, backup_path(&path)).unwrap();
        fs::write(&path, b"truncated").unwrap();

        let recovered = load_state_path(&path).unwrap().unwrap();
        assert_eq!(recovered.version, "0.6.0");
        assert_eq!(recovered.source_sha, Some("b".repeat(40)));
    }

    #[test]
    fn resume_keeps_frozen_source_until_restart_is_explicit() {
        let mut state = state();
        let original_source = state.source_sha.clone();
        let git = GitSnapshot {
            head_sha: "c".repeat(40),
            origin_sha: "c".repeat(40),
            clean: true,
        };
        let version = Version::parse("0.6.0").unwrap();

        state.reconcile(&git, &version, &version).unwrap();
        assert_eq!(state.source_sha, original_source);

        state.restart_from_main(&git);
        assert_eq!(state.source_sha, Some("c".repeat(40)));
        assert!(state.ci_run.is_none());
        assert!(state.release_run.is_none());
    }
}
