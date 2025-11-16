use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::engines::EngineResult;
use crate::process::run_command;
use crate::registry::PatchSet;

pub fn apply(
    patch: &PatchSet,
    cfg: &Config,
    vendor_dir: &Path,
    dry_run: bool,
) -> Result<EngineResult> {
    let Some(bin) = cfg.coccinelle_bin.as_ref() else {
        return Ok(EngineResult {
            matches: None,
            status: "skipped: coccinelle binary not configured".to_string(),
        });
    };

    let mut total_matches: u32 = 0;

    for rule in &patch.rules {
        let rule_path = PathBuf::from(rule);
        let rule_abs = if rule_path.is_absolute() {
            rule_path
        } else {
            let root = vendor_dir.parent().unwrap_or(vendor_dir);
            root.join(rule_path)
        };

        // NOTE: CLI here is illustrative; adjust to real coccinelle-for-rust usage.
        let mut args = Vec::new();
        args.push("--patch");
        args.push(rule_abs.to_str().unwrap());
        if dry_run {
            args.push("--dry-run");
        }
        args.push(vendor_dir.to_str().unwrap());

        let output = run_command(bin, &args, None).with_context(|| {
            format!(
                "coccinelle invocation failed for rule {}",
                rule_abs.display()
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "coccinelle returned non-zero status for {}: {stderr}",
                rule_abs.display()
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // For now we don't attempt to parse match counts; this can be refined later.
        if !stdout.is_empty() {
            total_matches = total_matches.saturating_add(1);
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
