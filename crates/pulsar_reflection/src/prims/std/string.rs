//! String type implementation

use crate::pulsar_type;

fn serialize_string_json(value: &String) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(value))
}

fn deserialize_string_json(value: serde_json::Value) -> crate::ReflectResult<String> {
    value
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| crate::ReflectError::TypeMismatch {
            expected: "String",
            found: format!("{:?}", value),
        })
}

#[cfg_attr(
    feature = "prims-gpui",
    pulsar_type(
        serialize_json_with = serialize_string_json,
        deserialize_json_with = deserialize_string_json,
        editor = render_string_editor
    )
)]
#[cfg_attr(
    not(feature = "prims-gpui"),
    pulsar_type(
        serialize_json_with = serialize_string_json,
        deserialize_json_with = deserialize_string_json
    )
)]
#[allow(dead_code)]
type RegisteredString = String;

#[cfg(feature = "prims-gpui")]
fn render_string_editor(args: &crate::PropertyEditorArgs<'_>, cx: &gpui::App) -> gpui::AnyElement {
    use gpui::{prelude::*, *};
    use ui::{ActiveTheme, Sizable, h_flex, input::Input};

    let value = args.current_value.downcast_ref::<String>().cloned().unwrap_or_default();
    h_flex()
        .w_full()
        .justify_between()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(args.display_name.to_string()),
        )
        .child(h_flex().items_center().gap_2().child(
            if let Some(input) = args.get_widget::<gpui::Entity<ui::input::InputState>>() {
                Input::new(&input)
                    .xsmall()
                    .w(gpui::px(160.0))
                    .into_any_element()
            } else {
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(value)
                    .into_any_element()
            },
        ))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use crate::{JsonDeserializer, JsonSerializer, RUNTIME_TYPE_REGISTRY, Reflectable};

    #[test]
    fn test_string_registered() {
        let info = RUNTIME_TYPE_REGISTRY.get::<String>().unwrap();
        assert_eq!(info.type_name, "String");
        assert_eq!(info.size, std::mem::size_of::<String>());
        assert_eq!(info.align, std::mem::align_of::<String>());
    }

    #[test]
    fn test_string_serialization() {
        let value = "Hello, Pulsar!".to_string();
        let mut serializer = JsonSerializer::new();
        value.serialize(&mut serializer).unwrap();

        let json = serializer.as_json();
        assert_eq!(json.as_str().unwrap(), "Hello, Pulsar!");
    }

    #[test]
    fn test_string_deserialization() {
        let json = serde_json::json!("Test String");
        let mut deserializer = JsonDeserializer::new(json);
        let value = String::deserialize(&mut deserializer).unwrap();
        assert_eq!(value, "Test String");
    }

    #[test]
    fn test_string_clone_any() {
        let value = "Cloneable".to_string();
        let boxed = value.clone_any();
        assert_eq!(*boxed.downcast::<String>().unwrap(), "Cloneable");
    }

    #[test]
    fn test_empty_string() {
        let value = String::new();
        let mut serializer = JsonSerializer::new();
        value.serialize(&mut serializer).unwrap();

        let json = serializer.as_json();
        assert_eq!(json.as_str().unwrap(), "");
    }

    #[test]
    fn test_unicode_string() {
        let value = "Hello 世界 🌍".to_string();
        let mut serializer = JsonSerializer::new();
        value.serialize(&mut serializer).unwrap();

        let json = serializer.as_json();
        assert_eq!(json.as_str().unwrap(), "Hello 世界 🌍");
    }
}
