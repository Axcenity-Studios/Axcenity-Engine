//! Character materials: each material slot of a base model or an item has
//! a shading model, and a description sets values for it. Values
//! **modulate** the material's own maps (a tint multiplies the base colour
//! map, a scale multiplies roughness), never replace them, and a material
//! with no values looks exactly as authored: every value's neutral setting
//! changes nothing. Until the renderer has the Skin, Eye, Cloth and Hair
//! shading models (CO7) they draw as Standard with the same values.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Problem;

/// How a material is lit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadingModel {
    /// The engine's PBR material: hard surfaces, rubber, leather, teeth.
    Standard,
    /// Pre-integrated diffusion and two specular lobes; screen-space
    /// scattering later.
    Skin,
    /// Iris under a refracting cornea, sclera, limbal ring.
    Eye,
    /// Sheen and anisotropy: cotton, wool, denim, satin.
    Cloth,
    /// Anisotropic strand lighting (cards and strands alike).
    Hair,
}

/// A material slot: a material of a model, by its name in the file, and how
/// it is lit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialSlot {
    pub name: String,
    pub shading: ShadingModel,
}

/// An sRGB colour, written `"#rrggbb"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Colour(pub [u8; 3]);

impl Colour {
    pub const WHITE: Colour = Colour([255, 255, 255]);

    /// Reads `#rrggbb`.
    pub fn parse(text: &str) -> Option<Colour> {
        let hex = text.strip_prefix('#')?;
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
        Some(Colour([byte(0)?, byte(2)?, byte(4)?]))
    }
}

impl fmt::Display for Colour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [r, g, b] = self.0;
        write!(f, "#{r:02x}{g:02x}{b:02x}")
    }
}

impl Serialize for Colour {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Colour {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Colour::parse(&text)
            .ok_or_else(|| serde::de::Error::custom(format!("`{text}` isn't a #rrggbb colour")))
    }
}

/// One value of a material: a number or a colour.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaterialValue {
    Number(f32),
    Colour(Colour),
}

/// A material's values, by name (see [`ShadingModel::values`]).
pub type MaterialValues = BTreeMap<String, MaterialValue>;

/// What kind of value a shading model takes under a name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueKind {
    /// A number from `min` to `max`; `neutral` leaves the material as made.
    Number { min: f32, max: f32, neutral: f32 },
    /// A colour; `neutral` (white for a tint) leaves the material as made.
    Colour { neutral: Colour },
}

/// A value a shading model takes: its name, kind and what it does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueSpec {
    pub name: &'static str,
    pub kind: ValueKind,
    pub meaning: &'static str,
}

const fn number(
    name: &'static str,
    min: f32,
    max: f32,
    neutral: f32,
    meaning: &'static str,
) -> ValueSpec {
    ValueSpec {
        name,
        kind: ValueKind::Number { min, max, neutral },
        meaning,
    }
}

const fn colour(name: &'static str, meaning: &'static str) -> ValueSpec {
    ValueSpec {
        name,
        kind: ValueKind::Colour {
            neutral: Colour::WHITE,
        },
        meaning,
    }
}

const STANDARD: &[ValueSpec] = &[
    colour("tint", "Multiplies the base colour map."),
    number("roughness_scale", 0.0, 2.0, 1.0, "Multiplies roughness."),
];

const SKIN: &[ValueSpec] = &[
    colour(
        "tint",
        "Multiplies the albedo after toning: a final adjustment.",
    ),
    number(
        "melanin",
        -1.0,
        1.0,
        0.0,
        "Lighter (-1) to darker (1) than the albedo map, along a melanin absorption curve rather than a plain darkening.",
    ),
    number(
        "redness",
        -1.0,
        1.0,
        0.0,
        "Less (-1) to more (1) haemoglobin: pallor to flush.",
    ),
    number("undertone", -1.0, 1.0, 0.0, "Cooler (-1) to warmer (1)."),
    number(
        "roughness_scale",
        0.5,
        1.5,
        1.0,
        "Multiplies roughness (oily to dry).",
    ),
    number(
        "detail_strength",
        0.0,
        2.0,
        1.0,
        "Strength of the pore and micro detail normal.",
    ),
    number(
        "scatter_scale",
        0.0,
        2.0,
        1.0,
        "Multiplies the subsurface scattering radius.",
    ),
];

const EYE: &[ValueSpec] = &[
    colour("iris_tint", "Multiplies the iris map."),
    colour("sclera_tint", "Multiplies the sclera map."),
    number(
        "pupil_size",
        0.5,
        2.0,
        1.0,
        "Pupil diameter relative to the authored one.",
    ),
    number(
        "limbal_ring",
        0.0,
        2.0,
        1.0,
        "Darkness of the ring round the iris, relative to authored.",
    ),
    number(
        "iris_depth",
        0.0,
        2.0,
        1.0,
        "Depth of the iris under the cornea (parallax), relative to authored.",
    ),
];

const CLOTH: &[ValueSpec] = &[
    colour("tint", "Multiplies the base colour map."),
    colour("sheen_tint", "Multiplies the authored sheen colour."),
    number("roughness_scale", 0.5, 1.5, 1.0, "Multiplies roughness."),
    number(
        "sheen_roughness_scale",
        0.5,
        1.5,
        1.0,
        "Multiplies sheen roughness.",
    ),
    number(
        "anisotropy_scale",
        0.0,
        2.0,
        1.0,
        "Multiplies the authored anisotropy (satin, silk).",
    ),
];

const HAIR: &[ValueSpec] = &[
    colour("tint", "Multiplies the colour after melanin and dye."),
    number(
        "melanin",
        -1.0,
        1.0,
        0.0,
        "Lighter (-1) to darker (1) than the authored colour, along a melanin curve.",
    ),
    number(
        "redness",
        -1.0,
        1.0,
        0.0,
        "Less (-1) to more (1) pheomelanin: ash to red.",
    ),
    colour("dye", "A dye colour, mixed in by `dye_amount`."),
    number(
        "dye_amount",
        0.0,
        1.0,
        0.0,
        "How much of `dye` replaces the natural colour.",
    ),
    number(
        "tip_lightening",
        0.0,
        1.0,
        0.0,
        "Lighter towards the tips (root-to-tip gradient).",
    ),
    number(
        "highlight",
        0.0,
        2.0,
        1.0,
        "Strength of the secondary (coloured) highlight.",
    ),
    number(
        "roughness_scale",
        0.5,
        1.5,
        1.0,
        "Multiplies the strands' roughness.",
    ),
];

impl ShadingModel {
    pub const ALL: [ShadingModel; 5] = [
        ShadingModel::Standard,
        ShadingModel::Skin,
        ShadingModel::Eye,
        ShadingModel::Cloth,
        ShadingModel::Hair,
    ];

    /// The values this model takes: the character material parameter set.
    pub fn values(self) -> &'static [ValueSpec] {
        match self {
            ShadingModel::Standard => STANDARD,
            ShadingModel::Skin => SKIN,
            ShadingModel::Eye => EYE,
            ShadingModel::Cloth => CLOTH,
            ShadingModel::Hair => HAIR,
        }
    }

    /// The value named `name`, if this model takes it.
    pub fn value(self, name: &str) -> Option<&'static ValueSpec> {
        self.values().iter().find(|spec| spec.name == name)
    }
}

/// Checks `values` against `shading`: every name is one it takes, of the
/// right kind and in range.
pub(crate) fn check_values(
    at: &str,
    shading: ShadingModel,
    values: &MaterialValues,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (name, value) in values {
        let at = format!("{at}.{name}");
        let Some(spec) = shading.value(name) else {
            problems.push(Problem::new(
                at,
                format!("`{name}` isn't a value of the {shading:?} shading model"),
            ));
            continue;
        };
        match (spec.kind, value) {
            (ValueKind::Number { min, max, .. }, MaterialValue::Number(number)) => {
                if !number.is_finite() || *number < min || *number > max {
                    problems.push(Problem::new(
                        at,
                        format!("`{name}` is {number}; it goes from {min} to {max}"),
                    ));
                }
            }
            (ValueKind::Colour { .. }, MaterialValue::Colour(_)) => {}
            (ValueKind::Number { .. }, MaterialValue::Colour(_)) => {
                problems.push(Problem::new(
                    at,
                    format!("`{name}` is a number, not a colour"),
                ));
            }
            (ValueKind::Colour { .. }, MaterialValue::Number(_)) => {
                problems.push(Problem::new(
                    at,
                    format!("`{name}` is a colour, not a number"),
                ));
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_read_and_write_as_hex() {
        assert_eq!(Colour::parse("#ff8000"), Some(Colour([255, 128, 0])));
        assert_eq!(Colour([10, 0, 255]).to_string(), "#0a00ff");
        for bad in ["ff8000", "#ff80", "#gg0000", "#ff80000"] {
            assert_eq!(Colour::parse(bad), None, "{bad}");
        }
        let values: MaterialValues =
            serde_json::from_str(r##"{"tint": "#102030", "melanin": 0.25}"##).unwrap();
        assert_eq!(values["tint"], MaterialValue::Colour(Colour([16, 32, 48])));
        assert_eq!(values["melanin"], MaterialValue::Number(0.25));
    }

    #[test]
    fn every_neutral_value_is_in_range_and_names_are_unique() {
        for model in ShadingModel::ALL {
            let mut names: Vec<&str> = model.values().iter().map(|spec| spec.name).collect();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), model.values().len(), "{model:?}");
            for spec in model.values() {
                if let ValueKind::Number { min, max, neutral } = spec.kind {
                    assert!(min < max && (min..=max).contains(&neutral), "{}", spec.name);
                }
            }
        }
    }

    #[test]
    fn values_are_checked_against_the_shading_model() {
        let values: MaterialValues = serde_json::from_str(
            r##"{"tint": "#ffffff", "melanin": 3.0, "sheen_tint": "#000000", "redness": "#ff0000"}"##,
        )
        .unwrap();
        let problems = check_values("materials.Skin", ShadingModel::Skin, &values);
        let messages: Vec<String> = problems.iter().map(ToString::to_string).collect();
        assert_eq!(problems.len(), 3, "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("melanin")));
        assert!(messages.iter().any(|m| m.contains("sheen_tint")));
        assert!(messages.iter().any(|m| m.contains("redness")));
        assert!(check_values("x", ShadingModel::Skin, &MaterialValues::new()).is_empty());
    }
}
