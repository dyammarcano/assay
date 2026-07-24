//! Opt-in: does a generated PWA→Tauri port actually COMPILE against the real tauri crates?
//!
//! `#[ignore]` by default — it needs network access and pulls the full tauri dependency tree
//! (minutes, hundreds of MB), so it must never sit in the fast default gate. Run it
//! deliberately:
//!
//! ```text
//! cargo test -p core --test port_compiles -- --ignored --nocapture
//! ```
//!
//! This exists because the first generated port did **not** build: it was missing `build.rs`,
//! and `tauri-build` then hard-failed a second time on an absent `icons/icon.ico`. Neither
//! fault was visible from the generated source — only a real compile found them. "It looks
//! right" is not the bar; `recipe = "proven"` means compile-verified.

use core::pwa::{detect_pwa, placeholder_icon, port_pwa_to_tauri};
use std::process::Command;

/// The real Instagram manifest shape: a hosted PWA with no executable of its own.
const HOSTED_PWA: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10">
  <Properties><DisplayName>Instagram</DisplayName></Properties>
  <Dependencies>
    <uap10:HostRuntimeDependency Name="Microsoft.MicrosoftEdge.Stable" MinVersion="1.0.0.0" />
  </Dependencies>
  <Applications>
    <Application Id="App" uap10:HostId="PWA" uap10:Parameters="--app-fallback-url=https://www.instagram.com/?__pwa=1 --display-mode=standalone">
      <Extensions>
        <uap:Extension Category="windows.shareTarget">
          <uap:ShareTarget><uap:SupportedFileTypes>
            <uap:FileType>.jpg</uap:FileType>
          </uap:SupportedFileTypes></uap:ShareTarget>
        </uap:Extension>
      </Extensions>
    </Application>
  </Applications>
</Package>"#;

#[test]
#[ignore = "needs network + pulls the full tauri dependency tree (minutes)"]
fn a_generated_pwa_port_compiles_against_real_tauri() {
    let app = detect_pwa(HOSTED_PWA).expect("manifest is a hosted PWA");
    let port = port_pwa_to_tauri(&app);

    // Mirror the layout `assay port` actually emits. This matters: `frontendDist` is
    // `../dist`, and tauri-codegen panics at macro-expansion time if that path is missing —
    // so a flattened test tree fails for a reason the real output would not.
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("src-tauri");
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::create_dir_all(root.join("icons")).expect("icons dir");
    std::fs::create_dir_all(dir.path().join("dist")).expect("dist dir");

    // `[workspace]` keeps the throwaway crate from being absorbed into an enclosing
    // workspace, which would silently invalidate the check.
    std::fs::write(
        root.join("Cargo.toml"),
        format!("{}\n[workspace]\n", port.cargo_toml),
    )
    .expect("write Cargo.toml");
    std::fs::write(root.join("build.rs"), &port.build_rs).expect("write build.rs");
    std::fs::write(root.join("tauri.conf.json"), &port.tauri_conf).expect("write tauri.conf.json");
    std::fs::write(root.join("src").join("main.rs"), &port.main_rs).expect("write main.rs");
    std::fs::write(root.join("icons").join("icon.ico"), placeholder_icon()).expect("write icon");

    let result = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(root)
        .output()
        .expect("run cargo check");

    assert!(
        result.status.success(),
        "generated port failed to compile:\n--- tauri.conf.json ---\n{}\n--- main.rs ---\n{}\n\
--- stderr ---\n{}",
        port.tauri_conf,
        port.main_rs,
        String::from_utf8_lossy(&result.stderr)
    );
}

/// Cheap structural guard so a malformed icon is caught by the fast gate rather than only by
/// the slow opt-in compile.
#[test]
fn the_placeholder_icon_is_a_structurally_valid_ico() {
    let ico = placeholder_icon();
    assert_eq!(&ico[0..4], &[0, 0, 1, 0], "ICONDIR magic: reserved=0, type=1");
    assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 1, "one image");
    assert_eq!(ico[6], 16, "width");
    assert_eq!(ico[7], 16, "height");

    let size = u32::from_le_bytes([ico[14], ico[15], ico[16], ico[17]]) as usize;
    let offset = u32::from_le_bytes([ico[18], ico[19], ico[20], ico[21]]) as usize;
    assert_eq!(offset, 22, "image data follows the single directory entry");
    assert_eq!(
        offset + size,
        ico.len(),
        "declared image size must match the actual bytes"
    );
}
