use crate::{Profile, Source};

pub fn parse_appx_manifest(xml: &str) -> Profile {
    let mut caps = Vec::new();
    if let Ok(doc) = roxmltree::Document::parse(xml) {
        for node in doc.descendants() {
            let name = node.tag_name().name();
            let category = node.attribute("Category").unwrap_or("");
            if name == "Protocol" {
                push(&mut caps, "uwp.protocol_activation");
            }
            if category == "windows.shareTarget" {
                push(&mut caps, "uwp.share_target");
            }
            if category == "windows.backgroundTasks" {
                push(&mut caps, "uwp.background_tasks");
            }
        }
    }
    Profile {
        source: Source::Uwp,
        capabilities: caps,
    }
}

pub fn parse_electron(package_json: &str, main_source: &str) -> Profile {
    let mut caps = Vec::new();
    let map = [
        ("Tray", "electron.tray"),
        ("globalShortcut", "electron.global_shortcut"),
        ("autoUpdater", "electron.auto_update"),
        ("ipcMain", "electron.ipc"),
        ("ipcRenderer", "electron.ipc"),
        ("powerMonitor", "electron.power_monitor"),
        ("setAsDefaultProtocolClient", "electron.deep_link"),
    ];
    for (needle, id) in map {
        if main_source.contains(needle) {
            push(&mut caps, id);
        }
    }
    // Reuse the canonical detector so `analyze` and `sidecar` never disagree about
    // whether a project has native modules.
    if !detect_native_modules(package_json).is_empty() {
        push(&mut caps, "electron.native_module");
    }
    Profile {
        source: Source::Electron,
        capabilities: caps,
    }
}

fn push(v: &mut Vec<String>, id: &str) {
    let s = id.to_string();
    if !v.contains(&s) {
        v.push(s);
    }
}

/// A native Node module detected in an Electron project's dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeModule {
    pub name: String,
    pub detection_reason: String,
    pub has_prebuilds: bool,
}

/// Detect native (N-API/node-gyp) modules from an Electron `package.json`.
///
/// Detection is name-based (no `node_modules` scan): a dependency is flagged if it is a
/// known native-addon toolchain/marker package, or a commonly-native package. `has_prebuilds`
/// is true when the project pulls a prebuild fetcher (`prebuild-install` / `node-gyp-build`).
pub fn detect_native_modules(package_json: &str) -> Vec<NativeModule> {
    // Toolchain/marker packages — presence signals native addons in the tree.
    let markers = [
        "node-gyp",
        "node-addon-api",
        "node-gyp-build",
        "bindings",
        "ffi-napi",
        "napi",
        "nan",
    ];
    // Commonly-native packages worth calling out by name.
    let known_native = [
        "sqlite3",
        "better-sqlite3",
        "serialport",
        "sharp",
        "bcrypt",
        "canvas",
        "robotjs",
        "keytar",
        "node-pty",
        "usb",
        "s7zip-bin",
    ];

    let mut out: Vec<NativeModule> = Vec::new();
    let json: serde_json::Value = match serde_json::from_str(package_json) {
        Ok(v) => v,
        Err(_) => return out,
    };
    let mut names: Vec<String> = Vec::new();
    for section in ["dependencies", "optionalDependencies"] {
        if let Some(obj) = json.get(section).and_then(|d| d.as_object()) {
            names.extend(obj.keys().cloned());
        }
    }
    let has_prebuilds = names
        .iter()
        .any(|n| n == "prebuild-install" || n == "node-gyp-build");

    for name in &names {
        let reason = if markers.contains(&name.as_str()) {
            Some(format!("native-addon toolchain/marker `{name}`"))
        } else if known_native.contains(&name.as_str()) {
            Some(format!("commonly-native package `{name}`"))
        } else {
            None
        };
        if let Some(detection_reason) = reason {
            if !out.iter().any(|m| &m.name == name) {
                out.push(NativeModule {
                    name: name.clone(),
                    detection_reason,
                    has_prebuilds,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;

    const APPX: &str = r#"<?xml version="1.0"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10">
  <Applications><Application>
    <Extensions>
      <uap:Extension Category="windows.protocol"><uap:Protocol Name="myapp"/></uap:Extension>
      <Extension Category="windows.shareTarget"/>
    </Extensions>
  </Application></Applications>
</Package>"#;

    #[test]
    fn appx_maps_protocol_and_share_target() {
        let p = parse_appx_manifest(APPX);
        assert_eq!(p.source, Source::Uwp);
        assert!(p
            .capabilities
            .contains(&"uwp.protocol_activation".to_string()));
        assert!(p.capabilities.contains(&"uwp.share_target".to_string()));
    }

    #[test]
    fn electron_maps_apis_from_main_source() {
        let pkg = r#"{"dependencies":{"foo":"1"}}"#;
        let main = "const { Tray, globalShortcut, ipcMain } = require('electron')";
        let p = parse_electron(pkg, main);
        assert_eq!(p.source, Source::Electron);
        assert!(p.capabilities.contains(&"electron.tray".to_string()));
        assert!(p
            .capabilities
            .contains(&"electron.global_shortcut".to_string()));
        assert!(p.capabilities.contains(&"electron.ipc".to_string()));
    }

    #[test]
    fn detects_native_modules_by_marker_and_known_name() {
        let pkg = r#"{
            "dependencies": { "serialport": "^12", "left-pad": "1", "node-gyp-build": "4" },
            "optionalDependencies": { "keytar": "^7" }
        }"#;
        let mods = detect_native_modules(pkg);
        let names: Vec<&str> = mods.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"serialport")); // known-native
        assert!(names.contains(&"keytar")); // known-native, optionalDependencies
        assert!(names.contains(&"node-gyp-build")); // marker
        assert!(!names.contains(&"left-pad")); // pure JS, not flagged
        assert!(mods.iter().all(|m| m.has_prebuilds)); // node-gyp-build present
    }

    #[test]
    fn detects_no_native_modules_in_pure_js_project() {
        let pkg = r#"{"dependencies":{"react":"18","lodash":"4"}}"#;
        assert!(detect_native_modules(pkg).is_empty());
    }

    /// `analyze` and `sidecar` must agree: any package.json that yields native
    /// modules must also flag the `electron.native_module` capability.
    #[test]
    fn native_module_capability_agrees_with_detector() {
        let pkg = r#"{"dependencies":{"serialport":"^12","node-gyp-build":"4"}}"#;
        let p = parse_electron(pkg, "");
        assert!(!detect_native_modules(pkg).is_empty());
        assert!(p
            .capabilities
            .contains(&"electron.native_module".to_string()));

        let pure = r#"{"dependencies":{"react":"18"}}"#;
        let p2 = parse_electron(pure, "");
        assert!(!p2
            .capabilities
            .contains(&"electron.native_module".to_string()));
    }
}
