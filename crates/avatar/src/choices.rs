//! What a parameter's choices wear (the realistic body's underwear, which
//! follows Male or Female): always on. Character Studio has no control to
//! take it off, and [`crate::resolve`] and the Character Runtime put it on
//! a description that leaves it out, so a script can't take it off either.

use crate::{Description, Library, Parameter, WornItem};

/// The slots of base model `base` that its parameters' choices fill: what's
/// worn there is the choice's, not picked, and can't be taken off.
pub fn chosen_slots(library: &Library, base: &str) -> Vec<String> {
    let Some(manifest) = library.base(base) else {
        return Vec::new();
    };
    let mut slots: Vec<String> = manifest
        .parameters
        .iter()
        .flat_map(|parameter| &parameter.choices)
        .flat_map(|choice| &choice.items)
        .filter_map(|id| library.item(id))
        .filter(|item| item.bases.iter().any(|b| b == base))
        .map(|item| item.slot.clone())
        .collect();
    slots.sort();
    slots.dedup();
    slots
}

/// Makes what `description` wears in the slots `parameter`'s choices fill
/// exactly what the choice nearest `value` names: its items on, anything
/// else there off. Changes nothing when that's already so.
pub fn wear_choice(
    description: &mut Description,
    library: &Library,
    parameter: &Parameter,
    value: f32,
) {
    let Some(chosen) = parameter.nearest_choice(value) else {
        return;
    };
    let base = description.base.clone();
    let fits = |id: &String| {
        library
            .item(id)
            .filter(|item| item.bases.contains(&base))
            .map(|item| item.slot.clone())
    };
    let slots: Vec<String> = parameter
        .choices
        .iter()
        .flat_map(|choice| &choice.items)
        .filter_map(fits)
        .collect();
    let wanted: Vec<String> = chosen
        .items
        .iter()
        .filter(|id| fits(id).is_some())
        .cloned()
        .collect();
    description.items.retain(|worn| {
        wanted.contains(&worn.id)
            || library
                .item(&worn.id)
                .is_none_or(|item| !slots.contains(&item.slot))
    });
    for id in wanted {
        if !description.items.iter().any(|worn| worn.id == id) {
            description.items.push(WornItem::new(id));
        }
    }
}

/// [`wear_choice`] for every parameter of `description`'s base model that
/// has choices, at its value in `description` (its default when left out).
/// Changes nothing when the base model isn't in `library`.
pub fn wear_choices(description: &mut Description, library: &Library) {
    let Some(base) = library.base(&description.base) else {
        return;
    };
    for parameter in base.parameters.iter().filter(|p| !p.choices.is_empty()) {
        let value = description
            .parameters
            .get(&parameter.id)
            .copied()
            .filter(|value| value.is_finite())
            .unwrap_or(parameter.default);
        wear_choice(description, library, parameter, value);
    }
}
