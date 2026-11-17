//! Randomize (plan section 11.3, brief §11): a new character drawn from an
//! old one, deterministic from a seed. Parameters are drawn inside their
//! `random` share (the middle half of the range when they have none), the
//! base model's constraints are kept, and items come only from what fits
//! the base and isn't excluded by what is worn. It lives here, not in the
//! editor, because games randomize NPCs too (plan section 0, decision 6).

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    BaseManifest, Category, Constraint, Description, ItemManifest, Library, WornItem, chosen_slots,
    wear_choices,
};

/// What Randomize may change.
#[derive(Clone, Debug, PartialEq)]
pub enum Scope {
    /// Every parameter and every slot.
    All,
    /// The parameters of groups, and the slots, in this category.
    Category(Category),
    /// The parameters of this feature ([`BaseManifest::feature_of`]).
    Feature(String),
    /// These parameters, by ID.
    Parameters(Vec<String>),
}

/// The share of a range Randomize draws from when a parameter has no
/// `random`: the middle half, so a random character is rarely extreme.
const MIDDLE_SHARE: f32 = 0.5;

/// The part of `range` Randomize draws a value from: `random` held inside
/// the range, or the middle half of the range.
pub fn random_span(range: [f32; 2], random: Option<[f32; 2]>) -> [f32; 2] {
    let [min, max] = range;
    match random {
        Some([low, high]) => {
            let (low, high) = (between(low, min, max), between(high, min, max));
            [low.min(high), low.max(high)]
        }
        None => {
            let middle = (min + max) / 2.0;
            let half = (max - min) * MIDDLE_SHARE / 2.0;
            [middle - half, middle + half]
        }
    }
}

/// A random character from `description`: what `scope` names is drawn
/// again from `seed`; what `locks` names (parameter IDs and slot IDs)
/// never changes; everything else, the name included, is kept. A
/// parameter with choices takes one of its choices. The same inputs always
/// give the same character, on every machine. A description whose base
/// model `library` lacks comes back unchanged.
pub fn randomize(
    description: &Description,
    scope: &Scope,
    locks: &BTreeSet<String>,
    seed: u64,
    library: &Library,
) -> Description {
    let mut out = description.clone();
    let Some(base) = library.base(&description.base) else {
        return out;
    };
    let mut rng = Rng::new(seed);

    // Parameters, in the manifest's order so a seed means one thing.
    let drawn: Vec<&str> = base
        .parameters
        .iter()
        .filter(|parameter| !locks.contains(&parameter.id))
        .filter(|parameter| in_scope(base, scope, parameter))
        .map(|parameter| parameter.id.as_str())
        .collect();
    let before = crate::parameter_values(base, description);
    for parameter in &base.parameters {
        if !drawn.contains(&parameter.id.as_str()) {
            continue;
        }
        let value = if parameter.choices.is_empty() {
            draw(&mut rng, parameter.range, parameter.random)
        } else {
            parameter.choices[rng.below(parameter.choices.len())].value
        };
        out.parameters.insert(parameter.id.clone(), value);
    }
    keep_constraints(base, &mut out, &before, &drawn, locks);

    // Items: every slot the scope names, but the locked ones and the ones
    // a parameter's choices fill (always the choice's).
    let chosen = chosen_slots(library, &base.id);
    let redrawn: Vec<&str> = base
        .slots
        .iter()
        .filter(|slot| match scope {
            Scope::All => true,
            Scope::Category(category) => slot.category == *category,
            Scope::Feature(_) | Scope::Parameters(_) => false,
        })
        .filter(|slot| !locks.contains(&slot.id) && !chosen.contains(&slot.id))
        .map(|slot| slot.id.as_str())
        .collect();
    if !redrawn.is_empty() {
        draw_items(&mut rng, base, library, &mut out, &redrawn);
    }
    wear_choices(&mut out, library);
    out
}

/// Whether `scope` names `parameter`.
fn in_scope(base: &BaseManifest, scope: &Scope, parameter: &crate::Parameter) -> bool {
    match scope {
        Scope::All => true,
        Scope::Category(category) => base.category_of(parameter) == Some(*category),
        Scope::Feature(feature) => base
            .feature_of(parameter)
            .is_some_and(|found| found.id == *feature),
        Scope::Parameters(ids) => ids.contains(&parameter.id),
    }
}

/// A value inside `range`'s random span, rounded to a thousandth so a
/// saved character reads well.
fn draw(rng: &mut Rng, range: [f32; 2], random: Option<[f32; 2]>) -> f32 {
    let [low, high] = random_span(range, random);
    let value = low + (high - low) * rng.unit();
    between((value * 1000.0).round() / 1000.0, low, high)
}

/// `value` held between `a` and `b`, whichever is lower: unlike
/// `f32::clamp` it never panics on a backwards range from a bad file.
fn between(value: f32, a: f32, b: f32) -> f32 {
    value.max(a.min(b)).min(a.max(b))
}

/// The base's constraints on what was drawn: a `max_sum` over its limit
/// scales the drawn parameters down (the others stay), and a `follow`
/// moves a target that wasn't drawn or locked with its drawn source.
fn keep_constraints(
    base: &BaseManifest,
    out: &mut Description,
    before: &BTreeMap<String, f32>,
    drawn: &[&str],
    locks: &BTreeSet<String>,
) {
    let value = |out: &Description, id: &str| {
        out.parameters
            .get(id)
            .copied()
            .or_else(|| base.parameter(id).map(|p| p.default))
            .unwrap_or(0.0)
    };
    for constraint in &base.constraints {
        match constraint {
            Constraint::MaxSum { parameters, limit } => {
                let (mut free, mut fixed) = (0.0, 0.0);
                for id in parameters {
                    let v = value(out, id).max(0.0);
                    if drawn.contains(&id.as_str()) {
                        free += v;
                    } else {
                        fixed += v;
                    }
                }
                if free + fixed <= *limit || free <= 0.0 {
                    continue;
                }
                // Rounded down, so the rounded values keep to the limit.
                let scale = ((limit - fixed).max(0.0) / free).min(1.0);
                for id in parameters.iter().filter(|id| drawn.contains(&id.as_str())) {
                    let scaled = (value(out, id) * scale * 1000.0).floor() / 1000.0;
                    let range = base.parameter(id).map_or([scaled, scaled], |p| p.range);
                    out.parameters
                        .insert(id.clone(), between(scaled, range[0], range[1]));
                }
            }
            Constraint::Follow {
                source,
                target,
                factor,
            } => {
                if !drawn.contains(&source.as_str())
                    || drawn.contains(&target.as_str())
                    || locks.contains(target)
                {
                    continue;
                }
                let Some(parameter) = base.parameter(target) else {
                    continue;
                };
                let moved = value(out, source) - before.get(source).copied().unwrap_or(0.0);
                let [min, max] = parameter.range;
                let next = between(value(out, target) + factor * moved, min, max);
                out.parameters
                    .insert(target.clone(), (next * 1000.0).round() / 1000.0);
            }
        }
    }
}

/// Wears new items in the `redrawn` slots: each slot gets one of the
/// items that fit the base there (or nothing), drawn evenly, skipping
/// items that conflict with what is worn, need what isn't, or also fill a
/// slot already taken. Items in other slots stay.
fn draw_items(
    rng: &mut Rng,
    base: &BaseManifest,
    library: &Library,
    out: &mut Description,
    redrawn: &[&str],
) {
    let slot_of = |id: &str| library.item(id).map(|item| item.slot.clone());
    out.items
        .retain(|worn| slot_of(&worn.id).is_none_or(|slot| !redrawn.contains(&slot.as_str())));
    let mut filled: BTreeSet<String> = out
        .items
        .iter()
        .filter_map(|worn| library.item(&worn.id))
        .flat_map(|item| item.slots().cloned())
        .filter(|slot| base.slot(slot).is_some_and(|s| !s.multiple))
        .collect();
    for slot in redrawn {
        // A slot that holds several gets up to two, so an item that needs
        // another there (a charm on a watch) can follow it.
        let picks = if base.slot(slot).is_some_and(|s| s.multiple) {
            2
        } else {
            1
        };
        for _ in 0..picks {
            if filled.contains(*slot) {
                break;
            }
            let worn: Vec<&ItemManifest> = out
                .items
                .iter()
                .filter_map(|worn| library.item(&worn.id))
                .collect();
            let options: Vec<&ItemManifest> = library
                .items
                .values()
                .filter(|item| item.slot == *slot && item.bases.contains(&base.id))
                .filter(|item| !worn.iter().any(|other| other.id == item.id))
                .filter(|item| !worn.iter().any(|other| item.conflicts_with(other)))
                .filter(|item| {
                    item.requires
                        .iter()
                        .all(|id| worn.iter().any(|other| other.id == *id))
                })
                .filter(|item| {
                    item.also_fills
                        .iter()
                        .all(|other| !filled.contains(other) && base.slot(other).is_some())
                })
                .collect();
            // One more choice than there are items: wearing nothing there.
            let pick = rng.below(options.len() + 1);
            let Some(item) = options.get(pick) else {
                break;
            };
            out.items.push(draw_item(rng, item));
            filled.extend(
                item.slots()
                    .filter(|slot| base.slot(slot).is_some_and(|s| !s.multiple))
                    .cloned(),
            );
        }
    }
}

/// `item` worn with a random variant and random values for its own
/// parameters.
fn draw_item(rng: &mut Rng, item: &ItemManifest) -> WornItem {
    let mut worn = WornItem::new(item.id.clone());
    if !item.variants.is_empty() {
        let variant = &item.variants[rng.below(item.variants.len())];
        worn.variant = Some(variant.id.clone());
    }
    for parameter in &item.parameters {
        let value = if parameter.choices.is_empty() {
            draw(rng, parameter.range, parameter.random)
        } else {
            parameter.choices[rng.below(parameter.choices.len())].value
        };
        worn.parameters.insert(parameter.id.clone(), value);
    }
    worn
}

/// SplitMix64: a small, well-mixed generator that gives the same numbers
/// on every machine. Enough for drawing characters; not for secrets.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 up to (not including) 1, from the top 24 bits (an
    /// `f32` holds 24 bits exactly).
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A whole number below `n` (`n` > 0).
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}
