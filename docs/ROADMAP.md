# wrap-swap Roadmap
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

UWP/Electron → Tauri v2 parity toolkit. Phases derive from the implementation plan
(`docs/superpowers/plans/2026-07-23-wrap-swap.md`) and the idea brief
(`docs/discovery/IDEA-BRIEF.md`).

## Phase 1 — Cited gap matrix + `report` (MVP foundation)
- [ ] Cargo workspace + `wrapswap-core` matrix data model (Task 1)
- [ ] `data/matrix.toml` cited dataset + citation lint (Task 2)
- [ ] `report` Markdown renderer (Task 3)
- [ ] `wrapswap-cli` + `report` subcommand — end-to-end (Task 4)

**Exit:** `wrap-swap report` prints the full public-docs-cited UWP/Electron→Tauri gap matrix.

## Phase 2 — Capability profile + parsers + `analyze`
- [ ] Capability profile model + manual TOML loader (Task 5)
- [ ] AppxManifest.xml + Electron package.json/main parsers (Task 6)
- [ ] Analyzer + known-divergence report (Task 7)
- [ ] `analyze` subcommand — end-to-end (Task 8)

**Exit:** `wrap-swap analyze` maps a target app (parsed manifest or manual profile) into a
gap list + a divergence report (no-path, open-question, WebView-engine risk).

## Phase 3 — Rust bridge scaffolder
- [ ] Scaffolder: proven recipes emit wiring, others stub (Task 9)
- [ ] `scaffold` subcommand — end-to-end (Task 10)
- [ ] README + docs wiring (Task 11)

**Exit:** `wrap-swap scaffold` emits `bridge.rs` + `deps.txt` — real code for proven
recipes, `todo!()` stubs elsewhere, nothing fabricated for no-path/open-question gaps.

## Guiding invariant
Public-docs-only. Every capability claim carries a citation; unconfirmed items surface as
**OPEN QUESTION**, never as asserted parity.
