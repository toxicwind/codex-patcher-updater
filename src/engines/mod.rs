use std::path::Path;

use anyhow::Result;

use crate::config::Config;
use crate::registry::{EngineKind, PatchSet};

pub struct EngineResult {
    pub matches: Option<u32>,
    pub status: String,
}

pub fn apply_patchset(
    patch: &PatchSet,
    cfg: &Config,
    vendor_dir: &Path,
    dry_run: bool,
) -> Result<EngineResult> {
    match patch.engine {
        EngineKind::AstGrep => {
            if !cfg.enable_ast_grep {
                return Ok(EngineResult {
                    matches: None,
                    status: "skipped: ast-grep engine disabled".to_string(),
                });
            }
            ast_grep::apply(patch, cfg, vendor_dir, dry_run)
        }
        EngineKind::Coccinelle => {
            if !cfg.enable_coccinelle {
                return Ok(EngineResult {
                    matches: None,
                    status: "skipped: coccinelle engine disabled".to_string(),
                });
            }
            coccinelle::apply(patch, cfg, vendor_dir, dry_run)
        }
        EngineKind::GritQl => {
            // Reserved for future engine; we just report a skip for now.
            Ok(EngineResult {
                matches: None,
                status: "skipped: gritql engine not implemented".to_string(),
            })
        }
    }
}

pub mod ast_grep;
pub mod coccinelle;
