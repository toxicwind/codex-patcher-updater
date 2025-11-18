use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Default)]
pub struct VendorSection {
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ToolsSection {
    #[serde(default)]
    pub ast_grep: Option<String>,
    #[serde(default)]
    pub coccinelle: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PatchRegistrySection {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PatchesSection {
    #[serde(default)]
    pub rules_root: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ForkSection {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub upstream_remote: Option<String>,
    #[serde(default)]
    pub upstream_branch: Option<String>,
    #[serde(default)]
    pub local_remote: Option<String>,
    #[serde(default)]
    pub local_branch: Option<String>,
    #[serde(default)]
    pub require_clean_worktree: Option<bool>,
    #[serde(default)]
    pub abort_on_divergence: Option<bool>,
    #[serde(default)]
    pub auto_merge_upstream: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct EnginesSection {
    #[serde(default)]
    pub ast_grep: Option<bool>,
    #[serde(default)]
    pub coccinelle: Option<bool>,
    #[serde(default)]
    pub gritql: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawConfig {
    #[serde(default)]
    pub vendor: VendorSection,
    #[serde(default)]
    pub tools: ToolsSection,
    #[serde(default)]
    pub patch_registry: PatchRegistrySection,
    #[serde(default)]
    pub patches: PatchesSection,
    #[serde(default)]
    pub fork: ForkSection,
    #[serde(default)]
    pub engines: EnginesSection,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub vendor_root: String,
    pub vendor_branch: String,
    pub ast_grep_bin: String,
    pub coccinelle_bin: Option<String>,
    pub enable_ast_grep: bool,
    pub enable_coccinelle: bool,
    pub enable_gritql: bool,
    pub patch_registry_path: String,
    pub rules_root: String,
    pub fork: ForkConfig,
}

#[derive(Debug, Clone)]
pub struct ForkConfig {
    pub enabled: bool,
    pub upstream_remote: String,
    pub upstream_branch: String,
    pub local_remote: String,
    pub local_branch: String,
    pub require_clean_worktree: bool,
    pub abort_on_divergence: bool,
    pub auto_merge_upstream: bool,
}

impl Config {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("codex-patcher-updater.toml");
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let raw: RawConfig = toml::from_str(&contents)
            .with_context(|| "Failed to parse codex-patcher-updater.toml")?;

        let vendor_root = raw
            .vendor
            .root
            .unwrap_or_else(|| "vendor/codex".to_string());
        let vendor_branch = raw.vendor.branch.unwrap_or_else(|| "main".to_string());
        let ast_grep_bin = raw.tools.ast_grep.unwrap_or_else(|| "ast-grep".to_string());
        let coccinelle_bin = raw.tools.coccinelle;

        let patch_registry_path = raw
            .patch_registry
            .path
            .unwrap_or_else(|| "patch-registry/registry.json".to_string());

        let rules_root = raw
            .patches
            .rules_root
            .unwrap_or_else(|| "rules".to_string());

        let enable_ast_grep = raw.engines.ast_grep.unwrap_or(true);
        let enable_coccinelle = raw.engines.coccinelle.unwrap_or(false);
        let enable_gritql = raw.engines.gritql.unwrap_or(false);

        let fork = ForkConfig::from_section(&raw.fork, &vendor_branch);

        Ok(Config {
            vendor_root,
            vendor_branch,
            ast_grep_bin,
            coccinelle_bin,
            enable_ast_grep,
            enable_coccinelle,
            enable_gritql,
            patch_registry_path,
            rules_root,
            fork,
        })
    }

    pub fn vendor_dir(&self, root: &Path) -> PathBuf {
        root.join(&self.vendor_root)
    }

    pub fn registry_path(&self, root: &Path) -> PathBuf {
        root.join(&self.patch_registry_path)
    }

    pub fn rules_root_path(&self, root: &Path) -> PathBuf {
        root.join(&self.rules_root)
    }
}

impl ForkConfig {
    fn from_section(section: &ForkSection, vendor_branch: &str) -> Self {
        Self {
            enabled: section.enabled.unwrap_or(false),
            upstream_remote: section
                .upstream_remote
                .clone()
                .unwrap_or_else(|| "upstream".to_string()),
            upstream_branch: section
                .upstream_branch
                .clone()
                .unwrap_or_else(|| vendor_branch.to_string()),
            local_remote: section
                .local_remote
                .clone()
                .unwrap_or_else(|| "origin".to_string()),
            local_branch: section
                .local_branch
                .clone()
                .unwrap_or_else(|| vendor_branch.to_string()),
            require_clean_worktree: section.require_clean_worktree.unwrap_or(true),
            abort_on_divergence: section.abort_on_divergence.unwrap_or(true),
            auto_merge_upstream: section.auto_merge_upstream.unwrap_or(false),
        }
    }
}
