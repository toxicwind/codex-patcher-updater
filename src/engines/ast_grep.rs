use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::engines::EngineResult;
use crate::process::run_command;
use crate::registry::PatchSet;

fn parse_match_count(stdout: &str) -> Option<u32> {
    // ast-grep typically prints lines like "Applied 3 changes"
    for line in stdout.lines().rev() {
        if let Some(rest) = line.trim().strip_prefix("Applied ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if let Some(num_str) = parts.first() {
                if let Ok(n) = num_str.parse::<u32>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

pub fn apply(
    patch: &PatchSet,
    cfg: &Config,
    vendor_dir: &Path,
    dry_run: bool,
) -> Result<EngineResult> {
    let bin = &cfg.ast_grep_bin;
    let mut total_matches: u32 = 0;

    for rule in &patch.rules {
        let rule_path = PathBuf::from(rule);
        let rule_abs = if rule_path.is_absolute() {
            rule_path
        } else {
            // Treat rule paths as relative to workspace root
            // (vendor_dir is usually vendor/codex, so we go one level up)
            let root = vendor_dir.parent().unwrap_or(vendor_dir);
            root.join(rule_path)
        };

        let mut args = vec!["scan", "--rule", rule_abs.to_str().unwrap()];
        if !dry_run {
            args.push("--update-all");
        }
        args.push(vendor_dir.to_str().unwrap());

        let output = run_command(bin, &args, None).with_context(|| {
            format!("ast-grep invocation failed for rule {}", rule_abs.display())
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "ast-grep returned non-zero status for {}: {stderr}",
                rule_abs.display()
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(n) = parse_match_count(&stdout) {
            total_matches += n;
        }
    }

    Ok(EngineResult {
        matches: Some(total_matches),
        status: if dry_run {
            "dry-run".to_string()
        } else {
            "applied".to_string()
        },
    })
}
