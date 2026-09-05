mod git;
mod github;
mod state;

use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use atelier_downloadable_resources::{DownloadableResourceCatalog, validate_catalog};
use semver::Version;
use serde::{Deserialize, Serialize};

use self::git::{GitSnapshot, commit_version, preflight_git, push_release};
use self::github::{
    GitHubContext, dispatch_release, ensure_release_prerequisites, find_ci_run,
    find_existing_release_run, find_release_run, get_run, published_release,
    release_artifact_expired, rerun, show_failed_logs, watch_run,
};
use self::state::{ReleaseState, load_state, save_state};

#[derive(Deserialize)]
struct PackageManifest {
    version: String,
}

#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct ApplicationReleaseRequest {
    pub selector: String,
    pub dry_run: bool,
    pub yes: bool,
    pub no_wait: bool,
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct ApplicationReleaseOutcome {
    version: String,
    source_sha: String,
    ci_run: Option<u64>,
    release_run: Option<u64>,
    release_url: Option<String>,
    published: bool,
    dry_run: bool,
}

struct Reporter {
    json: bool,
}

impl Reporter {
    fn stage(&self, message: &str) {
        if !self.json {
            println!("==> {message}");
        }
    }

    fn detail(&self, message: &str) {
        if !self.json {
            println!("    {message}");
        }
    }
}

/// Resolves a stable release selector against the current application version.
///
/// # Errors
/// Returns an error for invalid selectors, unstable versions, or non-increasing explicit versions.
pub fn resolve_release_version(current: &Version, selector: &str) -> Result<Version, String> {
    let mut next = current.clone();
    match selector {
        "patch" => {
            next.patch = next.patch.checked_add(1).ok_or("patch version overflow")?;
        }
        "minor" => {
            next.minor = next.minor.checked_add(1).ok_or("minor version overflow")?;
            next.patch = 0;
        }
        "major" => {
            next.major = next.major.checked_add(1).ok_or("major version overflow")?;
            next.minor = 0;
            next.patch = 0;
        }
        explicit => {
            next = Version::parse(explicit)
                .map_err(|error| format!("invalid release selector `{explicit}`: {error}"))?;
            if next <= *current {
                return Err(format!(
                    "release version {next} must be newer than current version {current}"
                ));
            }
        }
    }
    require_stable(&next)?;
    Ok(next)
}

/// Runs the local release orchestration and delegates build/sign/publish to GitHub Actions.
///
/// # Errors
/// Returns an actionable error when a local or remote release stage cannot be completed safely.
pub fn run_application_release(
    root: &Path,
    request: &ApplicationReleaseRequest,
) -> Result<(), String> {
    let reporter = Reporter { json: request.json };
    reporter.stage("Inspecting release state");
    let current = read_application_version(root)?;
    let saved = load_state(root)?;
    if let Some(saved) = &saved
        && !saved.completed
        && saved.selector != request.selector
    {
        return Err(format!(
            "unfinished Atelier {} release uses selector `{}`; rerun that selector before starting `{}`",
            saved.version, saved.selector, request.selector
        ));
    }
    let mut state = if let Some(saved) = saved.filter(|saved| !saved.completed) {
        reporter.detail(&format!("Resuming Atelier {}", saved.version));
        saved
    } else {
        let version = resolve_release_version(&current, &request.selector)?;
        ReleaseState::new(request.selector.clone(), version.to_string())
    };
    let target = Version::parse(&state.version).map_err(|error| error.to_string())?;

    reporter.stage("Running Git and GitHub preflight checks");
    let git = preflight_git(root, &target, &state)?;
    let github = ensure_release_prerequisites(root)?;
    state.reconcile(&git, &current, &target)?;

    reporter.detail(&format!("Repository: {}", github.repository));
    reporter.detail(&format!("Version: {current} -> {target}"));
    reporter.detail(&format!("Main SHA: {}", short_sha(&git.origin_sha)));
    if request.dry_run {
        return emit_outcome(
            request,
            &ApplicationReleaseOutcome {
                version: target.to_string(),
                source_sha: git.head_sha,
                ci_run: None,
                release_run: None,
                release_url: None,
                published: false,
                dry_run: true,
            },
        );
    }
    confirm(request, &target)?;

    if published_release(root, &github, &target)?.is_some() {
        reporter.stage(&format!("Atelier {target} is already published"));
        state.completed = true;
        save_state(root, &state)?;
        return emit_outcome(request, &outcome(&state, true, false));
    }

    let source_sha = prepare_source(root, &reporter, &current, &target, &git, &mut state)?;
    wait_for_ci(root, request, &reporter, &github, &source_sha, &mut state)?;
    let published = run_release_workflow(
        root,
        request,
        &reporter,
        &github,
        &source_sha,
        &target,
        &mut state,
    )?;
    emit_outcome(request, &outcome(&state, published, false))
}

fn prepare_source(
    root: &Path,
    reporter: &Reporter,
    current: &Version,
    target: &Version,
    git: &GitSnapshot,
    state: &mut ReleaseState,
) -> Result<String, String> {
    if current != target && state.source_sha.is_none() {
        reporter.stage(&format!("Preparing Atelier {target}"));
        save_state(root, state)?;
        update_application_version(root, &state.version)?;
    }
    if state.source_sha.is_none() {
        reporter.stage("Creating the release commit");
        state.source_sha = Some(commit_version(root, target)?);
        save_state(root, state)?;
    }

    let source_sha = state
        .source_sha
        .clone()
        .ok_or("release commit was not established")?;
    reporter.stage(&format!(
        "Pushing {} to origin/main",
        short_sha(&source_sha)
    ));
    push_release(root, git, &source_sha)?;
    save_state(root, state)?;
    Ok(source_sha)
}

fn wait_for_ci(
    root: &Path,
    request: &ApplicationReleaseRequest,
    reporter: &Reporter,
    github: &GitHubContext,
    source_sha: &str,
    state: &mut ReleaseState,
) -> Result<(), String> {
    reporter.stage("Waiting for CI on the exact release commit");
    let ci = find_ci_run(root, github, source_sha)?;
    state.ci_run = Some(ci.database_id);
    save_state(root, state)?;
    reporter.detail(&ci.url);
    if let Err(watch_error) = watch_run(root, github, &ci, request.json) {
        let refreshed = get_run(root, github, ci.database_id)?;
        if !refreshed.failed() {
            return Err(format!(
                "could not monitor CI run {}: {watch_error}; inspect {} and rerun this command",
                ci.database_id, ci.url
            ));
        }
        show_failed_logs(root, github, ci.database_id);
        return Err(format!(
            "CI failed for {}; fix main and rerun `cargo xtask release {}`",
            short_sha(source_sha),
            request.selector
        ));
    }
    Ok(())
}

fn run_release_workflow(
    root: &Path,
    request: &ApplicationReleaseRequest,
    reporter: &Reporter,
    github: &GitHubContext,
    source_sha: &str,
    target: &Version,
    state: &mut ReleaseState,
) -> Result<bool, String> {
    let release_run = if let Some(run_id) = state.release_run {
        let run = find_release_run(root, github, &state.request_id, Some(run_id))?;
        if run.failed() {
            if release_artifact_expired(root, github, run_id)? {
                reporter.stage("The saved artifact expired; dispatching a fresh release run");
                state.restart_request();
                let restarted = dispatch_release(root, github, source_sha, &state.request_id)?;
                state.release_run = Some(restarted.database_id);
                state.release_url = Some(restarted.url.clone());
                save_state(root, state)?;
                restarted
            } else {
                reporter.stage(&format!("Re-running failed jobs for release run {run_id}"));
                rerun(root, github, run_id, run.supports_failed_only_rerun())?;
                run
            }
        } else {
            run
        }
    } else if let Some(existing) = find_existing_release_run(root, github, &state.request_id)? {
        state.release_run = Some(existing.database_id);
        state.release_url = Some(existing.url.clone());
        save_state(root, state)?;
        existing
    } else {
        reporter.stage("Dispatching Release application");
        let run = dispatch_release(root, github, source_sha, &state.request_id)?;
        state.release_run = Some(run.database_id);
        state.release_url = Some(run.url.clone());
        save_state(root, state)?;
        run
    };
    reporter.detail(&release_run.url);

    if request.no_wait {
        return Ok(false);
    }
    reporter.stage("Waiting for prepare, build, and publish");
    if let Err(watch_error) = watch_run(root, github, &release_run, request.json) {
        let refreshed = get_run(root, github, release_run.database_id)?;
        if !refreshed.failed() {
            return Err(format!(
                "could not monitor release run {}: {watch_error}; inspect {} and rerun this command",
                release_run.database_id, release_run.url
            ));
        }
        show_failed_logs(root, github, release_run.database_id);
        return Err(format!(
            "release run {} failed; rerun this command to retry failed jobs using the saved artifact",
            release_run.database_id
        ));
    }
    let release = published_release(root, github, target)?
        .ok_or_else(|| format!("workflow succeeded but GitHub release v{target} was not found"))?;
    state.release_url = Some(release.url);
    state.completed = true;
    save_state(root, state)?;
    reporter.stage(&format!("Published Atelier {target}"));
    Ok(true)
}

fn outcome(state: &ReleaseState, published: bool, dry_run: bool) -> ApplicationReleaseOutcome {
    ApplicationReleaseOutcome {
        version: state.version.clone(),
        source_sha: state.source_sha.clone().unwrap_or_default(),
        ci_run: state.ci_run,
        release_run: state.release_run,
        release_url: state.release_url.clone(),
        published,
        dry_run,
    }
}

fn emit_outcome(
    request: &ApplicationReleaseRequest,
    result: &ApplicationReleaseOutcome,
) -> Result<(), String> {
    if request.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
        );
    }
    Ok(())
}

fn confirm(request: &ApplicationReleaseRequest, target: &Version) -> Result<(), String> {
    if request.yes {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        return Err(
            "release confirmation requires an interactive terminal; pass --yes to continue"
                .to_owned(),
        );
    }
    eprintln!("Release Atelier {target}, commit its version, and push main? [y/N]");
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|error| error.to_string())?;
    if matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        Ok(())
    } else {
        Err("release cancelled before making changes".to_owned())
    }
}

fn read_application_version(root: &Path) -> Result<Version, String> {
    let source = fs::read_to_string(root.join("apps/desktop/package.json"))
        .map_err(|error| error.to_string())?;
    let package: PackageManifest =
        serde_json::from_str(&source).map_err(|error| error.to_string())?;
    let version = Version::parse(&package.version).map_err(|error| error.to_string())?;
    require_stable(&version)?;
    Ok(version)
}

fn require_stable(version: &Version) -> Result<(), String> {
    if version.pre.is_empty() && version.build.is_empty() {
        Ok(())
    } else {
        Err("application releases must use a stable SemVer".to_owned())
    }
}

fn short_sha(sha: &str) -> &str {
    sha.get(..8).unwrap_or(sha)
}

fn request_id(version: &str) -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    format!("atelier-{version}-{seconds}-{}", std::process::id())
}

/// Updates the single desktop application version source without committing or publishing.
///
/// # Errors
/// Returns an error for invalid/non-increasing versions or unreadable manifests.
pub fn prepare_app_release(root: &Path, version: &str) -> Result<(), String> {
    update_application_version(root, version)?;
    println!("Prepared Atelier {version}; commit, push to main, then run Release application.");
    Ok(())
}

fn update_application_version(root: &Path, version: &str) -> Result<(), String> {
    let next =
        Version::parse(version).map_err(|error| format!("invalid release version: {error}"))?;
    require_stable(&next)?;
    let path = root.join("apps/desktop/package.json");
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
    Ok(())
}

/// Checks the source catalog using the same contract as the runtime consumer.
/// Payload bytes are checked once during resource staging, not by this command.
///
/// # Errors
/// Returns an error when the catalog cannot be read or violates its domain contract.
pub fn validate_resource_catalog(root: &Path) -> Result<(), String> {
    let path = root.join("resources/catalog/catalog-v1.json");
    let catalog: DownloadableResourceCatalog =
        serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    validate_catalog(&catalog).map_err(|error| error.to_string())?;
    println!("Downloadable resource catalog is valid: {}", path.display());
    Ok(())
}
