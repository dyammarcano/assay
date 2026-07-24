//! Live engine drivers — the producer side of the harness.
//!
//! [`ChromiumDriver`] drives a headless Microsoft Edge over the Chrome DevTools Protocol
//! and evaluates the probe (see [`crate::probe`]) in the page, returning a real
//! [`EngineBlob`]. Edge's Chromium is the same engine WebView2 embeds, so the capture is
//! representative of WebView2 rendering — but it is labelled `chromium-edge`, not
//! `webview2`, because it is Edge and not an embedded WebView2 host.
//!
//! WKWebView (macOS) and WebKitGTK (Linux) drivers are not implemented; they need their
//! respective OSes. Until they exist, runs are single-engine and the report says so.

use crate::{render_probe, Config, EngineBlob};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Everything that can go wrong producing a capture.
#[derive(Debug)]
pub enum DriverError {
    /// No engine binary found on this host.
    EngineNotFound(String),
    /// The browser process could not be started.
    Spawn(String),
    /// DevTools never became reachable / no page target appeared.
    DevToolsUnavailable(String),
    /// CDP transport or protocol failure.
    Protocol(String),
    /// The probe ran but its result wasn't a usable EngineBlob.
    BadProbeResult(String),
}

impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverError::EngineNotFound(m) => write!(f, "engine not found: {m}"),
            DriverError::Spawn(m) => write!(f, "cannot start engine: {m}"),
            DriverError::DevToolsUnavailable(m) => write!(f, "devtools unavailable: {m}"),
            DriverError::Protocol(m) => write!(f, "devtools protocol error: {m}"),
            DriverError::BadProbeResult(m) => write!(f, "bad probe result: {m}"),
        }
    }
}
impl std::error::Error for DriverError {}

type Result<T> = std::result::Result<T, DriverError>;

/// Captures an [`EngineBlob`] from one live engine.
pub trait WebViewDriver {
    fn capture(&self, url: &str, cfg: &Config) -> Result<EngineBlob>;
}

/// Well-known Edge install locations on Windows.
const EDGE_PATHS: [&str; 2] = [
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
];

/// Locate a Chromium engine binary, or `None` when this host has none.
pub fn find_edge() -> Option<PathBuf> {
    EDGE_PATHS
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
        .or_else(|| {
            // Fall back to anything Chromium-ish on PATH (chrome, chromium).
            for name in ["msedge.exe", "chrome.exe", "chromium.exe"] {
                if let Ok(out) = Command::new("where").arg(name).output() {
                    if out.status.success() {
                        let s = String::from_utf8_lossy(&out.stdout);
                        if let Some(first) = s.lines().next() {
                            let p = PathBuf::from(first.trim());
                            if p.is_file() {
                                return Some(p);
                            }
                        }
                    }
                }
            }
            None
        })
}

/// Drives a headless Chromium (Edge) over CDP.
pub struct ChromiumDriver {
    pub exe: PathBuf,
    pub engine: String,
}

impl ChromiumDriver {
    /// Build a driver, auto-detecting the engine binary.
    pub fn detect() -> Result<ChromiumDriver> {
        let exe = find_edge().ok_or_else(|| {
            DriverError::EngineNotFound("no msedge/chrome/chromium binary on this host".into())
        })?;
        Ok(ChromiumDriver {
            exe,
            engine: "chromium-edge".into(),
        })
    }

    pub fn with_engine_label(mut self, label: impl Into<String>) -> Self {
        self.engine = label.into();
        self
    }
}

/// Ask the OS for a free localhost port (bind then immediately drop).
fn free_port() -> Result<u16> {
    let l = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| DriverError::Spawn(format!("cannot reserve a port: {e}")))?;
    l.local_addr()
        .map(|a| a.port())
        .map_err(|e| DriverError::Spawn(format!("cannot read reserved port: {e}")))
}

/// Minimal HTTP/1.1 GET over a raw socket (localhost DevTools only).
fn http_get(port: u16, path: &str) -> Result<String> {
    let mut s = TcpStream::connect(("127.0.0.1", port))
        .map_err(|e| DriverError::DevToolsUnavailable(format!("connect: {e}")))?;
    s.set_read_timeout(Some(Duration::from_secs(5))).ok();
    // The Host header MUST include the port: Chromium's DevTools HTTP server validates it
    // (DNS-rebinding protection) and simply never replies to a mismatched Host, which
    // presents as a read timeout rather than a refusal.
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    s.write_all(req.as_bytes())
        .map_err(|e| DriverError::DevToolsUnavailable(format!("write: {e}")))?;

    // NOTE: do NOT read to EOF. Chromium's DevTools HTTP server keeps the connection alive
    // regardless of `Connection: close`, so EOF never arrives — a read-to-end would block
    // until the timeout AND discard the bytes it already read. Parse Content-Length instead.
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut header_end: Option<usize> = None;
    let mut content_len: Option<usize> = None;
    loop {
        if header_end.is_none() {
            if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                header_end = Some(pos + 4);
                let headers = String::from_utf8_lossy(&buf[..pos]).to_lowercase();
                content_len = headers.lines().find_map(|l| {
                    l.strip_prefix("content-length:")
                        .and_then(|v| v.trim().parse::<usize>().ok())
                });
            }
        }
        if let (Some(he), Some(cl)) = (header_end, content_len) {
            if buf.len() >= he + cl {
                return Ok(String::from_utf8_lossy(&buf[he..he + cl]).to_string());
            }
        }
        match s.read(&mut chunk) {
            Ok(0) => break, // server closed — use whatever we have
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) => return Err(DriverError::DevToolsUnavailable(format!("read: {e}"))),
        }
    }
    match header_end {
        Some(he) => Ok(String::from_utf8_lossy(&buf[he..]).to_string()),
        None => Err(DriverError::DevToolsUnavailable(
            "malformed response (no header terminator)".into(),
        )),
    }
}

/// First index of `needle` in `hay`.
fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Poll `/json/list` until a page target exposes a WebSocket debugger URL.
fn wait_for_page_ws(port: u16, timeout: Duration) -> Result<String> {
    let deadline = Instant::now() + timeout;
    let mut last = String::from("no attempt made");
    while Instant::now() < deadline {
        match http_get(port, "/json/list") {
            Ok(body) => match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(v) => {
                    if let Some(arr) = v.as_array() {
                        for t in arr {
                            let is_page = t.get("type").and_then(|x| x.as_str()) == Some("page");
                            if let (true, Some(ws)) = (
                                is_page,
                                t.get("webSocketDebuggerUrl").and_then(|x| x.as_str()),
                            ) {
                                return Ok(ws.to_string());
                            }
                        }
                    }
                    last = "no page target yet".into();
                }
                Err(e) => last = format!("bad json: {e}"),
            },
            Err(e) => last = e.to_string(),
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err(DriverError::DevToolsUnavailable(format!(
        "timed out waiting for a page target ({last})"
    )))
}

/// Kill the child and clean the temp profile, best-effort.
struct Cleanup {
    child: Child,
    profile: PathBuf,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

fn temp_profile_dir() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("wvqa-profile-{}", std::process::id()));
    p
}

impl WebViewDriver for ChromiumDriver {
    fn capture(&self, url: &str, cfg: &Config) -> Result<EngineBlob> {
        let port = free_port()?;
        let profile = temp_profile_dir();
        let _ = std::fs::create_dir_all(&profile);

        let child = Command::new(&self.exe)
            .arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg(format!("--remote-debugging-port={port}"))
            // Chromium 111+ rejects DevTools WebSocket upgrades from disallowed origins.
            .arg("--remote-allow-origins=*")
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| DriverError::Spawn(format!("{}: {e}", self.exe.display())))?;
        // Guarantees the process dies and the profile is removed on every exit path.
        let _guard = Cleanup { child, profile };

        let ws_url = wait_for_page_ws(port, Duration::from_secs(20))?;
        let (mut socket, _resp) = tungstenite::connect(&ws_url)
            .map_err(|e| DriverError::Protocol(format!("ws connect: {e}")))?;

        let expression = render_probe(&self.engine, cfg);
        let msg = serde_json::json!({
            "id": 1,
            "method": "Runtime.evaluate",
            "params": { "expression": expression, "returnByValue": true, "awaitPromise": true }
        });
        socket
            .send(tungstenite::Message::Text(msg.to_string()))
            .map_err(|e| DriverError::Protocol(format!("ws send: {e}")))?;

        // Read until our id=1 reply (CDP interleaves events).
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let frame = socket
                .read()
                .map_err(|e| DriverError::Protocol(format!("ws read: {e}")))?;
            let text = match frame {
                tungstenite::Message::Text(t) => t,
                tungstenite::Message::Close(_) => {
                    return Err(DriverError::Protocol("socket closed early".into()))
                }
                _ => continue,
            };
            let v: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if v.get("id").and_then(|x| x.as_u64()) != Some(1) {
                continue;
            }
            if let Some(err) = v.get("error") {
                return Err(DriverError::Protocol(err.to_string()));
            }
            let value = v
                .pointer("/result/result/value")
                .and_then(|x| x.as_str())
                .ok_or_else(|| {
                    DriverError::BadProbeResult(format!("no string result in {text}"))
                })?;
            return serde_json::from_str::<EngineBlob>(value)
                .map_err(|e| DriverError::BadProbeResult(format!("{e}: {value}")));
        }
        Err(DriverError::Protocol(
            "timed out awaiting probe result".into(),
        ))
    }
}

/// Write a tiny fixture page and return its `file://` URL — used by tests and demos.
pub fn write_fixture_page(dir: &Path) -> std::io::Result<String> {
    let file = dir.join("wvqa-fixture.html");
    std::fs::write(
        &file,
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>wvqa</title>\
<style>body{font-family:system-ui;font-size:16px}</style></head>\
<body><p>assay webview-qa fixture</p></body></html>",
    )?;
    Ok(format!(
        "file:///{}",
        file.display().to_string().replace('\\', "/")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_edge_returns_none_or_a_real_file() {
        match find_edge() {
            None => { /* host has no Chromium — valid */ }
            Some(p) => assert!(p.is_file(), "find_edge returned a non-existent path: {p:?}"),
        }
    }

    #[test]
    fn driver_errors_display_usefully() {
        let e = DriverError::EngineNotFound("nothing installed".into());
        assert!(e.to_string().contains("engine not found"));
        let e = DriverError::Protocol("boom".into());
        assert!(e.to_string().contains("devtools protocol error"));
    }

    #[test]
    fn fixture_page_is_written_and_urlified() {
        let dir = std::env::temp_dir();
        let url = write_fixture_page(&dir).expect("write fixture");
        assert!(url.starts_with("file:///"));
        assert!(url.ends_with("wvqa-fixture.html"));
    }
}
