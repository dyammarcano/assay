# Assay Backlog
<!-- rev:002 (RFC 3339) 2026-07-23T00:00:00Z -->

## Frozen — out of scope until Windows is done (2026-07-23)
Deferred **by decision**, not blocked. Do not re-list these as blockers or start them until the
operator lifts the freeze (see the Windows Definition of Done in `docs/ROADMAP.md`).
- **WKWebView driver** (macOS) and **WebKitGTK driver** (Linux) for `webview-qa`.
- Any macOS/Linux packaging, path handling, or platform capability rows.
- Cross-*engine* divergence work in general: on Windows both the Electron original and the
  Tauri port are Chromium, so version skew (W4) is the signal that matters here.

Deferred ideas and follow-up work — sourced from `docs/discovery/IDEA-BRIEF.md` (§5 runner-up
ideas, §6 open questions) and the plan's deferred hardening. Not in the MVP (Phases 1-3).

## Deferred features (runner-up ideas)
- [~] **WinRT capability shim crate** — **started** (`crates/winrt-shim`): `ToastContent` +
  `ToastShim` trait + tested `ToastGeneric` XML builder shipped. Remaining: bind `to_xml()` to
  `ToastNotificationManager` via `windows-rs` (needs a registered AppUserModelID), and extend to
  tile/background-task/PasswordVault-equivalent calls. (Brief §5.2)
- [x] **Sidecar-based native-module migration kit** — **done** (`e3e32a4`): `detect_native_modules`
  + `generate_sidecar` (stdio-JSON sidecar skeleton, `SidecarClient`, `MIGRATION.md`) + the
  `assay sidecar` subcommand. Spec:
  `docs/superpowers/specs/2026-07-23-sidecar-migration-kit-design.md`. (Brief §5.3)
- [~] **Cross-WebView QA harness** — core + **Chromium driver done**: `crates/webview-qa`
  (`EngineBlob` + pairwise `diff` + `render_report` + `probe`/`capture`/`diff` CLI, and a real
  `ChromiumDriver` driving headless Edge over CDP). Remaining: **WKWebView** (macOS) and
  **WebKitGTK** (Linux) drivers — each needs its own OS, so a true cross-engine diff needs a
  mac/linux runner. Specs:
  `docs/superpowers/specs/2026-07-23-cross-webview-qa-harness-design.md`,
  `…-chromium-driver-design.md`. (Brief §5.4)
  **Hard dependency** of ADR 0001: a `visual`-tier capability can only be claimed once the
  engines it must match are actually captured — today every one reports "visual parity NOT
  measured".

## Open research questions
- [x] App Services / package-identity RPC — **resolved** (`4d1c950`): AppServiceConnection is
  keyed on MSIX PackageFamilyName; path = `custom_rust` under an MSIX-packaged build, `none`
  for a plain-exe app. Matrix row updated + cited.
- [x] Electron native-module compatibility posture — **resolved** (`4d1c950`): different ABI
  from Node, must be recompiled/reimplemented; matrix `native_module` row re-cited to
  electronjs.org.
- [x] PowerMonitor-equivalent Tauri plugin — **resolved** (`4d1c950`): none exists
  (plugins-workspace #990 open); path = `custom_rust` via windows-rs. Matrix row updated.
- [x] Decide the mimicry bar depth per capability — **accepted & implemented** (ADR 0001,
  Option B phased): `parity_tier` matrix field + explicit "visual parity NOT measured"
  divergence. Decided under autonomy charter, reversible. (Brief §6)

## Deferred hardening (from the plan self-review)
- [x] Syntactic validity of generated scaffolding — **done** (`4d1c950`): `syn::parse_file`
  test proves `bridge.rs` is valid Rust. (Full temp-crate `cargo check` — which needs the real
  tauri + plugin deps — remains deferred below.)
- [x] Full temp-crate `cargo check` of generated scaffolding — **done**: opt-in test
  `crates/core/tests/scaffold_compiles.rs` builds a throwaway crate around the generated
  bridge with the **real** tauri + plugin crates and compiles it. Covers every
  plugin-backed capability (13). Run it deliberately:
  `cargo test -p core --test scaffold_compiles -- --ignored --test-threads=1`
  (`#[ignore]` by default — needs network, ~4 min, pulls the full tauri tree.)
- Run `/unravel:*` RE workers against a concrete pilot binary to validate the matrix against a
  real app (none was supplied at discovery time).
