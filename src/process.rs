use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::{Command, Output};

pub fn run_command(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.output()
        .with_context(|| format!("Failed to spawn {program} with args {args:?}"))
}

pub fn require_tool(name: &str) -> Result<()> {
    let out = run_command(name, &["--version"], None)?;
    if !out.status.success() {
        anyhow::bail!("{name} is not available or returned a non-zero status");
    }
    Ok(())
}

pub fn git_reset_to_branch(repo: &Path, branch: &str) -> Result<()> {
    run_command("git", &["fetch", "origin"], Some(repo))
        .with_context(|| "git fetch origin failed")?;
    let target = format!("origin/{branch}");
    let out = run_command("git", &["reset", "--hard", &target], Some(repo))?;
    if !out.status.success() {
        anyhow::bail!(
            "git reset --hard {target} failed with status {:?} and stderr:
{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub fn git_head_commit(repo: &Path) -> Result<String> {
    let out = run_command("git", &["rev-parse", "HEAD"], Some(repo))?;
    if !out.status.success() {
        anyhow::bail!(
            "git rev-parse HEAD failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(s)
}

pub fn git_current_branch(repo: &Path) -> Result<String> {
    let out = run_command("git", &["rev-parse", "--abbrev-ref", "HEAD"], Some(repo))?;
    if !out.status.success() {
        anyhow::bail!(
            "git rev-parse --abbrev-ref HEAD failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn git_is_clean(repo: &Path) -> Result<bool> {
    let out = run_command("git", &["status", "--porcelain"], Some(repo))?;
    if !out.status.success() {
        anyhow::bail!(
            "git status --porcelain failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(out.stdout.is_empty())
}

pub fn git_fetch_remote(repo: &Path, remote: &str) -> Result<()> {
    let out = run_command("git", &["fetch", remote], Some(repo))
        .with_context(|| format!("git fetch {remote} failed"))?;
    if !out.status.success() {
        anyhow::bail!(
            "git fetch {remote} returned {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub fn git_divergence(repo: &Path, left: &str, right: &str) -> Result<(u32, u32)> {
    let range = format!("{left}...{right}");
    let out = run_command(
        "git",
        &["rev-list", "--left-right", "--count", &range],
        Some(repo),
    )?;
    if !out.status.success() {
        anyhow::bail!(
            "git rev-list --left-right --count {range} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut parts = stdout.trim().split_whitespace();
    let left_only = parts
        .next()
        .ok_or_else(|| anyhow!("Failed to parse git rev-list output: {}", stdout))?;
    let right_only = parts
        .next()
        .ok_or_else(|| anyhow!("Failed to parse git rev-list output: {}", stdout))?;
    let ahead = left_only.parse::<u32>().with_context(|| {
        format!("Failed to parse ahead count ({left_only}) from git rev-list output: {stdout}")
    })?;
    let behind = right_only.parse::<u32>().with_context(|| {
        format!("Failed to parse behind count ({right_only}) from git rev-list output: {stdout}")
    })?;
    Ok((ahead, behind))
}

pub fn git_merge_ff_only(repo: &Path, target: &str) -> Result<()> {
    let out = run_command("git", &["merge", "--ff-only", target], Some(repo))?;
    if !out.status.success() {
        anyhow::bail!(
            "git merge --ff-only {target} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub fn cargo_build_release(repo: &Path) -> Result<()> {
    let out = run_command("cargo", &["build", "--release"], Some(repo))?;
    if !out.status.success() {
        anyhow::bail!(
            "cargo build --release failed:
{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}
