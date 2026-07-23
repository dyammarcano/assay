use std::io::Write;
use std::process::Command;

#[test]
fn sidecar_writes_kit_for_native_modules() {
    let dir = tempfile::tempdir().unwrap();
    let mut pkg = tempfile::NamedTempFile::new().unwrap();
    write!(
        pkg,
        r#"{{"dependencies":{{"serialport":"^12","left-pad":"1"}}}}"#
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wrap-swap"))
        .arg("sidecar")
        .arg("--electron-pkg")
        .arg(pkg.path())
        .arg("--out-dir")
        .arg(dir.path())
        .output()
        .expect("run sidecar");
    assert!(out.status.success());

    let main_rs = std::fs::read_to_string(dir.path().join("sidecar").join("src").join("main.rs"))
        .expect("main.rs written");
    assert!(main_rs.contains("\"serialport\" =>"));
    assert!(!main_rs.contains("left-pad")); // pure-JS dep not flagged

    assert!(dir.path().join("sidecar").join("Cargo.toml").exists());
    assert!(dir.path().join("sidecar_client.rs").exists());
    let migration = std::fs::read_to_string(dir.path().join("MIGRATION.md")).unwrap();
    assert!(migration.contains("| `serialport` |"));
}
