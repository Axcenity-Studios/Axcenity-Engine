//! Makeup and markings on a character's skin (Character Studio R5,
//! ADR-0042). A base model offers **skin layers** (`skin_layers` in its
//! manifest: foundation, blush, freckles, a scar), each a mask in its
//! model file that its skin material draws; a description turns them on in
//! its optional `skin` block, with a colour, an opacity, a roughness and,
//! for markings, an amount and a seed. Resolving turns them into **draw
//! values** on the layer's material (`layer_<id>_<field>`, see
//! [`skin_layer_value`]), which the applier writes as the body's material
//! value attributes like every other value, so a change re-applies live:
//! no bake, no texture made per character.
//!
//! Nothing here is a new enum value of the format: a layer is named by a
//! string the base offers, and the block is an optional field an older
//! version keeps in [`Description::extra`](crate::Description::extra).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{BaseManifest, Colour, MaterialValue, MaterialValues, Problem, ShadingModel};

/// Which list of the description a skin layer goes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkinLayerKind {
    /// Cosmetics over the skin: foundation, blush, eyeshadow, eyeliner,
    /// lip colour, brow fill. A colour, an opacity and a roughness.
    Makeup,
    /// In the skin: freckles, moles, a scar (birthmarks and tattoos are
    /// ADR-0042's decals, later). An amount, a seed and an opacity.
    Marking,
    /// A kind a newer version adds: listed, never drawn by this one.
    #[serde(other)]
    Other,
}

/// A skin layer a base model offers (`BaseManifest::skin_layers`). Its
/// mask is in the model file, on `material`'s skin; this says what a
/// person sees and where a new layer starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkinLayerSpec {
    /// The ID descriptions use (`blush`); the renderer knows a fixed set
    /// (`docs/rendering/skin.md`).
    pub id: String,
    pub label: String,
    pub kind: SkinLayerKind,
    /// The base's material slot it is drawn on: one with skin shading.
    pub material: String,
    /// One sentence for the tooltip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// The colour a new layer has; without one, makeup takes the
    /// character's own tone (foundation) and a marking its pigment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<Colour>,
    /// The roughness a layer has where it covers (0 glossy to 1 matte);
    /// without one, the skin's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f32>,
    /// The opacity a layer starts at when turned on; 1 without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    /// A marking's share of its spots a new layer shows; all without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<f32>,
}

/// The description's `skin` block: makeup over the skin and markings in
/// it, each list in any order (the renderer draws them in the order of
/// `references/skin/README.md`: markings, then foundation up to brow fill).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Skin {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub makeup: Vec<SkinLayer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markings: Vec<SkinLayer>,
    /// Fields this version doesn't know (decals, later), kept as they were.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Skin {
    /// Whether it holds nothing (and isn't written).
    pub fn is_empty(&self) -> bool {
        self.makeup.is_empty() && self.markings.is_empty() && self.extra.is_empty()
    }

    /// The layer `id` in either list.
    pub fn layer(&self, id: &str) -> Option<&SkinLayer> {
        self.makeup
            .iter()
            .chain(&self.markings)
            .find(|layer| layer.layer == id)
    }
}

/// One layer turned on in a description.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkinLayer {
    /// The base's skin layer it is (`SkinLayerSpec::id`).
    pub layer: String,
    /// Off keeps its settings in the file and draws nothing.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// Makeup's colour; the base's when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<Colour>,
    /// 0 to 1: how much it covers where its mask is whole.
    #[serde(default = "full", skip_serializing_if = "is_full")]
    pub opacity: f32,
    /// 0 (glossy) to 1 (matte) where it covers; the base's when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f32>,
    /// A marking's share of its spots shown (0 to 1); the base's when
    /// absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<f32>,
    /// Which of a marking's spots show: another seed, other freckles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// Fields this version doesn't know (placement, later), kept.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

fn yes() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn full() -> f32 {
    1.0
}

fn is_full(value: &f32) -> bool {
    *value == 1.0
}

impl SkinLayer {
    /// Layer `id`, on, at full opacity, with the base's settings.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            layer: id.into(),
            enabled: true,
            colour: None,
            opacity: 1.0,
            roughness: None,
            amount: None,
            seed: None,
            extra: BTreeMap::new(),
        }
    }

    /// The layer as a person turning `spec` on first sees it: its default
    /// opacity and, for a marking, its default amount.
    pub fn from_spec(spec: &SkinLayerSpec) -> Self {
        let mut layer = Self::new(&spec.id);
        layer.opacity = spec.opacity.unwrap_or(1.0);
        layer.amount = spec.amount;
        layer
    }
}

/// The name of a skin layer's draw value on its material: `layer_blush_colour`.
/// `field` is `opacity`, `colour`, `roughness`, `amount` or `seed`. Written
/// by [`resolve`](crate::resolve) and read by `scene` (`material::modulate`)
/// from the material value attribute of that name.
pub fn skin_layer_value(layer: &str, field: &str) -> String {
    format!("layer_{layer}_{field}")
}

/// Checks the base's `skin_layers` (`BaseManifest::validate`): IDs once,
/// labels that say something, each on a material of the base with skin
/// shading, numbers from 0 to 1.
pub(crate) fn check_specs(p: &mut Vec<Problem>, base: &BaseManifest) {
    crate::manifest::unique(p, "skin_layers", base.skin_layers.iter().map(|l| &l.id));
    for (index, spec) in base.skin_layers.iter().enumerate() {
        let at = format!("skin_layers[{index}]");
        if spec.label.trim().is_empty() {
            p.push(Problem::new(format!("{at}.label"), "is empty"));
        }
        match base.material(&spec.material) {
            None => p.push(Problem::new(
                &at,
                format!("`{}` has no material `{}`", base.id, spec.material),
            )),
            Some(slot) if slot.shading != ShadingModel::Skin => p.push(Problem::new(
                &at,
                format!(
                    "`{}` is drawn with {:?} shading, not skin: it can't take a skin layer",
                    spec.material, slot.shading
                ),
            )),
            Some(_) => {}
        }
        for (name, value) in [
            ("roughness", spec.roughness),
            ("opacity", spec.opacity),
            ("amount", spec.amount),
        ] {
            share(p, &format!("{at}.{name}"), value);
        }
    }
}

/// A share from 0 to 1, when there is one.
fn share(p: &mut Vec<Problem>, at: &str, value: Option<f32>) {
    if let Some(value) = value
        && !(value.is_finite() && (0.0..=1.0).contains(&value))
    {
        p.push(Problem::new(at, format!("is {value}; it goes from 0 to 1")));
    }
}

/// Checks a description's `skin` block against its base: every layer is
/// one the base offers, in the list of its kind, once; its numbers are
/// from 0 to 1.
pub(crate) fn check_skin(p: &mut Vec<Problem>, base: &BaseManifest, skin: &Skin) {
    for (list, layers, kind) in [
        ("makeup", &skin.makeup, SkinLayerKind::Makeup),
        ("markings", &skin.markings, SkinLayerKind::Marking),
    ] {
        let mut seen = std::collections::BTreeSet::new();
        for (index, layer) in layers.iter().enumerate() {
            let at = format!("skin.{list}[{index}]");
            match base.skin_layer(&layer.layer) {
                None => p.push(Problem::new(
                    &at,
                    format!("`{}` offers no skin layer `{}`", base.id, layer.layer),
                )),
                Some(spec) if spec.kind != kind => p.push(Problem::new(
                    &at,
                    format!(
                        "`{}` is a {}, not {list}",
                        layer.layer,
                        match spec.kind {
                            SkinLayerKind::Makeup => "makeup layer",
                            SkinLayerKind::Marking => "marking",
                            SkinLayerKind::Other => "layer of a newer kind",
                        }
                    ),
                )),
                Some(_) => {}
            }
            if !seen.insert(layer.layer.as_str()) {
                p.push(Problem::new(
                    &at,
                    format!("`{}` is turned on twice", layer.layer),
                ));
            }
            share(p, &format!("{at}.opacity"), Some(layer.opacity));
            share(p, &format!("{at}.roughness"), layer.roughness);
            share(p, &format!("{at}.amount"), layer.amount);
        }
    }
}

/// The draw values of `skin`'s layers, by material slot: for every layer
/// the base offers that is on with some opacity, its `opacity` and
/// `roughness`, its `colour` (none: the character's own tone), and for a
/// marking its `amount` and `seed`; the base's settings fill what the
/// description leaves out. A layer the base doesn't offer, of a kind this
/// version doesn't know, or turned off, draws nothing.
pub(crate) fn draw_values(base: &BaseManifest, skin: &Skin) -> BTreeMap<String, MaterialValues> {
    let mut out: BTreeMap<String, MaterialValues> = BTreeMap::new();
    for spec in &base.skin_layers {
        let list = match spec.kind {
            SkinLayerKind::Makeup => &skin.makeup,
            SkinLayerKind::Marking => &skin.markings,
            SkinLayerKind::Other => continue,
        };
        let Some(layer) = list.iter().find(|layer| layer.layer == spec.id) else {
            continue;
        };
        let opacity = if layer.opacity.is_finite() {
            layer.opacity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if !layer.enabled || opacity <= 0.0 {
            continue;
        }
        let values = out.entry(spec.material.clone()).or_default();
        let mut set = |field: &str, value: MaterialValue| {
            values.insert(skin_layer_value(&spec.id, field), value);
        };
        set("opacity", MaterialValue::Number(opacity));
        if let Some(roughness) = layer.roughness.or(spec.roughness)
            && roughness.is_finite()
        {
            set(
                "roughness",
                MaterialValue::Number(roughness.clamp(0.0, 1.0)),
            );
        }
        if let Some(colour) = layer.colour.or(spec.colour) {
            set("colour", MaterialValue::Colour(colour));
        }
        if spec.kind == SkinLayerKind::Marking {
            let amount = layer
                .amount
                .or(spec.amount)
                .filter(|amount| amount.is_finite())
                .unwrap_or(1.0);
            set("amount", MaterialValue::Number(amount.clamp(0.0, 1.0)));
            // A seed is a whole number; f32 holds every one up to 2^24
            // exactly, and `scene` uses only its golden-ratio offset.
            set(
                "seed",
                MaterialValue::Number(layer.seed.unwrap_or(0) as f32),
            );
        }
    }
    out
}
