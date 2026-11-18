//! Makeup and markings on the skin (R5, ADR-0042): the description's
//! optional `skin` block, the base manifest's `skin_layers`, their checks,
//! what they resolve to and how presets carry them.

use serde_json::json;

use crate::fixtures::{self, BASE};
use crate::tests::assert_problem;
use crate::{
    BaseManifest, Colour, Description, Library, MaterialValue, PresetKind, SkinLayer,
    SkinLayerKind, apply_preset, resolve, skin_layer_value,
};

/// The test body with makeup and markings on its `Skin`: foundation (in
/// the character's own tone), blush, lips, freckles and a scar.
fn base() -> BaseManifest {
    let mut value: serde_json::Value = serde_json::from_str(BASE).unwrap();
    value["skin_layers"] = json!([
        { "id": "foundation", "label": "Foundation", "kind": "makeup", "material": "Skin",
          "about": "Evens the skin's tone.", "roughness": 0.55 },
        { "id": "blush", "label": "Blush", "kind": "makeup", "material": "Skin",
          "colour": "#b5524a", "roughness": 0.5, "opacity": 0.4 },
        { "id": "lips", "label": "Lip colour", "kind": "makeup", "material": "Skin",
          "colour": "#8e2a35", "roughness": 0.3 },
        { "id": "freckles", "label": "Freckles", "kind": "marking", "material": "Skin",
          "amount": 0.5, "opacity": 0.7 },
        { "id": "scar", "label": "Scar", "kind": "marking", "material": "Skin" }
    ]);
    BaseManifest::from_json(&value.to_string()).expect("the layered base reads")
}

fn library() -> Library {
    let mut library = fixtures::library();
    library.add_base("test/Body.base.json", base());
    library
}

fn layer(id: &str) -> SkinLayer {
    SkinLayer::new(id)
}

/// A character with blush, lipstick (turned off), freckles and a scar.
fn layered() -> Description {
    let mut description = Description::new("Ada", "test/body");
    let mut blush = layer("blush");
    blush.colour = Some(Colour([200, 90, 80]));
    blush.opacity = 0.35;
    blush.roughness = Some(0.45);
    let mut lips = layer("lips");
    lips.enabled = false;
    lips.opacity = 0.8;
    let mut freckles = layer("freckles");
    freckles.amount = Some(0.6);
    freckles.seed = Some(7);
    let mut scar = layer("scar");
    scar.opacity = 0.9;
    description.skin.makeup = vec![blush, lips];
    description.skin.markings = vec![freckles, scar];
    description
}

#[test]
fn a_layered_description_round_trips() {
    let mut description = layered();
    // What a newer version adds to a layer and to the block survives.
    description.skin.makeup[0]
        .extra
        .insert("shimmer".to_owned(), json!(0.2));
    description
        .skin
        .extra
        .insert("decals".to_owned(), json!([{ "mark": "test/star" }]));
    let text = description.to_json();
    let back = Description::from_json(&text).unwrap();
    assert_eq!(back, description);
    assert_eq!(back.to_json(), text, "written the same way again");
    assert!(text.contains("\"skin\""), "{text}");
    assert!(text.contains("\"#c85a50\""), "colours are #rrggbb: {text}");
    // A layer on, at full opacity, writes neither.
    let plain: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(plain["skin"]["markings"][0].get("enabled").is_none());
    assert_eq!(plain["skin"]["makeup"][1]["enabled"], json!(false));
}

#[test]
fn a_description_without_makeup_writes_no_skin_block_and_an_older_file_reads() {
    let description = Description::new("Ada", "test/body");
    assert!(description.skin.is_empty());
    assert!(!description.to_json().contains("\"skin\""));
    // A file from before R5: no skin block at all.
    let old = r#"{ "format_version": 2, "name": "Ada", "base": "test/body",
                   "materials": { "Skin": { "melanin": 0.2 } } }"#;
    let read = Description::from_json(old).unwrap();
    assert!(read.skin.is_empty());
    assert!(read.extra.is_empty());
}

#[test]
fn the_skin_block_is_an_additive_field_older_versions_keep() {
    // An older engine has no `skin` field: it keeps the block in `extra`
    // (the R1 rule), so it round-trips there untouched. Shown by reading
    // the text as the plain map such a version keeps.
    let text = layered().to_json();
    let mut map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&text).unwrap();
    let skin = map.remove("skin").expect("the block");
    assert!(skin["makeup"].is_array() && skin["markings"].is_array());
    // No enum value of the format is new: a layer is named by a string the
    // base manifest offers, and its kind lives in the manifest.
    assert_eq!(skin["markings"][0]["layer"], json!("freckles"));
}

#[test]
fn a_base_offers_its_skin_layers_and_checks_them() {
    let base = base();
    assert!(base.validate().is_empty(), "{:?}", base.validate());
    let text = base.to_json();
    assert_eq!(BaseManifest::from_json(&text).unwrap(), base);
    let blush = base.skin_layer("blush").unwrap();
    assert_eq!(blush.kind, SkinLayerKind::Makeup);
    assert_eq!(blush.colour, Some(Colour([0xb5, 0x52, 0x4a])));
    assert_eq!(
        base.skin_layer("scar").unwrap().kind,
        SkinLayerKind::Marking
    );
    // A kind a newer version adds reads as `Other`.
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["skin_layers"][4]["kind"] = json!("tattoo");
    let newer = BaseManifest::from_json(&value.to_string()).unwrap();
    assert_eq!(newer.skin_layers[4].kind, SkinLayerKind::Other);

    let mut wrong = base.clone();
    wrong.skin_layers[1].id = "foundation".to_owned();
    wrong.skin_layers[2].material = "Iris".to_owned();
    wrong.skin_layers[3].material = "Nowhere".to_owned();
    wrong.skin_layers[0].roughness = Some(1.5);
    wrong.skin_layers[3].opacity = Some(-0.2);
    wrong.skin_layers[4].label = " ".to_owned();
    let problems = wrong.validate();
    assert_problem(&problems, &["skin_layers[1]", "foundation", "twice"]);
    assert_problem(&problems, &["skin_layers[2]", "Iris", "skin"]);
    assert_problem(&problems, &["skin_layers[3]", "Nowhere"]);
    assert_problem(&problems, &["skin_layers[0].roughness", "1.5"]);
    assert_problem(&problems, &["skin_layers[3].opacity", "-0.2"]);
    assert_problem(&problems, &["skin_layers[4].label"]);
}

#[test]
fn skin_layers_are_checked_against_what_the_base_offers() {
    let library = library();
    let description = layered();
    assert!(
        description.validate(&library).is_empty(),
        "{:?}",
        description.validate(&library)
    );
    let mut wrong = description.clone();
    wrong.skin.makeup.push(layer("glitter"));
    wrong.skin.makeup.push(layer("freckles"));
    wrong.skin.markings.push(layer("scar"));
    wrong.skin.makeup[0].opacity = 1.5;
    wrong.skin.makeup[0].roughness = Some(-0.1);
    wrong.skin.markings[0].amount = Some(2.0);
    wrong.skin.markings[1].opacity = f32::NAN;
    let problems = wrong.validate(&library);
    assert_problem(&problems, &["skin.makeup[2]", "glitter"]);
    assert_problem(&problems, &["skin.makeup[3]", "freckles", "marking"]);
    assert_problem(&problems, &["skin.markings[2]", "scar", "twice"]);
    assert_problem(&problems, &["skin.makeup[0].opacity", "1.5"]);
    assert_problem(&problems, &["skin.makeup[0].roughness", "-0.1"]);
    assert_problem(&problems, &["skin.markings[0].amount", "2"]);
    assert_problem(&problems, &["skin.markings[1].opacity"]);
    // A base with no skin layers takes none.
    let mut bare = Library::new();
    bare.add_base("test/Body.base.json", fixtures::base());
    assert_problem(
        &description.validate(&bare),
        &["skin.makeup[0]", "blush", "test/body"],
    );
}

fn number(values: &crate::MaterialValues, layer: &str, field: &str) -> Option<f32> {
    match values.get(&skin_layer_value(layer, field)) {
        Some(MaterialValue::Number(value)) => Some(*value),
        Some(other) => panic!("{layer} {field} is {other:?}"),
        None => None,
    }
}

#[test]
fn layers_resolve_to_draw_values_on_their_material() {
    let library = library();
    let mut description = layered();
    description.skin.makeup.push(layer("foundation"));
    let resolved = resolve(&library, &description).unwrap();
    let skin = &resolved.materials["Skin"];
    // Blush: the description's colour, opacity and roughness.
    assert_eq!(number(skin, "blush", "opacity"), Some(0.35));
    assert_eq!(number(skin, "blush", "roughness"), Some(0.45));
    assert_eq!(
        skin.get(&skin_layer_value("blush", "colour")),
        Some(&MaterialValue::Colour(Colour([200, 90, 80])))
    );
    // Lips are turned off: nothing drawn, though the description keeps them.
    assert!(skin.keys().all(|name| !name.contains("lips")), "{skin:?}");
    // Foundation without a colour: the base has none either, so it takes
    // the character's own tone (no colour value); its roughness is the
    // base's.
    assert_eq!(number(skin, "foundation", "opacity"), Some(1.0));
    assert_eq!(number(skin, "foundation", "roughness"), Some(0.55));
    assert!(!skin.contains_key(&skin_layer_value("foundation", "colour")));
    // Markings: amount and seed; the scar's amount is the whole mask.
    assert_eq!(number(skin, "freckles", "amount"), Some(0.6));
    assert_eq!(number(skin, "freckles", "seed"), Some(7.0));
    assert_eq!(number(skin, "freckles", "opacity"), Some(1.0));
    assert_eq!(number(skin, "scar", "opacity"), Some(0.9));
    assert_eq!(number(skin, "scar", "amount"), Some(1.0));
    // A layer left out of the description draws nothing; so does one at
    // opacity 0.
    let mut none = Description::new("Ada", "test/body");
    let mut off = layer("blush");
    off.opacity = 0.0;
    none.skin.makeup.push(off);
    let resolved = resolve(&library, &none).unwrap();
    assert!(
        resolved
            .materials
            .get("Skin")
            .is_none_or(|values| values.keys().all(|name| !name.starts_with("layer_"))),
        "{:?}",
        resolved.materials
    );
    // Defaults come from the base: a blush turned on with nothing else set.
    let mut plain = Description::new("Ada", "test/body");
    plain.skin.makeup.push(layer("blush"));
    plain.skin.markings.push(layer("freckles"));
    let resolved = resolve(&library, &plain).unwrap();
    let skin = &resolved.materials["Skin"];
    assert_eq!(number(skin, "blush", "opacity"), Some(1.0));
    assert_eq!(number(skin, "blush", "roughness"), Some(0.5));
    assert_eq!(
        skin.get(&skin_layer_value("blush", "colour")),
        Some(&MaterialValue::Colour(Colour([0xb5, 0x52, 0x4a])))
    );
    assert_eq!(number(skin, "freckles", "amount"), Some(0.5));
    assert_eq!(number(skin, "freckles", "seed"), Some(0.0));
}

#[test]
fn a_skin_layers_default_opacity_is_where_a_new_layer_starts() {
    let base = base();
    assert_eq!(
        SkinLayer::from_spec(base.skin_layer("blush").unwrap()).opacity,
        0.4
    );
    let freckles = SkinLayer::from_spec(base.skin_layer("freckles").unwrap());
    assert_eq!(freckles.opacity, 0.7);
    assert_eq!(freckles.amount, Some(0.5));
    assert!(freckles.enabled);
    assert_eq!(
        SkinLayer::from_spec(base.skin_layer("scar").unwrap()).opacity,
        1.0
    );
}

#[test]
fn skin_presets_carry_makeup_and_markings_and_other_kinds_leave_them() {
    let library = library();
    let preset = fixtures::preset(
        "skin",
        "rosy",
        r##""materials": { "Skin": { "redness": 0.3 } },
           "skin": { "makeup": [{ "layer": "blush", "opacity": 0.5 }],
                     "markings": [{ "layer": "freckles", "amount": 0.2 }] },"##,
    );
    assert_eq!(preset.preset().unwrap().kind, PresetKind::Skin);
    let mut character = layered();
    apply_preset(&mut character, &preset, &library).unwrap();
    assert_eq!(character.skin, preset.skin);
    // A body preset doesn't touch them.
    let body = fixtures::preset("body", "lean", r#""parameters": { "fat": 0.1 },"#);
    let mut kept = layered();
    apply_preset(&mut kept, &body, &library).unwrap();
    assert_eq!(kept.skin, layered().skin);
    // And a body preset that sets them is reported.
    assert!(crate::check_preset(&preset, &library).is_empty());
    let mut wrong = body.clone();
    wrong.skin = preset.skin.clone();
    assert_problem(
        &crate::check_preset(&wrong, &library),
        &["skin", "body preset"],
    );
}
