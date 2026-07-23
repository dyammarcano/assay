use std::process::Command;

#[test]
fn report_prints_matrix_to_stdout() {
    let out = Command::new(env!("CARGO_BIN_EXE_wrap-swap"))
        .arg("report")
        .output()
        .expect("run wrap-swap report");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Capability & Gap Matrix"));
    assert!(text.contains("## UWP"));
}
