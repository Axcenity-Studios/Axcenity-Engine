//! The studio driven the way its panels drive it, through [`Intent`]s
//! (refinement plan DIS-17), and the audit's save and undo defects.

use super::edit::Scope;
use super::intent::{Gesture, Intent};
use super::selection::Level;
use super::*;
use crate::character_studio::test_folder;

fn set(id: &str, value: f32, gesture: Gesture) -> Intent {
    Intent::SetParameter {
        id: id.to_owned(),
        value,
        gesture,
    }
}

fn mirrored(id: &str, value: f32, gesture: Gesture) -> Intent {
    Intent::SetMirrored {
        id: id.to_owned(),
        value,
        gesture,
    }
}

fn preset(name: &str) -> Start {
    Start::shipped(name)
}

/// The brief's task list (R1's part): a new character from a preset, a
/// body and a face change, undo, redo, a category reset, save, reopen:
/// the same character.
#[test]
fn a_walkthrough_through_intents_saves_and_reopens_the_same_character() {
    let folder = test_folder("walkthrough");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert_eq!(workspace.apply(Intent::Save), None);
    assert!(!workspace.dirty());

    // New, from the Athletic female preset: a fresh, unsaved character.
    workspace.apply(Intent::New(preset("athletic-female")));
    assert!(workspace.dirty());
    assert_eq!(workspace.file(), None);
    assert!(!workspace.can_undo(), "a new character starts a fresh undo");
    assert_eq!(workspace.parameter("frame"), 1.0);
    assert_eq!(workspace.parameter("muscle"), 0.6);
    assert_eq!(
        workspace.worn_in("underwear_top").as_deref(),
        Some("axcenity/realistic-sports-bra")
    );

    // A body change: one click.
    workspace.apply(Intent::PickCategory(Category::Body));
    workspace.apply(set("fat", 0.5, Gesture::Commit));
    assert_eq!(workspace.parameter("fat"), 0.5);

    // A face change: a mirrored drag, one undo step.
    workspace.apply(Intent::PickCategory(Category::Face));
    workspace.apply(Intent::PickFeature(Some("eyes".to_owned())));
    for value in [0.1, 0.2, 0.3] {
        workspace.apply(mirrored("eye_size_left", value, Gesture::Drag));
    }
    workspace.apply(mirrored("eye_size_left", 0.4, Gesture::Commit));
    assert_eq!(workspace.parameter("eye_size_left"), 0.4);
    assert_eq!(workspace.parameter("eye_size_right"), 0.4);

    workspace.apply(Intent::Undo);
    assert_eq!(workspace.parameter("eye_size_left"), 0.0);
    assert_eq!(workspace.parameter("eye_size_right"), 0.0);
    assert_eq!(workspace.parameter("fat"), 0.5, "only the drag was undone");
    workspace.apply(Intent::Redo);
    assert_eq!(workspace.parameter("eye_size_right"), 0.4);

    // Reset the face: the eyes go back, the body stays.
    workspace.apply(Intent::Reset(Scope::Category(
        axcenity_avatar::Category::Face,
    )));
    assert_eq!(workspace.parameter("eye_size_left"), 0.0);
    assert_eq!(workspace.parameter("fat"), 0.5);
    assert_eq!(workspace.parameter("frame"), 1.0, "the body type stays");
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.parameter("eye_size_left"), 0.4);

    // Editor state is never in the description.
    let before = workspace.description().clone();
    for intent in [
        Intent::PickLevel(Level::Advanced),
        Intent::SetSymmetry(false),
        Intent::FocusView(ViewPreset::Side),
        Intent::SetLighting(Lighting::Sunset),
        Intent::SetBackdrop(Backdrop::Dark),
        Intent::PickCategory(Category::Expressions),
        Intent::PreviewExpression(Some("Happy".to_owned())),
        Intent::PickCategory(Category::Presets),
    ] {
        workspace.apply(intent);
    }
    assert_eq!(workspace.description(), &before);

    workspace.apply(Intent::Rename("Walker".to_owned()));
    workspace.apply(Intent::Save);
    assert!(!workspace.dirty(), "{:?}", workspace.notice);
    let path = workspace.file().unwrap().to_owned();
    assert!(
        path.ends_with("Walker.character.json"),
        "{}",
        path.display()
    );
    let reopened = CharacterWorkspace::open_file(&folder, &path).unwrap();
    assert_eq!(reopened.description(), workspace.description());
    assert!(!reopened.dirty());
    let _ = std::fs::remove_dir_all(&folder);
}

/// New and Open with unsaved changes ask first; Don't Save goes ahead,
/// Cancel leaves everything as it was.
#[test]
fn new_and_open_ask_to_save_unsaved_changes_first() {
    let folder = test_folder("walkthrough-ask");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::Rename("Kept".to_owned()));
    let kept = workspace.save().unwrap();
    workspace.apply(set("fat", 0.5, Gesture::Commit));

    workspace.apply(Intent::New(preset("female")));
    assert_eq!(workspace.asking, Some(Intent::New(preset("female"))));
    assert_eq!(workspace.parameter("fat"), 0.5, "nothing happened yet");
    workspace.answer(None);
    assert_eq!(workspace.asking, None);
    assert_eq!(workspace.parameter("fat"), 0.5);

    workspace.apply(Intent::New(preset("female")));
    workspace.answer(Some(false));
    assert_eq!(workspace.parameter("frame"), 1.0);
    assert_eq!(workspace.description().name, "New Character");
    assert!(workspace.dirty());

    // Save first, then open: the new one is saved, the kept one opens.
    workspace.apply(Intent::Open(kept.clone()));
    workspace.answer(Some(true));
    assert_eq!(workspace.description().name, "Kept");
    assert_eq!(workspace.file(), Some(kept.as_path()));
    assert!(!workspace.dirty());
    assert_eq!(
        workspace.parameter("fat"),
        0.0,
        "Kept was saved before the change"
    );
    assert!(
        folder
            .join(CHARACTERS_FOLDER)
            .join("New Character.character.json")
            .is_file()
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// Audit D-01: Save never writes over another character with the same
/// name; it saves under the first free one and says so. Saving the same
/// character again writes its own file.
#[test]
fn save_never_overwrites_another_character_with_the_same_name() {
    let folder = test_folder("walkthrough-d01");
    let mut first = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    first.apply(set("fat", 0.5, Gesture::Commit));
    let first_path = first.save().unwrap();
    let mut second = CharacterWorkspace::open_new(&folder, &Start::shipped("female")).unwrap();
    second.apply(Intent::Save);
    let second_path = second.file().unwrap().to_owned();
    assert_ne!(first_path, second_path, "the second got a file of its own");
    let kept = Description::from_json(&std::fs::read_to_string(&first_path).unwrap()).unwrap();
    assert_eq!(
        &kept,
        first.description(),
        "the first character is still there"
    );
    assert_eq!(second.description().name, "New Character 2");
    assert!(!second.dirty());
    let notice = second.notice.clone().unwrap();
    assert!(notice.contains("\"New Character 2\""), "{notice}");

    // Its own file is written again without a new name.
    first.apply(set("fat", 0.6, Gesture::Commit));
    assert_eq!(first.save().unwrap(), first_path);
    assert_eq!(first.description().name, "New Character");
    assert_eq!(shelf::list(&folder).len(), 2);
    let _ = std::fs::remove_dir_all(&folder);
}

/// Audit D-03: a rename then Save moves the file, with its Recent entry
/// and its places in packs; it doesn't leave a copy.
#[test]
fn a_rename_moves_the_saved_file_instead_of_copying_it() {
    let folder = test_folder("walkthrough-d03");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::Rename("Ada".to_owned()));
    let old = workspace.save().unwrap();
    packs::save_pack(&folder, None, "Cast", vec!["Ada.character.json".into()]).unwrap();
    workspace.apply(Intent::Rename("Ada Lovelace".to_owned()));
    let new = workspace.save().unwrap();
    assert!(new.exists());
    assert!(!old.exists(), "the old file is gone");
    let listed: Vec<String> = shelf::list(&folder).into_iter().map(|c| c.name).collect();
    assert_eq!(listed, ["Ada Lovelace"]);
    assert_eq!(recent::recent(&folder), vec![new.clone()]);
    assert_eq!(
        packs::packs(&folder)[0].characters,
        vec!["Ada Lovelace.character.json".to_owned()]
    );
    // A rename that only changes case keeps the one file.
    workspace.apply(Intent::Rename("ada lovelace".to_owned()));
    workspace.save().unwrap();
    assert_eq!(shelf::list(&folder).len(), 1);
    let _ = std::fs::remove_dir_all(&folder);
}

/// Audit D-05: a click that changes nothing (a slider clicked where it
/// is, the name field visited) keeps redo; an emptied name is "New
/// Character" once the field is left (D-04).
#[test]
fn a_change_of_nothing_keeps_redo() {
    let folder = test_folder("walkthrough-d05");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(set("fat", 0.5, Gesture::Commit));
    workspace.apply(Intent::Undo);
    assert!(workspace.can_redo());
    workspace.apply(set("fat", 0.0, Gesture::Commit));
    workspace.apply(Intent::Rename("New Character".to_owned()));
    assert!(workspace.can_redo(), "redo survives a change of nothing");
    workspace.apply(Intent::Redo);
    assert_eq!(workspace.parameter("fat"), 0.5);

    workspace.apply(Intent::Rename(String::new()));
    assert_eq!(workspace.description().name, "New Character");

    // Esc during a drag goes back to where it started, with no step.
    workspace.apply(set("fat", 0.9, Gesture::Drag));
    assert!(workspace.dragging());
    workspace.apply(Intent::CancelEdit);
    assert_eq!(workspace.parameter("fat"), 0.5);
    assert!(!workspace.dragging());
    workspace.apply(Intent::Undo);
    assert_eq!(
        workspace.parameter("fat"),
        0.0,
        "the cancelled drag left no step"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// Reset by scope: a feature (group), a category, everything; each one
/// undo step, the name kept.
#[test]
fn reset_puts_back_a_feature_a_category_or_everything() {
    let folder = test_folder("walkthrough-reset");
    let mut workspace = CharacterWorkspace::open_new(&folder, &preset("athletic-female")).unwrap();
    workspace.apply(Intent::Rename("Scoped".to_owned()));
    workspace.apply(set("nose_length", 0.5, Gesture::Commit));
    workspace.apply(set("chin_width", -0.5, Gesture::Commit));
    workspace.apply(set("height", 1.1, Gesture::Commit));

    workspace.apply(Intent::Reset(Scope::Group("nose".to_owned())));
    assert_eq!(workspace.parameter("nose_length"), 0.0);
    assert_eq!(workspace.parameter("chin_width"), -0.5);

    workspace.apply(Intent::Reset(Scope::Category(
        axcenity_avatar::Category::Body,
    )));
    assert_eq!(workspace.parameter("height"), 1.0);
    assert_eq!(workspace.parameter("muscle"), 0.0);
    assert_eq!(workspace.parameter("frame"), 1.0, "the body type stays");
    assert_eq!(
        workspace.worn_in("underwear_top").as_deref(),
        Some("axcenity/realistic-sports-bra"),
        "what the body type wears stays"
    );
    assert_eq!(workspace.parameter("chin_width"), -0.5, "the face is left");

    workspace.apply(Intent::SetMaterial {
        slot: "Skin".to_owned(),
        value: "tint".to_owned(),
        to: Some(axcenity_avatar::MaterialValue::Colour(
            axcenity_avatar::Colour([200, 160, 140]),
        )),
        gesture: Gesture::Commit,
    });
    assert!(workspace.material_value("Skin", "tint").is_some());
    workspace.apply(Intent::Reset(Scope::Category(
        axcenity_avatar::Category::Skin,
    )));
    assert_eq!(workspace.material_value("Skin", "tint"), None);
    assert!(workspace.description().materials.is_empty());

    workspace.apply(Intent::Reset(Scope::All));
    assert_eq!(workspace.parameter("chin_width"), 0.0);
    assert_eq!(
        workspace.parameter("frame"),
        0.0,
        "everything: the default character"
    );
    assert_eq!(workspace.description().name, "Scoped");
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.parameter("chin_width"), -0.5);
    let _ = std::fs::remove_dir_all(&folder);
}

/// A skin value is one undo step and reaches the preview's body as a
/// material attribute, without a new model (no rebake).
#[test]
fn a_skin_value_is_undoable_and_reaches_the_preview_without_a_rebake() {
    let folder = test_folder("walkthrough-skin");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let body_model = workspace.worn()[0].clone();
    workspace.apply(Intent::SetMaterial {
        slot: "Skin".to_owned(),
        value: "roughness_scale".to_owned(),
        to: Some(axcenity_avatar::MaterialValue::Number(1.3)),
        gesture: Gesture::Commit,
    });
    assert_eq!(
        workspace.material_value("Skin", "roughness_scale"),
        Some(axcenity_avatar::MaterialValue::Number(1.3))
    );
    assert_eq!(
        workspace.worn()[0],
        body_model,
        "a material value doesn't rebake"
    );
    let body = applier::body_of(&workspace.world, workspace.character.unwrap()).unwrap();
    let attribute = axcenity_core::material_value_attribute("Skin", "roughness_scale");
    assert_eq!(
        workspace
            .world
            .get(body)
            .unwrap()
            .attribute(&attribute)
            .and_then(axcenity_core::Value::as_number),
        Some(1.3f32 as f64)
    );
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.material_value("Skin", "roughness_scale"), None);
    let _ = std::fs::remove_dir_all(&folder);
}

/// The brief's task list, R1 complete (RA1, part 2): a new realistic and a
/// new stylized character; a body preset and a face preset; the face
/// randomized with the nose locked; A held, a change, compared, B kept;
/// saved and reopened the same; and the work recovered after a crash.
/// Since wave 3: one eye made different and mirrored back with the
/// asymmetry tools (one step), and a walk previewed, which nothing saves.
#[test]
fn the_whole_r1_walkthrough_with_presets_randomize_comparison_and_recovery() {
    use super::random::RandomScope;
    let folder = test_folder("walkthrough-r1");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();

    // A new stylized character, then a new realistic one (realistic is
    // the default; both families are offered).
    workspace.apply(Intent::New(Start::Family(axcenity_avatar::V1_BASE.into())));
    workspace.answer(Some(false));
    assert_eq!(workspace.description().base, axcenity_avatar::V1_BASE);
    workspace.apply(set("nose_width", 0.5, Gesture::Commit));
    workspace.apply(Intent::Rename("Toon".into()));
    workspace.apply(Intent::Save);
    let toon = workspace.file().unwrap().to_owned();
    workspace.apply(Intent::New(preset("female")));
    assert_eq!(workspace.description().base, DEFAULT_BASE);
    assert_eq!(workspace.parameter("frame"), 1.0);

    // A body preset, then a face preset: one undo step each.
    workspace.apply(Intent::ApplyPreset(
        "axcenity/realistic-body-athletic-female".into(),
    ));
    let library = workspace.library();
    let athletic = library
        .preset("axcenity/realistic-body-athletic-female")
        .unwrap();
    assert_eq!(workspace.parameter("muscle"), athletic.parameters["muscle"]);
    workspace.apply(Intent::ApplyPreset("axcenity/realistic-face-heart".into()));
    let heart = library.preset("axcenity/realistic-face-heart").unwrap();
    assert!(axcenity_avatar::matches_preset(
        workspace.description(),
        heart,
        &library
    ));
    workspace.apply(Intent::Undo);
    assert!(!axcenity_avatar::matches_preset(
        workspace.description(),
        heart,
        &library
    ));
    workspace.apply(Intent::Redo);

    // The face randomized with the nose locked: the nose and the body stay.
    workspace.apply(Intent::PickCategory(Category::Face));
    let nose: Vec<String> = library
        .base(DEFAULT_BASE)
        .unwrap()
        .parameters
        .iter()
        .filter(|p| p.feature.as_deref() == Some("nose"))
        .map(|p| p.id.clone())
        .collect();
    workspace.apply(Intent::SetLock {
        ids: nose.clone(),
        locked: true,
    });
    let before = workspace.description().clone();
    workspace.apply(Intent::Randomize(RandomScope::Category));
    assert_ne!(workspace.description(), &before);
    for id in &nose {
        assert_eq!(
            workspace.description().parameters.get(id),
            before.parameters.get(id),
            "{id}"
        );
    }
    assert_eq!(workspace.parameter("muscle"), athletic.parameters["muscle"]);
    assert_eq!(workspace.parameter("frame"), 1.0);

    // Hold A, change, compare, hold B, keep B.
    workspace.apply(Intent::Capture);
    let a = workspace.description().clone();
    workspace.apply(set("fat", 0.6, Gesture::Commit));
    workspace.apply(Intent::Capture);
    let b = workspace.description().clone();
    workspace.apply(set("fat", -0.3, Gesture::Commit));
    workspace.apply(Intent::ShowHeld(Some(0)));
    assert_eq!(workspace.shown(), &a);
    workspace.apply(Intent::SideBySide(true));
    workspace.apply(Intent::ShowHeld(Some(1)));
    workspace.apply(Intent::SideBySide(false));
    workspace.apply(Intent::KeepHeld(1));
    assert_eq!(workspace.description(), &b);

    // R4: the left eye made bigger on its own, then the eyes mirrored left
    // to right: one undo step, Symmetry still on.
    workspace.apply(Intent::PickCategory(Category::Face));
    workspace.apply(Intent::SetSymmetry(false));
    workspace.apply(set("eye_size_left", 0.35, Gesture::Commit));
    workspace.apply(Intent::SetSymmetry(true));
    assert_ne!(
        workspace.parameter("eye_size_left"),
        workspace.parameter("eye_size_right")
    );
    let uneven = workspace.description().clone();
    workspace.apply(Intent::Sides {
        tool: super::sides::SideTool::MirrorLeftToRight,
        of: super::sides::SidesOf::Feature {
            feature: "eyes".into(),
            level: Level::Detailed,
        },
    });
    assert_eq!(workspace.parameter("eye_size_right"), 0.35);
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.description(), &uneven, "one step");
    workspace.apply(Intent::Redo);
    let mirrored_eyes = workspace.description().clone();

    // R9: a walk previewed on the Expressions page, with a smile: the
    // character doesn't change and nothing of it is saved.
    workspace.apply(Intent::PickCategory(Category::Expressions));
    workspace.apply(Intent::PreviewMotion(Motion::Walk));
    workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
    for _ in 0..15 {
        workspace.advance(1.0 / 30.0);
    }
    assert_eq!(workspace.description(), &mirrored_eyes);
    assert_eq!(workspace.motion, Motion::Walk);

    // Save, reopen: the same character; the held versions aren't in it.
    workspace.apply(Intent::Rename("Heart".into()));
    workspace.apply(Intent::Save);
    assert!(!workspace.dirty(), "{:?}", workspace.notice);
    let path = workspace.file().unwrap().to_owned();
    let reopened = CharacterWorkspace::open_file(&folder, &path).unwrap();
    assert_eq!(reopened.description(), workspace.description());
    assert!(reopened.comparison.held.is_empty());
    // Neither the walk nor the smile went into the file.
    assert_eq!(reopened.motion, Motion::Idle);
    assert_eq!(reopened.expression, None);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("Walk") && !text.contains("Happy"), "{text}");
    assert_eq!(reopened.parameter("eye_size_right"), 0.35);
    drop(reopened);

    let toon_again = CharacterWorkspace::open_file(&folder, &toon).unwrap();
    assert_eq!(toon_again.description().base, axcenity_avatar::V1_BASE);
    assert_eq!(toon_again.parameter("nose_width"), 0.5);
    drop(toon_again);

    // Unsaved work, then a crash: the next open offers it back.
    workspace.apply(set("height", 1.05, Gesture::Commit));
    workspace.autosave_step(std::time::Instant::now());
    let lost = workspace.description().clone();
    std::mem::forget(workspace);
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert!(workspace.recovery.is_some());
    workspace.apply(Intent::Recover(true));
    assert_eq!(workspace.description(), &lost);
    assert_eq!(workspace.file(), Some(path.as_path()));
    assert!(workspace.dirty());
    let _ = std::fs::remove_dir_all(&folder);
}

/// Makeup and markings (R5, ADR-0042): a layer turned on, recoloured,
/// made fainter and turned off is one undo step each, reaches the
/// preview's body as material attributes without a new model, and a
/// Skin reset clears it.
#[test]
fn a_makeup_layer_is_one_undo_step_per_change_and_reaches_the_preview_live() {
    use axcenity_avatar::{Colour, SkinLayer};
    let folder = test_folder("walkthrough-makeup");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let body_model = workspace.worn()[0].clone();
    let body = applier::body_of(&workspace.world, workspace.character.unwrap()).unwrap();
    let attribute = |workspace: &CharacterWorkspace, name: &str| {
        workspace
            .world
            .get(body)
            .unwrap()
            .attribute(&axcenity_core::material_value_attribute("Skin", name))
            .cloned()
    };
    let set = |layer: SkinLayer, gesture| Intent::SetSkinLayer {
        id: layer.layer.clone(),
        marking: false,
        to: Some(layer),
        gesture,
    };

    // On: the base's starting settings.
    let spec = workspace
        .library()
        .base(&workspace.description().base)
        .and_then(|base| base.skin_layer("blush").cloned())
        .expect("the realistic base offers blush");
    workspace.apply(set(SkinLayer::from_spec(&spec), Gesture::Commit));
    assert!(workspace.skin_layer("blush").is_some_and(|l| l.enabled));
    assert!(attribute(&workspace, "layer_blush_opacity").is_some());
    // Recoloured.
    let mut blush = workspace.skin_layer("blush").unwrap().clone();
    blush.colour = Some(Colour([200, 60, 90]));
    workspace.apply(set(blush.clone(), Gesture::Commit));
    assert_eq!(
        attribute(&workspace, "layer_blush_colour"),
        Some(axcenity_core::Value::Color3(axcenity_core::Color3::new(
            200.0 / 255.0,
            60.0 / 255.0,
            90.0 / 255.0
        )))
    );
    // A fainter one in a drag: one step for the whole drag.
    for opacity in [0.3, 0.25, 0.2] {
        blush.opacity = opacity;
        workspace.apply(set(blush.clone(), Gesture::Drag));
    }
    blush.opacity = 0.15;
    workspace.apply(set(blush.clone(), Gesture::Commit));
    assert_eq!(workspace.skin_layer("blush").unwrap().opacity, 0.15);
    // Off: kept, drawn no more.
    blush.enabled = false;
    workspace.apply(set(blush, Gesture::Commit));
    assert!(attribute(&workspace, "layer_blush_opacity").is_none());
    assert_eq!(workspace.worn()[0], body_model, "makeup doesn't rebake");

    // Back through the steps: on, the colour, the drag, off.
    workspace.apply(Intent::Undo);
    assert!(workspace.skin_layer("blush").unwrap().enabled);
    workspace.apply(Intent::Undo);
    assert_eq!(
        workspace.skin_layer("blush").unwrap().opacity,
        spec.opacity.unwrap()
    );
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.skin_layer("blush").unwrap().colour, None);
    workspace.apply(Intent::Undo);
    assert!(workspace.skin_layer("blush").is_none());

    // A marking, and the Skin reset.
    workspace.apply(Intent::SetSkinLayer {
        id: "freckles".to_owned(),
        marking: true,
        to: Some(SkinLayer::new("freckles")),
        gesture: Gesture::Commit,
    });
    assert!(attribute(&workspace, "layer_freckles_amount").is_some());
    workspace.apply(Intent::Reset(Scope::Category(
        axcenity_avatar::Category::Skin,
    )));
    assert!(workspace.description().skin.is_empty());
    let _ = std::fs::remove_dir_all(&folder);
}
