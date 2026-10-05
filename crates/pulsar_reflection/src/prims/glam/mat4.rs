//! glam::Mat4 registration.

use crate::pulsar_type;

fn serialize_mat4_json(value: &glam::Mat4) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(value.to_cols_array()))
}

fn deserialize_mat4_json(value: serde_json::Value) -> crate::ReflectResult<glam::Mat4> {
    let arr = value
        .as_array()
        .ok_or_else(|| crate::ReflectError::TypeMismatch {
            expected: "[f32; 16]",
            found: format!("{:?}", value),
        })?;

    if arr.len() != 16 {
        return Err(crate::ReflectError::TypeMismatch {
            expected: "[f32; 16]",
            found: format!("array length {}", arr.len()),
        });
    }

    let mut cols = [0.0f32; 16];
    for (idx, item) in arr.iter().enumerate() {
        cols[idx] = item.as_f64().unwrap_or(0.0) as f32;
    }

    Ok(glam::Mat4::from_cols_array(&cols))
}

#[pulsar_type(
    serialize_json_with = serialize_mat4_json,
    deserialize_json_with = deserialize_mat4_json
)]
type RegisteredMat4 = glam::Mat4;

#[crate::pulsar_conversion]
fn json_to_mat4(value: serde_json::Value) -> Result<glam::Mat4, serde_json::Error> {
    let columns: [f32; 16] = serde_json::from_value(value)?;
    Ok(glam::Mat4::from_cols_array(&columns))
}

#[crate::pulsar_conversion]
fn mat4_to_json(value: glam::Mat4) -> serde_json::Value {
    serde_json::json!(value.to_cols_array())
}
