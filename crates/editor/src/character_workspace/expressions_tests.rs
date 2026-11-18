//! The Expressions page (refinement R9, brief §10): the built-in
//! expressions as tiles, the face controls grouped by feature, "Neutral",
//! and the face preview mode. All of it is preview only: nothing here
//! enters the description, undo or a saved file.

use axcenity_core::{ClassName, Value};
use eframe::egui;

use super::expressions::{FaceGroup, ROWS, RowControls};
use super::intent::Intent;
use super::motion::Motion;
use super::stage::{Lighting, ViewPreset};
use super::*;
use crate::character_studio::test_folder;

const FRAME: f64 = 1.0 / 30.0;

fn run_for(workspace: &mut CharacterWorkspace, seconds: f64) {
    for _ in 0..(seconds / FRAME).round() as usize {
        workspace.advance(FRAME);
    }
}

fn body(workspace: &CharacterWorkspace) -> InstanceId {
    applier::body_of(&workspace.world, workspace.character.unwrap()).unwrap()
}

/// The preview body's morph weights as posed.
fn weights(workspace: &CharacterWorkspace) -> Vec<f32> {
    workspace.poses()[&body(workspace)].morph_weights().to_vec()
}

fn largest_difference(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max)
}

/// The preview's own FaceControls, if it has one.
fn preview_face(workspace: &CharacterWorkspace) -> Option<InstanceId> {
    workspace
        .world
        .get(body(workspace))?
        .children()
        .iter()
        .copied()
        .find(|child| {
            workspace.world.get(*child).is_some_and(|child| {
                child.class() == ClassName::FaceControls && child.name() == PREVIEW_FACE
            })
        })
}

fn property(workspace: &CharacterWorkspace, face: InstanceId, name: &str) -> Value {
    workspace
        .world
        .get(face)
        .unwrap()
        .property(name)
        .cloned()
        .unwrap()
}

/// Brief §10, the R9 row's "Done when": whatever is done on the page
/// (expressions picked, controls moved, the face preview turned on and
/// off, Neutral), saving gives the same file, and nothing is undoable.
#[test]
fn expression_edits_never_change_the_saved_file() {
    let folder = test_folder("expressions-saved");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let path = workspace.save().unwrap();
    let before = std::fs::read(&path).unwrap();
    let description = workspace.description().clone();

    workspace.apply(Intent::PickCategory(Category::Expressions));
    workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
    for (name, value) in [
        ("Mouth_Smile_Left", Some(0.4)),
        ("Mouth_Smile_Right", Some(0.4)),
        ("Jaw_Open", Some(0.7)),
        ("Brow_Raise_Left", Some(1.0)),
        ("Brow_Raise_Left", None),
    ] {
        workspace.apply(Intent::PreviewFaceControl {
            name: name.into(),
            value,
        });
    }
    workspace.apply(Intent::FacePreview(true));
    workspace.apply(Intent::FacePreviewMotion {
        blink: true,
        idle: true,
    });
    workspace.apply(Intent::SetLighting(Lighting::Neutral));
    run_for(&mut workspace, 0.5);
    workspace.apply(Intent::PreviewExpression(Some("Laughing".into())));
    workspace.apply(Intent::FacePreview(false));
    workspace.apply(Intent::PreviewExpression(None));
    run_for(&mut workspace, 0.2);

    assert_eq!(workspace.description(), &description);
    assert!(!workspace.dirty(), "the page changes nothing to save");
    assert!(!workspace.can_undo(), "the page makes no undo step");
    let again = workspace.save().unwrap();
    assert_eq!(again, path);
    assert_eq!(
        std::fs::read(&again).unwrap(),
        before,
        "saving after the page's edits gives the same file"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// A face control moves the preview's face; its slider starts at what the
/// expression shown gives it (Happy's smile), a move holds it over
/// the expression, and Neutral puts every control and the expression back.
#[test]
fn face_controls_move_the_face_over_the_expression_and_neutral_resets_them() {
    let folder = test_folder("expressions-controls");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::PickCategory(Category::Expressions));
    workspace.apply(Intent::PreviewMotion(Motion::Still));
    run_for(&mut workspace, 0.5);
    let neutral = weights(&workspace);

    workspace.apply(Intent::PreviewFaceControl {
        name: "Jaw_Open".into(),
        value: Some(0.6),
    });
    assert_eq!(workspace.face_value("Jaw_Open"), 0.6);
    run_for(&mut workspace, 0.5);
    let open = weights(&workspace);
    assert!(
        largest_difference(&neutral, &open) > 0.3,
        "the jaw opens the mouth's shapes"
    );
    assert!(
        preview_face(&workspace).is_some(),
        "a control alone is shown through the preview's face"
    );

    // An expression starts the sliders from its own values.
    workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
    assert!(
        workspace.face_controls.is_empty(),
        "an expression picked starts from its own values"
    );
    let smile = axcenity_anim::built_in_expression("Happy")
        .unwrap()
        .controls["Mouth_Smile_Left"];
    assert!(smile > 0.5);
    assert!((workspace.face_value("Mouth_Smile_Left") - smile).abs() < 1e-6);
    assert_eq!(workspace.face_value("Jaw_Open"), 0.0);
    workspace.apply(Intent::PreviewFaceControl {
        name: "Mouth_Smile_Left".into(),
        value: Some(0.2),
    });
    assert_eq!(workspace.face_value("Mouth_Smile_Left"), 0.2);
    assert_eq!(workspace.expression.as_deref(), Some("Happy"));
    // Taken off again, the expression's own value shows.
    workspace.apply(Intent::PreviewFaceControl {
        name: "Mouth_Smile_Left".into(),
        value: None,
    });
    assert!((workspace.face_value("Mouth_Smile_Left") - smile).abs() < 1e-6);

    // Neutral: no expression and no control held; the face rests.
    workspace.apply(Intent::PreviewFaceControl {
        name: "Brow_Lower_Left".into(),
        value: Some(1.0),
    });
    workspace.apply(Intent::PreviewExpression(Some("Neutral".into())));
    assert_eq!(workspace.expression, None);
    assert!(workspace.face_controls.is_empty());
    run_for(&mut workspace, 1.0);
    assert!(
        largest_difference(&neutral, &weights(&workspace)) < 0.02,
        "Neutral puts the face back at rest"
    );
    // Leaving the page takes the controls off too.
    workspace.apply(Intent::PreviewFaceControl {
        name: "Jaw_Open".into(),
        value: Some(1.0),
    });
    workspace.apply(Intent::PickCategory(Category::Body));
    assert!(workspace.face_controls.is_empty());
    assert!(preview_face(&workspace).is_none());
    let _ = std::fs::remove_dir_all(&folder);
}

/// The face preview mode: the camera close on the face, the body still,
/// blinking and idle motion off (unless asked), the neutral light, the
/// expression held; leaving puts the view, light and motion back.
#[test]
fn the_face_preview_frames_the_face_holds_it_still_and_leaving_restores_the_view() {
    let folder = test_folder("expressions-face-preview");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::PickCategory(Category::Expressions));
    workspace.apply(Intent::SetLighting(Lighting::Sunset));
    run_for(&mut workspace, 0.3);
    let (orbit, view, motion) = (workspace.orbit, workspace.view, workspace.motion.clone());
    assert_eq!(view, Some(ViewPreset::Face));
    assert_eq!(motion, Motion::Idle);

    workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
    workspace.apply(Intent::FacePreview(true));
    assert!(workspace.face_view.is_some());
    assert_eq!(workspace.motion, Motion::Still, "the body holds still");
    assert_eq!(workspace.lighting, Lighting::Neutral);
    run_for(&mut workspace, 0.6);
    let head = workspace.head().unwrap();
    let camera = workspace.orbit.camera();
    assert!(
        workspace.orbit.distance < orbit.distance,
        "closer than the Face view: {} against {}",
        workspace.orbit.distance,
        orbit.distance
    );
    assert!(
        (camera.look_at.y - head.y).abs() < 0.2 && (camera.look_at.x - head.x).abs() < 0.1,
        "aimed at the face: {:?} against the head bone at {head:?}",
        camera.look_at
    );
    assert!(workspace.orbit.yaw.abs() < 1.0, "straight on");
    let face = preview_face(&workspace).expect("the preview's face");
    assert_eq!(
        property(&workspace, face, "BlinkEnabled"),
        Value::Bool(false)
    );
    assert_eq!(property(&workspace, face, "IdleMotion"), Value::Number(0.0));

    // Held: over seven seconds (a blink comes every two to six) the face
    // and the body don't move.
    let held = weights(&workspace);
    let mut moved = 0.0f32;
    for _ in 0..(7.0 / FRAME) as usize {
        workspace.advance(FRAME);
        moved = moved.max(largest_difference(&held, &weights(&workspace)));
    }
    assert!(moved < 1e-4, "the face is held: it moved by {moved}");
    assert_eq!(workspace.expression.as_deref(), Some("Happy"));

    // Blinking when asked.
    workspace.apply(Intent::FacePreviewMotion {
        blink: true,
        idle: false,
    });
    let face = preview_face(&workspace).unwrap();
    assert_eq!(
        property(&workspace, face, "BlinkEnabled"),
        Value::Bool(true)
    );
    let mut blinked = 0.0f32;
    for _ in 0..(7.0 / FRAME) as usize {
        workspace.advance(FRAME);
        blinked = blinked.max(largest_difference(&held, &weights(&workspace)));
    }
    assert!(blinked > 0.3, "it blinks when asked: {blinked}");
    // Idle motion when asked: the face's and the body's.
    workspace.apply(Intent::FacePreviewMotion {
        blink: false,
        idle: true,
    });
    assert_eq!(workspace.motion, Motion::Idle);
    let face = preview_face(&workspace).unwrap();
    assert_eq!(property(&workspace, face, "IdleMotion"), Value::Number(0.5));

    // Leaving puts back what was there.
    workspace.apply(Intent::FacePreview(false));
    assert!(workspace.face_view.is_none());
    assert_eq!(workspace.orbit, orbit);
    assert_eq!(workspace.view, view);
    assert_eq!(workspace.lighting, Lighting::Sunset);
    assert_eq!(workspace.motion, motion);
    assert_eq!(
        workspace.expression.as_deref(),
        Some("Happy"),
        "the expression stays on the page"
    );

    // Leaving the page leaves the face preview too, and puts back the
    // view the page found.
    workspace.apply(Intent::FacePreview(true));
    workspace.apply(Intent::PickCategory(Category::Body));
    assert!(workspace.face_view.is_none());
    assert_eq!(workspace.lighting, Lighting::Sunset);
    assert_eq!(workspace.view, Some(ViewPreset::FullBody));
    let _ = std::fs::remove_dir_all(&folder);
}

/// The control rows: every standard face control the brief asks for
/// (smile, brows, squint, mouth opening, jaw, blink) is on a row, in the
/// six groups, and each row names real controls.
#[test]
fn the_face_controls_are_grouped_by_feature() {
    let groups: Vec<FaceGroup> = ROWS.iter().map(|row| row.group).collect();
    for group in FaceGroup::ALL {
        assert!(groups.contains(&group), "{group:?} has rows");
    }
    let mut named = Vec::new();
    for row in ROWS {
        let controls: Vec<&str> = match row.controls {
            RowControls::Pair(stem) => vec![
                Box::leak(format!("{stem}_Left").into_boxed_str()),
                Box::leak(format!("{stem}_Right").into_boxed_str()),
            ],
            RowControls::One(name) => vec![name],
            RowControls::Signed { less, more } => vec![less, more],
        };
        for name in controls {
            assert!(
                axcenity_core::face::face_control(name).is_some(),
                "{name} is a face control"
            );
            named.push(name);
        }
        assert!(!row.label.is_empty() && !row.about.is_empty());
    }
    for brief in [
        "Mouth_Smile_Left",
        "Brow_Raise_Left",
        "Brow_Lower_Left",
        "Eye_Squint_Left",
        "Mouth_Open",
        "Jaw_Open",
        "Jaw_Left",
        "Blink_Left",
        "Cheek_Raise_Left",
        "Nose_Sneer_Left",
    ] {
        assert!(named.contains(&brief), "{brief} has a row");
    }
}

/// The page draws the 15 built-in expressions, Neutral among them, the
/// controls under their six group names and the face preview's switch.
#[test]
fn the_expressions_page_draws_tiles_groups_and_the_face_preview() {
    let folder = test_folder("expressions-drawn");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::PickCategory(Category::Expressions));
    workspace.apply(Intent::FacePreview(true));
    let ctx = crate::theme::test_context();
    let mut texts = Vec::new();
    // A tall window, so the whole page is on the screen (only what is in
    // view is drawn).
    let tall = || {
        let mut input = super::shell_tests::input(Vec::new());
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1440.0, 3200.0),
        ));
        input
    };
    for _ in 0..2 {
        let output = crate::theme::run(&ctx, tall(), |ui| {
            workspace.ui(ui, None);
        });
        texts = super::shell_tests::drawn_texts(&output);
    }
    let has = |text: &str| {
        texts
            .iter()
            .any(|t| t.to_lowercase().contains(&text.to_lowercase()))
    };
    for name in axcenity_anim::BUILT_IN_EXPRESSIONS {
        assert!(has(name), "{name} in {texts:?}");
    }
    for group in FaceGroup::ALL {
        assert!(has(group.label()), "{} in {texts:?}", group.label());
    }
    for text in ["Face preview", "Blink", "Idle motion", "Neutral"] {
        assert!(has(text), "{text} in {texts:?}");
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// With `AXCENITY_SNAPSHOTS` naming a folder: the Expressions page drawn
/// for real at 1440 by 900, at rest, with Happy and a control moved, and
/// in the face preview (Grey and Light backdrops).
#[test]
fn expressions_pages_are_drawn() {
    use super::shell_tests::snap;
    use super::stage::Backdrop;
    let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(std::path::PathBuf::from)
    else {
        return;
    };
    let folder = test_folder("expressions-snapshots");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    for backdrop in [Backdrop::Grey, Backdrop::Light] {
        let suffix = if backdrop == Backdrop::Grey {
            ""
        } else {
            "-light"
        };
        workspace.apply(Intent::SetBackdrop(backdrop));
        workspace.apply(Intent::PickCategory(Category::Expressions));
        workspace.apply(Intent::PreviewExpression(None));
        run_for(&mut workspace, 0.5);
        snap(
            &mut workspace,
            &snapshots,
            &format!("r9-expressions{suffix}"),
            3,
        );
        workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
        workspace.apply(Intent::PreviewFaceControl {
            name: "Jaw_Open".into(),
            value: Some(0.35),
        });
        run_for(&mut workspace, 0.6);
        snap(
            &mut workspace,
            &snapshots,
            &format!("r9-expressions-happy{suffix}"),
            3,
        );
        workspace.apply(Intent::FacePreview(true));
        run_for(&mut workspace, 0.6);
        snap(
            &mut workspace,
            &snapshots,
            &format!("r9-face-preview{suffix}"),
            3,
        );
        workspace.apply(Intent::FacePreview(false));
        workspace.apply(Intent::PickCategory(Category::Body));
    }
    drop(workspace);
    let _ = std::fs::remove_dir_all(&folder);
}

/// Review fix: a new or opened character leaves the face preview, and
/// puts back the light, camera and motion it found (the old framing was
/// the old character's head).
#[test]
fn a_new_or_opened_character_leaves_the_face_preview() {
    let folder = test_folder("expressions-face-preview-new");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::Save);
    let saved = workspace.file().unwrap().to_owned();
    workspace.apply(Intent::PickCategory(Category::Expressions));
    workspace.apply(Intent::SetLighting(Lighting::Sunset));
    run_for(&mut workspace, 0.3);

    // Opened first: the opened file is saved, so the New after it asks
    // nothing.
    for start_over in [Intent::Open(saved.clone()), Intent::New(Start::default())] {
        assert!(!workspace.dirty());
        let (orbit, view, motion) = (workspace.orbit, workspace.view, workspace.motion.clone());
        workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
        workspace.apply(Intent::FacePreview(true));
        assert!(workspace.face_view.is_some());
        run_for(&mut workspace, 0.3);
        workspace.apply(start_over);
        assert!(workspace.face_view.is_none(), "the face preview is off");
        assert_eq!(workspace.lighting, Lighting::Sunset, "the light put back");
        assert_eq!(workspace.orbit, orbit, "the camera put back");
        assert_eq!(workspace.view, view);
        assert_eq!(workspace.motion, motion);
        assert!(workspace.expression.is_none());
        assert!(preview_face(&workspace).is_none());
        run_for(&mut workspace, 0.3);
    }
    let _ = std::fs::remove_dir_all(&folder);
}
