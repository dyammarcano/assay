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

fn path_key(p: TauriPath) -> &'static str {
    match p {
        TauriPath::Native => "native",
        TauriPath::Plugin => "plugin",
        TauriPath::CustomRust => "custom Rust",
        TauriPath::Sidecar => "sidecar",
        TauriPath::None => "no viable path",
        TauriPath::OpenQuestion => "OPEN QUESTION",
    }
}

fn section(out: &mut String, title: &str, src: Source, m: &Matrix) {
    out.push_str(&format!("## {title}\n\n"));
    out.push_str("| Capability | Tauri path | Severity | Tier | Source |\n");
    out.push_str("|---|---|---|---|---|\n");
    for c in m.capabilities.iter().filter(|c| c.source == src) {
        out.push_str(&format!(
            "| {} | {} | {:?} | {:?} | [doc]({}) |\n",
            c.name,
            path_label(c),
            c.severity,
            c.parity_tier(),
            c.citation_url
        ));
    }
    out.push('\n');
}

/// Roll up how many capabilities fall under each Tauri path, in a stable order.
fn summary(out: &mut String, caps: &[&Capability]) {
    let order = [
        TauriPath::Native,
        TauriPath::Plugin,
        TauriPath::CustomRust,
        TauriPath::Sidecar,
        TauriPath::None,
        TauriPath::OpenQuestion,
    ];
    out.push_str(&format!("**{} capabilities.** Path rollup: ", caps.len()));
    let mut parts = Vec::new();
    for p in order {
        let n = caps.iter().filter(|c| c.tauri_path == p).count();
        if n > 0 {
            parts.push(format!("{} {}", n, path_key(p)));
        }
    }
    out.push_str(&parts.join(", "));
    out.push_str(
        "\n\n_Legend: **no viable path** = cannot be replicated; \
**OPEN QUESTION** = unconfirmed, needs research; \
plugin/native/custom Rust/sidecar = a path exists._\n\n",
    );
}

/// Render the capability/gap matrix as Markdown. `only` restricts to one source.
pub fn render_report(m: &Matrix, only: Option<Source>) -> String {
    let mut out = String::from("# UWP/Electron \u{2192} Tauri v2 Capability & Gap Matrix\n\n");
    let selected: Vec<&Capability> = m
        .capabilities
        .iter()
        .filter(|c| match only {
            Some(s) => c.source == s,
            None => true,
        })
        .collect();
    summary(&mut out, &selected);
    if only != Some(Source::Electron) {
        section(&mut out, "UWP / WinRT", Source::Uwp, m);
    }
    if only != Some(Source::Uwp) {
        section(&mut out, "Electron", Source::Electron, m);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Matrix, Source};

    #[test]
    fn renders_embedded_matrix_markdown() {
        let out = render_report(&Matrix::embedded(), None);
        assert!(out.contains("## UWP"));
        assert!(out.contains("## Electron"));
        assert!(out.contains("no viable path")); // live_tiles / share_target
        assert!(out.contains("Path rollup:"));
        insta::assert_snapshot!(out);
    }

    #[test]
    fn source_filter_restricts_sections() {
        let out = render_report(&Matrix::embedded(), Some(Source::Electron));
        assert!(out.contains("## Electron"));
        assert!(!out.contains("## UWP"));
    }
}
