# Assay — State & Gap to Usable

**Date:** 2026-07-24 · **HEAD:** `39f0bfe` (19 commits) · **Version:** 0.1.0 (untagged)
**Type:** Rust cargo workspace → two developer CLIs (`assay`, `webview-qa`)

## TL;DR

- **The engine is real and unusually well-verified.** 24 cited capability rows, 49 test
  functions, `clippy -D warnings` clean, generated scaffolding is *compile-verified against the
  real tauri crates*, and the WebView capture driver runs a live headless Edge. That rigor is
  above the bar for a 0.1.0.
- **Nobody can actually install it.** No remote, no tags, no release, no `CHANGELOG.md`, and
  crates.io is permanently excluded by operator policy (`publish = false` everywhere). The only
  way to run it today is `git clone` + `cargo run -p cli -- …`.
- **`assay --version` fails** (`error: unexpected argument '--version'`) — verified, not
  assumed. Basic table stakes for a distributable CLI.
- **The single biggest capability risk: `analyze` reads exactly ONE Electron main file.** Real
  Electron apps split the main process across many files, so on a real app it will
  **silently under-report** capabilities — undermining trust in the core output.
- **CI exists but has never executed.** `.github/workflows/ci.yml` is present; there is no
  remote, so no run has ever happened. Do not read the green local suite as "CI green".
- **Rough effort to Usable v1: ~half a day of work, plus one operator decision** (whether to
  create a git remote — without it, "one-command install" is not reachable).

## Where we are

### Capability surface (verified via `--help` and smoke runs)

`assay` (bin from `crates/cli`, package `cli`):

| Command | Does | State |
|---|---|---|
| `report` | Render the cited capability/gap matrix as Markdown (`--source uwp\|electron`, path rollup, Tier column, legend) | Working |
| `analyze` | Profile / AppxManifest / Electron pkg+main → gap list + known-divergence report | Working, **input limitation below** |
| `scaffold` | Emit `bridge.rs` + `deps.txt` for a profile's gaps | Working, compile-verified |
| `sidecar` | Detect native modules → stdio-JSON sidecar kit + `tauri.conf.snippet.json` + `MIGRATION.md` | Working |

`webview-qa` (bin from `crates/webview-qa`): `capture` (live headless Edge over CDP),
`diff` (pairwise engine divergence report), `probe` (render injectable JS), `init-config`.

All eight subcommands were smoke-run end-to-end against `examples/`.

### Quality bar (measured, not claimed)

- **49 test functions** across 4 crates; `cargo test --workspace` green (15 test binaries).
- **`cargo clippy --workspace --all-targets -- -D warnings`** exits 0.
- **Honesty invariants enforced by tests:** every matrix row must carry `citation_url`
  (`every_row_has_a_citation`); generated Rust must parse (`syn::parse_file`); generator output
  is locked by 5 `insta` golden snapshots.
- **Opt-in `scaffold_compiles` test** builds a throwaway crate around the generated bridge with
  the *real* tauri + plugin crates and compiles it — all 14 dependency-contributing rows pass.
  This caught three rows claiming a `tauri_plugin_x::init()` that does not exist.
- **Live-engine test** (`capture_live`) captures a real `EngineBlob` from headless Edge, or
  skips with a printed reason — never a false green.

### Platform

Windows only, by explicit scope freeze (`AGENTS.md`, `docs/ROADMAP.md` W1–W5). macOS/Linux is
**deferred by decision, not blocked**. The Chromium driver hardcodes Windows Edge paths
(`crates/webview-qa/src/driver.rs`, `EDGE_PATHS`).

## What "usable" means here

**Persona:** a Windows developer with an existing Electron or UWP app who is evaluating or
executing a Tauri port. They have never seen this repo. They may or may not have Rust installed.

**Core value they want:** *"Tell me which of my app's capabilities Tauri can and cannot do —
with citations, not guesses — and generate the bridge code for the ones it can."*

**Usable v1** = that developer can:
1. **Install in one step** — get `assay` runnable without learning the repo's crate layout.
2. **Follow a quickstart to a first useful result** — pointed at **their own app**, not at our
   fixtures, and get a gap report they understand.
3. **Trust it on Windows** — the output is not silently incomplete, and the limits are stated
   where they'll actually read them (README), not only in `docs/ISSUES.md`.

Disagree with this? The gap table below is keyed to it, so changing the definition changes the
critical path.

## Gap analysis

```
┌──────────────────┬──────────────────────────────────────────────────────────┬──────────┬────────┐
│    Dimension     │                           Gap                            │ Severity │ Effort │
├──────────────────┼──────────────────────────────────────────────────────────┼──────────┼────────┤
│ Distribution     │ No install path at all: no remote, no tag, no release,    │ Blocker  │ Medium │
│ (MUST)           │ no CHANGELOG. crates.io permanently excluded by policy.   │          │ gated  │
│                  │ Only `git clone` + `cargo run -p cli --` works today.     │          │        │
├──────────────────┼──────────────────────────────────────────────────────────┼──────────┼────────┤
│ Capability       │ `analyze --electron-main` takes ONE file; real Electron   │ Blocker  │ Medium │
│ (MUST)           │ main processes span many files → silent under-reporting.  │          │        │
├──────────────────┼──────────────────────────────────────────────────────────┼──────────┼────────┤
│ Distribution     │ `assay --version` errors out (no clap `version`).     │ Major    │ Quick  │
│ (MUST)           │ Blocks bug reports and reproducibility.                   │          │        │
├──────────────────┼──────────────────────────────────────────────────────────┼──────────┼────────┤
│ Docs / UX        │ Quickstart is a repo-developer flow (`cargo run -p cli`)  │ Major    │ Quick  │
│ (MUST)           │ against OUR fixtures — never "point it at YOUR app".      │          │        │
├──────────────────┼──────────────────────────────────────────────────────────┼──────────┼────────┤
│ Quality / trust  │ CI workflow exists but has NEVER RUN (no remote). Local   │ Major    │ Quick  │
│ (MUST)           │ green ≠ CI green; must not be presented as verified.      │          │ gated  │
├──────────────────┼──────────────────────────────────────────────────────────┼──────────┼────────┤
│ Docs / UX        │ Real limits (single-file parse, single-engine capture,    │ Major    │ Quick  │
│ (MUST)           │ visual tier unmeasured) live only in docs/ISSUES.md.      │          │        │
├──────────────────┼──────────────────────────────────────────────────────────┼──────────┼────────┤
│ Capability       │ 24 capability rows — decent but thin vs the real WinRT /  │ Minor    │ Medium │
│ (v2 — W2)        │ Electron surface a large app touches.                     │          │        │
├──────────────────┼──────────────────────────────────────────────────────────┼──────────┼────────┤
│ Integration      │ No `--json` output; everything is Markdown/prose, so the  │ Minor    │ Medium │
│ (v2)             │ tool can't feed a scripted migration pipeline.            │          │        │
├──────────────────┼──────────────────────────────────────────────────────────┼──────────┼────────┤
│ Capability       │ winrt-shim toast never actually fires (stub); MSIX path   │ Minor    │ Large  │
│ (v2 — W1/W3)     │ undocumented, so app_services/background_tasks unreachable│          │ gated  │
└──────────────────┴──────────────────────────────────────────────────────────┴──────────┴────────┘
```

## Critical path to Usable v1

> **Status 2026-07-24:** items **1–4 are DONE** (commit below). Only the gated distribution
> decision (5–6) remains between here and Usable v1.

Ordered. Items 1–4 need nothing from anyone; 5–6 need an operator decision.

1. **Add `--version` to both CLIs** — `#[command(version)]` in `crates/cli/src/main.rs` and
   `crates/webview-qa/src/main.rs`, wired to `CARGO_PKG_VERSION`. *(Quick)*
2. **Scan a directory for Electron main sources.** Accept `--electron-main` as a file *or* a
   directory (and/or add `--electron-src <dir>`); walk `.js`/`.mjs`/`.cjs`/`.ts`, concatenate,
   then parse. When given a single file, warn that detection is limited to it. This converts the
   core output from "silently incomplete" to "trustworthy or explicitly caveated". *(Medium)*
3. **Rewrite the README quickstart for an end user.** Install → point at *your* app → read the
   report, with a realistic expected output block. Keep the fixture walkthrough below it as a
   "try it without your own app" section. *(Quick)*
4. **Promote the real limits into the README** (single-engine capture, visual tier unmeasured,
   Windows-only, matrix breadth) — a short "Known limits" section linking `docs/ISSUES.md`.
   *(Quick)*
5. **Decide the distribution channel** *(operator)*. crates.io is permanently out. Options:
   (a) create a GitHub remote → `cargo install --git <url>` becomes the one-command install and
   CI starts actually running; (b) stay local → document + verify
   `cargo install --path crates/cli` as the supported install and drop the "one-command" claim
   from the definition of usable. *(Medium, gated)*
6. **Tag `v0.1.0` + add `CHANGELOG.md`**, once (5) is decided. If (a), attach a prebuilt
   `assay.exe` to the release so non-Rust users are served at all. *(Medium, gated)*

## Explicitly deferred / out of scope

Not on the path — absent **by decision**, so don't read them as accidental omissions:

- **macOS/Linux everything** — WKWebView/WebKitGTK drivers, `.app`/AppImage packaging. Frozen
  until the Windows DoD is met (`AGENTS.md` scope freeze).
- **crates.io publishing** — permanently excluded by operator policy; every crate is
  `publish = false`.
- **W1 WinRT toast dispatch** — needs consent to register a Start Menu AppUserModelID shortcut.
- **W3 MSIX packaging path** — unlocks `uwp.app_services` / `uwp.background_tasks`; v2.
- **W5 `/unravel:*` validation against a real binary** — needs a pilot app from the operator.
- **Visual-tier parity measurement** — ADR 0001 deliberately reports it as *not measured*.
- **`--json` output, matrix breadth (W2)** — real, but v2; they don't block first use.

## Definition of Done for "Usable v1"

- [x] `assay --version` and `webview-qa --version` both print the version.
      *(verified: `assay 0.1.0`)*
- [x] A documented install command that a fresh user can run, **verified by actually running
      it** — `cargo install --path crates/cli` exits 0 and produces a working binary. Verified
      via `--root <temp>` so the operator's `~/.cargo/bin` was not modified.
- [x] README quickstart takes a user from install → **their own app** → a gap report, with a
      realistic expected-output block.
- [x] `analyze` handles a multi-file Electron main process (directory scan, recursive, skips
      `node_modules`), and warns explicitly when a single-file run is partial.
- [x] README has a "Known limits" section (Windows-only, textual detection, bundled-main
      caveat, matrix breadth, visual tier unmeasured, CI never run).
- [ ] CI has either **actually executed once**, or the README states plainly that it hasn't.
      *(README now states it plainly — upgrade to "executed" requires a remote.)*
- [ ] `v0.1.0` tagged with a `CHANGELOG.md`. **Gated on the distribution decision.**

## Honest uncertainties

- ~~`cargo install --path crates/cli` is untested.~~ **Resolved 2026-07-24** — verified with
  `--root <temp dir>`, which proves the command without writing to the operator's `~/.cargo/bin`.
  Exit 0, binary produced, `--version` and `report` both work.
- **Bundled main processes remain a real hole.** The directory scan fixes multi-file apps, but a
  webpack'd/transpiled main process can defeat identifier grepping entirely. That is a deeper
  capability question (parse the bundle? read the source map?), not a parsing tweak. Documented
  in the README's Known limits rather than papered over.
- **The `core` crate name has a standing cost.** A dependency literally named `core` shadows
  Rust's built-in `core` in the consuming crate; this broke clap's `version` derive and is now
  worked around by importing it in `cli` as `corelib`. Any future consumer hits the same thing.
- **"24 capabilities is thin"** is a judgment call, not a measurement — there is no canonical
  list of "capabilities a real app uses" to measure against.
