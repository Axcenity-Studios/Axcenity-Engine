//! The item manifest (`*.item.json`): one garment, accessory or groom in a
//! library, with what it needs besides its model: its slot and layer, the
//! body regions it hides, its materials and variants, how it fits, its
//! tags, thumbnail and source.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::manifest::{
    Choice, Level, MaterialDriver, ShapeDriver, check_about, check_choice, check_id,
    check_material_driver, check_path, check_random, check_span, check_value_range, check_version,
    not_empty, unique,
};
use crate::material::{MaterialSlot, MaterialValues, check_values};
use crate::model_check::ModelRef;
use crate::{FormatError, Problem, pretty, read_versioned};

/// The item manifest's format version.
pub const ITEM_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemManifest {
    pub format_version: u32,
    /// Its ID in the library, such as `axcenity/tee`.
    pub id: String,
    pub label: String,
    pub kind: ItemKind,
    /// A slot of the base models it fits.
    pub slot: String,
    /// Garments only: the order outward, for pushing layers clear of each
    /// other. 0 underwear, 1 base (shirts, trousers, shoes), 2 mid
    /// (sweaters, vests), 3 outer (jackets, coats).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<u8>,
    #[serde(default)]
    pub tier: ContentTier,
    /// The base models it fits, by ID.
    pub bases: Vec<String>,
    /// Garments and accessories: the mesh. Grooms name theirs in `groom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelRef>,
    /// The body regions it hides, by ID.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hides: Vec<String>,
    /// What it hides triangle by triangle (wave 5, hiding the body under
    /// clothing): masks over the body's (or a lower layer's) texture
    /// coordinates, made by the fit's enclosure rule
    /// (`fit::hide_under`). Optional: an engine without them draws the
    /// skin as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hide_maps: Vec<HideMap>,
    /// Every material of its model, with its shading model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<MaterialSlot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<Variant>,
    pub fit: Fit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groom: Option<Groom>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// An image for the wardrobe browser, relative to the manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// A sentence for the browser.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// The item's own sliders (a hairstyle's length): they drive the
    /// item's shapes and material values, and a worn item holds their
    /// values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ItemParameter>,
    /// Other slots it occupies (a dress fills `bottom` too).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also_fills: Vec<String>,
    /// Items that must be worn with it, by ID.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    /// Items it can't be worn with, by ID (either naming the other is
    /// enough).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excludes: Vec<String>,
    /// Known limits, as notes for the browser; with a parameter and a
    /// range the editor can warn outside the range.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<Limitation>,
    /// Socket items only: how far a person may move, turn and scale it.
    /// None means it stays as made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjust: Option<Adjust>,
    /// Its content state (Paul's decision 12, ADR-0043,
    /// `asset-quality.md` §5): `production`, `legacy` or `draft`. Unset is
    /// legacy: a manifest from before the three states (an older
    /// project's copy of the stylized items). Every shipped item says it
    /// ([`crate::check_declared_state`]). Read it through
    /// [`ItemManifest::state`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,
    /// Its pose correctives (wave 5, `architecture.md` section 8): shapes
    /// of its model, baked from cloth simulation, that a joint drives by
    /// the rule the body's correctives use. Its model's file carries the
    /// same rules (`extras.axcenity_correctives`), which the runtime reads;
    /// none for an item without (it poses with the body's shapes alone).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correctives: Vec<PoseCorrective>,
}

/// A pose corrective an item carries (wave 5): its shape's weight is
/// `smoothstep((θ − start) / (full − start))`, θ the twist in degrees
/// about `axis` (a unit vector in the space of the bone's parent at rest)
/// of `bone`'s turn from rest. The body's correctives' rule
/// (`architecture.md` section 7), in the same form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoseCorrective {
    pub shape: String,
    pub bone: String,
    pub axis: [f32; 3],
    pub angles: [f32; 2],
}

impl PoseCorrective {
    /// Its problems, `at` its place in the manifest.
    fn check(&self, at: &str, p: &mut Vec<Problem>) {
        not_empty(p, &format!("{at}.bone"), &self.bone);
        let length = self.axis.iter().map(|v| v * v).sum::<f32>().sqrt();
        if !length.is_finite() || (length - 1.0).abs() > 1e-3 {
            p.push(Problem::new(
                format!("{at}.axis"),
                format!("is {:?}; it must be a unit vector", self.axis),
            ));
        }
        let [start, full] = self.angles;
        if !(start.is_finite() && full.is_finite() && start < full) {
            p.push(Problem::new(
                format!("{at}.angles"),
                format!(
                    "are {:?}; the start must be below the full angle",
                    self.angles
                ),
            ));
        }
    }
}

/// A mask of what an item hides under it, triangle by triangle: an image
/// over the first texture coordinates of the base body (or of a lower
/// layer's model, `under`). A triangle whose middle falls on a texel with
/// red at least 128 isn't drawn while the item is worn (and, for a lower
/// layer, while that layer is worn under it).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HideMap {
    /// The lower layer it hides, by item ID; unset is the base body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub under: Option<String>,
    /// The image, relative to the manifest.
    pub map: String,
}

/// An item's content state (Paul's decision 12 of 2026-11-18, ADR-0043;
/// `docs/development/character-studio/asset-quality.md` §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    /// Incomplete: never in normal character creation, presets or
    /// Randomize; listed only for content review.
    Draft,
    /// Older content kept for compatibility and optional use: hidden by
    /// default behind Character Studio's "Legacy items" filter, never
    /// drawn by Randomize; a character that wears it loads and draws it.
    Legacy,
    /// Approved: passed every check of its kind and the review.
    Production,
    /// A state this engine doesn't know (a newer engine's): read as a
    /// draft, so the manifest still loads (`#[serde(other)]` takes any
    /// unknown string here instead of failing the whole file).
    #[serde(other)]
    Other,
}

/// An item's own slider: shaped like a base model's parameter, without a
/// group, a mirror, a feature, bones or attributes (an item moves only
/// its own shapes and material values).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemParameter {
    pub id: String,
    pub label: String,
    pub range: [f32; 2],
    #[serde(default)]
    pub default: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<[f32; 2]>,
    /// Shapes of the item's own model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<ShapeDriver>,
    /// Number values of the item's own material slots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub material_values: Vec<MaterialDriver>,
    /// Named values instead of a slider (they wear nothing).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice>,
}

impl ItemParameter {
    /// Its level: `detailed` unless it says otherwise.
    pub fn level(&self) -> Level {
        match self.level {
            Some(Level::Other) | None => Level::Detailed,
            Some(level) => level,
        }
    }
}

/// A known limit of an item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Limitation {
    pub note: String,
    /// A parameter of the item or of the base model it is worn on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter: Option<String>,
    /// Where on `parameter` the item works well.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f32; 2]>,
}

/// How far a socket item may be moved (metres per model axis), turned
/// (degrees about each axis) and scaled. A part left out can't change.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Adjust {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<[[f32; 2]; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<[[f32; 2]; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<[f32; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// Worn over the body and deformed with it.
    Garment,
    /// Rigid, on a socket (or fitted like a garment: gloves, a watch strap).
    Accessory,
    /// Hair: card, strand and mesh tiers.
    Groom,
}

/// The two content tiers: built-in items are generated by the engine's
/// scripts; premium items are artist-made or imported.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentTier {
    #[default]
    BuiltIn,
    Premium,
}

/// How an item follows the body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum Fit {
    /// The item carries its own copies of these body shapes (the stylized
    /// humanoid's garments and hair); the body's weights are copied onto
    /// them by name.
    Shapes { follows: Vec<String> },
    /// Each vertex is bound to the body's surface (made at build or import,
    /// rebuilt when the body changes) and pushed clear of the layers under
    /// it by `margin` metres. `data` is the binding file, when it is kept.
    Binding {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<String>,
        #[serde(default = "default_margin")]
        margin: f32,
    },
    /// Rigid on a socket of the base model.
    Socket { socket: String },
}

fn default_margin() -> f32 {
    0.002
}

/// A colour or material variant: values for some of the item's materials.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub materials: BTreeMap<String, MaterialValues>,
    /// A picture of it, relative to the manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// Values for the item's own parameters.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, f32>,
}

/// A groom's tiers. Cards or a mesh are required; strands are optional and
/// never the only tier, so a character always loads and draws without
/// them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Groom {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cards: Option<ModelRef>,
    /// The groom file (`*.groom`), relative to the manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strands: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<ModelRef>,
}

/// Who made an item and its licence (an SPDX ID such as `CC0-1.0`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub author: String,
    pub licence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Original Axcenity work (class 5) or made from an imported asset
    /// (Paul's decision 11). Unset reads as original: every item made
    /// before 2026-11-18 is generated by the engine's own scripts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,
    /// An imported item's provenance record (`*.provenance.json`),
    /// relative to the manifest ([`crate::Provenance`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
}

/// Where an item comes from (`docs/development/character-studio/
/// references/README.md`, licence classes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Made by Axcenity's scripts or people (class 5), released CC0-1.0.
    Original,
    /// Made from a third-party asset (class 1 or 2): it names its
    /// provenance record.
    Imported,
}

impl ItemManifest {
    pub fn from_json(text: &str) -> Result<Self, FormatError> {
        read_versioned(text, "item manifest", ITEM_VERSION)
    }

    pub fn to_json(&self) -> String {
        pretty(self)
    }

    pub fn material(&self, name: &str) -> Option<&MaterialSlot> {
        self.materials.iter().find(|m| m.name == name)
    }

    /// Its content state, one of the three: `quality` unset is
    /// [`Quality::Legacy`] (made before the states), a state this engine
    /// doesn't know is [`Quality::Draft`].
    pub fn state(&self) -> Quality {
        match self.quality {
            None => Quality::Legacy,
            Some(Quality::Other) => Quality::Draft,
            Some(state) => state,
        }
    }

    /// Whether it is a draft ([`ItemManifest::state`]).
    pub fn is_draft(&self) -> bool {
        self.state() == Quality::Draft
    }

    /// Whether it is legacy content ([`ItemManifest::state`]).
    pub fn is_legacy(&self) -> bool {
        self.state() == Quality::Legacy
    }

    /// Whether it is production content: the only state Randomize draws
    /// and Character Studio lists by default.
    pub fn is_production(&self) -> bool {
        self.state() == Quality::Production
    }

    pub fn variant(&self, id: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.id == id)
    }

    pub fn parameter(&self, id: &str) -> Option<&ItemParameter> {
        self.parameters.iter().find(|p| p.id == id)
    }

    /// The slots it occupies: its own, then `also_fills`.
    pub fn slots(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.slot).chain(&self.also_fills)
    }

    /// Whether this item and `other` can't be worn together (either
    /// excludes the other).
    pub fn conflicts_with(&self, other: &ItemManifest) -> bool {
        self.excludes.contains(&other.id) || other.excludes.contains(&self.id)
    }

    /// Checks a value of the item parameter `id`: it exists, is finite and
    /// in range.
    pub(crate) fn check_parameter_value(
        &self,
        p: &mut Vec<Problem>,
        at: &str,
        id: &str,
        value: f32,
    ) {
        let Some(parameter) = self.parameter(id) else {
            p.push(Problem::new(
                at,
                format!("`{}` has no parameter `{id}`", self.id),
            ));
            return;
        };
        let [min, max] = parameter.range;
        if !value.is_finite() {
            p.push(Problem::new(at, "isn't a number"));
        } else if value < min || value > max {
            p.push(Problem::new(
                at,
                format!("{value} is outside its range, {min} to {max}"),
            ));
        }
    }

    fn check_capabilities(&self, p: &mut Vec<Problem>) {
        if let Some(about) = &self.about {
            not_empty(p, "about", about);
        }
        unique(p, "parameters", self.parameters.iter().map(|par| &par.id));
        for (index, parameter) in self.parameters.iter().enumerate() {
            let at = format!("parameters[{index}]");
            not_empty(p, &format!("{at}.label"), &parameter.label);
            check_value_range(p, &at, &parameter.id, parameter.range, parameter.default);
            check_random(p, &at, &parameter.id, parameter.range, parameter.random);
            check_about(p, &at, &parameter.about);
            for (k, choice) in parameter.choices.iter().enumerate() {
                let at = format!("{at}.choices[{k}]");
                check_choice(p, &at, &parameter.id, parameter.range, choice);
                if !choice.items.is_empty() {
                    p.push(Problem::new(at, "an item's choice can't wear items"));
                }
            }
            for (k, driver) in parameter.shapes.iter().enumerate() {
                let at = format!("{at}.shapes[{k}]");
                not_empty(p, &at, &driver.shape);
                check_span(p, &at, driver.from, driver.to);
            }
            for (k, driver) in parameter.material_values.iter().enumerate() {
                let at = format!("{at}.material_values[{k}]");
                check_material_driver(p, &at, driver, self.material(&driver.material));
            }
        }
        for (index, slot) in self.materials.iter().enumerate() {
            crate::material::check_slot(p, &format!("materials[{index}]"), slot);
        }
        for (index, variant) in self.variants.iter().enumerate() {
            let at = format!("variants[{index}]");
            if let Some(thumbnail) = &variant.thumbnail {
                check_path(p, &format!("{at}.thumbnail"), thumbnail);
            }
            for (id, value) in &variant.parameters {
                self.check_parameter_value(p, &format!("{at}.parameters.{id}"), id, *value);
            }
        }
        unique(p, "also_fills", self.also_fills.iter());
        for (index, slot) in self.also_fills.iter().enumerate() {
            if *slot == self.slot {
                p.push(Problem::new(
                    format!("also_fills[{index}]"),
                    format!("`{slot}` is its own slot"),
                ));
            }
        }
        for (list, ids) in [("requires", &self.requires), ("excludes", &self.excludes)] {
            unique(p, list, ids.iter());
            for (index, id) in ids.iter().enumerate() {
                let at = format!("{list}[{index}]");
                check_id(p, &at, id);
                if *id == self.id {
                    p.push(Problem::new(at, "names the item itself"));
                }
            }
        }
        for (index, limitation) in self.limitations.iter().enumerate() {
            let at = format!("limitations[{index}]");
            not_empty(p, &format!("{at}.note"), &limitation.note);
            if let Some([low, high]) = limitation.range {
                if limitation.parameter.is_none() {
                    p.push(Problem::new(&at, "a range needs a parameter"));
                }
                if !(low.is_finite() && high.is_finite() && low <= high) {
                    p.push(Problem::new(
                        &at,
                        format!("the range {low} to {high} goes nowhere"),
                    ));
                }
            }
        }
        if let Some(adjust) = &self.adjust {
            if !matches!(self.fit, Fit::Socket { .. }) {
                p.push(Problem::new(
                    "adjust",
                    "only an item on a socket can be moved, turned or scaled",
                ));
            }
            // Every limit holds the item as it was made (0 offset, 0 turn,
            // scale 1).
            let mut limits = |at: &str, span: [f32; 2], made: f32| {
                let [low, high] = span;
                if !(low.is_finite() && high.is_finite() && low <= made && made <= high) {
                    p.push(Problem::new(
                        at,
                        format!("{low} to {high} must be finite and hold {made}"),
                    ));
                }
            };
            for (k, span) in adjust.offset.iter().flatten().enumerate() {
                limits(&format!("adjust.offset[{k}]"), *span, 0.0);
            }
            for (k, span) in adjust.turn.iter().flatten().enumerate() {
                limits(&format!("adjust.turn[{k}]"), *span, 0.0);
            }
            if let Some(scale) = adjust.scale {
                limits("adjust.scale", scale, 1.0);
                if scale[0] <= 0.0 {
                    p.push(Problem::new("adjust.scale", "a scale must stay above 0"));
                }
            }
        }
    }

    /// The models it draws with: its model, or its groom's tiers.
    pub fn models(&self) -> Vec<&ModelRef> {
        let mut models: Vec<&ModelRef> = self.model.iter().collect();
        if let Some(groom) = &self.groom {
            models.extend(groom.cards.iter().chain(groom.mesh.iter()));
        }
        models
    }

    /// Checks the manifest on its own: ID and paths, what its kind needs
    /// (a garment's model and layer, a groom's drawable tier, a socket only
    /// for accessories), unique materials and variants, variant values.
    pub fn validate(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        let p = &mut problems;
        check_version(p, self.format_version, ITEM_VERSION);
        check_id(p, "id", &self.id);
        not_empty(p, "label", &self.label);
        not_empty(p, "slot", &self.slot);
        if self.bases.is_empty() {
            p.push(Problem::new("bases", "fits no base model"));
        }
        for (index, base) in self.bases.iter().enumerate() {
            check_id(p, &format!("bases[{index}]"), base);
        }
        for (at, model) in self.model_refs() {
            check_path(p, &format!("{at}.file"), &model.file);
        }
        match self.kind {
            ItemKind::Garment | ItemKind::Accessory => {
                if self.model.is_none() {
                    p.push(Problem::new(
                        "model",
                        format!("a {:?} needs a model", self.kind),
                    ));
                }
                if self.groom.is_some() {
                    p.push(Problem::new("groom", "only a groom has one"));
                }
            }
            ItemKind::Groom => {
                if self.model.is_some() {
                    p.push(Problem::new(
                        "model",
                        "a groom names its models in `groom` (cards, strands, mesh)",
                    ));
                }
                match &self.groom {
                    None => p.push(Problem::new("groom", "a groom needs its tiers")),
                    Some(groom) => {
                        if groom.cards.is_none() && groom.mesh.is_none() {
                            p.push(Problem::new(
                                "groom",
                                "needs cards or a mesh: a character must draw without strands",
                            ));
                        }
                        if let Some(strands) = &groom.strands {
                            check_path(p, "groom.strands", strands);
                        }
                    }
                }
            }
        }
        match (self.kind, self.layer) {
            (ItemKind::Garment, None) => {
                p.push(Problem::new("layer", "a garment needs a layer (0 to 3)"));
            }
            (ItemKind::Garment, Some(layer)) if layer > 3 => {
                p.push(Problem::new(
                    "layer",
                    format!("is {layer}; layers go from 0 to 3"),
                ));
            }
            (ItemKind::Accessory | ItemKind::Groom, Some(_)) => {
                p.push(Problem::new("layer", "only garments have layers"));
            }
            _ => {}
        }
        match &self.fit {
            Fit::Shapes { follows } => unique(p, "fit.follows", follows.iter()),
            Fit::Binding { data, margin } => {
                if let Some(data) = data {
                    check_path(p, "fit.data", data);
                }
                if !(margin.is_finite() && *margin >= 0.0) {
                    p.push(Problem::new(
                        "fit.margin",
                        format!("is {margin}; it can't be below 0"),
                    ));
                }
            }
            Fit::Socket { socket } => {
                not_empty(p, "fit.socket", socket);
                if self.kind != ItemKind::Accessory {
                    p.push(Problem::new(
                        "fit",
                        "only accessories hang on a socket; garments and grooms deform with the body",
                    ));
                }
            }
        }
        unique(p, "hides", self.hides.iter());
        let mut hidden_under = std::collections::BTreeSet::new();
        for (index, hide) in self.hide_maps.iter().enumerate() {
            let at = format!("hide_maps[{index}]");
            check_path(p, &format!("{at}.map"), &hide.map);
            if let Some(under) = &hide.under {
                check_id(p, &format!("{at}.under"), under);
                if *under == self.id {
                    p.push(Problem::new(&at, "names the item itself"));
                }
            }
            if !hidden_under.insert(hide.under.as_deref()) {
                p.push(Problem::new(
                    &at,
                    "a second mask for the same model (one for the body, one per lower layer)",
                ));
            }
        }
        unique(p, "materials", self.materials.iter().map(|m| &m.name));
        unique(p, "variants", self.variants.iter().map(|v| &v.id));
        for (index, variant) in self.variants.iter().enumerate() {
            let at = format!("variants[{index}]");
            not_empty(p, &format!("{at}.label"), &variant.label);
            for (name, values) in &variant.materials {
                match self.material(name) {
                    None => p.push(Problem::new(&at, format!("names no material `{name}`"))),
                    Some(slot) => p.extend(check_values(
                        &format!("{at}.materials.{name}"),
                        slot.shading,
                        values,
                    )),
                }
            }
        }
        if let Some(thumbnail) = &self.thumbnail {
            check_path(p, "thumbnail", thumbnail);
        }
        unique(p, "tags", self.tags.iter());
        if let Some(source) = &self.source {
            not_empty(p, "source.author", &source.author);
            not_empty(p, "source.licence", &source.licence);
        }
        self.check_capabilities(p);
        unique(p, "correctives", self.correctives.iter().map(|c| &c.shape));
        for (index, corrective) in self.correctives.iter().enumerate() {
            corrective.check(&format!("correctives[{index}]"), p);
        }
        problems
    }

    /// Checks it against a base model it says it fits: the slot, regions
    /// and socket exist there.
    pub fn check_against(&self, base: &crate::BaseManifest) -> Vec<Problem> {
        let mut problems = Vec::new();
        if base.slot(&self.slot).is_none() {
            problems.push(Problem::new(
                "slot",
                format!("`{}` has no slot `{}`", base.id, self.slot),
            ));
        }
        for (index, region) in self.hides.iter().enumerate() {
            if base.region(region).is_none() {
                problems.push(Problem::new(
                    format!("hides[{index}]"),
                    format!("`{}` has no region `{region}`", base.id),
                ));
            }
        }
        if let Fit::Socket { socket } = &self.fit
            && base.socket(socket).is_none()
        {
            problems.push(Problem::new(
                "fit",
                format!("`{}` has no socket `{socket}`", base.id),
            ));
        }
        for (index, slot) in self.also_fills.iter().enumerate() {
            if base.slot(slot).is_none() {
                problems.push(Problem::new(
                    format!("also_fills[{index}]"),
                    format!("`{}` has no slot `{slot}`", base.id),
                ));
            }
        }
        for (index, limitation) in self.limitations.iter().enumerate() {
            if let Some(id) = &limitation.parameter
                && self.parameter(id).is_none()
                && base.parameter(id).is_none()
            {
                problems.push(Problem::new(
                    format!("limitations[{index}]"),
                    format!("neither the item nor `{}` has a parameter `{id}`", base.id),
                ));
            }
        }
        problems
    }

    /// Its model references, each with where it is in the file.
    fn model_refs(&self) -> Vec<(&'static str, &ModelRef)> {
        let mut refs: Vec<(&'static str, &ModelRef)> =
            self.model.iter().map(|model| ("model", model)).collect();
        if let Some(groom) = &self.groom {
            refs.extend(groom.cards.iter().map(|model| ("groom.cards", model)));
            refs.extend(groom.mesh.iter().map(|model| ("groom.mesh", model)));
        }
        refs
    }
}
