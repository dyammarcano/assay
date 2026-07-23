# wrap-swap Backlog
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

Deferred ideas and follow-up work — sourced from `docs/discovery/IDEA-BRIEF.md` (§5 runner-up
ideas, §6 open questions) and the plan's deferred hardening. Not in the MVP (Phases 1-3).

## Deferred features (runner-up ideas)
- **WinRT capability shim crate** — a reusable `windows-rs`-backed Rust crate exposing
  toast/tile/background-task/PasswordVault-equivalent calls behind a stable API, generalizing
  the `tauri-winrt-notification`/`winrt-toast` pattern. (Brief §5.2)
- **Sidecar-based native-module migration kit** — tooling to detect Electron native-module
  usage and scaffold a sidecar-process replacement (stdout-port-handoff pattern) instead of
  hand-porting each module. (Brief §5.3)
- **Cross-WebView QA harness** — run the same UI across WebView2/WKWebView/WebKitGTK to catch
  rendering/JS divergence before it undermines seamless mimicry. (Brief §5.4)

## Open research questions (resolve before committing an approach)
- Confirm the App Services / package-identity RPC model with a docs.microsoft.com citation and
  whether any Tauri analog is architecturally possible for non-MSIX apps. (Brief §6)
- Confirm current Electron native-module compatibility posture (fresh electronjs.org citation)
  to size the native-module migration problem. (Brief §6)
- Confirm whether any official/community Tauri plugin covers PowerMonitor-equivalent power
  events. (Brief §6)
- Decide the mimicry bar depth per capability: behavior-only vs. interaction/visual parity —
  the WebView-divergence finding bounds how far "exactly like the original" can go. (Brief §6)

## Deferred hardening (from the plan self-review)
- Full temp-crate `cargo check` of generated scaffolding (Tasks 9-10 assert syntactically only).
- Run `/unravel:*` RE workers against a concrete pilot binary to validate the matrix against a
  real app (none was supplied at discovery time).
