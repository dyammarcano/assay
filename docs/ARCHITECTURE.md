# Assay Architecture
<!-- rev:001 (RFC 3339) 2026-07-23T00:00:00Z -->

Four crates in one cargo workspace. `core` holds all logic and is data-driven off
`data/matrix.toml`; `cli` is a thin shell; `winrt-shim` and `webview-qa` are independent
tools that close specific gaps the matrix identifies.

## Crate graph

```mermaid
graph TD
    matrix[("data/matrix.toml<br/>cited capability dataset")]
    core["core<br/>matrix · parse · analyze<br/>scaffold · sidecar · report"]
    cli["cli<br/>bin: assay"]
    shim["winrt-shim<br/>toast content + XML"]
    wvqa["webview-qa<br/>probe · diff · report"]

    matrix -->|include_str! / --matrix| core
    core --> cli
    shim -.->|closes uwp.toast gap| matrix
    wvqa -.->|measures WebView divergence| matrix
```

## The main flow (`analyze` / `scaffold`)

```mermaid
flowchart LR
    A["AppxManifest.xml<br/>or package.json + main.js"] -->|parse| P["Profile<br/>(capability ids)"]
    M["profile.toml<br/>(hand-written)"] --> P
    P --> AN{{"analyze<br/>vs Matrix"}}
    AN -->|path exists| G["Gap list"]
    AN -->|none / open_question| D["Divergence report<br/>+ WebView engine note"]
    AN -->|id not in matrix| U["Unknown (warn, skip)"]
    G --> S{{"scaffold"}}
    S -->|recipe = proven| C["bridge.rs<br/>real plugin/crate wiring"]
    S -->|no proven recipe| T["bridge.rs<br/>todo!() stub + citation"]
    S --> DEP["deps.txt"]
```

**The honesty boundary is the `analyze` split:** only capabilities that reach the *gap list*
can ever reach the scaffolder. `none` and `open_question` rows are routed to the divergence
report and never produce code.

## Sidecar kit flow (`sidecar`)

```mermaid
flowchart LR
    PKG["package.json"] -->|detect_native_modules| NM["NativeModule[]<br/>name + reason + prebuilds"]
    NM --> GEN["generate_sidecar"]
    GEN --> M1["sidecar/src/main.rs<br/>stdio-JSON loop, todo!() per module"]
    GEN --> M2["sidecar/Cargo.toml"]
    GEN --> M3["sidecar_client.rs<br/>SidecarClient"]
    GEN --> M4["MIGRATION.md<br/>decision checklist"]
    GEN --> M5["tauri.conf.snippet.json<br/>externalBin + shell scope"]
```

## Cross-WebView harness flow

```mermaid
flowchart LR
    CFG["webview-qa.toml<br/>features + selectors"] -->|render_probe| JS["probe JS (per engine)"]
    JS -.->|driver evals in page<br/>(host-gated, not yet built)| B1["EngineBlob (webview2)"]
    JS -.-> B2["EngineBlob (wkwebview)"]
    B1 --> DIFF{{"diff (pairwise)"}}
    B2 --> DIFF
    DIFF --> R["Divergence report<br/>HIGH feature · MEDIUM style/console · INFO ua"]
```

The dashed edges are the **only** unbuilt part: live engine drivers. Everything downstream of
a recorded blob works today, which is why `webview-qa diff` operates on blob files.

## Key invariants in code

| Invariant | Enforced by |
|---|---|
| Every matrix row cites a public doc | `core::matrix` test `every_row_has_a_citation` |
| Generated bridge is valid Rust | `core::scaffold` test via `syn::parse_file` |
| Generated sidecar is valid Rust | `core::sidecar` test via `syn::parse_file` |
| Generator output doesn't drift | `insta` golden snapshots (bridge, sidecar, report) |
| A 1-engine run isn't cross-engine | `webview_qa::render_report` engines-exercised line |
