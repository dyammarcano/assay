# Cross-WebView QA Harness — Design

**Date:** 2026-07-23
**Status:** Spec — ready to implement (BACKLOG §5.4)
**Depends on:** external browser-engine runtimes (WebView2 / WKWebView / WebKitGTK)

## Problem

Tauri renders in the OS-native WebView (WebView2 on Windows, WKWebView on macOS, WebKitGTK on
Linux), unlike Electron's bundled Chromium. The same UI can render or behave differently across
these engines — the "write once, test three times" risk (Brief §5.4). This silently undermines
"respond exactly like the original app" even after functional parity is reached. The harness makes
that divergence visible and testable.

## Scope

- **In:** load a set of URLs/local pages in each available engine, capture per-engine signals
  (screenshot, DOM-serialized snapshot, console errors, computed-style probes for a supplied
  selector list, feature-support checks), then diff engine-to-engine and emit a divergence report.
- **Out:** fixing divergences; pixel-perfect visual regression against a design baseline (a
  follow-on); mobile WebViews.

## Architecture

A Rust CLI `webview-qa` orchestrating per-engine **drivers** behind a `WebViewDriver` trait:
`load(url)`, `screenshot()`, `dom_snapshot()`, `eval(js) -> json`, `console_errors()`.

- **Windows/WebView2 driver** — via the `webview2-com` / `wry` stack, or drive Edge in WebView2
  mode through CDP.
- **macOS/WKWebView**, **Linux/WebKitGTK** — via `wry`'s platform webviews with an injected probe
  script, or a headless fallback.
- Drivers are feature-gated per OS; a run uses whichever engines are present and records the rest
  as "not available on this host" (never a silent skip).

## Probe protocol

Inject a probe JS that returns a JSON blob: `{ userAgent, features: {...caniuse-style checks...},
computedStyles: { selector -> {prop: value} }, consoleErrors: [...] }`. The selector + feature
lists come from a `webview-qa.toml` config.

## Diffing & report

Normalize each engine's blob, then diff pairwise:
- feature support present in one engine, absent in another → **HIGH** divergence.
- computed-style mismatch for a probed selector → **MEDIUM**.
- console error in one engine only → **MEDIUM**.
- UA-string differences → **INFO**.

Emit `webview-divergence-report.md` (per-page, per-severity) + the raw per-engine JSON blobs.

## Testing

- Trait-level unit tests with a `MockDriver` returning canned blobs → assert the differ classifies
  known divergences correctly (the differ is the testable core; real engines are integration-only).
- Integration (host-gated): run against a tiny fixture page using whatever engine the host has;
  assert a report is produced and lists the engines actually exercised.

## Honesty invariant

The report always states which engines ran and which were unavailable on the host — a 1-engine run
is labeled as such and never presented as cross-engine confidence.
