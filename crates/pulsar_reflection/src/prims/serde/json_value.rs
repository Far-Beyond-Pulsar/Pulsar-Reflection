//! serde_json::Value primitive type implementation

use crate::pulsar_type;

fn serialize_json_value_json(value: &serde_json::Value) -> crate::ReflectResult<serde_json::Value> {
    Ok(value.clone())
}

fn deserialize_json_value_json(
    value: serde_json::Value,
) -> crate::ReflectResult<serde_json::Value> {
    Ok(value)
}

#[pulsar_type(
    serialize_json_with = serialize_json_value_json,
    deserialize_json_with = deserialize_json_value_json
)]
#[allow(dead_code)]
type RegisteredJsonValue = serde_json::Value;

#[crate::pulsar_conversion(id = "json.parse_string")]
fn parse_json_string(value: String) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(&value)
}

#[crate::pulsar_conversion(id = "json.stringify")]
fn stringify_json(value: serde_json::Value) -> String {
    value.to_string()
}

#[crate::pulsar_conversion]
fn json_to_bool(value: serde_json::Value) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| "JSON value is not a boolean".to_string())
}

#[crate::pulsar_conversion]
fn json_to_i32(value: serde_json::Value) -> Result<i32, String> {
    let value = value
        .as_i64()
        .ok_or_else(|| "JSON value is not a signed integer".to_string())?;
    i32::try_from(value).map_err(|_| "JSON integer is outside the i32 range".to_string())
}

#[crate::pulsar_conversion]
fn json_to_i64(value: serde_json::Value) -> Result<i64, String> {
    value
        .as_i64()
        .ok_or_else(|| "JSON value is not an i64 integer".to_string())
}

#[crate::pulsar_conversion]
fn json_to_u32(value: serde_json::Value) -> Result<u32, String> {
    let value = value
        .as_u64()
        .ok_or_else(|| "JSON value is not an unsigned integer".to_string())?;
    u32::try_from(value).map_err(|_| "JSON integer is outside the u32 range".to_string())
}

#[crate::pulsar_conversion]
fn json_to_u64(value: serde_json::Value) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or_else(|| "JSON value is not a u64 integer".to_string())
}

#[crate::pulsar_conversion]
fn json_to_f32(value: serde_json::Value) -> Result<f32, String> {
    let value = value
        .as_f64()
        .ok_or_else(|| "JSON value is not a number".to_string())?;
    if value < -(f32::MAX as f64) || value > f32::MAX as f64 {
        return Err("JSON number is outside the f32 range".to_string());
    }
    Ok(value as f32)
}

#[crate::pulsar_conversion]
fn json_to_f64(value: serde_json::Value) -> Result<f64, String> {
    value
        .as_f64()
        .ok_or_else(|| "JSON value is not a number".to_string())
}

#[crate::pulsar_conversion]
fn json_to_vec3(value: serde_json::Value) -> Result<[f32; 3], serde_json::Error> {
    serde_json::from_value(value)
}

#[crate::pulsar_conversion]
fn vec3_to_json(value: [f32; 3]) -> serde_json::Value {
    serde_json::json!(value)
}

#[crate::pulsar_conversion]
fn json_to_color(value: serde_json::Value) -> Result<[f32; 4], serde_json::Error> {
    serde_json::from_value(value)
}

#[crate::pulsar_conversion]
fn color_to_json(value: [f32; 4]) -> serde_json::Value {
    serde_json::json!(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        JsonDeserializer, JsonSerializer, Reflectable, TypeStructure, RUNTIME_TYPE_REGISTRY,
    };

    #[test]
    fn test_json_value_registered() {
        let info = RUNTIME_TYPE_REGISTRY.get::<serde_json::Value>().unwrap();
        assert_eq!(info.type_name, "serde_json :: Value");
        assert!(matches!(info.structure, TypeStructure::Primitive));
    }

    #[test]
    fn test_json_value_round_trip() {
        let value = serde_json::json!({
            "name": "Pulsar",
            "items": [1, true, "x"],
            "nested": { "ok": true }
        });

        let mut serializer = JsonSerializer::new();
        value.serialize(&mut serializer).unwrap();
        assert_eq!(serializer.as_json(), &value);

        let mut deserializer = JsonDeserializer::new(value.clone());
        let restored = serde_json::Value::deserialize(&mut deserializer).unwrap();
        assert_eq!(restored, value);
    }
}
