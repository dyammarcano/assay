//! Probe protocol (harness spec "Probe protocol").
//!
//! A [`Config`] declares what to measure — feature expressions and per-selector CSS
//! properties — and [`render_probe`] inlines it into a self-contained JS snippet that a
//! driver evaluates in the page. The snippet returns a JSON string matching
//! [`crate::EngineBlob`], so a driver only has to `eval()` it and hand the result to
//! [`crate::diff`]. This is the missing producer for the blobs the differ consumes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a probe run measures.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    /// Pages to load (drivers iterate these).
    #[serde(default)]
    pub urls: Vec<String>,
    /// feature name -> a JS expression whose truthiness is the support signal.
    #[serde(default)]
    pub features: BTreeMap<String, String>,
    /// CSS selector -> the computed properties to capture for it.
    #[serde(default)]
    pub selectors: BTreeMap<String, Vec<String>>,
}

impl Config {
    /// Parse a `webview-qa.toml`.
    pub fn from_toml(s: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(s)
    }

    /// A starter config covering the divergences most likely to bite a port.
    pub fn sample() -> Config {
        let mut features = BTreeMap::new();
        features.insert("css_gap".into(), "CSS.supports('gap', '1px')".into());
        features.insert(
            "dialog_element".into(),
            "typeof HTMLDialogElement !== 'undefined'".into(),
        );
        features.insert(
            "backdrop_filter".into(),
            "CSS.supports('backdrop-filter', 'blur(1px)')".into(),
        );
        features.insert(
            "resize_observer".into(),
            "typeof ResizeObserver !== 'undefined'".into(),
        );
        let mut selectors = BTreeMap::new();
        selectors.insert(
            "body".into(),
            vec!["font-family".into(), "font-size".into()],
        );
        Config {
            urls: vec!["http://localhost:1420/".into()],
            features,
            selectors,
        }
    }

    /// Render this config as a `webview-qa.toml` document.
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("serialize config")
    }
}

/// Render the injectable probe snippet for `engine`, inlining `cfg`.
///
/// The result is an IIFE that returns a JSON string shaped like [`crate::EngineBlob`].
pub fn render_probe(engine: &str, cfg: &Config) -> String {
    let features_json = serde_json::to_string(&cfg.features).expect("features json");
    let selectors_json = serde_json::to_string(&cfg.selectors).expect("selectors json");
    let engine_json = serde_json::to_string(engine).expect("engine json");
    format!(
        r#"(function () {{
  var ENGINE = {engine_json};
  var FEATURES = {features_json};
  var SELECTORS = {selectors_json};
  var errors = [];
  try {{
    window.addEventListener('error', function (e) {{ errors.push(String(e.message)); }});
  }} catch (e) {{ /* no window (worker/headless shim) */ }}

  var features = {{}};
  for (var name in FEATURES) {{
    try {{ features[name] = !!eval(FEATURES[name]); }}
    catch (e) {{ features[name] = false; }}
  }}

  var computed_styles = {{}};
  for (var sel in SELECTORS) {{
    var el = null;
    try {{ el = document.querySelector(sel); }} catch (e) {{ el = null; }}
    if (!el) {{ continue; }}
    var cs = getComputedStyle(el);
    var props = {{}};
    var list = SELECTORS[sel];
    for (var i = 0; i < list.length; i++) {{
      props[list[i]] = cs.getPropertyValue(list[i]);
    }}
    computed_styles[sel] = props;
  }}

  return JSON.stringify({{
    engine: ENGINE,
    user_agent: (typeof navigator !== 'undefined' ? navigator.userAgent : ''),
    features: features,
    computed_styles: computed_styles,
    console_errors: errors
  }});
}})()
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineBlob;

    #[test]
    fn sample_config_round_trips_through_toml() {
        let c = Config::sample();
        let text = c.to_toml();
        let back = Config::from_toml(&text).expect("reparse");
        assert_eq!(back.features.len(), c.features.len());
        assert!(back.selectors.contains_key("body"));
        assert!(!back.urls.is_empty());
    }

    #[test]
    fn probe_inlines_engine_features_and_selectors() {
        let js = render_probe("webview2", &Config::sample());
        assert!(js.contains("\"webview2\""));
        assert!(js.contains("css_gap"));
        assert!(js.contains("font-family"));
        assert!(js.contains("JSON.stringify"));
    }

    #[test]
    fn probe_output_shape_deserializes_as_an_engine_blob() {
        // The snippet's return shape must match EngineBlob; assert on a literal
        // instance of that shape so a field rename breaks this test.
        let sample = r#"{"engine":"webview2","user_agent":"UA",
            "features":{"css_gap":true},
            "computed_styles":{"body":{"font-size":"16px"}},
            "console_errors":[]}"#;
        let blob: EngineBlob = serde_json::from_str(sample).expect("blob parses");
        assert_eq!(blob.engine, "webview2");
        assert_eq!(blob.features.get("css_gap"), Some(&true));
    }

    #[test]
    fn empty_config_still_renders_runnable_probe() {
        let js = render_probe("wkwebview", &Config::default());
        assert!(js.contains("\"wkwebview\""));
        assert!(js.contains("computed_styles"));
    }
}
