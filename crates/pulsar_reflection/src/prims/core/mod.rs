//! Core primitive type registrations.

mod bool;
pub mod enum_dropdown;
mod f32;
mod f64;
mod i32;
mod i64;
mod mat4x3;
mod mat4x4;
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
