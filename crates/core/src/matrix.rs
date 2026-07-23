use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Uwp,
    Electron,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TauriPath {
    Native,
    Plugin,
    CustomRust,
    Sidecar,
    None,
    OpenQuestion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Full,
    Partial,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Recipe {
    Proven,
    Stub,
}

/// How deep the parity promise goes for a capability (ADR 0001).
///
/// `Behavioral` = the capability works. `Visual` = it must also look/feel the same, which
/// requires measurement by the webview-qa harness before it may be claimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParityTier {
    #[default]
    Behavioral,
    Visual,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Capability {
    pub id: String,
    pub source: Source,
    pub name: String,
    pub description: String,
    pub tauri_path: TauriPath,
    pub severity: Severity,
    pub citation_url: String,
    #[serde(default)]
    pub recipe: Option<Recipe>,
    #[serde(default)]
    pub plugin: Option<String>,
    #[serde(default)]
    pub crate_name: Option<String>,
    /// Absent = [`ParityTier::Behavioral`]. See [`Capability::parity_tier`].
    #[serde(default)]
    pub parity_tier: Option<ParityTier>,
}

impl Capability {
    /// The resolved parity tier (defaults to `Behavioral` when unset).
    pub fn parity_tier(&self) -> ParityTier {
        self.parity_tier.unwrap_or_default()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Matrix {
    #[serde(default)]
    pub capabilities: Vec<Capability>,
}

#[derive(Debug)]
pub enum MatrixError {
    Parse(String),
}

impl std::fmt::Display for MatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatrixError::Parse(m) => write!(f, "matrix parse error: {m}"),
        }
    }
}
impl std::error::Error for MatrixError {}

pub const EMBEDDED_MATRIX: &str = include_str!("../../../data/matrix.toml");

impl Matrix {
    pub fn from_toml(s: &str) -> Result<Matrix, MatrixError> {
        toml::from_str(s).map_err(|e| MatrixError::Parse(e.to_string()))
    }
    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.capabilities.iter().find(|c| c.id == id)
    }
    pub fn embedded() -> Matrix {
        Matrix::from_toml(EMBEDDED_MATRIX).expect("embedded matrix must be valid TOML")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[[capabilities]]
id = "uwp.toast"
source = "uwp"
name = "Toast notifications"
description = "OS notification-center toasts"
tauri_path = "custom_rust"
severity = "partial"
citation_url = "https://docs.rs/tauri-winrt-notification"
recipe = "proven"
crate_name = "tauri-winrt-notification"

[[capabilities]]
id = "uwp.live_tiles"
source = "uwp"
name = "Live Tiles"
description = "Dynamic Start-menu tile content"
tauri_path = "none"
severity = "none"
citation_url = "https://learn.microsoft.com/en-us/windows/uwp/launch-resume/update-a-live-tile-from-a-background-task"
"#;

    #[test]
    fn parses_capabilities_and_looks_up_by_id() {
        let m = Matrix::from_toml(SAMPLE).expect("should parse");
        assert_eq!(m.capabilities.len(), 2);
        let toast = m.get("uwp.toast").expect("toast present");
        assert_eq!(toast.tauri_path, TauriPath::CustomRust);
        assert_eq!(toast.recipe, Some(Recipe::Proven));
        assert_eq!(
            toast.crate_name.as_deref(),
            Some("tauri-winrt-notification")
        );
        let tiles = m.get("uwp.live_tiles").expect("tiles present");
        assert_eq!(tiles.tauri_path, TauriPath::None);
        assert!(tiles.recipe.is_none());
    }

    #[test]
    fn embedded_matrix_parses() {
        let m = Matrix::embedded();
        assert!(m.get("uwp.toast").is_some());
        assert!(m.get("electron.ipc").is_some());
    }

    #[test]
    fn parity_tier_defaults_to_behavioral_when_absent() {
        let m = Matrix::from_toml(SAMPLE).expect("should parse");
        // Neither SAMPLE row declares a tier.
        assert_eq!(
            m.get("uwp.toast").unwrap().parity_tier(),
            ParityTier::Behavioral
        );
    }

    #[test]
    fn embedded_matrix_marks_ui_capabilities_visual() {
        let m = Matrix::embedded();
        for id in ["uwp.toast", "electron.tray", "electron.menu"] {
            assert_eq!(
                m.get(id).unwrap().parity_tier(),
                ParityTier::Visual,
                "{id} should be visual tier"
            );
        }
        // A non-UI capability stays behavioral.
        assert_eq!(
            m.get("electron.ipc").unwrap().parity_tier(),
            ParityTier::Behavioral
        );
    }

    #[test]
    fn every_row_has_a_citation() {
        let m = Matrix::embedded();
        for c in &m.capabilities {
            assert!(
                !c.citation_url.trim().is_empty(),
                "row {} missing citation",
                c.id
            );
        }
    }
}
