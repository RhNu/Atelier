use std::path::{Path, PathBuf};
use std::{fs, io};

use walkdir::WalkDir;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineBudgetConfig {
    pub scan_roots: Vec<PathBuf>,
    pub warn_lines: usize,
    pub deny_lines: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineBudgetLevel {
    Warning,
    Violation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineBudgetFinding {
    pub level: LineBudgetLevel,
    pub path: PathBuf,
    pub line_count: usize,
    pub threshold: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineBudgetReport {
    pub findings: Vec<LineBudgetFinding>,
}

/// Checks Rust source files against warning and deny line budgets.
///
/// # Errors
/// Returns an error when a scanned directory entry cannot be read or a source
/// file cannot be read.
pub fn check_line_budget(
    workspace_root: impl AsRef<Path>,
    config: &LineBudgetConfig,
) -> io::Result<LineBudgetReport> {
    let workspace_root = workspace_root.as_ref();
    let mut findings = Vec::new();

    for root in &config.scan_roots {
        let scan_root = workspace_root.join(root);
        if !scan_root.exists() {
            continue;
        }

        for entry in WalkDir::new(scan_root) {
            let entry = entry.map_err(io::Error::other)?;
            let path = entry.path();
            if !entry.file_type().is_file()
                || path.extension().is_none_or(|extension| extension != "rs")
            {
                continue;
            }

            let line_count = count_lines(path)?;
            let finding = classify_file(path, line_count, config);
            if let Some(finding) = finding {
                findings.push(finding);
            }
        }
    }

    findings.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(LineBudgetReport { findings })
}

fn classify_file(
    path: &Path,
    line_count: usize,
    config: &LineBudgetConfig,
) -> Option<LineBudgetFinding> {
    if line_count > config.deny_lines {
        return Some(LineBudgetFinding {
            level: LineBudgetLevel::Violation,
            path: path.to_path_buf(),
            line_count,
            threshold: config.deny_lines,
        });
    }

    if line_count > config.warn_lines {
        return Some(LineBudgetFinding {
            level: LineBudgetLevel::Warning,
            path: path.to_path_buf(),
            line_count,
            threshold: config.warn_lines,
        });
    }

    None
}

fn count_lines(path: &Path) -> io::Result<usize> {
    let bytes = fs::read(path)?;
    if bytes.is_empty() {
        return Ok(0);
    }

    let newline_count = bytecount::count(&bytes, b'\n');
    Ok(newline_count + usize::from(!bytes.ends_with(b"\n")))
}
