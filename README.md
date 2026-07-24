# Assay
<!-- rev:002 (RFC 3339) 2026-07-24T01:19:21Z -->

**Porting a Windows Electron or UWP app to Tauri v2? Find out what breaks before you start.**

`assay` reads your app, tells you which of its OS-integration capabilities Tauri can and
cannot do — **every answer cited to a public doc, never guessed** — and generates the Rust
bridge code for the ones it can.

## Install

Requires [Rust](https://rustup.rs) (1.74+). This crate is **not published to crates.io**, so
install it from a clone:

```sh
git clone <this-repo> assay
cd assay
cargo install --path crates/cli        # installs the `assay` binary
cargo install --path crates/webview-qa # optional: the WebView divergence harness
assay --version
```

Prefer not to install? Every command below also works in-repo as
`cargo run -p cli -- <args>`.

## Quickstart — point it at *your* app

### Electron

Give it your `package.json` and your **main-process directory** (not just `main.js` — see
[Known limits](#known-limits)):

```sh
assay analyze --electron-pkg ./package.json --electron-main ./src/main
```

You get two things: a **gap list** (what Tauri can do and how) and a **known-divergence
report** (what it can't, and why). Roughly:

```
# Gap List

- Tray icon + menu (electron.tray): Native
- Global shortcuts (electron.global_shortcut): Plugin
- Native Node modules (electron.native_module): Sidecar

# Known-Divergence Report

- **Tray icon + menu — visual parity NOT measured** (electron.tray): behavior is covered,
  but look/feel equivalence is unverified …
- **WebView engine divergence** (webview.engine): Tauri uses the OS-native WebView …
```

Then generate the bridge code for the achievable gaps:

```sh
assay scaffold --electron-pkg ./package.json --electron-main ./src/main --out-dir bridge
```

`bridge/bridge.rs` wires the plugins that are proven drop-ins; `bridge/deps.txt` lists the exact
crates to add. Native Node modules get their own migration kit:

```sh
assay sidecar --electron-pkg ./package.json --out-dir sidecar
```

### UWP

```sh
assay analyze --appx ./AppxManifest.xml --emit-profile my-profile.toml
```

`--emit-profile` saves what was detected so you can **edit it** — add capabilities the manifest
doesn't declare, drop ones you don't use — then feed it back:

```sh
assay analyze  --profile my-profile.toml
assay scaffold --profile my-profile.toml --out-dir bridge
```

### Just browsing?

`assay report` prints the whole cited matrix with no input at all
(`--source uwp|electron` to narrow it).

## Known limits

Read these before trusting the output:

- **Windows only.** macOS/Linux support is deliberately deferred, not in progress.
- **Electron detection is textual.** It greps your main-process sources for Electron API
  identifiers. It will miss dynamically-built or aliased requires, and a **bundled/webpack'd**
  main process may defeat it entirely. Always sanity-check the gap list against what you know
  your app does.
- **Pass a directory, not one file.** A single-file `--electron-main` scans only that file and
  prints a warning saying so; real main processes span several modules.
- **Capability coverage is 24 rows** — the common surface, not everything WinRT/Electron expose.
  Anything not in the matrix is reported as *unknown and skipped*, never silently assumed fine.
- **"Visual parity" is declared, not measured.** Six UI-surfacing capabilities are marked
  visual-tier (ADR 0001) and always report *not measured* — see `docs/adr/0001-mimicry-bar.md`.
- **CI has not yet produced a green run.** `.github/workflows/ci.yml` is wired and triggers, but
  **no runner is ever allocated**: the job is created and fails in ~2 seconds with
  `runner_name: ""`, zero steps executed and no logs. Ruled out: the workflow file (valid YAML,
  byte-identical local↔remote), repo Actions settings (`enabled`, `allowed_actions: all`), and
  repo visibility (the failure is identical public and private). The remaining cause is
  account-level GitHub Actions availability — check
  [billing/spending limits](https://github.com/settings/billing). The local suite is green
  (`cargo test --workspace`, `cargo clippy -- -D warnings`) — which is not the same thing.

Full list: [`docs/ISSUES.md`](docs/ISSUES.md).

## Honesty rules

Every matrix row cites a public doc (enforced by a test). Rows with **no viable Tauri path** or
unresolved research are reported as divergences / **OPEN QUESTION** — never scaffolded as
fabricated code. The scaffolder emits working wiring only where a recipe is *compile-verified*
against the real tauri crates; everything else is a stub or a commented example with its
citation. A plugin that needs configuration (e.g. `tauri-plugin-stronghold`) is never wired
blindly.

## Cross-WebView harness (optional)

```sh
webview-qa init-config --out webview-qa.toml
webview-qa capture --url http://localhost:1420/ --config webview-qa.toml --out edge.json
webview-qa diff --blob edge.json --blob other-engine.json
```

`capture` drives **headless Edge over the DevTools protocol** — Edge's Chromium is the engine
WebView2 embeds, so the capture is representative of WebView2 (labelled `chromium-edge`, not
`webview2`, because it is Edge and not an embedded WebView2 host). On Windows a run is
single-engine and the report says so explicitly.

## Try it without your own app

The repo ships fixtures under `examples/`:

```sh
cargo run -p cli -- report --source uwp
cargo run -p cli -- analyze --appx examples/uwp-app/AppxManifest.xml
cargo run -p cli -- analyze --electron-pkg examples/electron-app/package.json \
    --electron-main examples/electron-app/main.js
cargo run -p cli -- scaffold --profile examples/profile-uwp.toml --out-dir assay-out
cargo run -p cli -- sidecar --electron-pkg examples/electron-app/package.json --out-dir sidecar-out
```

The scaffold step is the honesty invariant in action: `uwp.protocol_activation` gets real
wiring, while `uwp.live_tiles` / `uwp.share_target` are reported as divergences and produce
**no code at all**.

A capability profile is plain TOML:

```toml
source = "electron"           # or "uwp"
capabilities = ["electron.tray", "electron.global_shortcut"]
```

## Layout

- `crates/core` — package `core`: matrix model, dataset loader, parsers, analyzer, scaffolder.
- `crates/cli` — the `assay` binary (`report | analyze | scaffold | sidecar`).
- `crates/winrt-shim` — reusable WinRT capability shim (toast content + XML builder).
- `crates/webview-qa` — the `webview-qa` binary: probe, live capture, divergence differ.
- `data/matrix.toml` — the cited capability/gap dataset (source of truth).

> Crate names are short and plain, with no project prefix, and every crate sets
> `publish = false`. `cli` imports the `core` package under the alias `corelib`, because a
> dependency literally named `core` shadows Rust's built-in `core` crate.

## Development

```sh
cargo test --workspace                                  # 52 tests
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace

# Opt-in (network, ~4 min): compile generated scaffolding against the REAL tauri crates
cargo test -p core --test scaffold_compiles -- --ignored --test-threads=1
```

## License

BSD 3-Clause — see [LICENSE](LICENSE).
