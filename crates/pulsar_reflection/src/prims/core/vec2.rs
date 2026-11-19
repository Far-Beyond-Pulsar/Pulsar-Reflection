//! [f32; 2] primitive type implementation (Vec2): JSON only.
//!
//! A pair of numbers a component keeps (a move input, an axis) is a property
//! scripts and other plugins read and write by name; without this entry the
//! reflected get/set of such a property fails with "type not registered".
//! There is no inspector row yet: the value is written by scripts and
//! systems.

// `[f32; 2]` is reflected by the array impls (`std::wrappers`) but nothing gave
// it a JSON form; register one. The registration is the registry's, as the
// `#[pulsar_type]` macro makes for the types it aliases.
::inventory::submit! {
    crate::RuntimeTypeRegistration {
        type_info: <[f32; 2] as crate::Reflectable>::type_info,
        serialize_json: |value| match value.downcast_ref::<[f32; 2]>() {
            Some(typed) => serialize_vec2_json(typed),
            None => Err(crate::ReflectError::SerializationFailed("expected [f32; 2]".into())),
        },
        deserialize_json: |value| {
            deserialize_vec2_json(value).map(|v| Box::new(v) as Box<dyn std::any::Any>)
        },
    }
}

fn serialize_vec2_json(value: &[f32; 2]) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!([value[0], value[1]]))
}

fn deserialize_vec2_json(value: serde_json::Value) -> crate::ReflectResult<[f32; 2]> {
    let arr = value
        .as_array()
        .ok_or_else(|| crate::ReflectError::TypeMismatch {
            expected: "[f32; 2]",
            found: format!("{:?}", value),
        })?;
    if arr.len() != 2 {
        return Err(crate::ReflectError::TypeMismatch {
            expected: "[f32; 2]",
            found: format!("array of length {}", arr.len()),
        });
    }
    Ok([
        arr[0].as_f64().unwrap_or(0.0) as f32,
        arr[1].as_f64().unwrap_or(0.0) as f32,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pair_round_trips_as_json() {
        let json = serialize_vec2_json(&[0.5, -1.0]).unwrap();
        assert_eq!(json, serde_json::json!([0.5, -1.0]));
        assert_eq!(deserialize_vec2_json(json).unwrap(), [0.5, -1.0]);
        assert!(deserialize_vec2_json(serde_json::json!([1.0])).is_err());
    }
}
