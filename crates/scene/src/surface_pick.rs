//! Rays against characters as they are drawn (Character Studio refinement
//! R2, direct manipulation): the posed, skinned mesh with its live
//! identity's shapes (ADR-0041), not the part's box that [`crate::pick`]
//! tests. Character Studio picks a feature under the pointer with it and
//! draws handles on the skin; a game could pick a body part the same way.
//!
//! A pose is morphed and skinned on the CPU the first time a ray asks for
//! it ([`Posed::primitive`]), and kept with that pose: every ray against
//! one frame's pose shares one skinning, and the next frame's pose (or a
//! changed identity) makes a new one. Without a pose of the model the rest
//! mesh is tested.
//!
//! Points are in the world; a model's own points are in its fitted space
//! (`assets`' `Rig::fit`), where the file's metres are scaled into a unit
//! box. An anchor ([`rest_anchor`]) is a bone and an offset from its head
//! in the file's space at rest, as the character manifests measure them.

use std::sync::Arc;

use axcenity_assets::{Assets, Model, ModelShading, Posed};
use axcenity_core::{InstanceId, Value, World};
use axcenity_render::glam::{Mat3, Mat4, Vec3};

use crate::draw_list::{Poses, part_placement};
use crate::pick::Ray;

/// One corner of a model: its primitive, and its index there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Corner {
    pub primitive: usize,
    pub index: usize,
}

/// Where a ray meets a model's surface as it is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshHit {
    /// Along the ray, in world units.
    pub distance: f32,
    pub point: Vec3,
    /// The triangle's own normal in the world, facing out (its corners go
    /// counter-clockwise seen from outside).
    pub normal: Vec3,
    pub primitive: usize,
    /// The triangle's index (its corners are `indices[3 × triangle ..]`).
    pub triangle: usize,
    /// How much of each of the triangle's three corners makes the point.
    pub weights: [f32; 3],
    /// The body region hit (the build's region map,
    /// `assets::Primitive::regions`), when the model has one.
    pub region: Option<u8>,
}

impl MeshHit {
    /// The triangle's three corners.
    pub fn corners(&self, model: &Model) -> Option<[Corner; 3]> {
        let indices = &model.primitives.get(self.primitive)?.indices;
        let at = self.triangle * 3;
        let triangle = indices.get(at..at + 3)?;
        Some([0, 1, 2].map(|k| Corner {
            primitive: self.primitive,
            index: triangle[k] as usize,
        }))
    }

    /// The same place on the model at rest, in its fitted space: the hit's
    /// blend of the triangle's corners as they were built. Neither the pose
    /// nor the identity changes it, so it can be compared with anchors
    /// measured at rest (which feature was hit).
    pub fn at_rest(&self, model: &Model) -> Option<Vec3> {
        let positions = &model.primitives.get(self.primitive)?.positions;
        let corners = self.corners(model)?;
        let mut point = Vec3::ZERO;
        for (corner, weight) in corners.iter().zip(self.weights) {
            point += Vec3::from(*positions.get(corner.index)?) * weight;
        }
        Some(point)
    }
}

/// A MeshPart a ray met, and where.
#[derive(Clone, Debug)]
pub struct PartHit {
    pub part: InstanceId,
    pub model: Arc<Model>,
    pub hit: MeshHit,
}

/// `model`'s corners in primitive `primitive` as `posed` draws them (a pose
/// of another model counts as none), else at rest; in its fitted space.
fn drawn<'a>(model: &'a Model, posed: Option<&'a Posed>, primitive: usize) -> &'a [[f32; 3]] {
    match posed.filter(|posed| posed.model_key == model.key()) {
        Some(posed) => &posed.primitive(model, primitive, 0).positions,
        None => model
            .primitives
            .get(primitive)
            .map_or(&[], |primitive| primitive.positions.as_slice()),
    }
}

/// The normals of [`drawn`]'s corners.
fn drawn_normals<'a>(
    model: &'a Model,
    posed: Option<&'a Posed>,
    primitive: usize,
) -> &'a [[f32; 3]] {
    match posed.filter(|posed| posed.model_key == model.key()) {
        Some(posed) => &posed.primitive(model, primitive, 0).normals,
        None => model
            .primitives
            .get(primitive)
            .map_or(&[], |primitive| primitive.normals.as_slice()),
    }
}

/// Where `ray` first meets `model` posed by `posed` (at rest without one)
/// and placed in the world by `transform`. Both sides of a triangle count.
pub fn ray_model(
    model: &Model,
    posed: Option<&Posed>,
    transform: Mat4,
    ray: Ray,
) -> Option<MeshHit> {
    let to_model = transform.inverse();
    if !to_model.is_finite() {
        return None;
    }
    // The ray in the model's space. Its direction isn't normalized there,
    // so a distance along it is still one in the world.
    let origin = to_model.transform_point3(ray.origin);
    let direction = to_model.transform_vector3(ray.direction);
    let mut best: Option<MeshHit> = None;
    for (index, primitive) in model.primitives.iter().enumerate() {
        let positions = drawn(model, posed, index);
        for (triangle, corners) in primitive.indices.as_chunks::<3>().0.iter().enumerate() {
            let [a, b, c] = corners.map(|corner| {
                positions
                    .get(corner as usize)
                    .copied()
                    .map_or(Vec3::NAN, Vec3::from)
            });
            let Some((distance, u, v)) = triangle_hit(origin, direction, a, b, c) else {
                continue;
            };
            if best.is_some_and(|best| best.distance <= distance) {
                continue;
            }
            let region = primitive.regions.get(corners[0] as usize).copied();
            best = Some(MeshHit {
                distance,
                point: ray.at(distance),
                normal: (b - a).cross(c - a),
                primitive: index,
                triangle,
                weights: [1.0 - u - v, u, v],
                region,
            });
        }
    }
    let turn = Mat3::from_mat4(transform).inverse().transpose();
    best.map(|mut hit| {
        hit.normal = (turn * hit.normal).normalize_or_zero();
        hit
    })
}

/// Möller and Trumbore's ray and triangle test: the distance along the ray
/// and the hit's share of `b` and `c`, or `None` for a miss (or a hit
/// behind the origin).
pub fn triangle_hit(
    origin: Vec3,
    direction: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<(f32, f32, f32)> {
    let (ab, ac) = (b - a, c - a);
    let p = direction.cross(ac);
    let det = ab.dot(p);
    if !det.is_finite() || det.abs() < 1e-12 {
        return None;
    }
    let inverse = 1.0 / det;
    let from_a = origin - a;
    let u = from_a.dot(p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = from_a.cross(ab);
    let v = direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = ac.dot(q) * inverse;
    (distance > 1e-6).then_some((distance, u, v))
}

/// Where `ray` first meets any of the MeshParts `parts` as they're drawn:
/// each posed as `poses` has it (the frame's pose, with its live identity),
/// at rest otherwise. Parts that aren't MeshParts with a model are skipped.
pub fn ray_parts(
    world: &World,
    assets: &Assets,
    poses: &Poses,
    parts: &[InstanceId],
    ray: Ray,
) -> Option<PartHit> {
    let mut best: Option<PartHit> = None;
    for &part in parts {
        let Some(instance) = world.get(part) else {
            continue;
        };
        let Some((_, transform)) = part_placement(instance) else {
            continue;
        };
        let Some(model) = instance
            .property("MeshId")
            .and_then(Value::as_str)
            .and_then(|mesh| assets.model_blocking(mesh))
        else {
            continue;
        };
        let posed = poses.get(&part).map(|posed| posed.as_ref());
        let Some(hit) = ray_model(&model, posed, transform, ray) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|best| hit.distance < best.hit.distance)
        {
            best = Some(PartHit { part, model, hit });
        }
    }
    best
}

/// Where a MeshPart (or Part) puts its model in the world: from its fitted
/// space by its `Size`, `Orientation` and `Position`, as it's drawn.
pub fn placement(world: &World, part: InstanceId) -> Option<Mat4> {
    part_placement(world.get(part)?).map(|(_, transform)| transform)
}

/// How many fitted units a metre (a unit of the model's file) is.
pub fn fitted_per_metre(model: &Model) -> Option<f32> {
    let rig = model.rig.as_ref()?;
    Some(rig.fit.transform_vector3(Vec3::X).length()).filter(|scale| scale.is_finite())
}

/// What turns a step in `model`'s fitted space into the file's metres,
/// axis by axis. The fitted box scales each axis by its own size (a body
/// is much wider, arms out, than deep), so a distance between fitted
/// points is only a distance once multiplied by this.
pub fn to_metres(model: &Model) -> Vec3 {
    let size = Vec3::from(model.size);
    Vec3::select(size.cmpgt(Vec3::splat(1e-6)), size, Vec3::ONE)
}

/// An anchor (a bone, and an offset from its head in the file's space at
/// rest, metres) in `model`'s fitted space at rest. The head is where the
/// skin was bound to the bone (its inverse bind), as the build measured it;
/// a bone no corner is bound to rests where the rig puts it.
pub fn rest_anchor(model: &Model, bone: &str, offset: Vec3) -> Option<Vec3> {
    let rig = model.rig.as_ref()?;
    let index = rig.bone(bone)?;
    let head = match rig.joints.iter().find(|joint| joint.bone == index) {
        Some(joint) => joint.inverse_bind.inverse().transform_point3(Vec3::ZERO),
        None => rig.world(&rig.rest_pose())[index].transform_point3(Vec3::ZERO),
    };
    Some(rig.fit.transform_point3(head + offset)).filter(|point| point.is_finite())
}

/// The corner of `model`'s full-detail mesh nearest `point` (its fitted
/// space, at rest): the skin a handle anchored there sits on. A model with
/// skin-shaded primitives (a character's body) answers with a corner of its
/// skin: where the skin meets the inside of the mouth or the eye sockets
/// the two share positions, and only the skin's corner faces out. Nearest
/// in the file's metres ([`to_metres`]), not in the fitted box.
pub fn nearest_corner(model: &Model, point: Vec3) -> Option<Corner> {
    let skin = |mesh: &axcenity_assets::Primitive| mesh.material.shading == ModelShading::Skin;
    let any_skin = model.primitives.iter().any(skin);
    let metres = to_metres(model);
    let mut best: Option<(f32, Corner)> = None;
    for (primitive, mesh) in model.primitives.iter().enumerate() {
        if any_skin && !skin(mesh) {
            continue;
        }
        for (index, position) in mesh.positions.iter().enumerate() {
            let distance = ((Vec3::from(*position) - point) * metres).length_squared();
            if best.is_none_or(|(nearest, _)| distance < nearest) {
                best = Some((distance, Corner { primitive, index }));
            }
        }
    }
    best.map(|(_, corner)| corner)
}

/// Where `corner` is drawn in the world, and its normal there: posed by
/// `posed` (at rest without a pose of the model) and placed by `transform`.
pub fn corner_drawn(
    model: &Model,
    posed: Option<&Posed>,
    transform: Mat4,
    corner: Corner,
) -> Option<(Vec3, Vec3)> {
    let position = Vec3::from(*drawn(model, posed, corner.primitive).get(corner.index)?);
    let normal = drawn_normals(model, posed, corner.primitive)
        .get(corner.index)
        .map_or(Vec3::ZERO, |normal| Vec3::from(*normal));
    let turn = Mat3::from_mat4(transform).inverse().transpose();
    Some((
        transform.transform_point3(position),
        (turn * normal).normalize_or_zero(),
    ))
}

/// Every corner of `model`'s primitive `primitive` as drawn, in the world
/// (for an overlay that follows the skin, such as a highlight).
pub fn corners_drawn(
    model: &Model,
    posed: Option<&Posed>,
    transform: Mat4,
    primitive: usize,
) -> Vec<Vec3> {
    drawn(model, posed, primitive)
        .iter()
        .map(|position| transform.transform_point3(Vec3::from(*position)))
        .collect()
}

/// The normals of [`corners_drawn`]'s corners, in the world (unit length).
pub fn normals_drawn(
    model: &Model,
    posed: Option<&Posed>,
    transform: Mat4,
    primitive: usize,
) -> Vec<Vec3> {
    let turn = Mat3::from_mat4(transform).inverse().transpose();
    drawn_normals(model, posed, primitive)
        .iter()
        .map(|normal| (turn * Vec3::from(*normal)).normalize_or_zero())
        .collect()
}

#[cfg(test)]
#[path = "surface_pick_tests.rs"]
mod tests;
