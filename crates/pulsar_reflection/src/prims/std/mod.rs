//! Standard library primitive type registrations.

mod string;
mod wrappers;

pub(crate) fn ensure_registered() {
    let _ = <String as crate::Reflectable>::type_info();
}