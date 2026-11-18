//! Wave 2's open list (refinement R10 polish): the status line while a
//! held version is shown, the Presets page's body tiles framed so the
//! builds read apart, and the stylized family's presets as data.

use axcenity_avatar::PresetKind;

use super::intent::{Gesture, Intent};
use super::shell_tests::{drawn_texts, input};
use super::thumbnails::{self, Subject};
use super::*;
use crate::character_studio::test_folder;
use crate::theme;

fn status_texts(workspace: &mut CharacterWorkspace) -> Vec<String> {
    let ctx = theme::test_context();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let output = theme::run(&ctx, input(Vec::new()), |ui| {
            workspace.ui(ui, None);
        });
        texts = drawn_texts(&output);
    }
    texts
}

/// While a held version is shown, the status line says so; it no longer
/// repeats the hint left from holding ("Held as A…").
#[test]
fn the_status_line_says_a_held_version_is_shown() {
    let folder = test_folder("polish-status");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::Capture);
    assert!(
        workspace
            .notice
            .as_deref()
            .unwrap()
            .starts_with("Held as A")
    );
    workspace.apply(Intent::SetParameter {
        id: "fat".into(),
        value: 0.8,
        gesture: Gesture::Commit,
    });
    workspace.apply(Intent::ShowHeld(Some(0)));
    let notice = workspace.notice.clone().unwrap();
    assert!(notice.starts_with("Showing A"), "{notice}");
    assert!(!notice.contains("Held as"), "{notice}");
    let drawn = status_texts(&mut workspace);
    assert!(
        drawn
            .iter()
            .any(|t| t.starts_with("Showing A") && t.contains("Current")),
        "{drawn:?}"
    );
    assert!(!drawn.iter().any(|t| t.contains("Held as A")), "{drawn:?}");

    workspace.apply(Intent::ShowHeld(None));
    let notice = workspace.notice.clone().unwrap();
    assert!(notice.contains("current character"), "{notice}");
    workspace.apply(Intent::SideBySide(true));
    let notice = workspace.notice.clone().unwrap();
    assert!(notice.contains("A beside"), "{notice}");
    workspace.apply(Intent::SideBySide(false));
    assert!(
        workspace
            .notice
            .as_deref()
            .unwrap()
            .contains("current character")
    );
    // Discarding the version shown, or starting another character, leaves
    // no status line about a held version that's gone.
    workspace.apply(Intent::ShowHeld(Some(0)));
    workspace.apply(Intent::DiscardHeld(0));
    assert_eq!(
        workspace.notice.as_deref(),
        Some("Showing the current character.")
    );
    workspace.apply(Intent::Capture);
    workspace.apply(Intent::ShowHeld(Some(0)));
    workspace.apply(Intent::New(Start::default()));
    workspace.answer(Some(false));
    assert_eq!(workspace.notice, None);
    let _ = std::fs::remove_dir_all(&folder);
}

/// The stylized family has presets as data beside its manifest (body,
/// face and whole characters, made with its own parameters): each is
/// sound, applies and resolves, New offers its characters, and a body
/// preset is offered for the frame it was made for.
#[test]
fn the_stylized_family_has_sound_presets_and_new_offers_them() {
    let folder = test_folder("polish-stylized-presets");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let library = workspace.library();
    let base = library.base(axcenity_avatar::V1_BASE).unwrap();
    for (kind, at_least) in [
        (PresetKind::Body, 4),
        (PresetKind::Face, 3),
        (PresetKind::Character, 2),
    ] {
        let listed = presets::of_kind(&library, workspace.own_presets(), &base.id, kind);
        assert!(listed.len() >= at_least, "{kind:?}: {}", listed.len());
        for listed in listed {
            assert!(listed.id.starts_with("axcenity/stylized-"), "{}", listed.id);
            let problems = axcenity_avatar::check_preset(listed.preset, &library);
            assert!(problems.is_empty(), "{}: {problems:?}", listed.id);
            // Only stylized parameters, inside their ranges.
            for (id, value) in &listed.preset.parameters {
                let parameter = base.parameter(id).unwrap_or_else(|| panic!("{id}"));
                let [low, high] = parameter.range;
                assert!((low..=high).contains(value), "{}: {id} {value}", listed.id);
            }
            let mut made = applier::default_description(&library, &base.id);
            axcenity_avatar::apply_preset(&mut made, listed.preset, &library).unwrap();
            assert!(made.validate(&library).is_empty(), "{}", listed.id);
            axcenity_avatar::resolve(&library, &made).unwrap();
            if kind != PresetKind::Character {
                assert_ne!(
                    made,
                    applier::default_description(&library, &base.id),
                    "{} changes the character",
                    listed.id
                );
            }
        }
    }
    // New lists the stylized characters (no longer only its default).
    let choices = toolbar::new_choices(&library, &workspace);
    let stylized: Vec<&Start> = choices
        .iter()
        .skip_while(|(label, _, start)| !(start.is_none() && label == "Stylized"))
        .filter_map(|(_, _, start)| start.as_ref())
        .collect();
    assert!(stylized.len() >= 2, "{choices:?}");
    assert!(
        stylized.iter().all(
            |start| matches!(start, Start::Preset(id) if id.starts_with("axcenity/stylized-"))
        ),
        "{stylized:?}"
    );
    // Its first is its default character, as the realistic one's is.
    assert_eq!(
        workspace.starting(stylized[0]),
        tidy(applier::default_description(&library, &base.id), &library)
    );

    // On a feminine stylized frame only its feminine body presets are
    // offered, and applying one keeps the frame.
    workspace.apply(Intent::New(Start::Family(base.id.clone())));
    workspace.answer(Some(false));
    workspace.apply(Intent::SetParameter {
        id: "frame".into(),
        value: 1.0,
        gesture: Gesture::Commit,
    });
    let offered: Vec<String> = presets::of_kind(
        &library,
        workspace.own_presets(),
        &base.id,
        PresetKind::Body,
    )
    .into_iter()
    .filter(|listed| presets::suits(listed, workspace.description(), base))
    .map(|listed| listed.id.to_owned())
    .collect();
    assert!(!offered.is_empty());
    for id in &offered {
        workspace.apply(Intent::ApplyPreset(id.clone()));
        assert_eq!(workspace.parameter("frame"), 1.0, "{id} keeps the frame");
    }
    let all = presets::of_kind(
        &library,
        workspace.own_presets(),
        &base.id,
        PresetKind::Body,
    )
    .len();
    assert!(
        offered.len() < all,
        "the masculine ones aren't offered here"
    );
    // The Body page's Start from chips show them.
    workspace.apply(Intent::PickCategory(Category::Body));
    let drawn = status_texts(&mut workspace);
    assert!(
        drawn
            .iter()
            .any(|t| t.to_lowercase().contains("start from")),
        "{drawn:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// A body preset's tile is framed on the body (not the whole figure, small
/// in a tall picture), so the builds read apart: neighbouring body tiles
/// differ in at least 3% of their pixels (a pixel counts when a channel
/// moves by more than 15, the plan's R3 measure).
#[test]
fn body_preset_tiles_are_framed_so_the_builds_read_apart() {
    let folder = test_folder("polish-body-tiles");
    let workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let library = workspace.library();
    let bodies: Vec<_> = workspace
        .presets_of(&library, PresetKind::Body)
        .into_iter()
        .filter(|listed| {
            presets::suits(
                listed,
                workspace.description(),
                library.base(DEFAULT_BASE).unwrap(),
            )
        })
        .collect();
    assert!(bodies.len() >= 7);
    for listed in &bodies {
        assert!(
            matches!(
                thumbnails::preset_subject(listed, &library),
                Subject::Body(_)
            ),
            "{} is framed on the body",
            listed.id
        );
    }
    let instance = axcenity_render::adapters::instance();
    let Some((device, queue)) =
        pollster::block_on(axcenity_render::adapters::adapter(&instance, None))
            .ok()
            .and_then(|adapter| {
                pollster::block_on(adapter.request_device(&Default::default())).ok()
            })
    else {
        eprintln!("no GPU: the pictures aren't drawn");
        return;
    };
    let mut painter = thumbnails::Painter::new(&folder, &device, &queue).unwrap();
    let pictures: Vec<Vec<u8>> = bodies
        .iter()
        .map(|listed| painter.paint(&thumbnails::preset_subject(listed, &library)))
        .collect();
    let mut report = Vec::new();
    for (index, pair) in pictures.windows(2).enumerate() {
        let differ = pair[0]
            .chunks_exact(4)
            .zip(pair[1].chunks_exact(4))
            .filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 15))
            .count();
        let share = differ as f32 / (pair[0].len() / 4) as f32;
        report.push((
            bodies[index].label().to_owned(),
            bodies[index + 1].label().to_owned(),
            share,
        ));
    }
    eprintln!("neighbouring body tiles differ by: {report:?}");
    for (a, b, share) in &report {
        assert!(
            *share >= 0.03,
            "{a} and {b} differ by {:.1}%",
            share * 100.0
        );
    }
    drop(painter);
    let _ = std::fs::remove_dir_all(&folder);
}

/// With `AXCENITY_SNAPSHOTS` naming a folder, the pages wave 3 changed,
/// drawn for real at 1440 by 900 under the Grey and the Light backdrop:
/// the Face page's Eyes with uneven sides (the Left and right menu and the
/// pair buttons), the Expressions page mid-walk with a smile, the Body
/// page mid-walk (the bottom bar's motion menu), the Presets page's body
/// tiles, a held version shown (the status line) and the stylized Body
/// page's Start from chips.
#[test]
fn wave3_pages_are_drawn() {
    use super::shell_tests::{paint_preset_pictures, snap};
    use super::stage::Backdrop;
    let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(std::path::PathBuf::from)
    else {
        return;
    };
    let folder = test_folder("polish-snapshots");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    paint_preset_pictures(&folder, &workspace);
    for backdrop in [Backdrop::Grey, Backdrop::Light] {
        let suffix = if backdrop == Backdrop::Grey {
            ""
        } else {
            "-light"
        };
        workspace.apply(Intent::SetBackdrop(backdrop));
        workspace.apply(Intent::Reset(super::edit::Scope::All));
        workspace.apply(Intent::PreviewMotion(Motion::Idle));

        // Face › Eyes, the sides uneven.
        workspace.apply(Intent::PickCategory(Category::Face));
        workspace.apply(Intent::PickFeature(Some("eyes".into())));
        workspace.apply(Intent::SetSymmetry(false));
        for (id, value) in [("eye_size_left", 0.6), ("eye_tilt_right", -0.5)] {
            workspace.apply(Intent::SetParameter {
                id: id.into(),
                value,
                gesture: Gesture::Commit,
            });
        }
        workspace.apply(Intent::SetSymmetry(true));
        snap(
            &mut workspace,
            &snapshots,
            &format!("w3-face-eyes-sides{suffix}"),
            3,
        );

        // Expressions: walking, smiling, framed on the face.
        workspace.apply(Intent::PickFeature(None));
        workspace.apply(Intent::PickCategory(Category::Expressions));
        workspace.apply(Intent::PreviewMotion(Motion::Walk));
        workspace.apply(Intent::PreviewExpression(Some("Happy".into())));
        for _ in 0..20 {
            workspace.advance(1.0 / 30.0);
        }
        snap(
            &mut workspace,
            &snapshots,
            &format!("w3-expressions-walk{suffix}"),
            3,
        );

        // The Body page, the whole character mid-walk.
        workspace.apply(Intent::PickCategory(Category::Body));
        for _ in 0..8 {
            workspace.advance(1.0 / 30.0);
        }
        snap(
            &mut workspace,
            &snapshots,
            &format!("w3-body-walk{suffix}"),
            3,
        );
        workspace.apply(Intent::FocusView(ViewPreset::Side));
        snap(
            &mut workspace,
            &snapshots,
            &format!("w3-body-walk-side{suffix}"),
            3,
        );
        workspace.apply(Intent::FocusView(ViewPreset::FullBody));
        workspace.apply(Intent::PreviewMotion(Motion::Gesture("Wave".into())));
        for _ in 0..25 {
            workspace.advance(1.0 / 30.0);
        }
        snap(
            &mut workspace,
            &snapshots,
            &format!("w3-body-wave{suffix}"),
            3,
        );
        workspace.apply(Intent::PreviewMotion(Motion::Idle));

        // The Presets page, its body tiles framed on the body. Once: the
        // page is the panel's, whatever the backdrop, and each snapshot
        // has a GPU device of its own, so a second one would show the
        // first one's textures, which it can't draw.
        workspace.apply(Intent::Reset(super::edit::Scope::All));
        if backdrop == Backdrop::Grey {
            workspace.apply(Intent::PickCategory(Category::Presets));
            snap(&mut workspace, &snapshots, "w3-presets", 10);
        }

        // A held version shown: the status line says so.
        workspace.apply(Intent::PickCategory(Category::Body));
        workspace.apply(Intent::Capture);
        workspace.apply(Intent::SetParameter {
            id: "fat".into(),
            value: 0.9,
            gesture: Gesture::Commit,
        });
        workspace.apply(Intent::ShowHeld(Some(0)));
        snap(
            &mut workspace,
            &snapshots,
            &format!("w3-showing-a{suffix}"),
            3,
        );
        workspace.apply(Intent::DiscardHeld(0));
    }
    // The stylized family's Body page with its Start from chips, on a
    // feminine frame.
    workspace.apply(Intent::SetBackdrop(super::stage::Backdrop::Grey));
    workspace.apply(Intent::New(Start::Preset(
        "axcenity/stylized-feminine".into(),
    )));
    workspace.answer(Some(false));
    workspace.apply(Intent::PickCategory(Category::Body));
    snap(&mut workspace, &snapshots, "w3-body-stylized", 3);
    workspace.apply(Intent::PickCategory(Category::Presets));
    paint_preset_pictures(&folder, &workspace);
    snap(&mut workspace, &snapshots, "w3-presets-stylized", 10);
    drop(workspace);
    let _ = std::fs::remove_dir_all(&folder);
}
