//! Host-gated live-engine capture test.
//!
//! If this host has a Chromium engine (Edge/Chrome), capture a real blob from a local
//! fixture page and assert its shape. If not, SKIP with a printed reason — never a false
//! green, never a false red on a host that simply lacks the engine.

use webview_qa::{find_edge, ChromiumDriver, Config, WebViewDriver};

#[test]
fn captures_a_real_blob_when_an_engine_is_present() {
    if find_edge().is_none() {
        eprintln!("skipped: no Chromium engine (msedge/chrome) on this host");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let url = webview_qa::driver::write_fixture_page(dir.path()).expect("fixture");

    let driver = ChromiumDriver::detect().expect("detect engine");
    let cfg = Config::sample();
    let blob = match driver.capture(&url, &cfg) {
        Ok(b) => b,
        Err(e) => panic!("live capture failed on a host that HAS an engine: {e}"),
    };

    assert_eq!(blob.engine, "chromium-edge");
    assert!(
        !blob.user_agent.is_empty(),
        "a real engine must report a user agent"
    );
    // Every configured feature must be answered (true or false), not missing.
    for key in cfg.features.keys() {
        assert!(
            blob.features.contains_key(key),
            "probe did not answer feature `{key}`"
        );
    }
    // The fixture sets body font-size; the probe config asks for it.
    let body = blob
        .computed_styles
        .get("body")
        .expect("body computed styles captured");
    assert!(
        body.contains_key("font-size"),
        "expected font-size in captured computed styles"
    );
}
