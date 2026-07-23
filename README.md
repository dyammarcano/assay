# wrap-swap
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

UWP/Electron → Tauri v2 parity toolkit. Ships a cited capability/gap matrix and
scaffolds Rust bridge code to close the achievable gaps.

## Layout

- `crates/core` — `core`: matrix model, dataset loader, parsers, analyzer, scaffolder.
- `crates/cli` — `cli`: the `wrap-swap` binary (`report | analyze | scaffold | sidecar`).
- `crates/winrt-shim` — `winrt-shim`: reusable WinRT capability shim (toast content + XML builder).
- `crates/webview-qa` — `webview-qa`: cross-WebView divergence differ + `diff` CLI over recorded blobs.
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

## Walkthrough (runnable, uses `examples/`)

```sh
# 1. See the whole cited gap matrix (or just one platform)
cargo run -p cli -- report --source uwp

# 2. Analyze the fixture UWP app straight from its manifest,
#    saving the detected profile so you can edit it
cargo run -p cli -- analyze --appx examples/uwp-app/AppxManifest.xml \
    --emit-profile my-profile.toml

# 3. Analyze the fixture Electron app from package.json + main process
cargo run -p cli -- analyze --electron-pkg examples/electron-app/package.json \
    --electron-main examples/electron-app/main.js

# 4. Scaffold bridge code from a hand-written profile
cargo run -p cli -- scaffold --profile examples/profile-uwp.toml --out-dir wrap-swap-out

# 5. Scaffold a sidecar kit for the fixture app's native modules
cargo run -p cli -- sidecar --electron-pkg examples/electron-app/package.json \
    --out-dir sidecar-out

# 6. Cross-WebView harness: starter config -> per-engine probe JS -> diff recorded blobs
cargo run -p webview-qa -- init-config --out webview-qa.toml
cargo run -p webview-qa -- probe --engine webview2 --config webview-qa.toml --out probe.js
cargo run -p webview-qa -- diff --blob webview2.json --blob webkitgtk.json
```

Step 4 on `examples/profile-uwp.toml` is the honesty invariant in action: `uwp.toast` and
`uwp.protocol_activation` get real wiring, while `uwp.live_tiles` / `uwp.share_target` are
reported as divergences and produce **no code at all**.

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
