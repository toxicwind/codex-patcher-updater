use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use walkdir::WalkDir;

use crate::config::Config;
use crate::engines;
use crate::process::{cargo_build_release, git_head_commit, git_reset_to_branch, require_tool};
use crate::registry::{EngineKind, PatchRegistry, PatchSet};

#[derive(Debug, Clone)]
pub struct UpdateOptions {
    pub dry_run: bool,
    pub skip_build: bool,
    pub ast_allowed: bool,
    pub cocci_allowed: bool,
    pub emit_json: bool,
}

impl UpdateOptions {
    pub fn new(
        dry_run: bool,
        skip_build: bool,
        ast_allowed: bool,
        cocci_allowed: bool,
        emit_json: bool,
    ) -> Self {
        Self {
            dry_run,
            skip_build,
            ast_allowed,
            cocci_allowed,
            emit_json,
        }
    }
}

#[derive(Debug, Serialize)]
struct PatchReport {
    id: String,
    engine: String,
    status: String,
    matches: Option<u32>,
}

#[derive(Debug, Default, Serialize)]
pub struct UpdateSummary {
    dry_run: bool,
    vendor_head_before: Option<String>,
    vendor_head_after: Option<String>,
    patch_reports: Vec<PatchReport>,
    warnings: Vec<String>,
    build_status: Option<String>,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    workspace: String,
    vendor_dir: String,
    vendor_exists: bool,
    registry_path: String,
    registry_exists: bool,
    ast_rules: usize,
    coccinelle_rules: usize,
}

pub fn run_health(root: &Path) -> Result<()> {
    let cfg = Config::load(root)?;
    let vendor = cfg.vendor_dir(root);
    let registry_path = cfg.registry_path(root);
    let ast_rules = count_files(cfg.rules_root_path(root).join("ast-grep"), &["yml", "yaml"]);
    let coccinelle_rules = count_files(cfg.rules_root_path(root).join("coccinelle"), &["cocci"]);

    let report = DoctorReport {
        workspace: root.display().to_string(),
        vendor_dir: vendor.display().to_string(),
        vendor_exists: vendor.exists(),
        registry_path: registry_path.display().to_string(),
        registry_exists: registry_path.exists(),
        ast_rules,
        coccinelle_rules,
    };

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn run_list_patches(root: &Path) -> Result<()> {
    let cfg = Config::load(root)?;
    let registry = PatchRegistry::load_or_init(&cfg, root)?;
    for patch in registry.list() {
        println!(
            "- {:<32} engine={:?} enabled={} tags={}",
            patch.id,
            patch.engine,
            patch.enabled,
            if patch.tags.is_empty() {
                "-".into()
            } else {
                patch.tags.join(", ")
            }
        );
    }
    Ok(())
}

pub fn run_explain_patch(root: &Path, id: &str) -> Result<()> {
    let cfg = Config::load(root)?;
    let registry = PatchRegistry::load_or_init(&cfg, root)?;
    if let Some(p) = registry.get(id) {
        println!("Patch-set: {}", p.id);
        println!("  description : {}", p.description);
        println!("  engine      : {:?}", p.engine);
        println!("  enabled     : {}", p.enabled);
        println!("  rules       :");
        for r in &p.rules {
            println!("    - {}", r);
        }
        if !p.tags.is_empty() {
            println!("  tags        : {}", p.tags.join(", "));
        }
        if let Some(conf) = p.engine_confidence {
            println!("  confidence  : {:.2}", conf);
        }
        if let Some(status) = &p.last_status {
            println!("  last_status : {}", status);
        }
        if let Some(commit) = &p.last_applied_commit {
            println!("  last_commit : {}", commit);
        }
        if let Some(ts) = &p.last_run_ts {
            println!("  last_run_ts : {}", ts);
        }
    } else {
        anyhow::bail!("No patch-set with id {id}");
    }
    Ok(())
}

pub fn run_toggle_patch(root: &Path, id: &str, enabled: bool) -> Result<()> {
    let cfg = Config::load(root)?;
    let mut registry = PatchRegistry::load_or_init(&cfg, root)?;
    let patch = registry
        .get_mut(id)
        .ok_or_else(|| anyhow!("No patch-set with id {id}"))?;
    patch.enabled = enabled;
    registry.save(&cfg, root)?;
    println!("{} {}", if enabled { "Enabled" } else { "Disabled" }, id);
    Ok(())
}

pub fn run_update(root: &Path, opts: UpdateOptions) -> Result<()> {
    let mut cfg = Config::load(root)?;
    let vendor_dir = cfg.vendor_dir(root);
    if !vendor_dir.exists() {
        return Err(anyhow!(
            "Vendor directory {} does not exist",
            vendor_dir.display()
        ));
    }

    let mut summary = UpdateSummary {
        dry_run: opts.dry_run,
        vendor_head_before: git_head_commit(&vendor_dir).ok(),
        ..Default::default()
    };

    cfg.enable_ast_grep = cfg.enable_ast_grep && opts.ast_allowed;
    cfg.enable_coccinelle = cfg.enable_coccinelle && opts.cocci_allowed;

    if cfg.enable_ast_grep {
        require_tool(&cfg.ast_grep_bin)
            .with_context(|| format!("ast-grep tool {} not available", cfg.ast_grep_bin))?;
    } else {
        summary
            .warnings
            .push("ast-grep engine disabled (config or flag)".into());
    }

    if cfg.enable_coccinelle {
        if let Some(bin) = &cfg.coccinelle_bin {
            require_tool(bin).with_context(|| format!("coccinelle tool {} not available", bin))?;
        } else {
            summary
                .warnings
                .push("coccinelle enabled but no binary configured".into());
            cfg.enable_coccinelle = false;
        }
    }

    println!("codex-patcher-updater update");
    println!("  workspace root: {}", root.display());
    println!("  vendor dir    : {}", vendor_dir.display());
    println!("  dry-run       : {}", opts.dry_run);

    println!("Step 1/4: Reset vendor to origin/{}...", cfg.vendor_branch);
    git_reset_to_branch(&vendor_dir, &cfg.vendor_branch)?;
    let commit = git_head_commit(&vendor_dir)?;
    summary.vendor_head_after = Some(commit.clone());

    println!("Step 2/4: Loading registry and registering rules...");
    let mut registry = PatchRegistry::load_or_init(&cfg, root)?;
    registry.ensure_rules_registered(&cfg, root)?;
    println!("  {} patch-sets registered", registry.patch_sets.len());

    println!("Step 3/4: Applying patch-sets...");
    for patch in registry.patch_sets.clone() {
        if !patch.enabled {
            record_patch(&mut summary, &patch, None, "skipped (disabled)");
            continue;
        }
        if !engine_allowed(&cfg, &patch.engine) {
            record_patch(&mut summary, &patch, None, "skipped (engine disabled)");
            continue;
        }
        let result = engines::apply_patchset(&patch, &cfg, &vendor_dir, opts.dry_run)?;
        record_patch(&mut summary, &patch, result.matches, result.status.clone());
        registry.update_after_run(&patch.id, &commit, result.matches, &result.status);
    }

    registry.save(&cfg, root)?;

    println!("Step 4/4: Build phase...");
    if opts.dry_run {
        summary.build_status = Some("skipped (dry-run)".into());
        println!("  build skipped (dry-run)");
    } else if opts.skip_build {
        summary.build_status = Some("skipped (--skip-build)".into());
        println!("  build skipped (--skip-build)");
    } else {
        cargo_build_release(&vendor_dir)?;
        summary.build_status = Some("succeeded".into());
        println!("  build succeeded");
    }

    if opts.emit_json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_summary(&summary);
    }

    Ok(())
}

fn engine_allowed(cfg: &Config, engine: &EngineKind) -> bool {
    match engine {
        EngineKind::AstGrep => cfg.enable_ast_grep,
        EngineKind::Coccinelle => cfg.enable_coccinelle,
        EngineKind::GritQl => cfg.enable_gritql,
    }
}

fn record_patch(
    summary: &mut UpdateSummary,
    patch: &PatchSet,
    matches: Option<u32>,
    status: impl Into<String>,
) {
    summary.patch_reports.push(PatchReport {
        id: patch.id.clone(),
        engine: format!("{:?}", patch.engine),
        status: status.into(),
        matches,
    });
}

fn print_summary(summary: &UpdateSummary) {
    println!("\nSummary:");
    println!("  vendor before : {:?}", summary.vendor_head_before);
    println!("  vendor after  : {:?}", summary.vendor_head_after);
    println!("  dry-run       : {}", summary.dry_run);
    if !summary.patch_reports.is_empty() {
        println!("  patches:");
        for report in &summary.patch_reports {
            println!(
                "    - {:<32} {:<12} matches={:?} status={}",
                report.id, report.engine, report.matches, report.status
            );
        }
    }
    if !summary.warnings.is_empty() {
        println!("  warnings:");
        for w in &summary.warnings {
            println!("    - {w}");
        }
    }
    println!("  build        : {:?}", summary.build_status);
}

fn count_files(dir: PathBuf, exts: &[&str]) -> usize {
    if !dir.exists() {
        return 0;
    }
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| exts.iter().any(|wanted| wanted.eq_ignore_ascii_case(ext)))
                .unwrap_or(false)
        })
        .count()
}
