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
- **WebView divergence is unmodeled.** The known-divergence report warns that
  WebView2/WKWebView/WebKitGTK can render/behave differently, but wrap-swap does not measure or
  test it (see BACKLOG: Cross-WebView QA harness).

## Parser limitations

- **Electron detection is heuristic** — `parse_electron` greps the single supplied main-source
  string for API identifiers and scans `dependencies` for known native-module package names. It
  will miss APIs used in files other than the one passed, and dynamic/aliased requires.
- **AppxManifest parsing requires well-formed, namespace-declared XML** — an undeclared prefix
  makes `roxmltree` reject the document (the parser returns an empty UWP profile in that case).

## Scaffolder limitations

- **Proven-recipe crates emit a wildcard version** (`crate = "*"`) in `deps.txt` — a starting
  point, not a pinned dependency; the developer must choose a real version.
- Generated `bridge.rs` is a wiring skeleton; `custom_rust`/`sidecar` gaps without a proven
  recipe are emitted as commented stubs, not working code (by design — honesty invariant).

## Open (tracked in BACKLOG)

- Full temp-crate `cargo check` of generated scaffolding (currently syntactic validation only).
- No RE validation against a concrete pilot binary yet.
