# Chromium/WebView2 Probe Driver — Phase Design

**Date:** 2026-07-23
**Status:** Approved (autonomous — charter decide-and-log)
**Unblocks:** the `webview-qa` live-driver dependency of ADR 0001 visual-tier claims

## Goal

Produce a **real** `EngineBlob` from a live engine, instead of only consuming recorded blobs.
This host has Microsoft Edge + the WebView2 Runtime installed, and Edge's Chromium is the same
engine WebView2 embeds — so a Chromium capture is representative of WebView2 rendering.

## Settled forks (decided under charter authority)

1. **Embed a WebView (`wry`/`tao`) vs drive headless Edge over CDP?** → **Headless Edge + CDP.**
   Rejected embedding. Rationale: no GUI event loop (works headless and in CI), no large native
   dependency tree, and it captures the same Chromium engine WebView2 uses. Embedding `wry`
   would be a heavier, GUI-bound path for an identical rendering result.

2. **HTTP client crate for the CDP target list?** → **Raw `TcpStream` HTTP/1.1 GET.** The only
   call is `GET /json/list` on `127.0.0.1`; a dependency would be disproportionate. WebSocket
   framing does need a crate (`tungstenite`, TLS disabled — `ws://` localhost only).

3. **Engine label honesty.** → Default label **`chromium-edge`**, not `webview2`. It is the same
   engine *family*, but this is Edge, not an embedded WebView2 host. Overridable via `--engine`.
   Rationale: the project's honesty invariants — never label a capture as something it isn't.

4. **What if Edge is absent?** → `capture` returns a clear error; the integration test **skips
   with a printed reason** rather than failing. Rationale: never a false green, never a false
   red on a host that simply lacks the engine.

## Design

```
webview_qa::driver
  trait WebViewDriver { fn capture(&self, url: &str, cfg: &Config) -> Result<EngineBlob, DriverError> }
  struct ChromiumDriver { exe: PathBuf, engine: String }
     - find_edge() -> Option<PathBuf>   (well-known install paths)
     - spawn: msedge --headless=new --disable-gpu --remote-debugging-port=<free>
              --user-data-dir=<temp> <url>
     - poll GET /json/list until a "page" target with webSocketDebuggerUrl appears (timeout)
     - tungstenite connect -> Runtime.evaluate { expression: render_probe(engine,cfg),
                                                returnByValue: true } (timeout)
     - parse result.result.value (a JSON string) -> EngineBlob
     - always kill the child + remove the temp profile dir
```

Every network/process wait is bounded by an explicit timeout so a test can never hang.

## CLI

`webview-qa capture --url <url> [--engine chromium-edge] [--exe <msedge>] [--config f.toml]
[--out blob.json]` — writes an `EngineBlob` JSON, which `webview-qa diff` already consumes.

## Testing

- Unit: `find_edge()` returns `None` or an existing path (never a bogus path); `DriverError`
  displays usefully.
- Integration (**host-gated**): if Edge is found, capture a local `file://` fixture page and
  assert the blob has the engine label, a non-empty `user_agent`, and the configured feature
  keys present. If Edge is absent, print `skipped: no Chromium engine` and pass.
- No network: the fixture is a temp HTML file loaded over `file://`.

## Honest limits (recorded, not hidden)

- Only the **Chromium/WebView2 engine family** can be captured on this host. WKWebView and
  WebKitGTK need macOS/Linux runners; until then any run is single-engine and the report
  already labels it as such.
- A Chromium capture is *representative of* WebView2, not identical to an embedded WebView2
  host (different browser chrome/flags). Stated in the README and the engine label.
