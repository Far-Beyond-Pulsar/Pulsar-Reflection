//! u32 primitive type implementation

use crate::pulsar_type;

#[pulsar_type(
    serialize_json_with = serialize_u32_json,
    deserialize_json_with = deserialize_u32_json
)]
type RegisteredU32 = u32;

fn serialize_u32_json(value: &u32) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(*value))
}

fn deserialize_u32_json(value: serde_json::Value) -> crate::ReflectResult<u32> {
    value
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| crate::ReflectError::TypeMismatch {
            expected: "u32",
            found: format!("{:?}", value),
        })
}