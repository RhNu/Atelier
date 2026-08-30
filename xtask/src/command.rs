use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Args, Parser, Subcommand, error::ErrorKind};

use crate::{
    AppApiTypeExportConfig, LexiconBenchmarkConfig, LexiconBundleConfig, LineBudgetConfig,
    LineBudgetLevel, benchmark_lexicon, build_lexicon_bundle, check_line_budget,
    export_app_api_types, prepare_app_release, tag_app_release, tag_resource_release,
    validate_lexicon_bundle, validate_resource, validate_resource_catalog, validate_resource_tag,
};

const DEFAULT_WARN_LINES: usize = 600;
const DEFAULT_DENY_LINES: usize = 1200;

#[derive(Debug, Parser)]
#[command(about = "Atelier workspace automation")]
struct Xtask {
    #[command(subcommand)]
    command: XtaskCommand,
}

#[derive(Debug, Subcommand)]
enum XtaskCommand {
    #[command(about = "Generate frontend TypeScript bindings from app-api DTOs")]
    AppApi(AppApiArgs),
    #[command(about = "Check Rust source files against warning and deny line budgets")]
    LineBudget(LineBudgetArgs),
    #[command(about = "Build or check prompt lexicon assets")]
    Lexicon(LexiconArgs),
    #[command(about = "Prepare and tag Atelier application releases")]
    Release(ReleaseArgs),
    #[command(about = "Validate and tag downloadable resource releases")]
    Resource(ResourceArgs),
}

#[derive(Debug, Args)]
struct ReleaseArgs {
    #[command(subcommand)]
    command: ReleaseCommand,
}

#[derive(Debug, Subcommand)]
enum ReleaseCommand {
    #[command(about = "Validate and update the desktop package version")]
    Prepare { version: String },
    #[command(about = "Create the local v<version> tag from a clean version commit")]
    Tag,
}

#[derive(Debug, Args)]
struct ResourceArgs {
    #[command(subcommand)]
    command: ResourceCommand,
}

#[derive(Debug, Subcommand)]
enum ResourceCommand {
    #[command(about = "Validate the stable downloadable resource catalog")]
    Catalog,
    #[command(about = "Validate a pushed resource tag against its catalog descriptor")]
    CheckTag { tag: String },
    #[command(about = "Validate a catalog descriptor and its local payload")]
    Validate { id: String },
    #[command(about = "Validate and create a local resource-<id>-v<version> tag")]
    Tag { id: String },
}

#[derive(Debug, Args)]
struct AppApiArgs {
    #[command(subcommand)]
    command: AppApiCommand,
}

#[derive(Debug, Subcommand)]
enum AppApiCommand {
    #[command(about = "Generate app-api TypeScript bindings for the desktop frontend")]
    Types,
}

#[derive(Copy, Clone, Debug, Args)]
struct LineBudgetArgs {
    #[arg(long, default_value_t = DEFAULT_WARN_LINES)]
    warn_lines: usize,
    #[arg(long, default_value_t = DEFAULT_DENY_LINES)]
    deny_lines: usize,
}

#[derive(Debug, Args)]
struct LexiconArgs {
    #[command(subcommand)]
    command: LexiconCommand,
}

#[derive(Debug, Subcommand)]
enum LexiconCommand {
    #[command(about = "Build a runtime SQLite/vector bundle from normalized pipeline output")]
    Bundle(LexiconBundleArgs),
    #[command(about = "Validate checksums and runtime structure of a lexicon bundle")]
    Validate(LexiconValidateArgs),
    #[command(about = "Compare compact semantic search against a pinned BGE-M3 run")]
    Benchmark(LexiconBenchmarkArgs),
}

#[derive(Debug, Args)]
struct LexiconBundleArgs {
    #[arg(long, default_value = "tools/lexicon-pipeline/build")]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "dev")]
    bundle_version: String,
}

#[derive(Debug, Args)]
struct LexiconValidateArgs {
    #[arg(long)]
    bundle: PathBuf,
}

#[derive(Debug, Args)]
struct LexiconBenchmarkArgs {
    #[arg(long)]
    queries: PathBuf,
    #[arg(long)]
    candidate_run: PathBuf,
    #[arg(long)]
    baseline_run: PathBuf,
    #[arg(long)]
    bundle: PathBuf,
    #[arg(
        long,
        default_value = "apps/desktop/src-tauri/resources/onnx-runtime/onnxruntime.dll"
    )]
    runtime_library: PathBuf,
}

/// Runs `xtask` using process arguments and the current working directory.
///
/// # Errors
/// Returns an error when argument parsing fails or the selected task fails.
pub fn run_from_env() -> Result<(), String> {
    run(std::env::args_os())
}

/// Runs `xtask` using explicit arguments and the current working directory.
///
/// # Errors
/// Returns an error when argument parsing fails, the current directory cannot
/// be read, or the selected task fails.
pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), String> {
    let workspace_root = std::env::current_dir().map_err(|error| error.to_string())?;
    run_in_workspace(args, workspace_root)
}

/// Runs `xtask` using explicit arguments and workspace root.
///
/// # Errors
/// Returns an error when argument parsing fails or the selected task fails.
pub fn run_in_workspace(
    args: impl IntoIterator<Item = OsString>,
    workspace_root: impl AsRef<Path>,
) -> Result<(), String> {
    let xtask = match Xtask::try_parse_from(args) {
        Ok(xtask) => xtask,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            print!("{error}");
            return Ok(());
        }
        Err(error) => return Err(error.to_string()),
    };

    match xtask.command {
        XtaskCommand::AppApi(args) => run_app_api(workspace_root, &args),
        XtaskCommand::LineBudget(args) => run_line_budget(workspace_root, &args),
        XtaskCommand::Lexicon(args) => run_lexicon(workspace_root, &args),
        XtaskCommand::Release(args) => match args.command {
            ReleaseCommand::Prepare { version } => {
                prepare_app_release(workspace_root.as_ref(), &version)
            }
            ReleaseCommand::Tag => tag_app_release(workspace_root.as_ref()),
        },
        XtaskCommand::Resource(args) => match args.command {
            ResourceCommand::Catalog => validate_resource_catalog(workspace_root.as_ref()),
            ResourceCommand::CheckTag { tag } => {
                validate_resource_tag(workspace_root.as_ref(), &tag).map(|_| ())
            }
            ResourceCommand::Validate { id } => {
                validate_resource(workspace_root.as_ref(), &id).map(|_| ())
            }
            ResourceCommand::Tag { id } => tag_resource_release(workspace_root.as_ref(), &id),
        },
    }
}

fn run_app_api(workspace_root: impl AsRef<Path>, args: &AppApiArgs) -> Result<(), String> {
    match args.command {
        AppApiCommand::Types => {
            let workspace_root = workspace_root.as_ref();
            let config = AppApiTypeExportConfig::default_for_workspace(workspace_root);
            export_app_api_types(&config)?;
            format_app_api_types(workspace_root, &config.out_dir)?;
            println!(
                "App API TypeScript bindings generated at {}.",
                config.out_dir.display()
            );
            Ok(())
        }
    }
}

fn format_app_api_types(workspace_root: &Path, out_dir: &Path) -> Result<(), String> {
    let desktop_root = workspace_root.join("apps").join("desktop");
    if !desktop_root.join("package.json").is_file() {
        return Ok(());
    }

    let formatted_path = out_dir
        .strip_prefix(&desktop_root)
        .unwrap_or(out_dir)
        .to_path_buf();

    let status = Command::new(pnpm_command())
        .arg("--dir")
        .arg(&desktop_root)
        .arg("exec")
        .arg("oxfmt")
        .arg(&formatted_path)
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("oxfmt failed for {}", out_dir.display()))
    }
}

const fn pnpm_command() -> &'static str {
    if cfg!(windows) { "pnpm.cmd" } else { "pnpm" }
}

fn run_line_budget(workspace_root: impl AsRef<Path>, args: &LineBudgetArgs) -> Result<(), String> {
    if args.warn_lines > args.deny_lines {
        return Err("warn-lines must be less than or equal to deny-lines".to_owned());
    }

    let config = LineBudgetConfig {
        scan_roots: vec!["crates".into(), "apps".into(), "xtask".into()],
        warn_lines: args.warn_lines,
        deny_lines: args.deny_lines,
    };
    let report = check_line_budget(workspace_root, &config).map_err(|error| error.to_string())?;

    if report.findings.is_empty() {
        println!(
            "All Rust source files are at or below {} lines.",
            args.warn_lines
        );
        return Ok(());
    }

    let mut has_violation = false;
    for finding in &report.findings {
        match finding.level {
            LineBudgetLevel::Warning => {
                eprintln!(
                    "warning: {} has {} lines, above warning threshold {}",
                    finding.path.display(),
                    finding.line_count,
                    finding.threshold
                );
            }
            LineBudgetLevel::Violation => {
                has_violation = true;
                eprintln!(
                    "error: {} has {} lines, above deny threshold {}",
                    finding.path.display(),
                    finding.line_count,
                    finding.threshold
                );
            }
        }
    }

    if has_violation {
        Err("line budget check failed".to_owned())
    } else {
        Ok(())
    }
}

fn run_lexicon(workspace_root: impl AsRef<Path>, args: &LexiconArgs) -> Result<(), String> {
    let root = workspace_root.as_ref();
    match &args.command {
        LexiconCommand::Bundle(args) => {
            let config = LexiconBundleConfig {
                input_dir: root.join(&args.input),
                output_dir: root.join(&args.output),
                bundle_version: args.bundle_version.clone(),
            };
            let summary = build_lexicon_bundle(&config)?;
            println!(
                "Lexicon bundle built: {} entities={} relations={} semantic={}",
                summary.output_dir.display(),
                summary.entity_count,
                summary.relation_count,
                summary.semantic_available
            );
            Ok(())
        }
        LexiconCommand::Validate(args) => {
            let bundle = root.join(&args.bundle);
            let summary = validate_lexicon_bundle(&bundle)?;
            println!(
                "Lexicon bundle valid: {} entities={} semantic={}",
                bundle.display(),
                summary.entity_count,
                summary.semantic_available
            );
            Ok(())
        }
        LexiconCommand::Benchmark(args) => {
            let summary = benchmark_lexicon(&LexiconBenchmarkConfig {
                queries: root.join(&args.queries),
                candidate_run: root.join(&args.candidate_run),
                baseline_run: root.join(&args.baseline_run),
                bundle: root.join(&args.bundle),
                runtime_library: root.join(&args.runtime_library),
            })?;
            println!(
                "Lexicon benchmark passed: queries={} candidate_ndcg@10={:.4} baseline_ndcg@10={:.4} relative={:.2}% bundle={}MiB completion_p95={:.1}ms lexical_p95={:.1}ms semantic_first={:.1}ms semantic_p95={:.1}ms",
                summary.query_count,
                summary.candidate_ndcg_10,
                summary.baseline_ndcg_10,
                summary.relative_quality * 100.0,
                summary.bundle_bytes / (1024 * 1024),
                summary.completion_p95.as_secs_f64() * 1_000.0,
                summary.lexical_p95.as_secs_f64() * 1_000.0,
                summary.semantic_first.as_secs_f64() * 1_000.0,
                summary.semantic_p95.as_secs_f64() * 1_000.0,
            );
            Ok(())
        }
    }
}
