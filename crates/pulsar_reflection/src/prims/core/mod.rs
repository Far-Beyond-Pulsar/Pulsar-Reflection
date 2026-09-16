//! Core primitive type registrations.

mod bool;
pub mod enum_dropdown;
mod f32;
mod f64;
mod i32;
mod i64;
mod matrices;
mod u32;
mod u64;
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