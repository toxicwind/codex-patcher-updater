# Codex Forksmith (Retired)

> **Important:** Active development now lives in the standalone
> [codex-forksmith](https://github.com/toxicwind/codex-forksmith) repo. This
> tree is frozen, archived at `~/backups/old-codex-patcher-updater-20251119-145847.zip`,
> and kept only for historical reference.

A Rust-native fork steward for `vendor/codex`. The new `codex-forksmith`
binary manages git status, syncing, builds, and execution of the vendored
Codex workspace so you can treat this repo as a turnkey fork manager.

## CLI

```
cargo run --bin codex-forksmith -- status   # branch, divergence, binary path
cargo run --bin codex-forksmith -- sync     # fetch + fast-forward to upstream
cargo run --bin codex-forksmith -- build    # cargo build --profile release
cargo run --bin codex-forksmith -- run -- <args passed to codex>
```

`status` inspects `vendor/codex` and prints:

- current branch, HEAD, and cleanliness
- ahead/behind counts vs `origin/<branch>` and `upstream/<branch>`
- whether the configured Codex binary exists

`sync` fetches the configured `local_remote` and `upstream_remote`, requires a
clean tree, then fast-forwards the working tree to the upstream ref. Any output
clearly calls out when the local remote still needs a push.

`build` runs `cargo build --profile <profile>` inside `vendor/codex` and
verifies the binary defined by `binary_relpath` exists before returning.

`run -- …` executes that binary with passthrough stdin/stdout/stderr, so you can
chain Codex invocations from scripts or AGENTS. It bails with a clear message if
the binary has not been built yet.

The legacy registry/patch pipeline still lives behind
`cargo run --bin codex-forksmith-legacy -- <command>` for historical reference,
but the default toolchain is the new fork-aware CLI described above.

- `update` resets `vendor/codex`, loads the patch registry, runs ast-grep/cocci
  rules, updates registry metadata, optionally runs `cargo build --release`, and
  prints a machine-readable JSON summary with `--json`.
- `doctor` reports workspace health (vendor presence, registry path, rule counts).
- `registry` commands are the single source of truth for toggling semantic patch
  sets so you never edit JSON manually.

The repository already vendors upstream under `vendor/codex` via a git submodule
pointing at `github.com/openai/codex`, so you always see the exact code the
pipeline mutates.

## Workspace layout

This repo now owns the entire Rust toolchain that used to live in `~/crates`.
Running `cargo metadata` shows the extra crates under `crates/`:

| Crate | Purpose |
| --- | --- |
| `codex-ast-driver` / `codex-cocci-driver` | Hermetic adapters for ast-grep and coccinelle-for-rust. |
| `codex-core` | Future orchestration layer that stitches drivers + registry + packaging. |
| `codex-registry` | JSON registry helpers (the CLI still uses the bespoke format but this gives us a migration path). |
| `codex-pkg` | Zip/packaging helper used by the experimental `codex-core`. |
| `codex-updater-cli` | Reference CLI wiring on top of `codex-core` (kept for experimentation). |
| `codex-wrapper` | Pure-Rust wrapper that can replace the Bash launcher under `~/.config/bash/hypebrut/bin/hb/codex`. |

You can build everything in one pass with `cargo build --workspace`. The
existing `codex-forksmith` binary remains the source of truth today; the
additional crates are vendored here so we can progressively migrate features
into them without juggling multiple repositories.

## Configuration

`codex-forksmith.toml` configures the workspace:

```toml
[workspace]
root = "."

[repo]
path = "vendor/codex"
local_remote = "origin"
local_branch = "main"
upstream_remote = "upstream"
upstream_branch = "main"

[build]
profile = "release"
workspace = "codex-rs"
binary_relpath = "codex-rs/target/release/codex"
```

All fields are optional; sensible defaults match the layout in this repo. If
you track a different fork branch or build profile, tweak those values and the
CLI will respect them.
