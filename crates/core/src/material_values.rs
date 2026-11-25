//! A character's material values on its MeshParts (Character Studio
//! overhaul CO3, `docs/development/character-studio/architecture.md`
//! section 10): each value of a material slot is an attribute of the
//! MeshPart whose model has that material, so it's saved, copied and
//! replicated with the part. `Humanoid:ApplyDescription` writes them and
//! the scene reads them: a value **modulates** the material as authored
//! (a `tint` multiplies its base colour and keeps its map), it never
//! replaces it.

/// What every material value attribute starts with.
pub const MATERIAL_VALUE_PREFIX: &str = "Mat_";

/// The name, in a material's place for values, of the colour map a
/// character's material draws in place of its model file's own
/// (`Mat_Skin_colour_map`, ADR-0048): a texture file in the project, as a
/// string. It is the one thing here that replaces a map; the values still
/// modulate what it shows.
pub const MATERIAL_COLOUR_MAP: &str = "colour_map";

/// The attribute holding the value `value` (`tint`, `roughness_scale`,
/// `melanin`, …) of the material named `material` (`Mat_Skin_tint`).
/// Characters an attribute name can't hold become `_`.
pub fn material_value_attribute(material: &str, value: &str) -> String {
    let mut name =
        String::with_capacity(MATERIAL_VALUE_PREFIX.len() + material.len() + 1 + value.len());
    name.push_str(MATERIAL_VALUE_PREFIX);
    let clean = |text: &str, name: &mut String| {
        name.extend(
            text.chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }),
        );
    };
    clean(material, &mut name);
    name.push('_');
    clean(value, &mut name);
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_material_value_is_an_attribute_name() {
        assert_eq!(material_value_attribute("Skin", "tint"), "Mat_Skin_tint");
        let odd = material_value_attribute("Hair.Secondary 2", "roughness_scale");
        assert_eq!(odd, "Mat_Hair_Secondary_2_roughness_scale");
        assert!(crate::is_attribute_name(&odd));
        assert_eq!(
            material_value_attribute("Skin", MATERIAL_COLOUR_MAP),
            "Mat_Skin_colour_map"
        );
    }
}
