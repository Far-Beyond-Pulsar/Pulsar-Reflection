//! The reflection system as one shared library.
//!
//! This crate contains `pulsar_reflection` and every crate it links that no other shared
//! library holds. A binary or plugin library that links it (`use reflection_dylib as _;`) gets
//! reflection from this library, so its registries and process-wide state exist once per
//! process. The UI libraries come from `wgpui_dylib` and `ui_dylib`.

// Linked, not just depended on: a crate this library shares with another shared library must
// come from that library, or it would be in both (rustc: "only shows up once").
use ui_dylib as _;
use wgpui_dylib as _;

pub use pulsar_reflection;
