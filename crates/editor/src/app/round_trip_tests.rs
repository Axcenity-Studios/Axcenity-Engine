//! Refinement R10 in the Game Editor: the round trip to Character Studio
//! and back (insert, edit, return, play), a described character shown in
//! edit mode (OPEN-21), the realistic body as the default game character
//! with both families in every project (OPEN-22), and the studio's keys
//! inside the editor.

use std::path::{Path, PathBuf};

use axcenity_avatar::{Description, LIBRARY_FOLDER, Library};
use axcenity_core::{ClassName, InstanceId, Value, World};
use axcenity_gameplay::description as applier;

use super::tests::{drawn_texts, inserted, play_for};
use super::*;
use crate::character_workspace::{Gesture, Intent};

const REALISTIC: &str = "axcenity/realistic";

/// A new Baseplate project in a folder of its own, and a studio folder.
fn new_project(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let parent =
        std::env::temp_dir().join(format!("axcenity-round-trip-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    let folder = axcenity_project::create_project_from(
        &parent,
        "Round Trip",
        axcenity_project::Template::Baseplate,
    )
    .unwrap();
    let studio = parent.join("studio");
    std::fs::create_dir_all(&studio).unwrap();
    (parent, folder, studio)
}

fn open(folder: &Path, studio: &Path) -> EditorApp {
    let mut app = EditorApp::new(Some(folder));
    assert!(app.project.is_some(), "the project opened");
    app.set_characters_folder(Some(studio.to_owned()));
    app
}

/// The description kept in `character` in `world`, read back.
fn kept(world: &World, character: InstanceId) -> Option<Description> {
    let held = applier::applied_instance(world, character)?;
    let data = world.get(held)?.property("Data")?.as_str()?;
    Description::from_json(data).ok()
}

fn mesh_id(world: &World, part: InstanceId) -> String {
    world
        .get(part)
        .and_then(|p| p.property("MeshId"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Every model `character` draws, loaded now (tests have no background
/// loading, but the posing step never waits).
fn load_models(app: &EditorApp, character: InstanceId) {
    let mut ids = vec![character];
    ids.extend(app.world.descendants(character));
    for id in ids {
        let mesh = mesh_id(&app.world, id);
        if !mesh.is_empty() {
            app.assets.model_blocking(&mesh);
        }
    }
}

fn frame(app: &mut EditorApp, ctx: &egui::Context, events: Vec<egui::Event>) -> Vec<String> {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1440.0, 900.0),
        )),
        events,
        ..Default::default()
    };
    let output = crate::theme::run(ctx, input, |ui| app.frame(ui, None));
    drawn_texts(&output).into_iter().map(|(t, _)| t).collect()
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

/// OPEN-22: Insert > Character in a new project is the realistic
/// character (decision 7), with both families in the project's library,
/// and it shows at once: applied as it's inserted, one undo step.
#[test]
fn insert_character_places_the_realistic_character_with_both_families() {
    let (parent, folder, studio) = new_project("insert");
    let mut app = open(&folder, &studio);
    let character = inserted(&mut app, Insertable::Character, None);
    assert_eq!(app.history.undo_label(), Some("Insert Character"));
    let description = kept(&app.world, character).expect("a described character");
    assert_eq!(description.base, REALISTIC);
    let body = applier::body_of(&app.world, character).unwrap();
    assert!(
        mesh_id(&app.world, body).starts_with(LIBRARY_FOLDER),
        "the realistic body is drawn: {}",
        mesh_id(&app.world, body)
    );
    let (library, _, _) = Library::read(&folder.join(LIBRARY_FOLDER));
    assert!(library.base(REALISTIC).is_some());
    assert!(library.base(axcenity_avatar::V1_BASE).is_some());
    // Undo takes the whole character away.
    app.undo();
    assert!(app.world.get(character).and_then(|c| c.parent()).is_none());
    drop(app);
    let _ = std::fs::remove_dir_all(parent);
}

/// OPEN-22: an older project (a stylized library and a stylized starter
/// character) keeps its players' prefab and its characters, and gains the
/// realistic family when a character is inserted.
#[test]
fn an_older_project_keeps_its_characters_and_gains_the_realistic_family() {
    let parent =
        std::env::temp_dir().join(format!("axcenity-round-trip-older-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    let folder = super::tests::copy_of_example("hello-character", "round-trip-older");
    axcenity_project::add_default_humanoid(&folder).unwrap();
    let prefab = folder.join(axcenity_project::DEFAULT_CHARACTER_PREFAB_PATH);
    let old_prefab = std::fs::read_to_string(&prefab).unwrap();
    let mut app = open(&folder, &parent);
    let old = app
        .world
        .find_first_child(app.world.workspace(), "Character")
        .unwrap();
    let old_tree = super::tests::describe(&app.world, old);
    let new = inserted(&mut app, Insertable::Character, None);
    assert_eq!(kept(&app.world, new).unwrap().base, REALISTIC);
    assert_eq!(
        super::tests::describe(&app.world, old),
        old_tree,
        "the old one is kept"
    );
    assert_eq!(std::fs::read_to_string(&prefab).unwrap(), old_prefab);
    let (library, _, _) = Library::read(&folder.join(LIBRARY_FOLDER));
    assert!(library.base(REALISTIC).is_some());
    assert!(library.base(axcenity_avatar::V1_BASE).is_some());
    drop(app);
    let _ = std::fs::remove_dir_all(&folder);
    let _ = std::fs::remove_dir_all(parent);
}

/// R10's Done-when: insert, edit in Character Studio, return, play. The
/// edit comes back as one undo step, and the played character wears it.
#[test]
fn insert_edit_in_character_studio_return_and_play_wears_the_edit() {
    let (parent, folder, studio) = new_project("insert-edit-play");
    let mut app = open(&folder, &studio);
    let character = inserted(&mut app, Insertable::Character, None);
    let steps = app.history.undo_labels().len();

    // The Explorer offers it for the character, its body and its parts.
    let body = applier::body_of(&app.world, character).unwrap();
    for id in [character, body] {
        assert_eq!(
            crate::character_workspace::game::editable_character(&app.world, id),
            Some(character)
        );
    }
    app.edit_in_character_studio(character).unwrap();
    let ctx = crate::theme::test_context();
    let drawn = frame(&mut app, &ctx, Vec::new());
    for text in ["Done", "Cancel"] {
        assert!(drawn.iter().any(|t| t.contains(text)), "{text}: {drawn:?}");
    }
    assert!(!drawn.iter().any(|t| t == "Debug"), "no Game Editor menus");
    let workspace = app.character_workspace.as_mut().expect("the studio opened");
    assert_eq!(workspace.game().map(|g| g.character), Some(character));
    assert_eq!(workspace.description().base, REALISTIC);
    assert!(!workspace.dirty(), "nothing changed yet");
    workspace.apply(Intent::SetParameter {
        id: "fat".into(),
        value: 0.8,
        gesture: Gesture::Commit,
    });
    let edited = workspace.description().clone();
    assert!(app.title().contains("Character Studio"), "{}", app.title());

    // Done: back in the Game Editor, the edit applied as one step.
    app.finish_character_round_trip(true);
    assert!(app.character_workspace.is_none());
    let now = kept(&app.world, character).unwrap();
    assert_eq!(now.parameters.get("fat"), Some(&0.8));
    assert_eq!(now.base, edited.base);
    assert_eq!(app.history.undo_labels().len(), steps + 1);
    assert_eq!(
        app.history.undo_label(),
        Some("Edit Character in Character Studio")
    );
    app.undo();
    assert_eq!(
        kept(&app.world, character).unwrap().parameters.get("fat"),
        None
    );
    app.redo();
    assert_eq!(
        kept(&app.world, character).unwrap().parameters.get("fat"),
        Some(&0.8)
    );

    // Play: the game's character wears the edit, its identity live.
    load_models(&app, character);
    app.play();
    assert!(app.playing());
    play_for(&mut app, 0.2);
    let game = shown(&app.world, &app.play);
    assert_eq!(
        kept(game, character).unwrap().parameters.get("fat"),
        Some(&0.8)
    );
    let library = applier::library(&app.assets);
    let resolved = axcenity_avatar::resolve(&library, &now).unwrap();
    let wanted = axcenity_assets::Identity::key(&resolved.identity);
    assert_eq!(axcenity_anim::identity_key(game, body), Some(wanted));
    let posed = app.play.as_ref().unwrap().poses().get(&body).cloned();
    assert!(
        posed.is_some_and(|p| p.is_morphed()),
        "the body is posed with its shapes"
    );
    app.stop();
    drop(app);
    let _ = std::fs::remove_dir_all(parent);
}

/// Cancel goes back with the game's character as it was and no step.
#[test]
fn cancel_leaves_the_game_character_as_it_was() {
    let (parent, folder, studio) = new_project("cancel");
    let mut app = open(&folder, &studio);
    let character = inserted(&mut app, Insertable::Character, None);
    let before = kept(&app.world, character).unwrap();
    let steps = app.history.undo_labels().len();
    app.edit_in_character_studio(character).unwrap();
    app.character_workspace
        .as_mut()
        .unwrap()
        .apply(Intent::SetParameter {
            id: "fat".into(),
            value: 0.8,
            gesture: Gesture::Commit,
        });
    app.finish_character_round_trip(false);
    assert!(app.character_workspace.is_none());
    assert_eq!(kept(&app.world, character).unwrap(), before);
    assert_eq!(app.history.undo_labels().len(), steps);
    // The studio's own recovery file was never written or taken.
    assert!(
        !studio
            .join(crate::character_workspace::autosave::RECOVERY_FILE)
            .exists()
    );
    drop(app);
    let _ = std::fs::remove_dir_all(parent);
}

/// A character made before descriptions (the stylized default humanoid)
/// opens on the stylized family, and an imported character isn't offered.
#[test]
fn a_character_without_a_description_opens_on_the_stylized_family() {
    let (parent, folder, studio) = new_project("stylized");
    let mut app = open(&folder, &studio);
    let character = axcenity_project::default_character(&mut app.world, Default::default());
    let workspace = app.world.workspace();
    app.world.set_parent(character, Some(workspace)).unwrap();
    assert!(kept(&app.world, character).is_none());
    assert_eq!(
        crate::character_workspace::game::editable_character(&app.world, character),
        Some(character)
    );
    app.edit_in_character_studio(character).unwrap();
    let workspace = app.character_workspace.as_ref().unwrap();
    assert_eq!(workspace.description().base, axcenity_avatar::V1_BASE);
    assert_eq!(workspace.description().name, "Character");
    app.finish_character_round_trip(false);

    // A Model whose body is a model of its own isn't a Character Studio
    // character: Done would put the engine's body in its place.
    let mut world = World::new();
    let model = world.create(ClassName::Model).unwrap();
    let block = world.create(ClassName::MeshPart).unwrap();
    world.set_name(block, "Body").unwrap();
    world
        .set_property(
            block,
            "MeshId",
            Value::String("assets/models/Robot.glb".into()),
        )
        .unwrap();
    world.set_parent(block, Some(model)).unwrap();
    let humanoid = world.create(ClassName::Humanoid).unwrap();
    world.set_parent(humanoid, Some(model)).unwrap();
    assert_eq!(
        crate::character_workspace::game::editable_character(&world, model),
        None
    );
    drop(app);
    let _ = std::fs::remove_dir_all(parent);
}

/// OPEN-21: a saved described character shows its identity, items and
/// materials in edit mode, worn live and posed at rest, without Play and
/// without making the world unsaved.
#[test]
fn a_saved_described_character_shows_its_identity_in_edit_mode() {
    let (parent, folder, studio) = new_project("edit-mode");
    let mut app = open(&folder, &studio);
    let character = inserted(&mut app, Insertable::Character, None);
    app.edit_in_character_studio(character).unwrap();
    app.character_workspace
        .as_mut()
        .unwrap()
        .apply(Intent::SetParameter {
            id: "fat".into(),
            value: 0.8,
            gesture: Gesture::Commit,
        });
    app.finish_character_round_trip(true);
    assert!(app.save());
    drop(app);

    // Opened again: a new run, so nothing is registered yet.
    let mut app = open(&folder, &studio);
    let character = app
        .world
        .find_first_child(app.world.workspace(), "Character")
        .unwrap();
    let body = applier::body_of(&app.world, character).unwrap();
    let key = axcenity_anim::identity_key(&app.world, body).expect("saved with its identity");
    assert!(
        app.assets.identity(key).is_none(),
        "not worn before a frame"
    );
    let parts: Vec<InstanceId> = app
        .world
        .get(body)
        .unwrap()
        .children()
        .iter()
        .copied()
        .filter(|id| {
            app.world
                .get(*id)
                .is_some_and(|p| p.attribute(applier::DESCRIPTION_PART).is_some())
        })
        .collect();
    assert!(!parts.is_empty(), "its parts were saved");
    load_models(&app, character);
    app.edit_characters_frame();
    assert!(app.assets.identity(key).is_some(), "the identity is worn");
    let poses = app.edit_characters.poses();
    let posed = poses.get(&body).expect("the body is posed in edit mode");
    assert!(posed.is_morphed(), "with its identity's shapes");
    for part in &parts {
        assert!(poses.contains_key(part), "a part follows the body");
    }
    assert!(!app.unsaved(), "showing it changed nothing to save");
    assert!(!app.playing());
    // Nothing plays: a second frame poses the same.
    let version = posed.version;
    app.edit_characters_frame();
    assert_eq!(
        app.edit_characters.poses().get(&body).unwrap().version,
        version
    );
    drop(app);
    let _ = std::fs::remove_dir_all(parent);
}

/// OPEN-21 across opens in one run: opening the same project again (File >
/// Open, Open Recent, close and reopen) gives it new `Assets` but the same
/// Instance IDs, so the edit-mode state of the last open must not count
/// its description as already worn.
#[test]
fn a_described_character_shows_its_identity_after_reopening_in_the_same_run() {
    let (parent, folder, studio) = new_project("reopen");
    let mut app = open(&folder, &studio);
    let character = inserted(&mut app, Insertable::Character, None);
    app.edit_in_character_studio(character).unwrap();
    app.character_workspace
        .as_mut()
        .unwrap()
        .apply(Intent::SetParameter {
            id: "fat".into(),
            value: 0.8,
            gesture: Gesture::Commit,
        });
    app.finish_character_round_trip(true);
    assert!(app.save());
    drop(app);

    let mut app = open(&folder, &studio);
    let body_and_key = |app: &EditorApp| {
        let character = app
            .world
            .find_first_child(app.world.workspace(), "Character")
            .unwrap();
        let body = applier::body_of(&app.world, character).unwrap();
        let key = axcenity_anim::identity_key(&app.world, body).unwrap();
        (character, body, key)
    };
    let (character, _, key) = body_and_key(&app);
    load_models(&app, character);
    app.edit_characters_frame();
    assert!(app.assets.identity(key).is_some(), "first open: worn");

    // The same project opened again in this run.
    app.open(&folder);
    let (character, body, key) = body_and_key(&app);
    assert!(
        app.assets.identity(key).is_none(),
        "new Assets: not worn yet"
    );
    load_models(&app, character);
    app.edit_characters_frame();
    assert!(
        app.assets.identity(key).is_some(),
        "second open in the same run: the identity is worn in the new Assets"
    );
    let posed = app.edit_characters.poses().get(&body);
    assert!(
        posed.is_some_and(|pose| pose.is_morphed()),
        "and the body is posed with its shapes"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(parent);
}

/// Save and open polish: inside the editor, Ctrl+Z, Ctrl+Shift+Z and
/// Ctrl+S reach Character Studio (the Game Editor's own shortcuts used to
/// take them first), and the Game Editor's world isn't undone or saved
/// under it.
#[test]
fn the_studios_keys_reach_it_inside_the_editor() {
    let (parent, folder, studio) = new_project("keys");
    let mut app = open(&folder, &studio);
    let character = inserted(&mut app, Insertable::Character, None);
    let steps = app.history.undo_labels().len();
    app.edit_in_character_studio(character).unwrap();
    let ctx = crate::theme::test_context();
    frame(&mut app, &ctx, Vec::new());
    app.character_workspace
        .as_mut()
        .unwrap()
        .apply(Intent::SetParameter {
            id: "fat".into(),
            value: 0.5,
            gesture: Gesture::Commit,
        });
    frame(&mut app, &ctx, key(egui::Key::Z, egui::Modifiers::COMMAND));
    let workspace = app.character_workspace.as_ref().unwrap();
    assert_eq!(
        workspace.description().parameters.get("fat"),
        None,
        "undone"
    );
    assert_eq!(
        app.history.undo_labels().len(),
        steps,
        "the world's undo untouched"
    );
    frame(
        &mut app,
        &ctx,
        key(
            egui::Key::Z,
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        ),
    );
    let workspace = app.character_workspace.as_ref().unwrap();
    assert_eq!(workspace.description().parameters.get("fat"), Some(&0.5));
    frame(&mut app, &ctx, key(egui::Key::S, egui::Modifiers::COMMAND));
    let workspace = app.character_workspace.as_ref().unwrap();
    assert!(!workspace.dirty(), "Ctrl+S saved the studio's copy");
    assert!(app.unsaved(), "and not the game's world");
    drop(app);
    let _ = std::fs::remove_dir_all(parent);
}

/// OPEN-21, drawn: the saved described character of the test above,
/// rendered offscreen from the front as the editor drew it before its
/// edit-mode step (base meshes, parts unplaced), as edit mode draws it
/// now, and in Play. Edit mode must differ visibly from the old drawing.
/// With `AXCENITY_SNAPSHOTS` naming a folder, the three are saved there
/// (`round-trip-edit-before.png`, `-edit-now.png`, `-play.png`).
#[test]
fn edit_mode_draws_the_described_character() {
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
    let (parent, folder, studio) = new_project("edit-mode-drawn");
    let mut app = open(&folder, &studio);
    let character = inserted(&mut app, Insertable::Character, None);
    app.edit_in_character_studio(character).unwrap();
    let workspace = app.character_workspace.as_mut().unwrap();
    for (id, value) in [("fat", 0.8), ("muscle", 0.3)] {
        workspace.apply(Intent::SetParameter {
            id: id.into(),
            value,
            gesture: Gesture::Commit,
        });
    }
    app.finish_character_round_trip(true);
    assert!(app.save());
    drop(app);

    let mut app = open(&folder, &studio);
    let character = app
        .world
        .find_first_child(app.world.workspace(), "Character")
        .unwrap();
    load_models(&app, character);
    let root = app
        .world
        .find_first_child(character, "HumanoidRootPart")
        .and_then(|root| app.world.get(root)?.property("Position")?.as_vector3())
        .unwrap();
    let camera = crate::character_workspace::Orbit {
        yaw: 20.0,
        pitch: 4.0,
        distance: 11.0,
        target: axcenity_render::glam::Vec3::new(root.x, root.y, root.z),
    }
    .camera();
    let (width, height) = (480, 640);
    let mut renderer = TextureRenderer::new(&device, &queue, width, height);
    let assets = app.assets.clone();
    let mut draw = |world: &World, poses: &axcenity_scene::Poses| {
        let list = axcenity_scene::build_draw_list_posed(
            world,
            &assets,
            camera,
            poses,
            &mut axcenity_scene::TerrainCache::new(),
        );
        // Models load the first time they're drawn.
        renderer.render(&list).unwrap();
        renderer.render(&list).unwrap();
        renderer.read_pixels()
    };
    let before = draw(&app.world, &axcenity_scene::Poses::new());
    app.edit_characters_frame();
    let now = draw(&app.world, app.edit_characters.poses());
    app.play();
    play_for(&mut app, 0.5);
    let play = {
        let poses = app.play.as_ref().unwrap().poses().clone();
        draw(shown(&app.world, &app.play), &poses)
    };
    app.stop();
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        for (name, pixels) in [("before", &before), ("now", &now), ("play", &play)] {
            let file = match name {
                "play" => "round-trip-play.png".to_owned(),
                _ => format!("round-trip-edit-{name}.png"),
            };
            crate::snapshot::save_png(&snapshots.join(file), (width, height), pixels);
        }
    }
    let differing = |a: &[u8], b: &[u8]| {
        a.chunks(4)
            .zip(b.chunks(4))
            .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 15))
            .count()
    };
    let total = (width * height) as usize;
    let changed = differing(&before, &now);
    eprintln!(
        "edit mode against the old drawing: {changed} of {total} pixels ({:.1}%); against Play: {:.1}%",
        100.0 * changed as f64 / total as f64,
        100.0 * differing(&now, &play) as f64 / total as f64
    );
    assert!(
        changed > total / 100,
        "edit mode draws the description: {changed} pixels changed"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(parent);
}
