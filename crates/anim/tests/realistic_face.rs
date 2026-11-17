//! The realistic base body's face, as shipped, against the facial profile
//! its manifest names (Character Studio overhaul CO6). These read
//! `assets/characters/realistic/RealisticParts.glb` and
//! `Realistic.base.json`: rebuild the package after changing its face
//! (`assets/characters/realistic/README.md`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use axcenity_anim::{FaceRig, built_in_face_profile, check_face};
use axcenity_assets::{Model, ModelFormat, Pick, load_models};
use axcenity_core::face::FACE_CONTROLS;

fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters/realistic")
}

fn manifest() -> serde_json::Value {
    let text = std::fs::read_to_string(folder().join("Realistic.base.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// One object of the parts file, by its path (`AxcenityRealistic/Body`).
fn object(path: &str) -> Model {
    let bytes = std::fs::read(folder().join("RealisticParts.glb")).unwrap();
    let picks = [Pick::Objects(vec![path.to_owned()])];
    load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
        .unwrap()
        .remove(0)
}

/// The blend shapes of `model` that move at least one vertex.
fn moving_shapes(model: &Model) -> BTreeSet<String> {
    let mut moving = BTreeSet::new();
    for primitive in &model.primitives {
        for morph in &primitive.morphs {
            let moves = morph
                .positions
                .iter()
                .any(|offset| offset.iter().any(|x| x.abs() > 1e-6));
            if moves {
                moving.insert(model.morph_targets[morph.target as usize].name.clone());
            }
        }
    }
    moving
}

fn part(manifest: &serde_json::Value, name: &str) -> Model {
    let parts = manifest["model"]["parts"].as_array().unwrap();
    let path = parts
        .iter()
        .filter_map(|part| part.as_str())
        .find(|path| path.rsplit('/').next() == Some(name))
        .unwrap_or_else(|| panic!("the manifest lists no part `{name}`"));
    object(path)
}

/// CO6's first criterion: `check_face` finds nothing wrong with the
/// realistic body and the profile its manifest names. Not even the note
/// that some standard controls move nothing.
#[test]
fn the_realistic_bodys_face_fits_its_profile_with_nothing_left_out() {
    let manifest = manifest();
    let name = manifest["face"]["profile"].as_str().unwrap();
    let profile = built_in_face_profile(name).expect("a built-in profile");
    let body = object(manifest["model"]["body"].as_str().unwrap());
    let problems = check_face(&body, name, &profile);
    let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
    assert!(problems.is_empty(), "{}", lines.join("\n"));
}

/// Every one of the standard controls moves the face: a blend shape that
/// moves vertices of the body, an eye or jaw bone, or (the tongue's) a
/// shape of the part that has the tongue.
#[test]
fn every_standard_control_moves_the_realistic_face() {
    let manifest = manifest();
    let name = manifest["face"]["profile"].as_str().unwrap();
    let profile = built_in_face_profile(name).unwrap();
    let body = object(manifest["model"]["body"].as_str().unwrap());
    let (rig, _) = FaceRig::bind(&body, name, &profile);
    assert_eq!(rig.controls_used(), FACE_CONTROLS.len());
    let on_body = moving_shapes(&body);
    let on_teeth = moving_shapes(&part(&manifest, "Teeth"));
    let by_bone = [
        "Eye_Look_Up",
        "Eye_Look_Down",
        "Eye_Look_Left",
        "Eye_Look_Right",
    ];
    let still: Vec<&str> = FACE_CONTROLS
        .iter()
        .map(|control| control.name)
        .filter(|name| !by_bone.contains(name))
        .filter(|name| !on_body.contains(*name) && !on_teeth.contains(*name))
        .collect();
    assert!(still.is_empty(), "controls that move no vertex: {still:?}");
    // The eyes and the jaw are bones the manifest names.
    let bones = &body.rig.as_ref().unwrap().bones;
    let has = |bone: &str| bones.iter().any(|b| b.name == bone);
    for eye in manifest["face"]["eye_bones"].as_array().unwrap() {
        assert!(has(eye.as_str().unwrap()));
    }
    assert!(has(manifest["face"]["jaw_bone"].as_str().unwrap()));
}

/// The face's parts follow the body's live shapes by name (architecture
/// section 14): each carries, moving, the shapes of the controls that
/// move the skin it sits on, and no part has an expression shape the
/// body lacks (it would never be driven).
#[test]
fn the_faces_parts_carry_the_shapes_they_follow() {
    let manifest = manifest();
    let body = object(manifest["model"]["body"].as_str().unwrap());
    let on_body: BTreeSet<&str> = body
        .morph_targets
        .iter()
        .map(|target| target.name.as_str())
        .collect();
    let lids = [
        "Blink_Left",
        "Blink_Right",
        "Eye_Closed_Left",
        "Eye_Closed_Right",
        "Eye_Squint_Left",
        "Eye_Squint_Right",
        "Eye_Wide_Left",
        "Eye_Wide_Right",
    ];
    let brows = [
        "Brow_Raise_Left",
        "Brow_Raise_Right",
        "Brow_Lower_Left",
        "Brow_Lower_Right",
        "Brow_Inward_Left",
        "Brow_Inward_Right",
    ];
    // (The teeth go with the mouth when an identity moves it: its height,
    // and since R4 the philtrum's length.)
    let mouth = [
        "Jaw_Forward",
        "Tongue_Out",
        "Tongue_Up",
        "Tongue_Down",
        "Face_Mouth_Height",
        "Face_Philtrum_Length",
    ];
    let wanted: [(&str, &[&str]); 4] = [
        ("Lashes", &lids),
        ("EyeShell", &lids),
        ("Brows", &brows),
        ("Teeth", &mouth),
    ];
    for (name, shapes) in wanted {
        let model = part(&manifest, name);
        let moving = moving_shapes(&model);
        for shape in shapes {
            assert!(moving.contains(*shape), "{name} doesn't move with {shape}");
        }
        for target in &model.morph_targets {
            assert!(
                on_body.contains(target.name.as_str()),
                "{name} has `{}`, which the body lacks",
                target.name
            );
        }
    }
}

/// The ARKit mapping (CO6): the engine's built-in `arkit` profile says
/// which control each of ARKit's face shapes plays. Every control it
/// uses is one this head plays, through the `axcenity` profile, by a
/// shape that moves vertices (the body's, or the tongue's part's) or by
/// an eye or jaw bone. So ARKit face data mapped onto the controls
/// drives the realistic head with nothing dropped but the seven ARKit
/// shapes the controls have no place for (`co6/README.md` lists them).
#[test]
fn every_control_the_arkit_profile_uses_plays_on_the_realistic_head() {
    use axcenity_core::face::face_control;

    let manifest = manifest();
    let name = manifest["face"]["profile"].as_str().unwrap();
    let body = object(manifest["model"]["body"].as_str().unwrap());
    let (rig, problems) = FaceRig::bind(&body, name, &built_in_face_profile(name).unwrap());
    assert!(problems.is_empty());
    let on_body = moving_shapes(&body);
    let on_teeth = moving_shapes(&part(&manifest, "Teeth"));
    let by_bone = [
        "Eye_Look_Up",
        "Eye_Look_Down",
        "Eye_Look_Left",
        "Eye_Look_Right",
        "Jaw_Open",
        "Jaw_Left",
        "Jaw_Right",
    ];
    let arkit = built_in_face_profile("arkit").unwrap();
    let mut played = 0;
    for (control, shapes) in &arkit.controls {
        assert!(!shapes.is_empty(), "{control} names no ARKit shape");
        let index = face_control(control).unwrap_or_else(|| panic!("{control} isn't a control"));
        assert!(
            rig.moves(index),
            "{control} moves nothing on the realistic head"
        );
        let moving = on_body.contains(control.as_str()) || on_teeth.contains(control.as_str());
        assert!(
            moving || by_bone.contains(&control.as_str()),
            "{control} has neither a moving shape nor a bone"
        );
        played += 1;
    }
    // All the standard controls but Tongue_Up and Tongue_Down, which
    // ARKit has no shape for.
    assert_eq!(played, FACE_CONTROLS.len() - 2);
}
