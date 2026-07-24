# wrap-swap Known Issues & Limitations
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

Known bugs, limitations, and honest caveats. Feature gaps live in `docs/BACKLOG.md`;
this file is for defects and hard boundaries of the current build.

## Limitations (by design / platform)

- **No-viable-path capabilities are not portable.** Live Tiles (`uwp.live_tiles`) and the
  Share Target contract (`uwp.share_target`) are Windows-shell/MSIX concepts with no Tauri
  analog. `analyze` reports them as divergences; `scaffold` never emits code for them.
- **App Services need MSIX packaging.** `uwp.app_services` is only reachable from a Tauri app
  shipped as an MSIX-packaged build (via `windows-rs`); a plain-exe Tauri app cannot declare or
  be addressed as an App Service (no `PackageFamilyName`). Cited in `data/matrix.toml`.
- **PowerMonitor has no plugin.** `electron.power_monitor` has no official/community Tauri v2
  plugin (plugins-workspace #990 open); the only path is a custom `windows-rs` command.
- **Cross-engine capture is single-engine on Windows.** `webview-qa capture` drives headless
  Edge (Chromium/WebView2 family) for real. WKWebView and WebKitGTK drivers require macOS and
  Linux respectively, so a genuine cross-engine diff needs a mac/linux runner. A one-engine run
  is always labelled "Engines exercised (1)" and never reads as cross-engine confidence.
- **A Chromium capture is representative of WebView2, not identical to it.** It is Edge, not an
  embedded WebView2 host (different flags/chrome), hence the `chromium-edge` engine label.
- **Visual-tier parity is declared but unmeasured.** ADR 0001 marks six capabilities
  `visual`; `analyze` reports each as "visual parity NOT measured" until the engines they must
  match are actually captured.

## Parser limitations

- **Electron detection is heuristic** — `parse_electron` greps the single supplied main-source
  string for API identifiers and scans `dependencies` for known native-module package names. It
  will miss APIs used in files other than the one passed, and dynamic/aliased requires.
- **AppxManifest parsing requires well-formed, namespace-declared XML** — an undeclared prefix
  makes `roxmltree` reject the document (the parser returns an empty UWP profile in that case).

## Scaffolder limitations

- **A plugin is only wired automatically when it is a true drop-in.** Plugins needing developer
  configuration (e.g. `tauri-plugin-stronghold`, which requires a password-hash function) are
  emitted as a dependency plus a commented example, never as code — otherwise the generated
  bridge would not compile. Enforced by the opt-in `scaffold_compiles` test.
- **Crate versions come from the matrix** (`crate_version`); a row without one falls back to
  `"*"`, which is a starting point rather than a pinned dependency.
- Generated `bridge.rs` is a wiring skeleton; `custom_rust`/`sidecar` gaps without a proven
  recipe are emitted as commented stubs, not working code (by design — honesty invariant).

## Open (tracked in BACKLOG)

- Full temp-crate `cargo check` of generated scaffolding (currently syntactic validation only).
- No RE validation against a concrete pilot binary yet.
