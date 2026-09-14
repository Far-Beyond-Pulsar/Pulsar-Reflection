//! `[[f32; 4]; 3]` primitive type implementation: three padded `vec4`
//! columns (`[f32; 4]` each, last lane typically unused padding), the
//! layout `StaticObjectComponent::normal_mat` and similar packed-3x3
//! GPU fields use.

use crate::pulsar_type;

#[pulsar_type(
    serialize_json_with = serialize_mat4x3_json,
    deserialize_json_with = deserialize_mat4x3_json
)]
type RegisteredMat4x3 = [[f32; 4]; 3];

fn serialize_mat4x3_json(value: &[[f32; 4]; 3]) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(
        value.iter().map(|col| col.to_vec()).collect::<Vec<_>>()
    ))
}

fn deserialize_mat4x3_json(value: serde_json::Value) -> crate::ReflectResult<[[f32; 4]; 3]> {
    let cols = value
        .as_array()
        .ok_or_else(|| crate::ReflectError::TypeMismatch {
            expected: "[[f32; 4]; 3]",
            found: format!("{:?}", value),
        })?;

    if cols.len() != 3 {
        return Err(crate::ReflectError::TypeMismatch {
            expected: "[[f32; 4]; 3]",
            found: format!("array of length {}", cols.len()),
        });
    }

    let mut out = [[0.0f32; 4]; 3];
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
    fn test_mat4x3_registered() {
        let info = RUNTIME_TYPE_REGISTRY.get::<[[f32; 4]; 3]>().unwrap();
        assert_eq!(info.size, 48);
        assert_eq!(info.align, 4);
    }

    #[test]
    fn test_mat4x3_round_trip() {
        let value: [[f32; 4]; 3] = [
            [1.0, 2.0, 3.0, 0.0],
            [4.0, 5.0, 6.0, 0.0],
            [7.0, 8.0, 9.0, 0.0],
        ];
        let mut serializer = JsonSerializer::new();
        value.serialize(&mut serializer).unwrap();
        let mut deserializer = JsonDeserializer::new(serializer.into_json());
        let round_tripped = <[[f32; 4]; 3]>::deserialize(&mut deserializer).unwrap();
        assert_eq!(round_tripped, value);
    }
}
