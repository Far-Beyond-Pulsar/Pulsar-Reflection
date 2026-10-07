//! Shared reflected asset-reference value types.

use serde::{Deserialize, Serialize};

/// Project-relative reference to a texture asset.
///
/// The path is serialized as a plain string, while consumers can use its
/// distinct reflected type to provide texture-specific pickers and compiler
/// handling.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TextureSrc(pub String);

impl TextureSrc {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for TextureSrc {
    fn from(path: String) -> Self {
        Self(path)
    }
}

impl From<&str> for TextureSrc {
    fn from(path: &str) -> Self {
        Self(path.to_owned())
    }
}

impl std::fmt::Display for TextureSrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn serialize_texture_src(value: &TextureSrc) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::Value::String(value.0.clone()))
}

fn deserialize_texture_src(value: serde_json::Value) -> crate::ReflectResult<TextureSrc> {
    value
        .as_str()
        .map(TextureSrc::from)
        .ok_or_else(|| crate::ReflectError::TypeMismatch {
            expected: "TextureSrc",
            found: format!("{value:?}"),
        })
}

#[crate::pulsar_type(
    serialize_json_with = serialize_texture_src,
    deserialize_json_with = deserialize_texture_src
)]
#[allow(dead_code)]
type RegisteredTextureSrc = TextureSrc;
