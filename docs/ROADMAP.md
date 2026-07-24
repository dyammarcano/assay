# Assay Roadmap
<!-- rev:003 (RFC 3339) 2026-07-23T00:00:00Z -->

**Status:** Phases 1–3 implemented and tested — commit `4ec3be3`. Post-MVP maturation
(steps 1–10) landed through `741146e`; 18 tests green, clippy `-D warnings` clean.

UWP/Electron → Tauri v2 parity toolkit. Phases derive from the implementation plan
(`docs/superpowers/plans/2026-07-23-assay.md`) and the idea brief
(`docs/discovery/IDEA-BRIEF.md`).

## Phase 1 — Cited gap matrix + `report` (MVP foundation)
- [x] Cargo workspace + `core` matrix data model (Task 1)
- [x] `data/matrix.toml` cited dataset + citation lint (Task 2)
- [x] `report` Markdown renderer (Task 3)
- [x] `wrapswap-cli` + `report` subcommand — end-to-end (Task 4)

**Exit:** `assay report` prints the full public-docs-cited UWP/Electron→Tauri gap matrix.

## Phase 2 — Capability profile + parsers + `analyze`
- [x] Capability profile model + manual TOML loader (Task 5)
- [x] AppxManifest.xml + Electron package.json/main parsers (Task 6)
- [x] Analyzer + known-divergence report (Task 7)
- [x] `analyze` subcommand — end-to-end (Task 8)

**Exit:** `assay analyze` maps a target app (parsed manifest or manual profile) into a
gap list + a divergence report (no-path, open-question, WebView-engine risk).

## Phase 3 — Rust bridge scaffolder
- [x] Scaffolder: proven recipes emit wiring, others stub (Task 9)
- [x] `scaffold` subcommand — end-to-end (Task 10)
- [x] README + docs wiring (Task 11)

**Exit:** `assay scaffold` emits `bridge.rs` + `deps.txt` — real code for proven
recipes, `todo!()` stubs elsewhere, nothing fabricated for no-path/open-question gaps.

## Post-MVP maturation (this cycle — `/steps:next`, done)
- [x] Quality gate: fmt + clippy `-D warnings` + check scripts
- [x] Scaffold syntactic-validity test (`syn::parse_file`)
- [x] Resolve open research questions (App Services, native modules, PowerMonitor) — matrix cited
- [x] Broaden matrix (+11 capabilities)
- [x] `report --source uwp|electron` filter + per-path summary rollup + legend
- [x] `docs/ISSUES.md`
- [~] WinRT capability shim crate (`crates/winrt-shim`) — XML core shipped; windows-rs dispatch next
- [x] Sidecar native-module migration kit — done (`e3e32a4`): detection + codegen + `sidecar` cmd
- [x] Cross-WebView QA harness — done: `webview-qa` crate (engine-blob model + differ + `diff` cmd)

## Hardening & compliance cycle (`/steps:next` all — done)
- [x] LICENSE (BSD 3-Clause) — was missing entirely
- [x] Project `AGENTS.md` + `CLAUDE.md`
- [x] Sidecar kit emits `tauri.conf.snippet.json` (externalBin + shell scope) — spec gap closed
- [x] `webview-qa` probe JS + `webview-qa.toml` config schema (+ `probe`/`init-config` cmds)
- [x] CLI error handling — no more `.expect()` panics; clean messages + exit codes (2 = usage)
- [x] Golden snapshot tests for generated bridge/sidecar/migration output
- [x] `docs/ARCHITECTURE.md` (mermaid: crate graph + all three flows)
- [x] `examples/` fixtures (UWP manifest, Electron app, profile) + README walkthrough
- [x] CI workflow (`.github/workflows/ci.yml`: fmt + clippy + test + build)
- [x] Mimicry-bar decision — ADR 0001 **accepted** (Option B, phased) and implemented:
  `parity_tier` field + "visual parity NOT measured" divergence + report Tier column

## SCOPE FREEZE — Windows platform first (2026-07-23)

All work targets **Windows** until the Definition of Done below is met. macOS/Linux items are
**deferred by decision, not blocked** — see "Frozen" in `docs/BACKLOG.md`. Rationale and the
technical consequences of the freeze are in `AGENTS.md`.

### Windows — Definition of Done
- [ ] **W1. WinRT toast actually fires.** Wire `winrt-shim`'s `ToastContent::to_xml()` to
  `ToastNotificationManager` via `windows-rs`. *Gated on operator consent:* an unpackaged app
  needs an AppUserModelID registered via a Start Menu shortcut (a change outside this repo).
- [ ] **W2. UWP capability coverage.** Broaden `data/matrix.toml` across the WinRT surface a
  real app uses (sensors, media, geolocation, print, contacts, background transfer, file
  associations) — every row cited, per the citation invariant.
- [ ] **W3. MSIX packaging path.** Document + scaffold the packaged-build route. This is the
  gate that unlocks `uwp.app_services` and `uwp.background_tasks` (and is the only context in
  which Live Tiles/Share Target could ever exist).
- [ ] **W4. WebView2 version-skew measurement.** On Windows both sides are Chromium, so the real
  divergence signal is version skew: capture the installed WebView2/Edge version and compare it
  against the original Electron app's bundled Chromium version. Feeds ADR 0001's visual tier.
- [ ] **W5. Validate against a real Windows binary** via `/unravel:*` — needs a pilot
  UWP/Electron app from the operator.

**Done for Windows already:** cited matrix + report/analyze/scaffold, sidecar kit, parity tier
(ADR 0001), live `chromium-edge` capture driver, compile-verified scaffolding, Windows CI.

## Remaining — deferred or blocked
These need inputs/runtimes not available in the build environment; each would merge on an
unverifiable green, so they are surfaced rather than faked:
- **winrt-shim native toast dispatch** — needs a registered AppUserModelID (packaged app) + a
  desktop session; "toast shown" isn't automatable-verifiable.
- ~~**Full temp-crate `cargo check` of generated scaffolding**~~ — **DONE**: opt-in test
  compiles the generated bridge against the real tauri + plugin crates. It immediately caught
  three matrix rows claiming a non-existent `init()` (stronghold, global-shortcut, updater)
  and a missing `tauri` dependency.
- ~~**webview-qa live engine drivers**~~ — **Chromium/WebView2 family DONE** (`ChromiumDriver`,
  headless Edge over CDP; host-gated live test passes). **WKWebView / WebKitGTK drivers are now
  FROZEN, not blocked** — deferred by the Windows-first scope decision. Single-engine runs are
  correct for Windows scope and are labelled as such.
- **`/unravel:*` matrix validation** — needs a concrete pilot binary (none supplied at intake).

## Guiding invariant
Public-docs-only. Every capability claim carries a citation; unconfirmed items surface as
**OPEN QUESTION**, never as asserted parity.

