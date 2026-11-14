//! What parameter values mean: a description resolved against its base
//! manifest. This is the contract's semantics, shared by Character Studio's
//! preview and the Character Runtime (CO3 adds bones, attributes and
//! material values on the same rules).

use std::collections::BTreeMap;

use crate::{BaseManifest, Constraint, Description, Library};

/// `value` mapped through `from` onto `to` (`to` defaults to `from`): how
/// far `value` is from `from[0]` towards `from[1]`, clamped to 0–1, then as
/// far from `to[0]` towards `to[1]`. `from` may run downwards (`[0, -1]`).
pub fn span(value: f32, from: [f32; 2], to: Option<[f32; 2]>) -> f32 {
    let to = to.unwrap_or(from);
    let length = from[1] - from[0];
    if length == 0.0 || !value.is_finite() {
        return to[0];
    }
    let t = ((value - from[0]) / length).clamp(0.0, 1.0);
    to[0] + (to[1] - to[0]) * t
}

/// Every parameter of `base` for `description`: its own value or the
/// default, clamped to the range, then each `max_sum` constraint applied.
pub fn parameter_values(base: &BaseManifest, description: &Description) -> BTreeMap<String, f32> {
    let mut values: BTreeMap<String, f32> = base
        .parameters
        .iter()
        .map(|parameter| {
            let [min, max] = parameter.range;
            let value = description
                .parameters
                .get(&parameter.id)
                .copied()
                .filter(|value| value.is_finite())
                .unwrap_or(parameter.default);
            (
                parameter.id.clone(),
                value.clamp(min.min(max), max.max(min)),
            )
        })
        .collect();
    for constraint in &base.constraints {
        let Constraint::MaxSum { parameters, limit } = constraint else {
            continue;
        };
        let sum: f32 = parameters
            .iter()
            .filter_map(|id| values.get(id))
            .map(|value| value.max(0.0))
            .sum();
        if sum > *limit && sum > 0.0 {
            let scale = limit / sum;
            for id in parameters {
                if let Some(value) = values.get_mut(id) {
                    *value *= scale;
                }
            }
        }
    }
    values
}

/// The body's blend shape weights for `description`: every shape the base
/// manifest names, at the sum of its parameter drivers' weights plus its
/// corners' (each the product of its factors), and the mask shape of every
/// region a worn item hides at 1 (others at 0). Items not in `library` are
/// skipped.
pub fn body_shape_weights(
    base: &BaseManifest,
    description: &Description,
    library: &Library,
) -> BTreeMap<String, f32> {
    let values = parameter_values(base, description);
    let value = |id: &str| values.get(id).copied().unwrap_or(0.0);
    let mut weights: BTreeMap<String, f32> = BTreeMap::new();
    for parameter in &base.parameters {
        for driver in &parameter.shapes {
            *weights.entry(driver.shape.clone()).or_default() +=
                span(value(&parameter.id), driver.from, driver.to);
        }
    }
    for corner in &base.corners {
        let weight: f32 = corner
            .factors
            .iter()
            .map(|factor| span(value(&factor.parameter), factor.from, factor.to))
            .product();
        *weights.entry(corner.shape.clone()).or_default() += weight;
    }
    for region in &base.regions {
        if let Some(mask) = &region.mask_shape {
            weights.entry(mask.clone()).or_default();
        }
    }
    let hidden = description
        .items
        .iter()
        .filter_map(|worn| library.item(&worn.id))
        .flat_map(|item| item.hides.iter());
    for region in hidden {
        if let Some(mask) = base.region(region).and_then(|r| r.mask_shape.as_ref()) {
            weights.insert(mask.clone(), 1.0);
        }
    }
    weights
}
