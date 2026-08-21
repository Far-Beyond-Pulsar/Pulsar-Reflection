//! Proves `#[derive(Reflectable)]` works on an ordinary struct with fields --
//! the exact shape this crate's own README documents as the basic usage
//! example (`PhysicsConfig { gravity: f32, friction: f32 }`).
//!
//! Before the `RuntimeTypeRegistration::type_info`/lazy-`type_info()` fix,
//! this failed to even COMPILE: `struct_impl.rs`'s generated code built
//! each type's `RuntimeTypeInfo` as a plain `static ... = RuntimeTypeInfo {
//! .. };`, and `FieldInfo`'s own construction (`field_info.rs`) embeds
//! `<FieldTy as Reflectable>::type_info()` -- an ordinary, non-`const`
//! trait method call -- directly inside that literal. Rust rejects any
//! non-const call inside a `static` initializer, for ANY field type,
//! including a bare `f32` (`Reflectable::type_info` isn't `const fn`
//! either). This is exactly the derive usage this test exercises.

use pulsar_reflection::Reflectable;

#[derive(Clone, Debug, Default, PartialEq, Reflectable)]
pub struct DeriveReflectableTestConfig {
    pub gravity: f32,
    pub friction: f32,
}

#[test]
fn deriving_reflectable_on_a_struct_with_plain_fields_compiles_and_reports_both_fields() {
    let type_info = DeriveReflectableTestConfig::type_info();
    assert_eq!(type_info.type_name, "DeriveReflectableTestConfig");

    let fields = type_info.fields().expect("a named-field struct must report Some(fields)");
    let names: Vec<&str> = fields.iter().map(|f| f.name).collect();
    assert_eq!(names, vec!["gravity", "friction"]);

    // Each field's own nested type_info is real, resolved f32 info -- proof
    // the deferred (OnceLock) construction actually ran the recursive
    // `<f32 as Reflectable>::type_info()` call, not a stub/placeholder.
    for field in fields {
        assert_eq!(field.type_info.type_name, "f32");
    }
}

#[test]
fn type_info_is_cached_not_recomputed_on_every_call() {
    // Same pointer every time -- OnceLock's whole point (and, incidentally,
    // proof this isn't secretly leaking a fresh Box on every call).
    let a = DeriveReflectableTestConfig::type_info() as *const _;
    let b = DeriveReflectableTestConfig::type_info() as *const _;
    assert_eq!(a, b);
}

#[test]
fn serialize_then_deserialize_round_trips_through_the_derived_impl() {
    let value = DeriveReflectableTestConfig { gravity: -9.8, friction: 0.4 };

    let mut serializer = pulsar_reflection::JsonSerializer::new();
    value.serialize(&mut serializer).expect("serialize");
    let json = serializer.into_json();

    let mut deserializer = pulsar_reflection::JsonDeserializer::new(json);
    let round_tripped =
        DeriveReflectableTestConfig::deserialize(&mut deserializer).expect("deserialize");

    assert_eq!(round_tripped, value);
}

#[test]
fn the_type_registers_itself_in_the_global_runtime_registry_via_inventory() {
    // Proves the OTHER half of the fix: RuntimeTypeRegistration::type_info
    // is now a fn pointer (const-constructible for inventory::submit!'s own
    // static, unlike the old `&'static RuntimeTypeInfo` it replaced), and
    // RuntimeTypeRegistry::new correctly calls through it while building
    // the registry.
    let registered = pulsar_reflection::RUNTIME_TYPE_REGISTRY.get::<DeriveReflectableTestConfig>();
    assert!(registered.is_some(), "derive must auto-register via inventory, same as every other Reflectable type");
    assert_eq!(registered.unwrap().type_name, "DeriveReflectableTestConfig");
}
