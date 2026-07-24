//! Opt-in: does generated scaffolding actually COMPILE against the real tauri crates?
//!
//! `#[ignore]` by default — it needs network access and pulls the full tauri dependency
//! tree (minutes, hundreds of MB), so it must never sit in the fast default gate. Run it
//! deliberately:
//!
//! ```text
//! cargo test -p core --test scaffold_compiles -- --ignored --nocapture
//! ```
//!
//! This is strictly stronger than the `syn::parse_file` check in `scaffold.rs`: that proves
//! the output is valid *syntax*, this proves the plugin names, `init()` signatures, and the
//! builder chaining actually line up with the published crates.

use core::{analyze, scaffold, Matrix, Profile, Source};
use std::process::Command;

/// Build a throwaway crate around the generated bridge and `cargo check` it.
/// Returns (success, combined output).
fn check_generated(profile: Profile) -> (bool, String) {
    let m = Matrix::embedded();
    let out = scaffold(&analyze(&m, &profile));

    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).expect("src dir");

    let manifest = format!(
        "[package]\nname = \"scaffold-check\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
publish = false\n\n[dependencies]\n{}\n",
        out.cargo_deps.join("\n")
    );
    std::fs::write(dir.path().join("Cargo.toml"), &manifest).expect("write manifest");
    std::fs::write(src.join("lib.rs"), &out.rust).expect("write lib.rs");

    let result = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(dir.path())
        .output()
        .expect("run cargo check");

    let combined = format!(
        "--- Cargo.toml ---\n{manifest}\n--- lib.rs ---\n{}\n--- stderr ---\n{}",
        out.rust,
        String::from_utf8_lossy(&result.stderr)
    );
    (result.status.success(), combined)
}

/// The real test: EVERY capability the matrix claims is a proven plugin drop-in must
/// actually compile. This is what caught `stronghold`/`global-shortcut`/`updater` claiming
/// an `init()` that does not exist.
#[test]
#[ignore = "needs network + pulls the full tauri dependency tree (minutes)"]
fn every_proven_plugin_row_compiles_against_real_tauri() {
    let m = Matrix::embedded();
    let ids: Vec<String> = m
        .capabilities
        .iter()
        // Every row that contributes a dependency: plugin wirings AND crate recipes
        // (the latter must at least resolve at the pinned `crate_version`).
        .filter(|c| c.plugin.is_some() || c.crate_name.is_some())
        .map(|c| c.id.clone())
        .collect();
    assert!(!ids.is_empty(), "matrix has no plugin-backed capabilities");
    eprintln!("checking {} plugin-backed capabilities: {ids:?}", ids.len());

    // One crate containing every plugin wiring at once — a single dependency resolve.
    let profile = Profile {
        source: Source::Uwp, // source is irrelevant; ids are looked up directly
        capabilities: ids,
    };
    let (ok, output) = check_generated(profile);
    assert!(ok, "generated bridge did not compile:\n{output}");
}
