# Changelog

All notable changes to Assay are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-07-24

First release. Windows-only by design (see `docs/ROADMAP.md` for the Windows Definition of Done).

### Added

- **Cited capability matrix** — 24 UWP/WinRT and Electron capabilities mapped to their Tauri v2
  path, every row carrying a public-doc `citation_url` (enforced by a test).
- **`assay report`** — renders the matrix as Markdown, with a per-path rollup, parity Tier
  column, legend, and a `--source uwp|electron` filter.
- **`assay analyze`** — classifies an app's capabilities into a gap list plus a
  known-divergence report. Input from a hand-written profile, an `AppxManifest.xml`, or an
  Electron `package.json` + main-process **directory** (scanned recursively, skipping
  `node_modules`); a single-file scan warns that it is probably partial.
- **`assay scaffold`** — emits `bridge.rs` + `deps.txt`. Plugin wiring is emitted only for
  recipes *compile-verified against the real tauri crates*; plugins needing configuration become
  a dependency plus a commented example rather than code that would not build.
- **`assay sidecar`** — detects native Node modules and scaffolds a stdio-JSON sidecar skeleton,
  a `SidecarClient`, a `tauri.conf.snippet.json`, and a per-module `MIGRATION.md` checklist.
  Every handler is an explicit `todo!()`; no module logic is ported.
- **`webview-qa`** — cross-WebView divergence harness: `init-config`, `probe` (injectable JS),
  `capture` (live headless Edge over the DevTools protocol), and `diff` (pairwise divergence
  report, severity-ranked).
- **`winrt-shim`** — reusable WinRT toast content model and `ToastGeneric` XML builder.
- **Parity tiers** (ADR 0001) — capabilities are `behavioral` or `visual`; every visual-tier gap
  is reported as *visual parity NOT measured* until live engine drivers exist.
- Docs: architecture diagrams, roadmap, backlog, known issues, autonomy charter, and a
  state/gap-to-usable report.

### Known limitations

- **Windows only.** macOS/Linux is deferred by decision, not blocked.
- **Electron detection is textual** — dynamically-built/aliased requires and bundled (webpack'd)
  main processes can defeat it.
- **Visual-tier parity is declared, not measured** — needs WKWebView/WebKitGTK drivers.
- **WinRT toast dispatch is not wired** — requires a registered AppUserModelID.
- A capture on Windows is **single-engine** and is always labelled as such.

[0.1.0]: https://github.com/dyammarcano/assay/releases/tag/v0.1.0
