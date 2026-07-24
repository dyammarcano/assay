use std::io::Write;
use std::process::Command;

#[test]
fn analyze_manual_profile_reports_gaps_and_divergence() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(
        f,
        "source = \"uwp\"\ncapabilities = [\"uwp.toast\", \"uwp.live_tiles\"]\n"
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_assay"))
        .arg("analyze")
        .arg("--profile")
        .arg(f.path())
        .output()
        .expect("run analyze");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Toast notifications"));
    assert!(text.contains("Known-Divergence Report"));
    assert!(text.contains("Live Tiles"));
}
