# Pulsar Reflection

Runtime type metadata for the Pulsar engine. Think Unreal UPROPERTY, but Rust.

## What it does

At compile time, captures type information — field names, types, offsets, constraints — and makes it available at runtime. You can inspect any reflected type, serialize/deserialize it without serde's derive, generate UI editors automatically, or build blueprint-callable methods on top of it.

Registration is automatic. Uses `inventory` under the hood — types register themselves at link time. No startup code, no manual registry population.

## Two crates

- `pulsar_reflection` — the library: types, traits, registries, JSON codec
- `pulsar_reflection_derive` — the proc macros: `#[derive(Reflectable)]`, `#[pulsar_type]`

## Getting started

### Add a Reflectable derive to your struct

```rust
use pulsar_reflection::Reflectable;

#[derive(Reflectable, Default)]
pub struct PhysicsConfig {
    #[reflect(name = "Gravity", min = 0.0, max = 100.0, category = "Physics")]
    pub gravity: f32,

    #[reflect(name = "Friction")]
    pub friction: f32,
}
```

Available `#[reflect(...)]` attributes on fields:

- `name` — display name in UI
- `category` — collapsible section header
- `category_color` — hex colour for header
- `category_default_collapsed` — start collapsed
- `min`, `max`, `step` — numeric constraints
- `editor` — custom render function
- `skip` — exclude from reflection

### Register a primitive with #[pulsar_type]

```rust
use pulsar_reflection::pulsar_type;

#[pulsar_type(
    serialize_json_with = serialize_my_type,
    deserialize_json_with = deserialize_my_type,
)]
type RegisteredMyType = MyType;
```

Arguments:

- `serialize_json_with` (required) — `fn(&T) -> ReflectResult<Value>`
- `deserialize_json_with` (required) — `fn(Value) -> ReflectResult<T>`
- `editor` (optional) — GPUI render function for the inspector
- `color` (optional) — hex colour for the type debugger

### Look up a type at runtime

```rust
use pulsar_reflection::RUNTIME_TYPE_REGISTRY;

let info = RUNTIME_TYPE_REGISTRY.get::<f32>().unwrap();
println!("f32: {} bytes, {} alignment", info.size, info.align);
```

### Serialize / deserialize via reflection

```rust
use pulsar_reflection::{JsonSerializer, JsonDeserializer, Reflectable};

let mut s = JsonSerializer::new();
42.0f32.serialize(&mut s).unwrap();

let mut d = JsonDeserializer::new(s.as_json());
let v = f32::deserialize(&mut d).unwrap();
```

## Feature flags

| Feature | Default | What it adds |
|---|---|---|
| `prims-core` | yes | bool, f32, i32, i64, u64, vec3, colour |
| `prims-std` | yes | String, Vec&lt;T&gt;, Option&lt;T&gt; |
| `prims-serde` | no | serde_json::Value |
| `prims-glam` | yes | glam::Mat4 |
| `prims-gpui` | no | GPUI property editors for core types |

```toml
# minimal
pulsar_reflection = { git = "https://github.com/Far-Beyond-Pulsar/Pulsar-Reflection" }

# with editors
pulsar_reflection = { git = "https://github.com/Far-Beyond-Pulsar/Pulsar-Reflection", features = ["prims-gpui"] }
```

## Project structure

```
src/
├── lib.rs                   # re-exports, PropertyEditorArgs, Subsystems, EngineClass, behaviour traits
├── registry.rs              # EngineClass registry (legacy)
├── runtime_types.rs         # RuntimeTypeInfo, TypeStructure, FieldInfo
├── runtime_registry.rs      # inventory-based type registry
├── type_traits.rs           # Reflectable, TypeSerializer, TypeDeserializer
├── type_renderer.rs         # custom type renderers for the property panel
├── dynamic_types.rs         # runtime-composed types from compile-time components
├── json_codec.rs            # JsonSerializer / JsonDeserializer
└── prims/
    ├── mod.rs               # feature-gated module tree
    ├── core/                # bool, f32, i32, i64, u64, [f32;3], [f32;4]
    ├── std/                 # String, Vec<T>, Option<T>
    ├── serde/               # serde_json::Value
    ├── glam/                # Mat4
    └── helio/               # helio::Movability
```

## Who uses it

- **Pulsar Native** — component property inspection, the inspector UI, scene-snapshot data flow
- **SceneDB** — typed property storage in the spatial database
- **pulsar_rendering** — per-component property editor registrations
- **pulsar_scene** — scene file serialization
