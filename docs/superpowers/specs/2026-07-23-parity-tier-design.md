# Parity Tier — Phase Design

**Date:** 2026-07-23
**Status:** Approved (autonomous — charter decide-and-log)
**Implements:** `docs/adr/0001-mimicry-bar.md` Option B (phased)

## Goal

Record, per capability, whether assay promises only that it **works** (`behavioral`) or
also that it **looks/feels the same** (`visual`) — and make an unmeasured `visual` claim
*visible* rather than silently implied. Behavioral parity remains the enforced bar today;
visual-tier measurement switches on when the `webview-qa` live drivers land.

## Settled forks (decided under charter authority)

1. **Where does the tier live?** → **A new optional `parity_tier` field on the matrix row.**
   Rejected: a separate config file. Rationale: `data/matrix.toml` is already the single
   source of truth and carries the citation discipline; a second file would drift.

2. **Which capabilities are `visual`?** → **Those that surface OS-rendered UI whose appearance
   a user would notice**: `uwp.toast`, `uwp.file_picker`, `electron.tray`, `electron.menu`,
   `electron.dialog`, `electron.notifications`. Everything else stays `behavioral` (the
   default). Rationale: for a clipboard write or a deep-link activation there is no
   "appearance" to match; for a toast or a native menu there is, and that is exactly where
   "responds exactly like the original" is felt.

3. **How is an unmeasured visual claim surfaced?** → **`analyze` emits a divergence entry per
   `visual`-tier gap** stating visual parity is *not measured* and naming the dependency
   (webview-qa live drivers). Rejected: a silent field with no reporting. Rationale: the
   project's honesty invariant — a claim that isn't measured must never read as passed.

## Changes

- `core::matrix`: `enum ParityTier { Behavioral, Visual }` (serde snake_case);
  `Capability.parity_tier: Option<ParityTier>`, absent = `Behavioral`. Add
  `Capability::parity_tier()` returning the resolved (non-optional) tier.
- `data/matrix.toml`: set `parity_tier = "visual"` on the six capabilities above.
- `core::analyze`: `GapItem` carries `parity_tier`; for each `visual`-tier gap push a
  `DivergenceItem` — "visual parity NOT measured (needs webview-qa live engine drivers)".
- `core::report`: add a Tier column so the dataset's intent is visible in the public doc.
- Snapshots regenerate (report + bridge unaffected in content, report gains a column).

## Non-goals

- No attempt to *measure* visual parity (blocked on live drivers).
- No change to the scaffolder: tier does not affect what code is generated.

## Testing

- A `visual`-tier capability in a profile produces both a gap AND an unmeasured-visual
  divergence entry.
- A `behavioral` capability produces no such divergence.
- Rows without `parity_tier` default to `Behavioral`.
- Report renders the Tier column; snapshot updated.

## Reversibility

The field is optional and additive; removing it and its two reporting sites reverts the
decision without touching the scaffolder or any citation.
