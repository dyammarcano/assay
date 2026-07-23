# wrap-swap Backlog
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

Deferred ideas and follow-up work — sourced from `docs/discovery/IDEA-BRIEF.md` (§5 runner-up
ideas, §6 open questions) and the plan's deferred hardening. Not in the MVP (Phases 1-3).

## Deferred features (runner-up ideas)
- [~] **WinRT capability shim crate** — **started** (`crates/winrt-shim`): `ToastContent` +
  `ToastShim` trait + tested `ToastGeneric` XML builder shipped. Remaining: bind `to_xml()` to
  `ToastNotificationManager` via `windows-rs` (needs a registered AppUserModelID), and extend to
  tile/background-task/PasswordVault-equivalent calls. (Brief §5.2)
- [x] **Sidecar-based native-module migration kit** — **done** (`e3e32a4`): `detect_native_modules`
  + `generate_sidecar` (stdio-JSON sidecar skeleton, `SidecarClient`, `MIGRATION.md`) + the
  `wrap-swap sidecar` subcommand. Spec:
  `docs/superpowers/specs/2026-07-23-sidecar-migration-kit-design.md`. (Brief §5.3)
- [~] **Cross-WebView QA harness** — **core done**: `crates/webview-qa` (`EngineBlob` model +
  pairwise `diff` + `render_report` + `webview-qa diff` CLI over recorded blobs). Remaining: the
  live per-engine drivers (WebView2/WKWebView/WebKitGTK) that produce the blobs — host-gated
  integration. Spec: `docs/superpowers/specs/2026-07-23-cross-webview-qa-harness-design.md`. (Brief §5.4)
  **Now a hard dependency** of ADR 0001: no `visual`-tier capability can be claimed until these
  drivers exist — every one is reported as "visual parity NOT measured" today.

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
- Full temp-crate `cargo check` of generated scaffolding (needs tauri + plugin deps resolvable).
- Run `/unravel:*` RE workers against a concrete pilot binary to validate the matrix against a
  real app (none was supplied at discovery time).
