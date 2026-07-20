# Pulsar Reflection

Runtime type introspection, serialization, and property-editor infrastructure for the Pulsar Engine — similar to Unreal's UPROPERTY system.

## Overview

`pulsar_reflection` provides compile-time captured type metadata that enables automatic UI generation, JSON serialization, blueprint-callable methods, and runtime type inspection without runtime registration overhead.

The system is split into two crates:

| Crate | Role |
|---|---|
| `pulsar_reflection` | Library — type descriptors, registries, codec, traits |
| `pulsar_reflection_derive` | Proc-macro — `#[derive(Reflectable)]`, `#[pulsar_type]` |

Registration uses the [`inventory`](https://github.com/dtolnay/inventory) crate for zero-cost link-time type collection — types self-register without any explicit startup code.

## Architecture

```
┌─────────────────────────────────────────────┐
│           pulsar_reflection_derive          │
│  #[derive(Reflectable)]  #[pulsar_type]     │
└──────────────────┬──────────────────────────┘
                   │ generates
                   ▼
┌─────────────────────────────────────────────┐
│           pulsar_reflection                  │
│                                             │
│  Type descriptors: RuntimeTypeInfo,         │
│    TypeStructure, FieldInfo                 │
│  Registries: RuntimeTypeRegistry,           │
│    EngineClassRegistry, TypeRendererRegistry│
│  Codec: JsonSerializer / JsonDeserializer   │
│  Editing: PropertyEditorArgs, TypeRenderer  │
│  Dynamic types: DynamicTypeBuilder,         │
│    DynamicValue                             │
│  Traits: Reflectable, TypeSerializer,       │
│    TypeDeserializer                         │
│  Prims: built-in type registrations for     │
│    bool, f32, i32, String, Vec, Option,     │
│    glam types, serde_json::Value, etc.      │
└─────────────────────────────────────────────┘
```

## Core Types & Traits

### Reflection primitives

| Type | Description |
|---|---|
| `RuntimeTypeInfo` | Describes a single reflected type (name, size, alignment, structure kind) |
| `TypeStructure` | Enum: `Primitive`, `String`, `Wrapper(T)`, `Struct{fields}`, `Enum{variants}`, `Wildcard` |
| `FieldInfo` | Describes one field in a struct (name, type, category, constraints) |

### Key traits

| Trait | Derive | Purpose |
|---|---|---|
| `Reflectable` | ✅ | Serialize/deserialize a value by its `RuntimeTypeInfo` |
| `TypeSerializer` | — | Write a reflected value into a serialization format |
| `TypeDeserializer` | — | Read a reflected value from a serialization format |
| `TypeRenderer` | — | Render a reflected value into a GPUI element |

### Registries

| Registry | Content | Collection mechanism |
|---|---|---|
| `RUNTIME_TYPE_REGISTRY` | All `Reflectable` types | `inventory` |
| `ENGINE_CLASS_REGISTRY` | Engine component classes | `inventory` |
| `TYPE_RENDERER_REGISTRY` | Custom property editors | Manual insertion |
| `DYNAMIC_TYPE_REGISTRY` | Runtime-composed types | Manual insertion |

## Usage

### Derive on user structs or enums

```rust
use pulsar_reflection::Reflectable;

#[derive(Reflectable, Default)]
pub struct PhysicsConfig {
    #[reflect(name = "Gravity", category = "Physics", min = 0.0, max = 100.0)]
    pub gravity: f32,

    #[reflect(name = "Friction Coefficient")]
    pub friction: f32,
}
```

Supported `#[reflect(...)]` attributes:

| Attribute | Applies to | Purpose |
|---|---|---|
| `name` | Fields | Display name in UI |
| `category` | Fields | Collapsible section header |
| `category_color` | Fields | Hex colour for the header |
| `category_default_collapsed` | Fields | Start collapsed |
| `min` / `max` | Numeric fields | Clamp range |
| `step` | Numeric fields | UI slider step |
| `editor` | Fields | Custom render function path |
| `skip` | Fields | Exclude from reflection |

### Register primitives with `#[pulsar_type]`

```rust
use pulsar_reflection::pulsar_type;

#[pulsar_type(
    serialize_json_with = serialize_my_type,
    deserialize_json_with = deserialize_my_type,
)]
type RegisteredMyType = MyType;
```

Arguments:

| Argument | Required | Purpose |
|---|---|---|
| `serialize_json_with` | ✅ | `fn(&T) -> ReflectResult<Value>` |
| `deserialize_json_with` | ✅ | `fn(Value) -> ReflectResult<T>` |
| `editor` | — | GPUI render function `fn(&PropertyEditorArgs, &App) -> AnyElement` |
| `color` | — | Hex colour for the type in the type debugger |

### Query types at runtime

```rust
use pulsar_reflection::RUNTIME_TYPE_REGISTRY;

if let Some(info) = RUNTIME_TYPE_REGISTRY.get::<f32>() {
    println!("f32: {} bytes, alignment {}", info.size, info.align);
}
```

### Serialize / deserialize

```rust
use pulsar_reflection::{JsonSerializer, JsonDeserializer, Reflectable};

let value = 42.0f32;

let mut serializer = JsonSerializer::new();
value.serialize(&mut serializer).unwrap();
let json = serializer.as_json();

let mut deserializer = JsonDeserializer::new(json);
let result = f32::deserialize(&mut deserializer).unwrap();
```

## Feature Flags

| Feature | Default | Primitives enabled | Dependencies |
|---|---|---|---|
| `prims-core` | ✅ | `bool`, `f32`, `i32`, `i64`, `u64`, `[f32;3]`, `[f32;4]` | — |
| `prims-std` | ✅ | `String`, `Vec<T>`, `Option<T>` | `prims-core` |
| `prims-serde` | — | `serde_json::Value` | `prims-std` |
| `prims-glam` | ✅ | `glam::Mat4` | `prims-core`, `glam` |
| `prims-gpui` | — | GPUI property editors for core primitives | `prims-core`, `gpui-ce`, `ui` |

```toml
# Minimal — just primitives
pulsar_reflection = { git = "https://github.com/Far-Beyond-Pulsar/Pulsar-Reflection" }

# Full editing support
pulsar_reflection = { git = "https://github.com/Far-Beyond-Pulsar/Pulsar-Reflection", features = ["prims-gpui"] }
```

## Module map

```
src/
├── lib.rs                   # Re-exports, PropertyEditorArgs, Subsystems, EngineClass, ComponentRuntimeBehavior
├── registry.rs              # EngineClass registry (legacy)
├── runtime_types.rs         # RuntimeTypeInfo, TypeStructure, FieldInfo, colour parsing
├── runtime_registry.rs      # Inventory-based RuntimeTypeRegistry
├── type_traits.rs           # Reflectable, TypeSerializer, TypeDeserializer, ReflectError
├── type_renderer.rs         # TypeRenderer trait + global registry
├── dynamic_types.rs         # Runtime-composed types
├── json_codec.rs            # JsonSerializer / JsonDeserializer
└── prims/
    ├── mod.rs               # Feature-gated module tree
    ├── core/                # bool, f32, i32, i64, u64, [f32;3], [f32;4]
    ├── std/                 # String, Vec<T>, Option<T>
    ├── serde/               # serde_json::Value
    ├── glam/                # glam::Mat4
    └── helio/               # helio::Movability
```

## Ecosystem

`pulsar_reflection` is consumed by:

- **Pulsar Native** — The main engine workspace; uses `Reflectable` derive for component properties, `PropertyEditorArgs` for the inspector UI, and `ScenePropsProjector` for scene-snapshot data flow.
- **SceneDB** — Uses `RuntimeTypeRegistry` for typed property storage in the spatial database.
- **pulsar_rendering** — Registers per-component property editors via `TypeRendererRegistry`.
- **pulsar_scene** — Uses `Reflectable` to serialize/deserialize component data in scene files.
