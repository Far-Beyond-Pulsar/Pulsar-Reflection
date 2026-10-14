//! Shared reflected asset-reference value types.

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// A category used to prevent assigning a reference to the wrong asset kind.
pub trait AssetKind: 'static {
    const KIND: &'static str;
    /// File extensions accepted by this reference kind, without leading dots.
    /// The picker uses this list to avoid returning a GUID belonging to a
    /// different asset category.
    const EXTENSIONS: &'static [&'static str] = &[];

    fn accepts_path(path: &std::path::Path) -> bool {
        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            return false;
        };
        Self::EXTENSIONS
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(extension))
    }
}

/// A stable GUID reference whose kind is checked by Rust's type system.
///
/// The marker has no serialized representation; every `AssetRef<K>` is a
/// canonical UUID string on disk.
pub struct AssetRef<K: AssetKind> {
    id: String,
    marker: PhantomData<fn() -> K>,
}

impl<K: AssetKind> AssetRef<K> {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            marker: PhantomData,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        uuid::Uuid::parse_str(value.trim().trim_matches(|c| c == '{' || c == '}'))
            .ok()
            .map(|id| Self {
                id: id.to_string(),
                marker: PhantomData,
            })
    }

    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl<K: AssetKind> Clone for AssetRef<K> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            marker: PhantomData,
        }
    }
}

impl<K: AssetKind> std::fmt::Debug for AssetRef<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AssetRef").field(&self.id).finish()
    }
}

impl<K: AssetKind> PartialEq for AssetRef<K> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<K: AssetKind> Eq for AssetRef<K> {}
impl<K: AssetKind> std::hash::Hash for AssetRef<K> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.id, state);
    }
}
impl<K: AssetKind> Default for AssetRef<K> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K: AssetKind> Serialize for AssetRef<K> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.id)
    }
}
impl<'de, K: AssetKind> Deserialize<'de> for AssetRef<K> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value)
            .ok_or_else(|| serde::de::Error::custom("asset reference must be a UUID"))
    }
}

pub enum TextureAssetKind {}
impl AssetKind for TextureAssetKind {
    const KIND: &'static str = "texture";
    const EXTENSIONS: &'static [&'static str] = &[
        "png", "jpg", "jpeg", "exr", "hdr", "tga", "webp", "bmp", "dds", "ktx", "pif",
    ];
}
pub enum MaterialAssetKind {}
impl AssetKind for MaterialAssetKind {
    const KIND: &'static str = "material";
    const EXTENSIONS: &'static [&'static str] = &["mat", "pulsarmat", "material"];
}
pub enum ImageAssetKind {}
impl AssetKind for ImageAssetKind {
    const KIND: &'static str = "image";
    const EXTENSIONS: &'static [&'static str] = &[
        "png", "jpg", "jpeg", "exr", "hdr", "tga", "webp", "bmp", "dds", "ktx", "gif", "tif",
        "tiff", "pif",
    ];
}
pub enum SkeletonAssetKind {}
impl AssetKind for SkeletonAssetKind {
    const KIND: &'static str = "skeleton";
    const EXTENSIONS: &'static [&'static str] = &["skeleton"];
}
pub enum SkeletalMeshAssetKind {}
impl AssetKind for SkeletalMeshAssetKind {
    const KIND: &'static str = "skmesh";
    const EXTENSIONS: &'static [&'static str] = &["skmesh"];
}
pub enum AnimationSequenceAssetKind {}
impl AssetKind for AnimationSequenceAssetKind {
    const KIND: &'static str = "animseq";
    const EXTENSIONS: &'static [&'static str] = &["animseq"];
}
pub enum BlendSpaceAssetKind {}
impl AssetKind for BlendSpaceAssetKind {
    const KIND: &'static str = "blendspace";
    const EXTENSIONS: &'static [&'static str] = &["blendspace"];
}
pub enum AnimationMontageAssetKind {}
impl AssetKind for AnimationMontageAssetKind {
    const KIND: &'static str = "anim_montage";
    const EXTENSIONS: &'static [&'static str] = &["montage", "anim_montage"];
}
pub enum AnimationGraphAssetKind {}
impl AssetKind for AnimationGraphAssetKind {
    const KIND: &'static str = "anim_graph";
    const EXTENSIONS: &'static [&'static str] = &["animgraph", "anim_graph"];
}

pub type TextureRef = AssetRef<TextureAssetKind>;
pub type MaterialRef = AssetRef<MaterialAssetKind>;
pub type ImageRef = AssetRef<ImageAssetKind>;

/// A texture reference that reads legacy project paths and writes stable GUIDs.
///
/// Both representations remain JSON strings so existing shader graph data can
/// be loaded without migration errors. New picker selections use the GUID form.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextureSrc {
    value: String,
    is_legacy_path: bool,
}

impl TextureSrc {
    /// Construct a compatibility reference from a project-relative path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            value: path.into(),
            is_legacy_path: true,
        }
    }

    /// Construct a GUID-backed reference for a selected texture asset.
    pub fn from_ref(reference: TextureRef) -> Self {
        Self {
            value: reference.as_str().to_owned(),
            is_legacy_path: false,
        }
    }

    fn from_serialized(value: String) -> Self {
        let is_legacy_path = TextureRef::parse(&value).is_none();
        Self {
            value,
            is_legacy_path,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn asset_ref(&self) -> Option<TextureRef> {
        (!self.is_legacy_path)
            .then(|| TextureRef::parse(&self.value))
            .flatten()
    }

    pub fn legacy_path(&self) -> Option<&str> {
        self.is_legacy_path.then_some(self.value.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

impl From<String> for TextureSrc {
    fn from(path: String) -> Self {
        Self::new(path)
    }
}

impl From<&str> for TextureSrc {
    fn from(path: &str) -> Self {
        Self::new(path)
    }
}

impl std::fmt::Display for TextureSrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}

impl Serialize for TextureSrc {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.value)
    }
}

impl<'de> Deserialize<'de> for TextureSrc {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from_serialized)
    }
}

fn serialize_texture_src(value: &TextureSrc) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::Value::String(value.value.clone()))
}

fn deserialize_texture_src(value: serde_json::Value) -> crate::ReflectResult<TextureSrc> {
    value
        .as_str()
        .map(|value| TextureSrc::from_serialized(value.to_owned()))
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
