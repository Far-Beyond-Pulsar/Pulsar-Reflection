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

#[cfg(test)]
mod tests {
    use crate::{JsonDeserializer, JsonSerializer, RUNTIME_TYPE_REGISTRY, Reflectable};

    #[test]
    fn test_u32_registered() {
        let info = RUNTIME_TYPE_REGISTRY.get::<u32>().unwrap();
        assert_eq!(info.type_name, "u32");
        assert_eq!(info.size, 4);
        assert_eq!(info.align, 4);
    }

    #[test]
    fn test_u32_serialization() {
        let value: u32 = 4294967295;
        let mut serializer = JsonSerializer::new();
        value.serialize(&mut serializer).unwrap();
        assert_eq!(serializer.as_json().as_u64().unwrap(), value as u64);
    }

    #[test]
    fn test_u32_deserialization() {
        let json = serde_json::json!(42u32);
        let mut deserializer = JsonDeserializer::new(json);
        let value = u32::deserialize(&mut deserializer).unwrap();
        assert_eq!(value, 42);
    }
}
