//! Items in groups (the list's chunks) are drawn through their clusters
//! (`renderer/clusters.rs`): what a cluster keeps of an item must be what
//! the item says, so a grouped list draws exactly as the same items do
//! listed one by one.

use super::*;
use crate::{ItemChunk, RayTracedAmbientOcclusion, RayTracedReflections, RayTracedShadows};

/// Blocks and balls of several looks over a floor: plain colors, materials
/// with values of their own (so their bindings differ), cut-outs,
/// see-through ones and one that glows.
fn plants() -> Vec<DrawItem> {
    (0..48)
        .map(|i| {
            let shape = if i % 3 == 0 {
                Shape::Sphere
            } else {
                Shape::Block
            };
            let position = Vec3::new((i % 8) as f32 * 1.4 - 5.0, (i / 8) as f32 * 0.9 - 2.0, -9.0);
            let mut item = item(shape, position, if i % 7 == 3 { 0.5 } else { 1.0 });
            item.material.base_color = [(i % 4) as f32 / 3.0, (i % 5) as f32 / 4.0, 0.6];
            if i % 4 == 1 {
                item.material.surface_parameters[0][0] = (i % 3) as f32 + 1.0;
            }
            if i % 6 == 2 {
                item.material.alpha_cutoff = 0.5;
            }
            if i == 20 {
                item.material.emissive = [2.0, 1.0, 0.0];
            }
            item.material.roughness = 0.1;
            item
        })
        .collect()
}

fn floor() -> DrawItem {
    let mut floor = item(Shape::Block, Vec3::new(0.0, -3.5, -9.0), 1.0);
    floor.transform = Mat4::from_translation(Vec3::new(0.0, -3.5, -9.0))
        * Mat4::from_scale(Vec3::new(30.0, 1.0, 30.0));
    floor.material.roughness = 0.05;
    floor
}

/// The floor and `items`, each listed on its own.
fn listed(items: Vec<DrawItem>) -> DrawList {
    let mut all = vec![floor()];
    all.extend(items);
    list_of(all)
}

/// The floor listed on its own and `items` in groups of `size`.
fn grouped(items: Vec<DrawItem>, size: usize) -> DrawList {
    let mut list = list_of(vec![floor()]);
    list.chunks = items
        .chunks(size)
        .map(|items| ItemChunk {
            center: Vec3::new(0.0, 0.0, -9.0),
            radius: 40.0,
            cull_distance: 0.0,
            items: items.to_vec().into(),
        })
        .collect();
    list
}

#[test]
fn a_grouped_items_draw_says_which_group_and_item_it_is() {
    let list = grouped(plants(), 10);
    let resolved = LodStates::default().resolve(&list, 1, None);
    assert_eq!(resolved.clusters.len(), 5);
    assert_eq!(resolved.draws.len(), 49);
    // The list's own items first, in no group.
    assert_eq!(resolved.draws[0].cluster, lod::NO_CLUSTER);
    for draw in &resolved.draws[1..] {
        let group = resolved.clusters[draw.cluster as usize];
        assert!(std::ptr::eq(draw.item, &group.items[draw.at as usize]));
    }
    let clustered = resolved
        .draws
        .partition_point(|draw| draw.cluster == lod::NO_CLUSTER);
    assert_eq!(clustered, 1, "own draws come before every group's");
}

#[test]
fn grouped_items_get_the_material_bindings_they_would_listed_one_by_one() {
    let uses_of = |list: &DrawList| {
        let resolved = LodStates::default().resolve(list, 1, None);
        let mut clusters = clusters::Clusters::default();
        let groups = clusters.of(&resolved.clusters, 1);
        let uses = FrameUses::of(&resolved.draws, &groups);
        // Every draw's binding is its item's.
        for (draw, &index) in resolved.draws.iter().zip(&uses.material_of) {
            assert_eq!(uses.materials[index as usize], material_key(draw.item));
        }
        // And each binding's first draw is the first to use it.
        for (index, &first) in uses.first_draw.iter().enumerate().skip(1) {
            let earliest = uses.material_of.iter().position(|&m| m as usize == index);
            assert_eq!(earliest, Some(first as usize));
        }
        (uses.materials, uses.material_of, uses.first_draw)
    };
    let one_by_one = uses_of(&listed(plants()));
    assert!(one_by_one.0.len() >= 4, "several bindings to tell apart");
    // The same items in the same order, so the same answer, however
    // they're grouped.
    assert_eq!(uses_of(&grouped(plants(), 10)), one_by_one);
    assert_eq!(uses_of(&grouped(plants(), 1)), one_by_one);
    assert_eq!(uses_of(&grouped(plants(), 48)), one_by_one);
}

/// `list` drawn a few frames (clusters are made on the first and found
/// again after), as pixels.
fn drawn(renderer: &mut TextureRenderer, list: &DrawList) -> Vec<u8> {
    for _ in 0..3 {
        renderer.render(list).unwrap();
    }
    renderer.read_pixels()
}

/// How far two pictures are apart at their worst pixel channel.
fn worst(a: &[u8], b: &[u8]) -> u8 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0)
}

#[test]
fn a_grouped_list_draws_the_same_picture_as_its_items_listed_one_by_one() {
    let Some((device, queue)) = test_device() else {
        return;
    };
    let one_by_one = drawn(
        &mut TextureRenderer::new(&device, &queue, 128, 96),
        &listed(plants()),
    );
    // Something is in the picture: it isn't one flat color.
    let colors: std::collections::HashSet<&[u8]> = one_by_one.chunks(4).collect();
    assert!(colors.len() > 20, "{} colors", colors.len());
    for size in [1, 10, 48] {
        let mut renderer = TextureRenderer::new(&device, &queue, 128, 96);
        let picture = drawn(&mut renderer, &grouped(plants(), size));
        assert_eq!(worst(&picture, &one_by_one), 0, "in groups of {size}");
    }
}

#[test]
fn a_grouped_list_is_ray_traced_the_same_as_its_items_listed_one_by_one() {
    let Some((device, queue)) = ray_test_device() else {
        return;
    };
    // Shadows and ambient occlusion read each item's mask and holes;
    // reflections read its surface (its color and glow).
    let settings = crate::GraphicsSettings {
        ray_traced_shadows: Some(RayTracedShadows::High),
        ray_traced_ambient_occlusion: Some(RayTracedAmbientOcclusion::High),
        ray_traced_reflections: Some(RayTracedReflections::High),
        ..crate::GraphicsSettings::default()
    };
    let picture_of = |list: &DrawList| {
        let mut renderer = TextureRenderer::new(&device, &queue, 128, 96);
        renderer.set_graphics(&settings);
        let picture = drawn(&mut renderer, list);
        let stats = renderer.stats().ray_tracing;
        (picture, stats.instances)
    };
    let (one_by_one, instances) = picture_of(&listed(plants()));
    assert_eq!(instances, 49, "every item is traced");
    for size in [1, 10, 48] {
        let (picture, traced) = picture_of(&grouped(plants(), size));
        assert_eq!(traced, instances, "in groups of {size}");
        assert_eq!(worst(&picture, &one_by_one), 0, "in groups of {size}");
    }
}
