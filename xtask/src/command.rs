use std::ffi::OsString;
use std::path::Path;

use clap::{Args, Parser, Subcommand, error::ErrorKind};

use crate::{
    LineBudgetConfig, LineBudgetLevel, PromptLexiconBuildConfig, build_prompt_lexicon,
    check_line_budget, check_prompt_lexicon,
};

const DEFAULT_WARN_LINES: usize = 600;
const DEFAULT_DENY_LINES: usize = 1200;

#[derive(Debug, Parser)]
#[command(about = "NAI Atelier workspace automation")]
struct Xtask {
    #[command(subcommand)]
    command: XtaskCommand,
}

#[derive(Debug, Subcommand)]
enum XtaskCommand {
    #[command(about = "Check Rust source files against warning and deny line budgets")]
    LineBudget(LineBudgetArgs),
    #[command(about = "Build or check prompt lexicon assets")]
    Lexicon(LexiconArgs),
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
    #[command(about = "Build the generated prompt lexicon asset")]
    Build,
    #[command(about = "Check the generated prompt lexicon asset is up to date")]
    Check,
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
        XtaskCommand::LineBudget(args) => run_line_budget(workspace_root, &args),
        XtaskCommand::Lexicon(args) => run_lexicon(workspace_root, &args),
    }
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
    let config = PromptLexiconBuildConfig::default_for_workspace(workspace_root);
    match args.command {
        LexiconCommand::Build => {
            let summary = build_prompt_lexicon(&config)?;
            println!(
                "Prompt lexicon built: {} total={} categorized={} other={} weighted={} translations={} aliased={}",
                summary.output_file.display(),
                summary.total_tags,
                summary.categorized_tags,
                summary.uncategorized_tags,
                summary.matched_weights,
                summary.total_translations,
                summary.tags_with_aliases
            );
            Ok(())
        }
        LexiconCommand::Check => {
            check_prompt_lexicon(&config)?;
            println!("Generated prompt lexicon is up to date.");
            Ok(())
        }
    }
}
