# AGENTS.md — wrap-swap
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

Project-specific agent instructions for **wrap-swap**, a UWP/Electron → Tauri v2 parity
toolkit. Global rules come from `~/.claude/AGENTS.md`; this file adds what's specific here
and wins on conflict within this repo.

## What this project is

A Rust cargo workspace that ships a **cited** UWP/Electron → Tauri capability/gap matrix,
analyzes a target app's capabilities against it, and scaffolds Rust/Tauri bridge code for the
achievable gaps — reporting the rest honestly instead of faking them.

## Layout

| Path | Crate | Responsibility |
|---|---|---|
| `crates/core` | `core` | matrix model + dataset loader, parsers, analyzer, scaffolder, sidecar codegen |
| `crates/cli` | `cli` | the `wrap-swap` binary (`report`/`analyze`/`scaffold`/`sidecar`) |
| `crates/winrt-shim` | `winrt-shim` | reusable WinRT capability shim (toast content + XML) |
| `crates/webview-qa` | `webview-qa` | cross-WebView divergence differ + `diff` CLI |
| `data/matrix.toml` | — | the cited capability/gap dataset (source of truth) |

## Safety invariants — these must survive every change

1. **Citation invariant** — every `data/matrix.toml` row keeps a non-empty `citation_url`.
   Enforced by the `every_row_has_a_citation` test. Unconfirmed → `tauri_path = "open_question"`,
   never asserted parity.
2. **No fabricated code** — the scaffolder emits implementation code ONLY for `recipe = "proven"`
   rows. `none` / `open_question` capabilities go to the divergence report, never to generated
   code. The sidecar kit emits `todo!()` stubs; it never claims a module is migrated.
   **`recipe = "proven"` means compile-verified**, not "looks right": a plugin row must pass the
   `scaffold_compiles` test. Never assume a plugin exposes `init()` — several don't; put the real
   call in `init_expr`, and if it needs developer input it is guidance, not a drop-in.
3. **Honest reporting** — the divergence report and the webview-qa report always state what was
   NOT covered (no-path gaps, engines not exercised). A 1-engine run never reads as cross-engine.
4. **No publishing** — every crate keeps `publish = false`. Crate names are short and plain with
   no project prefix (operator override: the core crate is literally `core`, imported as `core::`).

## Build / test / lint

```powershell
cargo test --workspace                                  # full suite
cargo clippy --workspace --all-targets -- -D warnings   # lint gate (must be clean)
cargo fmt --all                                         # format

# Opt-in (network, ~4 min): compile generated scaffolding against the REAL tauri crates.
# Run this whenever you add/change a `plugin`, `init_expr`, or `recipe = "proven"` row.
cargo test -p core --test scaffold_compiles -- --ignored --test-threads=1
```

The green gate is a **direct tool run** — never merge or check anything off on a self-reported
green. Scripts live in `.scripts/` (gitignored, append-only audit trail) per the global
scripts-first rule.

## Conventions

- Rust edition 2021, MSRV 1.74 — do not use APIs newer than the MSRV floor (e.g. prefer a
  `match` over `Option::is_none_or`, which needs 1.82).
- Conventional commits, no AI attribution.
- Adding a capability = add a `data/matrix.toml` row **with a public-doc citation**, then let the
  snapshot test regenerate (`INSTA_UPDATE=always cargo test`, review, commit the `.snap`).
- Point-in-time docs (specs, plans, ADRs) get no `rev:` tag; living docs (this file, README,
  ROADMAP, BACKLOG, ISSUES, ARCHITECTURE) do.

## Docs map

`docs/ROADMAP.md` (phases) · `docs/BACKLOG.md` (deferred + blocked) · `docs/ISSUES.md` (known
limits) · `docs/ARCHITECTURE.md` (diagrams) · `docs/AUTONOMY.md` (autonomous-run charter +
decision log) · `docs/discovery/IDEA-BRIEF.md` (the cited origin research) ·
`docs/superpowers/specs|plans/` (point-in-time designs) · `docs/adr/` (decisions).
