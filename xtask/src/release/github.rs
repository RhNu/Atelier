use std::path::Path;
use std::thread;
use std::time::Duration;

use semver::Version;
use serde::Deserialize;

use super::git::{output, run_live};

const GH_MAX_RETRIES: usize = 10;
const GH_RETRY_DELAY: Duration = Duration::from_secs(3);

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FoundRelease {
    url: String,
    is_draft: bool,
}

fn gh_capture(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = gh_output(root, args)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn gh_output(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let command = format!("gh {}", args.join(" "));
    retry_gh(&command, || {
        let output = output(root, "gh", args)?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(format!(
                "`gh {}` exited with {}: {}",
                args.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    })
}

fn gh_run(root: &Path, args: &[&str]) -> Result<(), String> {
    gh_output(root, args).map(|_| ())
}

fn gh_run_live(root: &Path, args: &[&str]) -> Result<(), String> {
    let command = format!("gh {}", args.join(" "));
    retry_gh(&command, || run_live(root, "gh", args))
}

fn retry_gh<T, F>(command: &str, mut operation: F) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
{
    retry_gh_with(command, &mut operation, thread::sleep)
}

fn retry_gh_with<T, F, S>(command: &str, mut operation: F, mut sleep: S) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
    S: FnMut(Duration),
{
    for retry in 0..=GH_MAX_RETRIES {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) if retry < GH_MAX_RETRIES && is_retryable_gh_error(&error) => {
                eprintln!(
                    "GitHub command `{command}` failed: {error}; retrying in {} seconds ({}/{})",
                    GH_RETRY_DELAY.as_secs(),
                    retry + 1,
                    GH_MAX_RETRIES
                );
                sleep(GH_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("retry loop always returns after the final attempt")
}

fn is_retryable_gh_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    [
        "eof",
        "fetch failed",
        "timed out",
        "timeout",
        "connection aborted",
        "connection closed",
        "connection refused",
        "connection reset",
        "network is unreachable",
        "no such host",
        "temporary failure",
        "broken pipe",
        "tls",
        "transport error",
        "http 500",
        "http 502",
        "http 503",
        "http 504",
        "bad gateway",
        "service unavailable",
        "gateway timeout",
    ]
    .iter()
    .any(|marker| error.contains(marker))
}

pub fn ensure_release_prerequisites(root: &Path) -> Result<GitHubContext, String> {
    gh_capture(root, &["auth", "status", "--active"])?;
    let repository = gh_capture(
        root,
        &[
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "--jq",
            ".nameWithOwner",
        ],
    )?;
    let default_branch = gh_capture(
        root,
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
    gh_capture(
        root,
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
    let output = match gh_output(
        root,
        &[
            "release",
            "view",
            &tag,
            "-R",
            &github.repository,
            "--json",
            "url,isDraft",
        ],
    ) {
        Ok(output) => output,
        Err(error) if error.contains("release not found") || error.contains("HTTP 404") => {
            return Ok(None);
        }
        Err(error) => {
            return Err(format!("failed to inspect GitHub release {tag}: {error}"));
        }
    };
    let found: FoundRelease = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid gh release JSON: {error}"))?;
    Ok((!found.is_draft).then_some(ReleaseView { url: found.url }))
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
    let output = gh_capture(
        root,
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
    let result = gh_capture(
        root,
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
    gh_run_live(
        root,
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
    let json = gh_capture(
        root,
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
    gh_run(root, &args)
}

pub fn show_failed_logs(root: &Path, github: &GitHubContext, run_id: u64) {
    let id = run_id.to_string();
    let _ = gh_run_live(
        root,
        &["run", "view", &id, "--log-failed", "-R", &github.repository],
    );
}

fn list_runs(root: &Path, repository: &str, filters: &[&str]) -> Result<Vec<WorkflowRun>, String> {
    let mut args = vec!["run", "list"];
    args.extend_from_slice(filters);
    args.extend_from_slice(&["--limit", "20", "--json", RUN_FIELDS, "-R", repository]);
    let json = gh_capture(root, &args)?;
    serde_json::from_str(&json).map_err(|error| format!("invalid gh run JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{GH_MAX_RETRIES, dispatch_run_id, is_retryable_gh_error, retry_gh_with};

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

    #[test]
    fn retries_transient_gh_failures_ten_times_before_succeeding() {
        let mut attempts = 0;
        let mut waits = Vec::new();
        let result = retry_gh_with(
            "gh test",
            || {
                attempts += 1;
                if attempts <= GH_MAX_RETRIES {
                    Err("request failed: EOF".to_owned())
                } else {
                    Ok::<_, String>("success")
                }
            },
            |duration: Duration| waits.push(duration),
        )
        .unwrap();

        assert_eq!(result, "success");
        assert_eq!(attempts, GH_MAX_RETRIES + 1);
        assert_eq!(waits, vec![Duration::from_secs(3); GH_MAX_RETRIES]);
    }

    #[test]
    fn does_not_retry_non_network_gh_failures() {
        let mut attempts = 0;
        let result = retry_gh_with(
            "gh test",
            || {
                attempts += 1;
                Err::<(), _>("permission denied: HTTP 403".to_owned())
            },
            |_| panic!("non-network failures must not wait"),
        );

        assert_eq!(attempts, 1);
        assert!(result.is_err());
        assert!(is_retryable_gh_error("Get https://api.github.com: EOF"));
        assert!(!is_retryable_gh_error("HTTP 422: validation failed"));
    }
}
