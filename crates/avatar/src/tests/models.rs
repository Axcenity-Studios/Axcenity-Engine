//! Manifests against their models: every shape, bone and material named
//! exists, and every material of a model has a shading model.

use super::{assert_problem, texts};
use crate::fixtures::{self, HAIR, TEE, WATCH, body_contents, contents, eyes_contents};

#[test]
fn a_manifest_that_matches_its_model_has_no_problems() {
    let base = fixtures::base();
    assert_eq!(
        texts(&base.check_model(&body_contents(), &[eyes_contents()])),
        Vec::<String>::new()
    );
    let tee = fixtures::item(TEE);
    assert_eq!(
        texts(&tee.check_model(&contents(&[], &["Shirt"]), &body_contents())),
        Vec::<String>::new()
    );
    let hair = fixtures::item(HAIR);
    assert_eq!(
        texts(&hair.check_model(&contents(&["Frame_Feminine"], &["Hair"]), &body_contents())),
        Vec::<String>::new()
    );
    let watch = fixtures::item(WATCH);
    let mut rigid = contents(&[], &["Metal"]);
    rigid.bones.clear(); // a rigid accessory has no skeleton
    assert_eq!(
        texts(&watch.check_model(&rigid, &body_contents())),
        Vec::<String>::new()
    );
}

#[test]
fn every_shape_bone_and_material_the_base_names_must_exist() {
    let base = fixtures::base();
    for (shape, at) in [
        ("Frame_Feminine", "parameters[0].shapes[0]"),
        ("Slim", "parameters[3].shapes[1]"),
        ("Feminine_Fat", "corners[0]"),
        ("Mask_Torso", "regions[0]"),
    ] {
        let mut body = body_contents();
        body.shapes.remove(shape);
        assert_problem(&base.check_model(&body, &[eyes_contents()]), &[at, shape]);
    }
    for (bone, at) in [
        ("Hips", "parameters[0].bones[0]"),
        ("LeftHand", "sockets[0]"),
        ("RightEye", "face.eye_bones[1]"),
        ("Jaw", "face.jaw_bone"),
        ("LeftUpperArmTwist", "skeleton.added_bones[0]"),
    ] {
        let mut body = body_contents();
        body.bones.remove(bone);
        assert_problem(&base.check_model(&body, &[eyes_contents()]), &[at, bone]);
    }
    // An added bone's parent must be there too.
    let mut body = body_contents();
    body.bones.remove("LeftUpperArm");
    assert_problem(
        &base.check_model(&body, &[eyes_contents()]),
        &["skeleton.added_bones[0]", "LeftUpperArm"],
    );
    // A listed material the model lacks, and one it has but isn't listed.
    let mut eyes = eyes_contents();
    eyes.materials.clear();
    assert_problem(
        &base.check_model(&body_contents(), &[eyes]),
        &["materials", "Iris"],
    );
    let mut body = body_contents();
    body.materials.insert("Mouth".to_owned());
    assert_problem(
        &base.check_model(&body, &[eyes_contents()]),
        &["Mouth", "shading"],
    );
    // One contents per part.
    assert_problem(&base.check_model(&body_contents(), &[]), &["model.parts"]);
}

#[test]
fn every_shape_bone_and_material_an_item_names_must_exist() {
    let hair = fixtures::item(HAIR);
    // The shape it follows: on the item, and on the body.
    assert_problem(
        &hair.check_model(&contents(&[], &["Hair"]), &body_contents()),
        &["fit.follows[0]", "Frame_Feminine"],
    );
    let mut body = body_contents();
    body.shapes.remove("Frame_Feminine");
    assert_problem(
        &hair.check_model(&contents(&["Frame_Feminine"], &["Hair"]), &body),
        &["fit.follows[0]", "body"],
    );
    // Its materials: listed and in the model, both ways.
    let tee = fixtures::item(TEE);
    assert_problem(
        &tee.check_model(&contents(&[], &[]), &body_contents()),
        &["materials", "Shirt"],
    );
    assert_problem(
        &tee.check_model(&contents(&[], &["Shirt", "Print"]), &body_contents()),
        &["Print", "shading"],
    );
    // A skinned item's bones are the body's.
    let mut item = contents(&[], &["Shirt"]);
    item.bones.insert("Tail".to_owned());
    assert_problem(&tee.check_model(&item, &body_contents()), &["bone", "Tail"]);
}

/// An item's pose correctives (wave 5, ADR-0044) name shapes of its own
/// model, driven by bones of the body it fits: a misspelt shape or bone
/// is a problem, not a corrective skipped at run time.
#[test]
fn an_items_pose_correctives_name_its_shapes_and_the_bodys_bones() {
    let mut tee = fixtures::item(TEE);
    tee.correctives = vec![crate::PoseCorrective {
        shape: "Corrective_Cloth_Shoulder_Lowered_Left".to_owned(),
        bone: "LeftUpperArm".to_owned(),
        axis: [0.0, 0.0, 1.0],
        angles: [0.0, 35.0],
    }];
    let shirt = contents(&["Corrective_Cloth_Shoulder_Lowered_Left"], &["Shirt"]);
    assert_eq!(
        texts(&tee.check_model(&shirt, &body_contents())),
        Vec::<String>::new()
    );
    assert_problem(
        &tee.check_model(&contents(&[], &["Shirt"]), &body_contents()),
        &[
            "correctives[0].shape",
            "Corrective_Cloth_Shoulder_Lowered_Left",
        ],
    );
    let mut body = body_contents();
    body.bones.remove("LeftUpperArm");
    let mut shirt = shirt.clone();
    shirt.bones.remove("LeftUpperArm");
    assert_problem(
        &tee.check_model(&shirt, &body),
        &["correctives[0].bone", "LeftUpperArm"],
    );
}
