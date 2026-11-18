//! Skinning on the GPU: a skin moves its mesh's corners, items posed
//! differently share one mesh and batch, and culling uses the posed box.

use super::*;
use crate::Skin;

/// A unit square facing +Z, 30 studs to the right of where it's drawn
/// until its skin moves it: every corner is weighted to one joint.
pub(super) struct SkinnedSquare {
    joints: [[u16; 4]; 4],
}

impl MeshSource for SkinnedSquare {
    fn key(&self) -> u64 {
        91 + u64::from(self.joints[0][0])
    }
    fn positions(&self) -> &[[f32; 3]] {
        &[
            [29.5, -0.5, 0.0],
            [30.5, -0.5, 0.0],
            [30.5, 0.5, 0.0],
            [29.5, 0.5, 0.0],
        ]
    }
    fn normals(&self) -> &[[f32; 3]] {
        &[[0.0, 0.0, 1.0]; 4]
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &[]
    }
    fn indices(&self) -> &[u32] {
        &[0, 1, 2, 0, 2, 3]
    }
    fn skin_joints(&self) -> &[[u16; 4]] {
        &self.joints
    }
    fn skin_weights(&self) -> &[[f32; 4]] {
        &[[1.0, 0.0, 0.0, 0.0]; 4]
    }
}

/// A skin of `count` joints, the last moving its corners by `offset`.
pub(super) fn skin(count: usize, offset: Vec3) -> Arc<Skin> {
    let mut joints = vec![Mat4::IDENTITY; count];
    joints[count - 1] = Mat4::from_translation(offset);
    Arc::new(Skin::new(joints))
}

/// The square, glowing white so it shows however it's lit.
pub(super) fn square(joint: u16, skin: Option<Arc<Skin>>) -> DrawItem {
    let mut item = item(
        Shape::Mesh(Arc::new(SkinnedSquare {
            joints: [[joint, 0, 0, 0]; 4],
        })),
        Vec3::new(0.0, 0.0, -3.0),
        1.0,
    );
    item.material.emissive = [1.0; 3];
    DrawItem { skin, ..item }
}

#[test]
fn a_skin_moves_its_mesh_into_view_and_is_not_culled_where_it_rested() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 32, 32);
    let middle = |renderer: &mut TextureRenderer, list: &DrawList| {
        renderer.render(list).unwrap();
        let pixels = renderer.read_pixels();
        let at = (16 * 32 + 16) * 4;
        [pixels[at], pixels[at + 1], pixels[at + 2]]
    };
    // Unposed, the square is off to the side: culled, and the sky shows.
    let sky = middle(&mut renderer, &list_of(vec![square(0, None)]));
    assert_eq!(renderer.stats().items_culled, 1);

    // Its one joint moves it back to the middle of the view.
    let posed = list_of(vec![square(0, Some(skin(1, Vec3::new(-30.0, 0.0, 0.0))))]);
    let drawn = middle(&mut renderer, &posed);
    let stats = renderer.stats();
    assert_eq!((stats.items_drawn, stats.items_culled), (1, 0));
    assert!(
        drawn.iter().zip(sky).any(|(a, b)| a.abs_diff(b) > 30),
        "the posed square isn't drawn: {drawn:?} vs sky {sky:?}"
    );

    // A skin with more joints than the frame's joints first had room for.
    let many = list_of(vec![square(
        99,
        Some(skin(100, Vec3::new(-30.0, 0.0, 0.0))),
    )]);
    assert_eq!(middle(&mut renderer, &many), drawn);
}

#[test]
fn items_posed_differently_share_one_mesh_and_batch() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 32, 32);
    let list = list_of(vec![
        square(0, Some(skin(1, Vec3::new(-30.6, 0.0, 0.0)))),
        square(0, Some(skin(1, Vec3::new(-29.4, 0.0, 0.0)))),
    ]);
    renderer.render(&list).unwrap();
    let stats = renderer.stats();
    assert_eq!((stats.items_drawn, stats.batches), (2, 1));
    // Two skins of one joint each, and each item posed into a range of
    // its own in the frame's posed buffer (CO8): its first corner there.
    let plan = plan_with(&list, |_| Bounds::UNIT);
    assert_eq!(plan.joints.len(), 2);
    assert_eq!(plan.posed.len(), 2, "two (skin, mesh) pairs to pose");
    assert_eq!(renderer.stats().posing.items, 2);
    assert_eq!(renderer.stats().posing.corners, 8);
}

#[test]
fn a_skins_box_holds_each_corner_wherever_its_joints_move_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 32, 32);
    let posed = skin(1, Vec3::new(-30.0, 2.0, 0.0));
    renderer
        .render(&list_of(vec![square(0, Some(posed.clone()))]))
        .unwrap();
    let key = MeshKey::Custom(91);
    assert_eq!(
        renderer.scene.mesh_bounds(key),
        Bounds {
            min: Vec3::new(29.5, -0.5, 0.0),
            max: Vec3::new(30.5, 0.5, 0.0),
        }
    );
    assert_eq!(
        renderer.scene.item_bounds(key, Some(&posed)),
        Bounds {
            min: Vec3::new(-0.5, 1.5, 0.0),
            max: Vec3::new(0.5, 2.5, 0.0),
        }
    );
}

/// A unit square facing +Z, 30 studs to the right of where it's drawn,
/// with no joints and one blend shape (target 1) moving every corner 30
/// studs left, back into view.
struct MorphedSquare;

const MORPHED_CORNERS: [u32; 4] = [0, 1, 2, 3];
const MORPHED_MOVES: [[f32; 3]; 4] = [[-30.0, 0.0, 0.0]; 4];
const MORPHED_NORMALS: [[f32; 3]; 4] = [[0.0; 3]; 4];

impl MeshSource for MorphedSquare {
    fn key(&self) -> u64 {
        77
    }
    fn positions(&self) -> &[[f32; 3]] {
        &[
            [29.5, -0.5, 0.0],
            [30.5, -0.5, 0.0],
            [30.5, 0.5, 0.0],
            [29.5, 0.5, 0.0],
        ]
    }
    fn normals(&self) -> &[[f32; 3]] {
        &[[0.0, 0.0, 1.0]; 4]
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &[]
    }
    fn indices(&self) -> &[u32] {
        &[0, 1, 2, 0, 2, 3]
    }
    fn morph_count(&self) -> usize {
        1
    }
    fn morph(&self, index: usize) -> Option<crate::MorphView<'_>> {
        (index == 0).then_some(crate::MorphView {
            target: 1,
            corners: &MORPHED_CORNERS,
            positions: &MORPHED_MOVES,
            normals: &MORPHED_NORMALS,
        })
    }
}

fn morphed_square(weights: Option<&[f32]>) -> DrawItem {
    let mut item = item(
        Shape::Mesh(Arc::new(MorphedSquare)),
        Vec3::new(0.0, 0.0, -3.0),
        1.0,
    );
    item.material.emissive = [1.0; 3];
    let skin = weights.map(|weights| {
        let mut skin = Skin::default();
        skin.set_morph_weights(weights);
        Arc::new(skin)
    });
    DrawItem { skin, ..item }
}

#[test]
fn a_blend_shape_moves_its_mesh_into_view_by_its_weight() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 32, 32);
    let middle = |renderer: &mut TextureRenderer, list: &DrawList| {
        renderer.render(list).unwrap();
        let pixels = renderer.read_pixels();
        let at = (16 * 32 + 16) * 4;
        [pixels[at], pixels[at + 1], pixels[at + 2]]
    };
    let sky = middle(&mut renderer, &list_of(vec![morphed_square(None)]));
    assert_eq!(renderer.stats().items_culled, 1, "off to the side at rest");
    // Weight 0 on its target keeps it there (its box reaches as far as
    // its shapes can move it, so it isn't culled, but nothing shows).
    let still = middle(
        &mut renderer,
        &list_of(vec![morphed_square(Some(&[1.0, 0.0]))]),
    );
    assert_eq!(still, sky);
    // Weight 1 brings it into the middle, and its box goes with it.
    let drawn = middle(
        &mut renderer,
        &list_of(vec![morphed_square(Some(&[0.0, 1.0]))]),
    );
    let stats = renderer.stats();
    assert_eq!((stats.items_drawn, stats.items_culled), (1, 0));
    assert!(
        drawn.iter().zip(sky).any(|(a, b)| a.abs_diff(b) > 30),
        "the morphed square isn't drawn: {drawn:?} vs sky {sky:?}"
    );
}

#[test]
fn a_skins_joints_go_to_the_gpu_without_its_shape_weights() {
    // Shape weights reach the posing pass as each item's list of active
    // shapes (CO8), not through the joints buffer.
    let mut skin = Skin::new(vec![Mat4::IDENTITY; 3]);
    let weights: Vec<f32> = (0..20).map(|w| w as f32).collect();
    skin.set_morph_weights(&weights);
    let raw: Vec<_> = super::super::skinning::JointRaw::of(&skin).collect();
    assert_eq!(raw.len(), 3);
}

/// A unit square facing +Z, 30 studs to the right of where it's drawn,
/// every corner weighted to joint 0, with one blend shape (target 0) moving
/// every corner 30 studs left: with weight 1 on it and its joint still, it
/// is drawn where the item is (CO8: the shapes, then the joints).
pub(super) struct MorphedSkinnedSquare;

const SQUARE_CORNERS: [[f32; 3]; 4] = [
    [29.5, -0.5, 0.0],
    [30.5, -0.5, 0.0],
    [30.5, 0.5, 0.0],
    [29.5, 0.5, 0.0],
];

impl MeshSource for MorphedSkinnedSquare {
    fn key(&self) -> u64 {
        78
    }
    fn positions(&self) -> &[[f32; 3]] {
        &SQUARE_CORNERS
    }
    fn normals(&self) -> &[[f32; 3]] {
        &[[0.0, 0.0, 1.0]; 4]
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &[]
    }
    fn indices(&self) -> &[u32] {
        &[0, 1, 2, 0, 2, 3]
    }
    fn skin_joints(&self) -> &[[u16; 4]] {
        &[[0; 4]; 4]
    }
    fn skin_weights(&self) -> &[[f32; 4]] {
        &[[1.0, 0.0, 0.0, 0.0]; 4]
    }
    fn morph_count(&self) -> usize {
        1
    }
    fn morph(&self, index: usize) -> Option<crate::MorphView<'_>> {
        (index == 0).then_some(crate::MorphView {
            target: 0,
            corners: &MORPHED_CORNERS,
            positions: &MORPHED_MOVES,
            normals: &MORPHED_NORMALS,
        })
    }
}

/// [`MorphedSkinnedSquare`] with its one joint still and `weight` on its
/// shape.
pub(super) fn morphed_skinned_square(weight: f32) -> DrawItem {
    let mut item = item(
        Shape::Mesh(Arc::new(MorphedSkinnedSquare)),
        Vec3::new(0.0, 0.0, -3.0),
        1.0,
    );
    item.material.emissive = [1.0; 3];
    let mut skin = Skin::new(vec![Mat4::IDENTITY]);
    skin.set_morph_weights(&[weight]);
    DrawItem {
        skin: Some(Arc::new(skin)),
        ..item
    }
}

/// Six corners of a bent strip, each weighted to two joints, with three
/// blend shapes (targets 0, 1 and 2) that move and turn some of them: what
/// the posed buffer is checked against.
struct ShapedStrip;

const STRIP_CORNERS: [[f32; 3]; 6] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.2],
    [0.0, 1.0, 0.1],
    [1.0, 1.0, -0.3],
    [0.0, 2.0, 0.0],
    [1.0, 2.0, 0.4],
];
const STRIP_NORMALS: [[f32; 3]; 6] = [
    [0.0, 0.0, 1.0],
    [0.6, 0.0, 0.8],
    [0.0, 0.6, 0.8],
    [-0.6, 0.0, 0.8],
    [0.0, -0.6, 0.8],
    [0.0, 0.0, 1.0],
];
const STRIP_TANGENTS: [[f32; 4]; 6] = [
    [1.0, 0.0, 0.0, 1.0],
    [0.8, 0.0, -0.6, 1.0],
    [1.0, 0.0, 0.0, -1.0],
    [0.8, 0.0, 0.6, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
];
const STRIP_JOINTS: [[u16; 4]; 6] = [[0, 1, 0, 0]; 6];
const STRIP_WEIGHTS: [[f32; 4]; 6] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.8, 0.2, 0.0, 0.0],
    [0.5, 0.5, 0.0, 0.0],
    [0.3, 0.7, 0.0, 0.0],
    [0.1, 0.9, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
];
const SHAPE_0_CORNERS: [u32; 3] = [0, 2, 4];
const SHAPE_0_MOVES: [[f32; 3]; 3] = [[0.1, 0.0, 0.3], [0.0, -0.2, 0.1], [0.25, 0.05, 0.0]];
const SHAPE_0_TURNS: [[f32; 3]; 3] = [[0.3, 0.0, 0.0], [0.0, 0.2, -0.1], [0.0, 0.0, 0.0]];
const SHAPE_1_CORNERS: [u32; 3] = [1, 2, 3];
const SHAPE_1_MOVES: [[f32; 3]; 3] = [[5.0, 5.0, 5.0]; 3];
const SHAPE_1_TURNS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0]; 3];
const SHAPE_2_CORNERS: [u32; 6] = [0, 1, 2, 3, 4, 5];
const SHAPE_2_MOVES: [[f32; 3]; 6] = [
    [0.0, 0.0, 0.5],
    [0.1, 0.0, 0.5],
    [0.0, 0.1, 0.5],
    [-0.1, 0.0, 0.5],
    [0.0, -0.1, 0.5],
    [0.05, 0.05, 0.5],
];

impl MeshSource for ShapedStrip {
    fn key(&self) -> u64 {
        79
    }
    fn positions(&self) -> &[[f32; 3]] {
        &STRIP_CORNERS
    }
    fn normals(&self) -> &[[f32; 3]] {
        &STRIP_NORMALS
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &[]
    }
    fn tangents(&self) -> &[[f32; 4]] {
        &STRIP_TANGENTS
    }
    fn indices(&self) -> &[u32] {
        &[0, 1, 2, 1, 3, 2, 2, 3, 4, 3, 5, 4]
    }
    fn skin_joints(&self) -> &[[u16; 4]] {
        &STRIP_JOINTS
    }
    fn skin_weights(&self) -> &[[f32; 4]] {
        &STRIP_WEIGHTS
    }
    fn morph_count(&self) -> usize {
        3
    }
    fn morph(&self, index: usize) -> Option<crate::MorphView<'_>> {
        match index {
            0 => Some(crate::MorphView {
                target: 0,
                corners: &SHAPE_0_CORNERS,
                positions: &SHAPE_0_MOVES,
                normals: &SHAPE_0_TURNS,
            }),
            1 => Some(crate::MorphView {
                target: 1,
                corners: &SHAPE_1_CORNERS,
                positions: &SHAPE_1_MOVES,
                normals: &SHAPE_1_TURNS,
            }),
            // A shape that moves without turning (no normals).
            2 => Some(crate::MorphView {
                target: 2,
                corners: &SHAPE_2_CORNERS,
                positions: &SHAPE_2_MOVES,
                normals: &[],
            }),
            _ => None,
        }
    }
}

/// The strip's two joints, one turned and moved, one grown and moved, and
/// `weights` on its shapes.
fn strip_skin(weights: &[f32]) -> Arc<Skin> {
    let mut skin = Skin::new(vec![
        Mat4::from_rotation_translation(
            glam::Quat::from_rotation_y(0.5) * glam::Quat::from_rotation_x(-0.2),
            Vec3::new(0.3, -0.1, 0.2),
        ),
        Mat4::from_scale_rotation_translation(
            Vec3::new(1.2, 0.9, 1.1),
            glam::Quat::from_rotation_z(0.3),
            Vec3::new(-0.2, 0.4, 0.0),
        ),
    ]);
    skin.set_morph_weights(weights);
    Arc::new(skin)
}

fn strip(weights: &[f32]) -> DrawItem {
    let item = item(
        Shape::Mesh(Arc::new(ShapedStrip)),
        Vec3::new(-0.5, -1.0, -4.0),
        1.0,
    );
    DrawItem {
        skin: Some(strip_skin(weights)),
        ..item
    }
}

fn unit_or_zero(v: Vec3) -> Vec3 {
    if v.length_squared() > 0.0 {
        v.normalize()
    } else {
        Vec3::ZERO
    }
}

/// Where the vertex shaders put each corner of `source` posed by `skin`
/// in every pass before CO8 (`morph_corner` then `pose_corner` in
/// shader.wgsl, as they were): its position, normal and tangent.
fn per_pass_pose(source: &dyn MeshSource, skin: &Skin) -> Vec<[Vec3; 3]> {
    let weights = skin.morph_weights();
    (0..source.positions().len())
        .map(|corner| {
            let mut position = Vec3::from(source.positions()[corner]);
            let mut normal = Vec3::from(source.normals()[corner]);
            // A mesh without tangents gets `mesh::NO_TANGENT`'s.
            let tangent = source
                .tangents()
                .get(corner)
                .map_or(Vec3::X, |tangent| Vec3::from_slice(&tangent[..3]));
            let mut turned = false;
            for index in 0..source.morph_count() {
                let Some(morph) = source.morph(index) else {
                    continue;
                };
                let Some(at) = morph.corners.iter().position(|&c| c as usize == corner) else {
                    continue;
                };
                let weight = weights
                    .get(usize::from(morph.target))
                    .copied()
                    .unwrap_or(0.0);
                position += Vec3::from(morph.positions[at]) * weight;
                normal += morph
                    .normals
                    .get(at)
                    .map_or(Vec3::ZERO, |turn| Vec3::from(*turn))
                    * weight;
                turned |= weight != 0.0;
            }
            if turned {
                normal = unit_or_zero(normal);
            }
            let joints = source.skin_joints()[corner];
            let joint_weights = source.skin_weights()[corner];
            let mut matrix = Mat4::ZERO;
            let mut normal_matrix = glam::Mat3::ZERO;
            let mut total = 0.0;
            for (joint, weight) in joints.iter().zip(joint_weights) {
                if weight <= 0.0 {
                    continue;
                }
                matrix += skin.joints()[usize::from(*joint)] * weight;
                normal_matrix += skin.normals()[usize::from(*joint)] * weight;
                total += weight;
            }
            [
                matrix.transform_point3(position) / total,
                unit_or_zero(normal_matrix * normal),
                unit_or_zero(matrix.transform_vector3(tangent)),
            ]
        })
        .collect()
}

#[test]
fn the_posed_buffer_holds_each_corner_where_every_pass_used_to_pose_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 32, 32);
    let weights = [0.5, 0.0, -0.75];
    let list = list_of(vec![strip(&weights)]);
    renderer.render(&list).unwrap();
    let skin = list.items[0].skin.clone().unwrap();
    let posed = renderer
        .scene
        .read_posed(&skin, MeshKey::Custom(79))
        .expect("the strip is posed into the frame's posed buffer");
    let expected = per_pass_pose(&ShapedStrip, &skin);
    assert_eq!(posed.len(), expected.len());
    for (corner, (posed, expected)) in posed.iter().zip(&expected).enumerate() {
        let [position, normal, tangent] = *posed;
        assert!(
            position.distance(expected[0]) < 1e-4,
            "corner {corner}: position {position} vs {}",
            expected[0]
        );
        assert!(
            normal.distance(expected[1]) < 2e-3,
            "corner {corner}: normal {normal} vs {}",
            expected[1]
        );
        assert!(
            tangent.distance(expected[2]) < 2e-3,
            "corner {corner}: tangent {tangent} vs {}",
            expected[2]
        );
    }
}

#[test]
fn a_shape_at_weight_zero_is_not_posed_at_all() {
    use super::super::posing::{ShapeRange, active_shapes};
    // The pure part: only shapes whose target has a weight other than 0
    // (and a number) are listed, each with its moves.
    let shapes = [
        ShapeRange {
            first: 0,
            count: 3,
            target: 0,
        },
        ShapeRange {
            first: 3,
            count: 3,
            target: 1,
        },
        ShapeRange {
            first: 6,
            count: 6,
            target: 2,
        },
        ShapeRange {
            first: 12,
            count: 2,
            target: 7,
        },
    ];
    let active = active_shapes(&shapes, &[0.5, 0.0, -0.75, 1.0]);
    let listed: Vec<(u32, u32, u32, f32)> = active
        .iter()
        .map(|shape| (shape.first, shape.count, shape.start, shape.weight))
        .collect();
    assert_eq!(listed, [(0, 3, 0, 0.5), (6, 6, 3, -0.75)]);
    assert!(active_shapes(&shapes, &[0.0, f32::NAN]).is_empty());

    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 32, 32);
    renderer
        .render(&list_of(vec![strip(&[0.5, 0.0, -0.75])]))
        .unwrap();
    let posing = renderer.stats().posing;
    assert_eq!(
        (posing.items, posing.corners, posing.shapes, posing.moves),
        (1, 6, 2, 3 + 6),
        "shape 1 (weight 0) is left out: {posing:?}"
    );
    // Nor is its data taken up: the arena holds only the active shapes' moves.
    assert_eq!(renderer.scene.posing.arena_moves(), 3 + 6);
    // At weight 0 everywhere, the joints still pose it and no shape costs.
    renderer.render(&list_of(vec![strip(&[0.0; 3])])).unwrap();
    let posing = renderer.stats().posing;
    assert_eq!(
        (posing.items, posing.shapes, posing.moves),
        (1, 0, 0),
        "{posing:?}"
    );
    assert!(posing.bytes > 0, "{posing:?}");
}

#[test]
fn a_shape_moves_a_jointed_mesh_before_its_joints() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 32, 32);
    let middle = |renderer: &mut TextureRenderer, list: &DrawList| {
        renderer.render(list).unwrap();
        let pixels = renderer.read_pixels();
        let at = (16 * 32 + 16) * 4;
        [pixels[at], pixels[at + 1], pixels[at + 2]]
    };
    let sky = middle(&mut renderer, &list_of(vec![morphed_skinned_square(0.0)]));
    let drawn = middle(&mut renderer, &list_of(vec![morphed_skinned_square(1.0)]));
    assert!(
        drawn.iter().zip(sky).any(|(a, b)| a.abs_diff(b) > 30),
        "the shape brings the square into view: {drawn:?} vs sky {sky:?}"
    );
}

/// [`ShapedStrip`] under another key: a mesh of its own to the renderer.
struct KeyedStrip(u64);

impl MeshSource for KeyedStrip {
    fn key(&self) -> u64 {
        self.0
    }
    fn positions(&self) -> &[[f32; 3]] {
        ShapedStrip.positions()
    }
    fn normals(&self) -> &[[f32; 3]] {
        ShapedStrip.normals()
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &[]
    }
    fn tangents(&self) -> &[[f32; 4]] {
        ShapedStrip.tangents()
    }
    fn indices(&self) -> &[u32] {
        ShapedStrip.indices()
    }
    fn skin_joints(&self) -> &[[u16; 4]] {
        ShapedStrip.skin_joints()
    }
    fn skin_weights(&self) -> &[[f32; 4]] {
        ShapedStrip.skin_weights()
    }
    fn morph_count(&self) -> usize {
        ShapedStrip.morph_count()
    }
    fn morph(&self, index: usize) -> Option<crate::MorphView<'_>> {
        match index {
            0 => Some(crate::MorphView {
                target: 0,
                corners: &SHAPE_0_CORNERS,
                positions: &SHAPE_0_MOVES,
                normals: &SHAPE_0_TURNS,
            }),
            2 => Some(crate::MorphView {
                target: 2,
                corners: &SHAPE_2_CORNERS,
                positions: &SHAPE_2_MOVES,
                normals: &[],
            }),
            _ => None,
        }
    }
}

#[test]
fn items_of_many_meshes_are_posed_together_as_the_arena_fills_afresh() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 32, 32);
    let keyed = |key: u64, weights: &[f32]| DrawItem {
        shape: Shape::Mesh(Arc::new(KeyedStrip(key))),
        ..strip(weights)
    };
    // One frame of a few meshes, then one of many more than the posing
    // arena held (it fills afresh), then the first again.
    let few: Vec<DrawItem> = (0..3)
        .map(|i| keyed(1000 + i, &[0.5, 0.0, -0.75]))
        .collect();
    let many: Vec<DrawItem> = (0..40)
        .map(|i| keyed(2000 + i, &[0.25 * (i % 4) as f32, 0.0, 0.5]))
        .chain([strip(&[0.5, 0.0, -0.75]), morphed_skinned_square(1.0)])
        .collect();
    for list in [list_of(few.clone()), list_of(many), list_of(few)] {
        renderer.render(&list).unwrap();
        assert_eq!(renderer.stats().posing.items as usize, list.items.len());
        for item in &list.items {
            let Shape::Mesh(mesh) = &item.shape else {
                unreachable!()
            };
            let skin = item.skin.as_ref().unwrap();
            let posed = renderer
                .scene
                .read_posed(skin, MeshKey::Custom(mesh.key()))
                .expect("posed");
            for (corner, (posed, expected)) in posed
                .iter()
                .zip(per_pass_pose(mesh.as_ref(), skin))
                .enumerate()
            {
                assert!(
                    posed[0].distance(expected[0]) < 1e-4 && posed[1].distance(expected[1]) < 2e-3,
                    "mesh {} corner {corner}: {posed:?} vs {expected:?}",
                    mesh.key()
                );
            }
        }
    }
}
