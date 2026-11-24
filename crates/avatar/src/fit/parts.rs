//! The body as parts, and a garment's cloth shared between them
//! (ADR-0045).
//!
//! The closest skin to a corner of cloth is often the wrong skin: the
//! underside of a wide sleeve is nearer the side of the chest than the
//! arm inside it, and the front of a skirt is nearer one thigh than the
//! other by a hair. So the body is five parts by its bones ([`of`]: the
//! trunk, each arm, each leg), and each corner of a garment has a
//! **share** of each ([`shares`]): all of one where it is clearly
//! nearest, and elsewhere what the cloth round it has, settled over the
//! garment's own surface. A sleeve is the arm's all the way round, the
//! underarm passes smoothly from the arm to the side, and a skirt hangs
//! between both legs.
//!
//! The intake draws a garment in round the body by these shares
//! (`intake::conform`), and a garment that came through it takes its
//! skin weights by them ([`weights`], [`FitOptions::weights_by_part`]
//! (super::FitOptions::weights_by_part)).

use std::collections::BTreeMap;

use glam::Vec3;

use super::BodyMesh;
use super::surface::Surface;
use super::transfer::Welds;

/// The bones whose skin (with their children's) is a part of its own:
/// each arm and each leg. The rest is the trunk.
const LIMBS: [&str; 4] = [
    "RightUpperArm",
    "LeftUpperArm",
    "RightUpperLeg",
    "LeftUpperLeg",
];
/// How many rounds the shares settle over the cloth.
const SETTLE: usize = 150;
/// Cloth this many times nearer one part than any other is that part's.
const CLEARLY: f32 = 0.5;
/// How far from a part cloth is still measured (metres).
const REACH: f32 = 0.6;

/// A corner of cloth's place on one part of the body.
#[derive(Clone, Copy, Debug)]
pub(super) struct Near {
    /// The corners of the body's triangle nearest it, and the point on
    /// it as their weights.
    pub corners: [u32; 3],
    pub bary: Vec3,
    /// How far it is from that point: negative inside the body.
    pub clearance: f32,
    /// The way out from the skin there, toward the corner.
    pub out: Vec3,
}

/// How much of its legs' shares a skirt's cloth gives to the trunk
/// ([`weights`]): a skirt hangs from the hips and is only pushed by the
/// legs, and cloth that followed each leg alone would part between them
/// at every step.
const SKIRT_STIFF: f32 = 0.55;
/// A skirt's cloth is all the legs' from this far over the height the
/// skirt starts at, and stiffened fully from this far under it (metres).
const SKIRT_FADE: f32 = 0.08;

/// The body as parts: the trunk, then each of [`LIMBS`] that has skin, by
/// the bone each triangle's corners follow most. Each part is a surface
/// over the body's own corners.
pub(super) fn of(body: &BodyMesh) -> Vec<Surface> {
    kinds(body).into_iter().map(|(_, part)| part).collect()
}

/// [`of`], each part with what it is: 0 the trunk, 1 and 2 the arms, 3
/// and 4 the legs.
fn kinds(body: &BodyMesh) -> Vec<(usize, Surface)> {
    let rig = &body.rig;
    // Each bone's part: 0 the trunk, 1 + k for LIMBS[k] and what hangs
    // from it.
    let mut of_bone = vec![0usize; rig.bones.len()];
    for (index, bone) in rig.bones.iter().enumerate() {
        of_bone[index] = match LIMBS.iter().position(|limb| bone.name == *limb) {
            Some(limb) => 1 + limb,
            // Parents come before their children.
            None => bone.parent.map_or(0, |parent| of_bone[parent]),
        };
    }
    let of_corner: Vec<usize> = body
        .joints
        .iter()
        .zip(&body.weights)
        .map(|(joints, weights)| {
            let mut most = [0.0f32; 1 + LIMBS.len()];
            for (joint, weight) in joints.iter().zip(weights) {
                if let Some(joint) = rig.joints.get(usize::from(*joint)) {
                    most[of_bone[joint.bone]] += *weight;
                }
            }
            (0..most.len())
                .max_by(|a, b| most[*a].total_cmp(&most[*b]))
                .unwrap_or(0)
        })
        .collect();
    let mut triangles: Vec<Vec<[u32; 3]>> = vec![Vec::new(); 1 + LIMBS.len()];
    for t in &body.triangles {
        let mut votes = [0u8; 1 + LIMBS.len()];
        for &v in t {
            votes[of_corner[v as usize]] += 1;
        }
        let part = (0..votes.len()).max_by_key(|k| votes[*k]).unwrap_or(0);
        triangles[part].push(*t);
    }
    triangles
        .into_iter()
        .enumerate()
        .filter(|(_, list)| !list.is_empty())
        .map(|(kind, list)| {
            (
                kind,
                Surface::new(body.positions.clone(), body.normals.clone(), list),
            )
        })
        .collect()
}

/// Each of `positions` (a garment's corners, welded by `welds`) on each
/// of `parts`, and its shares of them (adding up to 1 where it is near
/// any).
pub(super) fn shares(
    parts: &[Surface],
    welds: &Welds,
    positions: &[Vec3],
) -> (Vec<Vec<Option<Near>>>, Vec<Vec<f32>>) {
    let measured: Vec<Vec<Option<Near>>> = positions
        .iter()
        .map(|p| {
            parts
                .iter()
                .map(|part| {
                    let hit = part.closest(*p, REACH)?;
                    let off = *p - hit.point;
                    let out = off.try_normalize().unwrap_or(hit.normal);
                    let inside = off.dot(hit.normal) < 0.0;
                    Some(Near {
                        corners: part.triangles[hit.triangle as usize],
                        bary: hit.bary,
                        clearance: if inside { -hit.distance } else { hit.distance },
                        out: if inside { -out } else { out },
                    })
                })
                .collect()
        })
        .collect();
    // All of one part where it is clearly nearest, held there; the rest
    // settle between.
    let mut shares: Vec<Vec<f32>> = Vec::with_capacity(positions.len());
    let mut loose: Vec<f32> = Vec::with_capacity(positions.len());
    for near in &measured {
        let mut order: Vec<(usize, f32)> = near
            .iter()
            .enumerate()
            .filter_map(|(part, hit)| Some((part, hit.as_ref()?.clearance.abs())))
            .collect();
        order.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut own = vec![0.0; parts.len()];
        let clear = match order.as_slice() {
            [] => false,
            [(part, _)] => {
                own[*part] = 1.0;
                true
            }
            [(part, nearest), (_, next), ..] => {
                own[*part] = 1.0;
                *nearest < CLEARLY * *next
            }
        };
        shares.push(own);
        loose.push(if clear { 0.0 } else { 0.7 });
    }
    for part in 0..parts.len() {
        let field: Vec<Vec3> = shares
            .iter()
            .map(|own| Vec3::new(own[part], 0.0, 0.0))
            .collect();
        let settled = welds.smooth(&field, &loose, SETTLE);
        for (own, value) in shares.iter_mut().zip(settled) {
            own[part] = value.x;
        }
    }
    (measured, shares)
}

/// For each of the body's joints, the joint cloth takes in its place: a
/// hand's, a finger's and the forearm's twist bone's become the
/// forearm's, a foot's and a toe's the shin's, and every other its own.
/// A cuff sits on the forearm and a hem on the shin: cloth that turned
/// with the wrist (as the skin of the forearm does, by its twist bone)
/// would swing a wide cuff round the arm, and a hem bent with the ankle
/// would fold over the foot.
fn held_joints(body: &BodyMesh) -> Vec<u16> {
    let rig = &body.rig;
    let joint_of = |name: &str| -> Option<u16> {
        let bone = rig.bone(name)?;
        rig.joints
            .iter()
            .position(|joint| joint.bone == bone)
            .map(|joint| joint as u16)
    };
    (0..rig.joints.len())
        .map(|own| {
            let mut at = Some(rig.joints[own].bone);
            let mut first = true;
            while let Some(bone) = at {
                let name = rig.bones[bone].name.as_str();
                for side in ["Left", "Right"] {
                    let Some(part) = name.strip_prefix(side) else {
                        continue;
                    };
                    let onto = match part {
                        "Hand" => Some("LowerArm"),
                        "LowerArmTwist" if first => Some("LowerArm"),
                        "Foot" => Some("LowerLeg"),
                        _ => None,
                    };
                    if let Some(joint) = onto.and_then(|onto| joint_of(&format!("{side}{onto}"))) {
                        return joint;
                    }
                }
                first = false;
                at = rig.bones[bone].parent;
            }
            own as u16
        })
        .collect()
}

/// Skin weights for a garment's corners at `positions` by their shares
/// of the body's parts: each part's own skin weights at the point of it
/// nearest the corner, mixed by the shares, the four largest kept,
/// adding up to 1. A corner near no part keeps `fallback`'s.
///
/// With `skirt_from` (a height: the body's crotch, for a garment without
/// legs) the cloth under it is a skirt's: it gives [`SKIRT_STIFF`] of its
/// legs' shares to the trunk, so it hangs from the hips and the legs
/// only push it.
pub(super) fn weights(
    body: &BodyMesh,
    welds: &Welds,
    positions: &[Vec3],
    fallback: (&[[u16; 4]], &[[f32; 4]]),
    skirt_from: Option<f32>,
) -> (Vec<[u16; 4]>, Vec<[f32; 4]>) {
    let (kinds, parts): (Vec<usize>, Vec<Surface>) = kinds(body).into_iter().unzip();
    let (measured, mut shares) = shares(&parts, welds, positions);
    if let (Some(from), Some(trunk)) = (skirt_from, kinds.iter().position(|kind| *kind == 0)) {
        for (own, p) in shares.iter_mut().zip(positions) {
            // Only where the trunk is in reach to hang from.
            let under = ((from + SKIRT_FADE - p.y) / (2.0 * SKIRT_FADE)).clamp(0.0, 1.0);
            if under <= 0.0 {
                continue;
            }
            for (part, kind) in kinds.iter().enumerate() {
                if *kind >= 3 {
                    let given = own[part] * SKIRT_STIFF * under;
                    own[part] -= given;
                    own[trunk] += given;
                }
            }
        }
    }
    let held = held_joints(body);
    let mut joints = Vec::with_capacity(positions.len());
    let mut weights = Vec::with_capacity(positions.len());
    for (k, (near, own)) in measured.iter().zip(&shares).enumerate() {
        let mut mix: BTreeMap<u16, f32> = BTreeMap::new();
        for (hit, share) in near.iter().zip(own) {
            let Some(hit) = hit else {
                continue;
            };
            if *share <= 0.02 {
                continue;
            }
            for (corner, at) in hit.corners.iter().zip(hit.bary.to_array()) {
                let corner = *corner as usize;
                for (joint, weight) in body.joints[corner].iter().zip(body.weights[corner]) {
                    if weight > 0.0 {
                        let joint = held.get(*joint as usize).copied().unwrap_or(*joint);
                        *mix.entry(joint).or_default() += weight * at * share;
                    }
                }
            }
        }
        let mut sorted: Vec<(u16, f32)> = mix.into_iter().collect();
        sorted.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        sorted.truncate(4);
        let total: f32 = sorted.iter().map(|(_, w)| w).sum();
        if total <= 0.0 {
            joints.push(fallback.0[k]);
            weights.push(fallback.1[k]);
            continue;
        }
        let mut j = [0u16; 4];
        let mut w = [0.0f32; 4];
        for (i, (joint, weight)) in sorted.into_iter().enumerate() {
            j[i] = joint;
            w[i] = weight / total;
        }
        joints.push(j);
        weights.push(w);
    }
    (joints, weights)
}
