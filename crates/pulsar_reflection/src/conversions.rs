//! Runtime-discoverable, type-erased conversions between reflected values.
//!
//! Rust does not expose arbitrary `From`/`Into` implementations for runtime
//! inspection. Conversions therefore enter this registry through inventory
//! registrations (usually the [`register_conversion!`] macro). This keeps the
//! conversion implementation statically typed while letting tools such as the
//! Blueprint editor enumerate the available edges at runtime.

use crate::{ReflectResult, RUNTIME_TYPE_REGISTRY};
use inventory;
use once_cell::sync::Lazy;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Link-time registration for one conversion edge.
pub struct ConversionRegistration {
    pub source_type_id: fn() -> TypeId,
    pub target_type_id: fn() -> TypeId,
    pub source_type_name: fn() -> &'static str,
    pub target_type_name: fn() -> &'static str,
    pub convert: fn(Box<dyn Any>) -> ReflectResult<Box<dyn Any>>,
    /// A stable identifier suitable for serialized graph node definitions.
    pub id: &'static str,
    /// `INTO` for infallible `From`; `FROM` for fallible `TryFrom`.
    pub label: &'static str,
}

inventory::collect!(ConversionRegistration);

/// Metadata for an available reflected conversion.
#[derive(Clone, Copy, Debug)]
pub struct ConversionInfo {
    pub id: &'static str,
    pub source_type_name: &'static str,
    pub target_type_name: &'static str,
    pub label: &'static str,
    source_type_id: TypeId,
    target_type_id: TypeId,
    convert: fn(Box<dyn Any>) -> ReflectResult<Box<dyn Any>>,
}

impl ConversionInfo {
    pub fn source_type_id(&self) -> TypeId {
        self.source_type_id
    }
    pub fn target_type_id(&self) -> TypeId {
        self.target_type_id
    }

    /// Convert a type-erased value. The result is always the target's concrete
    /// Rust type and failures (including checked numeric overflow) are explicit.
    pub fn convert(&self, value: Box<dyn Any>) -> ReflectResult<Box<dyn Any>> {
        (self.convert)(value)
    }
}

/// Registry of compile-time registered conversion edges.
pub struct ConversionRegistry {
    by_id: HashMap<&'static str, ConversionInfo>,
    by_types: HashMap<(TypeId, TypeId), &'static str>,
}

impl ConversionRegistry {
    fn new() -> Self {
        let mut by_id = HashMap::new();
        let mut by_types = HashMap::new();
        for registration in inventory::iter::<ConversionRegistration> {
            let source_type_id = (registration.source_type_id)();
            let target_type_id = (registration.target_type_id)();
            // Duplicate ids or edges are ambiguous and must not depend on link
            // order. Keep the first registration and emit a useful diagnostic.
            if by_id.contains_key(registration.id)
                || by_types.contains_key(&(source_type_id, target_type_id))
            {
                tracing::error!(
                    conversion = registration.id,
                    "duplicate conversion registration ignored"
                );
                continue;
            }
            if RUNTIME_TYPE_REGISTRY.get_by_id(source_type_id).is_none()
                || RUNTIME_TYPE_REGISTRY.get_by_id(target_type_id).is_none()
            {
                tracing::error!(
                    conversion = registration.id,
                    "conversion references an unreflected type; registration ignored"
                );
                continue;
            }
            by_types.insert((source_type_id, target_type_id), registration.id);
            by_id.insert(
                registration.id,
                ConversionInfo {
                    id: registration.id,
                    source_type_name: (registration.source_type_name)(),
                    target_type_name: (registration.target_type_name)(),
                    label: registration.label,
                    source_type_id,
                    target_type_id,
                    convert: registration.convert,
                },
            );
        }
        Self { by_id, by_types }
    }

    pub fn get(&self, id: &str) -> Option<&ConversionInfo> {
        self.by_id.get(id)
    }

    pub fn between(&self, source: TypeId, target: TypeId) -> Option<&ConversionInfo> {
        self.by_types
            .get(&(source, target))
            .and_then(|id| self.by_id.get(id))
    }

    pub fn between_types<S: 'static, T: 'static>(&self) -> Option<&ConversionInfo> {
        self.between(TypeId::of::<S>(), TypeId::of::<T>())
    }

    pub fn iter(&self) -> impl Iterator<Item = &ConversionInfo> {
        self.by_id.values()
    }
}

pub static CONVERSION_REGISTRY: Lazy<ConversionRegistry> = Lazy::new(ConversionRegistry::new);

/// Register an infallible conversion backed by `From<Source> for Target`.
/// The implementation itself is checked by Rust at compile time.
#[macro_export]
macro_rules! register_conversion {
    ($source:ty => $target:ty, id = $id:literal) => {
        const _: () = {
            fn source_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<$source>()
            }
            fn target_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<$target>()
            }
            fn source_name() -> &'static str {
                ::std::any::type_name::<$source>()
            }
            fn target_name() -> &'static str {
                ::std::any::type_name::<$target>()
            }
            fn convert(
                value: Box<dyn::std::any::Any>,
            ) -> $crate::ReflectResult<Box<dyn::std::any::Any>> {
                let value = value.downcast::<$source>().map_err(|_| {
                    $crate::ReflectError::TypeMismatch {
                        expected: ::std::any::type_name::<$source>(),
                        found: "different source value".to_string(),
                    }
                })?;
                let converted: $target = <$target as ::std::convert::From<$source>>::from(*value);
                Ok(Box::new(converted))
            }
            ::inventory::submit! {
                $crate::ConversionRegistration {
                    source_type_id: source_id, target_type_id: target_id,
                    source_type_name: source_name, target_type_name: target_name,
                    convert, id: $id, label: "INTO",
                }
            }
        };
    };
}

/// Register a checked conversion backed by `TryFrom<Source> for Target`.
#[macro_export]
macro_rules! register_try_conversion {
    ($source:ty => $target:ty, id = $id:literal) => {
        const _: () = {
            fn source_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<$source>()
            }
            fn target_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<$target>()
            }
            fn source_name() -> &'static str {
                ::std::any::type_name::<$source>()
            }
            fn target_name() -> &'static str {
                ::std::any::type_name::<$target>()
            }
            fn convert(
                value: Box<dyn::std::any::Any>,
            ) -> $crate::ReflectResult<Box<dyn::std::any::Any>> {
                let value = value.downcast::<$source>().map_err(|_| {
                    $crate::ReflectError::TypeMismatch {
                        expected: ::std::any::type_name::<$source>(),
                        found: "different source value".to_string(),
                    }
                })?;
                let converted = <$target as ::std::convert::TryFrom<$source>>::try_from(*value)
                    .map_err(|error| $crate::ReflectError::Custom(format!("{}", error)))?;
                Ok(Box::new(converted))
            }
            ::inventory::submit! {
                $crate::ConversionRegistration {
                    source_type_id: source_id, target_type_id: target_id,
                    source_type_name: source_name, target_type_name: target_name,
                    convert, id: $id, label: "FROM",
                }
            }
        };
    };
}

// These are explicit built-in registrations; Rust cannot enumerate every
// downstream `From` impl without an annotation at its definition site.
register_conversion!(i32 => i64, id = "numeric.i32_to_i64");
register_try_conversion!(i64 => i32, id = "numeric.i64_to_i32_checked");
