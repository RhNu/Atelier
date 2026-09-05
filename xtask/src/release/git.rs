use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;

use semver::Version;

use super::state::ReleaseState;

#[derive(Clone, Debug)]
pub struct GitSnapshot {
    pub head_sha: String,
    pub origin_sha: String,
    pub clean: bool,
}

pub fn preflight_git(
    root: &Path,
    target: &Version,
    state: &ReleaseState,
) -> Result<GitSnapshot, String> {
    if capture(root, "git", &["branch", "--show-current"])? != "main" {
        return Err("application releases must run from the main branch".to_owned());
    }
    run(root, "git", &["fetch", "--quiet", "origin", "main"])?;
    let head_sha = capture(root, "git", &["rev-parse", "HEAD"])?;
    let origin_sha = capture(root, "git", &["rev-parse", "origin/main"])?;
    let status = capture(root, "git", &["status", "--porcelain=v1"])?;
    let clean = status.is_empty();
    let recognized_version_edit = status
        .lines()
        .all(|line| line.len() > 3 && line[3..].replace('\\', "/") == "apps/desktop/package.json")
        && !status.is_empty();
    if !(clean || recognized_version_edit && state.version == target.to_string()) {
        return Err(format!(
            "working tree contains unrelated changes; release requires a clean main branch:\n{status}"
        ));
    }
    let recoverable_release_commit = state.source_sha.is_none()
        && clean
        && state.base_sha.as_deref() == Some(origin_sha.as_str())
        && capture(root, "git", &["diff", "--name-only", "origin/main..HEAD"])?
            .trim()
            .replace('\\', "/")
            == "apps/desktop/package.json";
    if state.source_sha.is_none() && clean && head_sha != origin_sha && !recoverable_release_commit
    {
        return Err(
            "local main must exactly match origin/main before preparing a release".to_owned(),
        );
    }
    if !clean && head_sha != origin_sha {
        return Err(
            "cannot resume the version edit because main moved; inspect the working tree manually"
                .to_owned(),
        );
    }
    Ok(GitSnapshot {
        head_sha,
        origin_sha,
        clean,
    })
}

pub fn commit_version(root: &Path, version: &Version) -> Result<String, String> {
    let changed = capture(root, "git", &["diff", "--name-only", "HEAD"])?;
    if changed.trim().replace('\\', "/") != "apps/desktop/package.json" {
        return Err(format!(
            "expected only apps/desktop/package.json to change; found:\n{changed}"
        ));
    }
    run(root, "git", &["add", "--", "apps/desktop/package.json"])?;
    let message = format!("chore(release): prepare {version}");
    run(root, "git", &["commit", "-m", &message])?;
    let committed = capture(
        root,
        "git",
        &["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"],
    )?;
    if committed.trim().replace('\\', "/") != "apps/desktop/package.json" {
        return Err(format!(
            "release commit contains unexpected files and was not pushed:\n{committed}"
        ));
    }
    capture(root, "git", &["rev-parse", "HEAD"])
}

pub fn push_release(root: &Path, snapshot: &GitSnapshot, source_sha: &str) -> Result<(), String> {
    let remote = capture(root, "git", &["rev-parse", "origin/main"])?;
    if remote == source_sha
        || succeeds(
            root,
            "git",
            &["merge-base", "--is-ancestor", source_sha, "origin/main"],
        )?
    {
        return Ok(());
    }
    if remote != snapshot.origin_sha {
        return Err(
            "origin/main moved during release preparation; no push was attempted".to_owned(),
        );
    }
    let head = capture(root, "git", &["rev-parse", "HEAD"])?;
    if head != source_sha {
        return Err("local HEAD no longer matches the prepared release commit".to_owned());
    }
    run(root, "git", &["push", "origin", "HEAD:main"])
}

pub fn capture(root: &Path, program: &str, args: &[&str]) -> Result<String, String> {
    let output = output(root, program, args)?;
    output_text(program, args, &output)
}

pub fn run(root: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    let output = output(root, program, args)?;
    output_text(program, args, &output).map(|_| ())
}

pub fn run_live(root: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    let mut child = command(root, program, args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start `{program}`: {error}"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("failed to capture `{program}` stderr"))?;
    let stderr_reader = thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buffer = [0_u8; 8 * 1024];
        loop {
            let read = stderr
                .read(&mut buffer)
                .map_err(|error| format!("failed to read command stderr: {error}"))?;
            if read == 0 {
                break;
            }
            io::stderr()
                .write_all(&buffer[..read])
                .map_err(|error| format!("failed to write command stderr: {error}"))?;
            io::stderr()
                .flush()
                .map_err(|error| format!("failed to flush command stderr: {error}"))?;
            captured.extend_from_slice(&buffer[..read]);
        }
        Ok::<_, String>(captured)
    });
    let status = child
        .wait()
        .map_err(|error| format!("failed to wait for `{program}`: {error}"))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| format!("failed to join `{program}` stderr reader"))??;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{program} {}` exited with {status}: {}",
            args.join(" "),
            String::from_utf8_lossy(&stderr).trim()
        ))
    }
}

pub fn output(root: &Path, program: &str, args: &[&str]) -> Result<Output, String> {
    command(root, program, args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to start `{program}`: {error}"))
}

fn succeeds(root: &Path, program: &str, args: &[&str]) -> Result<bool, String> {
    command(root, program, args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .map_err(|error| format!("failed to start `{program}`: {error}"))
}

fn command(root: &Path, program: &str, args: &[&str]) -> Command {
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(root)
        .env("GH_PROMPT_DISABLED", "1");
    command
}

fn output_text(program: &str, args: &[&str], output: &Output) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(format!("`{program} {}` failed: {stderr}", args.join(" ")))
    }
}
