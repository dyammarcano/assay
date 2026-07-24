//! Detect MSIX packages that are really **hosted PWAs**, and port them to Tauri.
//!
//! A surprising share of "Store apps" ship no executable at all. They declare
//! `uap10:HostId="PWA"` plus a `HostRuntimeDependency` on Microsoft Edge, and the package
//! contains only icons and a manifest — the app *is* a URL rendered by Edge with no browser
//! chrome. Instagram's Windows package is exactly this shape.
//!
//! That distinction decides the entire porting strategy, so getting it wrong is expensive:
//! treating such a package as a normal UWP app produces an empty capability list and implies
//! there is nothing to port, when in fact this is the one case that ports almost perfectly.
//! Edge renders with Chromium; Tauri on Windows renders in WebView2, which is also Chromium.
//! Same engine, same URL.
//!
//! What does NOT carry over is recorded honestly in the generated `MIGRATION.md` rather than
//! papered over — the Share Target contract in particular has no Tauri equivalent.

/// A hosted-PWA package: the facts needed to reproduce it as a Tauri app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PwaApp {
    pub name: String,
    /// The URL the shell actually loads.
    pub start_url: String,
    /// e.g. `standalone` — how the window is presented.
    pub display_mode: Option<String>,
    /// The runtime that hosts it (Edge, in every case observed so far).
    pub host_runtime: Option<String>,
    /// File extensions the app accepts via the Share Target contract, if declared.
    pub share_target_types: Vec<String>,
}

/// Detect a hosted PWA from an `AppxManifest.xml`.
///
/// Returns `None` for a conventional UWP package — this is a narrow, evidence-driven check, not
/// a guess: it requires an explicit PWA host id or an Edge host-runtime dependency, *and* a
/// recoverable start URL. Without a URL there is nothing to port, and claiming otherwise would
/// be fabrication.
pub fn detect_pwa(xml: &str) -> Option<PwaApp> {
    let doc = roxmltree::Document::parse(xml).ok()?;

    let mut is_pwa_host = false;
    let mut host_runtime = None;
    let mut params = String::new();
    let mut descriptions = Vec::new();
    let mut name = None;
    let mut share_target_types = Vec::new();

    for node in doc.descendants() {
        let tag = node.tag_name().name();

        if tag == "HostRuntimeDependency" {
            host_runtime = node.attribute("Name").map(String::from);
            if host_runtime
                .as_deref()
                .is_some_and(|h| h.contains("MicrosoftEdge"))
            {
                is_pwa_host = true;
            }
        }
        // The HostId attribute is namespaced (uap10); match on the local name so the lookup
        // does not depend on the prefix the packager happened to choose.
        if node
            .attributes()
            .any(|a| a.name() == "HostId" && a.value() == "PWA")
        {
            is_pwa_host = true;
        }
        if let Some(p) = node.attributes().find(|a| a.name() == "Parameters") {
            if params.is_empty() {
                params = p.value().to_string();
            }
        }
        if let Some(d) = node.attribute("Description") {
            descriptions.push(d.to_string());
        }
        if tag == "DisplayName" && name.is_none() {
            name = node.text().map(str::trim).map(String::from);
        }
        if tag == "FileType" {
            if let Some(t) = node.text() {
                share_target_types.push(t.trim().to_string());
            }
        }
    }

    if !is_pwa_host {
        return None;
    }

    let start_url = extract_start_url(&params)
        .or_else(|| descriptions.iter().find_map(|d| extract_start_url(d)))?;

    Some(PwaApp {
        name: name.unwrap_or_else(|| "App".into()),
        start_url,
        display_mode: extract_flag(&params, "--display-mode="),
        host_runtime,
        share_target_types,
    })
}

/// Pull the launch URL out of an Edge parameter blob.
///
/// Two spellings occur: the `--app-fallback-url=` command-line flag, and a `start-url?…;`
/// entry inside the web-app-internals extension description.
fn extract_start_url(blob: &str) -> Option<String> {
    if let Some(u) = extract_flag(blob, "--app-fallback-url=") {
        return Some(u);
    }
    let idx = blob.find("start-url?")? + "start-url?".len();
    let rest = &blob[idx..];
    let end = rest.find(';').unwrap_or(rest.len());
    let url = rest[..end].trim();
    if url.is_empty() {
        None
    } else {
        Some(url.to_string())
    }
}

/// Read a `--flag=value` value, terminated by whitespace.
fn extract_flag(blob: &str, flag: &str) -> Option<String> {
    let idx = blob.find(flag)? + flag.len();
    let rest = &blob[idx..];
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let val = rest[..end].trim();
    if val.is_empty() {
        None
    } else {
        Some(val.to_string())
    }
}

/// A generated Tauri project reproducing a hosted PWA.
#[derive(Debug, Clone)]
pub struct TauriPort {
    pub cargo_toml: String,
    pub main_rs: String,
    /// Tauri will not compile without this: `tauri_build::build()` generates the context that
    /// `tauri::generate_context!()` expands to.
    pub build_rs: String,
    pub tauri_conf: String,
    pub migration_md: String,
    /// What the original does that the port does not. Never empty in practice — it always at
    /// least records that the original was Edge and the port is WebView2.
    pub not_ported: Vec<String>,
}

/// A minimal valid 16x16 Windows `.ico`.
///
/// `tauri-build` hard-fails without `icons/icon.ico` — it needs one to generate the Windows
/// resource file — so a port that omits it does not compile. The original package's artwork
/// belongs to its publisher and is not copied; this is a plain placeholder the developer is
/// told to replace.
pub fn placeholder_icon() -> Vec<u8> {
    const W: u8 = 16;
    const PIXELS: usize = 16 * 16;
    // Rows of the AND mask are padded to a 4-byte boundary: 16 bits -> 2 bytes -> 4 bytes.
    const MASK_LEN: usize = 4 * 16;
    const DIB_LEN: usize = 40 + PIXELS * 4 + MASK_LEN;

    let mut out = Vec::with_capacity(22 + DIB_LEN);
    // ICONDIR: reserved, type=1 (icon), count=1
    out.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    // ICONDIRENTRY
    out.push(W); // width
    out.push(W); // height
    out.push(0); // palette size (0 = truecolor)
    out.push(0); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // color planes
    out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    out.extend_from_slice(&(DIB_LEN as u32).to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes()); // offset to the image data

    // BITMAPINFOHEADER. Height is doubled because the DIB holds the colour data followed by
    // the AND mask.
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&16i32.to_le_bytes());
    out.extend_from_slice(&32i32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&((PIXELS * 4 + MASK_LEN) as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]); // resolution + palette counts

    // Opaque BGRA fill.
    for _ in 0..PIXELS {
        out.extend_from_slice(&[0x81, 0x4E, 0x3D, 0xFF]);
    }
    // Fully-opaque AND mask.
    out.extend_from_slice(&[0u8; MASK_LEN]);
    out
}

/// Lowercase, dot-free identifier fragment for a bundle id / crate name.
fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "app".into()
    } else {
        s
    }
}

/// Generate a Tauri v2 project that loads the PWA's start URL.
pub fn port_pwa_to_tauri(app: &PwaApp) -> TauriPort {
    let slug = slug(&app.name);
    // `decorations: false` would be wrong: `standalone` means "no browser UI", not "no title
    // bar". A standalone PWA still gets an OS window frame.
    let (width, height) = (1000, 800);

    let tauri_conf = format!(
        r#"{{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "{name}",
  "version": "0.1.0",
  "identifier": "com.example.{slug}",
  "build": {{
    "frontendDist": "../dist"
  }},
  "app": {{
    "windows": [
      {{
        "title": "{name}",
        "url": "{url}",
        "width": {width},
        "height": {height},
        "resizable": true
      }}
    ],
    "security": {{
      "csp": null
    }}
  }},
  "bundle": {{
    "active": true,
    "targets": ["msi"],
    "icon": ["icons/icon.ico"]
  }}
}}
"#,
        name = app.name,
        slug = slug,
        url = app.start_url,
    );

    let cargo_toml = format!(
        r#"[package]
name = "{slug}"
version = "0.1.0"
edition = "2021"
publish = false

[build-dependencies]
tauri-build = {{ version = "2", features = [] }}

[dependencies]
tauri = {{ version = "2", features = [] }}
"#
    );

    let main_rs = format!(
        r#"// Tauri port of the "{name}" hosted PWA.
//
// The original is an MSIX package with no executable: it asks Edge to render
// {url} in standalone mode. This loads the same URL in WebView2, which is the
// same Chromium engine Edge uses.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {{
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}}
"#,
        name = app.name,
        url = app.start_url,
    );

    let mut not_ported = Vec::new();
    if !app.share_target_types.is_empty() {
        not_ported.push(format!(
            "Share Target contract ({}). Windows can hand shared files to the original; Tauri \
has no equivalent registration, so this is lost.",
            app.share_target_types.join(", ")
        ));
    }
    not_ported.push(
        "Live tiles and Store-managed updates: the original is a Store package; the port is a \
plain MSI and updates through whatever mechanism you add."
            .into(),
    );
    not_ported.push(
        "Engine version: Edge is evergreen and the WebView2 runtime updates independently, so \
the port tracks a different Chromium build than the machine's Edge at any given moment."
            .into(),
    );

    let migration_md = format!(
        r#"# {name} — Tauri port

Generated by `assay port`. The original package is a **hosted PWA**, not a conventional UWP
app: it ships no executable and declares `HostId="PWA"`{host}, asking Edge to render a URL.

| | |
|---|---|
| Start URL | `{url}` |
| Display mode | `{display}` |
| Host runtime | `{host_plain}` |

## Why this port is close to faithful

Edge renders with Chromium. Tauri on Windows renders in WebView2, which is also Chromium. The
original app *is* the remote site, so pointing a WebView2 window at the same URL reproduces it
rather than reimplementing it.

## What is NOT ported

{not_ported}

## Before you ship this

- The window loads **remote content**. Review Tauri's remote-content security guidance before
  exposing any command to it; this scaffold registers no commands, deliberately.
- `frontendDist` points at `../dist`, which is unused for a remote URL but keeps `tauri build`
  happy — create an empty `dist/` directory.
- Supply your own `icons/icon.ico`; the original's images are in the MSIX package's `Images\`
  folder and are the publisher's assets, so they are not copied for you.
- Verify the site works without Edge-specific PWA integration (install prompts, Store hooks).
"#,
        name = app.name,
        url = app.start_url,
        display = app.display_mode.as_deref().unwrap_or("(not declared)"),
        host = app
            .host_runtime
            .as_deref()
            .map(|h| format!(" with a host-runtime dependency on `{h}`"))
            .unwrap_or_default(),
        host_plain = app.host_runtime.as_deref().unwrap_or("(not declared)"),
        not_ported = not_ported
            .iter()
            .map(|n| format!("- {n}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let build_rs = "fn main() {\n    tauri_build::build()\n}\n".to_string();

    TauriPort {
        cargo_toml,
        main_rs,
        build_rs,
        tauri_conf,
        migration_md,
        not_ported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed from the real Instagram package on Windows.
    const INSTAGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:uap3="http://schemas.microsoft.com/appx/manifest/uap/windows10/3"
  xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10">
  <Properties><DisplayName>Instagram</DisplayName></Properties>
  <Dependencies>
    <uap10:HostRuntimeDependency Name="Microsoft.MicrosoftEdge.Stable" MinVersion="1.0.0.0" />
  </Dependencies>
  <Applications>
    <Application Id="App" uap10:HostId="PWA" uap10:Parameters="--app-id=akpam --app-fallback-url=https://www.instagram.com/?utm_source=pwa_homescreen&amp;__pwa=1 --display-mode=standalone --windows-store-app">
      <Extensions>
        <uap:Extension Category="windows.shareTarget">
          <uap:ShareTarget><uap:SupportedFileTypes>
            <uap:FileType>.mp4</uap:FileType>
            <uap:FileType>.jpg</uap:FileType>
          </uap:SupportedFileTypes></uap:ShareTarget>
        </uap:Extension>
      </Extensions>
    </Application>
  </Applications>
</Package>"#;

    /// A conventional UWP app must NOT be misread as a PWA.
    const PLAIN_UWP: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10">
  <Properties><DisplayName>Real App</DisplayName></Properties>
  <Applications>
    <Application Id="App" Executable="App.exe" EntryPoint="App.App">
      <Extensions>
        <uap:Extension Category="windows.protocol">
          <uap:Protocol Name="myapp" />
        </uap:Extension>
      </Extensions>
    </Application>
  </Applications>
</Package>"#;

    #[test]
    fn detects_the_real_instagram_package_as_a_hosted_pwa() {
        let app = detect_pwa(INSTAGRAM).expect("should detect a PWA");
        assert_eq!(app.name, "Instagram");
        // roxmltree resolves &amp; so the URL comes back usable, not escaped.
        assert_eq!(
            app.start_url,
            "https://www.instagram.com/?utm_source=pwa_homescreen&__pwa=1"
        );
        assert_eq!(app.display_mode.as_deref(), Some("standalone"));
        assert_eq!(app.host_runtime.as_deref(), Some("Microsoft.MicrosoftEdge.Stable"));
        assert_eq!(app.share_target_types, vec![".mp4", ".jpg"]);
    }

    #[test]
    fn a_conventional_uwp_app_is_not_a_pwa() {
        assert!(detect_pwa(PLAIN_UWP).is_none());
    }

    #[test]
    fn a_pwa_host_without_a_recoverable_url_is_not_claimed() {
        // No URL anywhere: there is nothing to port, so detection must decline rather than
        // invent a start page.
        let xml = INSTAGRAM
            .replace(
                "--app-fallback-url=https://www.instagram.com/?utm_source=pwa_homescreen&amp;__pwa=1 ",
                "",
            );
        assert!(detect_pwa(&xml).is_none());
    }

    #[test]
    fn start_url_falls_back_to_the_web_app_internals_description() {
        let blob = "parameters?--app-id=x;profile-directory?Default;start-url?https://example.com/app;handlers?share_target";
        assert_eq!(
            extract_start_url(blob).as_deref(),
            Some("https://example.com/app")
        );
    }

    #[test]
    fn generated_config_carries_the_real_url_and_names() {
        let app = detect_pwa(INSTAGRAM).expect("detect");
        let port = port_pwa_to_tauri(&app);
        assert!(port.tauri_conf.contains("https://www.instagram.com/"));
        assert!(port.tauri_conf.contains("\"productName\": \"Instagram\""));
        assert!(port.tauri_conf.contains("\"identifier\": \"com.example.instagram\""));
        assert!(port.cargo_toml.contains("name = \"instagram\""));
        assert!(port.cargo_toml.contains("publish = false"));
        assert!(port.main_rs.contains("tauri::Builder::default()"));
    }

    #[test]
    fn the_share_target_loss_is_stated_not_hidden() {
        let app = detect_pwa(INSTAGRAM).expect("detect");
        let port = port_pwa_to_tauri(&app);
        let shares: Vec<_> = port
            .not_ported
            .iter()
            .filter(|n| n.contains("Share Target"))
            .collect();
        assert_eq!(shares.len(), 1, "share target loss must be reported exactly once");
        assert!(port.migration_md.contains("Share Target"));
        assert!(port.migration_md.contains("What is NOT ported"));
    }

    #[test]
    fn a_build_script_is_emitted_because_tauri_cannot_compile_without_one() {
        let app = detect_pwa(INSTAGRAM).expect("detect");
        let port = port_pwa_to_tauri(&app);
        assert!(port.build_rs.contains("tauri_build::build()"));
        assert!(port.cargo_toml.contains("tauri-build"));
    }

    #[test]
    fn slugs_are_safe_for_crate_and_bundle_ids() {
        assert_eq!(slug("Instagram"), "instagram");
        assert_eq!(slug("My App 2.0"), "my-app-2-0");
        assert_eq!(slug("!!!"), "app");
    }
}
