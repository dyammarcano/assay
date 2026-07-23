use std::io::Write;
use std::process::Command;

#[test]
fn scaffold_writes_bridge_and_deps() {
    let dir = tempfile::tempdir().unwrap();
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(
        f,
        "source = \"electron\"\ncapabilities = [\"electron.global_shortcut\"]\n"
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wrap-swap"))
        .arg("scaffold")
        .arg("--profile")
        .arg(f.path())
        .arg("--out-dir")
        .arg(dir.path())
        .output()
        .expect("run scaffold");
    assert!(out.status.success());
    let bridge = std::fs::read_to_string(dir.path().join("bridge.rs")).unwrap();
    assert!(bridge.contains(".plugin("));
    let deps = std::fs::read_to_string(dir.path().join("deps.txt")).unwrap();
    assert!(deps.contains("tauri-plugin-global-shortcut"));
}
