use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::process::run_command;
use crate::registry::PatchSet;

use super::EngineResult;

pub fn apply(patch: &PatchSet, vendor_dir: &Path, dry_run: bool) -> Result<EngineResult> {
    let workspace = vendor_dir.parent().unwrap_or(vendor_dir);
    let mut applied = 0u32;

    for file in &patch.rules {
        let path = PathBuf::from(file);
        let abs = if path.is_absolute() {
            path
        } else {
            workspace.join(path)
        };
        if !abs.exists() {
            bail!("patch file {} missing", abs.display());
        }
        let patch_str = abs.to_string_lossy().to_string();
        let args: Vec<&str> = if dry_run {
            vec!["apply", "--check", &patch_str]
        } else {
            vec!["apply", "--3way", &patch_str]
        };
        let output = run_command("git", &args, Some(vendor_dir))
            .with_context(|| format!("git apply failed for {}", patch_str))?;
        if !output.status.success() {
            bail!(
                "git apply failed for {}: {}",
                patch_str,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        applied = applied.saturating_add(1);
    }

    Ok(EngineResult {
        matches: Some(applied),
        status: if dry_run {
            "dry-run".to_string()
        } else {
            "applied".to_string()
        },
    })
}
