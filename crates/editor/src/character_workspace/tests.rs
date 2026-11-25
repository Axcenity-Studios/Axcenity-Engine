use std::collections::BTreeMap;

use super::stage::ViewPreset;
use super::*;
use crate::character_studio::test_folder;
use crate::theme;

/// The body's `MeshId` and the parts it wears, as [`CharacterWorkspace::worn`]
/// lists them, with derived models' identity keys left out (a key names a
/// bake, which the test needn't pin).
fn worn_models(workspace: &CharacterWorkspace) -> Vec<String> {
    workspace
        .worn()
        .into_iter()
        .map(|name| axcenity_assets::split_derived(&name).0.to_owned())
        .collect()
}

/// Overhaul CO12: the workspace offers every body family the engine ships
/// (one: the human base, ADR-0047) and shows it with its own parts.
#[test]
fn the_workspace_offers_the_human_body_and_shows_it() {
    let folder = test_folder("workspace-human");
    let started = Instant::now();
    let workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    println!("opened on the human body in {:?}", started.elapsed());
    let library = workspace.library();
    assert!(
        library.base(DEFAULT_BASE).is_some(),
        "the shipped library was copied in"
    );
    assert_eq!(DEFAULT_BASE, "axcenity/human");
    assert_eq!(workspace.description().base, DEFAULT_BASE);
    assert_eq!(workspace.notice, None);

    let worn = worn_models(&workspace);
    assert!(
        worn[0].starts_with("assets/characters/human/HumanParts.glb#AxcenityHuman/Body"),
        "the body is the human one: {worn:?}"
    );
    for part in ["Eyes", "Teeth", "Tongue", "Lashes", "Brows", "BoxerBriefs"] {
        assert!(worn.iter().any(|name| name == part), "{part} in {worn:?}");
    }
    // Every family the library has is offered (plan section 0, decision
    // 7): the engine ships the human base alone.
    assert!(offers_base(&library, DEFAULT_BASE));
    assert!(!offers_base(&library, axcenity_avatar::V1_BASE));
    let families: Vec<&str> = presets::families(&library)
        .iter()
        .map(|base| base.id.as_str())
        .collect();
    assert_eq!(families, [DEFAULT_BASE]);
    let _ = std::fs::remove_dir_all(&folder);
}

/// The human body is Male or Female, a choice and not a slider: each wears
/// its own underwear (boxer briefs; the female one a bandeau top too), the
/// switch is one undo step, and a value in between (from an older file) is
/// taken to the nearest.
#[test]
fn male_or_female_is_a_choice_that_wears_its_own_underwear() {
    const BRIEFS: &str = "axcenity/human-boxer-briefs";
    const BANDEAU: &str = "axcenity/human-bandeau";
    let folder = test_folder("workspace-frame");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let frame = workspace
        .base()
        .unwrap()
        .parameter("frame")
        .unwrap()
        .clone();
    let labels: Vec<&str> = frame.choices.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["Male", "Female"]);
    assert_eq!(workspace.parameter("frame"), 0.0);
    assert_eq!(
        workspace.worn_in("underwear_bottom").as_deref(),
        Some(BRIEFS)
    );
    assert_eq!(workspace.worn_in("underwear_top"), None);

    workspace.choose(&frame, 1.0);
    assert_eq!(workspace.parameter("frame"), 1.0);
    assert_eq!(
        workspace.worn_in("underwear_bottom").as_deref(),
        Some(BRIEFS)
    );
    assert_eq!(workspace.worn_in("underwear_top").as_deref(), Some(BANDEAU));
    let worn = worn_models(&workspace);
    for part in ["Bandeau", "BoxerBriefs"] {
        assert!(worn.iter().any(|name| name == part), "{part} in {worn:?}");
    }

    workspace.undo();
    assert_eq!(workspace.parameter("frame"), 0.0);
    assert_eq!(workspace.worn_in("underwear_top"), None);
    let worn = worn_models(&workspace);
    assert!(!worn.iter().any(|name| name == "Bandeau"), "{worn:?}");

    // The underwear follows the choice and can't be taken off or swapped:
    // the Clothing page has no chips for it, and an edit that tries is
    // put right.
    let chosen = chosen_slots(&workspace.library(), DEFAULT_BASE);
    assert_eq!(chosen, ["underwear_bottom", "underwear_top"]);
    workspace.choose(&frame, 1.0);
    workspace.wear("underwear_top", None);
    workspace.wear("underwear_bottom", Some("makehuman/female-panties-01"));
    assert_eq!(
        workspace.worn_in("underwear_bottom").as_deref(),
        Some(BRIEFS)
    );
    assert_eq!(workspace.worn_in("underwear_top").as_deref(), Some(BANDEAU));
    workspace.wear("underwear_top", Some("makehuman/camisole-top"));
    assert_eq!(workspace.worn_in("underwear_top").as_deref(), Some(BANDEAU));
    workspace.edit(|description| description.items.clear());
    assert_eq!(
        workspace.worn_in("underwear_bottom").as_deref(),
        Some(BRIEFS)
    );
    assert_eq!(workspace.worn_in("underwear_top").as_deref(), Some(BANDEAU));
    workspace.choose(&frame, 0.0);
    workspace.wear("underwear_top", Some(BANDEAU));
    assert_eq!(workspace.worn_in("underwear_top"), None);
    assert_eq!(
        workspace.worn_in("underwear_bottom").as_deref(),
        Some(BRIEFS)
    );

    // A saved character without its underwear (an older file) gets it.
    let mut bare = workspace.description().clone();
    bare.parameters.insert("frame".to_owned(), 1.0);
    bare.items.clear();
    let reopened = CharacterWorkspace::open(&folder, bare, true).unwrap();
    assert_eq!(
        reopened.worn_in("underwear_bottom").as_deref(),
        Some(BRIEFS)
    );
    assert_eq!(reopened.worn_in("underwear_top").as_deref(), Some(BANDEAU));

    // A value between the two is taken to the nearest choice.
    workspace.edit(|description| {
        description.parameters.insert("frame".to_owned(), 0.7);
    });
    assert_eq!(workspace.parameter("frame"), 1.0);
    let _ = std::fs::remove_dir_all(&folder);
}

/// A slider on the human body changes the preview's identity, worn
/// live (ADR-0041): the body keeps its base model and wears a new identity
/// key, nothing is baked, and a drag is shown while it goes on.
#[test]
fn a_slider_changes_the_preview_while_it_is_dragged() {
    let folder = test_folder("workspace-slider");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let body = applier::body_of(&workspace.world, workspace.character.unwrap()).unwrap();
    let identity = |workspace: &CharacterWorkspace| {
        axcenity_anim::identity_key(&workspace.world, body).expect("worn live")
    };
    let mesh = workspace.worn()[0].clone();
    let before = identity(&workspace);
    let base = workspace.base().unwrap();
    let fat = base.parameter("fat").unwrap().clone();

    let started = Instant::now();
    workspace.set_parameter(&fat, 0.8, true);
    println!("applied a slider in {:?}", started.elapsed());
    assert_eq!(workspace.parameter("fat"), 0.8);
    let after = identity(&workspace);
    assert_ne!(after, before, "a new identity");
    assert_eq!(workspace.worn()[0], mesh, "on the same base model");

    // While a slider is dragged, every step shows it (no throttle: a
    // live change is new weights).
    workspace.set_parameter(&fat, 0.2, false);
    assert_ne!(identity(&workspace), after, "shown during the drag");
    workspace.set_parameter(&fat, 0.3, true);
    workspace.undo();
    assert_eq!(
        workspace.parameter("fat"),
        0.8,
        "the drag was one undo step"
    );
    assert_eq!(identity(&workspace), after);
    assert_eq!(workspace.assets.derived_count(), 0, "nothing was baked");
    let _ = std::fs::remove_dir_all(&folder);
}

/// With Symmetry on a slider moves its mirror too; off, only itself.
#[test]
fn symmetry_moves_a_parameters_mirror_too() {
    let folder = test_folder("workspace-symmetry");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let base = workspace.base().unwrap();
    let left = base
        .parameters
        .iter()
        .find(|parameter| parameter.mirror.is_some())
        .expect("the human face has mirrored pairs")
        .clone();
    let right = left.mirror.clone().unwrap();
    let [low, high] = left.range;
    let value = low.max(high) * 0.5;

    workspace.set_parameter(&left, value, true);
    assert_eq!(workspace.parameter(&left.id), value);
    assert_eq!(workspace.parameter(&right), value);

    workspace.symmetry = false;
    workspace.set_parameter(&left, 0.0, true);
    assert_eq!(workspace.parameter(&left.id), 0.0);
    assert_eq!(workspace.parameter(&right), value);
    assert_eq!(symmetric_label("Right ear protrusion"), "Ear protrusion");
    let _ = std::fs::remove_dir_all(&folder);
}

/// A character file CS2's workspace saved (format 1) is read as a
/// character on the stylized body wearing what it wore. The library no
/// longer ships that body, so the workspace shows the default character
/// under the file's name and says so; what it saves is format 2.
#[test]
fn a_format_1_character_file_opens_as_the_default_character_with_a_notice() {
    let folder = test_folder("workspace-format-1");
    let file = folder.join("Old.character.json");
    std::fs::write(
        &file,
        r#"{"format_version": 1, "name": "Old", "build": 0.4, "muscle": 0.2,
            "hair": "Hair_Bob", "top": "Shirt_Tee", "outerwear": null,
            "bottom": "Pants_Cargo", "shoes": "Shoes_Sneakers"}"#,
    )
    .unwrap();
    // The file still migrates: the old Look is a stylized character.
    let migrated = read_character(&file).unwrap();
    assert_eq!(migrated.base, axcenity_avatar::V1_BASE);
    assert_eq!(migrated.name, "Old");
    assert!(
        migrated
            .items
            .iter()
            .any(|worn| worn.id == "axcenity/hair-bob"),
        "{:?}",
        migrated.items
    );

    let mut workspace = CharacterWorkspace::open_file(&folder, &file).unwrap();
    assert!(!workspace.dirty());
    let description = workspace.description().clone();
    let library = workspace.library();
    let mut default = applier::default_description(&library, DEFAULT_BASE);
    default.name = "Old".to_owned();
    assert_eq!(description, tidy(default, &library));
    let notice = workspace.notice.clone().expect("the fallback is said");
    assert!(
        notice.contains("\"Old\"") && notice.contains(axcenity_avatar::V1_BASE),
        "{notice}"
    );
    assert_eq!(workspace.worn_in("hair"), None);
    let worn = worn_models(&workspace);
    assert!(
        worn[0].starts_with("assets/characters/human/HumanParts.glb#"),
        "{worn:?}"
    );

    let saved = workspace.save().unwrap();
    let text = std::fs::read_to_string(&saved).unwrap();
    assert_eq!(Description::from_json(&text).unwrap(), description);
    assert!(text.contains("\"format_version\": 2"), "{text}");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_workspace_dresses_the_preview_undoes_and_saves() {
    const BRIEFS: &str = "axcenity/human-boxer-briefs";
    let folder = test_folder("workspace");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::shipped("female")).unwrap();
    // The humanoid's files are written for the preview.
    assert!(
        folder
            .join(axcenity_project::DEFAULT_HUMANOID_STATE_MACHINE_PATH)
            .is_file()
    );
    assert!(workspace.dirty(), "a new character isn't saved yet");
    let default_bottom = workspace.worn_in("underwear_bottom");
    assert_eq!(default_bottom.as_deref(), Some(BRIEFS));
    assert_eq!(
        workspace.worn_in("underwear_top").as_deref(),
        Some("axcenity/human-bandeau")
    );
    let worn = workspace.worn().join(" ");
    assert!(worn.contains("Bandeau"), "{worn}");

    let frame = workspace
        .base()
        .unwrap()
        .parameter("frame")
        .unwrap()
        .clone();
    workspace.choose(&frame, 0.0);
    let worn = workspace.worn().join(" ");
    assert!(worn.contains("BoxerBriefs"), "{worn}");
    assert!(!worn.contains("Bandeau"), "{worn}");
    // Only items that fit this body, in the slot asked for, are offered.
    let library = workspace.library();
    let bottoms = workspace.items_for("underwear_bottom");
    assert!(bottoms.iter().any(|(id, _)| id == BRIEFS));
    for (id, _) in &bottoms {
        let item = library.item(id).unwrap();
        assert_eq!(item.slot, "underwear_bottom", "{id}");
        assert!(item.bases.iter().any(|base| base == DEFAULT_BASE), "{id}");
    }
    assert!(
        !bottoms.iter().any(|(id, _)| id == "makehuman/cargo-pants"),
        "{bottoms:?}"
    );

    // A slider's drag is one undo step.
    let fat = workspace.base().unwrap().parameter("fat").unwrap().clone();
    workspace.set_parameter(&fat, 0.2, false);
    workspace.set_parameter(&fat, 0.4, false);
    workspace.set_parameter(&fat, 0.5, true);
    workspace.undo();
    assert_eq!(workspace.parameter("fat"), 0.0);
    assert_eq!(workspace.parameter("frame"), 0.0);
    assert_eq!(workspace.worn_in("underwear_top"), None);
    workspace.undo();
    assert_eq!(workspace.parameter("frame"), 1.0);
    assert!(workspace.worn_in("underwear_top").is_some());
    assert!(!workspace.can_undo());
    workspace.redo();
    assert_eq!(workspace.parameter("frame"), 0.0);
    assert_eq!(workspace.worn_in("underwear_top"), None);
    workspace.choose(&frame, 1.0);

    workspace.edit(|description| description.name = "Test Hero".to_owned());
    let path = workspace.save().unwrap();
    assert!(
        path.ends_with("characters/Test Hero.character.json")
            || path.ends_with("characters\\Test Hero.character.json"),
        "{}",
        path.display()
    );
    assert!(!workspace.dirty());
    let listed: Vec<(String, PathBuf)> = shelf::list(&folder)
        .into_iter()
        .map(|saved| (saved.name, saved.path))
        .collect();
    assert_eq!(listed, vec![("Test Hero".to_owned(), path.clone())]);
    // Saving puts it first in Recent.
    assert_eq!(recent::recent(&folder), vec![path.clone()]);
    let reopened = CharacterWorkspace::open_file(&folder, &path).unwrap();
    assert_eq!(reopened.description(), workspace.description());
    assert!(!reopened.dirty());
    // Reset is the body's default character, keeping the name, and undoable.
    workspace.reset();
    assert_eq!(
        workspace.worn_in("underwear_top"),
        None,
        "the default character is Male"
    );
    assert_eq!(
        workspace.worn_in("underwear_bottom").as_deref(),
        Some(BRIEFS)
    );
    assert_eq!(workspace.parameter("frame"), 0.0);
    assert_eq!(workspace.description().name, "Test Hero");
    workspace.undo();
    assert_eq!(workspace.parameter("frame"), 1.0);
    let _ = std::fs::remove_dir_all(&folder);
}

/// Every new character the studio offers (New's families and presets,
/// the start page's cards) is sound and on its family's body: the engine
/// ships one family, the human base, whose first choice is its default
/// character.
#[test]
fn every_new_character_choice_is_sound() {
    let folder = test_folder("workspace-new-choices");
    let workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let library = workspace.library();
    let choices = toolbar::new_choices(&library, &workspace);
    let headings: Vec<&str> = choices
        .iter()
        .filter(|(_, _, start)| start.is_none())
        .map(|(label, _, _)| label.as_str())
        .collect();
    assert_eq!(headings, ["Human"]);
    let starts: Vec<&Start> = choices.iter().filter_map(|(_, _, s)| s.as_ref()).collect();
    assert_eq!(
        starts,
        [&Start::shipped("male"), &Start::shipped("female")],
        "the family's character presets, its default character first"
    );
    assert_eq!(
        workspace.starting(starts[0]),
        tidy(
            applier::default_description(&library, DEFAULT_BASE),
            &library
        ),
        "the first is the default character"
    );

    for start in starts {
        let made = workspace.starting(start);
        assert!(
            made.validate(&library).is_empty(),
            "{start:?}: {:?}",
            made.validate(&library)
        );
        assert!(
            made.extra.is_empty(),
            "{start:?}: no preset block comes along"
        );
        if let Start::Preset(id) = start {
            assert_eq!(made.base, library.preset(id).unwrap().base, "{start:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_workspace_draws_every_category_on_both_body_types_without_a_gpu() {
    let folder = test_folder("workspace-ui");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let ctx = theme::test_context();
    let frame = workspace
        .base()
        .unwrap()
        .parameter("frame")
        .unwrap()
        .clone();
    for body_type in [1.0, 0.0] {
        workspace.choose(&frame, body_type);
        for level in selection::Level::ALL {
            workspace.selection.level = level;
            for category in Category::ALL {
                workspace.selection.category = category;
                for _ in 0..2 {
                    let _ = theme::run(&ctx, Default::default(), |ui| {
                        assert_eq!(workspace.ui(ui, None), None);
                    });
                }
            }
        }
    }
    // With `AXCENITY_SNAPSHOTS`, the whole workspace on the human body,
    // drawn for real, on the Body and Face pages (every page:
    // `shell_tests::every_page_is_drawn_with_its_controls`).
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        workspace.selection.level = selection::Level::Detailed;
        for category in [Category::Body, Category::Face] {
            workspace.pick_category(category);
            // Each snapshot makes its own GPU device; the editor has one.
            workspace.canvas = Canvas::default();
            let path = snapshots.join(format!("character-studio-{}.png", category_name(category)));
            crate::snapshot::snapshot_rendered(
                &path,
                eframe::egui::vec2(1440.0, 900.0),
                vec![Vec::new(); 3],
                |ui, render_state| {
                    workspace.ui(ui, Some(render_state));
                },
            );
        }
    }
    let _ = std::fs::remove_dir_all(&folder);
}

fn category_name(category: Category) -> String {
    format!("{category:?}").to_lowercase()
}

#[test]
fn the_views_look_at_the_character_from_where_they_say() {
    use axcenity_render::glam::Vec3;
    let camera = |preset: ViewPreset| preset.orbit().camera();
    // It faces -Z: the front view stands at -Z, the back at +Z.
    assert!(camera(ViewPreset::Front).position.z < -5.0);
    assert!(camera(ViewPreset::Back).position.z > 5.0);
    assert!(camera(ViewPreset::Side).position.x > 5.0);
    // Three-quarter: between the front and the side.
    let three_quarter = camera(ViewPreset::ThreeQuarter).position;
    assert!(
        three_quarter.x > 3.0 && three_quarter.z < -3.0,
        "{three_quarter}"
    );
    // The face view is close, at head height.
    let face = camera(ViewPreset::Face);
    assert!((face.position - face.look_at).length() < 4.0);
    assert!(face.look_at.y > 4.0);
    // Turning, zooming and panning stay in range.
    let mut orbit = ViewPreset::FullBody.orbit();
    orbit.turn(0.0, 10_000.0);
    assert!(orbit.pitch <= 80.0);
    orbit.zoom(1.0e6);
    assert!(orbit.distance >= 1.2);
    orbit.pan(1.0e6, 1.0e6);
    assert!(orbit.target.length() < Vec3::new(4.0, 6.0, 4.0).length() + 0.01);
}

/// Overhaul CO3 and CO12, refinement R2a: a character the workspace saves,
/// applied to a character in a game, is drawn from the same models as the
/// preview and puts every corner of every part in the same place, in the
/// same pose, to 0.2 mm. Both wear the identity live (ADR-0041): no
/// derived `MeshId` anywhere in either World.
#[test]
fn a_character_file_applied_in_a_game_matches_the_workspaces_preview() {
    use axcenity_render::glam::Vec3;
    let folder = test_folder("workspace-agrees");
    let mut workspace =
        CharacterWorkspace::open_new(&folder, &Start::shipped("body-athletic")).unwrap();
    assert_eq!(workspace.parameter("muscle"), 0.65, "the Athletic build");
    workspace.edit(|description| {
        description.name = "Agrees".to_owned();
        description.parameters.insert("fat".to_owned(), 0.6);
        description
            .parameters
            .insert("nose_scale_vert".to_owned(), 0.5);
    });
    let path = workspace.save().unwrap();

    let assets = Assets::new(&folder);
    let mut world = World::new();
    let character = axcenity_project::default_character(&mut world, Vector3::default());
    let service = world.workspace();
    world.set_parent(character, Some(service)).unwrap();
    let description = Description::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let applied = applier::apply_description(&mut world, &assets, character, &description).unwrap();
    assert!(applied.problems.is_empty(), "{:?}", applied.problems);
    let mut animators = axcenity_anim::Animators::new();
    for _ in 0..20 {
        workspace.step_by(1.0 / 60.0);
        animators.update(&mut world, &assets, 1.0 / 60.0);
    }

    // Each MeshPart's `MeshId` and its corners where its pose puts them,
    // in its file's space (metres), by the part's name.
    let corners = |world: &World,
                   assets: &Assets,
                   poses: &axcenity_scene::Poses,
                   body: InstanceId|
     -> BTreeMap<String, (String, Vec<Vec3>)> {
        let mut parts = vec![body];
        parts.extend(world.get(body).unwrap().children().iter().copied());
        parts
            .into_iter()
            .filter_map(|part| {
                let instance = world.get(part)?;
                let mesh = instance.property("MeshId")?.as_str()?.to_owned();
                assert!(
                    axcenity_assets::split_derived(&mesh).1.is_none(),
                    "{} draws a derived model: {mesh}",
                    instance.name()
                );
                let model = assets.model_blocking(&mesh).unwrap();
                let posed = poses.get(&part).expect("every part is posed");
                let unfit = model.rig.as_ref().unwrap().fit.inverse();
                let points = (0..model.primitives.len())
                    .flat_map(|index| posed.primitive(&model, index, 0).positions.clone())
                    .map(|corner| unfit.transform_point3(Vec3::from(corner)))
                    .collect();
                Some((instance.name().to_owned(), (mesh, points)))
            })
            .collect()
    };
    let preview_body = applier::body_of(&workspace.world, workspace.character.unwrap()).unwrap();
    let preview = corners(
        &workspace.world,
        &workspace.assets,
        workspace.poses(),
        preview_body,
    );
    let game = corners(&world, &assets, animators.poses(), applied.body);
    assert_eq!(
        preview.keys().collect::<Vec<_>>(),
        game.keys().collect::<Vec<_>>()
    );
    for (name, (mesh, points)) in &preview {
        let (game_mesh, game_points) = &game[name];
        assert_eq!(mesh, game_mesh, "{name}");
        assert_eq!(points.len(), game_points.len(), "{name}");
        let worst = points
            .iter()
            .zip(game_points)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0, f32::max);
        assert!(worst < 2e-4, "{name}: {worst} m apart");
    }
    // The identity is worn: the body is not its plain base.
    let plain = axcenity_anim::identity_key(&world, applied.body);
    assert!(plain.is_some(), "the game's body wears an identity");
    let _ = std::fs::remove_dir_all(&folder);
}

/// The preview as Character Studio draws it (its camera, stage lighting and
/// backdrop, the applied character idling), rendered offscreen for the
/// human body on both body types and dressed. With `AXCENITY_SNAPSHOTS`
/// naming a folder, each is saved there to look at.
#[test]
fn the_preview_draws_the_human_character() {
    use axcenity_render::TextureRenderer;
    let instance = axcenity_render::adapters::instance();
    let Some((device, queue)) =
        pollster::block_on(axcenity_render::adapters::adapter(&instance, None))
            .ok()
            .and_then(|adapter| {
                pollster::block_on(adapter.request_device(&Default::default())).ok()
            })
    else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let folder = test_folder("workspace-draws");
    let snapshots = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from);
    let (width, height) = (640, 720);
    let mut renderer = TextureRenderer::new(&device, &queue, width, height);
    let mut frames = Vec::new();
    for (name, start, outfit) in [
        ("human", Start::default(), None),
        ("human-female", Start::shipped("female"), None),
        (
            "human-dressed",
            Start::default(),
            Some("makehuman/male-casualsuit01"),
        ),
    ] {
        let mut workspace = CharacterWorkspace::open_new(&folder, &start).unwrap();
        if let Some(outfit) = outfit {
            workspace.wear("top", Some(outfit));
            assert_eq!(workspace.worn_in("top").as_deref(), Some(outfit));
        }
        for _ in 0..10 {
            workspace.step_by(1.0 / 30.0);
        }
        stage::apply(&mut workspace.world, workspace.lighting, workspace.backdrop);
        let mut list = axcenity_scene::build_draw_list_posed(
            &workspace.world,
            &workspace.assets,
            ViewPreset::FullBody.orbit().camera(),
            workspace.poses(),
            &mut axcenity_scene::TerrainCache::new(),
        );
        stage::backdrop(&mut list, workspace.backdrop);
        // Models load the first time they're drawn.
        renderer.render(&list).unwrap();
        renderer.render(&list).unwrap();
        let pixels = renderer.read_pixels();
        if let Some(snapshots) = &snapshots {
            crate::snapshot::save_png(
                &snapshots.join(format!("character-studio-{name}.png")),
                (width, height),
                &pixels,
            );
        }
        frames.push((name, pixels));
    }
    // The character covers the middle of the frame: the centre column
    // differs from the backdrop at its edge.
    for (name, pixels) in &frames {
        let at = |x: u32, y: u32| {
            let i = ((y * width + x) * 4) as usize;
            [pixels[i], pixels[i + 1], pixels[i + 2]]
        };
        let edge = at(8, height / 2);
        let covered = (height / 4..height * 3 / 4)
            .filter(|y| {
                let p = at(width / 2, *y);
                p.iter().zip(&edge).any(|(a, b)| a.abs_diff(*b) > 12)
            })
            .count();
        assert!(
            covered > (height / 2) as usize * 3 / 4,
            "{name}: the character is drawn ({covered})"
        );
    }
    let changed = frames[0]
        .1
        .chunks(4)
        .zip(frames[2].1.chunks(4))
        .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 8))
        .count();
    println!("dressed differs from undressed in {changed} pixels");
    assert!(
        changed > (width * height) as usize / 50,
        "the dressed character looks different"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// ADR-0048: the human base's Skin and Makeup page offers its skins as
/// one choice, and choosing one writes that skin's file on the body, the
/// female or the male one with the body type; undo puts the old one back.
#[test]
fn the_human_bases_skin_is_a_choice_on_the_skin_page() {
    let folder = test_folder("workspace-human-skin");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert_eq!(workspace.notice, None);
    workspace.apply(Intent::PickCategory(Category::SkinAndMakeup));
    let ctx = theme::test_context();
    let mut drawn = Vec::new();
    for _ in 0..3 {
        let output = theme::run(&ctx, Default::default(), |ui| {
            assert_eq!(workspace.ui(ui, None), None);
        });
        drawn = super::shell_tests::drawn_texts(&output);
    }
    for label in ["African", "Asian", "Caucasian", "Caucasian 2", "Bodysuit"] {
        assert!(
            drawn.iter().any(|text| text == label),
            "{label} in {drawn:?}"
        );
    }

    let attribute =
        axcenity_core::material_value_attribute("Skin", axcenity_core::MATERIAL_COLOUR_MAP);
    let skin = |workspace: &CharacterWorkspace| {
        use axcenity_gameplay::description as applier;
        let body = applier::body_of(&workspace.world, workspace.character.unwrap()).unwrap();
        workspace
            .world
            .get(body)
            .and_then(|body| body.attribute(&attribute))
            .and_then(axcenity_core::Value::as_str)
            .map(str::to_owned)
    };
    let file = |name: &str| Some(format!("assets/characters/human/skins/{name}.jpg"));
    assert_eq!(skin(&workspace), file("young_caucasian_male"));
    let choice = workspace.base().unwrap().parameter("skin").unwrap().clone();
    workspace.choose(&choice, 1.0);
    assert_eq!(skin(&workspace), file("young_asian_male"));
    let frame = workspace
        .base()
        .unwrap()
        .parameter("frame")
        .unwrap()
        .clone();
    workspace.choose(&frame, 1.0);
    assert_eq!(skin(&workspace), file("young_asian_female"));
    workspace.undo();
    workspace.undo();
    assert_eq!(skin(&workspace), file("young_caucasian_male"));
    // The file is in the studio's own library, copied with the base.
    let copied = folder.join("assets/characters/human/skins/young_asian_female.jpg");
    assert!(copied.is_file(), "{}", copied.display());
    // With `AXCENITY_SNAPSHOTS`, the page drawn for real on two skins.
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        for (value, name) in [(0.0, "human-skin-african"), (1.0, "human-skin-asian")] {
            workspace.choose(&choice, value);
            super::shell_tests::snap(&mut workspace, &snapshots, name, 8);
        }
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// ADR-0047 stage 5: the human base's hairstyles (MakeHuman's, converted
/// by `hair.py`) are on the Hair page, and one put on is a part on the
/// body that wears the body's identity, in the colour variant picked.
#[test]
fn the_human_bases_hairstyles_are_on_the_hair_page_and_can_be_worn() {
    let folder = test_folder("workspace-human-hair");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::PickCategory(Category::Hair));
    let ctx = theme::test_context();
    let mut drawn = Vec::new();
    for _ in 0..3 {
        let output = theme::run(&ctx, Default::default(), |ui| {
            assert_eq!(workspace.ui(ui, None), None);
        });
        drawn = super::shell_tests::drawn_texts(&output);
    }
    // Every one is a draft, and a page of nothing but drafts lists them.
    for label in ["Afro", "Long and straight", "Short and tousled"] {
        assert!(
            drawn.iter().any(|text| text.contains(label)),
            "{label} in {drawn:?}"
        );
    }
    assert_eq!(workspace.worn_in("hair"), None, "a new character is bald");

    workspace.wear("hair", Some("makehuman/hair-short02"));
    assert_eq!(workspace.notice, None);
    assert_eq!(
        workspace.worn_in("hair").as_deref(),
        Some("makehuman/hair-short02")
    );
    let worn = worn_models(&workspace);
    assert!(worn.iter().any(|name| name == "HairShort02"), "{worn:?}");
    // It keeps to the head through the body type and a face slider.
    let frame = workspace
        .base()
        .unwrap()
        .parameter("frame")
        .unwrap()
        .clone();
    workspace.choose(&frame, 1.0);
    assert_eq!(
        workspace.worn_in("hair").as_deref(),
        Some("makehuman/hair-short02")
    );
    let library = workspace.library();
    let resolved = axcenity_avatar::resolve(&library, workspace.description()).unwrap();
    assert_eq!(resolved.problems, Vec::new());
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        super::shell_tests::snap(&mut workspace, &snapshots, "human-hair", 8);
    }
    let _ = std::fs::remove_dir_all(&folder);
}
