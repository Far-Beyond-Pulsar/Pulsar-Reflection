//! Runtime-discoverable, type-erased conversions between reflected values.
//!
//! Rust does not expose arbitrary `From`/`Into` implementations for runtime
//! inspection. Conversions therefore enter this registry through inventory
//! registrations (through [`register_conversion!`],
//! [`register_try_conversion!`], or [`pulsar_conversion`]). This keeps the
//! conversion implementation statically typed while letting tools such as the
//! Blueprint editor enumerate the available edges at runtime.

use crate::{ReflectResult, RUNTIME_TYPE_REGISTRY};
use inventory;
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
    /// The registry of `registrations`: this copy's `inventory` collection,
    /// extended by attached copies' (see [`crate::runtime`]).
    pub(crate) fn from_registrations(registrations: &[&'static ConversionRegistration]) -> Self {
        let mut by_id = HashMap::new();
        let mut by_types = HashMap::new();
        for &registration in registrations {
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
            let Some(source_type) = RUNTIME_TYPE_REGISTRY.get_by_id(source_type_id) else {
                tracing::error!(
                    conversion = registration.id,
                    "conversion source type is unreflected; registration ignored"
                );
                continue;
            };
            let Some(target_type) = RUNTIME_TYPE_REGISTRY.get_by_id(target_type_id) else {
                tracing::error!(
                    conversion = registration.id,
                    "conversion target type is unreflected; registration ignored"
                );
                continue;
            };
            by_types.insert((source_type_id, target_type_id), registration.id);
            by_id.insert(
                registration.id,
                ConversionInfo {
                    id: registration.id,
                    // Pin types use Reflection's stable names, not Rust's
                    // implementation paths (for example, String vs.
                    // alloc::string::String).
                    source_type_name: source_type.type_name,
                    target_type_name: target_type.type_name,
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

/// Shared by every linked copy of this crate (see [`crate::runtime`]).
pub static CONVERSION_REGISTRY: crate::runtime::Shared<ConversionRegistry> =
    crate::runtime::Shared::new(|runtime| (runtime.conversions)());

/// Register an arbitrary conversion function with runtime reflection.
///
/// Put this on a free function taking one owned source value. A direct return
/// value is an infallible `INTO` conversion; `Result<Target, Error>` is a
/// checked `FROM` conversion and requires `Error: Display`. If omitted, the
/// registry ID is the function's module path and name.
///
/// ```ignore
/// #[pulsar_reflection::pulsar_conversion(id = "units.meters_to_feet")]
/// fn meters_to_feet(meters: Meters) -> Feet {
///     Feet(meters.0 * 3.28084)
/// }
///
/// #[pulsar_reflection::pulsar_conversion]
/// fn feet_to_meters(feet: Feet) -> Result<Meters, UnitError> {
///     // Custom validation and conversion logic.
/// }
/// ```
///
/// Both source and target types must be registered with Pulsar Reflection.
/// Generic, async, unsafe, reference-taking, and multi-argument functions are
/// rejected because they cannot represent one concrete registry edge.
pub use pulsar_reflection_derive::pulsar_conversion;

/// Register an infallible conversion backed by `Into<Target>` (including the
/// blanket implementation generated by `From<Source> for Target`). The trait
/// implementation itself is checked by Rust at compile time.
///
/// Use this beside the reflected type's registration so conversion ownership
/// stays with the type/module that defines it:
///
/// ```ignore
/// impl From<LocalPosition> for WorldPosition { /* ... */ }
/// pulsar_reflection::register_conversion!(
///     LocalPosition => WorldPosition,
///     id = "position.local_to_world",
/// );
/// ```
///
/// Both types must be registered with Pulsar Reflection. Rust does not expose
/// arbitrary trait implementations for automatic runtime enumeration, so each
/// conversion edge needs one registration invocation. This also accepts a
/// direct `Into<Target>` implementation, though implementing `From` remains
/// the idiomatic choice when the target type is yours.
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
                value: Box<dyn ::std::any::Any>,
            ) -> $crate::ReflectResult<Box<dyn ::std::any::Any>> {
                let value = value.downcast::<$source>().map_err(|_| {
                    $crate::ReflectError::TypeMismatch {
                        expected: ::std::any::type_name::<$source>(),
                        found: "different source value".to_string(),
                    }
                })?;
                let converted: $target = <$source as ::std::convert::Into<$target>>::into(*value);
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
/// The source value is moved into the conversion, so non-`Copy` types work.
/// Keep this invocation beside one of the reflected types involved.
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
                value: Box<dyn ::std::any::Any>,
            ) -> $crate::ReflectResult<Box<dyn ::std::any::Any>> {
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
