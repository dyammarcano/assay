pub mod analyze;
pub mod matrix;
pub mod parse;
pub mod profile;
pub mod report;
pub mod scaffold;
pub mod sidecar;

pub use analyze::{analyze, render_divergence, Analysis, DivergenceItem, GapItem};
pub use matrix::{
    Capability, Matrix, MatrixError, ParityTier, Recipe, Severity, Source, TauriPath,
};
pub use parse::{detect_native_modules, parse_appx_manifest, parse_electron, NativeModule};
pub use profile::Profile;
pub use report::render_report;
pub use scaffold::{scaffold, ScaffoldOutput};
pub use sidecar::{generate_sidecar, SidecarKit};
