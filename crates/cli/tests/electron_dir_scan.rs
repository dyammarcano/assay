//! `--electron-main` must accept a DIRECTORY, not just one file.
//!
//! Real Electron main processes span several modules. Scanning only the entry file silently
//! under-reports capabilities, which is worse than failing — the user gets a confident,
//! incomplete answer. These tests pin the directory behaviour and the single-file warning.

use std::io::Write;
use std::process::Command;

fn pkg_json() -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
    write!(f, r#"{{"dependencies":{{"lodash":"4"}}}}"#).unwrap();
    f
}

#[test]
fn directory_scan_finds_capabilities_spread_across_files() {
    let dir = tempfile::tempdir().unwrap();
    // Entry file: only the tray.
    std::fs::write(
        dir.path().join("main.js"),
        "const { Tray } = require('electron'); new Tray('i.png');",
    )
    .unwrap();
    // A sibling module the entry file never mentions.
    std::fs::write(
        dir.path().join("shortcuts.js"),
        "const { globalShortcut } = require('electron');",
    )
    .unwrap();
    // A nested module, to prove recursion.
    std::fs::create_dir_all(dir.path().join("ipc")).unwrap();
    std::fs::write(
        dir.path().join("ipc").join("handlers.ts"),
        "import { ipcMain } from 'electron';",
    )
    .unwrap();
    // Dependency source must be ignored, or every app looks identical.
    std::fs::create_dir_all(dir.path().join("node_modules").join("evil")).unwrap();
    std::fs::write(
        dir.path().join("node_modules").join("evil").join("x.js"),
        "const { powerMonitor } = require('electron');",
    )
    .unwrap();

    let pkg = pkg_json();
    let out = Command::new(env!("CARGO_BIN_EXE_assay"))
        .arg("analyze")
        .arg("--electron-pkg")
        .arg(pkg.path())
        .arg("--electron-main")
        .arg(dir.path())
        .output()
        .expect("run analyze");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);

    assert!(
        text.contains("Tray icon + menu"),
        "missed entry file:\n{text}"
    );
    assert!(
        text.contains("Global shortcuts"),
        "missed a sibling module — directory scan is not working:\n{text}"
    );
    assert!(
        text.contains("Main/renderer IPC"),
        "missed a nested module — recursion is not working:\n{text}"
    );
    assert!(
        !text.contains("PowerMonitor"),
        "picked up node_modules — dependency code must be ignored:\n{text}"
    );
}

#[test]
fn single_file_run_warns_that_it_is_partial() {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main.js");
    std::fs::write(&main, "const { Tray } = require('electron');").unwrap();

    let pkg = pkg_json();
    let out = Command::new(env!("CARGO_BIN_EXE_assay"))
        .arg("analyze")
        .arg("--electron-pkg")
        .arg(pkg.path())
        .arg("--electron-main")
        .arg(&main)
        .output()
        .expect("run analyze");
    assert!(out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("scanned 1 file") && err.contains("DIRECTORY"),
        "a single-file scan must warn that it is probably partial:\n{err}"
    );
}

#[test]
fn version_flag_works() {
    let out = Command::new(env!("CARGO_BIN_EXE_assay"))
        .arg("--version")
        .output()
        .expect("run --version");
    assert!(out.status.success(), "--version must exit 0");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains(env!("CARGO_PKG_VERSION")),
        "--version must print the crate version, got: {text}"
    );
}
