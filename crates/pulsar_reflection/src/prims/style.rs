//! How reflected values look, wherever they are shown: one compact table row
//! per value, the name in a muted column of its own, the value beside it
//! drawn by what it is.
//!
//! The property editors here lay out with [`property_row`]; hosts that draw
//! values themselves (read-only inspectors, trees over containers, their own
//! asset pickers) use the same pieces, so every panel reads alike.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, Div, FontWeight, Hsla, SharedString};
use ui::{h_flex, ActiveTheme, Icon, IconName, Sizable as _};

/// The face of numbers, addresses and other literal values.
pub const MONO: &str = "JetBrains Mono";
/// The least height of a value's row.
pub const ROW_HEIGHT: f32 = 26.0;
/// How far a child row is indented under its parent.
pub const INDENT: f32 = 16.0;
/// The share of a row the name column takes, so values line up in a column.
pub const NAME_SHARE: f32 = 0.4;
/// The widest a single-value input grows.
const CONTROL_MAX_WIDTH: f32 = 240.0;
/// The widest one component of a vector grows.
const PILL_MAX_WIDTH: f32 = 110.0;

/// A property's row: its name, then its control.
pub fn property_row(label: &str, control: impl IntoElement, cx: &App) -> AnyElement {
    h_flex()
        .w_full()
        .min_h(px(ROW_HEIGHT))
        .items_center()
        .gap_2()
        .text_xs()
        .child(
            div()
                .flex_none()
                .w(gpui::relative(NAME_SHARE))
                .truncate()
                .text_color(cx.theme().muted_foreground)
                .child(SharedString::from(label.to_string())),
        )
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap_1p5()
                .child(control),
        )
        .into_any_element()
}

/// A box for a text or number input: as wide as the value column, up to a
/// width an input still reads well at, in the literal face.
pub fn control_box(control: impl IntoElement) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .max_w(px(CONTROL_MAX_WIDTH))
        .font_family(MONO)
        .child(control)
}

/// A heading over a group of rows.
pub fn heading(text: &str, color: Hsla) -> Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color)
        .child(SharedString::from(text.to_uppercase()))
}

/// One line of literal text.
pub fn mono() -> Div {
    div().truncate().font_family(MONO).text_xs()
}

/// The open/closed marker of a row with children.
pub fn chevron(open: bool, cx: &App) -> Icon {
    Icon::new(if open {
        IconName::ChevronDown
    } else {
        IconName::ChevronRight
    })
    .xsmall()
    .text_color(cx.theme().muted_foreground)
}

/// A boolean: a check box and the word.
pub fn bool_value(value: bool, cx: &App) -> Div {
    let theme = cx.theme();
    h_flex()
        .gap_1p5()
        .items_center()
        .child(
            div()
                .size(px(12.0))
                .rounded(px(3.0))
                .border_1()
                .border_color(if value { theme.success } else { theme.border })
                .when(value, |el| {
                    el.bg(theme.success.opacity(0.25)).child(
                        Icon::new(IconName::Check)
                            .size_2p5()
                            .text_color(theme.success),
                    )
                }),
        )
        .child(
            mono()
                .text_color(if value {
                    theme.success
                } else {
                    theme.muted_foreground
                })
                .child(value.to_string()),
        )
}

/// The colour of the `index`th component of a vector: x, y, z, w.
pub fn axis_color(index: usize, cx: &App) -> Hsla {
    let theme = cx.theme();
    [theme.red, theme.green, theme.blue, theme.yellow][index % 4]
}

/// A vector of two to four numbers, one tinted box each.
pub fn vector_value(components: &[impl AsRef<str>], cx: &App) -> Div {
    let theme = cx.theme();
    h_flex().flex_1().min_w_0().gap_1().children(
        components.iter().zip(["x", "y", "z", "w"]).enumerate().map(
            |(index, (component, name))| {
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .max_w(px(PILL_MAX_WIDTH))
                    .gap_1()
                    .px_1p5()
                    .rounded(px(3.0))
                    .bg(theme.muted.opacity(0.3))
                    .font_family(MONO)
                    .text_xs()
                    .child(div().text_color(axis_color(index, cx)).child(name))
                    .child(
                        div()
                            .truncate()
                            .text_color(theme.foreground)
                            .child(SharedString::from(component.as_ref().to_string())),
                    )
            },
        ),
    )
}

/// A swatch of `color`.
pub fn color_swatch(color: Hsla, cx: &App) -> Div {
    div()
        .flex_none()
        .w(px(28.0))
        .h(px(14.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(color)
}

/// The tinted box around one of a closed set of choices: an enum's variant,
/// or the control that picks it.
pub fn variant_box(content: impl IntoElement, cx: &App) -> Div {
    let color = cx.theme().magenta;
    div()
        .flex_none()
        .px_1p5()
        .rounded(px(4.0))
        .bg(color.opacity(0.16))
        .font_family(MONO)
        .text_xs()
        .text_color(color)
        .child(content)
}
