//! Fitting garments to a base body (Character Studio overhaul CO9,
//! ADR-0035, ADR-0041 decision 4; `docs/development/character-studio/
//! architecture.md` section 9).
//!
//! A garment made in the body's rest pose is **bound** to the body's
//! surface: each of its vertices to the closest point of a body triangle
//! (the triangle, the point's weights of its corners, and the vertex's
//! offset in the frame the triangle's normal and first edge make there,
//! [`bind`]). Through the binding the garment then takes, at build or
//! import time ([`fit_garment`]):
//!
//! - the body's **skin weights** at its bound points, so it poses with the
//!   same bones;
//! - every **identity and corrective shape** of the body, as shapes of its
//!   own with the same names, so the runtime drives them with the body's
//!   live weights (architecture section 14: a part takes the body's weight
//!   of every shape of the same name) and nothing is fitted at run time;
//! - **push-out**: wherever the garment would be inside the body, or
//!   inside a lower layer, closer than its margin, at rest, at each
//!   shape's full weight, or at an extreme combination of parameters, it
//!   is moved out (and the move spread to its neighbours, so it stays
//!   smooth). What each layer pushed is in the [`FitReport`].
//!
//! [`coverage`] says which body regions a garment covers, so an item can
//! hide them; [`import`] is the whole import flow for a third-party
//! garment (align, classify, fit, thumbnail, write into a library).
//!
//! Everything works in the model files' space (metres, Y up, the
//! character facing +Z), not the engine's fitted 1×1×1 space.

mod bind;
mod coverage;
mod enclosure;
mod glb;
mod hiding;
mod import;
mod pose;
mod posed;
mod surface;
mod thumbnail;
mod transfer;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use axcenity_assets::{Model, Rig};
use glam::{Mat3, Mat4, Vec3};

pub use bind::{Binding, Bound, bind, place};
pub use coverage::{
    Coverage, HIDE_SHARE, RegionMap, SlotGuess, classify, coverage, hidden_regions,
    triangle_regions,
};
pub use enclosure::{
    BEHIND, BODY_MASK, FINE_GRAZE, GRAZE, HIDE_RULE, HideCase, HideInput, HideReport, HideRule,
    LAYER_MASK, NUMBERED_WIDTH, Openings, Placed, SeenHole, Views, encloses, hidden_under,
    hide_under, holes_seen, layer_masks, mask_texels, numbered_texels, shows, shows_at,
    view_directions, write_hide_maps, write_mask,
};
pub use glb::{FittedModel, write_fitted_glb};
pub use hiding::{CREASE_ACROSS, HOLE, HOLE_REACH, outside_cloth, pose_safe_hides};
pub use import::{
    Alignment, ImportOptions, ImportRequest, Imported, align, extremes_of, import_garment,
    import_garment_with, pose_states_of,
};
pub use pose::{
    CORRECTIVE_RULES, CorrectiveRule, FinishPose, FitPose, corrective_weights, fit_poses,
    hide_poses, skin, skin_matrices,
};
pub use posed::PoseReport;
pub use surface::{Hit, Surface, closest_on_triangle, vertex_normals};
pub use thumbnail::{thumbnail, write_png};
pub use transfer::{
    FitOptions, FitReport, Fitted, Layer, LayerOffset, carried_shapes, extremes_near, fit_garment,
    transfer_weights,
};

/// Why a garment couldn't be fitted or imported.
#[derive(Debug, thiserror::Error)]
pub enum FitError {
    #[error("the body model has no skeleton to weight a garment to")]
    NoSkeleton,
    #[error("the garment has no triangles")]
    Empty,
    #[error("{0}")]
    Model(String),
    #[error("{0}")]
    Library(String),
    #[error("couldn't write `{path}`: {message}")]
    Write { path: String, message: String },
}

/// Shapes the transfer never carries: a body's mask shapes sink its skin
/// under a worn garment, and would sink the garment with it.
pub fn is_mask_shape(name: &str) -> bool {
    name.starts_with("Mask_")
}

/// The matrix from a model's file space into its fitted 1×1×1 space (the
/// rig's, or the box's for a model without one).
pub fn fitted_from_file(model: &Model) -> Mat4 {
    if let Some(rig) = &model.rig {
        return rig.fit;
    }
    let size = Vec3::from(model.size);
    let center = Vec3::from(model.center);
    // `fit_with` leaves a flat axis unscaled.
    let scale = Vec3::select(size.cmpgt(Vec3::splat(1e-6)), size.recip(), Vec3::ONE);
    Mat4::from_scale(scale) * Mat4::from_translation(-center)
}

/// One blend shape of a body, in its file space: the corners it moves,
/// how far, and how their normals change (each normal with the change
/// added is one unit long again).
#[derive(Clone, Debug, PartialEq)]
pub struct BodyShape {
    pub name: String,
    pub corners: Vec<u32>,
    pub offsets: Vec<Vec3>,
    pub normals: Vec<Vec3>,
}

impl BodyShape {
    /// Its offsets as one per corner of a mesh of `count` corners.
    pub fn dense(&self, count: usize) -> Vec<Vec3> {
        let mut out = vec![Vec3::ZERO; count];
        for (&corner, &offset) in self.corners.iter().zip(&self.offsets) {
            out[corner as usize] = offset;
        }
        out
    }

    /// The farthest it moves a corner.
    pub fn reach(&self) -> f32 {
        self.offsets.iter().map(|o| o.length()).fold(0.0, f32::max)
    }
}

/// A body as fitting sees it: every full-detail primitive of its model as
/// one mesh in the file's space, with its skin weights and blend shapes.
#[derive(Clone, Debug)]
pub struct BodyMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    /// The first texture coordinates, one per corner (empty without).
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    /// Up to four joints of [`Rig::joints`] per corner, and their weights.
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub shapes: Vec<BodyShape>,
    pub rig: Arc<Rig>,
}

impl BodyMesh {
    /// The body of `model` (a skinned model: the base model's body).
    pub fn from_model(model: &Model) -> Result<BodyMesh, FitError> {
        let rig = model.rig.clone().ok_or(FitError::NoSkeleton)?;
        let fitted = fitted_from_file(model);
        let unfit = fitted.inverse();
        // Normals turn by the inverse transpose: from the fitted space
        // back, that is the fit's own transpose.
        let turn = Mat3::from_mat4(fitted).transpose();
        let mut body = BodyMesh {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            triangles: Vec::new(),
            joints: Vec::new(),
            weights: Vec::new(),
            shapes: model
                .morph_targets
                .iter()
                .map(|target| BodyShape {
                    name: target.name.clone(),
                    corners: Vec::new(),
                    offsets: Vec::new(),
                    normals: Vec::new(),
                })
                .collect(),
            rig,
        };
        let with_uvs = model
            .primitives
            .iter()
            .all(|p| p.uvs.len() == p.positions.len());
        for primitive in &model.primitives {
            let start = body.positions.len() as u32;
            let count = primitive.positions.len();
            body.positions.extend(
                primitive
                    .positions
                    .iter()
                    .map(|p| unfit.transform_point3(Vec3::from(*p))),
            );
            let normals: Vec<Vec3> = (0..count)
                .map(|i| {
                    primitive
                        .normals
                        .get(i)
                        .map_or(Vec3::ZERO, |n| (turn * Vec3::from(*n)).normalize_or_zero())
                })
                .collect();
            body.normals.extend(&normals);
            if with_uvs {
                body.uvs.extend(&primitive.uvs);
            }
            body.triangles.extend(
                primitive
                    .indices
                    .chunks_exact(3)
                    .map(|t| [t[0] + start, t[1] + start, t[2] + start]),
            );
            for vertex in 0..count {
                body.joints
                    .push(primitive.joints.get(vertex).copied().unwrap_or_default());
                body.weights
                    .push(primitive.weights.get(vertex).copied().unwrap_or_default());
            }
            for morph in &primitive.morphs {
                let Some(shape) = body.shapes.get_mut(usize::from(morph.target)) else {
                    continue;
                };
                for (k, &corner) in morph.corners.iter().enumerate() {
                    let offset = morph.positions.get(k).copied().unwrap_or_default();
                    shape.corners.push(corner + start);
                    shape
                        .offsets
                        .push(unfit.transform_vector3(Vec3::from(offset)));
                    // The normal's change, in the file's space, at full
                    // weight.
                    let rest = primitive
                        .normals
                        .get(corner as usize)
                        .map_or(Vec3::ZERO, |n| Vec3::from(*n));
                    let change = morph.normals.get(k).map_or(Vec3::ZERO, |n| Vec3::from(*n));
                    let moved = (turn * (rest + change)).normalize_or_zero();
                    shape.normals.push(moved - normals[corner as usize]);
                }
            }
        }
        if body.triangles.is_empty() {
            return Err(FitError::Model("the body has no triangles".to_owned()));
        }
        Ok(body)
    }

    /// The index of the shape `name`.
    pub fn shape_index(&self, name: &str) -> Option<usize> {
        self.shapes.iter().position(|shape| shape.name == name)
    }

    /// Positions and normals with `weights` of its shapes (by name; names
    /// it hasn't are skipped).
    pub fn shaped(&self, weights: &[(String, f32)]) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut positions = self.positions.clone();
        let mut normals = self.normals.clone();
        for (name, weight) in weights {
            if *weight == 0.0 {
                continue;
            }
            let Some(index) = self.shape_index(name) else {
                continue;
            };
            let shape = &self.shapes[index];
            for k in 0..shape.corners.len() {
                let corner = shape.corners[k] as usize;
                positions[corner] += shape.offsets[k] * *weight;
                normals[corner] += shape.normals[k] * *weight;
            }
        }
        for normal in &mut normals {
            *normal = normal.normalize_or_zero();
        }
        (positions, normals)
    }

    /// Its surface with `weights` of its shapes.
    pub fn surface(&self, weights: &[(String, f32)]) -> Surface {
        let (positions, normals) = self.shaped(weights);
        Surface::new(positions, normals, self.triangles.clone())
    }
}

/// One primitive of a garment: a material and its own corners.
#[derive(Clone, Debug, PartialEq)]
pub struct GarmentPiece {
    /// The material's name in the file.
    pub material: String,
    /// Linear RGBA.
    pub colour: [f32; 4],
    pub roughness: f32,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    /// Empty when it has none.
    pub uvs: Vec<[f32; 2]>,
    /// Three per triangle, counter-clockwise seen from outside.
    pub indices: Vec<u32>,
}

/// A garment's mesh in the file's space: its pieces, one per material.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GarmentMesh {
    pub pieces: Vec<GarmentPiece>,
}

impl GarmentMesh {
    /// The full-detail primitives of `model`, back in its file's space.
    pub fn from_model(model: &Model) -> GarmentMesh {
        let fitted = fitted_from_file(model);
        let unfit = fitted.inverse();
        let turn = Mat3::from_mat4(fitted).transpose();
        let pieces = model
            .primitives
            .iter()
            .filter(|primitive| !primitive.indices.is_empty())
            .map(|primitive| GarmentPiece {
                material: primitive.material.name.clone(),
                colour: primitive.material.base_color,
                roughness: primitive.material.roughness,
                positions: primitive
                    .positions
                    .iter()
                    .map(|p| unfit.transform_point3(Vec3::from(*p)))
                    .collect(),
                normals: primitive
                    .normals
                    .iter()
                    .map(|n| (turn * Vec3::from(*n)).normalize_or_zero())
                    .collect(),
                uvs: primitive.uvs.clone(),
                indices: primitive.indices.clone(),
            })
            .collect();
        GarmentMesh { pieces }
    }

    /// Every piece's corners, one after another.
    pub fn positions(&self) -> Vec<Vec3> {
        self.pieces
            .iter()
            .flat_map(|p| p.positions.iter().copied())
            .collect()
    }

    /// Every piece's normals, one after another (made from the triangles
    /// where a piece has none).
    pub fn normals(&self) -> Vec<Vec3> {
        let mut out = Vec::new();
        for piece in &self.pieces {
            if piece.normals.len() == piece.positions.len() {
                out.extend(&piece.normals);
            } else {
                let triangles: Vec<[u32; 3]> = piece
                    .indices
                    .chunks_exact(3)
                    .map(|t| [t[0], t[1], t[2]])
                    .collect();
                out.extend(vertex_normals(&piece.positions, &triangles));
            }
        }
        out
    }

    /// Every piece's triangles over [`positions`](Self::positions).
    pub fn triangles(&self) -> Vec<[u32; 3]> {
        let mut out = Vec::new();
        let mut start = 0;
        for piece in &self.pieces {
            out.extend(
                piece
                    .indices
                    .chunks_exact(3)
                    .map(|t| [t[0] + start, t[1] + start, t[2] + start]),
            );
            start += piece.positions.len() as u32;
        }
        out
    }

    /// How many corners all its pieces have.
    pub fn len(&self) -> usize {
        self.pieces.iter().map(|p| p.positions.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Its box: (least, most) corner.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        self.pieces
            .iter()
            .flat_map(|p| &p.positions)
            .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            })
    }

    /// Sets every piece's corners from `positions` (as
    /// [`positions`](Self::positions) orders them).
    pub fn set_positions(&mut self, positions: &[Vec3]) {
        let mut at = 0;
        for piece in &mut self.pieces {
            let count = piece.positions.len();
            piece.positions.copy_from_slice(&positions[at..at + count]);
            at += count;
        }
    }

    /// Sets every piece's normals from `normals` (as
    /// [`normals`](Self::normals) orders them).
    pub fn set_normals(&mut self, normals: &[Vec3]) {
        let mut at = 0;
        for piece in &mut self.pieces {
            let count = piece.positions.len();
            piece.normals = normals[at..at + count].to_vec();
            at += count;
        }
    }

    /// Moves and scales it: each corner `p` becomes `p * scale + offset`.
    pub fn transform(&mut self, scale: f32, offset: Vec3) {
        for piece in &mut self.pieces {
            for p in &mut piece.positions {
                *p = *p * scale + offset;
            }
        }
    }
}
