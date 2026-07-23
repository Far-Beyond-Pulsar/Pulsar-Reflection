//! u64 primitive type implementation

use crate::pulsar_type;

#[cfg_attr(
    feature = "prims-gpui",
    pulsar_type(
        serialize_json_with = serialize_u64_json,
        deserialize_json_with = deserialize_u64_json,
        editor = render_u64_editor
    )
)]
#[cfg_attr(
    not(feature = "prims-gpui"),
    pulsar_type(
        serialize_json_with = serialize_u64_json,
        deserialize_json_with = deserialize_u64_json
    )
)]
#[allow(dead_code)]
type RegisteredU64 = u64;

fn serialize_u64_json(value: &u64) -> crate::ReflectResult<serde_json::Value> {
    Ok(serde_json::json!(*value))
}

fn deserialize_u64_json(value: serde_json::Value) -> crate::ReflectResult<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::ReflectError::TypeMismatch {
            expected: "u64",
            found: format!("{:?}", value),
        })
}

#[cfg(feature = "prims-gpui")]
fn render_u64_editor(args: &crate::PropertyEditorArgs<'_>, cx: &gpui::App) -> gpui::AnyElement {
    use gpui::{prelude::*, *};
    use ui::{
        ActiveTheme, Sizable, h_flex, button::Button, menu::PopupMenuItem,
    };

    if let Some(variants) = args.type_info.enum_variants() {
        let current_ix = args.current_value.downcast_ref::<u64>().copied().unwrap_or(0) as usize;
        let selected_ix = current_ix.min(variants.len().saturating_sub(1));
        let label = variants
            .get(selected_ix)
            .copied()
            .unwrap_or("Select");
        let on_write = args.write_back.clone();
        let id = format!(
            "u64-{}-{}-{}",
            args.id_prefix, args.class_name, args.prop_name
        );
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
            .child(
                Button::new(id)
                    .label(label)
                    .xsmall()
                    .ghost()
                    .dropdown_caret(true)
                    .dropdown_menu_with_anchor(
                        Corner::BottomRight,
                        move |menu, _window, _cx| {
                            let mut menu = menu;
                            for (ix, option) in variants.iter().enumerate() {
                                let on_write = on_write.clone();
                                menu = menu.item(
                                    PopupMenuItem::new(option.to_string())
                                        .checked(ix == selected_ix)
                                        .on_click(move |_event, window, cx| {
                                            (on_write)(Box::new(ix as u64), window, cx);
                                        }),
                                );
                            }
                            menu
                        },
                    ),
            )
            .into_any_element()
    } else {
        let value = args.current_value.downcast_ref::<u64>().copied().unwrap_or(0);
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
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(value.to_string()),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use crate::{JsonDeserializer, JsonSerializer, RUNTIME_TYPE_REGISTRY, Reflectable};

    #[test]
    fn test_u64_registered() {
        let info = RUNTIME_TYPE_REGISTRY.get::<u64>().unwrap();
        assert_eq!(info.type_name, "u64");
        assert_eq!(info.size, 8);
        assert_eq!(info.align, 8);
    }

    #[test]
    fn test_u64_serialization() {
        let value: u64 = 18446744073709551615;
        let mut serializer = JsonSerializer::new();
        value.serialize(&mut serializer).unwrap();

        let json = serializer.as_json();
        assert_eq!(json.as_u64().unwrap(), value);
    }

    #[test]
    fn test_u64_deserialization() {
        let json = serde_json::json!(999999);
        let mut deserializer = JsonDeserializer::new(json);
        let value = u64::deserialize(&mut deserializer).unwrap();
        assert_eq!(value, 999999);
    }

    #[test]
    fn test_u64_clone_any() {
        let value: u64 = 12345678;
        let boxed = value.clone_any();
        assert_eq!(*boxed.downcast::<u64>().unwrap(), 12345678);
    }
}
