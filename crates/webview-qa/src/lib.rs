//! Cross-WebView QA harness core (BACKLOG §5.4 / spec
//! `docs/superpowers/specs/2026-07-23-cross-webview-qa-harness-design.md`).
//!
//! The testable core: an [`EngineBlob`] captured from one WebView engine
//! (WebView2 / WKWebView / WebKitGTK) and a pairwise [`diff`] that classifies
//! engine-to-engine divergences into a [`Divergence`] list, plus a Markdown
//! [`render_report`]. Real engine drivers (which produce the blobs) are host-gated
//! integration work; this crate operates on recorded blobs so it is useful and
//! testable without a live engine.

pub mod driver;
pub mod probe;
pub use driver::{find_edge, ChromiumDriver, DriverError, WebViewDriver};
pub use probe::{render_probe, Config};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A probe capture from one WebView engine.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EngineBlob {
    pub engine: String,
    #[serde(default)]
    pub user_agent: String,
    /// caniuse-style feature -> supported.
    #[serde(default)]
    pub features: BTreeMap<String, bool>,
    /// selector -> (css property -> computed value).
    #[serde(default)]
    pub computed_styles: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub console_errors: Vec<String>,
}

/// Divergence severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    High,
    Medium,
    Info,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Info => "INFO",
        }
    }
}

/// One classified engine-to-engine difference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    pub severity_rank: u8,
    pub kind: String,
    pub detail: String,
}

fn div(sev: Severity, kind: &str, detail: String) -> Divergence {
    Divergence {
        severity_rank: match sev {
            Severity::High => 0,
            Severity::Medium => 1,
            Severity::Info => 2,
        },
        kind: format!("[{}] {}", sev.label(), kind),
        detail,
    }
}

fn compare_pair(a: &EngineBlob, b: &EngineBlob, out: &mut Vec<Divergence>) {
    // Feature support: present-in-one / differing -> HIGH.
    let feat_keys: BTreeSet<&String> = a.features.keys().chain(b.features.keys()).collect();
    for k in feat_keys {
        let av = a.features.get(k);
        let bv = b.features.get(k);
        if av != bv {
            out.push(div(
                Severity::High,
                "feature",
                format!("`{k}`: {}={:?} vs {}={:?}", a.engine, av, b.engine, bv),
            ));
        }
    }

    // Computed-style mismatch for a probed selector/prop -> MEDIUM.
    let sel_keys: BTreeSet<&String> = a
        .computed_styles
        .keys()
        .chain(b.computed_styles.keys())
        .collect();
    for sel in sel_keys {
        let empty = BTreeMap::new();
        let am = a.computed_styles.get(sel).unwrap_or(&empty);
        let bm = b.computed_styles.get(sel).unwrap_or(&empty);
        let props: BTreeSet<&String> = am.keys().chain(bm.keys()).collect();
        for p in props {
            let av = am.get(p);
            let bv = bm.get(p);
            if av != bv {
                out.push(div(
                    Severity::Medium,
                    "computed-style",
                    format!(
                        "`{sel}` {{{p}}}: {}={:?} vs {}={:?}",
                        a.engine, av, b.engine, bv
                    ),
                ));
            }
        }
    }

    // Console error present in one engine only -> MEDIUM.
    let a_errs: BTreeSet<&String> = a.console_errors.iter().collect();
    let b_errs: BTreeSet<&String> = b.console_errors.iter().collect();
    for e in a_errs.difference(&b_errs) {
        out.push(div(
            Severity::Medium,
            "console-error",
            format!("only on {}: {e}", a.engine),
        ));
    }
    for e in b_errs.difference(&a_errs) {
        out.push(div(
            Severity::Medium,
            "console-error",
            format!("only on {}: {e}", b.engine),
        ));
    }

    // UA differences -> INFO.
    if a.user_agent != b.user_agent {
        out.push(div(
            Severity::Info,
            "user-agent",
            format!("{} vs {}", a.engine, b.engine),
        ));
    }
}

/// Diff every pair of engine blobs, most-severe first.
pub fn diff(blobs: &[EngineBlob]) -> Vec<Divergence> {
    let mut out = Vec::new();
    for i in 0..blobs.len() {
        for j in (i + 1)..blobs.len() {
            compare_pair(&blobs[i], &blobs[j], &mut out);
        }
    }
    out.sort_by(|x, y| {
        x.severity_rank
            .cmp(&y.severity_rank)
            .then_with(|| x.detail.cmp(&y.detail))
    });
    out
}

/// Render the divergence report. Always states which engines were exercised, so a
/// single-engine run is never mistaken for cross-engine confidence.
pub fn render_report(blobs: &[EngineBlob], divergences: &[Divergence]) -> String {
    let mut out = String::from("# Cross-WebView Divergence Report\n\n");
    let engines: Vec<&str> = blobs.iter().map(|b| b.engine.as_str()).collect();
    out.push_str(&format!(
        "**Engines exercised ({}):** {}\n\n",
        engines.len(),
        engines.join(", ")
    ));
    if engines.len() < 2 {
        out.push_str("_Fewer than two engines — no cross-engine comparison possible._\n\n");
    }
    if divergences.is_empty() {
        out.push_str("No divergences detected across the exercised engines.\n");
        return out;
    }
    out.push_str(&format!("**{} divergence(s):**\n\n", divergences.len()));
    for d in divergences {
        out.push_str(&format!("- {} — {}\n", d.kind, d.detail));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(engine: &str) -> EngineBlob {
        EngineBlob {
            engine: engine.into(),
            ..Default::default()
        }
    }

    #[test]
    fn feature_mismatch_is_high() {
        let mut a = blob("webview2");
        a.features.insert("dialog".into(), true);
        let mut b = blob("webkitgtk");
        b.features.insert("dialog".into(), false);
        let d = diff(&[a, b]);
        assert!(d
            .iter()
            .any(|x| x.kind.contains("HIGH") && x.kind.contains("feature")));
    }

    #[test]
    fn console_error_on_one_engine_is_medium() {
        let a = blob("webview2");
        let mut b = blob("wkwebview");
        b.console_errors.push("TypeError: x".into());
        let d = diff(&[a, b]);
        assert!(d
            .iter()
            .any(|x| x.kind.contains("MEDIUM") && x.detail.contains("only on wkwebview")));
    }

    #[test]
    fn computed_style_mismatch_is_medium() {
        let mut a = blob("webview2");
        a.computed_styles
            .entry(".btn".into())
            .or_default()
            .insert("gap".into(), "8px".into());
        let mut b = blob("webkitgtk");
        b.computed_styles
            .entry(".btn".into())
            .or_default()
            .insert("gap".into(), "0px".into());
        let d = diff(&[a, b]);
        assert!(d.iter().any(|x| x.kind.contains("computed-style")));
    }

    #[test]
    fn identical_blobs_produce_no_divergence() {
        let a = blob("webview2");
        let b = blob("webview2");
        assert!(diff(&[a, b]).is_empty());
    }

    #[test]
    fn report_lists_engines_and_flags_single_engine() {
        let out = render_report(&[blob("webview2")], &[]);
        assert!(out.contains("Engines exercised (1)"));
        assert!(out.contains("no cross-engine comparison"));
    }
}
