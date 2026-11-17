//! Group as Mesh: parts made into one model, saved as a `.glb` that loads
//! with their looks, textures and collision pieces.

use std::sync::Arc;

use axcenity_assets::{AlphaMode, Image, ModelFormat, ModelMaterial, load_model};
use glam::{Quat, Vec3};

use crate::{GroupLook, GroupSource, MaterialMaps, Mesh, group};

fn plain(color: [f32; 4]) -> GroupLook {
    let material = ModelMaterial {
        name: "Plastic".to_owned(),
        base_color: color,
        ..ModelMaterial::default()
    };
    GroupLook::new(&material, MaterialMaps::default())
}

fn block(size: [f32; 3], at: [f32; 3], look: GroupLook) -> GroupSource {
    let mesh = Mesh::block(Vec3::from(size));
    let corners = mesh.positions.clone();
    GroupSource {
        name: "Block".to_owned(),
        mesh,
        looks: vec![look],
        position: Vec3::from(at),
        rotation: Quat::IDENTITY,
        drawn: true,
        collision: vec![corners],
    }
}

#[test]
fn two_blocks_group_into_one_object_that_loads_with_both_collision_pieces() {
    let grey = plain([0.5, 0.5, 0.5, 1.0]);
    let sources = [
        block([1.0; 3], [10.0, 0.5, 0.0], grey.clone()),
        block([1.0; 3], [11.0, 0.5, 0.0], grey),
    ];
    let grouped = group(&sources).unwrap();
    assert_eq!(grouped.mesh.triangles.len(), 12);
    assert_eq!(grouped.looks.len(), 1, "the same look is one material");
    assert_eq!(grouped.position, Vec3::new(10.5, 0.5, 0.0));
    assert_eq!(grouped.size, Vec3::new(2.0, 1.0, 1.0));
    assert_eq!(grouped.stats.triangles_before, 24);
    assert_eq!(grouped.stats.triangles_after, 12);
    assert_eq!(grouped.stats.parts, 2);
    assert_eq!(
        grouped.stats.summary(),
        "Grouped 2 parts: 24 → 12 triangles, 48 → 24 vertices (triangles taken away: \
         4 pressed together, 8 merged in flat faces)."
    );

    let model = load_model(&grouped.glb(""), &std::env::temp_dir(), ModelFormat::Gltf).unwrap();
    assert_eq!(model.primitives.len(), 1);
    assert_eq!(model.primitives[0].indices.len(), 36);
    assert_eq!(model.size, [2.0, 1.0, 1.0]);
    assert_eq!(model.collision.len(), 2, "each block collides as itself");
    // Pieces are fitted into the model's 1×1×1 cube like its triangles.
    let xs: Vec<f32> = model.collision[1].iter().map(|p| p[0]).collect();
    assert!(xs.iter().all(|&x| (0.0..=0.5).contains(&x)), "{xs:?}");
}

#[test]
fn turned_and_see_through_parts_keep_their_place_and_looks() {
    let red = plain([1.0, 0.0, 0.0, 1.0]);
    let mut glass = plain([0.0, 0.0, 1.0, 0.5]);
    glass.material.alpha_mode = AlphaMode::Blend;
    let mut turned = block([2.0, 1.0, 1.0], [0.0, 3.0, 0.0], glass);
    turned.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let sources = [block([1.0; 3], [0.0; 3], red), turned];
    let grouped = group(&sources).unwrap();
    assert_eq!(grouped.looks.len(), 2);
    assert_eq!(grouped.mesh.triangles.len(), 24, "nothing touches");
    // The turned block lies along Z now.
    let (min, max) = grouped.mesh.bounds();
    let (min, max) = (min + grouped.position, max + grouped.position);
    assert!((min - Vec3::new(-0.5, -0.5, -1.0)).abs().max_element() < 1e-5);
    assert!((max - Vec3::new(0.5, 3.5, 1.0)).abs().max_element() < 1e-5);
}

#[test]
fn an_invisible_part_isnt_drawn_but_still_collides() {
    let grey = plain([0.5, 0.5, 0.5, 1.0]);
    let mut wall = block([1.0, 4.0, 1.0], [5.0, 0.0, 0.0], grey.clone());
    wall.drawn = false;
    let grouped = group(&[block([1.0; 3], [0.0; 3], grey), wall]).unwrap();
    assert_eq!(grouped.mesh.triangles.len(), 12);
    assert_eq!(grouped.size, Vec3::ONE);
    assert_eq!(grouped.collision.len(), 2);
    let model = load_model(&grouped.glb(""), &std::env::temp_dir(), ModelFormat::Gltf).unwrap();
    assert_eq!(model.collision.len(), 2);
}

#[test]
fn textures_are_linked_from_the_folder_the_group_is_saved_in() {
    let mut image = Image::new(1, 1, vec![255; 4]);
    image.file = Some("textures/wood.png".to_owned());
    let material = ModelMaterial {
        base_color_texture: Some(Arc::new(image)),
        ..ModelMaterial::default()
    };
    // As a model in `assets/models/chair` links it.
    let maps = MaterialMaps::of(&material).map(|file| format!("assets/models/chair/{file}"));
    let chair = GroupLook::new(&material, maps);
    assert!(chair.material.base_color_texture.is_none());
    let sources = [
        block([1.0; 3], [0.0; 3], chair.clone()),
        block([1.0; 3], [3.0, 0.0, 0.0], plain([1.0; 4])),
    ];
    let grouped = group(&sources).unwrap();
    assert_eq!(grouped.folder(), "assets/models");
    let glb = grouped.glb(&grouped.folder());
    let text = String::from_utf8_lossy(&glb);
    assert!(
        text.contains("\"uri\":\"chair/textures/wood.png\""),
        "{text}"
    );

    // A model outside `assets/models`: the group goes where both are.
    let mut elsewhere = chair;
    elsewhere.maps.base_color = Some("assets/props/textures/wood.png".to_owned());
    let grouped = group(&[block([1.0; 3], [0.0; 3], elsewhere)]).unwrap();
    assert_eq!(grouped.folder(), "assets");
    let glb = grouped.glb(&grouped.folder());
    assert!(String::from_utf8_lossy(&glb).contains("\"uri\":\"props/textures/wood.png\""));
}

#[test]
fn nothing_to_see_is_an_error() {
    let mut hidden = block([1.0; 3], [0.0; 3], plain([1.0; 4]));
    hidden.drawn = false;
    assert!(group(&[hidden]).is_err());
}
