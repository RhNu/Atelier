use std::path::Path;
use std::thread;
use std::time::Duration;

use semver::Version;
use serde::Deserialize;

use super::git::{capture, run, run_live};

const RUN_FIELDS: &str = "databaseId,displayTitle,headSha,status,conclusion,url";

pub struct GitHubContext {
    pub repository: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRun {
    pub database_id: u64,
    #[serde(default)]
    display_title: String,
    #[serde(default)]
    head_sha: String,
    status: String,
    conclusion: Option<String>,
    pub url: String,
}

impl WorkflowRun {
    pub fn failed(&self) -> bool {
        self.status == "completed" && self.conclusion.as_deref() != Some("success")
    }

    pub fn supports_failed_only_rerun(&self) -> bool {
        self.conclusion.as_deref() == Some("failure")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseView {
    pub url: String,
}

pub fn ensure_release_prerequisites(root: &Path) -> Result<GitHubContext, String> {
    capture(root, "gh", &["auth", "status", "--active"])?;
    let repository = capture(
        root,
        "gh",
        &[
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "--jq",
            ".nameWithOwner",
        ],
    )?;
    let default_branch = capture(
        root,
        "gh",
        &[
            "repo",
            "view",
            "--json",
            "defaultBranchRef",
            "--jq",
            ".defaultBranchRef.name",
        ],
    )?;
    if default_branch != "main" {
        return Err(format!(
            "GitHub default branch is `{default_branch}`, expected `main`"
        ));
    }
    capture(
        root,
        "gh",
        &["workflow", "view", "release-app.yml", "-R", &repository],
    )?;
    Ok(GitHubContext { repository })
}

pub fn published_release(
    root: &Path,
    github: &GitHubContext,
    version: &Version,
) -> Result<Option<ReleaseView>, String> {
    let tag = format!("v{version}");
    let output = std::process::Command::new("gh")
        .args([
            "release",
            "view",
            &tag,
            "-R",
            &github.repository,
            "--json",
            "url,isDraft",
        ])
        .current_dir(root)
        .env("GH_PROMPT_DISABLED", "1")
        .output()
        .map_err(|error| format!("failed to start `gh`: {error}"))?;
    if output.status.success() {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FoundRelease {
            url: String,
            is_draft: bool,
        }
        let found: FoundRelease = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("invalid gh release JSON: {error}"))?;
        return Ok((!found.is_draft).then_some(ReleaseView { url: found.url }));
    }
    let error = String::from_utf8_lossy(&output.stderr);
    if error.contains("release not found") || error.contains("HTTP 404") {
        Ok(None)
    } else {
        Err(format!(
            "failed to inspect GitHub release {tag}: {}",
            error.trim()
        ))
    }
}

pub fn find_ci_run(root: &Path, github: &GitHubContext, sha: &str) -> Result<WorkflowRun, String> {
    for _ in 0..45 {
        let runs = list_runs(
            root,
            &github.repository,
            &["--workflow", "ci.yml", "--event", "push", "--commit", sha],
        )?;
        if let Some(run) = runs.into_iter().find(|run| run.head_sha == sha) {
            return Ok(run);
        }
        thread::sleep(Duration::from_secs(2));
    }
    Err(format!(
        "CI run for commit {sha} did not appear within 90 seconds"
    ))
}

pub fn dispatch_release(
    root: &Path,
    github: &GitHubContext,
    sha: &str,
    request_id: &str,
) -> Result<WorkflowRun, String> {
    let commit = format!("commit={sha}");
    let request = format!("request_id={request_id}");
    let output = capture(
        root,
        "gh",
        &[
            "workflow",
            "run",
            "release-app.yml",
            "--ref",
            "main",
            "--raw-field",
            &commit,
            "--raw-field",
            &request,
            "-R",
            &github.repository,
        ],
    )?;
    if let Some(id) = dispatch_run_id(&output) {
        return find_release_run(root, github, request_id, Some(id));
    }
    find_release_run(root, github, request_id, None)
}

pub fn find_release_run(
    root: &Path,
    github: &GitHubContext,
    request_id: &str,
    expected_id: Option<u64>,
) -> Result<WorkflowRun, String> {
    if let Some(run_id) = expected_id {
        let run = get_run(root, github, run_id)?;
        if !run.display_title.contains(request_id) {
            return Err(format!(
                "release checkpoint run {run_id} does not belong to request {request_id}"
            ));
        }
        return Ok(run);
    }
    for _ in 0..30 {
        let runs = list_runs(
            root,
            &github.repository,
            &[
                "--workflow",
                "release-app.yml",
                "--event",
                "workflow_dispatch",
            ],
        )?;
        if let Some(run) = runs.into_iter().find(|run| {
            expected_id == Some(run.database_id) || run.display_title.contains(request_id)
        }) {
            return Ok(run);
        }
        thread::sleep(Duration::from_secs(2));
    }
    Err(format!(
        "release workflow for request {request_id} did not appear within 60 seconds"
    ))
}

fn dispatch_run_id(output: &str) -> Option<u64> {
    output.split_whitespace().find_map(|word| {
        if !word.contains("/actions/runs/") {
            return None;
        }
        word.trim_end_matches('/')
            .rsplit('/')
            .next()
            .and_then(|part| part.parse().ok())
    })
}

pub fn find_existing_release_run(
    root: &Path,
    github: &GitHubContext,
    request_id: &str,
) -> Result<Option<WorkflowRun>, String> {
    let runs = list_runs(
        root,
        &github.repository,
        &[
            "--workflow",
            "release-app.yml",
            "--event",
            "workflow_dispatch",
        ],
    )?;
    Ok(runs
        .into_iter()
        .find(|run| run.display_title.contains(request_id)))
}

pub fn release_artifact_expired(
    root: &Path,
    github: &GitHubContext,
    run_id: u64,
) -> Result<bool, String> {
    let endpoint = format!(
        "repos/{}/actions/runs/{run_id}/artifacts",
        github.repository
    );
    let result = capture(
        root,
        "gh",
        &[
            "api",
            &endpoint,
            "--jq",
            "[.artifacts[] | select(.name == \"application-release\") | .expired] | any",
        ],
    )?;
    match result.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("unexpected artifact expiration response: {result}")),
    }
}

pub fn watch_run(
    root: &Path,
    github: &GitHubContext,
    run: &WorkflowRun,
    quiet: bool,
) -> Result<(), String> {
    if quiet {
        return watch_run_quiet(root, github, run.database_id);
    }
    let id = run.database_id.to_string();
    run_live(
        root,
        "gh",
        &[
            "run",
            "watch",
            &id,
            "--compact",
            "--exit-status",
            "-R",
            &github.repository,
        ],
    )
}

fn watch_run_quiet(root: &Path, github: &GitHubContext, run_id: u64) -> Result<(), String> {
    for _ in 0..1_440 {
        let run = get_run(root, github, run_id)?;
        if run.status == "completed" {
            return if run.conclusion.as_deref() == Some("success") {
                Ok(())
            } else {
                Err(format!("run {run_id} concluded as {:?}", run.conclusion))
            };
        }
        thread::sleep(Duration::from_secs(5));
    }
    Err(format!("run {run_id} did not finish within two hours"))
}

pub fn get_run(root: &Path, github: &GitHubContext, run_id: u64) -> Result<WorkflowRun, String> {
    let id = run_id.to_string();
    let json = capture(
        root,
        "gh",
        &[
            "run",
            "view",
            &id,
            "--json",
            RUN_FIELDS,
            "-R",
            &github.repository,
        ],
    )?;
    serde_json::from_str(&json).map_err(|error| format!("invalid gh run JSON: {error}"))
}

pub fn rerun(
    root: &Path,
    github: &GitHubContext,
    run_id: u64,
    failed_only: bool,
) -> Result<(), String> {
    let id = run_id.to_string();
    let mut args = vec!["run", "rerun", &id];
    if failed_only {
        args.push("--failed");
    }
    args.extend_from_slice(&["-R", &github.repository]);
    run(root, "gh", &args)
}

pub fn show_failed_logs(root: &Path, github: &GitHubContext, run_id: u64) {
    let id = run_id.to_string();
    let _ = run_live(
        root,
        "gh",
        &["run", "view", &id, "--log-failed", "-R", &github.repository],
    );
}

fn list_runs(root: &Path, repository: &str, filters: &[&str]) -> Result<Vec<WorkflowRun>, String> {
    let mut args = vec!["run", "list"];
    args.extend_from_slice(filters);
    args.extend_from_slice(&["--limit", "20", "--json", RUN_FIELDS, "-R", repository]);
    let json = capture(root, "gh", &args)?;
    serde_json::from_str(&json).map_err(|error| format!("invalid gh run JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::dispatch_run_id;

    #[test]
    fn extracts_dispatch_run_id_from_plain_or_decorated_output() {
        assert_eq!(
            dispatch_run_id("https://github.com/owner/repo/actions/runs/12345"),
            Some(12_345)
        );
        assert_eq!(
            dispatch_run_id("Created run: https://github.com/owner/repo/actions/runs/98\n"),
            Some(98)
        );
        assert_eq!(dispatch_run_id("workflow dispatched"), None);
    }
}
