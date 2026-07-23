# ADR 0001 — How deep does "responds exactly like the original app" go?

- **Status:** Proposed — awaiting operator decision
- **Date:** 2026-07-23
- **Context source:** `docs/discovery/IDEA-BRIEF.md` §6 (last open research question), `idea.txt`

## Context

`idea.txt` asks for a Tauri port that responds **"exactly like the original app."** The
discovery research found that literal claim is not achievable in general:

- **No-viable-path capabilities** — Live Tiles and the Share Target contract are Windows
  shell/MSIX concepts with no Tauri analog at all.
- **Architectural exclusions** — native Node modules can't be relinked; they must be
  re-authored (Rust command) or isolated (sidecar).
- **Engine divergence** — Tauri renders in the OS-native WebView (WebView2/WKWebView/
  WebKitGTK) rather than Electron's bundled Chromium, so identical markup can render or
  behave differently. This is a *silent* difference, not a missing feature.

The toolkit currently implements **behavioral parity + an explicit divergence report** (the
brainstorm decision). This ADR asks whether that stays the permanent bar, per capability.

## Options

### A. Behavioral parity only (current implementation)
A capability "passes" when it *works* — a toast fires, a deep link activates, credentials
persist — regardless of backend or pixel-level differences. Everything unachievable is named
in the divergence report.

- ➕ Honest, achievable, already built and tested.
- ➕ The divergence report makes every shortfall explicit rather than hidden.
- ➖ Does not answer "does it *look and feel* the same," which is part of the original ask.

### B. Behavioral parity + measured visual/interaction parity (tiered)
Keep A as the floor, and add a **measured** second tier for capabilities where look-and-feel
matters (rendering, fonts, spacing, animation timing) using the `webview-qa` harness — with a
per-capability tier recorded in `data/matrix.toml`.

- ➕ Directly addresses the "exactly like" ask where it's meaningful, with evidence.
- ➕ `webview-qa` already exists; this gives it a purpose inside the matrix.
- ➖ Requires the live engine drivers (currently blocked) to be real.
- ➖ Adds a per-row `parity_tier` field and a second reporting axis.

### C. Full interaction/visual parity as the promise
Claim pixel/timing equivalence across the board.

- ➖ Not achievable — contradicted by the WebView-divergence and no-path findings.
- ➖ Would violate the project's own honesty invariants.

## Recommendation

**Option B, phased** — keep **A** as the shipped, enforced bar today, and add an explicit
per-capability `parity_tier` (`behavioral` | `visual`) to the matrix so the *intent* is
recorded now, with the visual tier's measurement switched on when the `webview-qa` live
drivers land. Rationale: it preserves every honesty invariant (nothing is claimed that isn't
measured), it answers the original ask where it's actually meaningful rather than uniformly,
and it converts the currently-blocked harness work into a concrete, motivated deliverable.

Option C is rejected outright: it would require asserting parity the research proves can't be
guaranteed, which the project's citation/no-fabrication invariants forbid.

## Consequences if B is accepted

- `data/matrix.toml` gains an optional `parity_tier` field (default `behavioral`).
- `analyze` reports the tier per gap; the divergence report notes when a `visual`-tier
  capability has not been measured (rather than implying it passed).
- The `webview-qa` live drivers move from "nice to have" to **required** for any `visual`-tier
  claim — an explicit dependency, tracked in `docs/BACKLOG.md`.

## Decision

_Pending._ Record the chosen option and date here, then set Status to Accepted.
