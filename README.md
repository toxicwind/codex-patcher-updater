# Codex Patcher Updater

A fully Rust-native patcher/updater for keeping `vendor/codex` aligned with upstream
while enforcing local semantic transforms via ast-grep (syntax aware) and
coccinelle-for-rust (type aware).

## CLI

```
cargo run -- update [--dry-run] [--skip-build] [--no-ast] [--no-cocci] [--json]
cargo run -- doctor
cargo run -- registry list
cargo run -- registry explain <id>
cargo run -- registry enable <id>
cargo run -- registry disable <id>
```

- `update` resets `vendor/codex`, loads the patch registry, runs ast-grep/cocci
  rules, updates registry metadata, optionally runs `cargo build --release`, and
  prints a machine-readable JSON summary with `--json`.
- `doctor` reports workspace health (vendor presence, registry path, rule counts).
- `registry` commands are the single source of truth for toggling semantic patch
  sets so you never edit JSON manually.

## Registry + Rules

- Rules live under `rules/ast-grep` and `rules/coccinelle`.
- `patch-registry/registry.json` captures patch intent, enable flags, engine,
  confidence ratios, and last-run metadata.
- On every run, `registry.ensure_rules_registered` auto-imports new rule files
  into the registry so adding a rule is just dropping YAML and rerunning.

## Engine Behavior

1. **ast-grep** – Syntax-aware rewrites with dry-run metrics before applying.
2. **coccinelle-for-rust** – Type-aware patches for the gnarliest codex tweaks;
   optional and auto-skipped when the binary is missing.
3. **Future** – Placeholder for GritQL or other semantic engines; CLI already has
   knobs to disable/enable engines per run.

The update pipeline always prefers graceful degradation: if a rule stops
matching it logs a warning, records the zero-match event in the registry, and
moves on so upstream bumps never brick the build.
