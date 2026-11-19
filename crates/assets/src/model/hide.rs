//! A model with some of its triangles left out: how a body's hidden
//! regions are made (`hidden.rs`, CO9).

use super::{Model, Primitive, new_key};

impl Model {
    /// A copy without the triangles `hide` says to leave out (it is given
    /// the middle of each triangle's texture coordinates; a primitive
    /// without them keeps every triangle), at every level of detail. The
    /// corners, shapes and skeleton are this model's; its key and its
    /// primitives' keys are new.
    pub fn without_triangles(&self, hide: &dyn Fn([f32; 2]) -> bool) -> Model {
        self.without_triangles_where(&|at: TriangleAt| at.uv.is_some_and(hide))
    }

    /// A copy without the triangles `hide` says to leave out, told each
    /// triangle's [`TriangleAt`] (wave 5: a lower layer without texture
    /// coordinates is masked by its triangles' numbers).
    pub fn without_triangles_where(&self, hide: &dyn Fn(TriangleAt) -> bool) -> Model {
        let mut start = 0u32;
        Model {
            key: new_key(),
            size: self.size,
            center: self.center,
            primitives: self
                .primitives
                .iter()
                .map(|primitive| {
                    let first = start;
                    start += (primitive.indices.len() / 3) as u32;
                    kept(primitive, Some(first), hide)
                })
                .collect(),
            hull_points: self.hull_points.clone(),
            collision: self.collision.clone(),
            warnings: Vec::new(),
            rig: self.rig.clone(),
            morph_targets: self.morph_targets.clone(),
            identity_pose: self.identity_pose.clone(),
        }
    }
}

/// Where a triangle is, for [`Model::without_triangles_where`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleAt {
    /// The middle of its texture coordinates (`None` when its primitive
    /// has none).
    pub uv: Option<[f32; 2]>,
    /// Its number over the full-detail model's primitives in order
    /// (`None` in a simpler copy, a level of detail).
    pub index: Option<u32>,
}

/// `primitive` (and its simpler copies) without the triangles `hide`
/// names. `first` is the number of its first triangle over the model (a
/// full-detail primitive's).
fn kept(primitive: &Primitive, first: Option<u32>, hide: &dyn Fn(TriangleAt) -> bool) -> Primitive {
    let with_uvs = primitive.uvs.len() == primitive.positions.len();
    let indices = primitive
        .indices
        .chunks_exact(3)
        .enumerate()
        .filter(|(k, t)| {
            let uv = with_uvs.then(|| {
                t.iter().fold([0.0f32; 2], |sum, &v| {
                    let uv = primitive.uvs[v as usize];
                    [sum[0] + uv[0] / 3.0, sum[1] + uv[1] / 3.0]
                })
            });
            let index = first.map(|first| first + *k as u32);
            !hide(TriangleAt { uv, index })
        })
        .flat_map(|(_, t)| t)
        .copied()
        .collect();
    Primitive {
        key: new_key(),
        positions: primitive.positions.clone(),
        normals: primitive.normals.clone(),
        uvs: primitive.uvs.clone(),
        tangents: primitive.tangents.clone(),
        regions: primitive.regions.clone(),
        indices,
        material: primitive.material.clone(),
        joints: primitive.joints.clone(),
        weights: primitive.weights.clone(),
        morphs: primitive.morphs.clone(),
        lods: primitive
            .lods
            .iter()
            .map(|lod| kept(lod, None, hide))
            .collect(),
        level: primitive.level,
    }
}
