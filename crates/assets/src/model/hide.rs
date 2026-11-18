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
        Model {
            key: new_key(),
            size: self.size,
            center: self.center,
            primitives: self
                .primitives
                .iter()
                .map(|primitive| kept(primitive, hide))
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

/// `primitive` (and its simpler copies) without the triangles `hide`
/// names.
fn kept(primitive: &Primitive, hide: &dyn Fn([f32; 2]) -> bool) -> Primitive {
    let indices = if primitive.uvs.len() == primitive.positions.len() {
        primitive
            .indices
            .chunks_exact(3)
            .filter(|t| {
                let uv = t.iter().fold([0.0f32; 2], |sum, &v| {
                    let uv = primitive.uvs[v as usize];
                    [sum[0] + uv[0] / 3.0, sum[1] + uv[1] / 3.0]
                });
                !hide(uv)
            })
            .flatten()
            .copied()
            .collect()
    } else {
        primitive.indices.clone()
    };
    Primitive {
        key: new_key(),
        positions: primitive.positions.clone(),
        normals: primitive.normals.clone(),
        uvs: primitive.uvs.clone(),
        tangents: primitive.tangents.clone(),
        indices,
        material: primitive.material.clone(),
        joints: primitive.joints.clone(),
        weights: primitive.weights.clone(),
        morphs: primitive.morphs.clone(),
        lods: primitive.lods.iter().map(|lod| kept(lod, hide)).collect(),
        level: primitive.level,
    }
}
