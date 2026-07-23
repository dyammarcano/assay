use std::io::Write;
use std::process::Command;

fn write_blob(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

#[test]
fn diff_two_blobs_reports_divergence() {
    let a = write_blob(r#"{"engine":"webview2","features":{"dialog":true}}"#);
    let b = write_blob(r#"{"engine":"webkitgtk","features":{"dialog":false}}"#);
    let out = Command::new(env!("CARGO_BIN_EXE_webview-qa"))
        .arg("diff")
        .arg("--blob")
        .arg(a.path())
        .arg("--blob")
        .arg(b.path())
        .output()
        .expect("run webview-qa diff");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Cross-WebView Divergence Report"));
    assert!(text.contains("Engines exercised (2)"));
    assert!(text.contains("HIGH"));
    assert!(text.contains("dialog"));
}
