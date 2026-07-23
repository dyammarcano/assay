# wrap-swap — Autonomy Charter
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

Standing authority for `/steps:autonomous`, granted by the operator on 2026-07-23.

## Envelope (granted once)
- **Scope:** drive the whole remaining roadmap until a real blocker or done.
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
