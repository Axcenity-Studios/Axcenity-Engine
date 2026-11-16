//! Character Studio overhaul CO3: a character description applied in the
//! player (`gameplay`'s applier, `docs/development/character-studio/
//! architecture.md`). The character wears the description's parts, its
//! meshes have the identity baked in, its eyes move with the frame's
//! bones, its body sliders are set and its material values modulate.

use axcenity_assets::Model;
use axcenity_avatar::{Colour, Description, MaterialValue, WornItem};
use axcenity_gameplay::description::{
    DESCRIPTION_PART, DescriptionSync, applied_instance, apply_description, description_of, library,
};
use axcenity_render::glam::Vec3;

use super::*;

const PARTS: &str = "assets/characters/humanoid/HumanoidParts.glb";

/// A new Baseplate project with the character library, its player spawned.
fn game(name: &str) -> Game {
    let mut game = Game::new(name);
    axcenity_project::add_character_library(&game.folder).unwrap();
    game.frames(5, MoveInput::default());
    game
}

fn described(parameters: &[(&str, f32)], items: &[&str]) -> Description {
    let mut description = Description::new("Ada", "axcenity/stylized");
    for (id, value) in parameters {
        description.parameters.insert((*id).to_owned(), *value);
    }
    description.items = items.iter().map(|id| WornItem::new(*id)).collect();
    description
}

const OUTFIT: [&str; 5] = [
    "axcenity/hair-swept",
    "axcenity/tee",
    "axcenity/cropped-jacket",
    "axcenity/cargo-pants",
    "axcenity/sneakers",
];

fn mesh_id(game: &Game, part: InstanceId) -> String {
    game.world
        .get(part)
        .unwrap()
        .property("MeshId")
        .and_then(Value::as_str)
        .unwrap()
        .to_owned()
}

/// The MeshParts inside `body`, by name.
fn parts(game: &Game, body: InstanceId) -> Vec<String> {
    let mut names: Vec<String> = game
        .world
        .get(body)
        .unwrap()
        .children()
        .iter()
        .filter_map(|child| game.world.get(*child))
        .filter(|child| child.class() == ClassName::MeshPart)
        .map(|child| child.name().to_owned())
        .collect();
    names.sort();
    names
}

/// A model's corners in its file's space (metres), all primitives.
fn corners(model: &Model, positions: impl Fn(usize) -> Vec<[f32; 3]>) -> Vec<Vec3> {
    let unfit = model.rig.as_ref().unwrap().fit.inverse();
    (0..model.primitives.len())
        .flat_map(positions)
        .map(|corner| unfit.transform_point3(Vec3::from(corner)))
        .collect()
}

fn rest_corners(model: &Model) -> Vec<Vec3> {
    corners(model, |index| model.primitives[index].positions.clone())
}

fn middle(points: &[Vec3]) -> Vec3 {
    points.iter().copied().sum::<Vec3>() / points.len() as f32
}

#[test]
fn a_description_applied_in_the_player_wears_its_parts() {
    let mut game = game("description-parts");
    let character = game.character();
    let body = game.world.find_first_child(character, "Body").unwrap();
    let before = game.vector(body, "Position");
    let applied = apply_description(
        &mut game.world,
        &game.assets,
        character,
        &described(&[], &OUTFIT),
    )
    .expect("the library has the stylized base");
    assert!(applied.problems.is_empty(), "{:?}", applied.problems);
    assert_eq!(applied.body, body);
    assert_eq!(
        parts(&game, body),
        [
            "Brows",
            "Eyes",
            "Hair_Swept",
            "Jacket_Cropped",
            "Lashes",
            "Pants_Cargo",
            "Shirt_Tee",
            "Shoes_Sneakers",
            "Teeth"
        ]
    );
    // The body is the library's, with the identity baked in: the clothes
    // hide regions of it, so it's a derived model.
    let body_mesh = mesh_id(&game, body);
    assert!(
        body_mesh.starts_with(&format!("{PARTS}#AxcenityHumanoid/Body#")),
        "{body_mesh}"
    );
    // Its feet stayed where they were: the two files share a skeleton.
    game.frames(30, MoveInput::default());
    let after = game.vector(body, "Position");
    assert!((after.y - before.y).abs() < 0.2, "{before:?} → {after:?}");
    for part in &applied.parts {
        assert!(game.animators.poses().contains_key(part), "posed");
    }
    game.no_errors();

    // Another description: parts no longer worn go, new ones come, the
    // ones kept are the same Instances, and what the creator put inside
    // the body is left alone.
    let hat = game.world.create(ClassName::MeshPart).unwrap();
    game.world.set_name(hat, "MyHat").unwrap();
    game.world.set_parent(hat, Some(body)).unwrap();
    let tee = game.world.find_first_child(body, "Shirt_Tee").unwrap();
    let again = apply_description(
        &mut game.world,
        &game.assets,
        character,
        &described(
            &[],
            &["axcenity/hair-bob", "axcenity/tee", "axcenity/briefs"],
        ),
    )
    .unwrap();
    assert_eq!(
        parts(&game, body),
        [
            "Brows",
            "Eyes",
            "Hair_Bob",
            "Lashes",
            "MyHat",
            "Shirt_Tee",
            "Teeth",
            "Underwear_Briefs"
        ]
    );
    assert!(again.parts.contains(&tee), "the T-shirt is the same part");
    let marker = |part| {
        game.world
            .get(part)
            .unwrap()
            .attribute(DESCRIPTION_PART)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    assert_eq!(marker(tee).as_deref(), Some("axcenity/tee"));
    assert_eq!(marker(hat), None);

    // What's applied is kept in the Humanoid, as the description.
    let kept = applied_instance(&game.world, character).expect("kept");
    assert_eq!(again.description, kept);
    let read = description_of(&game.world, kept, &library(&game.assets)).unwrap();
    assert_eq!(read.items.len(), 3);
    assert_eq!(read.base, "axcenity/stylized");
}

#[test]
fn applied_parts_have_the_bodys_identity_baked_in_and_follow_its_live_shapes() {
    let mut game = game("description-shapes");
    let character = game.character();
    let body = game.world.find_first_child(character, "Body").unwrap();
    let plain = described(&[], &["axcenity/cropped-jacket"]);
    apply_description(&mut game.world, &game.assets, character, &plain).unwrap();
    let jacket = game.world.find_first_child(body, "Jacket_Cropped").unwrap();
    let slim_jacket = game.assets.model_blocking(&mesh_id(&game, jacket)).unwrap();
    let slim_body = game.assets.model_blocking(&mesh_id(&game, body)).unwrap();

    let heavy = described(&[("build", 1.0)], &["axcenity/cropped-jacket"]);
    apply_description(&mut game.world, &game.assets, character, &heavy).unwrap();
    let heavy_jacket = game.assets.model_blocking(&mesh_id(&game, jacket)).unwrap();
    let heavy_body = game.assets.model_blocking(&mesh_id(&game, body)).unwrap();
    assert_ne!(heavy_jacket.key(), slim_jacket.key());

    // The heavy build is in the rest meshes themselves, the jacket's as
    // well as the body's: exactly the base's `Build_Heavy` shape.
    let base = game
        .assets
        .model_blocking(&format!("{PARTS}#AxcenityHumanoid/Jacket_Cropped"))
        .unwrap();
    let target = base
        .morph_targets
        .iter()
        .position(|target| target.name == "Build_Heavy")
        .expect("the jacket carries the build");
    let mut leaned = base.rest_posed();
    let mut weights = vec![0.0; base.morph_targets.len()];
    weights[target] = 1.0;
    leaned.set_morph_weights(&weights);
    let expected = corners(&base, |index| {
        leaned.primitive(&base, index, 0).positions.clone()
    });
    let baked = rest_corners(&heavy_jacket);
    assert_eq!(baked.len(), expected.len());
    let worst = baked
        .iter()
        .zip(&expected)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0, f32::max);
    assert!(worst < 1e-4, "{worst} m from the base's shape");
    let grew = middle(&rest_corners(&heavy_body)).distance(middle(&rest_corners(&slim_body)));
    assert!(grew > 1e-4, "the body is heavier: {grew}");

    // Identity shapes are gone from the derived models (nothing morphs
    // them each frame); the face's expression shapes are still live.
    let names = |model: &Model| -> Vec<String> {
        model
            .morph_targets
            .iter()
            .map(|target| target.name.clone())
            .collect()
    };
    assert!(!names(&heavy_body).iter().any(|name| name == "Build_Heavy"));
    assert!(
        !names(&heavy_body)
            .iter()
            .any(|name| name.starts_with("Mask_"))
    );
    assert!(
        !names(&heavy_jacket)
            .iter()
            .any(|name| name == "Build_Heavy")
    );
    let blink = names(&heavy_body)
        .iter()
        .position(|name| name == "Blink_Left")
        .expect("expressions stay live");

    // A live shape on the body reaches the parts that have it: the lashes
    // close with the lid.
    let face = game.world.create(ClassName::FaceControls).unwrap();
    game.world.set_parent(face, Some(body)).unwrap();
    for name in ["BlinkEnabled", "ProceduralEnabled"] {
        game.world
            .set_property(face, name, Value::Bool(false))
            .unwrap();
    }
    game.world
        .set_property(face, "Blink_Left", Value::Number(1.0))
        .unwrap();
    game.frames(20, MoveInput::default());
    let body_pose = game.animators.poses()[&body].clone();
    assert!(
        body_pose.morph_weights()[blink] > 0.9,
        "the body blinks: {:?}",
        body_pose.morph_weights()[blink]
    );
    let lashes = game.world.find_first_child(body, "Lashes").unwrap();
    let lash_model = game.assets.model_blocking(&mesh_id(&game, lashes)).unwrap();
    let lash_blink = names(&lash_model)
        .iter()
        .position(|name| name == "Blink_Left")
        .expect("the lashes carry the blink");
    let lash_pose = game.animators.poses()[&lashes].clone();
    assert_eq!(
        lash_pose.morph_weights()[lash_blink],
        body_pose.morph_weights()[blink],
        "the lashes blink with the lid"
    );
    game.no_errors();
}

#[test]
fn the_frames_bone_offsets_move_the_skeleton_and_carry_the_eyes() {
    let mut game = game("description-frame");
    let character = game.character();
    let body = game.world.find_first_child(character, "Body").unwrap();
    let manifest = library(&game.assets);
    let frame = manifest
        .base("axcenity/stylized")
        .unwrap()
        .parameter("frame")
        .unwrap()
        .clone();
    let offset_of = |bone: &str| {
        let driver = frame
            .bones
            .iter()
            .find(|driver| driver.bone == bone)
            .unwrap();
        Vec3::from(driver.offset.unwrap_or_default())
    };
    let eye_scale = frame
        .bones
        .iter()
        .find(|driver| driver.bone == "LeftEye")
        .and_then(|driver| driver.scale)
        .expect("the feminine eyes are another size");

    let measure = |game: &mut Game, frame: f32| {
        let description = described(&[("frame", frame)], &[]);
        apply_description(&mut game.world, &game.assets, character, &description).unwrap();
        game.frames(3, MoveInput::default());
        let eyes = game.world.find_first_child(body, "Eyes").unwrap();
        let body_model = game.assets.model_blocking(&mesh_id(game, body)).unwrap();
        let eyes_model = game.assets.model_blocking(&mesh_id(game, eyes)).unwrap();
        // Where the rest pose puts each bone once the identity is added.
        let rig = body_model.rig.clone().unwrap();
        let mut pose = rig.rest_pose();
        body_model.settle(&mut pose);
        let world = rig.world(&pose);
        let bone = |name: &str| world[rig.bone(name).unwrap()].transform_point3(Vec3::ZERO);
        // The eyes as they're posed in the game, in their file's space.
        let posed = game.animators.poses()[&eyes].clone();
        let unfit = eyes_model.rig.as_ref().unwrap().fit.inverse();
        let posed_bones: Vec<Vec3> = posed
            .bones()
            .iter()
            .map(|bone| unfit.transform_point3(bone.position))
            .collect();
        let eye_rig = eyes_model.rig.clone().unwrap();
        let posed_left = posed_bones[eye_rig.bone("LeftEye").unwrap()];
        let posed_head = posed_bones[eye_rig.bone("Head").unwrap()];
        (
            bone("Hips"),
            bone("LeftHand"),
            bone("LeftEye"),
            rest_corners(&eyes_model),
            rest_corners(&body_model),
            posed_left - posed_head,
            bone("LeftEye") - bone("Head"),
        )
    };
    let (m_hips, m_hand, m_eye, m_eyes, m_body, _, _) = measure(&mut game, 0.0);
    let (f_hips, f_hand, f_eye, f_eyes, f_body, posed_eye, rest_eye) = measure(&mut game, 1.0);
    // The skeleton's joints are the feminine frame's.
    for (name, masculine, feminine) in [
        ("Hips", m_hips, f_hips),
        ("LeftHand", m_hand, f_hand),
        ("LeftEye", m_eye, f_eye),
    ] {
        let moved = feminine - masculine;
        let wanted = offset_of(name);
        assert!(moved.distance(wanted) < 1e-4, "{name}: {moved} vs {wanted}");
    }
    // The body's surface is the `Frame_Feminine` shape and nothing more:
    // moving its joints didn't drag the skin a second time. Its hands are
    // where the shape put them (the hand bone's offset is centimetres).
    let far = m_body
        .iter()
        .zip(&f_body)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0, f32::max);
    assert!(
        far > 0.01 && far < 0.15,
        "the frame moves the skin by {far} m"
    );
    // The eyeballs have no frame shape: they went with their bones, and
    // grew about them.
    let left =
        |corners: &[Vec3]| -> Vec<Vec3> { corners.iter().copied().filter(|c| c.x > 0.0).collect() };
    let (m_left, f_left) = (left(&m_eyes), left(&f_eyes));
    let moved = middle(&f_left) - middle(&m_left);
    assert!(
        moved.distance(offset_of("LeftEye")) < 2e-3,
        "the left eyeball moved by {moved}, its bone by {}",
        offset_of("LeftEye")
    );
    let radius = |corners: &[Vec3]| {
        let centre = middle(corners);
        corners.iter().map(|c| c.distance(centre)).sum::<f32>() / corners.len() as f32
    };
    let grew = radius(&f_left) / radius(&m_left);
    assert!((grew - eye_scale).abs() < 0.01, "{grew} vs {eye_scale}");
    // And posed by the game's Animator, the eye bone is where the settled
    // skeleton has it, relative to the head (the idle clip turns the
    // head a little, so compare lengths).
    assert!(
        (posed_eye.length() - rest_eye.length()).abs() < 1e-3,
        "{posed_eye} vs {rest_eye}"
    );
    game.no_errors();
}

/// A character nothing animates (no state machine, or no Animator at
/// all until the applier adds one) still leads its parts: they're placed
/// on the body at rest, not left where they were made.
#[test]
fn an_applied_character_with_nothing_playing_still_wears_its_parts() {
    let mut game = game("description-still");
    let character = game.character();
    let body = game.world.find_first_child(character, "Body").unwrap();
    let animator = game.world.find_first_child(body, "Animator").unwrap();
    game.world.destroy(animator).unwrap();
    let applied = apply_description(
        &mut game.world,
        &game.assets,
        character,
        &described(&[("frame", 1.0)], &OUTFIT),
    )
    .unwrap();
    let animator = game
        .world
        .find_first_child(body, "Animator")
        .expect("the applier gives the body an Animator to lead its parts");
    let machine = game.world.get(animator).unwrap().property("StateMachine");
    assert_eq!(machine.and_then(Value::as_str), Some(""));
    game.frames(3, MoveInput::default());
    for part in &applied.parts {
        assert!(
            game.animators.poses().contains_key(part),
            "{} is posed",
            game.world.get(*part).unwrap().name()
        );
    }
    // Each part sits around the body, not at the origin in a default box.
    let tee = game.world.find_first_child(body, "Shirt_Tee").unwrap();
    let (tee_at, body_at) = (game.vector(tee, "Position"), game.vector(body, "Position"));
    assert!(
        (tee_at.x - body_at.x).abs() < 1.0 && (tee_at.z - body_at.z).abs() < 1.0,
        "{tee_at:?} vs {body_at:?}"
    );
    assert!(
        tee_at.y > body_at.y,
        "the T-shirt is above the body's middle"
    );
    game.no_errors();
}

#[test]
fn a_descriptions_proportions_reach_the_body_sliders() {
    let mut game = game("description-sliders");
    let character = game.character();
    let body = game.world.find_first_child(character, "Body").unwrap();
    let height = |game: &Game| {
        game.world
            .get(body)
            .unwrap()
            .attribute("BodyHeight")
            .and_then(Value::as_number)
    };
    apply_description(
        &mut game.world,
        &game.assets,
        character,
        &described(&[], &[]),
    )
    .unwrap();
    assert_eq!(height(&game), None, "a slider at 1 isn't written");
    let tall = described(&[("height", 1.2), ("head_size", 0.9)], &[]);
    apply_description(&mut game.world, &game.assets, character, &tall).unwrap();
    assert!((height(&game).unwrap() - 1.2).abs() < 1e-6);
    assert!(axcenity_anim::BodyShape::read(&game.world, body).is_some());
    // Back to the default: the slider is set back, not left behind.
    apply_description(
        &mut game.world,
        &game.assets,
        character,
        &described(&[], &[]),
    )
    .unwrap();
    assert!((height(&game).unwrap() - 1.0).abs() < 1e-6);
    assert!(axcenity_anim::BodyShape::read(&game.world, body).is_none());
}

#[test]
fn material_values_modulate_the_material_and_keep_its_maps() {
    let mut game = game("description-materials");
    let character = game.character();
    let body = game.world.find_first_child(character, "Body").unwrap();
    let draw = |game: &mut Game, part: InstanceId| -> Vec<axcenity_render::Material> {
        game.frames(2, MoveInput::default());
        let mut terrain = axcenity_scene::TerrainCache::new();
        let camera = axcenity_render::Camera {
            position: Vec3::new(0.0, 6.0, 20.0),
            look_at: Vec3::new(0.0, 3.0, 0.0),
            fov_y_degrees: 70.0,
        };
        let id = axcenity_scene::item_id(part);
        let list = axcenity_scene::build_draw_list_posed(
            &game.world,
            &game.assets,
            camera,
            game.animators.poses(),
            &mut terrain,
        );
        list.items
            .iter()
            .filter(|item| item.id == id)
            .map(|item| item.material.clone())
            .collect()
    };
    let plain = described(&[], &["axcenity/tee"]);
    apply_description(&mut game.world, &game.assets, character, &plain).unwrap();
    let tee = game.world.find_first_child(body, "Shirt_Tee").unwrap();
    let authored = draw(&mut game, tee);
    assert_eq!(authored.len(), 1, "the T-shirt has one material");
    let skin_authored = draw(&mut game, body);

    let mut tinted = plain.clone();
    tinted.items[0].materials.insert(
        "Shirt".to_owned(),
        [
            (
                "tint".to_owned(),
                MaterialValue::Colour(Colour::parse("#804020").unwrap()),
            ),
            ("roughness_scale".to_owned(), MaterialValue::Number(0.5)),
        ]
        .into(),
    );
    tinted.materials.insert(
        "Skin".to_owned(),
        [(
            "tint".to_owned(),
            MaterialValue::Colour(Colour::parse("#c08060").unwrap()),
        )]
        .into(),
    );
    apply_description(&mut game.world, &game.assets, character, &tinted).unwrap();
    let drawn = draw(&mut game, tee);
    let (before, after) = (&authored[0], &drawn[0]);
    let tint = [128.0 / 255.0, 64.0 / 255.0, 32.0 / 255.0];
    for channel in 0..3 {
        let wanted = before.base_color[channel] * tint[channel];
        assert!(
            (after.base_color[channel] - wanted).abs() < 1e-5,
            "{:?} × {tint:?} isn't {:?}",
            before.base_color,
            after.base_color
        );
    }
    assert!((after.roughness - before.roughness * 0.5).abs() < 1e-5);
    // Everything else is as authored: the maps above all.
    let key = |texture: &Option<std::sync::Arc<dyn axcenity_render::TextureSource>>| {
        texture.as_ref().map(|texture| texture.key())
    };
    assert_eq!(
        key(&after.base_color_texture),
        key(&before.base_color_texture)
    );
    assert_eq!(
        after.maps.normal.as_ref().map(|map| map.texture.key()),
        before.maps.normal.as_ref().map(|map| map.texture.key())
    );
    assert_eq!(after.metalness, before.metalness);
    // The skin's tint is on the body, on its skin material only.
    let skin = draw(&mut game, body);
    assert_eq!(skin.len(), skin_authored.len());
    let changed = skin
        .iter()
        .zip(&skin_authored)
        .filter(|(now, was)| now.base_color != was.base_color)
        .count();
    assert_eq!(changed, 1, "only the Skin material is tinted");
    // Taking the values away gives the authored material back.
    apply_description(&mut game.world, &game.assets, character, &plain).unwrap();
    assert_eq!(draw(&mut game, tee)[0].base_color, before.base_color);
}

#[test]
fn a_kept_description_is_applied_again_where_the_world_is_loaded() {
    let mut game = game("description-sync");
    let character = game.character();
    let body = game.world.find_first_child(character, "Body").unwrap();
    let heavy = described(&[("build", 1.0), ("frame", 1.0)], &OUTFIT);
    apply_description(&mut game.world, &game.assets, character, &heavy).unwrap();
    let derived = mesh_id(&game, body);
    // Another run of the engine: its assets know no identities, so the
    // saved `MeshId` is the plain body until the description is applied.
    let fresh = Assets::new(&game.folder);
    let plain = fresh.model_blocking(&derived).unwrap();
    assert!(!plain.settles());
    let mut sync = DescriptionSync::default();
    let errors = sync.step(&mut game.world, &fresh);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        mesh_id(&game, body),
        derived,
        "the same identity, the same ID"
    );
    let baked = fresh.model_blocking(&derived).unwrap();
    assert!(baked.settles(), "now it's the derived model");
    // Stepped again, nothing is applied twice.
    let before = game.world.structure_version();
    assert!(sync.step(&mut game.world, &fresh).is_empty());
    assert_eq!(game.world.structure_version(), before);
    // A changed kept description is applied at the next step.
    let kept = applied_instance(&game.world, character).unwrap();
    let bald = described(&[], &[]);
    game.world
        .set_property(kept, "Data", Value::String(bald.to_json()))
        .unwrap();
    assert!(sync.step(&mut game.world, &fresh).is_empty());
    assert_eq!(parts(&game, body), ["Brows", "Eyes", "Lashes", "Teeth"]);
    // One that can't be read is said once.
    game.world
        .set_property(kept, "Data", Value::String("{ not json".to_owned()))
        .unwrap();
    assert_eq!(sync.step(&mut game.world, &fresh).len(), 1);
    assert!(sync.step(&mut game.world, &fresh).is_empty());
}
