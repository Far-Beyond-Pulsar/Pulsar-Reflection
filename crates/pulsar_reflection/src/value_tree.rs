//! The rules for showing a reflected value as a tree of rows, shared by every
//! host that does (the editor's properties panel and its SceneDB Inspector),
//! so a value opens, splits and summarizes the same way in each.
//!
//! This is the part without widgets; `prims::style` draws the values.

use serde_json::{Number, Value};

/// Most children a long container shows before it splits into ranges.
pub const CHUNK: usize = 100;
/// A top-level value with no more children than this starts open.
pub const OPEN_BY_DEFAULT: usize = 16;
/// Most names a closed object's summary lists.
const SUMMARY_NAMES: usize = 4;

/// Whether a top-level value with `children` children starts open.
pub fn opens_by_default(children: usize) -> bool {
    (1..=OPEN_BY_DEFAULT).contains(&children)
}

/// How many children each range holds when `len` of them, more than
/// [`CHUNK`], are split: the smallest power of [`CHUNK`] that needs at most
/// [`CHUNK`] ranges.
pub fn range_span(len: usize) -> usize {
    let mut span = CHUNK;
    while len.div_ceil(span) > CHUNK {
        span *= CHUNK;
    }
    span
}

/// The name of the range of `len` children from `start`.
pub fn range_label(start: usize, len: usize) -> String {
    format!(
        "[{} … {}]",
        grouped(start),
        grouped(start + len.saturating_sub(1))
    )
}

/// A count with thousands separators.
pub fn grouped(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// What a closed container of `len` children holds: an object's first names,
/// or how many items.  A range has no value of its own: pass [`Value::Null`].
pub fn summary(value: &Value, len: usize) -> String {
    match value {
        Value::Object(map) if !map.is_empty() => {
            let mut names: Vec<&str> = map.keys().take(SUMMARY_NAMES).map(String::as_str).collect();
            if map.len() > names.len() {
                names.push("…");
            }
            format!("{{ {} }}", names.join(", "))
        }
        _ if len == 0 => "empty".to_string(),
        _ if len == 1 => "1 item".to_string(),
        _ => format!("{} items", grouped(len)),
    }
}

/// A number as written.  An `f32` widened to `f64` prints its widening noise,
/// so a value that is exactly an `f32` prints as one.
pub fn number_text(number: &Number) -> String {
    match number.as_f64() {
        Some(float) if number.is_f64() => {
            let narrow = float as f32;
            if f64::from(narrow) == float {
                format!("{narrow:?}")
            } else {
                format!("{float:?}")
            }
        }
        _ => number.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_small_values_start_open() {
        assert!(!opens_by_default(0));
        assert!(opens_by_default(1));
        assert!(opens_by_default(OPEN_BY_DEFAULT));
        assert!(!opens_by_default(OPEN_BY_DEFAULT + 1));
    }

    #[test]
    fn ranges_never_number_more_than_a_chunk() {
        assert_eq!(range_span(101), 100);
        assert_eq!(range_span(10_000), 100);
        assert_eq!(range_span(10_001), 10_000);
        assert_eq!(range_span(1_000_001), 1_000_000);
        assert_eq!(range_label(10_000, 100), "[10,000 … 10,099]");
    }

    #[test]
    fn closed_containers_say_what_they_hold() {
        assert_eq!(summary(&json!([]), 0), "empty");
        assert_eq!(summary(&json!({}), 0), "empty");
        assert_eq!(summary(&json!([7]), 1), "1 item");
        assert_eq!(summary(&Value::Null, 25_000), "25,000 items");
        let object = json!({"a": 1, "b": 2, "c": 3, "d": 4, "e": 5});
        assert_eq!(summary(&object, 5), "{ a, b, c, d, … }");
        assert_eq!(
            number_text(json!(0.1f32).as_number().expect("a number")),
            "0.1"
        );
    }
}
