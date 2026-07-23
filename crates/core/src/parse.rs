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
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(package_json) {
        if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
            let native = ["node-gyp", "bindings", "node-addon-api", "ffi-napi"];
            if deps.keys().any(|k| native.contains(&k.as_str())) {
                push(&mut caps, "electron.native_module");
            }
        }
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
        assert!(p.capabilities.contains(&"uwp.protocol_activation".to_string()));
        assert!(p.capabilities.contains(&"uwp.share_target".to_string()));
    }

    #[test]
    fn electron_maps_apis_from_main_source() {
        let pkg = r#"{"dependencies":{"foo":"1"}}"#;
        let main = "const { Tray, globalShortcut, ipcMain } = require('electron')";
        let p = parse_electron(pkg, main);
        assert_eq!(p.source, Source::Electron);
        assert!(p.capabilities.contains(&"electron.tray".to_string()));
        assert!(p.capabilities.contains(&"electron.global_shortcut".to_string()));
        assert!(p.capabilities.contains(&"electron.ipc".to_string()));
    }
}
