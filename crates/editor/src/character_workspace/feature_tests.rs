//! R1 step 2 part 2 (RA1): both families, presets as data (by ID, "Start
//! from" chips, "Custom", the Presets page, Save as preset, favourites),
//! Randomize with scopes and locks, comparison (held versions, before and
//! after, side by side, keep and discard) and autosave with recovery. Each
//! is driven through [`Intent`]s, as the panels drive it.

use std::time::{Duration, Instant};

use axcenity_avatar::{PresetKind, matches_preset};

use super::autosave::{self, AUTOSAVE_EVERY, RECOVERY_FILE};
use super::intent::{Gesture, Intent};
use super::random::RandomScope;
use super::selection::Level;
use super::shell_tests::{drawn_texts, input};
use super::*;
use crate::character_studio::test_folder;
use crate::theme;

const BODY_ATHLETIC: &str = "axcenity/realistic-body-athletic-male";
const FACE_ROUND: &str = "axcenity/realistic-face-round";

fn set(id: &str, value: f32) -> Intent {
    Intent::SetParameter {
        id: id.to_owned(),
        value,
        gesture: Gesture::Commit,
    }
}

/// The texts of the workspace drawn (twice: a window is sized on its
/// first frame).
fn draw(workspace: &mut CharacterWorkspace) -> Vec<String> {
    draw_sized(workspace, input(Vec::new()))
}

/// [`draw`] in a window tall enough for the whole Presets page.
fn draw_tall(workspace: &mut CharacterWorkspace) -> Vec<String> {
    let mut tall = input(Vec::new());
    tall.screen_rect = Some(eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::vec2(1440.0, 2600.0),
    ));
    draw_sized(workspace, tall)
}

fn draw_sized(workspace: &mut CharacterWorkspace, raw: eframe::egui::RawInput) -> Vec<String> {
    let ctx = theme::test_context();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let output = theme::run(&ctx, raw.clone(), |ui| {
            workspace.ui(ui, None);
        });
        texts = drawn_texts(&output);
    }
    texts
}

fn has(texts: &[String], text: &str) -> bool {
    texts
        .iter()
        .any(|t| t.to_lowercase().contains(&text.to_lowercase()))
}

/// How many steps undo goes back.
fn undo_depth(workspace: &CharacterWorkspace) -> usize {
    let mut history = workspace.history.clone();
    let mut current = workspace.description().clone();
    let mut depth = 0;
    while let Some(before) = history.undo(current) {
        current = before;
        depth += 1;
    }
    depth
}

// ---- Both families (plan section 0, decision 7) ----

/// A stylized character is made from New, edited from its own manifest's
/// controls (its groups stand in for features), saved and reopened the
/// same, as a realistic one is.
#[test]
fn a_stylized_character_opens_edits_saves_and_reopens() {
    let folder = test_folder("feature-stylized");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert_eq!(
        workspace.description().base,
        DEFAULT_BASE,
        "realistic first"
    );
    workspace.apply(Intent::New(Start::Family(axcenity_avatar::V1_BASE.into())));
    // A new character isn't saved: New asks first (Don't Save).
    workspace.answer(Some(false));
    assert_eq!(workspace.description().base, axcenity_avatar::V1_BASE);
    assert!(!workspace.can_undo());
    assert!(workspace.worn().len() > 1, "it wears its default clothes");

    // Its controls: the manifest's groups, no features, listed in the
    // navigation and edited as sliders.
    let library = workspace.library();
    let base = library.base(axcenity_avatar::V1_BASE).unwrap();
    let face: Vec<String> = nav::features(
        base,
        workspace.description(),
        Category::Face,
        Level::Detailed,
    )
    .into_iter()
    .map(|feature| feature.id)
    .collect();
    assert!(face.contains(&"nose".to_owned()), "{face:?}");
    workspace.apply(Intent::PickCategory(Category::Face));
    workspace.apply(Intent::PickFeature(Some("nose".into())));
    let drawn = draw(&mut workspace);
    assert!(has(&drawn, "Nose width"), "{drawn:?}");
    workspace.apply(set("nose_width", 0.6));
    workspace.apply(Intent::Randomize(RandomScope::Feature));
    let nose = workspace.parameter("nose_length");
    assert_ne!(nose, 0.0, "the stylized nose randomizes by its group");
    assert_eq!(workspace.parameter("eyes_size"), 0.0, "only the nose");

    // The Body page shows the family.
    workspace.apply(Intent::PickCategory(Category::Body));
    let drawn = draw(&mut workspace);
    for text in ["Family", "Realistic", "Stylized", "Height"] {
        assert!(has(&drawn, text), "{text} in {drawn:?}");
    }

    workspace.apply(Intent::Rename("Toon".into()));
    workspace.apply(Intent::Save);
    assert!(!workspace.dirty(), "{:?}", workspace.notice);
    let path = workspace.file().unwrap().to_owned();
    let reopened = CharacterWorkspace::open_file(&folder, &path).unwrap();
    assert_eq!(reopened.description(), workspace.description());
    assert_eq!(reopened.description().base, axcenity_avatar::V1_BASE);
    assert_eq!(reopened.notice, None, "no fallback to another body");
    assert!(!reopened.dirty());
    drop(reopened);

    // The Body page's family chips move a character across, one step.
    workspace.apply(Intent::SetBase(DEFAULT_BASE.into()));
    assert_eq!(workspace.description().base, DEFAULT_BASE);
    assert_eq!(workspace.description().name, "Toon");
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.description().base, axcenity_avatar::V1_BASE);
    let _ = std::fs::remove_dir_all(&folder);
}

// ---- Presets from data ----

/// A body preset and a face preset each apply what their kind owns as one
/// undo step; the chips show which is applied and "Custom" otherwise.
#[test]
fn body_and_face_presets_apply_as_one_step_each_and_custom_shows_otherwise() {
    let folder = test_folder("feature-presets");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let library = workspace.library();
    let athletic = library.preset(BODY_ATHLETIC).unwrap().clone();
    let round = library.preset(FACE_ROUND).unwrap().clone();

    workspace.apply(set("nose_length", 0.3));
    workspace.apply(Intent::ApplyPreset(BODY_ATHLETIC.into()));
    for (id, value) in &athletic.parameters {
        assert_eq!(workspace.parameter(id), *value, "{id}");
    }
    assert_eq!(workspace.parameter("nose_length"), 0.3, "the face is left");
    assert_eq!(workspace.description().name, "New Character");
    assert!(matches_preset(workspace.description(), &athletic, &library));

    workspace.apply(Intent::ApplyPreset(FACE_ROUND.into()));
    for (id, value) in &round.parameters {
        assert_eq!(workspace.parameter(id), *value, "{id}");
    }
    assert_eq!(
        workspace.parameter("nose_length"),
        round.parameters.get("nose_length").copied().unwrap_or(0.0),
        "a face preset is a whole face"
    );
    assert!(matches_preset(workspace.description(), &athletic, &library));
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.parameter("nose_length"), 0.3, "one step back");
    assert!(matches_preset(workspace.description(), &athletic, &library));
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.parameter("muscle"), 0.0);
    workspace.apply(Intent::Redo);

    // The Body page's chips: this frame's presets only, Athletic lit.
    workspace.apply(Intent::PickCategory(Category::Body));
    let drawn = draw(&mut workspace);
    for text in ["Start from", "Athletic", "Heavyset", "Custom"] {
        assert!(has(&drawn, text), "{text} in {drawn:?}");
    }
    let base = library.base(DEFAULT_BASE).unwrap();
    let body = workspace.presets_of(&library, PresetKind::Body);
    let suits: Vec<bool> = body
        .iter()
        .map(|listed| presets::suits(listed, workspace.description(), base))
        .collect();
    assert_eq!(suits.iter().filter(|s| **s).count(), body.len() / 2);
    assert!(
        body.iter()
            .filter(|listed| presets::suits(listed, workspace.description(), base))
            .all(|listed| listed.info.tags.contains(&"male".to_owned()))
    );
    // A slider moved: no body preset is what the character is.
    let body: Vec<Description> = body.iter().map(|listed| listed.preset.clone()).collect();
    workspace.apply(set("muscle", 0.55));
    assert!(
        !body
            .iter()
            .any(|preset| matches_preset(workspace.description(), preset, &library))
    );

    // The Face page has its chips; the Presets page groups by kind.
    workspace.apply(Intent::PickCategory(Category::Face));
    let drawn = draw(&mut workspace);
    assert!(has(&drawn, "Heart-shaped"), "{drawn:?}");
    workspace.apply(Intent::PickCategory(Category::Presets));
    let drawn = draw_tall(&mut workspace);
    for text in [
        "Characters",
        "Bodies",
        "Faces",
        "Save as preset",
        "Heavyset female",
    ] {
        assert!(has(&drawn, text), "{text} in {drawn:?}");
    }
    // A preset of another family isn't listed for this one.
    assert!(
        workspace
            .presets_of(&library, PresetKind::Character)
            .iter()
            .all(|listed| listed.preset.base == DEFAULT_BASE)
    );
    // An unknown ID changes nothing and says so.
    let before = workspace.description().clone();
    workspace.apply(Intent::ApplyPreset("axcenity/no-such-preset".into()));
    assert_eq!(workspace.description(), &before);
    assert!(workspace.notice.is_some());
    let _ = std::fs::remove_dir_all(&folder);
}

/// A feature's own presets (a nose, a face shape) are "Shape" chips on
/// that feature's page: one sets that feature's parameters and nothing
/// else of the face, as one undo step, and is lit until a slider of the
/// feature moves.
#[test]
fn a_feature_s_shapes_are_chips_that_set_that_feature_alone() {
    const AQUILINE: &str = "axcenity/realistic-nose-aquiline";
    let folder = test_folder("feature-shapes");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let library = workspace.library();
    let aquiline = library.preset(AQUILINE).unwrap().clone();
    assert!(!aquiline.parameters.is_empty());

    workspace.apply(set("jaw_definition", 0.5));
    workspace.apply(set("nose_length", 0.3));
    workspace.apply(Intent::ApplyPreset(AQUILINE.into()));
    for (id, value) in &aquiline.parameters {
        assert_eq!(workspace.parameter(id), *value, "{id}");
    }
    assert_eq!(
        workspace.parameter("nose_length"),
        aquiline
            .parameters
            .get("nose_length")
            .copied()
            .unwrap_or(0.0),
        "a nose preset is a whole nose"
    );
    assert_eq!(
        workspace.parameter("jaw_definition"),
        0.5,
        "the jaw is left"
    );
    assert!(matches_preset(workspace.description(), &aquiline, &library));
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.parameter("nose_length"), 0.3, "one step back");
    workspace.apply(Intent::Redo);

    // The Nose page offers the nose's shapes, the applied one among them;
    // the whole Face page and another feature's page don't.
    workspace.apply(Intent::PickCategory(Category::Face));
    let drawn = draw(&mut workspace);
    assert!(!has(&drawn, "Aquiline"), "{drawn:?}");
    workspace.apply(Intent::PickFeature(Some("nose".into())));
    let drawn = draw(&mut workspace);
    for text in [
        "Shape",
        "Straight",
        "Aquiline",
        "Snub",
        "Low bridge",
        "Custom",
    ] {
        assert!(has(&drawn, text), "{text} in {drawn:?}");
    }
    assert!(!has(&drawn, "Heart-shaped"), "{drawn:?}");
    workspace.apply(Intent::PickFeature(Some("face_shape".into())));
    let drawn = draw(&mut workspace);
    for text in ["Shape", "Oval", "Round", "Square", "Heart-shaped"] {
        assert!(has(&drawn, text), "{text} in {drawn:?}");
    }
    assert!(!has(&drawn, "Aquiline"), "{drawn:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// "Save as preset" writes a person's own preset of the chosen kind into
/// `presets/` (only what the kind owns), lists it with the others, and a
/// new workspace finds it; a second of the same name gets its own file.
#[test]
fn save_as_preset_writes_a_preset_of_its_kind_into_the_workspace() {
    let folder = test_folder("feature-own-presets");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::shipped("female")).unwrap();
    workspace.apply(set("nose_length", 0.4));
    workspace.apply(set("fat", 0.3));
    workspace.apply(Intent::SaveAsPreset {
        kind: PresetKind::Face,
        name: "Long nose".into(),
    });
    let file = folder
        .join(presets::OWN_PRESETS_FOLDER)
        .join("long-nose.preset.json");
    assert!(file.is_file(), "{:?}", workspace.notice);
    let saved = Description::from_json(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let library = workspace.library();
    assert!(axcenity_avatar::check_preset(&saved, &library).is_empty());
    let info = saved.preset().unwrap();
    assert_eq!(info.id, "my/long-nose");
    assert_eq!(info.kind, PresetKind::Face);
    assert_eq!(saved.parameters.get("nose_length"), Some(&0.4));
    assert!(!saved.parameters.contains_key("fat"), "not a face value");
    let faces = workspace.presets_of(&library, PresetKind::Face);
    assert!(
        faces
            .iter()
            .any(|listed| listed.id == "my/long-nose" && listed.own)
    );

    // A body preset leaves out the body type and is tagged with it.
    workspace.apply(Intent::SaveAsPreset {
        kind: PresetKind::Body,
        name: "Soft".into(),
    });
    let soft = workspace.own_presets().get("my/soft").unwrap().clone();
    assert_eq!(soft.parameters.get("fat"), Some(&0.3));
    assert!(!soft.parameters.contains_key("frame"));
    assert_eq!(soft.preset().unwrap().tags, ["female"]);
    // The same name again: a file of its own.
    workspace.apply(Intent::SaveAsPreset {
        kind: PresetKind::Face,
        name: "Long nose".into(),
    });
    assert!(workspace.own_presets().contains_key("my/long-nose-2"));

    // Another workspace on the folder lists them and applies one.
    let mut other = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    other.apply(Intent::ApplyPreset("my/long-nose".into()));
    assert_eq!(other.parameter("nose_length"), 0.4);
    assert_eq!(
        other.parameter("frame"),
        0.0,
        "a face preset keeps the body"
    );
    other.apply(Intent::New(Start::Preset("my/long-nose".into())));
    other.answer(Some(false));
    assert_eq!(other.parameter("nose_length"), 0.4);
    // Nothing of the preset rides along into a character.
    assert!(other.description().extra.is_empty());
    let _ = std::fs::remove_dir_all(&folder);
}

/// Favourites are editor state: a small file beside Recent, kept for the
/// next time, never in the character and never an undo step.
#[test]
fn favourites_are_kept_beside_recent_and_never_in_the_character() {
    let folder = test_folder("feature-favourites");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let before = workspace.description().clone();
    workspace.apply(Intent::ToggleFavourite(FACE_ROUND.into()));
    assert!(workspace.favourites.contains(FACE_ROUND));
    assert!(folder.join(presets::FAVOURITES_FILE).is_file());
    assert_eq!(workspace.description(), &before);
    assert!(!workspace.can_undo());
    let again = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert!(again.favourites.contains(FACE_ROUND));
    workspace.apply(Intent::ToggleFavourite(FACE_ROUND.into()));
    assert!(presets::read_favourites(&folder).is_empty());
    // The Presets page draws a favourite's star.
    workspace.apply(Intent::ToggleFavourite(FACE_ROUND.into()));
    workspace.apply(Intent::PickCategory(Category::Presets));
    let drawn = draw(&mut workspace);
    assert!(
        drawn.iter().any(|t| t.contains(theme::filled_icons::STAR)),
        "{drawn:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

// ---- Randomize ----

/// Randomize draws the scope again as one undo step: everything, the
/// category or the feature. The body type is locked by default, a lock on
/// a slider row holds it, and locks are never in the character. The same
/// presses give the same characters (the seed is a counter).
#[test]
fn randomize_draws_its_scope_again_as_one_step_and_keeps_locks() {
    let folder = test_folder("feature-randomize");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert!(workspace.locks.contains("frame"), "the body type is locked");
    let start = workspace.description().clone();

    workspace.apply(Intent::Randomize(RandomScope::Everything));
    assert_ne!(workspace.description(), &start);
    assert_eq!(workspace.parameter("frame"), 0.0, "still the male body");
    assert_eq!(undo_depth(&workspace), 1);
    let library = workspace.library();
    assert!(workspace.description().validate(&library).is_empty());
    let first = workspace.description().clone();
    workspace.apply(Intent::Randomize(RandomScope::Everything));
    assert_ne!(
        workspace.description(),
        &first,
        "another press, another one"
    );
    workspace.apply(Intent::Undo);
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.description(), &start);

    // The same presses on another workspace: the same characters.
    let other_folder = test_folder("feature-randomize-again");
    let mut other = CharacterWorkspace::open_new(&other_folder, &Start::default()).unwrap();
    other.apply(Intent::Randomize(RandomScope::Everything));
    assert_eq!(other.description(), &first);
    drop(other);

    // The face only, with the nose locked: the body and the nose stay.
    workspace.apply(Intent::PickCategory(Category::Face));
    let nose: Vec<String> = library
        .base(DEFAULT_BASE)
        .unwrap()
        .parameters
        .iter()
        .filter(|p| p.feature.as_deref() == Some("nose"))
        .map(|p| p.id.clone())
        .collect();
    assert!(!nose.is_empty());
    workspace.apply(set("nose_length", 0.25));
    workspace.apply(Intent::SetLock {
        ids: nose.clone(),
        locked: true,
    });
    let before = workspace.description().clone();
    let body = ["height", "muscle", "fat"].map(|id| workspace.parameter(id));
    let depth = undo_depth(&workspace);
    workspace.apply(Intent::Randomize(RandomScope::Category));
    assert_eq!(undo_depth(&workspace), depth + 1);
    let after = workspace.description().clone();
    assert_ne!(after, before);
    for id in &nose {
        assert_eq!(
            workspace.parameter(id),
            before.parameters.get(id).copied().unwrap_or(0.0),
            "{id} is locked"
        );
    }
    assert_eq!(
        ["height", "muscle", "fat"].map(|id| workspace.parameter(id)),
        body,
        "the body is left"
    );
    assert!(workspace.description().extra.is_empty(), "no locks inside");

    // The feature only: the eyes.
    workspace.apply(Intent::PickFeature(Some("eyes".into())));
    let before = workspace.description().clone();
    workspace.apply(Intent::Randomize(RandomScope::Feature));
    let base = library.base(DEFAULT_BASE).unwrap();
    for parameter in &base.parameters {
        let was = before.parameters.get(&parameter.id);
        let now = workspace.description().parameters.get(&parameter.id);
        if selection::parameter_feature(parameter) != "eyes" {
            assert_eq!(was, now, "{} is outside the eyes", parameter.id);
        }
    }
    assert_ne!(workspace.description(), &before);

    // Unlocked, the nose is drawn again too; a lock is no undo step.
    let depth = undo_depth(&workspace);
    workspace.apply(Intent::SetLock {
        ids: nose.clone(),
        locked: false,
    });
    assert_eq!(undo_depth(&workspace), depth);
    workspace.apply(Intent::PickFeature(Some("nose".into())));
    workspace.apply(Intent::Randomize(RandomScope::Feature));
    assert_ne!(workspace.parameter("nose_length"), 0.25);

    // The toolbar offers the three scopes and says what's locked.
    let choices = toolbar::random_choices(&workspace.selection, &workspace, &library);
    let labels: Vec<&str> = choices.iter().map(|(l, _, _)| l.as_str()).collect();
    assert_eq!(labels, ["Everything", "Face", "Nose"]);
    assert!(choices[0].1.contains("1 locked value"), "{:?}", choices[0]);
    let _ = std::fs::remove_dir_all(&folder);
    let _ = std::fs::remove_dir_all(&other_folder);
}

/// Slider rows and the body type have a lock; the body type's is lit.
#[test]
fn slider_rows_and_the_body_type_draw_a_lock() {
    let folder = test_folder("feature-locks-drawn");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::PickCategory(Category::Body));
    let drawn = draw(&mut workspace);
    let count = |icon: &str| drawn.iter().filter(|t| t.as_str() == icon).count();
    let locked = count(crate::theme::icons::LOCK_SIMPLE);
    let open = count(crate::theme::icons::LOCK_SIMPLE_OPEN);
    assert!(locked >= 1, "the body type's lock is lit: {drawn:?}");
    assert!(open >= 3, "each slider has one: {drawn:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

// ---- Comparison (CO19) ----

/// Held versions: capture, show one in the character's place, side by
/// side with a second character, keep one as one undo step, discard. None
/// of it is an undo step or reaches the saved file.
#[test]
fn held_versions_never_enter_undo_or_the_saved_file() {
    let folder = test_folder("feature-compare");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::Rename("Compared".into()));
    workspace.apply(Intent::Capture);
    let a = workspace.description().clone();
    assert_eq!(workspace.comparison.held, std::slice::from_ref(&a));
    assert_eq!(undo_depth(&workspace), 1, "only the rename");
    workspace.apply(set("fat", 0.6));
    workspace.apply(Intent::Capture);
    let b = workspace.description().clone();
    workspace.apply(set("muscle", 0.8));
    let current = workspace.description().clone();
    let depth = undo_depth(&workspace);

    // Before and after: A in the character's place, the description and
    // undo untouched, the preview showing A.
    workspace.apply(Intent::ShowHeld(Some(0)));
    assert_eq!(workspace.shown(), &a);
    assert_eq!(workspace.description(), &current);
    assert_eq!(workspace.applied.as_ref().map(|(d, _)| d), Some(&a));
    assert_eq!(undo_depth(&workspace), depth);
    let drawn = draw(&mut workspace);
    assert!(has(&drawn, "Showing A (held)"), "{drawn:?}");
    workspace.apply(Intent::ShowHeld(None));
    assert_eq!(workspace.applied.as_ref().map(|(d, _)| d), Some(&current));

    // Side by side: a second character in the preview World shows B.
    workspace.apply(Intent::SideBySide(true));
    workspace.apply(Intent::ShowHeld(Some(1)));
    assert_eq!(workspace.other_shows(), Some(&b));
    assert_eq!(workspace.shown(), &current, "the current one stays");
    let drawn = draw(&mut workspace);
    assert!(
        has(&drawn, "B (held)") && has(&drawn, "Current"),
        "{drawn:?}"
    );
    workspace.apply(Intent::SideBySide(false));
    assert_eq!(workspace.other_shows(), None, "the second one is gone");

    // An edit while A is shown edits the current character, seen.
    workspace.apply(Intent::ShowHeld(Some(0)));
    workspace.apply(set("height", 1.05));
    assert_eq!(workspace.comparison.showing, None);
    assert_eq!(workspace.parameter("muscle"), 0.8);
    workspace.apply(Intent::Undo);
    let depth = undo_depth(&workspace);

    // Keep B: one undo step, the name kept.
    workspace.apply(Intent::ShowHeld(Some(1)));
    workspace.apply(Intent::KeepHeld(1));
    assert_eq!(workspace.description(), &b);
    assert_eq!(undo_depth(&workspace), depth + 1);
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.description(), &current);
    workspace.apply(Intent::Redo);

    // Four can't be held; discard drops one.
    workspace.apply(Intent::Capture);
    workspace.apply(Intent::Capture);
    assert_eq!(workspace.comparison.held.len(), compare::MAX_HELD);
    workspace.apply(Intent::DiscardHeld(0));
    assert_eq!(workspace.comparison.held.len(), 2);
    assert_eq!(workspace.comparison.held[0], b);

    // Saved: the file is the character, nothing of the comparison.
    workspace.apply(Intent::Save);
    let text = std::fs::read_to_string(workspace.file().unwrap()).unwrap();
    assert_eq!(
        &Description::from_json(&text).unwrap(),
        workspace.description()
    );
    assert!(
        !text.contains("held") && !text.contains("comparison"),
        "{text}"
    );
    // Another character drops the held versions.
    workspace.apply(Intent::New(Start::default()));
    assert!(workspace.comparison.held.is_empty());
    let _ = std::fs::remove_dir_all(&folder);
}

/// A piece of text drawn: what it says, where, its colour, and every
/// filled rectangle painted before it (later ones are on top).
struct DrawnText {
    text: String,
    rect: eframe::egui::Rect,
    color: eframe::egui::Color32,
    under: Vec<eframe::egui::epaint::RectShape>,
}

fn texts_and_backings(output: &eframe::egui::FullOutput) -> Vec<DrawnText> {
    use eframe::egui::{Color32, Shape, epaint::RectShape};
    fn walk(shape: &Shape, fills: &mut Vec<RectShape>, found: &mut Vec<DrawnText>) {
        match shape {
            Shape::Rect(rect) if rect.fill.a() > 0 => fills.push(rect.clone()),
            Shape::Text(text) => {
                let format = text.galley.job.sections.first().map(|s| s.format.color);
                let color = match (text.override_text_color, format) {
                    (Some(color), _) => color,
                    (None, Some(color)) if color != Color32::PLACEHOLDER => color,
                    _ => text.fallback_color,
                };
                found.push(DrawnText {
                    text: text.galley.text().to_owned(),
                    rect: eframe::egui::Rect::from_min_size(text.pos, text.galley.size()),
                    color,
                    under: fills.clone(),
                });
            }
            Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, fills, found)),
            _ => {}
        }
    }
    let (mut fills, mut found) = (Vec::new(), Vec::new());
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut fills, &mut found);
    }
    found
}

/// Review fix: the comparison strip's chips, its Discard icon and the
/// viewport's "held" and "Current" labels sit on a backing of their own,
/// not straight on the backdrop (whose colour the person picks: on the
/// Light one the quiet chip text could hardly be read). The backing is a
/// small, nearly opaque card behind the text, and the text reads on it at
/// AA (4.5:1) or better.
#[test]
fn the_comparison_controls_and_labels_read_on_any_backdrop() {
    use eframe::egui::Color32;
    let folder = test_folder("feature-compare-legible");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::SetBackdrop(stage::Backdrop::Light));
    workspace.apply(Intent::Capture);
    workspace.apply(set("fat", 0.9));
    let ctx = theme::test_context();
    let frame = |workspace: &mut CharacterWorkspace| {
        let mut texts = Vec::new();
        for _ in 0..2 {
            let output = theme::run(&ctx, input(Vec::new()), |ui| {
                workspace.ui(ui, None);
            });
            texts = texts_and_backings(&output);
        }
        texts
    };
    let check = |drawn: &[DrawnText], wanted: &str| {
        let text = drawn
            .iter()
            .find(|drawn| drawn.text.starts_with(wanted))
            .unwrap_or_else(|| panic!("{wanted:?} is not drawn"));
        // The nearest fill under the text that is a small card (not the
        // viewport's picture, or its stand-in without a GPU).
        let backing = text
            .under
            .iter()
            .rev()
            .find(|fill| {
                fill.rect.contains_rect(text.rect.shrink(1.0)) && fill.rect.height() <= 60.0
            })
            .unwrap_or_else(|| panic!("{wanted:?} has no backing of its own"));
        let [r, g, b, a] = backing.fill.to_srgba_unmultiplied();
        assert!(a >= 215, "{wanted:?}'s backing is too see-through ({a})");
        let ratio = theme::contrast(text.color, Color32::from_rgb(r, g, b));
        assert!(
            ratio >= 4.5,
            "{wanted:?} reads at {ratio:.2}:1 on its backing"
        );
    };

    // Before and after: A shown in the character's place.
    workspace.apply(Intent::ShowHeld(Some(0)));
    let drawn = frame(&mut workspace);
    for wanted in [
        "Current",
        "Side by side",
        theme::icons::TRASH,
        "Showing A (held)",
    ] {
        check(&drawn, wanted);
    }
    // Side by side: the two halves' labels and the strip.
    workspace.apply(Intent::ShowHeld(None));
    workspace.apply(Intent::SideBySide(true));
    let drawn = frame(&mut workspace);
    for wanted in ["A (held)", "Current", "Side by side"] {
        check(&drawn, wanted);
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// Review fix: turning side by side on and off leaves one floor for the
/// character, and one more only while the second character stands.
#[test]
fn side_by_side_on_and_off_leaves_no_floor_behind() {
    let folder = test_folder("feature-compare-floors");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let floors = |workspace: &CharacterWorkspace| {
        let world = &workspace.world;
        world
            .descendants(world.workspace())
            .into_iter()
            .filter(|id| world.get(*id).is_some_and(|i| i.name() == "Floor"))
            .count()
    };
    assert_eq!(floors(&workspace), 1);
    workspace.apply(Intent::Capture);
    for _ in 0..3 {
        workspace.apply(Intent::SideBySide(true));
        assert_eq!(floors(&workspace), 2, "the second character's own floor");
        workspace.apply(Intent::SideBySide(false));
        assert_eq!(floors(&workspace), 1, "its floor goes with it");
    }
    let _ = std::fs::remove_dir_all(&folder);
}

// ---- Autosave (CO20) ----

/// The recovery copy is written when the work changes, at most every few
/// seconds, never for an untouched new character, and removed on Save and
/// when the work is undone back to the saved character.
#[test]
fn autosave_writes_at_most_every_few_seconds_and_save_removes_it() {
    let folder = test_folder("feature-autosave");
    let recovery = folder.join(RECOVERY_FILE);
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let t0 = Instant::now();
    workspace.autosave_step(t0);
    assert!(!recovery.exists(), "an untouched new character isn't work");

    workspace.apply(set("fat", 0.5));
    workspace.autosave_step(t0);
    assert!(recovery.is_file());
    assert_eq!(
        autosave::read_recovery(&folder).unwrap().description,
        *workspace.description()
    );
    workspace.apply(set("fat", 0.7));
    workspace.autosave_step(t0 + Duration::from_secs(1));
    let held = autosave::read_recovery(&folder).unwrap().description;
    assert_eq!(held.parameters.get("fat"), Some(&0.5), "not yet");
    workspace.autosave_step(t0 + AUTOSAVE_EVERY);
    let held = autosave::read_recovery(&folder).unwrap().description;
    assert_eq!(held.parameters.get("fat"), Some(&0.7));

    // Never during a drag.
    workspace.apply(Intent::SetParameter {
        id: "fat".into(),
        value: 0.9,
        gesture: Gesture::Drag,
    });
    workspace.autosave_step(t0 + AUTOSAVE_EVERY * 3);
    let held = autosave::read_recovery(&folder).unwrap().description;
    assert_eq!(held.parameters.get("fat"), Some(&0.7));
    workspace.apply(Intent::CancelEdit);

    workspace.apply(Intent::Save);
    assert!(!recovery.exists(), "saved: nothing to recover");
    workspace.apply(set("muscle", 0.4));
    workspace.autosave_step(t0 + AUTOSAVE_EVERY * 4);
    assert!(recovery.is_file());
    workspace.apply(Intent::Undo);
    workspace.autosave_step(t0 + AUTOSAVE_EVERY * 5);
    assert!(!recovery.exists(), "back to the saved character");

    // A clean close removes it.
    workspace.apply(set("muscle", 0.4));
    workspace.autosave_step(t0 + AUTOSAVE_EVERY * 6);
    assert!(recovery.is_file());
    drop(workspace);
    assert!(!recovery.exists(), "closed cleanly");
    let _ = std::fs::remove_dir_all(&folder);
}

/// A crash leaves the recovery copy; the next open offers it, writes
/// nothing over it while it's offered, and Recover brings it back as
/// unsaved work on its own file; Discard throws it away.
#[test]
fn a_crash_leaves_a_recovery_that_the_next_open_offers() {
    let folder = test_folder("feature-recover");
    let recovery = folder.join(RECOVERY_FILE);
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::Rename("Lost".into()));
    workspace.apply(Intent::Save);
    let file = workspace.file().unwrap().to_owned();
    workspace.apply(set("fat", 0.5));
    workspace.autosave_step(Instant::now());
    let lost = workspace.description().clone();
    // A crash: nothing runs on the way out.
    std::mem::forget(workspace);
    assert!(recovery.is_file());

    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let offered = workspace.recovery.clone().expect("offered");
    assert_eq!(offered.description, lost);
    assert_eq!(offered.file.as_deref(), Some(file.as_path()));
    let drawn = draw(&mut workspace);
    let drawn = [drawn, draw(&mut workspace)].concat();
    assert!(has(&drawn, "Recover the unsaved character?"), "{drawn:?}");
    workspace.apply(set("muscle", 0.3));
    workspace.autosave_step(Instant::now() + AUTOSAVE_EVERY * 2);
    assert_eq!(
        autosave::read_recovery(&folder).unwrap().description,
        lost,
        "not written over while offered"
    );
    workspace.apply(Intent::Recover(true));
    assert_eq!(workspace.description(), &lost);
    assert_eq!(workspace.file(), Some(file.as_path()));
    assert!(workspace.dirty(), "recovered work isn't saved yet");
    workspace.apply(Intent::Save);
    assert_eq!(
        Description::from_json(&std::fs::read_to_string(&file).unwrap()).unwrap(),
        lost
    );
    assert!(!recovery.exists());
    drop(workspace);

    // Discard throws a recovery away.
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(set("fat", 0.2));
    workspace.autosave_step(Instant::now());
    std::mem::forget(workspace);
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert!(workspace.recovery.is_some());
    workspace.apply(Intent::Recover(false));
    assert!(workspace.recovery.is_none());
    assert!(!recovery.exists());
    assert_eq!(workspace.parameter("fat"), 0.0);
    let _ = std::fs::remove_dir_all(&folder);
}
