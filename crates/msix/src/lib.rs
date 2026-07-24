//! `msix` — MSIX/AppX package inspection.
//!
//! PROVENANCE: vendored from the author's `unravel-rs` workspace
//! (`crates/msix`, BSD-3-Clause, same author). Copied rather than referenced because
//! `unravel-rs` is local-only and unpublished, and `assay` must stay buildable from a clone.
//! Re-sync by hand if the upstream copy changes.
//!
//! KNOWN GAP carried over from upstream: the deep `<Extensions>` tree is NOT parsed (upstream
//! unit "5b" was never ported). Extension categories — `windows.shareTarget`,
//! `windows.appExtension`, protocol activation — must still be read separately. See
//! `core::parse` for that half.
//!
//! Faithful 1:1 port of Go `pkg/msix`.
//!
//! Unit 5a lands the pure AppxManifest parser (`parse_appx_manifest`, the
//! ordered `CapabilitiesBlock` walk, and `format_bytes`). The zip-backed
//! `Info`/`Extract`/`Verify`/`IsMSIX` surface, the deep `<Extension>` flatten,
//! and `InfoFromDir` land in unit 5b.

pub mod manifest;

pub use manifest::{
    format_bytes, parse_appx_manifest, AppxApplication, AppxManifest, AppxVisualElementsXml,
    CapabilitiesBlock, DeviceCap, DeviceChild, DeviceFunc, Identity, NamedCap, OrderedCapRef,
    Properties, TargetDeviceFamily, MAX_CAPABILITY_ENTRIES,
};
