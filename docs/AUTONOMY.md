# Assay — Autonomy Charter
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

Standing authority for `/steps:autonomous`, granted by the operator on 2026-07-23.

## Envelope (granted once)
- **Scope:** drive the whole remaining roadmap until a real blocker or done — **narrowed
  2026-07-23 to the Windows platform only** (see the Windows Definition of Done in
  `docs/ROADMAP.md`). macOS/Linux work is frozen until the operator lifts it.
- **Design forks:** decide-and-log — pick the recommended option, record rationale in the
  spec's "Settled forks" + the decision log below; do NOT pause except a NEVER guardrail.
- **Guardrails — allowed without asking:** commit locally · build/run/smoke-test locally ·
  merge feature branches to `main`.
- **Cadence:** report at each phase boundary, then continue.

## Standing authority
Per phase, run spec → self-review → plan → execute (TDD) → whole-branch review → verify
(direct build/test/clippy/fmt) → merge to main → docs sync → next, without per-step approval.

## Guardrails — NEVER without explicit say-so
- Rewrite/delete git history, force-push, `reset --hard` shared refs.
- Delete operator data/files this work didn't create.
- Publish anywhere (no remote exists; `cargo publish` is already globally forbidden — see AGENTS.md).
- Spend money / paid-cloud services.
- Change the license or the project's safety invariants below.

## Project safety invariants (must survive every change)
1. **Public-docs-only / citation invariant** — every matrix row keeps a non-empty
   `citation_url` (enforced by `every_row_has_a_citation`); unconfirmed → `open_question`, never
   asserted parity.
2. **No fabricated code** — the scaffolder emits implementation code ONLY for `recipe = "proven"`
   rows; `none`/`open_question` go to the divergence report, never to generated code.
3. **No-publish** — all crates keep `publish = false`; plain crate names (no registry-uniqueness
   prefix). Per operator override (2026-07-23), the core crate is named `core` (imported as
   `core::`) — the std-shadow exception was explicitly waived.
4. **Green gate is a direct tool run** — never merge on a self-reported green; `cargo test` +
   `cargo clippy -D warnings` + `cargo fmt --check` must pass.

## Stop conditions
- A blocker I can't resolve (missing capability, a gate I can't green, a product-direction ambiguity).
- Would need a NEVER guardrail or an unauthorized gated action.
- Scope satisfied (roadmap + backlog exhausted).

## Decision log (newest first)
- **2026-07-23 — SCOPE FREEZE: Windows first (operator decision).** Complete the Windows
  platform before any cross-platform work. Recorded as a Definition of Done (W1–W5) in
  `docs/ROADMAP.md`; macOS/Linux items moved to a "Frozen" section in `docs/BACKLOG.md` and
  reclassified **deferred-by-decision, not blocked**. Key technical consequence captured in
  `AGENTS.md`: on Windows an Electron→Tauri port is Chromium-to-Chromium (WebView2 *is*
  Chromium), so engine divergence collapses to version skew — while UWP→Tauri is native-XAML to
  HTML, a different problem entirely.
- **2026-07-23 — scaffold-compiles check, and what it proved:** operator greenlit the real-tauri
  `cargo check`. It found the scaffolder had been emitting code that **does not compile**:
  `tauri` was missing from `deps.txt`, and three rows (`stronghold`, `global-shortcut`,
  `updater`) claimed a `tauri_plugin_x::init()` that does not exist. Fix is data-driven — new
  optional `init_expr` / `crate_version` matrix fields, and `.plugin(...)` is emitted only for a
  genuinely proven drop-in; a plugin needing developer configuration becomes a dependency plus a
  commented example. All 13 plugin-backed rows now compile. Lesson recorded in AGENTS.md:
  `recipe = "proven"` must mean compile-verified, never "looks right".
- **2026-07-23 — Chromium driver approach (headless Edge + CDP, not embedded `wry`):** probed the
  host instead of re-reporting "blocked" and found Edge + the WebView2 Runtime installed. Chose
  CDP over embedding: no GUI event loop, no large native dep tree, same Chromium engine. Labelled
  `chromium-edge` (not `webview2`) per the honesty invariants. Forks in
  `docs/superpowers/specs/2026-07-23-chromium-driver-design.md`. Two real bugs were found only
  because the test drove a live engine: a `Host` header missing its port (DevTools silently
  ignores mismatched Host) and a read-to-EOF against a keep-alive server (discarded the response
  on timeout).
- **2026-07-23 — ADR 0001 mimicry bar settled autonomously (Option B, phased):** the operator
  re-invoked `/steps:autonomous` with the ADR still Proposed; under decide-and-log authority I
  accepted my own recommendation rather than pausing. Chose B because it is the *conservative*
  option — it asserts nothing new, keeps behavioral parity as the only enforced bar, and makes
  unmeasured visual claims explicit. Implemented as an optional `parity_tier` matrix field +
  an "unmeasured visual parity" divergence entry. **Reversible**; flag if the operator would
  have chosen A or C. Sub-forks (field location, which caps are visual, how to surface) are
  recorded in `docs/superpowers/specs/2026-07-23-parity-tier-design.md`.
- **2026-07-23 — Autonomous run stopped (blocker cluster):** shipped Phase 1 (sidecar kit,
  `e3e32a4`) and Phase 2 (webview-qa core, `c38ce45`). Stopped before Phase 3: all remaining
  roadmap work (winrt-shim native dispatch, full scaffold `cargo check`, webview-qa live drivers,
  `/unravel:*` validation) needs external prerequisites (AUMID/packaging, real tauri+plugin deps,
  live WebView engines, a pilot binary) and would merge on an unverifiable green — surfaced per
  invariant #4 rather than faked.
- **2026-07-23 — Crate naming override:** operator instructed "rename all crates, remove prefix";
  renamed package `wrapswap-core` → `core`, waiving the std-`core`-shadow exception for this
  project. `cli` imports it as `core::`; verified building clean. The global AGENTS.md exception
  is unchanged (applies to other projects unless similarly overridden).
- **2026-07-23 — Execution engine:** solo-agent direct implementation on `main` with direct
  `cargo` verification as the green gate, rather than per-task subagent fan-out. Rationale: single
  local repo, small well-scoped phases, full context in hand; direct verification satisfies the
  "never trust a self-reported green" rule. Branch/merge workflow available if a phase warrants it.
- **2026-07-23 — Phase order:** sidecar migration kit → cross-WebView harness → winrt-shim
  windows-rs wiring. Rationale: the sidecar kit is pure-Rust codegen (fully implementable +
  testable here); the harness differ-core is mock-testable (drivers host-gated); winrt-shim
  dispatch needs a registered AppUserModelID (limited testability in this env) so it goes last.
