use anyhow::{Context, Result};
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
