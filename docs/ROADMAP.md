# wrap-swap Roadmap
<!-- rev:003 (RFC 3339) 2026-07-23T00:00:00Z -->

**Status:** Phases 1–3 implemented and tested — commit `4ec3be3`. Post-MVP maturation
(steps 1–10) landed through `741146e`; 18 tests green, clippy `-D warnings` clean.

UWP/Electron → Tauri v2 parity toolkit. Phases derive from the implementation plan
(`docs/superpowers/plans/2026-07-23-wrap-swap.md`) and the idea brief
(`docs/discovery/IDEA-BRIEF.md`).

## Phase 1 — Cited gap matrix + `report` (MVP foundation)
- [x] Cargo workspace + `wrapswap-core` matrix data model (Task 1)
- [x] `data/matrix.toml` cited dataset + citation lint (Task 2)
- [x] `report` Markdown renderer (Task 3)
- [x] `wrapswap-cli` + `report` subcommand — end-to-end (Task 4)

**Exit:** `wrap-swap report` prints the full public-docs-cited UWP/Electron→Tauri gap matrix.

## Phase 2 — Capability profile + parsers + `analyze`
- [x] Capability profile model + manual TOML loader (Task 5)
- [x] AppxManifest.xml + Electron package.json/main parsers (Task 6)
- [x] Analyzer + known-divergence report (Task 7)
- [x] `analyze` subcommand — end-to-end (Task 8)

**Exit:** `wrap-swap analyze` maps a target app (parsed manifest or manual profile) into a
gap list + a divergence report (no-path, open-question, WebView-engine risk).

## Phase 3 — Rust bridge scaffolder
- [x] Scaffolder: proven recipes emit wiring, others stub (Task 9)
- [x] `scaffold` subcommand — end-to-end (Task 10)
- [x] README + docs wiring (Task 11)

**Exit:** `wrap-swap scaffold` emits `bridge.rs` + `deps.txt` — real code for proven
recipes, `todo!()` stubs elsewhere, nothing fabricated for no-path/open-question gaps.

## Post-MVP maturation (this cycle — `/steps:next`, done)
- [x] Quality gate: fmt + clippy `-D warnings` + check scripts
- [x] Scaffold syntactic-validity test (`syn::parse_file`)
- [x] Resolve open research questions (App Services, native modules, PowerMonitor) — matrix cited
- [x] Broaden matrix (+11 capabilities)
- [x] `report --source uwp|electron` filter + per-path summary rollup + legend
- [x] `docs/ISSUES.md`
- [~] WinRT capability shim crate (`crates/winrt-shim`) — XML core shipped; windows-rs dispatch next
- [ ] Sidecar native-module migration kit — spec ready (`docs/superpowers/specs/…sidecar…`)
- [ ] Cross-WebView QA harness — spec ready (`docs/superpowers/specs/…webview…`)

## Guiding invariant
Public-docs-only. Every capability claim carries a citation; unconfirmed items surface as
**OPEN QUESTION**, never as asserted parity.

