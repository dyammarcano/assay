# IDEA BRIEF — UWP/Electron → Tauri Seamless-Mimicry Parity Project
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

## 0. Scope note
No source binary or codebase was supplied (`idea.txt` is the only file in the target
folder). Per the questionnaire, this is a DEEP but **public-docs-only** research pass —
no reverse-engineering workers were run (nothing to decompile) and no dynamic network
capture was performed. Every capability row below cites a public source; anything not
independently confirmed in this pass is marked **OPEN QUESTION** rather than guessed,
per the seed idea's explicit instruction.

## 1. The idea

**Thesis:** Apps built as UWP (WinRT/XAML/WinUI) or Electron carry a large surface of
OS-integration features (tiles, background tasks, native menus, IPC, protocol handlers,
secure credential storage, etc.) that a naive "just port the UI to Tauri" effort silently
drops. The goal is two-layered:

1. **DOC FIRST** — produce an evidence-backed **capability/gap matrix** enumerating what
   UWP apps and Electron apps commonly do at the OS-integration level, and for each
   capability, state Tauri v2's answer: native/plugin support, custom-Rust-command path,
   sidecar-required, or **no viable path**.
2. **THEN** — use that matrix to scope a **Rust-based "seamless mimicry" converter/toolkit**
   that helps a migrated Tauri app close those gaps (config/asset extraction, IPC shims,
   WinRT bridging via `windows-rs`, sidecar scaffolding) so the ported app behaves as close
   to indistinguishable from the original as the platform allows.

"Seamless" is the aspirational target, not a guarantee — the research surfaces multiple gaps
with **no direct Tauri equivalent** (see §4), which bound how seamless a port can honestly be.

## 2. UWP (WinRT) → Tauri v2 capability/gap matrix

| UWP/WinRT capability | What it does | Tauri v2 path | Gap severity | Source |
|---|---|---|---|---|
| Live Tiles | Dynamic Start-menu tile content via `Windows.UI.StartScreen` + `TileUpdateManager` | No Tauri/plugin equivalent; Start-menu tiles are a Windows-shell-only concept tied to package identity | **No viable path** (concept doesn't exist outside MSIX-registered apps) | [MS Learn: Update a live tile from a background task](https://learn.microsoft.com/en-us/windows/uwp/launch-resume/update-a-live-tile-from-a-background-task) |
| Toast notifications | OS notification-center toasts via `Windows.UI.Notifications.ToastNotificationManager` | `tauri-plugin-notification` (official) for basic toasts; `tauri-winrt-notification`/`winrt-toast` Rust crates for full WinRT toast fidelity (actions, images, sound) via custom command | Partial — basic parity via plugin, rich parity needs custom Rust + windows-rs | [MS Learn PWA/Toast blog](https://techcommunity.microsoft.com/blog/modernworkappconsult/progressive-web-apps-on-windows-10-live-tiles-toast-notifications-and-action-cen/317092); [tauri-winrt-notification docs.rs](https://docs.rs/tauri-winrt-notification); [winrt-toast lib.rs](https://lib.rs/crates/winrt-toast) |
| Background tasks | `Windows.ApplicationModel.Background.BackgroundTaskBuilder`, requires package identity | No Tauri equivalent; would need native `windows-rs` background-task registration bound to an MSIX-wrapped Tauri build, or a scheduled-task/sidecar substitute | Partial-to-none depending on whether the ported app is MSIX-packaged | [MS Learn: create-and-register-a-background-task](https://learn.microsoft.com/en-us/windows/apps/develop/launch/create-and-register-a-background-task) |
| Share Target contract | Declares app as a share destination for other apps' content | No Tauri equivalent found; would require custom Windows shell registration + a Rust command to receive the invocation | **No viable official path** — custom shell integration required | [MS Learn: receive-data](https://learn.microsoft.com/en-us/windows/uwp/app-to-app/receive-data) |
| Protocol activation (custom URI scheme) | Manifest-declared protocol triggers app activation | `tauri-plugin-deep-link` (official) — direct equivalent | Full parity available | [Tauri plugin directory](https://v2.tauri.app/plugin/); [MS Learn: protocol registration](https://learn.microsoft.com/en-us/archive/msdn-magazine/2017/september/modern-apps-protocol-registration-and-activation-in-uwp-apps) |
| Secure credential storage | `Windows.Security.Credentials.PasswordVault`, sandboxed per-AppContainer identity, stronger than raw DPAPI | `tauri-plugin-stronghold` (official, cross-platform vault) is the closest analog; not an exact PasswordVault clone since it isn't OS-credential-locker-backed | Partial — functional parity, not identical storage backend/ACL model | [MS Learn PasswordVault](https://learn.microsoft.com/en-us/uwp/api/windows.security.credentials.passwordvault?view=winrt-26100); [MS Q&A secure storage](https://learn.microsoft.com/en-us/answers/questions/2043534/secure-storage-of-data-in-a-winui-application) |
| App services (inter-app RPC) / MSIX package identity | Lets packaged apps expose RPC-like services to other apps | **OPEN QUESTION** — not independently re-confirmed with a fresh citation this pass; needs a follow-up `AppServiceConnection` doc citation before scoping | Unknown | Flagged, no citation yet |

## 3. Electron → Tauri v2 capability/gap matrix

| Electron capability | What it does | Tauri v2 path | Gap severity | Source |
|---|---|---|---|---|
| Main/renderer + IPC (`ipcMain`/`ipcRenderer`) | Sanctioned bridge between Node-privileged main process and sandboxed renderer | Tauri's `invoke`/command system + events is the direct architectural analog (Rust core instead of Node main) | Full architectural parity, different language/runtime | [Electron IPC docs](https://github.com/electron/electron/blob/main/docs/tutorial/ipc.md) |
| Native Node modules (N-API/node-gyp) | In-process native code with full Node/OS access | No 1:1 equivalent — Tauri deliberately keeps JS sandboxed from Rust; native logic must become a Rust command or external sidecar | Partial — requires re-implementation, not drop-in | [Evil Martians: Tauri sidecar pattern](https://evilmartians.com/chronicles/making-desktop-apps-with-revved-up-potential-rust-tauri-sidecar) |
| Tray icon + menu | `Electron.Tray`, native context menu | Tauri core tray API (built-in since v2) — direct equivalent | Full parity | [Tauri architecture](https://v2.tauri.app/concept/architecture/) |
| Global shortcuts | `Electron.GlobalShortcut` | `tauri-plugin-global-shortcut` (official) — direct equivalent | Full parity | [Tauri plugin directory](https://v2.tauri.app/plugin/) |
| Auto-update | `Electron.AutoUpdater` | `tauri-plugin-updater` (official) — direct equivalent, different update-bundle format | Full parity, migration effort in packaging | [Tauri plugin directory](https://v2.tauri.app/plugin/) |
| PowerMonitor (sleep/wake/battery events) | System power-state events | **OPEN QUESTION** — no official Tauri plugin confirmed in this pass; likely requires a custom `windows-rs`/platform-API Rust command | Unknown, tentatively "custom Rust command" | Not found in plugin list this pass |
| Deep links / custom protocol | `open-url` (macOS) / argv parsing (Win/Linux) via IPC | `tauri-plugin-deep-link` (official) — direct equivalent | Full parity | [Tauri plugin directory](https://v2.tauri.app/plugin/) |
| Native dialogs / clipboard / autostart / single-instance | Various Electron built-ins | `tauri-plugin-dialog`, `tauri-plugin-clipboard-manager`, `tauri-plugin-autostart`, single-instance support — all official | Full parity | [plugins-workspace repo](https://github.com/tauri-apps/plugins-workspace) |
| WebView rendering fidelity | Electron ships its own bundled Chromium | Tauri uses the OS-native WebView (WebView2/WKWebView/WebKitGTK) — CSS/JS behavior can diverge, esp. newer CSS features | Structural gap, requires cross-webview QA, not a feature gap | [tech-insider.org 2026 comparison](https://tech-insider.org/tauri-vs-electron-2026/) |
| Mobile targets (iOS/Android) | Electron has no first-party mobile story either | Tauri v2 (2025) added iOS/Android from the same Rust core | Tauri advantage, not a gap | [tech-insider.org 2026 comparison](https://tech-insider.org/tauri-vs-electron-2026/) |

## 4. Hardest / no-viable-path gaps (bound the "seamless" claim)

1. **Live Tiles** — a Start-menu-shell concept with no analog outside MSIX-registered
   Windows apps; cannot be replicated in a portable Tauri build.
2. **Share Target contract** — needs custom OS shell registration Tauri doesn't broker;
   would require bespoke Windows integration work per-OS.
3. **Native Node modules with deep OS/hardware access** — architecturally excluded by
   Tauri's Rust/JS isolation; each must be re-authored as a Rust command or sidecar,
   not just relinked.
4. **WebView engine divergence** — not a missing feature but a silent behavior/rendering
   risk across WebView2/WKWebView/WebKitGTK that undermines "respond exactly like the
   original app" even after functional parity is reached.
5. **PasswordVault / App services / PowerMonitor** — flagged OPEN QUESTION; need dedicated
   follow-up research passes with fresh citations before the converter design can commit
   to an approach.

## 5. Ranked idea candidates for the Rust converter tool

1. **[TOP] "Parity Matrix + Rust Bridge Scaffolder"** — Ship the capability matrix above as
   a living, machine-readable dataset (per-capability: source app type, Tauri path,
   severity, citation), then generate Rust command/plugin scaffolding (windows-rs calls,
   sidecar stubs, Tauri plugin wiring) for each gap a target app is detected to use.
   Rationale: directly satisfies idea.txt's explicit ask ("document all features... missing
   on tauri, based on public documents"), and the scaffolder is the natural next step once
   the doc exists — no prior-art tool like this was found (§ Prior art), so it's a
   greenfield opportunity, not a reinvention. Cited: entire §2–§4.

2. **WinRT capability shim crate (windows-rs-backed)** — A reusable Rust crate exposing
   toast/tile/background-task/PasswordVault-equivalent calls behind a stable API, building
   directly on the precedent set by `tauri-winrt-notification`/`winrt-toast`. Rationale:
   there's already a working pattern (WinRT-via-Rust-command) to generalize. Cited: windows-rs
   and tauri-winrt-notification sources in §2.

3. **Sidecar-based native-module migration kit** — Tooling to identify Electron
   native-module usage and stub out a sidecar-process replacement (using the documented
   stdout-port-handoff pattern) rather than hand-porting each native module. Rationale:
   the 2-3 month migration cost cited for native-module-heavy Electron apps is the single
   biggest documented pain point; automating scaffolding directly attacks it. Cited: Evil
   Martians sidecar article, tech-insider.org migration-timeline finding.

4. **Cross-WebView QA harness** — Not a converter per se, but a Rust/Tauri-side test
   harness that runs the same UI across WebView2/WKWebView/WebKitGTK to catch the "write
   once, test three times" divergence before it undermines the seamless-mimicry goal.
   Rationale: addresses a real, cited structural risk that pure feature-parity work would
   miss. Cited: tech-insider.org 2026 comparison.

## 6. Open questions for brainstorming to resolve

- Confirm (with a docs.microsoft.com citation) the App Services / package-identity RPC
  model and whether any Tauri-side analog is even architecturally possible for
  non-MSIX-packaged apps.
- Confirm current Electron Node-API/native-module compatibility posture (needs a fresh
  electronjs.org citation) to size the "native module migration" problem accurately.
- Confirm whether any official or well-known community Tauri plugin covers PowerMonitor-
  equivalent system power events — none was found in this pass.
- Decide how deep "seamless mimicry" should go: is matching *behavior* (feature working)
  the bar, or matching *bit-for-bit UI/interaction* (which the WebView-divergence finding
  suggests may not be fully achievable)?
- Since no target binary/app was named, decide whether the converter tool should be
  generic (framework-level, as scoped here) or eventually needs a concrete pilot app to
  validate against.

## 7. Evidence

All findings trace to: `C:\Users\dyamm\AppData\Local\Temp\claude\D--new-page-wrap-swap\6b2c3c7c-60de-4fdd-b0f0-b3c31d1f8047\scratchpad\web-research-uwp-electron-tauri.md`
(raw web-research citations, one researcher pass covering UWP/WinRT, Electron, Tauri v2
plugins, windows-rs, and prior-art/migration-guide dimensions per the questionnaire).
