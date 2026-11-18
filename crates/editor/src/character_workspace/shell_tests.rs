//! The shell: shortcuts, camera framing, notices, the expression preview
//! and every page drawn (without a GPU, and with `AXCENITY_SNAPSHOTS` for
//! real, as pictures to look at).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;

use super::intent::{Gesture, Intent};
use super::selection::Level;
use super::*;
use crate::character_studio::test_folder;
use crate::theme;

/// Every piece of text drawn in `output`.
pub(super) fn drawn_texts(output: &egui::FullOutput) -> Vec<String> {
    fn walk(shape: &egui::Shape, found: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(text) => found.push(text.galley.text().to_owned()),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut found);
    }
    found
}

pub(super) fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1440.0, 900.0),
        )),
        events,
        ..Default::default()
    }
}

fn key(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    vec![
        egui::Event::ModifiersChanged(modifiers),
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        },
    ]
}

/// Audit D-16: Ctrl+Shift+Z redoes (it used to undo); Ctrl+Z undoes and
/// Ctrl+Y redoes.
#[test]
fn ctrl_shift_z_redoes_and_ctrl_z_undoes() {
    let folder = test_folder("shell-keys");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    for value in [0.5, 0.7] {
        workspace.apply(Intent::SetParameter {
            id: "fat".into(),
            value,
            gesture: Gesture::Commit,
        });
    }
    let ctx = theme::test_context();
    let press = |workspace: &mut CharacterWorkspace, events| {
        let _ = theme::run(&ctx, input(events), |ui| {
            workspace.ui(ui, None);
        });
    };
    press(&mut workspace, key(egui::Key::Z, egui::Modifiers::COMMAND));
    assert_eq!(workspace.parameter("fat"), 0.5, "Ctrl+Z undid");
    press(
        &mut workspace,
        key(
            egui::Key::Z,
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        ),
    );
    assert_eq!(workspace.parameter("fat"), 0.7, "Ctrl+Shift+Z redid");
    press(&mut workspace, key(egui::Key::Z, egui::Modifiers::COMMAND));
    press(&mut workspace, key(egui::Key::Y, egui::Modifiers::COMMAND));
    assert_eq!(workspace.parameter("fat"), 0.7, "Ctrl+Y redid");
    let _ = std::fs::remove_dir_all(&folder);
}

/// Audit D-04: the name field can be emptied while it's typed in. What's
/// typed stays a draft (nothing renamed, no undo step) and becomes one
/// rename when the field is left with Enter.
#[test]
fn the_name_field_can_be_emptied_and_renames_once_when_left() {
    let folder = test_folder("shell-name");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let before = workspace.description().name.clone();
    assert!(!workspace.can_undo());
    let draft_id = egui::Id::new("character studio name");
    let ctx = theme::test_context();
    let frame = |workspace: &mut CharacterWorkspace, events| -> Vec<String> {
        let output = theme::run(&ctx, input(events), |ui| {
            workspace.ui(ui, None);
        });
        drawn_texts(&output)
    };
    // The field exists after a first frame; then it's given the focus.
    frame(&mut workspace, Vec::new());
    assert!(frame(&mut workspace, Vec::new()).contains(&before));
    ctx.memory_mut(|memory| memory.request_focus(draft_id.with("field")));
    frame(&mut workspace, Vec::new());

    // Select all and delete: the field is empty and stays empty.
    let mut events = key(egui::Key::A, egui::Modifiers::COMMAND);
    events.extend(key(egui::Key::Backspace, egui::Modifiers::NONE));
    frame(&mut workspace, events);
    let drawn = frame(&mut workspace, Vec::new());
    assert!(
        ctx.memory(|memory| memory.has_focus(draft_id.with("field"))),
        "the field keeps the focus while it's typed in"
    );
    assert!(
        !drawn.contains(&before),
        "the field doesn't fill itself back in: {drawn:?}"
    );
    assert_eq!(
        ctx.data(|data| data.get_temp::<String>(draft_id))
            .as_deref(),
        Some(""),
        "the draft is empty"
    );
    assert_eq!(workspace.description().name, before, "nothing renamed yet");
    assert!(!workspace.can_undo(), "a draft is no undo step");

    // Type a name and press Enter: one rename, one undo step.
    frame(&mut workspace, vec![egui::Event::Text("Ada".into())]);
    assert_eq!(workspace.description().name, before, "still a draft");
    frame(&mut workspace, key(egui::Key::Enter, egui::Modifiers::NONE));
    frame(&mut workspace, Vec::new());
    assert_eq!(workspace.description().name, "Ada");
    assert_eq!(ctx.data(|data| data.get_temp::<String>(draft_id)), None);
    assert!(workspace.can_undo());
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.description().name, before, "one undo step");
    assert!(!workspace.can_undo(), "exactly one undo step");
    let _ = std::fs::remove_dir_all(&folder);
}

/// Audit D-06: the Face view aims at the head as it's posed, so a tall
/// character's face is framed higher than a short one's; on the default
/// character it's where the fixed view was.
#[test]
fn the_face_view_aims_at_the_head_of_a_tall_or_short_character() {
    let folder = test_folder("shell-face-view");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.step_by(1.0 / 30.0);
    workspace.focus(ViewPreset::Face);
    let default = workspace.orbit.target.y;
    let fixed = ViewPreset::Face.orbit().target.y;
    println!("face view target: {default} (the fixed view's: {fixed})");
    assert!((default - fixed).abs() < 0.05, "{default} against {fixed}");
    let mut targets = Vec::new();
    for height in [0.85, 1.15] {
        workspace.apply(Intent::SetParameter {
            id: "height".into(),
            value: height,
            gesture: Gesture::Commit,
        });
        workspace.step_by(1.0 / 30.0);
        workspace.apply(Intent::FocusView(ViewPreset::Face));
        targets.push(workspace.orbit.target.y);
    }
    assert!(
        targets[1] - targets[0] > 0.4,
        "the tall character's face is higher: {targets:?}"
    );
    assert_eq!(workspace.view, Some(ViewPreset::Face));
    let _ = std::fs::remove_dir_all(&folder);
}

/// Audit D-10 and D-12: Face and Expressions frame the face; leaving them
/// frames the whole character again, unless the camera was moved; the
/// view the camera is at is known (its chip is lit) until it moves.
#[test]
fn pages_frame_the_face_or_the_whole_character() {
    let folder = test_folder("shell-framing");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert_eq!(workspace.view, Some(ViewPreset::FullBody));
    workspace.apply(Intent::PickCategory(Category::Face));
    assert_eq!(workspace.view, Some(ViewPreset::Face));
    workspace.apply(Intent::PickCategory(Category::Expressions));
    assert_eq!(workspace.view, Some(ViewPreset::Face));
    workspace.apply(Intent::PickCategory(Category::Clothing));
    assert_eq!(workspace.view, Some(ViewPreset::FullBody));
    assert_eq!(workspace.orbit, ViewPreset::FullBody.orbit());

    // A camera the person moved stays where it is.
    workspace.apply(Intent::PickCategory(Category::Face));
    workspace.view = None;
    workspace.orbit.turn(40.0, 0.0);
    let moved = workspace.orbit;
    workspace.apply(Intent::PickCategory(Category::Body));
    assert_eq!(workspace.orbit, moved);
    assert_eq!(workspace.view, None);
    let _ = std::fs::remove_dir_all(&folder);
}

/// Audit D-13: a notice clears itself after a while; a new one starts its
/// own time.
#[test]
fn a_notice_clears_itself() {
    let folder = test_folder("shell-notice");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let start = Instant::now();
    workspace.notice = Some("Saved".to_owned());
    workspace.expire_notice(start);
    workspace.expire_notice(start + Duration::from_secs(3));
    assert_eq!(workspace.notice.as_deref(), Some("Saved"));
    workspace.notice = Some("Saved again".to_owned());
    workspace.expire_notice(start + Duration::from_secs(4));
    workspace.expire_notice(start + Duration::from_secs(10));
    assert_eq!(workspace.notice.as_deref(), Some("Saved again"));
    workspace.expire_notice(start + Duration::from_secs(13));
    assert_eq!(workspace.notice, None);
    let _ = std::fs::remove_dir_all(&folder);
}

/// Brief §10: an expression shows on the preview's face and never changes
/// the description; leaving the page takes it off.
#[test]
fn an_expression_is_previewed_on_the_face_and_never_saved() {
    let folder = test_folder("shell-expression");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let body = applier::body_of(&workspace.world, workspace.character.unwrap()).unwrap();
    let weights = |workspace: &CharacterWorkspace| -> f32 {
        workspace.poses()[&body]
            .morph_weights()
            .iter()
            .map(|w| w.abs())
            .sum()
    };
    for _ in 0..10 {
        workspace.step_by(1.0 / 30.0);
    }
    let neutral = weights(&workspace);
    let description = workspace.description().clone();
    let was_dirty = workspace.dirty();

    workspace.apply(Intent::PickCategory(Category::Expressions));
    workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
    for _ in 0..30 {
        workspace.step_by(1.0 / 30.0);
    }
    let happy = weights(&workspace);
    assert!(
        happy > neutral + 0.5,
        "the face smiles: {neutral} -> {happy}"
    );
    assert_eq!(workspace.description(), &description);
    assert_eq!(workspace.dirty(), was_dirty);
    assert!(!workspace.can_undo(), "an expression is no undo step");

    // A slider applied meanwhile keeps the expression on the face.
    workspace.apply(Intent::SetParameter {
        id: "nose_length".into(),
        value: 0.3,
        gesture: Gesture::Commit,
    });
    for _ in 0..5 {
        workspace.step_by(1.0 / 30.0);
    }
    assert!(weights(&workspace) > neutral + 0.5);

    workspace.apply(Intent::PickCategory(Category::Body));
    assert_eq!(workspace.expression, None);
    for _ in 0..30 {
        workspace.step_by(1.0 / 30.0);
    }
    assert!(
        weights(&workspace) < happy * 0.5,
        "the expression is gone: {}",
        weights(&workspace)
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// Every page draws on the realistic body at every level and shows its
/// own controls by name (the drawing test before this asserted no text).
/// With `AXCENITY_SNAPSHOTS` naming a folder, each page is also saved
/// there drawn for real at 1440 by 900.
#[test]
fn every_page_is_drawn_with_its_controls() {
    let folder = test_folder("shell-pages");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let ctx = theme::test_context();
    let draw = |workspace: &mut CharacterWorkspace| -> Vec<String> {
        let mut texts = Vec::new();
        // Twice: a window is sized on its first frame.
        for _ in 0..2 {
            let output = theme::run(&ctx, input(Vec::new()), |ui| {
                assert_eq!(workspace.ui(ui, None), None);
            });
            texts = drawn_texts(&output);
        }
        texts
    };
    let expected: [(Category, Level, &[&str]); 9] = [
        (
            Category::Body,
            Level::Quick,
            &["Body type", "Male", "Athletic", "Custom", "Family", "Quick"],
        ),
        (
            Category::Body,
            Level::Detailed,
            &[
                "Build",
                "Body fat",
                "Height",
                "Shoulder width",
                "Start from",
            ],
        ),
        (
            // Hands and feet have only advanced sliders, so Advanced adds
            // them to the navigation (the sliders themselves are below the
            // fold here; panel::tests holds the rows).
            Category::Body,
            Level::Advanced,
            &["Proportions", "Hands", "Feet"],
        ),
        (
            Category::Face,
            Level::Detailed,
            &["Eyes", "Nose", "Ears", "Symmetry"],
        ),
        // The realistic short crop (R6) is a draft, hidden until shown.
        (Category::Hair, Level::Detailed, &["Show drafts (1 hidden)"]),
        (
            Category::Clothing,
            Level::Detailed,
            &["which comes with the body type"],
        ),
        (
            Category::SkinAndMakeup,
            Level::Detailed,
            &["Skin material", "Tint", "Melanin"],
        ),
        (
            Category::Accessories,
            Level::Detailed,
            &["No accessories yet"],
        ),
        (
            Category::Expressions,
            Level::Detailed,
            &["Happy", "Sleepy", "preview only"],
        ),
    ];
    for (category, level, texts) in expected {
        workspace.apply(Intent::PickCategory(category));
        workspace.apply(Intent::PickLevel(level));
        let drawn = draw(&mut workspace);
        for text in texts {
            // Section labels are drawn in capitals.
            assert!(
                drawn
                    .iter()
                    .any(|t| t.to_lowercase().contains(&text.to_lowercase())),
                "{category:?} at {level:?}: {text} in {drawn:?}"
            );
        }
        // The navigation in the brief's order, the views and the stage
        // in the bottom bar, on every page.
        for text in [
            "Skin and Makeup",
            "Accessories",
            "Three-quarter",
            "Studio",
            "Grey",
            "Reset",
        ] {
            assert!(drawn.iter().any(|t| t.contains(text)), "{text}");
        }
    }
    // Quick shows the face's broad sliders only and says where the rest are.
    workspace.apply(Intent::PickCategory(Category::Face));
    workspace.apply(Intent::PickLevel(Level::Quick));
    let quick = draw(&mut workspace);
    // Nose length is a quick control (the manifest's `level`); the bridge isn't.
    assert!(!quick.iter().any(|t| t.contains("Nose bridge")));
    assert!(
        quick.iter().any(|t| t.contains("Detailed level")),
        "{quick:?}"
    );
    // A change hidden at the level is said, with a way to it.
    workspace.apply(Intent::PickLevel(Level::Detailed));
    workspace.apply(Intent::SetParameter {
        id: "jaw_shift".into(),
        value: 0.4,
        gesture: Gesture::Commit,
    });
    let detailed = draw(&mut workspace);
    assert!(
        detailed
            .iter()
            .any(|t| t.contains("1 more change in Advanced")),
        "{detailed:?}"
    );
    // A picked feature shows only its own rows, named in the title.
    workspace.apply(Intent::PickFeature(Some("nose".into())));
    let nose = draw(&mut workspace);
    assert!(nose.iter().any(|t| t.contains("Face › Nose")), "{nose:?}");
    assert!(nose.iter().any(|t| t.contains("Nose length")));
    assert!(!nose.iter().any(|t| t.contains("Eye size")));

    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        workspace.apply(Intent::Reset(super::edit::Scope::All));
        workspace.apply(Intent::PickFeature(None));
        // The Presets page's pictures, drawn as its thread draws them.
        paint_preset_pictures(&folder, &workspace);
        for (category, level, name) in [
            (Category::Body, Level::Quick, "body-quick"),
            (Category::Body, Level::Detailed, "body"),
            (Category::Body, Level::Advanced, "body-advanced"),
            (Category::Face, Level::Detailed, "face"),
            (Category::Hair, Level::Detailed, "hair"),
            (Category::Clothing, Level::Detailed, "clothing"),
            (Category::SkinAndMakeup, Level::Detailed, "skin"),
            (Category::Accessories, Level::Detailed, "accessories"),
            (Category::Expressions, Level::Detailed, "expressions"),
            (Category::Presets, Level::Detailed, "presets"),
        ] {
            workspace.apply(Intent::PickCategory(category));
            workspace.apply(Intent::PickLevel(level));
            if category == Category::Expressions {
                workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
            }
            if category == Category::Face {
                workspace.apply(Intent::PickFeature(Some("nose".into())));
            }
            // A few pictures are read from disk each frame.
            let frames = if category == Category::Presets { 10 } else { 3 };
            snap(&mut workspace, &snapshots, name, frames);
            workspace.apply(Intent::PickFeature(None));
        }
        // Comparison: A held, a heavier body, side by side and in place.
        workspace.apply(Intent::PickCategory(Category::Body));
        workspace.apply(Intent::Capture);
        workspace.apply(Intent::SetParameter {
            id: "fat".into(),
            value: 0.9,
            gesture: Gesture::Commit,
        });
        workspace.apply(Intent::SideBySide(true));
        snap(&mut workspace, &snapshots, "compare-side-by-side", 3);
        workspace.apply(Intent::SideBySide(false));
        workspace.apply(Intent::ShowHeld(Some(0)));
        snap(&mut workspace, &snapshots, "compare-showing-a", 3);
        // The same on the Light backdrop: the strip and labels must still
        // read (they sit on cards of their own).
        let backdrop = workspace.backdrop;
        workspace.apply(Intent::SetBackdrop(stage::Backdrop::Light));
        snap(&mut workspace, &snapshots, "compare-showing-a-light", 3);
        workspace.apply(Intent::ShowHeld(None));
        workspace.apply(Intent::SideBySide(true));
        snap(&mut workspace, &snapshots, "compare-side-by-side-light", 3);
        workspace.apply(Intent::SideBySide(false));
        workspace.apply(Intent::SetBackdrop(backdrop));
        // The stylized family's Body and Face pages.
        workspace.apply(Intent::New(Start::Family(axcenity_avatar::V1_BASE.into())));
        workspace.answer(Some(false));
        snap(&mut workspace, &snapshots, "body-stylized", 3);
        workspace.apply(Intent::PickCategory(Category::Face));
        snap(&mut workspace, &snapshots, "face-stylized", 3);
        // Unsaved work left by a crash is offered on the next open.
        workspace.apply(Intent::SetParameter {
            id: "nose_width".into(),
            value: 0.5,
            gesture: Gesture::Commit,
        });
        workspace.autosave_step(Instant::now());
        std::mem::forget(workspace);
        let mut reopened = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
        snap(&mut reopened, &snapshots, "recover", 4);
        reopened.apply(Intent::Recover(false));
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// Saves the workspace drawn for real at 1440 by 900 after `frames` frames.
pub(super) fn snap(
    workspace: &mut CharacterWorkspace,
    snapshots: &std::path::Path,
    name: &str,
    frames: usize,
) {
    // Each snapshot makes its own GPU device; the editor has one.
    workspace.canvas = Canvas::default();
    workspace.other_canvas = Canvas::default();
    crate::snapshot::snapshot_rendered(
        &snapshots.join(format!("character-studio-page-{name}.png")),
        egui::vec2(1440.0, 900.0),
        vec![Vec::new(); frames],
        |ui, render_state| {
            workspace.ui(ui, Some(render_state));
        },
    );
}

/// Draws the pictures of every preset of the workspace's base model into
/// its cache, as the Presets page's thread would (none without a GPU).
pub(super) fn paint_preset_pictures(folder: &std::path::Path, workspace: &CharacterWorkspace) {
    let instance = axcenity_render::adapters::instance();
    let Some((device, queue)) =
        pollster::block_on(axcenity_render::adapters::adapter(&instance, None))
            .ok()
            .and_then(|adapter| {
                pollster::block_on(adapter.request_device(&Default::default())).ok()
            })
    else {
        return;
    };
    let mut painter = thumbnails::Painter::new(folder, &device, &queue).unwrap();
    let library = workspace.library();
    for listed in presets::all(&library, workspace.own_presets()) {
        if listed.preset.base != workspace.description().base {
            continue;
        }
        let subject = thumbnails::preset_subject(&listed, &library);
        let out = thumbnails::preset_picture(folder, listed.id, &subject);
        painter.paint_to(&thumbnails::Job { subject, out }).unwrap();
    }
}
