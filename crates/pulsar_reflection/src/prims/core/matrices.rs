//! Flat fixed-size arrays used by renderer component reflection. The
//! nested `[[f32; 4]; N]` matrices are registered in `mat4x3.rs` and
//! `mat4x4.rs`.

use crate::pulsar_type;

#[pulsar_type(
    serialize_json_with = serialize_f32x16_json,
    deserialize_json_with = deserialize_f32x16_json
)]
type RegisteredF32x16 = [f32; 16];

#[pulsar_type(
    serialize_json_with = serialize_f32x12_json,
    deserialize_json_with = deserialize_f32x12_json
)]
type RegisteredF32x12 = [f32; 12];

#[pulsar_type(serialize_json_with = serialize_f32x2_json, deserialize_json_with = deserialize_f32x2_json)]
type RegisteredF32x2 = [f32; 2];

#[pulsar_type(
    serialize_json_with = serialize_u32x3_json,
    deserialize_json_with = deserialize_u32x3_json
)]
type RegisteredU32x3 = [u32; 3];

#[pulsar_type(
    serialize_json_with = serialize_u32x116_json,
    deserialize_json_with = deserialize_u32x116_json
)]
type RegisteredU32x116 = [u32; 116];

fn serialize_u32x116_json(value: &[u32; 116]) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::Value::Array(
        value.iter().copied().map(serde_json::Value::from).collect(),
    ))
}

fn deserialize_u32x116_json(value: serde_json::Value) -> crate::ReflectResult<[u32; 116]> {
    let values = value.as_array().ok_or_else(|| crate::ReflectError::TypeMismatch {
        expected: "[u32; 116]",
        found: value.to_string(),
    })?;
    if values.len() != 116 {
        return Err(crate::ReflectError::TypeMismatch {
            expected: "[u32; 116]",
            found: format!("array length {}", values.len()),
        });
    }

    let mut result = [0u32; 116];
    for (index, item) in values.iter().enumerate() {
        result[index] = item.as_u64().and_then(|number| u32::try_from(number).ok()).ok_or_else(|| {
            crate::ReflectError::TypeMismatch {
                expected: "u32",
                found: item.to_string(),
            }
        })?;
    }
    Ok(result)
}
fn serialize_f32x16_json(value: &[f32; 16]) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(value))
}

fn serialize_f32x12_json(value: &[f32; 12]) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(value))
}

fn deserialize_f32x16_json(value: serde_json::Value) -> crate::ReflectResult<[f32; 16]> {
    serde_json::from_value(value).map_err(|error| crate::ReflectError::TypeMismatch {
        expected: "[f32; 16]",
        found: error.to_string(),
    })
}

fn deserialize_f32x12_json(value: serde_json::Value) -> crate::ReflectResult<[f32; 12]> {
    serde_json::from_value(value).map_err(|error| crate::ReflectError::TypeMismatch {
        expected: "[f32; 12]",
        found: error.to_string(),
    })
}
fn serialize_f32x2_json(value: &[f32; 2]) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(value))
}

fn deserialize_f32x2_json(value: serde_json::Value) -> crate::ReflectResult<[f32; 2]> {
    serde_json::from_value(value).map_err(|error| crate::ReflectError::TypeMismatch {
        expected: "[f32; 2]",
        found: error.to_string(),
    })
}

fn serialize_u32x3_json(value: &[u32; 3]) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(value))
}

fn deserialize_u32x3_json(value: serde_json::Value) -> crate::ReflectResult<[u32; 3]> {
    serde_json::from_value(value).map_err(|error| crate::ReflectError::TypeMismatch {
        expected: "[u32; 3]",
        found: error.to_string(),
    })
}