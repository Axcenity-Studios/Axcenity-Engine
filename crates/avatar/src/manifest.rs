//! The base model manifest (`*.base.json`): what ships with a base model
//! besides its glTF file. Its parameters are the sliders Character Studio
//! draws and a description sets; each drives shapes, bones, attributes
//! and material values. It also names the body's regions (what clothes
//! hide), its slots, sockets, material slots, skeleton and face.

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
        unique(p, "sockets", self.sockets.iter().map(|socket| &socket.name));
        unique(p, "materials", self.materials.iter().map(|m| &m.name));
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
        let [min, max] = parameter.range;
        if !(min.is_finite() && max.is_finite() && min < max) {
            p.push(Problem::new(
                at,
                format!("`{}` has the range {min} to {max}", parameter.id),
            ));
        } else if !(min..=max).contains(&parameter.default) {
            p.push(Problem::new(
                at,
                format!(
                    "`{}`'s default {} is outside {min} to {max}",
                    parameter.id, parameter.default
                ),
            ));
        }
        for (k, choice) in parameter.choices.iter().enumerate() {
            let at = format!("{at}.choices[{k}]");
            not_empty(p, &format!("{at}.label"), &choice.label);
            for (n, id) in choice.items.iter().enumerate() {
                check_id(p, &format!("{at}.items[{n}]"), id);
            }
            if !(min..=max).contains(&choice.value) {
                p.push(Problem::new(
                    &at,
                    format!(
                        "`{}`'s choice {} is outside {min} to {max}",
                        parameter.id, choice.value
                    ),
                ));
            }
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
            check_span(p, &at, driver.from, driver.to);
            let Some(slot) = self.material(&driver.material) else {
                p.push(Problem::new(
                    at,
                    format!("names no material `{}`", driver.material),
                ));
                continue;
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
