//! The shell: shortcuts, camera framing, notices, the expression preview,
//! the panel's width (its edge dragged, a small window) and its "Search
//! controls" field, and every page drawn (without a GPU, and with
//! `AXCENITY_SNAPSHOTS` for real, as pictures to look at).

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
                "Search controls",
                "Detailed",
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
        // The realistic short crop (R6) is a draft: with nothing else on
        // the page it is listed without asking, marked.
        (
            Category::Hair,
            Level::Detailed,
            &["Search hairstyles", "Filters", "Draft"],
        ),
        (
            Category::Clothing,
            Level::Detailed,
            &[
                "Your library",
                "Search clothes",
                "Filters",
                "which comes with the body type",
            ],
        ),
        (
            Category::SkinAndMakeup,
            Level::Detailed,
            &["Search controls", "Skin material", "Tint", "Melanin"],
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
        // The panel at its narrowest and its widest, and a search.
        workspace.apply(Intent::PickCategory(Category::Face));
        workspace.apply(Intent::PickFeature(Some("eyes".into())));
        workspace.apply(Intent::SetParameter {
            id: "eye_size_left".into(),
            value: 0.4,
            gesture: Gesture::Commit,
        });
        for (width, name) in [
            (shell::PANEL_MIN, "face-panel-narrow"),
            (shell::PANEL_MAX, "face-panel-wide"),
        ] {
            workspace.apply(Intent::SetPanelWidth {
                width,
                gesture: Gesture::Drag,
            });
            snap(&mut workspace, &snapshots, name, 3);
        }
        workspace.apply(Intent::PickCategory(Category::Clothing));
        snap(&mut workspace, &snapshots, "clothing-panel-wide", 3);
        workspace.apply(Intent::SetPanelWidth {
            width: shell::PANEL_WIDTH,
            gesture: Gesture::Drag,
        });
        workspace.apply(Intent::PickCategory(Category::Face));
        workspace.apply(Intent::PickLevel(Level::Quick));
        workspace.apply(Intent::SearchControls("jaw".into()));
        snap(&mut workspace, &snapshots, "face-search", 3);
        workspace.apply(Intent::SearchControls(String::new()));
        workspace.apply(Intent::PickLevel(Level::Detailed));
        workspace.apply(Intent::Reset(super::edit::Scope::All));
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

/// The panel is as wide as the person made it, 400 points until they drag
/// its inner edge, between 320 and 640; a window too narrow for that
/// narrows the panel first (down to 270), so the viewport keeps 420. The
/// width is editor state: no undo step, kept beside the favourites for
/// the next session.
#[test]
fn the_panel_is_as_wide_as_its_edge_is_dragged() {
    use super::shell::{PANEL_FLOOR, PANEL_MAX, PANEL_MIN, PANEL_WIDTH, VIEW_MIN, panel_width};
    // The rule, by itself: a window 1440 wide has a navigation of 200.
    assert_eq!(panel_width(PANEL_WIDTH, 1440.0, 200.0), 400.0);
    assert_eq!(panel_width(900.0, 1440.0, 200.0), PANEL_MAX);
    assert_eq!(panel_width(100.0, 1440.0, 200.0), PANEL_MIN);
    // A small window: the panel gives way before the viewport does...
    assert_eq!(
        panel_width(PANEL_WIDTH, 900.0, 150.0),
        900.0 - 150.0 - VIEW_MIN
    );
    assert_eq!(
        panel_width(PANEL_MAX, 1100.0, 165.0),
        1100.0 - 165.0 - VIEW_MIN
    );
    // ...down to its floor; smaller still, the viewport narrows.
    assert_eq!(panel_width(PANEL_WIDTH, 700.0, 150.0), PANEL_FLOOR);

    let folder = test_folder("shell-panel-width");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert_eq!(workspace.panel_width, PANEL_WIDTH);
    let before = workspace.description().clone();
    let ctx = theme::test_context();
    let frame = |workspace: &mut CharacterWorkspace, events| {
        let _ = theme::run(&ctx, input(events), |ui| {
            workspace.ui(ui, None);
        });
    };
    // Where the panel's title is drawn says where the panel starts.
    let title_left = |workspace: &mut CharacterWorkspace| -> f32 {
        let output = theme::run(&ctx, input(Vec::new()), |ui| {
            workspace.ui(ui, None);
        });
        fn walk(shape: &egui::Shape, left: &mut Option<f32>) {
            match shape {
                egui::Shape::Text(text)
                    if text.galley.text() == "Body" && text.galley.size().y > 18.0 =>
                {
                    *left = Some(text.pos.x);
                }
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, left)),
                _ => {}
            }
        }
        let mut left = None;
        for clipped in &output.shapes {
            walk(&clipped.shape, &mut left);
        }
        left.expect("the panel's title is drawn")
    };
    frame(&mut workspace, Vec::new());
    assert_eq!(title_left(&mut workspace), 1440.0 - 400.0 + 12.0);

    // The inner edge, dragged 120 points toward the viewport.
    let edge = egui::pos2(1440.0 - 400.0, 450.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut workspace, vec![egui::Event::PointerMoved(edge)]);
    frame(&mut workspace, vec![button(edge, true)]);
    for step in 1..=4 {
        let at = edge - egui::vec2(30.0 * step as f32, 0.0);
        frame(&mut workspace, vec![egui::Event::PointerMoved(at)]);
    }
    assert_eq!(workspace.panel_width, 520.0, "it follows the pointer");
    assert_eq!(
        presets::read_panel_width(&folder),
        None,
        "not kept until the edge is let go"
    );
    frame(
        &mut workspace,
        vec![button(edge - egui::vec2(120.0, 0.0), false)],
    );
    frame(&mut workspace, Vec::new());
    assert_eq!(workspace.panel_width, 520.0);
    assert_eq!(title_left(&mut workspace), 1440.0 - 520.0 + 12.0);
    // Editor state: the character and its undo are untouched.
    assert_eq!(*workspace.description(), before);
    assert!(!workspace.can_undo());
    // Asked for more than it may have, it is held to its limits.
    workspace.apply(Intent::SetPanelWidth {
        width: 5000.0,
        gesture: Gesture::Drag,
    });
    assert_eq!(workspace.panel_width, PANEL_MAX);
    workspace.apply(Intent::SetPanelWidth {
        width: 10.0,
        gesture: Gesture::Drag,
    });
    assert_eq!(workspace.panel_width, PANEL_MIN);
    // Kept for the next session, beside the favourites (which stay).
    workspace.apply(Intent::ToggleItemFavourite("x/item".to_owned()));
    assert_eq!(presets::read_panel_width(&folder), Some(520.0));
    workspace.apply(Intent::SetPanelWidth {
        width: 480.0,
        gesture: Gesture::Commit,
    });
    drop(workspace);
    let reopened = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert_eq!(reopened.panel_width, 480.0);
    assert!(reopened.item_favourites.contains("x/item"));
    let _ = std::fs::remove_dir_all(&folder);
}

/// The panel's header is one line: the title, Symmetry and the detail
/// level, a menu that says what each level shows (the caption that was
/// under the control). The page's description is the title's tooltip.
#[test]
fn the_header_is_one_line_with_the_level_in_a_menu() {
    fn placed(output: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
        fn walk(shape: &egui::Shape, found: &mut Vec<(String, egui::Rect)>) {
            match shape {
                egui::Shape::Text(text) => {
                    found.push((text.galley.text().to_owned(), text.visual_bounding_rect()))
                }
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
    let folder = test_folder("shell-header");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::PickCategory(Category::Face));
    let ctx = theme::test_context();
    let frame = |workspace: &mut CharacterWorkspace, events| {
        placed(&theme::run(&ctx, input(events), |ui| {
            workspace.ui(ui, None);
        }))
    };
    let click = |workspace: &mut CharacterWorkspace, at: egui::Pos2| {
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(workspace, vec![egui::Event::PointerMoved(at)]);
        frame(workspace, vec![button(true)]);
        frame(workspace, vec![button(false)]);
        for _ in 0..3 {
            frame(workspace, Vec::new());
        }
    };
    frame(&mut workspace, Vec::new());
    let drawn = frame(&mut workspace, Vec::new());
    let find = |drawn: &[(String, egui::Rect)], wanted: &dyn Fn(&str) -> bool| {
        drawn
            .iter()
            .find(|(text, _)| wanted(text))
            .map(|(_, rect)| *rect)
    };
    let title = find(&drawn, &|t| t == "Face").filter(|rect| rect.left() > 1000.0);
    let title = title
        .or_else(|| {
            drawn
                .iter()
                .filter(|(text, rect)| text == "Face" && rect.left() > 1000.0)
                .map(|(_, rect)| *rect)
                .next()
        })
        .expect("the panel's title");
    let symmetry = find(&drawn, &|t| t == "Symmetry").expect("Symmetry");
    let level = find(&drawn, &|t| t.starts_with("Detailed")).expect("the level");
    for (name, rect) in [("Symmetry", symmetry), ("the level", level)] {
        assert!(
            (rect.center().y - title.center().y).abs() < 6.0,
            "{name} is on the title's line: {rect:?} beside {title:?}"
        );
        assert!(rect.left() > title.right(), "{name} is after the title");
    }
    assert!(symmetry.right() < level.left(), "side by side");
    // No caption under the level, no sentence under the title.
    let said = |drawn: &[(String, egui::Rect)], text: &str| drawn.iter().any(|(t, _)| t == text);
    assert!(!said(&drawn, Level::Detailed.about()));
    assert!(!said(&drawn, panel::about(Category::Face)));
    // The menu lists the levels with what each shows; picking one sets it.
    click(&mut workspace, level.center());
    let drawn = frame(&mut workspace, Vec::new());
    for option in Level::ALL {
        assert!(said(&drawn, option.label()), "{option:?}: {drawn:?}");
        assert!(said(&drawn, option.about()), "{option:?}: {drawn:?}");
    }
    let advanced = find(&drawn, &|t| t == "Advanced").unwrap();
    click(&mut workspace, advanced.center());
    assert_eq!(workspace.selection.level, Level::Advanced);
    let drawn = frame(&mut workspace, Vec::new());
    assert!(!said(&drawn, Level::Quick.about()), "the menu closed");
    assert!(find(&drawn, &|t| t.starts_with("Advanced")).is_some());
    // Symmetry is a click away, on the same line.
    assert!(workspace.symmetry);
    let symmetry = find(&drawn, &|t| t == "Symmetry").expect("Symmetry");
    click(&mut workspace, symmetry.center());
    assert!(!workspace.symmetry);
    // A long title in the panel at its narrowest: Symmetry keeps its
    // place as an icon, and nothing runs into anything else.
    workspace.apply(Intent::PickFeature(Some("forehead".into())));
    workspace.apply(Intent::SetPanelWidth {
        width: shell::PANEL_MIN,
        gesture: Gesture::Drag,
    });
    frame(&mut workspace, Vec::new());
    let drawn = frame(&mut workspace, Vec::new());
    let left = 1440.0 - shell::PANEL_MIN;
    let mut line: Vec<_> = drawn
        .iter()
        .filter(|(_, rect)| rect.left() > left && rect.center().y < 48.0 + 12.0 + 28.0)
        .filter(|(_, rect)| rect.top() > 48.0)
        .collect();
    line.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    assert!(line[0].0.starts_with("Face › Forehead"), "{line:?}");
    assert!(!said(&drawn, "Symmetry"), "an icon in the narrow panel");
    for pair in line.windows(2) {
        assert!(
            pair[0].1.right() <= pair[1].1.left() + 0.5,
            "{:?} runs into {:?}",
            pair[0],
            pair[1]
        );
    }
    assert!(line.last().unwrap().1.right() <= 1440.0 - 12.0 + 0.5);
    let _ = std::fs::remove_dir_all(&folder);
}

/// The bar under the viewport fits it: every choice a chip where there is
/// room for them all; under a narrower viewport the backdrop is a menu
/// (saying what is on), then the lighting, and under the narrowest the
/// views are one too. Nothing in it is ever drawn over anything else.
#[test]
fn the_bottom_bar_folds_its_chips_into_menus_where_it_is_narrow() {
    let folder = test_folder("shell-bottom-bar");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let ctx = theme::test_context();
    // The texts of the bar (the 40 points over the status line), left to
    // right, with where each is.
    let bar = |workspace: &mut CharacterWorkspace, window: f32| -> Vec<(String, egui::Rect)> {
        fn walk(shape: &egui::Shape, found: &mut Vec<(String, egui::Rect)>) {
            match shape {
                egui::Shape::Text(text) => {
                    found.push((text.galley.text().to_owned(), text.visual_bounding_rect()))
                }
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, found)),
                _ => {}
            }
        }
        let mut found = Vec::new();
        for _ in 0..2 {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(window, 900.0),
                )),
                ..Default::default()
            };
            let output = theme::run(&ctx, raw, |ui| {
                workspace.ui(ui, None);
            });
            found.clear();
            for clipped in &output.shapes {
                walk(&clipped.shape, &mut found);
            }
        }
        let (top, bottom) = (900.0 - 26.0 - 40.0, 900.0 - 26.0);
        found.retain(|(_, rect)| rect.center().y > top && rect.center().y < bottom);
        found.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
        found
    };
    let has = |texts: &[(String, egui::Rect)], text: &str| texts.iter().any(|(t, _)| t == text);
    let says = |texts: &[(String, egui::Rect)], text: &str| {
        texts.iter().any(|(t, _)| t.contains(text) && t != text)
    };
    let mut seen = Vec::new();
    for (window, panel) in [
        (1920.0_f32, shell::PANEL_WIDTH),
        (1440.0, shell::PANEL_WIDTH),
        (1440.0, shell::PANEL_MAX),
        (1000.0, shell::PANEL_WIDTH),
        (760.0, shell::PANEL_WIDTH),
    ] {
        workspace.apply(Intent::SetPanelWidth {
            width: panel,
            gesture: Gesture::Drag,
        });
        // The bar is under the viewport: between the navigation and the
        // panel.
        let left = (window * 0.15).clamp(150.0, 200.0);
        let right = window - shell::panel_width(panel, window, left);
        let mut texts = bar(&mut workspace, window);
        texts.retain(|(_, rect)| rect.center().x > left && rect.center().x < right);
        // Nothing over anything else, and all of it under the viewport.
        for pair in texts.windows(2) {
            assert!(
                pair[0].1.right() <= pair[1].1.left() + 0.5,
                "{window} wide, panel {panel}: {:?} is over {:?}",
                pair[0],
                pair[1]
            );
        }
        for (text, rect) in &texts {
            assert!(
                rect.left() >= left && rect.right() <= right,
                "{window} wide, panel {panel}: {text:?} at {rect:?} is outside {left}..{right}"
            );
        }
        // The view, the motion, the lighting and the backdrop are always
        // said, as chips or on their menus.
        for text in ["Full body", "Idle", "Studio", "Grey"] {
            assert!(
                has(&texts, text) || says(&texts, text),
                "{window} wide, panel {panel}: {text} in {texts:?}"
            );
        }
        // (Which are rows of chips: the views, the lighting, the backdrop.)
        seen.push((
            has(&texts, "Three-quarter"),
            has(&texts, "Daylight"),
            has(&texts, "Dark"),
        ));
    }
    println!("views, lighting and backdrop as chips, widest first: {seen:?}");
    // Chips where they fit: all of them under a wide viewport, none under
    // the narrowest; in between the backdrop folds first, then the
    // lighting, then the views, and a narrower viewport never has a row of
    // chips that a wider one folded.
    assert_eq!(seen[0], (true, true, true));
    assert_eq!(seen[4], (false, false, false));
    assert!(
        seen[1].0,
        "the views are chips at the workspace's usual size"
    );
    for (views, lighting, backdrop) in &seen {
        assert!(*views || !*lighting, "{seen:?}");
        assert!(*lighting || !*backdrop, "{seen:?}");
    }
    for pair in seen.windows(2) {
        let chips = |shape: &(bool, bool, bool)| {
            usize::from(shape.0) + usize::from(shape.1) + usize::from(shape.2)
        };
        assert!(chips(&pair[1]) <= chips(&pair[0]), "{seen:?}");
    }
    // A menu's choice is the chip's: the same intent.
    workspace.apply(Intent::SetLighting(stage::Lighting::Sunset));
    let texts = bar(&mut workspace, 760.0);
    assert!(says(&texts, "Sunset"), "{texts:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// "Search controls" (Body, Face, Skin and Makeup) lists the page's
/// controls that match, whatever feature is picked and at every level,
/// and says the level of one above the level shown. It is editor state:
/// nothing is saved or undone, and another page starts without it.
#[test]
fn search_controls_finds_a_slider_in_every_feature_at_every_level() {
    let folder = test_folder("shell-search");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let before = workspace.description().clone();
    let ctx = theme::test_context();
    let draw = |workspace: &mut CharacterWorkspace| -> Vec<String> {
        let mut texts = Vec::new();
        for _ in 0..2 {
            let output = theme::run(&ctx, input(Vec::new()), |ui| {
                assert_eq!(workspace.ui(ui, None), None);
            });
            texts = drawn_texts(&output);
        }
        texts
    };
    // A row's label and what is drawn after it: its level, where it says
    // one, else its value.
    let row = |drawn: &[String], label: &str| -> Option<String> {
        let at = drawn.iter().position(|t| t == label)?;
        drawn.get(at + 1).cloned()
    };
    // The Face page at Quick, the nose picked: only the nose's broad rows.
    workspace.apply(Intent::PickCategory(Category::Face));
    workspace.apply(Intent::PickLevel(Level::Quick));
    workspace.apply(Intent::PickFeature(Some("nose".into())));
    let drawn = draw(&mut workspace);
    assert!(drawn.iter().any(|t| t == "Search controls"), "{drawn:?}");
    assert!(row(&drawn, "Nose length").is_some());
    assert!(row(&drawn, "Jaw to one side").is_none(), "{drawn:?}");
    assert!(row(&drawn, "Eye size").is_none(), "{drawn:?}");

    // "jaw" finds the jaw's controls: another feature's, at every level.
    // A row above the level shown says its level after its label; one of
    // the level shown (the jaw's width is a Quick control) says nothing.
    workspace.apply(Intent::SearchControls("jaw".into()));
    let drawn = draw(&mut workspace);
    assert_eq!(
        row(&drawn, "Jaw to one side").as_deref(),
        Some("Advanced"),
        "{drawn:?}"
    );
    assert_eq!(
        row(&drawn, "Jaw angle").as_deref(),
        Some("Detailed"),
        "{drawn:?}"
    );
    assert_eq!(
        row(&drawn, "Jaw width").as_deref(),
        Some("0.00"),
        "{drawn:?}"
    );
    assert!(row(&drawn, "Nose length").is_none(), "{drawn:?}");
    assert!(row(&drawn, "Eye size").is_none(), "{drawn:?}");
    // The picked feature and the level are as they were: only the list
    // is the search's.
    assert_eq!(workspace.selection.feature.as_deref(), Some("nose"));
    assert_eq!(workspace.selection.level, Level::Quick);
    // What it finds is a row like any other: it sets the value.
    workspace.apply(Intent::SetParameter {
        id: "jaw_shift".into(),
        value: 0.25,
        gesture: Gesture::Commit,
    });
    assert_eq!(workspace.parameter("jaw_shift"), 0.25);
    workspace.apply(Intent::Undo);

    // Every word typed must match; the ID and what a control does (its
    // `about`) are searched too; capitals don't matter.
    workspace.apply(Intent::SearchControls("NOSE length".into()));
    let drawn = draw(&mut workspace);
    assert!(row(&drawn, "Nose length").is_some(), "{drawn:?}");
    assert!(row(&drawn, "Nose width").is_none(), "{drawn:?}");
    workspace.apply(Intent::SearchControls("eye_size".into()));
    let drawn = draw(&mut workspace);
    assert!(row(&drawn, "Eye size").is_some(), "the ID: {drawn:?}");
    // Nothing matches: the page says so.
    workspace.apply(Intent::SearchControls("zzz".into()));
    let drawn = draw(&mut workspace);
    assert!(
        drawn.iter().any(|t| t == "No control matches \"zzz\"."),
        "{drawn:?}"
    );
    // Cleared, the page is as it was.
    workspace.apply(Intent::SearchControls(String::new()));
    let drawn = draw(&mut workspace);
    assert!(row(&drawn, "Nose length").is_some());
    assert!(row(&drawn, "Jaw to one side").is_none(), "{drawn:?}");

    // Editor state: never an undo step, never the character.
    workspace.apply(Intent::SearchControls("jaw".into()));
    assert_eq!(*workspace.description(), before);
    assert!(!workspace.can_undo());
    // It belonged to the page it was typed on.
    workspace.apply(Intent::PickCategory(Category::Body));
    assert_eq!(workspace.control_search, "");
    // The Body page: the starting points give way to what was searched.
    workspace.apply(Intent::PickLevel(Level::Detailed));
    let drawn = draw(&mut workspace);
    assert!(drawn.iter().any(|t| t == "START FROM"), "{drawn:?}");
    workspace.apply(Intent::SearchControls("waist".into()));
    let drawn = draw(&mut workspace);
    assert!(row(&drawn, "Waist width").is_some(), "{drawn:?}");
    assert!(!drawn.iter().any(|t| t == "START FROM"), "{drawn:?}");
    assert!(row(&drawn, "Height").is_none(), "{drawn:?}");
    // The Skin and Makeup page's values and layers are searched too.
    workspace.apply(Intent::PickCategory(Category::SkinAndMakeup));
    workspace.apply(Intent::SearchControls("melanin".into()));
    let drawn = draw(&mut workspace);
    assert!(row(&drawn, "Melanin").is_some(), "{drawn:?}");
    assert!(row(&drawn, "Tint").is_none(), "{drawn:?}");
    // The browsers have a search of their own.
    workspace.apply(Intent::PickCategory(Category::Clothing));
    let drawn = draw(&mut workspace);
    assert!(!drawn.iter().any(|t| t == "Search controls"), "{drawn:?}");
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
