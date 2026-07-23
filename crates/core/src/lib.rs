pub mod analyze;
pub mod matrix;
pub mod parse;
pub mod profile;
pub mod report;
pub mod scaffold;

pub use analyze::{analyze, render_divergence, Analysis, DivergenceItem, GapItem};
pub use matrix::{Capability, Matrix, MatrixError, Recipe, Severity, Source, TauriPath};
pub use parse::{parse_appx_manifest, parse_electron};
pub use profile::Profile;
pub use report::render_report;
pub use scaffold::{scaffold, ScaffoldOutput};
