//! What parameter values mean: a description resolved against its base
//! manifest. This is the contract's semantics, shared by Character Studio's
//! preview and the Character Runtime (CO3 adds bones, attributes and
//! material values on the same rules).

use std::collections::BTreeMap;

use axcenity_assets::{BoneChange, Identity, IdentityLayer};

use crate::{BaseManifest, Constraint, Description, Library, MaterialValues, ModelRef, Problem};

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

/// A description resolved against its library: everything the Character
/// Runtime applies (CO3). Plain data: the host turns it into Instances,
/// derived models and material values.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    /// The base model's ID.
    pub base: String,
    /// The body: the base model's own mesh.
    pub body: ResolvedPart,
    /// What the body wears, in order: the base model's parts (eyes, teeth,
    /// brows, lashes), then one part per worn item, in the description's
    /// order.
    pub parts: Vec<ResolvedPart>,
    /// The body's shape weights by name ([`body_shape_weights`]).
    pub shapes: BTreeMap<String, f32>,
    /// Each changed bone's offset (model space, metres) and scale, summed
    /// over the parameters that drive it.
    pub bones: BTreeMap<String, BoneChange>,
    /// The identity to bake into the body and every part
    /// (`axcenity_assets::Model::bake`): a layer per parameter, corner and
    /// region mask, so each model reseats or carries by the shapes it has.
    pub identity: Identity,
    /// Number attributes for the body MeshPart (`anim`'s body sliders).
    pub attributes: BTreeMap<String, f32>,
    /// The base model's material values by slot: the description's over
    /// what parameters drive.
    pub materials: BTreeMap<String, MaterialValues>,
    /// What was left out and why (an item the library lacks, one that
    /// doesn't fit the base, a second item in a slot that holds one).
    pub problems: Vec<Problem>,
    /// The body regions the worn items hide, by ID, in the base's order
    /// (CO9): a region with a `map_index` is left out of the body drawn
    /// (`axcenity_assets::RegionHiding`), one with a mask shape sinks.
    pub hidden_regions: Vec<String>,
    /// The cover maps of the worn garments (ADR-0045), as library paths:
    /// the body leaves out the skin they cover.
    pub covers: Vec<String>,
    /// The colour map each material of the base model draws in place of
    /// its file's own (ADR-0048), by slot, as library paths: the one of
    /// the base's `material_maps` nearest the character's values. A slot
    /// that isn't here draws its file's own.
    pub colour_maps: BTreeMap<String, String>,
    /// For a slot of `colour_maps` between two maps (the Age slider
    /// between two photographed skins): the map its own fades toward, as
    /// a library path, and how far it has (0 to 1; a half where the two
    /// are as near). A shading model that can't draw two (all but Skin)
    /// draws `colour_maps`'s alone.
    pub colour_blends: BTreeMap<String, (String, f32)>,
}

/// One mesh of a resolved character.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedPart {
    /// Its Instance's name: the object's own name in its file, or the
    /// item's.
    pub name: String,
    /// The item it is, or `None` for the body and the base model's parts.
    pub item: Option<String>,
    /// The slot it fills.
    pub slot: Option<String>,
    /// Its model as a `MeshId` relative to the library's folder
    /// (`humanoid/HumanoidParts.glb#AxcenityHumanoid/Shirt_Tee`).
    pub model: String,
    pub fit: PartFit,
    /// Its material values by slot: the worn item's over its variant's,
    /// over what the item's own parameters drive.
    pub materials: BTreeMap<String, MaterialValues>,
    /// An item's own shape weights, from its own parameters (the worn
    /// item's values over its variant's, over the defaults): every shape
    /// they name, by name. Empty for the body, its parts and items without
    /// parameters. They are identity, baked into the item's model only.
    pub shapes: BTreeMap<String, f32>,
    /// How a socket item sits relative to how it was made (the worn
    /// item's `transform`; as made when it has none).
    pub transform: crate::Transform,
    /// The cover maps (library paths) of the garments worn over this one
    /// ([`ItemManifest::wearing_order`](crate::ItemManifest::wearing_order)):
    /// its cloth under them is left out. Empty for the body's own parts
    /// and for what isn't a garment.
    pub under: Vec<String>,
    /// Those of them it is **tucked into**: the cover maps of bottoms
    /// worn over a top. A waistband has nothing to be seen under, as a
    /// hem has, so this one's cloth is left out wherever theirs is over
    /// it, with no setback from the waistband.
    pub tucked_into: Vec<String>,
}

/// How a part follows the body.
#[derive(Clone, Debug, PartialEq)]
pub enum PartFit {
    /// On the body's skeleton, taking its pose and shapes (the body's own
    /// parts, `fit: shapes` items and, until CO9 fits them, `binding`
    /// items).
    Skinned,
    /// Rigid on this socket of the base model.
    Socket(String),
}

/// Resolves `description` against `library`, or says why it can't be (no
/// such base model). Items that can't be worn are left out and listed in
/// [`Resolved::problems`]; values outside their ranges are clamped. What a
/// parameter's choices wear is always worn ([`crate::wear_choices`]: the
/// human body's underwear), so the item indexes in problems are of the
/// description with it on.
pub fn resolve(library: &Library, description: &Description) -> Result<Resolved, Problem> {
    let base = library.base(&description.base).ok_or_else(|| {
        Problem::new(
            "base",
            format!("the library has no base model `{}`", description.base),
        )
    })?;
    let mut dressed = description.clone();
    crate::wear_choices(&mut dressed, library);
    let description = &dressed;
    let mesh_id = |owner: &str, model: &ModelRef| -> Option<String> {
        let file = library.path_of(owner, &model.file)?;
        Some(match &model.object {
            Some(object) => format!("{file}#{object}"),
            None => file,
        })
    };
    let own_part = |object: &str| -> Option<ResolvedPart> {
        let model = ModelRef {
            file: base.model.file.clone(),
            object: Some(object.to_owned()),
        };
        Some(ResolvedPart {
            name: last_segment(object).to_owned(),
            item: None,
            slot: None,
            model: mesh_id(&base.id, &model)?,
            fit: PartFit::Skinned,
            materials: BTreeMap::new(),
            shapes: BTreeMap::new(),
            transform: crate::Transform::default(),
            under: Vec::new(),
            tucked_into: Vec::new(),
        })
    };
    let body = own_part(&base.model.body).ok_or_else(|| {
        Problem::new(
            "base",
            format!("`{}`'s model leads out of the library", base.id),
        )
    })?;
    let mut parts: Vec<ResolvedPart> = base
        .model
        .parts
        .iter()
        .filter_map(|part| own_part(part))
        .collect();

    // The worn items, one per slot unless the slot holds several, and
    // none with an item it excludes (the later one is worn), each with why
    // another was left out.
    let mut problems: Vec<(usize, Problem)> = Vec::new();
    let mut worn: Vec<(usize, &crate::WornItem, &crate::ItemManifest)> = Vec::new();
    for (index, item) in description.items.iter().enumerate() {
        let mut skip = |message: String| {
            problems.push((index, Problem::new(format!("items[{index}]"), message)));
        };
        let Some(manifest) = library.item(&item.id) else {
            skip(format!("the library has no item `{}`", item.id));
            continue;
        };
        if !manifest.bases.contains(&base.id) {
            skip(format!("`{}` doesn't fit `{}`", manifest.id, base.id));
            continue;
        }
        if base.slot(&manifest.slot).is_none() {
            skip(format!("`{}` has no slot `{}`", base.id, manifest.slot));
            continue;
        }
        let single: Vec<&String> = manifest
            .slots()
            .filter(|slot| base.slot(slot).is_some_and(|slot| !slot.multiple))
            .collect();
        let mut kept = Vec::with_capacity(worn.len());
        for (earlier, earlier_item, other) in worn.drain(..) {
            let shared = other.slots().find(|slot| single.contains(slot));
            let why = match shared {
                Some(slot) => format!(
                    "the {slot} slot holds one item: `{}` is worn in place of `{}`",
                    manifest.id, other.id
                ),
                None if manifest.conflicts_with(other) => format!(
                    "`{}` can't be worn with `{}`: `{}` is worn in place of it",
                    other.id, manifest.id, manifest.id
                ),
                None => {
                    kept.push((earlier, earlier_item, other));
                    continue;
                }
            };
            problems.push((earlier, Problem::new(format!("items[{earlier}]"), why)));
        }
        worn = kept;
        worn.push((index, item, manifest));
    }
    // An item whose requirements aren't worn is left out (again, as long
    // as leaving one out leaves another without what it needs).
    loop {
        let missing = worn.iter().position(|(_, _, manifest)| {
            manifest
                .requires
                .iter()
                .any(|id| !worn.iter().any(|(_, _, other)| other.id == *id))
        });
        let Some(at) = missing else {
            break;
        };
        let (index, _, manifest) = worn.remove(at);
        let needs = manifest.requires.join("`, `");
        problems.push((
            index,
            Problem::new(
                format!("items[{index}]"),
                format!("`{}` needs `{needs}` worn with it", manifest.id),
            ),
        ));
    }
    // What a slot says another slot's item takes off the character
    // (`Slot::hidden_by`: the cut underwear under a bottom or a top) is
    // worn but not drawn: no part, no cover of its own, no problem.
    let filled: Vec<&String> = worn
        .iter()
        .flat_map(|(_, _, manifest)| manifest.slots())
        .collect();
    let off: Vec<&str> = worn
        .iter()
        .filter(|(_, _, manifest)| {
            base.slot(&manifest.slot)
                .is_some_and(|slot| slot.hidden_by.iter().any(|over| filled.contains(&over)))
        })
        .map(|(_, _, manifest)| manifest.id.as_str())
        .collect();
    worn.retain(|(_, _, manifest)| !off.contains(&manifest.id.as_str()));
    let mut hidden: Vec<&str> = Vec::new();
    // Each worn garment's cover map and where it is worn, inside out.
    let covering: Vec<(&str, (u8, u8), String, bool)> = worn
        .iter()
        .filter_map(|(_, _, manifest)| {
            let order = manifest.wearing_order()?;
            let map = library.path_of(&manifest.id, manifest.cover_map.as_deref()?)?;
            Some((manifest.id.as_str(), order, map, manifest.slot == "bottom"))
        })
        .collect();
    for (index, item, manifest) in &worn {
        let model = match manifest.kind {
            crate::ItemKind::Groom => manifest
                .groom
                .as_ref()
                .and_then(|groom| groom.cards.as_ref().or(groom.mesh.as_ref())),
            _ => manifest.model.as_ref(),
        };
        let Some(model_id) = model.and_then(|model| mesh_id(&manifest.id, model)) else {
            problems.push((
                *index,
                Problem::new(
                    format!("items[{index}]"),
                    format!("`{}` has no model in the library", manifest.id),
                ),
            ));
            continue;
        };
        let variant = item
            .variant
            .as_deref()
            .and_then(|variant| manifest.variant(variant));
        // The item's own parameters: worn value, else the variant's, else
        // the default, clamped to the range.
        let mut shapes: BTreeMap<String, f32> = BTreeMap::new();
        let mut materials: BTreeMap<String, MaterialValues> = BTreeMap::new();
        for parameter in &manifest.parameters {
            let [min, max] = parameter.range;
            let own = item
                .parameters
                .get(&parameter.id)
                .or_else(|| variant.and_then(|v| v.parameters.get(&parameter.id)))
                .copied()
                .filter(|value| value.is_finite())
                .unwrap_or(parameter.default)
                .max(min.min(max))
                .min(max.max(min));
            for driver in &parameter.shapes {
                *shapes.entry(driver.shape.clone()).or_default() +=
                    span(own, driver.from, driver.to);
            }
            for driver in &parameter.material_values {
                materials
                    .entry(driver.material.clone())
                    .or_default()
                    .insert(
                        driver.value.clone(),
                        crate::MaterialValue::Number(span(own, driver.from, driver.to)),
                    );
            }
        }
        for (slot, values) in variant.iter().flat_map(|v| &v.materials) {
            materials
                .entry(slot.clone())
                .or_default()
                .extend(values.clone());
        }
        for (slot, values) in &item.materials {
            materials
                .entry(slot.clone())
                .or_default()
                .extend(values.clone());
        }
        let name = model
            .and_then(|model| model.object.as_deref())
            .map_or_else(|| last_segment(&manifest.id), last_segment);
        let under: Vec<String> = manifest
            .wearing_order()
            .map(|own| {
                covering
                    .iter()
                    .filter(|(id, order, _, _)| *id != manifest.id && *order > own)
                    .map(|(_, _, map, _)| map.clone())
                    .collect()
            })
            .unwrap_or_default();
        // ADR-0046: the non-destructive `Shrink_Under` morph is on only
        // while a higher garment is worn over this one. It isn't named at
        // all otherwise: a part with no shapes of its own wears the body's
        // identity, and one with any gets an identity of its own.
        if manifest.kind == crate::ItemKind::Garment && !under.is_empty() {
            shapes.insert(crate::fit::SHRINK_UNDER_SHAPE.to_owned(), 1.0);
        }
        parts.push(ResolvedPart {
            name: name.to_owned(),
            item: Some(manifest.id.clone()),
            slot: Some(manifest.slot.clone()),
            model: model_id,
            fit: match &manifest.fit {
                crate::Fit::Socket { socket } => PartFit::Socket(socket.clone()),
                crate::Fit::Shapes { .. } | crate::Fit::Binding { .. } => PartFit::Skinned,
            },
            materials,
            shapes,
            transform: item.transform.unwrap_or_default(),
            under,
            // A top or a jacket under a bottom is tucked into it.
            tucked_into: manifest
                .wearing_order()
                .filter(|_| matches!(manifest.slot.as_str(), "top" | "outerwear"))
                .map(|own| {
                    covering
                        .iter()
                        .filter(|(id, order, _, bottom)| {
                            *bottom && *id != manifest.id && *order > own
                        })
                        .map(|(_, _, map, _)| map.clone())
                        .collect()
                })
                .unwrap_or_default(),
        });
        hidden.extend(manifest.hides.iter().map(String::as_str));
    }
    problems.sort_by_key(|(index, _)| *index);

    let values = parameter_values(base, description);
    let value = |id: &str| values.get(id).copied().unwrap_or(0.0);
    let mut identity = Identity::default();
    let mut bones: BTreeMap<String, BoneChange> = BTreeMap::new();
    let mut attributes: BTreeMap<String, f32> = BTreeMap::new();
    let mut materials: BTreeMap<String, MaterialValues> = BTreeMap::new();
    for parameter in &base.parameters {
        let own = value(&parameter.id);
        let mut layer = IdentityLayer::default();
        for driver in &parameter.shapes {
            layer
                .shapes
                .push((driver.shape.clone(), span(own, driver.from, driver.to)));
        }
        for driver in &parameter.bones {
            let weight = span(own, driver.from, driver.to);
            let offset = driver.offset.unwrap_or_default().map(|axis| axis * weight);
            let scale = 1.0 + (driver.scale.unwrap_or(1.0) - 1.0) * weight;
            if offset == [0.0; 3] && scale == 1.0 {
                continue;
            }
            layer.bones.push(BoneChange {
                bone: driver.bone.clone(),
                offset,
                scale,
            });
            let total = bones
                .entry(driver.bone.clone())
                .or_insert_with(|| BoneChange {
                    bone: driver.bone.clone(),
                    offset: [0.0; 3],
                    scale: 1.0,
                });
            for (sum, axis) in total.offset.iter_mut().zip(offset) {
                *sum += axis;
            }
            total.scale *= scale;
        }
        if !layer.shapes.is_empty() || !layer.bones.is_empty() {
            identity.layers.push(layer);
        }
        for driver in &parameter.attributes {
            attributes.insert(driver.attribute.clone(), span(own, driver.from, driver.to));
        }
        for driver in &parameter.material_values {
            materials
                .entry(driver.material.clone())
                .or_default()
                .insert(
                    driver.value.clone(),
                    crate::MaterialValue::Number(span(own, driver.from, driver.to)),
                );
        }
    }
    for corner in &base.corners {
        let weight: f32 = corner
            .factors
            .iter()
            .map(|factor| span(value(&factor.parameter), factor.from, factor.to))
            .product();
        identity.layers.push(IdentityLayer {
            shapes: vec![(corner.shape.clone(), weight)],
            bones: Vec::new(),
        });
    }
    for region in &base.regions {
        if let Some(mask) = &region.mask_shape {
            let weight = if hidden.contains(&region.id.as_str()) {
                1.0
            } else {
                0.0
            };
            identity.layers.push(IdentityLayer {
                shapes: vec![(mask.clone(), weight)],
                bones: Vec::new(),
            });
        }
    }
    for (slot, own) in &description.materials {
        materials
            .entry(slot.clone())
            .or_default()
            .extend(own.clone());
    }
    // Makeup and markings (ADR-0042): draw values beside the slot's own.
    for (slot, layers) in crate::skin::draw_values(base, &description.skin) {
        materials.entry(slot).or_default().extend(layers);
    }
    // The colour map nearest the character's values, for each material
    // the base offers any for (ADR-0048).
    let nearest = base.material_maps_for(&values);
    let colour_maps: BTreeMap<String, String> = nearest
        .iter()
        .filter_map(|(material, map)| {
            Some((
                (*material).to_owned(),
                library.path_of(&base.id, &map.colour_map)?,
            ))
        })
        .collect();
    // And the one it fades toward, where it is between two (the Age
    // slider between two photographed skins).
    let colour_blends: BTreeMap<String, (String, f32)> = nearest
        .iter()
        .filter(|(material, _)| colour_maps.contains_key(**material))
        .filter_map(|(material, map)| {
            let (toward, amount) = base.material_blend_for(map, &values)?;
            let path = library.path_of(&base.id, &toward.colour_map)?;
            (amount > 0.0).then(|| ((*material).to_owned(), (path, amount)))
        })
        .collect();
    // The body's weights are what the layers add up to, so the two can't
    // disagree about which items are worn.
    let mut shapes: BTreeMap<String, f32> = BTreeMap::new();
    for (name, weight) in identity.layers.iter().flat_map(|layer| &layer.shapes) {
        *shapes.entry(name.clone()).or_default() += weight;
    }
    Ok(Resolved {
        base: base.id.clone(),
        body,
        parts,
        shapes,
        bones,
        identity,
        attributes,
        materials,
        problems: problems.into_iter().map(|(_, problem)| problem).collect(),
        hidden_regions: base
            .regions
            .iter()
            .filter(|region| hidden.contains(&region.id.as_str()))
            .map(|region| region.id.clone())
            .collect(),
        covers: covering.into_iter().map(|(_, _, map, _)| map).collect(),
        colour_maps,
        colour_blends,
    })
}

/// The last name of an object path or an ID (`Rig/Tee` is `Tee`).
fn last_segment(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
