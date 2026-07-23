# CLAUDE.md — wrap-swap
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

Claude Code entry point for this repo. The canonical, cross-tool instructions live in
**AGENTS.md** (imported below) — edit that file, not this one.

@AGENTS.md

## Claude-Code-only notes

- **Autonomous runs:** `docs/AUTONOMY.md` is the standing charter (scope, guardrails, decision
  log). If it exists and is current, `/steps:autonomous` skips the envelope question and
  proceeds under it.
- **Blocked work:** the "Remaining — blocked on external prerequisites" section of
  `docs/ROADMAP.md` lists items that need inputs this environment can't supply (an AppUserModelID
  for the winrt-shim toast dispatch, live WebView engines, resolvable tauri deps, a pilot binary
  for `/unravel:*`). Don't re-attempt these blind — they'd merge on an unverifiable green.
- **`/unravel:*`:** if a real UWP/Electron binary is ever added to this folder, it's the intended
  input for validating `data/matrix.toml` against a real app.
