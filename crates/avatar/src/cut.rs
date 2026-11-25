//! Who an item is made for: the male body, the female one, or either.
//!
//! An item says it with one tag (`masculine`, `feminine`, `unisex`; one
//! that says nothing is for both). A body has a type when its base has a
//! `frame` parameter (0 masculine, 1 feminine). Character Studio's Hair
//! and Clothing pages list by it, and [`crate::randomize`] draws only what
//! is made for the body it drew.

use crate::{BaseManifest, Description};

/// The tags that say who an item is made for.
pub const CUT_TAGS: [&str; 3] = ["masculine", "feminine", "unisex"];
/// The base parameter a body's frame is (0 masculine, 1 feminine).
pub const FRAME: &str = "frame";

/// Whether an item with `tags` is made for the feminine frame
/// (`Some(true)`), the masculine one (`Some(false)`) or both (`None`:
/// unisex, or it doesn't say).
pub fn made_for(tags: &[String]) -> Option<bool> {
    let has = |tag: &str| tags.iter().any(|t| t.eq_ignore_ascii_case(tag));
    match (has("masculine"), has("feminine")) {
        (true, false) => Some(false),
        (false, true) => Some(true),
        _ => None,
    }
}

/// Whether `description`'s body is the feminine frame rather than the
/// masculine one; `None` when its base has no frames. A frame that is a
/// choice (Male or Female) goes by the nearer choice, as what the body
/// type wears does ([`crate::wear_choices`]); a plain slider by its upper
/// half.
pub fn feminine_frame(base: &BaseManifest, description: &Description) -> Option<bool> {
    let frame = base.parameter(FRAME)?;
    let value = description
        .parameters
        .get(FRAME)
        .copied()
        .filter(|value| value.is_finite())
        .unwrap_or(frame.default);
    Some(match frame.nearest_choice(value) {
        Some(choice) => choice.value >= 0.5,
        None => value >= 0.5,
    })
}

/// Whether an item with `tags` is made for a body that is feminine or not
/// (`None`: a body without frames, which every item is for).
pub fn made_for_body(tags: &[String], feminine_body: Option<bool>) -> bool {
    match (made_for(tags), feminine_body) {
        (Some(cut), Some(body)) => cut == body,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|tag| (*tag).to_owned()).collect()
    }

    #[test]
    fn an_item_is_for_one_body_type_or_both() {
        assert_eq!(made_for(&tags(&["masculine", "casual"])), Some(false));
        assert_eq!(made_for(&tags(&["Feminine"])), Some(true));
        assert_eq!(made_for(&tags(&["unisex"])), None);
        assert_eq!(made_for(&tags(&["casual"])), None, "it doesn't say");
        assert_eq!(made_for(&tags(&["masculine", "feminine"])), None);
        assert!(made_for_body(&tags(&["feminine"]), Some(true)));
        assert!(!made_for_body(&tags(&["feminine"]), Some(false)));
        assert!(made_for_body(&tags(&["feminine"]), None));
        assert!(made_for_body(&tags(&["unisex"]), Some(false)));
    }
}
