//! The posed fit (CO9 review fixes 2): a garment fitted at rest and at
//! every shape state stands off the skin across a joint (a T-shirt's
//! underarm, its shoulder), and there linear skinning moves the cloth and
//! the skin under it differently. In each [`FitPose`](super::FitPose) a
//! body corrective leans on, on a few body states (the frames, the build's
//! corners, the body presets), each garment corner should be where its
//! bound point of the posed skin and the frame there put it, as at rest;
//! what skinning leaves between goes into the garment's copies of the
//! leaning correctives on its side, so at run time the cloth follows the
//! skin it is bound to with the body's own live corrective weights.
//!
//! (Pushing the cloth out where the skin came through in a pose, into
//! those correctives or its rest, was tried as well: where two covered
//! parts come together, a hanging arm against the side or the front of the
//! armpit as the arm swings back, the pushes for one side are into the
//! other, and the fit gate got worse in every pose. See
//! `docs/development/character-studio/refinement/wave3/co9-binding.md`,
//! Review fixes 2.)

use std::collections::BTreeMap;

use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

use super::BodyMesh;
use super::bind::{Binding, frames};
use super::pose::{FitPose, skin, skin_matrices};
use super::transfer::{CarriedShape, garment_at, par_map};

/// A corrective leaning less than this in a pose takes none of the gap.
const LEANING: f32 = 0.05;

/// What the posed fit did.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoseReport {
    /// Poses and body states it fitted in (each pose on each state).
    pub poses: usize,
    pub states: usize,
    /// Corrective shapes the cloth's following of the skin it is bound to
    /// went into, and the farthest any of them now moves a corner
    /// (metres).
    pub correctives_followed: usize,
    pub follow_most: f32,
}

/// The garment being fitted: its binding and its corners' skin.
pub(super) struct Garment<'a> {
    pub binding: &'a Binding,
    pub joints: &'a [[u16; 4]],
    pub weights: &'a [[f32; 4]],
}

pub(super) fn solve(
    body: &BodyMesh,
    garment: &Garment,
    poses: &[FitPose],
    states: &[Vec<(String, f32)>],
    rest: &[Vec3],
    shapes: &mut [CarriedShape],
) -> PoseReport {
    let mut report = PoseReport {
        poses: poses.len(),
        states: states.len(),
        ..PoseReport::default()
    };
    if poses.is_empty() || states.is_empty() {
        return report;
    }
    // Which side of the body each corrective's bone is on (+1 or -1 along
    // X), so a corner's gap goes into its own side's correctives only.
    let world = body.rig.world(&body.rig.rest_pose());
    let side_of = |shape: &str| -> f32 {
        super::pose::CORRECTIVE_RULES
            .iter()
            .find(|rule| rule.0 == shape)
            .and_then(|rule| body.rig.bone(rule.1))
            .map_or(0.0, |bone| world[bone].w_axis.x.signum())
    };
    let (followed, most) = follow(body, garment, poses, states, &side_of, rest, shapes);
    report.correctives_followed = followed;
    report.follow_most = most;
    report
}

/// How many rounds the garment's correctives are fitted to follow its
/// binding.
const FOLLOW_ROUNDS: usize = 4;

/// First, the cloth follows the skin it is bound to: in each pose a
/// corrective leans on, each corner should be where its bound point of
/// the posed skin and the frame there put it (as at rest), and what linear
/// skinning leaves between (in the pose, less what is between at rest on
/// the same body, taken back to rest) goes into the leaning correctives on
/// its side, the mean over the poses and body states. Then, in a pose, the
/// cloth over a joint moves as the skin under it does. Returns how many
/// correctives took a share, and the farthest one now moves a corner.
fn follow(
    body: &BodyMesh,
    garment: &Garment,
    poses: &[FitPose],
    states: &[Vec<(String, f32)>],
    side_of: &dyn Fn(&str) -> f32,
    rest: &[Vec3],
    shapes: &mut [CarriedShape],
) -> (usize, f32) {
    let cases: Vec<(usize, usize)> = (0..poses.len())
        .filter(|p| poses[*p].correctives.iter().any(|(_, w)| *w > LEANING))
        .flat_map(|p| (0..states.len()).map(move |s| (p, s)))
        .collect();
    let mut moved: BTreeMap<String, f32> = BTreeMap::new();
    for _ in 0..FOLLOW_ROUNDS {
        let gaps = par_map(&cases, |&(p, s)| {
            let at_rest = gap(body, garment, None, &states[s], rest, shapes);
            let weights = with(&states[s], &poses[p].correctives);
            let posed = gap(body, garment, Some(&poses[p]), &weights, rest, shapes);
            posed
                .into_iter()
                .zip(at_rest)
                .map(|(a, b)| a - b)
                .collect::<Vec<Vec3>>()
        });
        let mut sums: BTreeMap<String, (Vec<Vec3>, Vec<f32>)> = BTreeMap::new();
        for (gaps, &(p, _)) in gaps.iter().zip(&cases) {
            let leaning: Vec<(&str, f32, f32)> = poses[p]
                .correctives
                .iter()
                .filter(|(_, w)| *w > LEANING)
                .map(|(name, w)| (name.as_str(), *w, side_of(name)))
                .collect();
            for (k, g) in gaps.iter().enumerate() {
                let x = rest[k].x;
                let here: Vec<&(&str, f32, f32)> = leaning
                    .iter()
                    .filter(|(_, _, side)| *side == 0.0 || x * *side > -0.01)
                    .collect();
                let total: f32 = here.iter().map(|(_, w, _)| w * w).sum();
                if total <= 0.0 {
                    continue;
                }
                for (name, w, _) in here {
                    let (sum, count) = sums
                        .entry((*name).to_owned())
                        .or_insert_with(|| (vec![Vec3::ZERO; rest.len()], vec![0.0; rest.len()]));
                    sum[k] += *g * (w / total);
                    count[k] += 1.0;
                }
            }
        }
        let mut most = 0.0f32;
        for (name, (sum, count)) in sums {
            let Some(shape) = shapes.iter_mut().find(|s| s.name == name) else {
                continue;
            };
            let mut biggest = 0.0f32;
            for ((o, s), c) in shape.offsets.iter_mut().zip(&sum).zip(&count) {
                if *c > 0.0 {
                    let step = *s / *c;
                    *o += step;
                    biggest = biggest.max(step.length());
                }
            }
            most = most.max(biggest);
            *moved.entry(name).or_default() += biggest;
        }
        if most < 0.0005 {
            break;
        }
    }
    let followed = moved.len();
    let most = shapes
        .iter()
        .filter(|s| moved.contains_key(&s.name))
        .map(|s| s.reach())
        .fold(0.0, f32::max);
    (followed, most)
}

/// Where each garment corner should be in `pose` (rest when `None`) with
/// `weights` (its bound point on the shaped, posed skin and the frame
/// there), less where skinning puts it: taken back to rest through the
/// corner's own skinning.
fn gap(
    body: &BodyMesh,
    garment: &Garment,
    pose: Option<&FitPose>,
    weights: &[(String, f32)],
    rest: &[Vec3],
    shapes: &[CarriedShape],
) -> Vec<Vec3> {
    let (positions, normals) = body.shaped(weights);
    let shaped = garment_at(rest, shapes, weights);
    let Some(pose) = pose else {
        let target = frames(garment.binding, &positions, &normals, &body.triangles);
        return target
            .iter()
            .zip(&garment.binding.vertices)
            .zip(&shaped)
            .map(|(((point, axes), bound), at)| *point + *axes * Vec3::from(bound.local) - *at)
            .collect();
    };
    let matrices = skin_matrices(&body.rig, &pose.bones);
    let (posed, posed_normals, _) =
        skin(&matrices, &body.joints, &body.weights, &positions, &normals);
    let target = frames(garment.binding, &posed, &posed_normals, &body.triangles);
    let empty = vec![Vec3::ZERO; shaped.len()];
    let (cloth, _, linear) = skin(&matrices, garment.joints, garment.weights, &shaped, &empty);
    target
        .iter()
        .zip(&garment.binding.vertices)
        .zip(cloth.iter().zip(&linear))
        .map(|(((point, axes), bound), (at, l))| {
            inverse(*l) * (*point + *axes * Vec3::from(bound.local) - *at)
        })
        .collect()
}

/// `a` with `b`'s weights added (names in both are summed).
fn with(a: &[(String, f32)], b: &[(String, f32)]) -> Vec<(String, f32)> {
    let mut out: BTreeMap<String, f32> = a.iter().cloned().collect();
    for (name, w) in b {
        *out.entry(name.clone()).or_default() += *w;
    }
    out.into_iter().filter(|(_, w)| *w != 0.0).collect()
}

fn inverse(m: Mat3) -> Mat3 {
    if m.determinant().abs() < 1e-8 {
        Mat3::IDENTITY
    } else {
        m.inverse()
    }
}
