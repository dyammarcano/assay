# wrap-swap — UWP/Electron → Tauri Parity Toolkit — Design

**Date:** 2026-07-23
**Status:** Approved (brainstorming)
**Seed:** `docs/discovery/IDEA-BRIEF.md`

## Purpose

Apps built as UWP (WinRT/XAML/WinUI) or Electron carry a large surface of
OS-integration features that a naive "port the UI to Tauri" effort silently drops.
`wrap-swap` is a Rust toolkit that (1) ships an evidence-backed, **public-docs-cited**
capability/gap matrix (UWP + Electron → Tauri v2), (2) analyzes a given app's used
capabilities against that matrix, and (3) scaffolds Rust/Tauri bridge code to close the
achievable gaps — while honestly reporting the gaps that have no viable path.

## Decisions (locked in brainstorming)

- **MVP scope:** dataset **+** read-only analyzer **+** Rust bridge scaffolder (phased internally).
- **Detection model:** parse real app manifests/config when available; editable manual
  capability profile as fallback and override.
- **Mimicry bar:** **behavioral parity** (each capability works) **plus an explicit
  known-divergence report** (WebView-engine risk, no-path gaps, storage-backend differences,
  open questions). No promise of bit-for-bit visual/interaction parity.
- **Scaffolder fidelity:** working code only where a **proven recipe** exists; everything
  else → compilable stub with `todo!()` + inline citation. Never fabricate code for
  `none`/`open_question` capabilities.
- **Architecture:** single `cargo` workspace, data-driven, with a clean core/CLI split
  (Approach A).

## Architecture

```
wrap-swap/
├─ crates/
│  ├─ wrapswap-core/     # matrix model, dataset loader, analyzer, divergence engine
│  └─ wrapswap-cli/      # clap CLI: report | analyze | scaffold
├─ data/
│  └─ matrix.toml        # THE cited capability/gap dataset (source of truth)
└─ templates/            # code-gen templates for scaffold
```

The matrix TOML is embedded at build time (`include_str!`) and also loadable from an
external path via `--matrix` for iteration. Everything keys off a stable `capability_id`
(e.g. `uwp.toast`, `electron.native_module`).

## Components & data flow

### Matrix dataset (`data/matrix.toml`)
One record per capability:

| field | meaning |
|---|---|
| `id` | stable capability id, e.g. `uwp.toast` |
| `source` | `uwp` \| `electron` |
| `name` | human name |
| `description` | what the capability does |
| `tauri_path` | `native` \| `plugin` \| `custom_rust` \| `sidecar` \| `none` \| `open_question` |
| `severity` | gap severity (full \| partial \| none) |
| `citation_url` | REQUIRED public-doc source |
| `recipe` | optional: `proven` (with plugin/crate names) or `stub` |

Seeded from the IDEA-BRIEF matrix (UWP: live tiles, toast, background tasks, share target,
protocol activation, PasswordVault, app services; Electron: IPC, native modules, tray,
global shortcuts, auto-update, powerMonitor, deep links, dialogs/clipboard/autostart,
WebView fidelity). OPEN-QUESTION rows included with `tauri_path = "open_question"`.

### `report` subcommand
Renders the full matrix → Markdown (the public gap document). No input required. Citations
inline; `open_question` rows rendered as **OPEN QUESTION**, never asserted parity.

### `analyze` subcommand
Input = a **capability profile**. Produces:
- a **gap list** (capability → tauri_path + severity), and
- a **divergence report** (no-path items, WebView-engine risk, storage-backend differences,
  open questions).

### `scaffold` subcommand
Consumes a gap list. For each gap emits Rust command signatures + Tauri plugin registration
+ `Cargo.toml` deps. **Proven recipes → working code; everything else → compilable stub with
`todo!()` + inline citation.** `none`/`open_question` rows are emitted to the divergence
report only, never as code.

## Capability profile & parsers

`analyze`/`scaffold` accept a `profile.toml` (list of `capability_id`s) from either:
- **Parsers** — `AppxManifest.xml` (capabilities, protocols, background tasks, share targets)
  and Electron `package.json` + main-source grep (`electron` API imports, native-module deps).
  Emit an editable `profile.toml`.
- **Manual profile** — user hand-writes/edits `profile.toml`. Fallback and override.

## Error handling

- Unknown capability in a profile → warning + skip (not fatal).
- Missing/unparseable manifest → clear error pointing to the manual-profile path.
- `open_question`/`none` requested for scaffold → divergence report entry with citation,
  never a fabricated stub.
- Matrix load/parse error → fail fast with the offending record id.

## Testing

- **Core:** unit tests for matrix deserialization, analyzer gap-mapping, divergence
  classification; golden-file tests for `report` Markdown and each scaffold template.
- **Parsers:** fixture `AppxManifest.xml` + sample Electron `package.json`/main file →
  expected `profile.toml`.
- **Integration:** end-to-end `analyze` and `scaffold` on fixtures; assert generated stubs
  compile (`cargo check` in a temp dir) for the proven-recipe cases.

## Honesty invariants ("based on public docs, don't guess")

- Every matrix row carries a `citation_url`; a row without one fails a dataset lint test.
- `report` renders citations inline; unconfirmed items render as **OPEN QUESTION**.
- Scaffolder refuses to generate implementation code for any row not marked `recipe = "proven"`.

## Phasing (inside the MVP)

1. **Phase 1** — matrix dataset + `report` (the public gap document).
2. **Phase 2** — capability profile + parsers + `analyze` + divergence report.
3. **Phase 3** — `scaffold` (proven recipes + stubs).

Each phase is independently useful.

## Known gaps / follow-up research (from the brief)

- Confirm App Services / package-identity RPC model (needs a docs.microsoft.com citation).
- Confirm Electron native-module compatibility posture (fresh electronjs.org citation).
- Confirm any Tauri plugin for PowerMonitor-equivalent power events.
- No concrete pilot app was named; a real target binary would let a later phase run the
  `/unravel:*` RE workers to validate the matrix against an actual app.
