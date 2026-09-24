//! Core primitive type registrations.

mod bool;
pub mod enum_dropdown;
mod f32;
mod f64;
mod i32;
mod i64;
mod mat4x3;
mod mat4x4;
// Flat arrays used by renderer components ([f32; 16], [u32; 116], ..).
// The nested [[f32; 4]; N] matrices live in mat4x3.rs / mat4x4.rs.
mod matrices;
mod u32;
mod u64;
// `[f32; 4]` is already registered by `color.rs` (Reflectable is a global,
// type-identity-keyed registration -- there is no per-field/per-semantic
// registration, so any `[f32; 4]` field decodes as RGBA-shaped `[f,f,f,f]`
// JSON regardless of whether it's actually a color).
mod vec3;

/// Public so consumers can reuse the RGBA↔HSLA conversion the colour editor
/// uses — several panels colour-code pin badges with the same maths.
pub mod color;

/// Force-link the built-in implementations into downstream binaries so their
/// inventory registrations are not discarded by the linker.
pub(crate) fn ensure_registered() {
    let _ = <bool as crate::Reflectable>::type_info();
    let _ = <f32 as crate::Reflectable>::type_info();
    let _ = <f64 as crate::Reflectable>::type_info();
    let _ = <i32 as crate::Reflectable>::type_info();
    let _ = <i64 as crate::Reflectable>::type_info();
    let _ = <u32 as crate::Reflectable>::type_info();
    let _ = <u64 as crate::Reflectable>::type_info();
    let _ = <[f32; 2] as crate::Reflectable>::type_info();
    let _ = <[f32; 3] as crate::Reflectable>::type_info();
    let _ = <[u32; 3] as crate::Reflectable>::type_info();
    let _ = <[u32; 116] as crate::Reflectable>::type_info();
    let _ = <[f32; 4] as crate::Reflectable>::type_info();
    let _ = <[[f32; 4]; 3] as crate::Reflectable>::type_info();
    let _ = <[[f32; 4]; 4] as crate::Reflectable>::type_info();
    let _ = <[f32; 12] as crate::Reflectable>::type_info();
    let _ = <[f32; 16] as crate::Reflectable>::type_info();
}