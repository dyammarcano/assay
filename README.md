# wrap-swap
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

UWP/Electron → Tauri v2 parity toolkit. Ships a cited capability/gap matrix and
scaffolds Rust bridge code to close the achievable gaps.

## Layout

- `crates/core` — `core`: matrix model, dataset loader, parsers, analyzer, scaffolder.
- `crates/cli` — `cli`: the `wrap-swap` binary (`report | analyze | scaffold | sidecar`).
- `crates/winrt-shim` — `winrt-shim`: reusable WinRT capability shim (toast content + XML builder).
- `data/matrix.toml` — the cited capability/gap dataset (source of truth).

> Crate names are short and plain, with no project prefix. All crates set `publish = false` —
> nothing here is published to crates.io. (`cli` imports the `core` crate as `core::`.)

## Commands

```sh
# Render the full cited gap matrix as Markdown (with a per-path summary rollup)
cargo run -p cli -- report [--out FILE] [--matrix data/matrix.toml] [--source uwp|electron]

# Classify a target app's capabilities into gaps + a known-divergence report
cargo run -p cli -- analyze --profile profile.toml
cargo run -p cli -- analyze --appx AppxManifest.xml
cargo run -p cli -- analyze --electron-pkg package.json --electron-main main.js

# Emit Rust/Tauri bridge scaffolding (bridge.rs + deps.txt) for a profile's gaps
cargo run -p cli -- scaffold --profile profile.toml --out-dir wrap-swap-out

# Detect Electron native modules and scaffold a stdio-JSON sidecar migration kit
cargo run -p cli -- sidecar --electron-pkg package.json --out-dir sidecar-out
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
cargo test --workspace                              # 18 tests (unit + integration)
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```
