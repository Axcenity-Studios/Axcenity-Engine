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

fn preset(name: &str) -> usize {
    presets().iter().position(|p| p.name == name).unwrap()
}

/// The brief's task list (R1's part): a new character from a preset, a
/// body and a face change, undo, redo, a category reset, save, reopen:
/// the same character.
#[test]
fn a_walkthrough_through_intents_saves_and_reopens_the_same_character() {
    let folder = test_folder("walkthrough");
    let mut workspace = CharacterWorkspace::open_preset(&folder, 0).unwrap();
    assert_eq!(workspace.apply(Intent::Save), None);
    assert!(!workspace.dirty());

    // New, from the Athletic female preset: a fresh, unsaved character.
    workspace.apply(Intent::New(preset("Athletic female")));
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
    let mut workspace = CharacterWorkspace::open_preset(&folder, 0).unwrap();
    workspace.apply(Intent::Rename("Kept".to_owned()));
    let kept = workspace.save().unwrap();
    workspace.apply(set("fat", 0.5, Gesture::Commit));

    workspace.apply(Intent::New(preset("Female")));
    assert_eq!(workspace.asking, Some(Intent::New(preset("Female"))));
    assert_eq!(workspace.parameter("fat"), 0.5, "nothing happened yet");
    workspace.answer(None);
    assert_eq!(workspace.asking, None);
    assert_eq!(workspace.parameter("fat"), 0.5);

    workspace.apply(Intent::New(preset("Female")));
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
    let mut first = CharacterWorkspace::open_preset(&folder, 0).unwrap();
    first.apply(set("fat", 0.5, Gesture::Commit));
    let first_path = first.save().unwrap();
    let mut second = CharacterWorkspace::open_preset(&folder, 1).unwrap();
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
    let mut workspace = CharacterWorkspace::open_preset(&folder, 0).unwrap();
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
    let mut workspace = CharacterWorkspace::open_preset(&folder, 0).unwrap();
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
    let mut workspace =
        CharacterWorkspace::open_preset(&folder, preset("Athletic female")).unwrap();
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
    let mut workspace = CharacterWorkspace::open_preset(&folder, 0).unwrap();
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
