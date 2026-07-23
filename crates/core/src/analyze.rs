use crate::{Matrix, ParityTier, Profile, TauriPath};

pub struct GapItem {
    pub id: String,
    pub name: String,
    pub tauri_path: TauriPath,
    pub plugin: Option<String>,
    pub crate_name: Option<String>,
    /// ADR 0001 — how deep the parity promise goes for this capability.
    pub parity_tier: ParityTier,
}

pub struct DivergenceItem {
    pub id: String,
    pub name: String,
    pub reason: String,
    pub citation_url: String,
}

pub struct Analysis {
    pub gaps: Vec<GapItem>,
    pub divergences: Vec<DivergenceItem>,
    pub unknown: Vec<String>,
}

pub fn analyze(m: &Matrix, p: &Profile) -> Analysis {
    let mut gaps = Vec::new();
    let mut divergences = Vec::new();
    let mut unknown = Vec::new();
    for id in &p.capabilities {
        match m.get(id) {
            None => unknown.push(id.clone()),
            Some(c) => match c.tauri_path {
                TauriPath::None => divergences.push(DivergenceItem {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    reason: "No viable Tauri path.".into(),
                    citation_url: c.citation_url.clone(),
                }),
                TauriPath::OpenQuestion => divergences.push(DivergenceItem {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    reason: "OPEN QUESTION — no confirmed path; needs research.".into(),
                    citation_url: c.citation_url.clone(),
                }),
                _ => gaps.push(GapItem {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    tauri_path: c.tauri_path,
                    plugin: c.plugin.clone(),
                    crate_name: c.crate_name.clone(),
                    parity_tier: c.parity_tier(),
                }),
            },
        }
    }
    // ADR 0001: a `visual`-tier capability promises look/feel parity, which nothing measures
    // yet. Say so explicitly — an unmeasured claim must never read as passed.
    for g in gaps.iter().filter(|g| g.parity_tier == ParityTier::Visual) {
        divergences.push(DivergenceItem {
            id: g.id.clone(),
            name: format!("{} — visual parity NOT measured", g.name),
            reason: "Capability is visual-tier (ADR 0001): behavior is covered, but look/feel \
                     equivalence is unverified — needs the webview-qa live engine drivers."
                .into(),
            citation_url: "https://v2.tauri.app/concept/architecture/".into(),
        });
    }

    divergences.push(DivergenceItem {
        id: "webview.engine".into(),
        name: "WebView engine divergence".into(),
        reason: "Tauri uses OS-native WebView (WebView2/WKWebView/WebKitGTK); rendering/JS behavior can diverge from the original app.".into(),
        citation_url: "https://v2.tauri.app/concept/architecture/".into(),
    });
    Analysis {
        gaps,
        divergences,
        unknown,
    }
}

pub fn render_divergence(a: &Analysis) -> String {
    let mut out = String::from("# Known-Divergence Report\n\n");
    for d in &a.divergences {
        out.push_str(&format!(
            "- **{}** ({}): {} [doc]({})\n",
            d.name, d.id, d.reason, d.citation_url
        ));
    }
    if !a.unknown.is_empty() {
        out.push_str("\n## Unknown capabilities (skipped)\n\n");
        for u in &a.unknown {
            out.push_str(&format!("- {u}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Matrix, Profile, Source, TauriPath};

    fn profile(ids: &[&str]) -> Profile {
        Profile {
            source: Source::Uwp,
            capabilities: ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn classifies_gaps_divergences_and_unknowns() {
        let m = Matrix::embedded();
        let a = analyze(
            &m,
            &profile(&[
                "uwp.toast",
                "uwp.live_tiles",
                "uwp.share_target",
                "nope.bogus",
            ]),
        );
        assert!(a
            .gaps
            .iter()
            .any(|g| g.id == "uwp.toast" && g.tauri_path == TauriPath::CustomRust));
        // no-viable-path capabilities become divergences
        assert!(a.divergences.iter().any(|d| d.id == "uwp.live_tiles"));
        assert!(a.divergences.iter().any(|d| d.id == "uwp.share_target"));
        assert_eq!(a.unknown, vec!["nope.bogus".to_string()]);
    }

    #[test]
    fn visual_tier_gap_gets_an_unmeasured_divergence() {
        use crate::ParityTier;
        let m = Matrix::embedded();
        // uwp.toast is visual-tier; uwp.credential_vault is behavioral.
        let a = analyze(&m, &profile(&["uwp.toast", "uwp.credential_vault"]));

        let toast = a.gaps.iter().find(|g| g.id == "uwp.toast").unwrap();
        assert_eq!(toast.parity_tier, ParityTier::Visual);
        let vault = a
            .gaps
            .iter()
            .find(|g| g.id == "uwp.credential_vault")
            .unwrap();
        assert_eq!(vault.parity_tier, ParityTier::Behavioral);

        // Exactly the visual one is called out as unmeasured.
        let unmeasured: Vec<&str> = a
            .divergences
            .iter()
            .filter(|d| d.name.contains("visual parity NOT measured"))
            .map(|d| d.id.as_str())
            .collect();
        assert_eq!(unmeasured, vec!["uwp.toast"]);
    }
}
