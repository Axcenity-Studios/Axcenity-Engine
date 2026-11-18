//! The R1 contracts (plan section 11.3): parameter metadata, features and
//! handles, item capabilities and what a worn item holds. Every field is
//! optional, so old files read to what they read before.

use std::collections::BTreeMap;

use serde_json::json;

use super::{assert_problem, texts};
use crate::fixtures::{self, BASE, BOB, CHARM, DRESS, HAIR, RICH_BASE, TEE, WATCH};
use crate::{
    Adjust, BaseManifest, Description, ItemManifest, Level, MaterialRole, ModelContents, PartFit,
    Transform, WornItem, resolve,
};

#[track_caller]
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}

#[test]
fn manifests_with_every_new_field_round_trip() {
    let base = fixtures::rich_base();
    assert_eq!(BaseManifest::from_json(&base.to_json()).unwrap(), base);
    for text in [BOB, DRESS, CHARM] {
        let item = fixtures::item(text);
        assert_eq!(ItemManifest::from_json(&item.to_json()).unwrap(), item);
    }
    // What the file says arrives.
    let fat = base.parameter("fat").unwrap();
    assert_eq!(fat.about.as_deref(), Some("How much body fat."));
    assert_eq!(fat.feature.as_deref(), Some("torso"));
    assert_eq!(fat.level, Some(Level::Quick));
    assert_eq!(fat.random, Some([0.0, 0.6]));
    assert_eq!(base.groups[0].icon.as_deref(), Some("body"));
    assert_eq!(
        base.slots[0].about.as_deref(),
        Some("What grows on the head.")
    );
    let skin = base.material("Skin").unwrap();
    assert_eq!(skin.label.as_deref(), Some("Skin"));
    assert_eq!(skin.values, ["melanin", "tint"]);
    assert_eq!(skin.role, Some(MaterialRole::Primary));
    assert_eq!(base.features.len(), 2);
    assert_eq!(base.feature("eyes").unwrap().anchors[1].bone, "RightEye");
    let handle = base.handle("eye_size_left").unwrap();
    assert_eq!(handle.drives[0].screen["front"], [1.0, 0.0]);
    assert_eq!(handle.drives[0].sensitivity, Some(1.5));
    assert_eq!(handle.views, ["front", "three_quarter"]);

    let bob = fixtures::item(BOB);
    assert_eq!(bob.about.as_deref(), Some("A chin-length bob."));
    assert_eq!(bob.parameters.len(), 2);
    assert_eq!(bob.parameter("length").unwrap().default, 0.5);
    assert_eq!(bob.variants[0].parameters["length"], 0.3);
    assert_eq!(bob.variants[0].thumbnail.as_deref(), Some("bob-blonde.png"));
    assert_eq!(bob.excludes, ["test/hood"]);
    assert_eq!(bob.limitations[0].range, Some([0.0, 0.7]));
    assert_eq!(fixtures::item(DRESS).also_fills, ["bottom"]);
    let charm = fixtures::item(CHARM);
    assert_eq!(charm.requires, ["test/watch"]);
    assert_eq!(charm.adjust.as_ref().unwrap().scale, Some([0.8, 1.25]));
}

#[test]
fn old_manifests_read_unchanged_and_write_none_of_the_new_fields() {
    let base = fixtures::base();
    for parameter in &base.parameters {
        assert_eq!(parameter.about, None);
        assert_eq!(parameter.feature, None);
        assert_eq!(parameter.level, None);
        assert_eq!(parameter.random, None);
    }
    assert!(base.features.is_empty() && base.handles.is_empty());
    assert!(
        base.materials
            .iter()
            .all(|m| m.label.is_none() && m.values.is_empty() && m.role.is_none())
    );
    // Written again, an old manifest says what it said: no new keys.
    let old: serde_json::Value = serde_json::from_str(BASE).unwrap();
    let again: serde_json::Value = serde_json::from_str(&base.to_json()).unwrap();
    for key in ["features", "handles"] {
        assert!(again.get(key).is_none(), "{key}");
    }
    assert_eq!(
        again["parameters"].as_array().unwrap().len(),
        old["parameters"].as_array().unwrap().len()
    );
    for parameter in again["parameters"].as_array().unwrap() {
        for key in ["about", "feature", "level", "random"] {
            assert!(parameter.get(key).is_none(), "{key} in {parameter}");
        }
    }
    for text in [TEE, HAIR, WATCH] {
        let item = fixtures::item(text);
        assert!(item.parameters.is_empty() && item.adjust.is_none() && item.about.is_none());
        let written: serde_json::Value = serde_json::from_str(&item.to_json()).unwrap();
        for key in [
            "about",
            "parameters",
            "also_fills",
            "requires",
            "excludes",
            "limitations",
            "adjust",
        ] {
            assert!(written.get(key).is_none(), "{key} in {}", item.id);
        }
    }
}

#[test]
fn an_old_description_reads_unchanged() {
    let text = r##"{
      "format_version": 2, "name": "Ada", "base": "test/body",
      "parameters": { "fat": 0.25 },
      "items": [{ "id": "test/tee", "variant": "navy" }, { "id": "test/hair" }]
    }"##;
    let description = Description::from_json(text).unwrap();
    assert_eq!(description.items[0], {
        let mut tee = WornItem::new("test/tee");
        tee.variant = Some("navy".to_owned());
        tee
    });
    assert!(
        description
            .items
            .iter()
            .all(|worn| worn.parameters.is_empty()
                && worn.transform.is_none()
                && worn.extra.is_empty())
    );
    let written = description.to_json();
    assert!(!written.contains("transform") && !written.contains("\"parameters\": {}"));
    assert_eq!(Description::from_json(&written).unwrap(), description);
}

#[test]
fn a_worn_item_keeps_its_parameters_transform_and_fields_it_doesnt_know() {
    let text = r##"{
      "format_version": 2, "name": "Bo", "base": "test/body",
      "items": [
        { "id": "test/bob", "variant": "blonde", "parameters": { "length": 0.4 }, "glow": { "on": true } },
        { "id": "test/charm", "transform": { "offset": [0, 0.005, 0], "turn": [0, 90, 0], "scale": 1.1 } }
      ]
    }"##;
    let description = Description::from_json(text).unwrap();
    let bob = &description.items[0];
    assert_eq!(bob.parameters["length"], 0.4);
    assert_eq!(bob.extra["glow"], json!({ "on": true }));
    let charm = description.items[1].transform.unwrap();
    assert_eq!(charm.offset, [0.0, 0.005, 0.0]);
    assert_eq!(charm.turn, [0.0, 90.0, 0.0]);
    assert_eq!(charm.scale, 1.1);
    let again = Description::from_json(&description.to_json()).unwrap();
    assert_eq!(again, description);
    assert_eq!(again.items[0].extra["glow"], json!({ "on": true }));
    // A transform that leaves out a part keeps it as it is.
    let partial: Transform = serde_json::from_str(r#"{ "turn": [0, 10, 0] }"#).unwrap();
    assert_eq!(partial.offset, [0.0; 3]);
    assert_eq!(partial.scale, 1.0);
    assert!(Transform::default().is_identity());
}

#[test]
fn a_parameters_level_comes_from_advanced_when_it_has_none() {
    let base = fixtures::rich_base();
    assert_eq!(base.parameter("frame").unwrap().level(), Level::Quick);
    assert_eq!(base.parameter("muscle").unwrap().level(), Level::Detailed);
    assert_eq!(
        base.parameter("eye_size_left").unwrap().level(),
        Level::Advanced
    );
    // No level: advanced when `advanced` is set, else detailed.
    assert_eq!(
        base.parameter("eye_size_right").unwrap().level(),
        Level::Advanced
    );
    assert_eq!(base.parameter("height").unwrap().level(), Level::Detailed);
    // A level a newer engine adds reads as no level, not a damaged file.
    let newer = RICH_BASE.replacen(r#""level": "detailed""#, r#""level": "expert""#, 1);
    let newer = BaseManifest::from_json(&newer).unwrap();
    assert_eq!(newer.parameter("muscle").unwrap().level(), Level::Detailed);
}

#[test]
fn a_parameters_feature_is_its_own_or_its_groups() {
    let base = fixtures::rich_base();
    let feature = |id: &str| {
        base.feature_of(base.parameter(id).unwrap())
            .map(|feature| feature.id.as_str())
    };
    assert_eq!(feature("fat"), Some("torso"));
    // No feature of its own: the group's ID names one.
    assert_eq!(feature("eye_size_right"), Some("eyes"));
    assert_eq!(feature("height"), None);
}

#[test]
fn the_rich_fixtures_are_sound() {
    let library = fixtures::rich_library();
    assert_eq!(
        texts(&fixtures::rich_base().validate()),
        Vec::<String>::new()
    );
    assert_eq!(texts(&library.validate()), Vec::<String>::new());
    let base = fixtures::rich_base();
    let mut body = fixtures::body_contents();
    body.shapes.extend(["Muscle".to_owned()]);
    assert_eq!(
        texts(&base.check_model(&body, &[fixtures::eyes_contents()])),
        Vec::<String>::new()
    );
    let bob = fixtures::item(BOB);
    let cards = fixtures::contents(
        &[
            "Frame_Feminine",
            "Hair_Length",
            "Hair_Part_Left",
            "Hair_Part_Right",
        ],
        &["Hair", "HairSecondary"],
    );
    assert_eq!(texts(&bob.check_model(&cards, &body)), Vec::<String>::new());
}

#[test]
fn parameter_metadata_features_and_handles_with_mistakes_report_each() {
    let mut base = fixtures::rich_base();
    base.parameters[1].feature = Some("belly".to_owned()); // fat
    base.parameters[2].level = Some(Level::Advanced); // muscle: advanced is false
    base.parameters[4].level = Some(Level::Quick); // eye_size_left: advanced is true
    base.parameters[1].random = Some([0.5, 2.0]); // fat: past its range
    base.parameters[2].random = Some([0.6, 0.2]); // muscle: backwards
    base.groups[1].about = Some(" ".to_owned());
    base.materials[0].values.push("sheen_tint".to_owned()); // not a skin value
    base.materials[0].values.push("melanin".to_owned()); // twice
    base.features.push(base.features[0].clone()); // torso twice
    base.features[1].radius = 0.0;
    base.features[1].anchors.clear();
    base.features[0].regions.push("wings".to_owned());
    base.features[0].anchors[0].offset[1] = f32::NAN;
    base.handles[0].feature = "nose".to_owned();
    base.handles[0].drives[0].parameter = "nose_length".to_owned();
    base.handles[0].drives[0].axis = [0.0; 3];
    base.handles[0].drives[0].travel = -0.01;
    base.handles[0].drives[0].sensitivity = Some(0.0);
    base.handles[1].mirror = Some("eye_size_left_again".to_owned());
    base.handles[1].drives.clear();
    let problems = base.validate();
    assert_problem(&problems, &["parameters[1]", "belly"]);
    assert_problem(&problems, &["parameters[2]", "level", "advanced"]);
    assert_problem(&problems, &["parameters[4]", "level", "advanced"]);
    assert_problem(&problems, &["parameters[1]", "random"]);
    assert_problem(&problems, &["parameters[2]", "random"]);
    assert_problem(&problems, &["groups[1].about"]);
    assert_problem(&problems, &["materials[0]", "sheen_tint"]);
    assert_problem(&problems, &["materials[0]", "melanin", "twice"]);
    assert_problem(&problems, &["features[2]", "torso", "twice"]);
    assert_problem(&problems, &["features[1]", "radius"]);
    assert_problem(&problems, &["features[1]", "anchor"]);
    assert_problem(&problems, &["features[0]", "wings"]);
    assert_problem(&problems, &["features[0].anchors[0]", "finite"]);
    assert_problem(&problems, &["handles[0]", "nose"]);
    assert_problem(&problems, &["handles[0].drives[0]", "nose_length"]);
    assert_problem(&problems, &["handles[0].drives[0]", "axis"]);
    assert_problem(&problems, &["handles[0].drives[0]", "travel"]);
    assert_problem(&problems, &["handles[0].drives[0]", "sensitivity"]);
    assert_problem(&problems, &["handles[1]", "mirror"]);
    assert_problem(&problems, &["handles[1]", "drives"]);
    // A handle whose mirror doesn't name it back.
    let mut base = fixtures::rich_base();
    base.handles[1].mirror = None;
    assert_problem(&base.validate(), &["handles[0]", "mirror", "back"]);
    // Two handles with one ID.
    let mut base = fixtures::rich_base();
    base.handles[1].id = "eye_size_left".to_owned();
    assert_problem(&base.validate(), &["handles[1]", "twice"]);
}

/// R2: a mirrored pair of handles is one feature's left and right: a
/// handle can't be its own mirror, and both sit on the same feature.
#[test]
fn a_handle_s_mirror_is_another_handle_of_its_feature() {
    let mut base = fixtures::rich_base();
    base.handles[0].mirror = Some("eye_size_left".to_owned());
    base.handles[1].mirror = None;
    assert_problem(&base.validate(), &["handles[0]", "its own mirror"]);
    let mut base = fixtures::rich_base();
    base.handles[1].feature = "torso".to_owned();
    let problems = base.validate();
    assert_problem(
        &problems,
        &["handles[0]", "eye_size_right", "another feature"],
    );
    assert_problem(
        &problems,
        &["handles[1]", "eye_size_left", "another feature"],
    );
    assert!(fixtures::rich_base().validate().is_empty());
}

/// Wave 3: a drive can list its axis and travel on another frame (the
/// realistic waist has a shape per frame); any other frame keeps its own.
#[test]
fn a_drive_on_a_listed_frame_uses_that_frames_axis_and_travel() {
    let mut base = fixtures::rich_base();
    let drive = &mut base.handles[0].drives[0];
    let own = (drive.axis, drive.travel);
    drive.frames.push(crate::FrameDrive {
        frame: 1.0,
        axis: [0.0, 1.0, 0.0],
        travel: 0.002,
    });
    assert_eq!(drive.on_frame(0.0), own);
    assert_eq!(drive.on_frame(0.3), own);
    assert_eq!(drive.on_frame(1.0), ([0.0, 1.0, 0.0], 0.002));
    assert_eq!(drive.on_frame(0.8), ([0.0, 1.0, 0.0], 0.002));
    assert!(base.validate().is_empty(), "{:?}", base.validate());
    // It round-trips, and a drive without frames writes none.
    let text = serde_json::to_string(&base).unwrap();
    let again: BaseManifest = serde_json::from_str(&text).unwrap();
    assert_eq!(again.handles[0].drives[0].frames.len(), 1);
    assert!(
        !serde_json::to_string(&base.handles[1])
            .unwrap()
            .contains("frames")
    );
    // A listed frame's axis and travel are checked as the drive's own.
    base.handles[0].drives[0].frames[0].axis = [0.0; 3];
    base.handles[0].drives[0].frames[0].travel = 0.0;
    let problems = base.validate();
    assert_problem(&problems, &["handles[0].drives[0]", "axis"]);
    assert_problem(&problems, &["handles[0].drives[0]", "travel"]);
}

#[test]
fn anchor_bones_must_be_in_the_model() {
    let mut base = fixtures::rich_base();
    base.features[0].anchors[0].bone = "Tail".to_owned();
    base.handles[1].anchor.bone = "Antenna".to_owned();
    let mut body = fixtures::body_contents();
    body.shapes.insert("Muscle".to_owned());
    let problems = base.check_model(&body, &[fixtures::eyes_contents()]);
    assert_problem(&problems, &["features[0].anchors[0]", "Tail"]);
    assert_problem(&problems, &["handles[1].anchor", "Antenna"]);
}

#[test]
fn item_capabilities_with_mistakes_report_each() {
    let mut bob = fixtures::item(BOB);
    bob.parameters.push(bob.parameters[0].clone()); // length twice
    bob.parameters[1].range = [2.0, 0.0]; // parting
    bob.parameters[0].default = 3.0;
    bob.parameters[0].shapes[0].shape = String::new();
    bob.parameters[0].material_values[0].material = "Scalp".to_owned();
    bob.parameters[1].choices[1].value = 5.0;
    bob.materials[1].values.push("melanine".to_owned());
    bob.variants[0].parameters.insert("curl".to_owned(), 1.0);
    bob.variants[0].parameters.insert("length".to_owned(), 9.0);
    bob.variants[0].thumbnail = Some("C:/bob.png".to_owned());
    bob.requires.push("bob".to_owned()); // not an ID
    bob.excludes.push("test/bob".to_owned()); // itself
    bob.limitations.push(crate::Limitation {
        note: " ".to_owned(),
        parameter: None,
        range: Some([0.0, 1.0]),
    });
    bob.also_fills.push("hair".to_owned()); // its own slot
    bob.adjust = Some(Adjust::default()); // not on a socket
    let problems = bob.validate();
    assert_problem(&problems, &["parameters[2]", "length", "twice"]);
    assert_problem(&problems, &["parameters[1]", "range"]);
    assert_problem(&problems, &["parameters[0]", "default"]);
    assert_problem(&problems, &["parameters[0].shapes[0]"]);
    assert_problem(&problems, &["parameters[0].material_values[0]", "Scalp"]);
    assert_problem(&problems, &["parameters[1].choices[1]"]);
    assert_problem(&problems, &["materials[1]", "melanine"]);
    assert_problem(&problems, &["variants[0]", "curl"]);
    assert_problem(&problems, &["variants[0]", "length", "outside"]);
    assert_problem(&problems, &["variants[0].thumbnail"]);
    assert_problem(&problems, &["requires[0]", "bob"]);
    assert_problem(&problems, &["excludes[1]", "itself"]);
    assert_problem(&problems, &["limitations[1].note"]);
    assert_problem(&problems, &["limitations[1]", "range", "parameter"]);
    assert_problem(&problems, &["also_fills[0]", "hair"]);
    assert_problem(&problems, &["adjust", "socket"]);

    // Adjust limits must hold the item as it is made.
    let mut charm = fixtures::item(CHARM);
    let adjust = charm.adjust.as_mut().unwrap();
    adjust.offset = Some([[0.01, 0.02], [-0.01, 0.01], [-0.01, 0.01]]);
    adjust.turn = Some([[f32::NAN, 1.0], [-1.0, 1.0], [-1.0, 1.0]]);
    adjust.scale = Some([1.1, 0.9]);
    let problems = charm.validate();
    assert_problem(&problems, &["adjust.offset", "0"]);
    assert_problem(&problems, &["adjust.turn"]);
    assert_problem(&problems, &["adjust.scale"]);
}

#[test]
fn item_capabilities_are_checked_against_the_base_and_the_library() {
    let base = fixtures::rich_base();
    let mut dress = fixtures::item(DRESS);
    dress.also_fills.push("cape".to_owned());
    dress.limitations.push(crate::Limitation {
        note: "Rides up on a long torso.".to_owned(),
        parameter: Some("torso_length".to_owned()),
        range: None,
    });
    let problems = dress.check_against(&base);
    assert_problem(&problems, &["also_fills[1]", "cape"]);
    assert_problem(&problems, &["limitations[0]", "torso_length"]);
    // A base parameter is a fine subject for a limitation.
    let mut bob = fixtures::item(BOB);
    bob.limitations[0].parameter = Some("fat".to_owned());
    assert_eq!(texts(&bob.check_against(&base)), Vec::<String>::new());

    // The library reports items it doesn't have.
    let mut library = fixtures::rich_library();
    let mut charm = fixtures::item(CHARM);
    charm.requires = vec!["test/chain".to_owned()];
    charm.excludes = vec!["test/glove".to_owned()];
    library.add_item("test/items/charm.item.json", charm);
    let problems = library.validate();
    assert_problem(&problems, &["test/charm", "requires", "test/chain"]);
    assert_problem(&problems, &["test/charm", "excludes", "test/glove"]);
    // A variant's picture must stay in the library.
    let mut library = fixtures::rich_library();
    let mut bob = fixtures::item(BOB);
    bob.variants[0].thumbnail = Some("../../../out.png".to_owned());
    library.add_item("test/items/bob.item.json", bob);
    assert_problem(&library.validate(), &["test/bob", "thumbnail", "out.png"]);
}

#[test]
fn an_items_parameter_shapes_must_be_in_its_model() {
    let bob = fixtures::item(BOB);
    let cards = fixtures::contents(
        &["Frame_Feminine", "Hair_Length"],
        &["Hair", "HairSecondary"],
    );
    let problems = bob.check_model(&cards, &fixtures::body_contents());
    assert_problem(&problems, &["parameters[1].shapes[0]", "Hair_Part_Left"]);
    assert_problem(&problems, &["parameters[1].shapes[1]", "Hair_Part_Right"]);
    let _ = ModelContents::default();
}

fn dressed(items: &[&str]) -> Description {
    let mut description = Description::new("Ada", "test/body");
    description.items = items.iter().map(|id| WornItem::new(*id)).collect();
    description
}

#[test]
fn a_description_with_item_capabilities_is_sound() {
    let library = fixtures::rich_library();
    let mut description = dressed(&["test/bob", "test/dress", "test/watch", "test/charm"]);
    description.items[0].variant = Some("blonde".to_owned());
    description.items[0]
        .parameters
        .insert("length".to_owned(), 0.9);
    description.items[0]
        .parameters
        .insert("parting".to_owned(), 2.0);
    description.items[3].transform = Some(Transform {
        offset: [0.005, 0.0, -0.02],
        turn: [0.0, 180.0, 0.0],
        scale: 0.8,
    });
    assert_eq!(texts(&description.validate(&library)), Vec::<String>::new());
}

#[test]
fn a_description_breaking_item_rules_reports_each() {
    let library = fixtures::rich_library();
    // The bob excludes the hood; the charm needs the watch; the dress
    // fills the bottom slot the skirt wants.
    let mut description = dressed(&[
        "test/bob",
        "test/hood",
        "test/charm",
        "test/dress",
        "test/skirt",
    ]);
    description.items[0]
        .parameters
        .insert("curl".to_owned(), 1.0);
    description.items[0]
        .parameters
        .insert("length".to_owned(), 1.5);
    description.items[0]
        .parameters
        .insert("parting".to_owned(), f32::NAN);
    description.items[2].transform = Some(Transform {
        offset: [0.0, 0.0, 0.5],
        turn: [45.0, 0.0, 0.0],
        scale: 2.0,
    });
    description.items[3].transform = Some(Transform {
        scale: 1.2,
        ..Transform::default()
    });
    let problems = description.validate(&library);
    assert_problem(&problems, &["items[0].parameters.curl"]);
    assert_problem(&problems, &["items[0].parameters.length", "range"]);
    assert_problem(&problems, &["items[0].parameters.parting", "number"]);
    assert_problem(&problems, &["items[1]", "test/bob", "test/hood"]);
    assert_problem(&problems, &["items[2]", "needs", "test/watch"]);
    assert_problem(&problems, &["items[2].transform", "offset"]);
    assert_problem(&problems, &["items[2].transform", "turn"]);
    assert_problem(&problems, &["items[2].transform", "scale"]);
    assert_problem(&problems, &["items[3].transform", "test/dress"]);
    assert_problem(&problems, &["items[4]", "bottom", "test/dress"]);
}

#[test]
fn resolving_gives_an_items_part_its_parameter_shapes_and_transform() {
    let library = fixtures::rich_library();
    let mut description = dressed(&["test/bob", "test/watch", "test/charm"]);
    let part = |resolved: &crate::Resolved, id: &str| {
        resolved
            .parts
            .iter()
            .find(|part| part.item.as_deref() == Some(id))
            .cloned()
            .unwrap()
    };
    // Defaults: length 0.5, parting middle.
    let resolved = resolve(&library, &description).unwrap();
    let bob = part(&resolved, "test/bob");
    close(bob.shapes["Hair_Length"], 0.5);
    close(bob.shapes["Hair_Part_Left"], 0.0);
    close(bob.shapes["Hair_Part_Right"], 0.0);
    // A parameter's material values go under the variant's and the worn
    // item's.
    assert_eq!(
        bob.materials["Hair"]["melanin"],
        crate::MaterialValue::Number(0.1)
    );
    // The body's shapes don't take an item's.
    assert!(!resolved.shapes.contains_key("Hair_Length"));
    // A variant's values, then the worn item's over them.
    description.items[0].variant = Some("blonde".to_owned());
    let bob = part(&resolve(&library, &description).unwrap(), "test/bob");
    close(bob.shapes["Hair_Length"], 0.3);
    assert_eq!(
        bob.materials["Hair"]["melanin"],
        crate::MaterialValue::Number(-0.6)
    );
    description.items[0]
        .parameters
        .insert("length".to_owned(), 0.8);
    description.items[0]
        .parameters
        .insert("parting".to_owned(), 2.0);
    let bob = part(&resolve(&library, &description).unwrap(), "test/bob");
    close(bob.shapes["Hair_Length"], 0.8);
    close(bob.shapes["Hair_Part_Left"], 1.0);
    close(bob.shapes["Hair_Part_Right"], 1.0);
    // Out of range is clamped, as a base parameter is.
    description.items[0]
        .parameters
        .insert("length".to_owned(), 7.0);
    let bob = part(&resolve(&library, &description).unwrap(), "test/bob");
    close(bob.shapes["Hair_Length"], 1.0);
    // An item with no parameters has no shapes of its own; a socket
    // item carries its transform.
    let resolved = resolve(&library, &description).unwrap();
    assert!(part(&resolved, "test/watch").shapes.is_empty());
    assert!(part(&resolved, "test/charm").transform.is_identity());
    description.items[2].transform = Some(Transform {
        scale: 1.2,
        ..Transform::default()
    });
    let charm = part(&resolve(&library, &description).unwrap(), "test/charm");
    assert_eq!(charm.fit, PartFit::Socket("LeftWristSocket".to_owned()));
    close(charm.transform.scale, 1.2);
}

#[test]
fn resolving_leaves_out_what_item_rules_forbid_and_says_why() {
    let library = fixtures::rich_library();
    let worn = |resolved: &crate::Resolved| -> Vec<String> {
        resolved
            .parts
            .iter()
            .filter_map(|part| part.item.clone())
            .collect()
    };
    // Excludes: the later one is worn, as for a slot that holds one.
    let resolved = resolve(&library, &dressed(&["test/bob", "test/hood"])).unwrap();
    assert_eq!(worn(&resolved), ["test/hood"]);
    assert_problem(&resolved.problems, &["items[0]", "test/bob", "test/hood"]);
    // A dress fills the bottom slot: the skirt after it takes its place.
    let resolved = resolve(&library, &dressed(&["test/dress", "test/skirt"])).unwrap();
    assert_eq!(worn(&resolved), ["test/skirt"]);
    assert_problem(&resolved.problems, &["items[0]", "bottom"]);
    // Requires: left out until what it needs is worn.
    let resolved = resolve(&library, &dressed(&["test/charm"])).unwrap();
    assert!(worn(&resolved).is_empty());
    assert_problem(&resolved.problems, &["items[0]", "test/watch"]);
    let resolved = resolve(&library, &dressed(&["test/charm", "test/watch"])).unwrap();
    assert_eq!(worn(&resolved), ["test/charm", "test/watch"]);
    assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
    let _ = BTreeMap::<String, f32>::new();
}
