//! `[[f32; 4]; 4]` primitive type implementation: a bare column-major 4x4
//! matrix stored as a raw nested array, distinct from any `glam::Mat4`
//! registration (see `prims/glam`). GPU component fields typically store
//! transforms this way (e.g. `StaticObjectComponent::transform`) rather
//! than as a glam type, since the array is what gets uploaded byte-for-byte.

use crate::pulsar_type;

#[pulsar_type(
    serialize_json_with = serialize_mat4x4_json,
    deserialize_json_with = deserialize_mat4x4_json
)]
type RegisteredMat4x4 = [[f32; 4]; 4];

fn serialize_mat4x4_json(value: &[[f32; 4]; 4]) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(
        value.iter().map(|col| col.to_vec()).collect::<Vec<_>>()
    ))
}

fn deserialize_mat4x4_json(value: serde_json::Value) -> crate::ReflectResult<[[f32; 4]; 4]> {
    let cols = value
        .as_array()
        .ok_or_else(|| crate::ReflectError::TypeMismatch {
            expected: "[[f32; 4]; 4]",
            found: format!("{:?}", value),
        })?;

    if cols.len() != 4 {
        return Err(crate::ReflectError::TypeMismatch {
            expected: "[[f32; 4]; 4]",
            found: format!("array of length {}", cols.len()),
        });
    }

    let mut out = [[0.0f32; 4]; 4];
    for (i, col) in cols.iter().enumerate() {
        let arr = col
            .as_array()
            .ok_or_else(|| crate::ReflectError::TypeMismatch {
                expected: "[f32; 4]",
                found: format!("{:?}", col),
            })?;
        if arr.len() != 4 {
            return Err(crate::ReflectError::TypeMismatch {
                expected: "[f32; 4]",
                found: format!("array of length {}", arr.len()),
            });
        }
        for (j, v) in arr.iter().enumerate() {
            out[i][j] = v.as_f64().unwrap_or(0.0) as f32;
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use crate::{JsonDeserializer, JsonSerializer, RUNTIME_TYPE_REGISTRY, Reflectable};

    #[test]
    fn test_mat4x4_registered() {
        let info = RUNTIME_TYPE_REGISTRY.get::<[[f32; 4]; 4]>().unwrap();
        assert_eq!(info.size, 64);
        assert_eq!(info.align, 4);
    }

    #[test]
    fn test_mat4x4_round_trip() {
        let value: [[f32; 4]; 4] = [
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ];
        let mut serializer = JsonSerializer::new();
        value.serialize(&mut serializer).unwrap();
        let mut deserializer = JsonDeserializer::new(serializer.into_json());
        let round_tripped = <[[f32; 4]; 4]>::deserialize(&mut deserializer).unwrap();
        assert_eq!(round_tripped, value);
    }
}
