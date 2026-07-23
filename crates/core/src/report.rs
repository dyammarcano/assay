use crate::{Capability, Matrix, Source, TauriPath};

fn path_label(c: &Capability) -> String {
    match c.tauri_path {
        TauriPath::Native => "native".into(),
        TauriPath::Plugin => format!("plugin: {}", c.plugin.as_deref().unwrap_or("?")),
        TauriPath::CustomRust => "custom Rust command".into(),
        TauriPath::Sidecar => "sidecar process".into(),
        TauriPath::None => "**no viable path**".into(),
        TauriPath::OpenQuestion => "**OPEN QUESTION**".into(),
    }
}

fn section(out: &mut String, title: &str, src: Source, m: &Matrix) {
    out.push_str(&format!("## {title}\n\n"));
    out.push_str("| Capability | Tauri path | Severity | Source |\n");
    out.push_str("|---|---|---|---|\n");
    for c in m.capabilities.iter().filter(|c| c.source == src) {
        out.push_str(&format!(
            "| {} | {} | {:?} | [doc]({}) |\n",
            c.name,
            path_label(c),
            c.severity,
            c.citation_url
        ));
    }
    out.push('\n');
}

pub fn render_report(m: &Matrix) -> String {
    let mut out = String::from("# UWP/Electron \u{2192} Tauri v2 Capability & Gap Matrix\n\n");
    section(&mut out, "UWP / WinRT", Source::Uwp, m);
    section(&mut out, "Electron", Source::Electron, m);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Matrix;

    #[test]
    fn renders_embedded_matrix_markdown() {
        let out = render_report(&Matrix::embedded());
        assert!(out.contains("## UWP"));
        assert!(out.contains("## Electron"));
        assert!(out.contains("OPEN QUESTION"));
        insta::assert_snapshot!(out);
    }
}
