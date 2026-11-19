//! The checks find what is wrong in a manifest, between an item and its
//! base, and in a description; sound files have no problems.

use std::collections::BTreeMap;

use super::{assert_problem, texts};
use crate::fixtures::{self, HAIR, TEE, WATCH};
use crate::{
    Colour, Constraint, Description, Fit, Library, MaterialValue, ModelRef, Region, WornItem,
};

#[test]
fn the_fixture_library_is_sound() {
    let library = fixtures::library();
    assert_eq!(texts(&fixtures::base().validate()), Vec::<String>::new());
    for text in [TEE, HAIR, WATCH] {
        let item = fixtures::item(text);
        assert_eq!(texts(&item.validate()), Vec::<String>::new(), "{}", item.id);
        assert_eq!(
            texts(&item.check_against(&fixtures::base())),
            Vec::<String>::new()
        );
    }
    assert_eq!(texts(&library.validate()), Vec::<String>::new());
}

#[test]
fn a_base_manifest_with_mistakes_reports_each() {
    let mut base = fixtures::base();
    base.id = "Test Body".to_owned();
    base.model.file = "C:/models/body.glb".to_owned();
    let mut copy = base.parameters[1].clone();
    copy.label = "Fat again".to_owned();
    base.parameters.push(copy);
    base.parameters[2].default = 4.0; // muscle: out of 0..1
    base.parameters[3].group = "nowhere".to_owned(); // build
    base.parameters[3].range = [1.0, -1.0];
    base.parameters[5].mirror = Some("frame".to_owned()); // eye_size_left
    base.parameters[0].shapes[0].from = [0.5, 0.5]; // frame: an empty span
    base.parameters[7].material_values[0].value = "sheen_tint".to_owned(); // tone
    base.corners[0].factors[1].parameter = "weight".to_owned();
    base.constraints.push(Constraint::MaxSum {
        parameters: vec!["build".to_owned(), "fat".to_owned()],
        limit: 1.0,
    });
    base.regions.push(Region {
        id: "hands".to_owned(),
        label: "Hands".to_owned(),
        mask_shape: None,
        map_index: None,
    });
    base.slots[1].id = "hair".to_owned();
    base.defaults.items.push("not an id".to_owned());
    let problems = base.validate();
    assert_problem(&problems, &["id", "Test Body"]);
    assert_problem(&problems, &["model.file"]);
    assert_problem(&problems, &["parameters[8]", "fat", "twice"]);
    assert_problem(&problems, &["parameters[2]", "default"]);
    assert_problem(&problems, &["parameters[3]", "nowhere"]);
    assert_problem(&problems, &["parameters[3]", "range"]);
    assert_problem(&problems, &["parameters[5]", "mirror"]);
    assert_problem(&problems, &["parameters[0].shapes[0]", "span"]);
    assert_problem(
        &problems,
        &["parameters[7].material_values[0]", "sheen_tint"],
    );
    assert_problem(&problems, &["corners[0]", "weight"]);
    assert_problem(&problems, &["constraints[2]", "build"]);
    assert_problem(&problems, &["regions[2]", "hands"]);
    assert_problem(&problems, &["slots[1]", "hair", "twice"]);
    assert_problem(&problems, &["defaults.items[1]"]);
}

#[test]
fn an_item_must_have_what_its_kind_needs() {
    // A groom needs cards or a mesh: strands are never the only tier.
    let mut groom = fixtures::item(HAIR);
    groom.groom.as_mut().unwrap().cards = None;
    assert_problem(&groom.validate(), &["groom", "cards or a mesh"]);
    groom.groom = None;
    assert_problem(&groom.validate(), &["groom"]);
    // A garment needs a model and a layer, and doesn't hang on a socket.
    let mut tee = fixtures::item(TEE);
    tee.model = None;
    tee.layer = None;
    tee.fit = Fit::Socket {
        socket: "LeftWristSocket".to_owned(),
    };
    let problems = tee.validate();
    assert_problem(&problems, &["model"]);
    assert_problem(&problems, &["layer"]);
    assert_problem(&problems, &["fit", "socket"]);
    // A groom has no model of its own, and an accessory needs one.
    let mut groom = fixtures::item(HAIR);
    groom.model = Some(ModelRef {
        file: "x.glb".to_owned(),
        object: None,
    });
    assert_problem(&groom.validate(), &["model", "groom"]);
    let mut watch = fixtures::item(WATCH);
    watch.model = None;
    assert_problem(&watch.validate(), &["model"]);
}

#[test]
fn an_item_with_mistakes_reports_each() {
    let mut tee = fixtures::item(TEE);
    tee.id = "tee".to_owned();
    tee.bases.clear();
    tee.materials.push(tee.materials[0].clone());
    tee.variants.push(tee.variants[0].clone());
    tee.variants[0].materials.insert(
        "Lining".to_owned(),
        BTreeMap::from([("tint".to_owned(), MaterialValue::Number(1.0))]),
    );
    tee.variants[1].materials.get_mut("Shirt").unwrap().insert(
        "tint".to_owned(),
        MaterialValue::Number(0.5), // a number where a colour goes
    );
    tee.fit = Fit::Binding {
        data: Some("/abs/tee.binding".to_owned()),
        margin: -0.01,
    };
    tee.thumbnail = Some("..\\tee.png".to_owned());
    let problems = tee.validate();
    assert_problem(&problems, &["id", "tee"]);
    assert_problem(&problems, &["bases"]);
    assert_problem(&problems, &["materials[1]", "Shirt", "twice"]);
    assert_problem(&problems, &["variants[1]", "navy", "twice"]);
    assert_problem(&problems, &["variants[0]", "Lining"]);
    assert_problem(&problems, &["variants[1]", "tint", "colour"]);
    assert_problem(&problems, &["fit", "data"]);
    assert_problem(&problems, &["fit", "margin"]);
    assert_problem(&problems, &["thumbnail"]);
}

#[test]
fn an_item_is_checked_against_its_base() {
    let base = fixtures::base();
    let mut tee = fixtures::item(TEE);
    tee.slot = "neck".to_owned();
    tee.hides.push("hands".to_owned());
    let problems = tee.check_against(&base);
    assert_problem(&problems, &["slot", "neck"]);
    assert_problem(&problems, &["hides[1]", "hands"]);
    let mut watch = fixtures::item(WATCH);
    watch.fit = Fit::Socket {
        socket: "NeckSocket".to_owned(),
    };
    assert_problem(&watch.check_against(&base), &["fit", "NeckSocket"]);
    // A library checks every item against every base it names.
    let mut library = fixtures::library();
    let mut hat = fixtures::item(WATCH);
    hat.id = "test/hat".to_owned();
    hat.bases = vec!["test/body".to_owned(), "test/other".to_owned()];
    library.add_item("test/items/hat.item.json", hat);
    assert_problem(&library.validate(), &["test/hat", "test/other"]);
}

fn description() -> Description {
    let mut description = Description::new("Ada", "test/body");
    description.parameters.insert("frame".to_owned(), 1.0);
    description.items = vec![
        WornItem::new("test/tee"),
        WornItem::new("test/hair"),
        WornItem::new("test/watch"),
    ];
    description
}

#[test]
fn a_sound_description_has_no_problems() {
    let library = fixtures::library();
    assert_eq!(
        texts(&description().validate(&library)),
        Vec::<String>::new()
    );
    // A slot that takes several holds two watches.
    let mut two = description();
    two.items.push(WornItem::new("test/watch"));
    assert_eq!(texts(&two.validate(&library)), Vec::<String>::new());
    // The base's default outfit is sound.
    let mut defaults = Description::new("New", "test/body");
    defaults.items = fixtures::base()
        .defaults
        .items
        .iter()
        .map(WornItem::new)
        .collect();
    assert_eq!(texts(&defaults.validate(&library)), Vec::<String>::new());
}

#[test]
fn a_description_with_mistakes_reports_each() {
    let mut library = fixtures::library();
    let mut other_tee = fixtures::item(TEE);
    other_tee.id = "test/other-tee".to_owned();
    library.add_item("test/items/other-tee.item.json", other_tee);
    let mut other_base_item = fixtures::item(WATCH);
    other_base_item.id = "test/elsewhere".to_owned();
    other_base_item.bases = vec!["test/other".to_owned()];
    library.add_item("test/items/elsewhere.item.json", other_base_item);

    let mut d = description();
    d.name = " ".to_owned();
    d.parameters.insert("wings".to_owned(), 1.0);
    d.parameters.insert("fat".to_owned(), 1.5);
    d.parameters.insert("muscle".to_owned(), f32::NAN);
    d.items.push(WornItem::new("test/cape"));
    d.items.push(WornItem::new("test/other-tee"));
    d.items.push(WornItem::new("test/elsewhere"));
    d.items[0].variant = Some("red".to_owned());
    d.items[0].materials.insert(
        "Lining".to_owned(),
        BTreeMap::from([("tint".to_owned(), MaterialValue::Colour(Colour::WHITE))]),
    );
    d.materials.insert(
        "Skin".to_owned(),
        BTreeMap::from([("melanin".to_owned(), MaterialValue::Number(2.0))]),
    );
    d.materials.insert("Scales".to_owned(), BTreeMap::new());
    let problems = d.validate(&library);
    assert_problem(&problems, &["name"]);
    assert_problem(&problems, &["parameters.wings"]);
    assert_problem(&problems, &["parameters.fat", "range"]);
    assert_problem(&problems, &["parameters.muscle", "number"]);
    assert_problem(&problems, &["items[3]", "test/cape"]);
    assert_problem(&problems, &["items[4]", "top", "test/tee"]);
    assert_problem(&problems, &["items[5]", "test/body"]);
    assert_problem(&problems, &["items[0].variant", "red"]);
    assert_problem(&problems, &["items[0].materials.Lining"]);
    assert_problem(&problems, &["materials.Skin", "melanin"]);
    assert_problem(&problems, &["materials.Scales"]);

    let mut lost = description();
    lost.base = "test/nobody".to_owned();
    assert_problem(&lost.validate(&library), &["base", "test/nobody"]);
    assert_problem(&lost.validate(&Library::new()), &["base"]);
}

/// Pose correctives (wave 5): an item's file may carry shapes a joint
/// drives; they read and write back as written, and a rule that can't
/// drive anything is a problem.
#[test]
fn an_items_pose_correctives_read_back_and_bad_ones_are_reported() {
    let mut item = fixtures::item(TEE);
    assert!(
        item.correctives.is_empty(),
        "optional: older files have none"
    );
    assert!(!item.to_json().contains("correctives"));
    let rule = |shape: &str, bone: &str, axis: [f32; 3], angles: [f32; 2]| crate::PoseCorrective {
        shape: shape.to_owned(),
        bone: bone.to_owned(),
        axis,
        angles,
    };
    item.correctives = vec![rule(
        "Corrective_Cloth_Shoulder_Lowered_Left",
        "LeftUpperArm",
        [0.0, 0.37768, -0.92594],
        [0.0, 35.0],
    )];
    assert_eq!(texts(&item.validate()), Vec::<String>::new());
    let again = crate::ItemManifest::from_json(&item.to_json()).unwrap();
    assert_eq!(again.correctives, item.correctives);
    item.correctives.extend([
        rule(
            "Corrective_Cloth_Shoulder_Lowered_Left",
            "LeftUpperArm",
            [0.0, 0.0, 1.0],
            [0.0, 35.0],
        ),
        rule(
            "Corrective_Cloth_Elbow_Bent_Left",
            "",
            [0.0, 0.0, 1.0],
            [0.0, 90.0],
        ),
        rule(
            "Corrective_Cloth_Knee_Bent_Left",
            "LeftLowerLeg",
            [0.0, 0.0, 0.0],
            [0.0, 90.0],
        ),
        rule(
            "Corrective_Cloth_Hip_Forward_Left",
            "LeftUpperLeg",
            [1.0, 0.0, 0.0],
            [30.0, 0.0],
        ),
        rule(
            "Corrective_Cloth_Hip_Back_Left",
            "LeftUpperLeg",
            [2.0, 0.0, 0.0],
            [0.0, 30.0],
        ),
    ]);
    let problems = item.validate();
    assert_problem(&problems, &["correctives[1]", "used twice"]);
    assert_problem(&problems, &["correctives[2].bone", "empty"]);
    assert_problem(&problems, &["correctives[3].axis"]);
    assert_problem(&problems, &["correctives[4].angles"]);
    assert_problem(&problems, &["correctives[5].axis", "unit"]);
}
