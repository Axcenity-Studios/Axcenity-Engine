//! Pose and motion in the preview (refinement R9): Still, Idle, Walk (the
//! Walk take at its own speed, in place), Run and the gestures; preview
//! only, expressions on top, the camera keeping a moving character framed.

use axcenity_core::Value;
use axcenity_render::glam::Vec3;

use super::intent::{Gesture, Intent};
use super::motion::{GESTURES, Motion, PREVIEW_CLIPS_FOLDER};
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

fn state_machine(workspace: &CharacterWorkspace) -> String {
    let animator = workspace
        .world
        .find_first_child(body(workspace), "Animator")
        .unwrap();
    workspace
        .world
        .get(animator)
        .unwrap()
        .property("StateMachine")
        .and_then(Value::as_str)
        .unwrap()
        .to_owned()
}

/// The preview's clip file, read.
fn preview_clip(workspace: &CharacterWorkspace) -> axcenity_anim::ClipFile {
    let animation = workspace.motion_animation().expect("a take plays");
    let path = workspace
        .world
        .get(animation)
        .unwrap()
        .property("Clip")
        .and_then(Value::as_str)
        .unwrap()
        .to_owned();
    assert!(path.starts_with(PREVIEW_CLIPS_FOLDER), "{path}");
    let text = std::fs::read_to_string(workspace.folder().join(&path)).unwrap();
    axcenity_anim::ClipFile::from_json(&text).unwrap()
}

/// The bones' positions, model space, as posed.
fn bones(workspace: &CharacterWorkspace) -> Vec<Vec3> {
    workspace.poses()[&body(workspace)]
        .bones()
        .iter()
        .map(|bone| bone.position)
        .collect()
}

fn largest_move(a: &[Vec3], b: &[Vec3]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0, f32::max)
}

#[test]
fn walk_plays_the_walk_take_at_its_own_speed_in_place() {
    let folder = test_folder("motion-walk");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    run_for(&mut workspace, 0.5);
    let idle = bones(&workspace);
    workspace.apply(Intent::PreviewMotion(Motion::Walk));
    assert_eq!(workspace.motion, Motion::Walk);
    assert_eq!(state_machine(&workspace), "", "the take plays alone");

    // The Walk take, whole, over its own length: its own speed.
    let clip = preview_clip(&workspace);
    let link = clip.link.clone().unwrap();
    assert_eq!(link.take, "Walk");
    assert_eq!(
        link.source,
        axcenity_project::DEFAULT_HUMANOID_ANIMATIONS_PATH
    );
    assert!((clip.duration - 1.0667).abs() < 0.01, "{}", clip.duration);
    assert_eq!((link.start, link.end), (0.0, clip.duration));
    assert!(clip.looping);
    let animation = workspace.motion_animation().unwrap();
    let speed = workspace
        .world
        .get(animation)
        .unwrap()
        .property("Speed")
        .and_then(Value::as_number);
    assert_eq!(speed, Some(1.0));
    // The game's Walk state still plays the Run take (the controller's
    // WalkSpeed is a jog): the true walk is the studio's preview only.
    let game_walk = std::fs::read_to_string(folder.join(format!(
        "{}/Walk.anim.json",
        axcenity_project::DEFAULT_HUMANOID_CLIPS_FOLDER
    )))
    .unwrap();
    assert!(game_walk.contains("\"take\": \"Run\""), "{game_walk}");

    run_for(&mut workspace, 0.4);
    let time = workspace
        .world
        .property_animation(animation)
        .map(|state| state.time)
        .unwrap();
    assert!((time - 0.4).abs() < 0.05, "a second a second: {time}");
    let walking = bones(&workspace);
    assert!(
        largest_move(&idle, &walking) > 0.05,
        "the legs move: {}",
        largest_move(&idle, &walking)
    );

    // In place: over three cycles the head sways but doesn't travel.
    let mut heads = Vec::new();
    for _ in 0..96 {
        workspace.step_by(FRAME);
        heads.push(workspace.head().unwrap());
    }
    let mean = |range: &[Vec3]| range.iter().copied().sum::<Vec3>() / range.len() as f32;
    let (first, last) = (mean(&heads[..32]), mean(&heads[64..]));
    let travel = Vec3::new(last.x - first.x, 0.0, last.z - first.z).length();
    assert!(travel < 0.1, "the walk stays in place: {travel} studs");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn still_idle_run_and_the_gestures_each_pose_the_preview() {
    let folder = test_folder("motion-each");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert_eq!(
        workspace.motion,
        Motion::Idle,
        "the preview idles, as before"
    );
    assert_eq!(
        state_machine(&workspace),
        axcenity_project::DEFAULT_HUMANOID_STATE_MACHINE_PATH
    );

    // Still: the rest pose, nothing playing; it holds.
    workspace.apply(Intent::PreviewMotion(Motion::Still));
    assert_eq!(state_machine(&workspace), "");
    assert!(workspace.motion_animation().is_none());
    run_for(&mut workspace, 0.3);
    let still = bones(&workspace);
    run_for(&mut workspace, 0.5);
    assert!(
        largest_move(&still, &bones(&workspace)) < 1.0e-4,
        "it holds"
    );

    workspace.apply(Intent::PreviewMotion(Motion::Run));
    assert_eq!(preview_clip(&workspace).link.unwrap().take, "Run");
    run_for(&mut workspace, 0.3);
    assert!(largest_move(&still, &bones(&workspace)) > 0.05, "it runs");

    // Every gesture the default character has.
    assert!(Motion::all().len() >= 4 + GESTURES.len());
    for name in GESTURES {
        workspace.apply(Intent::PreviewMotion(Motion::Gesture(name.into())));
        assert_eq!(preview_clip(&workspace).link.unwrap().take, name);
        run_for(&mut workspace, 0.5);
        assert!(
            largest_move(&still, &bones(&workspace)) > 0.02,
            "{name} moves"
        );
    }
    // One Animation of the preview's own, however many were picked.
    let animations = workspace
        .world
        .get(body(&workspace))
        .unwrap()
        .children()
        .iter()
        .filter(|child| {
            workspace.world.get(**child).unwrap().class() == axcenity_core::ClassName::Animation
        })
        .count();
    assert_eq!(animations, 1);

    // Idle again: the state machine plays, the take is gone.
    workspace.apply(Intent::PreviewMotion(Motion::Idle));
    assert!(workspace.motion_animation().is_none());
    assert_eq!(
        state_machine(&workspace),
        axcenity_project::DEFAULT_HUMANOID_STATE_MACHINE_PATH
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// A motion is editor state: no undo step, no change to the description,
/// nothing in the saved file; an edit while walking keeps it walking.
#[test]
fn a_motion_is_never_saved_or_undone() {
    let folder = test_folder("motion-unsaved");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::Rename("Walker".into()));
    workspace.apply(Intent::Save);
    let saved = workspace.description().clone();
    let can_undo = workspace.can_undo();
    workspace.apply(Intent::PreviewMotion(Motion::Walk));
    run_for(&mut workspace, 0.2);
    assert_eq!(workspace.description(), &saved);
    assert!(!workspace.dirty(), "a walk is no change");
    assert_eq!(workspace.can_undo(), can_undo, "and no undo step");
    workspace.apply(Intent::Undo);
    assert_eq!(
        workspace.motion,
        Motion::Walk,
        "undo leaves the preview alone"
    );

    // An edit while walking: the walk goes on.
    workspace.apply(Intent::SetParameter {
        id: "fat".into(),
        value: 0.5,
        gesture: Gesture::Commit,
    });
    run_for(&mut workspace, 0.2);
    assert!(workspace.motion_animation().is_some());
    assert_eq!(state_machine(&workspace), "");
    workspace.apply(Intent::Save);
    let text = std::fs::read_to_string(workspace.file().unwrap()).unwrap();
    for word in ["Walk", "motion", "Preview", "anim"] {
        assert!(!text.contains(word), "{word} in {text}");
    }
    let reopened = CharacterWorkspace::open_file(&folder, workspace.file().unwrap()).unwrap();
    assert_eq!(reopened.description(), workspace.description());
    assert_eq!(reopened.motion, Motion::Idle);
    drop(reopened);

    // A new character keeps the preview's motion (like the light), and
    // the held version beside it moves too.
    workspace.apply(Intent::Capture);
    workspace.apply(Intent::SideBySide(true));
    let other = applier::body_of(&workspace.world, workspace.other.unwrap()).unwrap();
    let plays = workspace
        .world
        .get(other)
        .unwrap()
        .children()
        .iter()
        .any(|child| workspace.world.get(*child).unwrap().name() == motion::PREVIEW_MOTION);
    assert!(plays, "side by side, both walk");
    let _ = std::fs::remove_dir_all(&folder);
}

/// An expression still shows on the face while the body walks.
#[test]
fn an_expression_shows_on_top_of_a_walk() {
    let folder = test_folder("motion-expression");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let weights = |workspace: &CharacterWorkspace| -> f32 {
        workspace.poses()[&body(workspace)]
            .morph_weights()
            .iter()
            .map(|w| w.abs())
            .sum()
    };
    workspace.apply(Intent::PickCategory(Category::Expressions));
    workspace.apply(Intent::PreviewMotion(Motion::Walk));
    run_for(&mut workspace, 0.5);
    let walking = weights(&workspace);
    workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
    run_for(&mut workspace, 1.0);
    assert!(
        weights(&workspace) > walking + 0.5,
        "it smiles while walking: {walking} -> {}",
        weights(&workspace)
    );
    assert!(workspace.motion_animation().is_some(), "and still walks");
    let _ = std::fs::remove_dir_all(&folder);
}

/// Where the head is in the picture, across and up from its middle, as a
/// share of half the picture's height (1 is the top edge).
fn in_picture(workspace: &CharacterWorkspace) -> axcenity_render::glam::Vec2 {
    let camera = workspace.orbit.camera();
    let head = workspace.head().unwrap();
    let forward = (camera.look_at - camera.position).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward);
    let to_head = head - camera.position;
    let depth = to_head.dot(forward);
    let half = (camera.fov_y_degrees * 0.5).to_radians().tan();
    axcenity_render::glam::Vec2::new(to_head.dot(right), to_head.dot(up)) / (depth * half)
}

/// The camera keeps a moving character framed: in the Face view, a run
/// (the most movement) keeps the head near the middle of the picture,
/// and back to Still the view's own framing returns.
#[test]
fn the_camera_keeps_framing_a_moving_character() {
    let folder = test_folder("motion-camera");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::PickCategory(Category::Face));
    assert_eq!(workspace.view, Some(ViewPreset::Face));
    run_for(&mut workspace, 0.5);
    let resting = in_picture(&workspace);
    let framed = workspace.orbit;
    for motion in [Motion::Run, Motion::Gesture("Cheer".into())] {
        workspace.apply(Intent::PreviewMotion(motion.clone()));
        run_for(&mut workspace, 1.0);
        // Over two seconds: where the head sits on average (the camera
        // follows that), and how far a step's bob takes it from there.
        let mut seen = Vec::new();
        for _ in 0..60 {
            workspace.advance(FRAME);
            seen.push(in_picture(&workspace));
        }
        let mean = seen.iter().copied().sum::<axcenity_render::glam::Vec2>() / seen.len() as f32;
        let worst = seen.iter().map(|at| at.length()).fold(0.0, f32::max);
        eprintln!(
            "{motion:?}: head at rest {:.3} off the middle; moving, on average {:.3}, at worst {worst:.3}",
            resting.length(),
            mean.length()
        );
        assert!(
            mean.length() < 0.1,
            "{motion:?}: on average the head is near the middle ({:.3})",
            mean.length()
        );
        assert!(
            worst < 0.35,
            "{motion:?}: and never near the edge ({worst:.3})"
        );

        assert!(workspace.following().is_some());
        assert_eq!(
            workspace.view,
            Some(ViewPreset::Face),
            "still the Face view"
        );
    }
    workspace.apply(Intent::PreviewMotion(Motion::Still));
    assert_eq!(workspace.following(), None);
    run_for(&mut workspace, 0.2);
    assert!(
        workspace.orbit.target.distance(framed.target) < 0.2,
        "the Face view frames the still head again"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// The controls: the Expressions page's Pose and motion chips and the
/// bottom bar's motion menu.
#[test]
fn pose_and_motion_are_on_the_expressions_page_and_the_bottom_bar() {
    let folder = test_folder("motion-drawn");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let ctx = crate::theme::test_context();
    let draw = |workspace: &mut CharacterWorkspace| {
        let mut texts = Vec::new();
        for _ in 0..2 {
            let output = crate::theme::run(&ctx, super::shell_tests::input(Vec::new()), |ui| {
                workspace.ui(ui, None);
            });
            texts = super::shell_tests::drawn_texts(&output);
        }
        texts
    };
    let body_page = draw(&mut workspace);
    assert!(
        body_page.iter().any(|t| t.contains("Idle")),
        "the bottom bar's motion menu: {body_page:?}"
    );
    workspace.apply(Intent::PickCategory(Category::Expressions));
    let page = draw(&mut workspace);
    for text in [
        "Pose and motion",
        "Still",
        "Walk",
        "Run",
        "Wave",
        "Shrug",
        "Happy",
    ] {
        assert!(
            page.iter()
                .any(|t| t.to_lowercase().contains(&text.to_lowercase())),
            "{text} in {page:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&folder);
}
