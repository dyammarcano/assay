# wrap-swap
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

UWP/Electron → Tauri v2 parity toolkit. Ships a cited capability/gap matrix and
scaffolds Rust bridge code to close the achievable gaps.

## Layout

- `crates/core` — `wrapswap-core`: matrix model, dataset loader, parsers, analyzer, scaffolder.
- `crates/cli` — `cli`: the `wrap-swap` binary (`report | analyze | scaffold`).
- `data/matrix.toml` — the cited capability/gap dataset (source of truth).

> Crate names are short and plain; the core **package** is `wrapswap-core` only so its library
> doesn't shadow the standard-library `core` crate. Both crates set `publish = false` —
> nothing here is published to crates.io.

## Commands

```sh
# Render the full cited gap matrix as Markdown
cargo run -p cli -- report [--out FILE] [--matrix data/matrix.toml]

# Classify a target app's capabilities into gaps + a known-divergence report
cargo run -p cli -- analyze --profile profile.toml
cargo run -p cli -- analyze --appx AppxManifest.xml
cargo run -p cli -- analyze --electron-pkg package.json --electron-main main.js

# Emit Rust/Tauri bridge scaffolding (bridge.rs + deps.txt) for a profile's gaps
cargo run -p cli -- scaffold --profile profile.toml --out-dir wrap-swap-out
```

A capability profile is TOML:

```toml
source = "electron"           # or "uwp"
capabilities = ["electron.tray", "electron.global_shortcut"]
```

`analyze`/`scaffold` also accept `--appx` or `--electron-pkg`+`--electron-main` to derive the
profile automatically; add `--emit-profile profile.toml` to save the parsed profile for editing.

## Honesty rules

Every matrix row cites a public doc (enforced by a test). Rows with **no viable Tauri path** or
unresolved research are reported as divergences / **OPEN QUESTION**, never scaffolded as
fabricated code. The scaffolder emits working wiring only where a proven recipe (official plugin
or established crate) exists; everything else is a `todo!()` stub with an inline citation.

## Development

```sh
cargo test --workspace   # 9 unit + 3 integration tests
cargo build --workspace
```
