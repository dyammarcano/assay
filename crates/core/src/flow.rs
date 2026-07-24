//! Two deterministic flows: one for UWP, one for Electron.
//!
//! The kind of app decides the pipeline, because the two are not variations on one problem:
//!
//! * A **UWP** package renders native XAML — unless it is a *hosted PWA*, which ships no
//!   executable and is really a URL in Edge. Those two sub-cases have completely different
//!   endings, so the UWP flow classifies before it does anything else.
//! * An **Electron** app is Chromium-to-Chromium on Windows, so the interesting risk is not
//!   the engine but what the main process touches — and whether the sources are even readable.
//!
//! "Deterministic" here means the same input always produces the same ordered step list and the
//! same output tree. Every step records whether it ran, was skipped, or could not run — a
//! skipped step is reported, never silently omitted, because an absent section is
//! indistinguishable from a clean result.

use crate::{
    analyze, detect_native_modules, generate_sidecar, parse_appx_manifest, parse_electron,
    port_pwa_to_tauri, render_divergence, scaffold, Matrix,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// What happened at one step of a flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Ok(String),
    /// Deliberately not applicable to this input.
    Skipped(String),
    /// Wanted to run, could not. This is a finding, not silence.
    Blocked(String),
}

impl StepStatus {
    pub fn label(&self) -> &'static str {
        match self {
            StepStatus::Ok(_) => "ok",
            StepStatus::Skipped(_) => "skipped",
            StepStatus::Blocked(_) => "BLOCKED",
        }
    }
    pub fn detail(&self) -> &str {
        match self {
            StepStatus::Ok(s) | StepStatus::Skipped(s) | StepStatus::Blocked(s) => s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Step {
    pub name: &'static str,
    pub status: StepStatus,
}

/// A file the flow wants written, as (relative path, contents).
#[derive(Debug, Clone)]
pub struct Artifact {
    pub path: PathBuf,
    pub contents: String,
}

/// The result of running a flow: the ordered steps, the files to write, and the report.
#[derive(Debug, Clone)]
pub struct FlowResult {
    pub kind: &'static str,
    pub app_name: String,
    pub steps: Vec<Step>,
    pub artifacts: Vec<Artifact>,
    pub report_md: String,
}

impl FlowResult {
    pub fn blocked(&self) -> Vec<&Step> {
        self.steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Blocked(_)))
            .collect()
    }

    /// Write every artifact under `out_dir`, creating parent directories.
    pub fn write_to(&self, out_dir: &Path) -> std::io::Result<()> {
        for a in &self.artifacts {
            let full = out_dir.join(&a.path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(full, &a.contents)?;
        }
        std::fs::write(out_dir.join("REPORT.md"), &self.report_md)
    }
}

fn ok(name: &'static str, d: impl Into<String>) -> Step {
    Step { name, status: StepStatus::Ok(d.into()) }
}
fn skipped(name: &'static str, d: impl Into<String>) -> Step {
    Step { name, status: StepStatus::Skipped(d.into()) }
}
fn blocked(name: &'static str, d: impl Into<String>) -> Step {
    Step { name, status: StepStatus::Blocked(d.into()) }
}

/// Extension categories and capabilities the matrix has no row for.
///
/// This is the fix for the flow's worst failure mode: previously, anything unrecognised was
/// dropped, so a package declaring `runFullTrust` and two `appExtension`s reported an empty gap
/// list — reading as "nothing to worry about" when it actually meant "not looked at".
pub fn unmatched_uwp_surface(xml: &str, matched: &[String]) -> Vec<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();

    // Capabilities come from the vendored structured parser. `CapabilitiesBlock` splits them
    // across ~16 per-namespace vectors (`capability`, `uap_capability`, `restricted_capability`,
    // …); walking the serialized form collects every `Name` without hard-coding that list, so a
    // namespace added upstream does not silently start being ignored here.
    if let Ok(m) = msix::parse_appx_manifest(xml.as_bytes()) {
        if let Ok(v) = serde_json::to_value(&m.capabilities) {
            collect_names(&v, &mut found);
        }
    }
    // Extension categories still need a separate pass: the vendored parser defers the deep
    // <Extensions> tree.
    if let Ok(doc) = roxmltree::Document::parse(xml) {
        for node in doc.descendants() {
            if let Some(cat) = node.attribute("Category") {
                found.insert(format!("extension:{cat}"));
            }
        }
    }

    // Explicit map from manifest token to the capability id the parser produces for it.
    // A fuzzy name comparison was tried first and got this wrong in both directions:
    // `windows.shareTarget` never matches the id tail `share_target` (case and separator
    // differ), so a capability that WAS assessed got reported as a coverage gap.
    let handled: &[(&str, &str)] = &[
        ("extension:windows.shareTarget", "uwp.share_target"),
        ("extension:windows.backgroundTasks", "uwp.background_tasks"),
        ("extension:windows.protocol", "uwp.protocol_activation"),
    ];

    found
        .into_iter()
        .filter(|f| {
            match handled.iter().find(|(token, _)| token == f) {
                // Known token: it is only "covered" if the parser actually produced its id.
                Some((_, id)) => !matched.iter().any(|m| m == id),
                // Unknown token: never assessed, so always a coverage gap.
                None => true,
            }
        })
        .collect()
}

/// Recursively collect every `Name` value from a serialized capabilities block.
fn collect_names(v: &serde_json::Value, out: &mut BTreeSet<String>) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, val) in map {
                if k == "Name" || k == "name" {
                    if let Some(s) = val.as_str() {
                        let s = s.trim();
                        if !s.is_empty() {
                            out.insert(format!("capability:{s}"));
                        }
                    }
                }
                collect_names(val, out);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|i| collect_names(i, out)),
        _ => {}
    }
}

/// Is this Electron source bundled (webpack/esbuild/rollup output)?
///
/// Bundled sources defeat the textual API detection the Electron flow relies on, so the flow
/// has to say so. Minified bundles pack thousands of bytes onto a line; hand-written source
/// does not. VS Code's shipped `main.js` is ~1800 bytes/line.
pub fn looks_bundled(source: &str) -> Option<String> {
    let bytes = source.len();
    let lines = source.lines().count().max(1);
    let per_line = bytes / lines;
    if per_line >= 400 {
        Some(format!(
            "~{per_line} bytes/line across {lines} line(s) — this looks bundled/minified"
        ))
    } else {
        None
    }
}

/// Everything the UWP flow needs, already read from disk by the caller.
pub struct UwpInput {
    pub app_name: String,
    pub manifest_xml: String,
}

/// Everything the Electron flow needs, already read from disk by the caller.
pub struct ElectronInput {
    pub app_name: String,
    pub package_json: String,
    /// Concatenated main-process sources, and how many files produced them.
    pub main_source: String,
    pub file_count: usize,
    /// Set when the main process is packed in an asar and could not be read.
    pub packed_asar: Option<PathBuf>,
}

/// The UWP flow: classify (hosted PWA vs native), then either port or analyze.
pub fn run_uwp_flow(matrix: &Matrix, input: &UwpInput) -> FlowResult {
    let mut steps = Vec::new();
    let mut artifacts = Vec::new();
    let mut body = String::new();

    // 1. Structured identity, from the vendored parser.
    let parsed = msix::parse_appx_manifest(input.manifest_xml.as_bytes());
    match &parsed {
        Ok(m) => steps.push(ok(
            "parse-manifest",
            format!(
                "{} {} (publisher {})",
                if m.identity.name.is_empty() { "?" } else { &m.identity.name },
                m.identity.version,
                if m.identity.publisher.is_empty() { "?" } else { &m.identity.publisher },
            ),
        )),
        Err(e) => steps.push(blocked("parse-manifest", format!("unreadable manifest: {e}"))),
    }

    // 2. Classify. This decides the rest of the flow, so it comes before any analysis.
    let pwa = crate::detect_pwa(&input.manifest_xml);
    let profile = parse_appx_manifest(&input.manifest_xml);
    let analysis = analyze(matrix, &profile);

    match &pwa {
        Some(app) => {
            steps.push(ok(
                "classify",
                format!("hosted PWA — no executable; hosts {}", app.start_url),
            ));
            let port = port_pwa_to_tauri(app);
            artifacts.push(Artifact {
                path: PathBuf::from("src-tauri/Cargo.toml"),
                contents: port.cargo_toml.clone(),
            });
            artifacts.push(Artifact {
                path: PathBuf::from("src-tauri/build.rs"),
                contents: port.build_rs.clone(),
            });
            artifacts.push(Artifact {
                path: PathBuf::from("src-tauri/tauri.conf.json"),
                contents: port.tauri_conf.clone(),
            });
            artifacts.push(Artifact {
                path: PathBuf::from("src-tauri/src/main.rs"),
                contents: port.main_rs.clone(),
            });
            artifacts.push(Artifact {
                path: PathBuf::from("MIGRATION.md"),
                contents: port.migration_md.clone(),
            });
            steps.push(ok("port", format!("Tauri project emitted ({} caveats)", port.not_ported.len())));
            steps.push(skipped(
                "scaffold-bridge",
                "not applicable: a hosted PWA has no native capabilities to bridge",
            ));

            body.push_str("## Port\n\nThis package is a **hosted PWA**: it ships no executable and asks Edge to render a URL. Edge and WebView2 are both Chromium, so the port loads the same URL.\n\n");
            body.push_str(&format!("- Start URL: `{}`\n", app.start_url));
            body.push_str("\n### Not carried over\n\n");
            for n in &port.not_ported {
                body.push_str(&format!("- {n}\n"));
            }
        }
        None => {
            steps.push(ok(
                "classify",
                "conventional UWP package (renders native XAML)",
            ));
            steps.push(skipped(
                "port",
                "no mechanical port exists from native XAML to HTML — analyzed instead",
            ));
            let s = scaffold(&analysis);
            artifacts.push(Artifact { path: PathBuf::from("bridge/bridge.rs"), contents: s.rust.clone() });
            artifacts.push(Artifact {
                path: PathBuf::from("bridge/deps.txt"),
                contents: s.cargo_deps.join("\n"),
            });
            steps.push(ok("scaffold-bridge", format!("{} gap(s) wired", analysis.gaps.len())));
        }
    }

    // 3. Always report coverage honestly, whichever branch ran.
    let unmatched = unmatched_uwp_surface(&input.manifest_xml, &profile.capabilities);
    if unmatched.is_empty() {
        steps.push(ok("coverage-check", "no declared surface outside the matrix"));
    } else {
        steps.push(blocked(
            "coverage-check",
            format!("{} declared item(s) have no matrix row", unmatched.len()),
        ));
    }

    body.push_str("\n## Capabilities matched\n\n");
    if analysis.gaps.is_empty() {
        body.push_str("_None._\n");
    } else {
        for g in &analysis.gaps {
            body.push_str(&format!("- {} ({}): {:?}\n", g.name, g.id, g.tauri_path));
        }
    }
    body.push_str(&format!("\n{}\n", render_divergence(&analysis)));
    body.push_str(&unmatched_section(&unmatched));

    let report_md = render_report_md("UWP", &input.app_name, &steps, &body);
    FlowResult {
        kind: "uwp",
        app_name: input.app_name.clone(),
        steps,
        artifacts,
        report_md,
    }
}

/// The Electron flow: read sources, check they are actually readable, then analyze + scaffold.
pub fn run_electron_flow(matrix: &Matrix, input: &ElectronInput) -> FlowResult {
    let mut steps = Vec::new();
    let mut artifacts = Vec::new();
    let mut body = String::new();

    // 1. Can we see the main process at all?
    if let Some(asar) = &input.packed_asar {
        steps.push(blocked(
            "read-main-process",
            format!(
                "packed in {} — unpack it first; every capability result below is from \
package.json alone",
                asar.display()
            ),
        ));
    } else if input.file_count == 0 {
        steps.push(blocked("read-main-process", "no JS/TS sources found"));
    } else {
        steps.push(ok(
            "read-main-process",
            format!("{} source file(s)", input.file_count),
        ));
    }

    // 2. Bundling check — the detector is textual, so a bundle silently weakens every result.
    match looks_bundled(&input.main_source) {
        Some(why) => steps.push(blocked(
            "bundling-check",
            format!("{why}; textual API detection is unreliable here"),
        )),
        None => steps.push(ok("bundling-check", "sources look unbundled")),
    }

    // 3. Capabilities.
    let profile = parse_electron(&input.package_json, &input.main_source);
    let analysis = analyze(matrix, &profile);
    steps.push(ok(
        "detect-capabilities",
        format!("{} matched", profile.capabilities.len()),
    ));

    // 4. Native modules -> sidecar kit.
    let modules = detect_native_modules(&input.package_json);
    if modules.is_empty() {
        steps.push(skipped("sidecar", "no native Node modules in dependencies"));
    } else {
        let kit = generate_sidecar(&modules);
        artifacts.push(Artifact { path: PathBuf::from("sidecar/sidecar/Cargo.toml"), contents: kit.cargo_toml });
        artifacts.push(Artifact { path: PathBuf::from("sidecar/sidecar/src/main.rs"), contents: kit.main_rs });
        artifacts.push(Artifact { path: PathBuf::from("sidecar/sidecar_client.rs"), contents: kit.client_rs });
        artifacts.push(Artifact { path: PathBuf::from("sidecar/MIGRATION.md"), contents: kit.migration_md });
        artifacts.push(Artifact {
            path: PathBuf::from("sidecar/tauri.conf.snippet.json"),
            contents: kit.tauri_conf_snippet,
        });
        steps.push(ok("sidecar", format!("{} native module(s)", modules.len())));
    }

    // 5. Bridge scaffolding.
    let s = scaffold(&analysis);
    artifacts.push(Artifact { path: PathBuf::from("bridge/bridge.rs"), contents: s.rust.clone() });
    artifacts.push(Artifact { path: PathBuf::from("bridge/deps.txt"), contents: s.cargo_deps.join("\n") });
    steps.push(ok("scaffold-bridge", format!("{} gap(s) wired", analysis.gaps.len())));

    body.push_str("## Capabilities matched\n\n");
    if analysis.gaps.is_empty() {
        body.push_str("_None._\n");
    } else {
        for g in &analysis.gaps {
            body.push_str(&format!("- {} ({}): {:?}\n", g.name, g.id, g.tauri_path));
        }
    }
    body.push_str(&format!("\n{}\n", render_divergence(&analysis)));
    body.push_str(
        "\n## Coverage caveat\n\nElectron detection is textual: it matches API identifiers in \
the main-process sources. Dynamically-built or aliased requires are not detected, and a \
bundled main process can defeat it entirely. Treat the list above as a floor, not a total.\n",
    );

    let report_md = render_report_md("Electron", &input.app_name, &steps, &body);
    FlowResult {
        kind: "electron",
        app_name: input.app_name.clone(),
        steps,
        artifacts,
        report_md,
    }
}

fn unmatched_section(unmatched: &[String]) -> String {
    let mut s = String::from("\n## Declared surface with no matrix row\n\n");
    if unmatched.is_empty() {
        s.push_str("_None — everything the manifest declares is represented in the matrix._\n");
    } else {
        s.push_str(
            "These are declared by the app but have no row in the capability matrix. They were \
**not assessed** — this is a gap in coverage, not a clean result:\n\n",
        );
        for u in unmatched {
            s.push_str(&format!("- `{u}`\n"));
        }
    }
    s
}

fn render_report_md(kind: &str, app: &str, steps: &[Step], body: &str) -> String {
    let mut s = format!("# {app} — {kind} flow\n\n## Steps\n\n");
    for st in steps {
        s.push_str(&format!(
            "- **{}** — {}: {}\n",
            st.name,
            st.status.label(),
            st.status.detail()
        ));
    }
    let blocked_count = steps
        .iter()
        .filter(|s| matches!(s.status, StepStatus::Blocked(_)))
        .count();
    if blocked_count > 0 {
        s.push_str(&format!(
            "\n> {blocked_count} step(s) BLOCKED. Results below are incomplete accordingly.\n"
        ));
    }
    s.push('\n');
    s.push_str(body);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const PWA: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:uap3="http://schemas.microsoft.com/appx/manifest/uap/windows10/3"
  xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities">
  <Identity Name="Facebook.InstagramBeta" Publisher="CN=X" Version="42.0.23.0" />
  <Properties><DisplayName>Instagram</DisplayName></Properties>
  <Dependencies>
    <uap10:HostRuntimeDependency Name="Microsoft.MicrosoftEdge.Stable" MinVersion="1.0.0.0" />
  </Dependencies>
  <Applications>
    <Application Id="App" uap10:HostId="PWA" uap10:Parameters="--app-fallback-url=https://www.instagram.com/ --display-mode=standalone">
      <Extensions>
        <uap:Extension Category="windows.shareTarget">
          <uap:ShareTarget><uap:SupportedFileTypes><uap:FileType>.jpg</uap:FileType></uap:SupportedFileTypes></uap:ShareTarget>
        </uap:Extension>
        <uap3:Extension Category="windows.appExtension">
          <uap3:AppExtension Name="microsoft.store.edgePWA" Id="MicrosoftEdge" PublicFolder="Public" />
        </uap3:Extension>
      </Extensions>
    </Application>
  </Applications>
  <Capabilities><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>"#;

    const NATIVE_UWP: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10">
  <Identity Name="Contoso.App" Publisher="CN=Y" Version="1.2.3.0" />
  <Properties><DisplayName>Contoso</DisplayName></Properties>
  <Applications>
    <Application Id="App" Executable="App.exe" EntryPoint="App.App">
      <Extensions>
        <uap:Extension Category="windows.protocol">
          <uap:Protocol Name="contoso" />
        </uap:Extension>
      </Extensions>
    </Application>
  </Applications>
</Package>"#;

    fn uwp(xml: &str, name: &str) -> FlowResult {
        run_uwp_flow(
            &Matrix::embedded(),
            &UwpInput { app_name: name.into(), manifest_xml: xml.into() },
        )
    }

    #[test]
    fn the_uwp_flow_ports_a_hosted_pwa_and_skips_bridging() {
        let r = uwp(PWA, "Instagram");
        assert_eq!(r.kind, "uwp");
        let names: Vec<_> = r.steps.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            vec!["parse-manifest", "classify", "port", "scaffold-bridge", "coverage-check"],
            "the step list must be deterministic"
        );
        let paths: Vec<String> = r.artifacts.iter().map(|a| a.path.display().to_string()).collect();
        assert!(paths.iter().any(|p| p.contains("tauri.conf.json")));
        assert!(paths.iter().any(|p| p.contains("build.rs")));
        // A hosted PWA has nothing to bridge, and that is stated rather than omitted.
        let bridge = r.steps.iter().find(|s| s.name == "scaffold-bridge").unwrap();
        assert!(matches!(bridge.status, StepStatus::Skipped(_)));
    }

    #[test]
    fn the_uwp_flow_refuses_to_port_a_native_package() {
        let r = uwp(NATIVE_UWP, "Contoso");
        let port = r.steps.iter().find(|s| s.name == "port").unwrap();
        assert!(matches!(port.status, StepStatus::Skipped(_)));
        assert!(port.status.detail().contains("native XAML"));
        // ...but it still scaffolds what it legitimately can.
        let bridge = r.steps.iter().find(|s| s.name == "scaffold-bridge").unwrap();
        assert!(matches!(bridge.status, StepStatus::Ok(_)));
    }

    #[test]
    fn an_assessed_extension_is_not_also_reported_as_a_coverage_gap() {
        // shareTarget IS in the matrix and does appear in the divergence report, so listing it
        // as "no matrix row" would contradict the rest of the same report.
        let unmatched = unmatched_uwp_surface(PWA, &["uwp.share_target".into()]);
        assert!(
            !unmatched.iter().any(|u| u.contains("shareTarget")),
            "shareTarget was assessed and must not be flagged as uncovered: {unmatched:?}"
        );
        // The genuinely unassessed ones are still reported.
        assert!(unmatched.iter().any(|u| u == "capability:runFullTrust"));
        assert!(unmatched.iter().any(|u| u.contains("appExtension")));
    }

    #[test]
    fn identity_is_parsed_from_a_self_closing_element() {
        // Real manifests write `<Identity ... />`; the vendored parser originally handled only
        // Start events and silently produced an empty identity.
        let m = msix::parse_appx_manifest(PWA.as_bytes()).expect("parse");
        assert_eq!(m.identity.name, "Facebook.InstagramBeta");
        assert_eq!(m.identity.version, "42.0.23.0");
    }

    #[test]
    fn runfulltrust_is_reported_as_unassessed_instead_of_dropped() {
        // The original bug: this capability vanished from the output entirely.
        let r = uwp(PWA, "Instagram");
        assert!(
            r.report_md.contains("runFullTrust"),
            "declared capability must appear in the report:\n{}",
            r.report_md
        );
        assert!(r.report_md.contains("no matrix row"));
        let cov = r.steps.iter().find(|s| s.name == "coverage-check").unwrap();
        assert!(matches!(cov.status, StepStatus::Blocked(_)));
    }

    #[test]
    fn a_fully_covered_manifest_reports_no_coverage_gap() {
        let unmatched = unmatched_uwp_surface(NATIVE_UWP, &["uwp.protocol_activation".into()]);
        assert!(
            unmatched.is_empty(),
            "protocol activation is in the matrix, so nothing should be unmatched: {unmatched:?}"
        );
    }

    fn electron(main: &str, files: usize, pkg: &str) -> FlowResult {
        run_electron_flow(
            &Matrix::embedded(),
            &ElectronInput {
                app_name: "Test".into(),
                package_json: pkg.into(),
                main_source: main.into(),
                file_count: files,
                packed_asar: None,
            },
        )
    }

    #[test]
    fn the_electron_flow_step_list_is_deterministic() {
        let r = electron("const { Tray } = require('electron')\n", 1, "{}");
        assert_eq!(r.kind, "electron");
        let names: Vec<_> = r.steps.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            vec![
                "read-main-process",
                "bundling-check",
                "detect-capabilities",
                "sidecar",
                "scaffold-bridge"
            ]
        );
    }

    #[test]
    fn a_bundled_main_process_is_flagged_not_silently_trusted() {
        // One enormous line, the shape of webpack output.
        let bundled = format!("const a=1;{}\n", "x".repeat(5000));
        let r = electron(&bundled, 1, "{}");
        let step = r.steps.iter().find(|s| s.name == "bundling-check").unwrap();
        assert!(
            matches!(step.status, StepStatus::Blocked(_)),
            "bundled sources must be reported: {step:?}"
        );
        assert!(step.status.detail().contains("unreliable"));
    }

    #[test]
    fn ordinary_sources_are_not_misflagged_as_bundled() {
        let normal = "const { app, Tray } = require('electron')\n\
                      app.on('ready', () => {\n  console.log('hi')\n})\n";
        assert!(looks_bundled(normal).is_none());
    }

    #[test]
    fn a_packed_asar_blocks_the_read_step_and_says_results_are_partial() {
        let r = run_electron_flow(
            &Matrix::embedded(),
            &ElectronInput {
                app_name: "Packed".into(),
                package_json: "{}".into(),
                main_source: String::new(),
                file_count: 0,
                packed_asar: Some(PathBuf::from("C:\\a\\resources\\app.asar")),
            },
        );
        let step = r.steps.iter().find(|s| s.name == "read-main-process").unwrap();
        assert!(matches!(step.status, StepStatus::Blocked(_)));
        assert!(r.report_md.contains("BLOCKED"));
        assert!(!r.blocked().is_empty());
    }

    #[test]
    fn every_report_states_the_electron_detection_caveat() {
        let r = electron("const { Tray } = require('electron')\n", 1, "{}");
        assert!(r.report_md.contains("Coverage caveat"));
        assert!(r.report_md.contains("floor, not a total"));
    }
}
