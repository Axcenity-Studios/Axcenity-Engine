//! The base model manifest (`*.base.json`): what ships with a base model
//! besides its glTF file. Its parameters are the sliders Character Studio
//! draws and a description sets; each drives shapes, bones, attributes
//! and material values. It also names the body's regions (what clothes
//! hide), its slots, sockets, material slots, skeleton and face.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::material::MaterialSlot;
use crate::{FormatError, Problem, is_false, pretty, read_versioned};

/// The base model manifest's format version.
pub const BASE_VERSION: u32 = 1;

/// A base model package's manifest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaseManifest {
    pub format_version: u32,
    /// Its ID in the library, such as `axcenity/stylized`.
    pub id: String,
    pub label: String,
    /// The body family (`stylized`, `realistic`): items name the base
    /// models they fit, families only group them.
    pub family: String,
    pub model: BaseModel,
    pub skeleton: Skeleton,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Group>,
    /// In the order Character Studio shows them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corners: Vec<Corner>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<Constraint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<Region>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<Slot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sockets: Vec<Socket>,
    /// Every material of the body and its parts, with its shading model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<MaterialSlot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<Face>,
    #[serde(default)]
    pub defaults: Defaults,
    /// What a person can point at on the character (R1): the nose, the
    /// shoulders. Selection, the camera's focus and the handles use them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<Feature>,
    /// Handles on the character that move parameters by dragging (R2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handles: Vec<Handle>,
    /// Makeup and markings its skin can draw (R5, ADR-0042), each a mask
    /// in the model file: what a description's `skin` block turns on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skin_layers: Vec<crate::SkinLayerSpec>,
}

/// Brief §3.3's three levels of customization. A value a newer version
/// adds reads as `Other`, which [`Parameter::level`] treats as no level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Quick,
    Detailed,
    Advanced,
    #[serde(other)]
    Other,
}

/// A selectable thing on the character: one anchor for a middle feature,
/// two for a pair (left first). A click picks the feature whose anchor is
/// nearest within `radius`; the camera frames its anchors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub label: String,
    pub category: Category,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub anchors: Vec<Anchor>,
    /// Metres around an anchor that count as the feature.
    pub radius: f32,
    /// Body regions (by ID) that pick it too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<String>,
}

/// A point that follows a bone: the bone, and an offset from its head in
/// model space at rest (metres, glTF axes: Y up, +Z front).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub bone: String,
    pub offset: [f32; 3],
}

/// A handle on the character. It holds no geometry of its own: dragging
/// it changes the parameters it `drives`, as their sliders would, and it
/// sits at its anchor plus `travel × t` along each drive's axis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Handle {
    pub id: String,
    /// Shown when this feature is selected.
    pub feature: String,
    /// The driven parameter's label when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The handle on the other side; each names the other.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<String>,
    /// Where it sits with its parameters at their defaults.
    pub anchor: Anchor,
    pub drives: Vec<Drive>,
    /// Only shown in these framings (`front`, `side`, `three_quarter`,
    /// `back`); in all when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<String>,
}

/// What a handle's drag changes: a parameter, moved as the surface at the
/// anchor moves `travel` metres along `axis` over the parameter's range.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Drive {
    pub parameter: String,
    /// Model-space direction the surface moves for a rising value.
    pub axis: [f32; 3],
    pub travel: f32,
    /// A screen direction (x right, y up) per view, for a parameter that
    /// isn't a straight move (a tilt, a size).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub screen: BTreeMap<String, [f32; 2]>,
    /// A multiplier on the one-to-one feel (1 when unset).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitivity: Option<f32>,
}

/// Where the base model's meshes are.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaseModel {
    /// The glTF file, relative to the manifest.
    pub file: String,
    /// The body's object path in the file (a `MeshId` pick).
    pub body: String,
    /// Parts always worn with the body (eyes, teeth, brows, lashes), by
    /// object path. They follow the body's pose and shapes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<String>,
}

/// The skeleton contract the model keeps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Skeleton {
    /// The bone naming convention: `humanoid` is the Unity/VRM humanoid
    /// names every Axcenity clip, retarget profile and face profile uses.
    pub convention: String,
    /// Standing height in metres (glTF axes: Y up, front +Z).
    pub height: f32,
    /// The rest pose: `a-pose-45` is arms 45° down.
    pub pose: String,
    /// Bones beyond the convention's (twist and helper bones), added,
    /// never renamed: each with its parent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_bones: Vec<AddedBone>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddedBone {
    pub name: String,
    pub parent: String,
}

/// A group of parameters, shown together in a Character Studio category.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub category: Category,
    /// A sentence for a tooltip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// A name from the editor's icon set (an unknown one draws none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// The Character Studio categories data can fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Body,
    Face,
    Skin,
    Hair,
    Clothing,
    Accessories,
}

/// A slider: a number from `range[0]` to `range[1]` that drives shapes,
/// bones, attributes and material values. Each driver maps the value
/// through its `from` span onto its `to` span ([`crate::span`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub id: String,
    pub label: String,
    /// A [`Group`]'s ID.
    pub group: String,
    pub range: [f32; 2],
    #[serde(default)]
    pub default: f32,
    /// Shown only in the advanced view.
    #[serde(default, skip_serializing_if = "is_false")]
    pub advanced: bool,
    /// The other side's parameter, for symmetric editing (each names the
    /// other).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<ShapeDriver>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bones: Vec<BoneDriver>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<AttributeDriver>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub material_values: Vec<MaterialDriver>,
    /// Named values to pick from instead of a slider (Character Studio
    /// shows a button each, and keeps the parameter at one of them): the
    /// realistic body's `frame` is Male (0) or Female (1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice>,
    /// One sentence: the friendly description and the tooltip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// The [`Feature`] it belongs to; its group's ID when unset and a
    /// feature has that ID ([`BaseManifest::feature_of`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    /// Which of the three levels shows it ([`Parameter::level`]).
    /// Generators write `advanced` in step, so an older editor hides the
    /// same sliders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
    /// The part of the range Randomize draws from; the middle half of the
    /// range when unset ([`crate::random_span`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<[f32; 2]>,
}

/// One of a [`Parameter`]'s `choices`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub value: f32,
    pub label: String,
    /// Items picking it puts on (in their slots), taking off the items the
    /// parameter's other choices name: the realistic body's Female wears a
    /// bra and briefs, its Male boxer briefs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
}

impl Parameter {
    /// The choice nearest `value`, if the parameter has choices.
    pub fn nearest_choice(&self, value: f32) -> Option<&Choice> {
        self.choices
            .iter()
            .min_by(|a, b| (a.value - value).abs().total_cmp(&(b.value - value).abs()))
    }

    /// The level that shows it: its `level`, or with none (or one this
    /// version doesn't know) `advanced` when `advanced` is set, else
    /// `detailed`.
    pub fn level(&self) -> Level {
        match self.level {
            Some(Level::Other) | None if self.advanced => Level::Advanced,
            Some(Level::Other) | None => Level::Detailed,
            Some(level) => level,
        }
    }
}

fn unit_span() -> [f32; 2] {
    [0.0, 1.0]
}

fn is_unit_span(span: &[f32; 2]) -> bool {
    *span == unit_span()
}

/// A blend shape's weight from a parameter: the value mapped from `from`
/// onto `to` (`to` defaults to `from`), clamped to `to`. Weights from
/// several drivers of one shape add.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeDriver {
    pub shape: String,
    #[serde(default = "unit_span", skip_serializing_if = "is_unit_span")]
    pub from: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<[f32; 2]>,
}

/// A bone's change from a parameter, at mapped weight 1 (scaled by the
/// mapped weight, which may be negative): `offset` moves the bone's head
/// in model space (metres, glTF axes), `scale` scales it and everything on
/// it (`1 + (scale - 1) × weight`). Applied after the animated pose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoneDriver {
    pub bone: String,
    #[serde(default = "unit_span", skip_serializing_if = "is_unit_span")]
    pub from: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
}

/// A number attribute on the character's skinned body from a parameter:
/// the body sliders `anim` already applies (`BodyHeight`, `LegLength`, …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttributeDriver {
    pub attribute: String,
    #[serde(default = "unit_span", skip_serializing_if = "is_unit_span")]
    pub from: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<[f32; 2]>,
}

/// A material value from a parameter: `value` of the material slot
/// `material` (a number value of its shading model).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialDriver {
    pub material: String,
    pub value: String,
    #[serde(default = "unit_span", skip_serializing_if = "is_unit_span")]
    pub from: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<[f32; 2]>,
}

/// A shape driven by several parameters at once (MakeHuman's corner
/// targets: feminine-and-heavy, muscular-and-thin): its weight is the
/// product of its factors, so the layers mix without cancelling.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Corner {
    pub shape: String,
    pub factors: Vec<Factor>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Factor {
    pub parameter: String,
    #[serde(default = "unit_span", skip_serializing_if = "is_unit_span")]
    pub from: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<[f32; 2]>,
}

/// A rule between parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Constraint {
    /// The named parameters (all with ranges from 0 up) add to at most
    /// `limit`; over it, all are scaled down in proportion. Applied when a
    /// description is resolved.
    MaxSum { parameters: Vec<String>, limit: f32 },
    /// While editing, changing `source` by d moves `target` by
    /// `factor × d` unless the person pinned `target`. Not applied when
    /// resolving: a saved description holds the result.
    Follow {
        source: String,
        target: String,
        factor: f32,
    },
}

/// A region of the body that items hide. How it is hidden: by a mask
/// shape that pulls the skin in (the stylized body, ADR-0030), or by its
/// index in the body's region map (binding and region hiding, CO9). At
/// least one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask_shape: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_index: Option<u8>,
}

/// Where items are worn. One item per slot unless `multiple`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub id: String,
    pub label: String,
    pub category: Category,
    #[serde(default, skip_serializing_if = "is_false")]
    pub multiple: bool,
    /// What wearing nothing here is called ("Bald"); "None" when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_label: Option<String>,
    /// A sentence for a tooltip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// A name from the editor's icon set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// An attachment point that follows a bone (`anim`'s sockets): rigid
/// accessories hang from it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Socket {
    pub name: String,
    pub bone: String,
}

/// The face: which FaceControls profile maps its 48 standard controls to
/// its shapes and bones (ADR-0028), and its eye and jaw bones. Expressions,
/// blinks, gaze and lip sync run on it; identity sliders are parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Face {
    pub profile: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eye_bones: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jaw_bone: Option<String>,
}

/// What a new character on this base wears.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
}

impl BaseManifest {
    pub fn from_json(text: &str) -> Result<Self, FormatError> {
        read_versioned(text, "base model manifest", BASE_VERSION)
    }

    pub fn to_json(&self) -> String {
        pretty(self)
    }

    pub fn parameter(&self, id: &str) -> Option<&Parameter> {
        self.parameters.iter().find(|p| p.id == id)
    }

    pub fn region(&self, id: &str) -> Option<&Region> {
        self.regions.iter().find(|r| r.id == id)
    }

    pub fn slot(&self, id: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }

    pub fn socket(&self, name: &str) -> Option<&Socket> {
        self.sockets.iter().find(|s| s.name == name)
    }

    pub fn material(&self, name: &str) -> Option<&MaterialSlot> {
        self.materials.iter().find(|m| m.name == name)
    }

    pub fn group(&self, id: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    pub fn feature(&self, id: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == id)
    }

    pub fn handle(&self, id: &str) -> Option<&Handle> {
        self.handles.iter().find(|h| h.id == id)
    }

    /// The skin layer `id` it offers.
    pub fn skin_layer(&self, id: &str) -> Option<&crate::SkinLayerSpec> {
        self.skin_layers.iter().find(|layer| layer.id == id)
    }

    /// The category of `parameter`'s group, if it has one.
    pub fn category_of(&self, parameter: &Parameter) -> Option<Category> {
        self.group(&parameter.group).map(|group| group.category)
    }

    /// The feature `parameter` belongs to: its own `feature`, or the
    /// feature with its group's ID.
    pub fn feature_of(&self, parameter: &Parameter) -> Option<&Feature> {
        match &parameter.feature {
            Some(id) => self.feature(id),
            None => self.feature(&parameter.group),
        }
    }

    /// Checks the manifest on its own: well-formed IDs and paths, unique
    /// names, ranges with their defaults inside, groups, mirrors, corners,
    /// constraints and material drivers that name what exists here.
    pub fn validate(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        let p = &mut problems;
        check_version(p, self.format_version, BASE_VERSION);
        check_id(p, "id", &self.id);
        not_empty(p, "label", &self.label);
        not_empty(p, "family", &self.family);
        check_path(p, "model.file", &self.model.file);
        not_empty(p, "model.body", &self.model.body);
        unique(p, "model.parts", self.model.parts.iter());
        not_empty(p, "skeleton.convention", &self.skeleton.convention);
        if !(self.skeleton.height.is_finite() && self.skeleton.height > 0.0) {
            p.push(Problem::new("skeleton.height", "must be above 0"));
        }
        unique(
            p,
            "skeleton.added_bones",
            self.skeleton.added_bones.iter().map(|bone| &bone.name),
        );
        unique(p, "groups", self.groups.iter().map(|group| &group.id));
        for (index, group) in self.groups.iter().enumerate() {
            check_about(p, &format!("groups[{index}]"), &group.about);
        }
        unique(p, "parameters", self.parameters.iter().map(|par| &par.id));
        for (index, parameter) in self.parameters.iter().enumerate() {
            self.check_parameter(p, &format!("parameters[{index}]"), parameter);
        }
        for (index, corner) in self.corners.iter().enumerate() {
            let at = format!("corners[{index}]");
            not_empty(p, &format!("{at}.shape"), &corner.shape);
            if corner.factors.is_empty() {
                p.push(Problem::new(&at, "has no factors"));
            }
            for (k, factor) in corner.factors.iter().enumerate() {
                if self.parameter(&factor.parameter).is_none() {
                    p.push(Problem::new(
                        &at,
                        format!("factor {k} names no parameter `{}`", factor.parameter),
                    ));
                }
                check_span(p, &format!("{at}.factors[{k}]"), factor.from, factor.to);
            }
        }
        for (index, constraint) in self.constraints.iter().enumerate() {
            self.check_constraint(p, &format!("constraints[{index}]"), constraint);
        }
        unique(p, "regions", self.regions.iter().map(|region| &region.id));
        for (index, region) in self.regions.iter().enumerate() {
            if region.mask_shape.is_none() && region.map_index.is_none() {
                p.push(Problem::new(
                    format!("regions[{index}]"),
                    format!("`{}` has neither a mask shape nor a map index", region.id),
                ));
            }
        }
        unique(p, "slots", self.slots.iter().map(|slot| &slot.id));
        for (index, slot) in self.slots.iter().enumerate() {
            check_about(p, &format!("slots[{index}]"), &slot.about);
        }
        unique(p, "sockets", self.sockets.iter().map(|socket| &socket.name));
        unique(p, "materials", self.materials.iter().map(|m| &m.name));
        for (index, slot) in self.materials.iter().enumerate() {
            crate::material::check_slot(p, &format!("materials[{index}]"), slot);
        }
        crate::skin::check_specs(p, self);
        unique(p, "features", self.features.iter().map(|f| &f.id));
        for (index, feature) in self.features.iter().enumerate() {
            self.check_feature(p, &format!("features[{index}]"), feature);
        }
        unique(p, "handles", self.handles.iter().map(|h| &h.id));
        for (index, handle) in self.handles.iter().enumerate() {
            self.check_handle(p, &format!("handles[{index}]"), handle);
        }
        if let Some(face) = &self.face {
            not_empty(p, "face.profile", &face.profile);
        }
        for (index, id) in self.defaults.items.iter().enumerate() {
            check_id(p, &format!("defaults.items[{index}]"), id);
        }
        problems
    }

    fn check_parameter(&self, p: &mut Vec<Problem>, at: &str, parameter: &Parameter) {
        not_empty(p, &format!("{at}.label"), &parameter.label);
        if !self.groups.iter().any(|group| group.id == parameter.group) {
            p.push(Problem::new(
                at,
                format!("`{}` is in no group `{}`", parameter.id, parameter.group),
            ));
        }
        check_value_range(p, at, &parameter.id, parameter.range, parameter.default);
        check_random(p, at, &parameter.id, parameter.range, parameter.random);
        check_about(p, at, &parameter.about);
        for (k, choice) in parameter.choices.iter().enumerate() {
            check_choice(
                p,
                &format!("{at}.choices[{k}]"),
                &parameter.id,
                parameter.range,
                choice,
            );
            for (n, id) in choice.items.iter().enumerate() {
                check_id(p, &format!("{at}.choices[{k}].items[{n}]"), id);
            }
        }
        if let Some(feature) = &parameter.feature
            && self.feature(feature).is_none()
        {
            p.push(Problem::new(
                at,
                format!("`{}` names no feature `{feature}`", parameter.id),
            ));
        }
        match (parameter.level, parameter.advanced) {
            (Some(Level::Advanced), false) => p.push(Problem::new(
                at,
                format!(
                    "`{}`'s level is advanced but `advanced` isn't set: write both",
                    parameter.id
                ),
            )),
            (Some(Level::Quick | Level::Detailed), true) => p.push(Problem::new(
                at,
                format!(
                    "`{}`'s level isn't advanced but `advanced` is set: write both",
                    parameter.id
                ),
            )),
            _ => {}
        }
        if let Some(mirror) = &parameter.mirror
            && self
                .parameter(mirror)
                .and_then(|other| other.mirror.as_deref())
                != Some(parameter.id.as_str())
        {
            p.push(Problem::new(
                at,
                format!(
                    "`{}`'s mirror `{mirror}` doesn't name it back",
                    parameter.id
                ),
            ));
        }
        for (k, driver) in parameter.shapes.iter().enumerate() {
            let at = format!("{at}.shapes[{k}]");
            not_empty(p, &at, &driver.shape);
            check_span(p, &at, driver.from, driver.to);
        }
        for (k, driver) in parameter.bones.iter().enumerate() {
            let at = format!("{at}.bones[{k}]");
            not_empty(p, &at, &driver.bone);
            check_span(p, &at, driver.from, driver.to);
            if driver.offset.is_none() && driver.scale.is_none() {
                p.push(Problem::new(
                    &at,
                    "moves nothing: give an offset or a scale",
                ));
            }
            if driver
                .scale
                .is_some_and(|scale| !(scale.is_finite() && scale > 0.0))
            {
                p.push(Problem::new(&at, "a scale must be above 0"));
            }
            if driver
                .offset
                .is_some_and(|offset| offset.iter().any(|v| !v.is_finite()))
            {
                p.push(Problem::new(&at, "the offset isn't a finite vector"));
            }
        }
        for (k, driver) in parameter.attributes.iter().enumerate() {
            let at = format!("{at}.attributes[{k}]");
            not_empty(p, &at, &driver.attribute);
            check_span(p, &at, driver.from, driver.to);
        }
        for (k, driver) in parameter.material_values.iter().enumerate() {
            let at = format!("{at}.material_values[{k}]");
            check_material_driver(p, &at, driver, self.material(&driver.material));
        }
    }

    fn check_feature(&self, p: &mut Vec<Problem>, at: &str, feature: &Feature) {
        not_empty(p, &format!("{at}.label"), &feature.label);
        check_about(p, at, &feature.about);
        if feature.anchors.is_empty() {
            p.push(Problem::new(
                at,
                format!("`{}` has no anchor to point at", feature.id),
            ));
        }
        for (k, anchor) in feature.anchors.iter().enumerate() {
            check_anchor(p, &format!("{at}.anchors[{k}]"), anchor);
        }
        if !(feature.radius.is_finite() && feature.radius > 0.0) {
            p.push(Problem::new(
                at,
                format!("`{}`'s radius must be above 0", feature.id),
            ));
        }
        for region in &feature.regions {
            if self.region(region).is_none() {
                p.push(Problem::new(
                    at,
                    format!("`{}` names no region `{region}`", feature.id),
                ));
            }
        }
    }

    fn check_handle(&self, p: &mut Vec<Problem>, at: &str, handle: &Handle) {
        if self.feature(&handle.feature).is_none() {
            p.push(Problem::new(
                at,
                format!("`{}` names no feature `{}`", handle.id, handle.feature),
            ));
        }
        if let Some(label) = &handle.label {
            not_empty(p, &format!("{at}.label"), label);
        }
        check_anchor(p, &format!("{at}.anchor"), &handle.anchor);
        if handle.drives.is_empty() {
            p.push(Problem::new(
                at,
                format!("`{}` has no drives: it would move nothing", handle.id),
            ));
        }
        for (k, drive) in handle.drives.iter().enumerate() {
            let at = format!("{at}.drives[{k}]");
            if self.parameter(&drive.parameter).is_none() {
                p.push(Problem::new(
                    &at,
                    format!("names no parameter `{}`", drive.parameter),
                ));
            }
            let finite = drive.axis.iter().all(|v| v.is_finite());
            if !finite || drive.axis.iter().all(|v| *v == 0.0) {
                p.push(Problem::new(
                    &at,
                    "the axis must be a finite, non-zero vector",
                ));
            }
            if !(drive.travel.is_finite() && drive.travel > 0.0) {
                p.push(Problem::new(&at, "the travel must be above 0 metres"));
            }
            if drive
                .sensitivity
                .is_some_and(|s| !(s.is_finite() && s > 0.0))
            {
                p.push(Problem::new(&at, "a sensitivity must be above 0"));
            }
            for (view, direction) in &drive.screen {
                if direction.iter().any(|v| !v.is_finite()) || *direction == [0.0, 0.0] {
                    p.push(Problem::new(
                        &at,
                        format!("the screen direction for `{view}` must be finite and non-zero"),
                    ));
                }
            }
        }
        if let Some(mirror) = &handle.mirror {
            let other = self.handle(mirror);
            if *mirror == handle.id {
                p.push(Problem::new(
                    at,
                    format!("`{}` is named as its own mirror", handle.id),
                ));
            } else if other.and_then(|other| other.mirror.as_deref()) != Some(handle.id.as_str()) {
                p.push(Problem::new(
                    at,
                    format!("`{}`'s mirror `{mirror}` doesn't name it back", handle.id),
                ));
            }
            // A pair is one feature's left and right.
            if let Some(other) = other
                && other.feature != handle.feature
            {
                p.push(Problem::new(
                    at,
                    format!(
                        "`{}`'s mirror `{mirror}` is on another feature (`{}`)",
                        handle.id, other.feature
                    ),
                ));
            }
        }
        unique(p, &format!("{at}.views"), handle.views.iter());
    }

    fn check_constraint(&self, p: &mut Vec<Problem>, at: &str, constraint: &Constraint) {
        match constraint {
            Constraint::MaxSum { parameters, limit } => {
                if parameters.len() < 2 {
                    p.push(Problem::new(at, "a sum needs two parameters or more"));
                }
                if !(limit.is_finite() && *limit > 0.0) {
                    p.push(Problem::new(at, "the limit must be above 0"));
                }
                for id in parameters {
                    match self.parameter(id) {
                        None => p.push(Problem::new(at, format!("names no parameter `{id}`"))),
                        Some(parameter) if parameter.range.iter().any(|end| *end < 0.0) => p.push(
                            Problem::new(at, format!("`{id}` goes below 0, so it can't be summed")),
                        ),
                        Some(_) => {}
                    }
                }
            }
            Constraint::Follow { source, target, .. } => {
                for id in [source, target] {
                    if self.parameter(id).is_none() {
                        p.push(Problem::new(at, format!("names no parameter `{id}`")));
                    }
                }
                if source == target {
                    p.push(Problem::new(at, "a parameter can't follow itself"));
                }
            }
        }
    }
}

pub(crate) fn check_version(p: &mut Vec<Problem>, found: u32, supported: u32) {
    if found == 0 || found > supported {
        p.push(Problem::new(
            "format_version",
            format!("is {found}; this version writes {supported}"),
        ));
    }
}

pub(crate) fn check_id(p: &mut Vec<Problem>, at: &str, id: &str) {
    if !crate::is_valid_id(id) {
        p.push(Problem::new(
            at,
            format!("`{id}` isn't an ID (lower-case words joined by `/`, such as `axcenity/tee`)"),
        ));
    }
}

pub(crate) fn check_path(p: &mut Vec<Problem>, at: &str, path: &str) {
    if !crate::is_valid_path(path) {
        p.push(Problem::new(
            at,
            format!("`{path}` isn't a relative path with `/` between folders"),
        ));
    }
}

pub(crate) fn not_empty(p: &mut Vec<Problem>, at: &str, text: &str) {
    if text.trim().is_empty() {
        p.push(Problem::new(at, "is empty"));
    }
}

/// An `about` that is there says something.
pub(crate) fn check_about(p: &mut Vec<Problem>, at: &str, about: &Option<String>) {
    if let Some(about) = about {
        not_empty(p, &format!("{at}.about"), about);
    }
}

/// A parameter's range goes upwards and holds its default.
pub(crate) fn check_value_range(
    p: &mut Vec<Problem>,
    at: &str,
    id: &str,
    range: [f32; 2],
    default: f32,
) {
    let [min, max] = range;
    if !(min.is_finite() && max.is_finite() && min < max) {
        p.push(Problem::new(
            at,
            format!("`{id}` has the range {min} to {max}"),
        ));
    } else if !(min..=max).contains(&default) {
        p.push(Problem::new(
            at,
            format!("`{id}`'s default {default} is outside {min} to {max}"),
        ));
    }
}

/// A `random` share is finite, goes upwards (or is one value) and lies in
/// the range.
pub(crate) fn check_random(
    p: &mut Vec<Problem>,
    at: &str,
    id: &str,
    range: [f32; 2],
    random: Option<[f32; 2]>,
) {
    let Some([low, high]) = random else {
        return;
    };
    let [min, max] = range;
    if !(low.is_finite() && high.is_finite() && low <= high && min <= low && high <= max) {
        p.push(Problem::new(
            at,
            format!("`{id}`'s random {low} to {high} isn't inside its range, {min} to {max}"),
        ));
    }
}

/// A choice has a label and a value in its parameter's range.
pub(crate) fn check_choice(
    p: &mut Vec<Problem>,
    at: &str,
    id: &str,
    range: [f32; 2],
    choice: &Choice,
) {
    not_empty(p, &format!("{at}.label"), &choice.label);
    let [min, max] = range;
    if !(min..=max).contains(&choice.value) {
        p.push(Problem::new(
            at,
            format!("`{id}`'s choice {} is outside {min} to {max}", choice.value),
        ));
    }
}

/// A material driver drives a number value of a material slot that exists.
pub(crate) fn check_material_driver(
    p: &mut Vec<Problem>,
    at: &str,
    driver: &MaterialDriver,
    slot: Option<&MaterialSlot>,
) {
    check_span(p, at, driver.from, driver.to);
    let Some(slot) = slot else {
        p.push(Problem::new(
            at,
            format!("names no material `{}`", driver.material),
        ));
        return;
    };
    match slot.shading.value(&driver.value).map(|spec| spec.kind) {
        Some(crate::ValueKind::Number { .. }) => {}
        Some(crate::ValueKind::Colour { .. }) => p.push(Problem::new(
            at,
            format!("`{}` is a colour; parameters drive numbers", driver.value),
        )),
        None => p.push(Problem::new(
            at,
            format!(
                "`{}` isn't a value of the {:?} shading model",
                driver.value, slot.shading
            ),
        )),
    }
}

fn check_anchor(p: &mut Vec<Problem>, at: &str, anchor: &Anchor) {
    not_empty(p, &format!("{at}.bone"), &anchor.bone);
    if anchor.offset.iter().any(|v| !v.is_finite()) {
        p.push(Problem::new(at, "the offset isn't a finite vector"));
    }
}

/// A span must go somewhere: `from`'s ends differ, and both spans are
/// finite.
pub(crate) fn check_span(p: &mut Vec<Problem>, at: &str, from: [f32; 2], to: Option<[f32; 2]>) {
    let finite = from
        .iter()
        .chain(to.iter().flatten())
        .all(|v| v.is_finite());
    if !finite {
        p.push(Problem::new(at, "a span isn't finite"));
    } else if from[0] == from[1] {
        p.push(Problem::new(
            at,
            format!("an empty span: `from` is {} to {}", from[0], from[1]),
        ));
    }
}

/// Each name once: the second and later copies are problems at their own
/// index.
pub(crate) fn unique<'a>(p: &mut Vec<Problem>, at: &str, names: impl Iterator<Item = &'a String>) {
    let mut seen = std::collections::BTreeSet::new();
    for (index, name) in names.enumerate() {
        if name.trim().is_empty() {
            p.push(Problem::new(format!("{at}[{index}]"), "has no name"));
        } else if !seen.insert(name.as_str()) {
            p.push(Problem::new(
                format!("{at}[{index}]"),
                format!("`{name}` is used twice"),
            ));
        }
    }
}
