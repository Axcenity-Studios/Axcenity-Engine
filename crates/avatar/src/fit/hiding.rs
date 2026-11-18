//! Hiding that holds in every pose (CO9 part 2, review fixes 2).
//!
//! An item hides the body regions it covers ([`hidden_regions`](super::hidden_regions)),
//! and hidden skin isn't drawn, so the fit gate's poke-through rule no
//! longer sees it. Covered at rest isn't covered in every pose: a sleeve
//! rides up an arm swung back, a heavy side comes out under the arm. Skin
//! left out of the drawing where the cloth doesn't cover it is a hole the
//! player sees through. So a region stays hidden only where, in every pose
//! the garment is fitted in ([`fit_poses`](super::fit_poses)) on every
//! pose state, none of its skin is more than [`HOLE`] outside the cloth
//! ([`outside_cloth`], the rule the player's fit gate holds hidden skin
//! to as well).

use std::collections::{BTreeMap, BTreeSet};

use glam::Vec3;

use super::BodyMesh;
use super::coverage::CREASE_REACH;
use super::pose::{FitPose, skin, skin_matrices};
use super::surface::Surface;
use super::transfer::{Fitted, garment_at, par_map};
use crate::BaseManifest;

/// How far hidden skin may be outside the cloth over it (as the fit gate's
/// poke-through).
pub const HOLE: f32 = 0.001;
/// How far from the cloth hidden skin is looked for: skin farther than
/// this from every garment is a hole by this much.
pub const HOLE_REACH: f32 = 0.10;
/// How near the skin across a crease must be for the crease to be closed
/// by it (a heavy arm against the side).
pub const CREASE_ACROSS: f32 = 0.04;

/// How far `p` (a body corner, with its normal `n`) is outside the cloth
/// of `garments`, the least over them: negative inside.
///
/// For one garment: where the closest point of the cloth lies along its
/// normal from `p`, how far `p` is out along it; where it lies beside `p`,
/// past the cloth's edge (a sleeve's hem that rode up the arm), the whole
/// distance to it, since nothing of this garment is over `p`. Cloth
/// facing the skin (a lining, the underwear's rim folded under) has `p`
/// inside it.
///
/// Skin deep in a crease (the armpit's apex, the side under a heavy arm)
/// can be far from the cloth, or have it beside, and still be shut in:
/// where its ray out along its normal meets a garment from that garment's
/// inside within [`CREASE_REACH`] (a tee over the armpit, as
/// [`coverage`](super::coverage)'s closed creases), or meets `skin` (the
/// whole body, hidden triangles too) facing back across the crease within
/// [`CREASE_ACROSS`] before any cloth.
pub fn outside_cloth(garments: &[Surface], skin: &Surface, p: Vec3, n: Vec3) -> f32 {
    let out = garments
        .iter()
        .map(|surface| match surface.closest(p, HOLE_REACH) {
            None => HOLE_REACH,
            Some(hit) if n.dot(hit.normal) <= 0.0 => -hit.distance,
            Some(hit) if hit.is_over(p) => hit.signed(p),
            Some(hit) => hit.distance,
        })
        .fold(HOLE_REACH, f32::min);
    if out <= HOLE {
        return out;
    }
    // Each garment on its own: a lower layer the ray meets first (a bra's
    // side band under the armpit) is drawn, and doesn't open the upper
    // one.
    let hits: Vec<_> = garments
        .iter()
        .filter_map(|surface| surface.cast(p, n, CREASE_REACH))
        .collect();
    if hits.iter().any(|hit| hit.normal.dot(n) > 0.0) {
        return 0.0;
    }
    let cloth = hits
        .iter()
        .map(|hit| hit.distance)
        .fold(f32::INFINITY, f32::min);
    // Off the skin a little, so the ray doesn't meet its own triangles.
    let lift = 0.002;
    match skin.cast(p + n * lift, n, CREASE_ACROSS) {
        Some(across) if across.normal.dot(n) < 0.0 && across.distance + lift < cloth => 0.0,
        _ => out,
    }
}

/// Of `hides` (region IDs of `base`), those whose skin stays under the
/// fitted garment in every pose: each of `poses` on each of `states`, and
/// each state at rest. Returns (the regions kept, the regions dropped
/// with the farthest their skin came out, metres). `regions` is each body
/// triangle's region index ([`triangle_regions`](super::triangle_regions)).
pub fn pose_safe_hides(
    base: &BaseManifest,
    body: &BodyMesh,
    fitted: &Fitted,
    regions: &[u8],
    hides: Vec<String>,
    poses: &[FitPose],
    states: &[Vec<(String, f32)>],
) -> (Vec<String>, Vec<(String, f32)>) {
    let index_of: BTreeMap<&str, u8> = base
        .regions
        .iter()
        .filter_map(|r| Some((r.id.as_str(), r.map_index?)))
        .collect();
    let candidates: BTreeSet<u8> = hides
        .iter()
        .filter_map(|id| index_of.get(id.as_str()).copied())
        .collect();
    // The hidden corners, each with the candidate regions it is in.
    let mut corner_regions: BTreeMap<u32, BTreeSet<u8>> = BTreeMap::new();
    for (t, region) in body.triangles.iter().zip(regions) {
        if candidates.contains(region) {
            for v in t {
                corner_regions.entry(*v).or_default().insert(*region);
            }
        }
    }
    if corner_regions.is_empty() || states.is_empty() {
        return (hides, Vec::new());
    }
    let corners: Vec<(u32, BTreeSet<u8>)> = corner_regions.into_iter().collect();
    // Every pose (and rest, `None`) on every state.
    let cases: Vec<(Option<usize>, usize)> = std::iter::once(None)
        .chain((0..poses.len()).map(Some))
        .flat_map(|p| (0..states.len()).map(move |s| (p, s)))
        .collect();
    let rest = fitted.garment.positions();
    let rest_normals = fitted.garment.normals();
    let cloth_triangles = fitted.garment.triangles();
    let worst_by_case = par_map(&cases, |&(p, s)| {
        let pose = p.map(|p| &poses[p]);
        let mut weights = states[s].clone();
        if let Some(pose) = pose {
            for (name, w) in &pose.correctives {
                match weights.iter_mut().find(|(n, _)| n == name) {
                    Some(entry) => entry.1 += *w,
                    None => weights.push((name.clone(), *w)),
                }
            }
        }
        let (mut skin_at, mut skin_normals) = body.shaped(&weights);
        let mut cloth = garment_at(&rest, &fitted.shapes, &weights);
        let mut cloth_normals = rest_normals.clone();
        if let Some(pose) = pose {
            let matrices = skin_matrices(&body.rig, &pose.bones);
            (skin_at, skin_normals, _) = skin(
                &matrices,
                &body.joints,
                &body.weights,
                &skin_at,
                &skin_normals,
            );
            (cloth, cloth_normals, _) = skin(
                &matrices,
                &fitted.joints,
                &fitted.weights,
                &cloth,
                &cloth_normals,
            );
        }
        let garment = [Surface::new(cloth, cloth_normals, cloth_triangles.clone())];
        let skin_surface = Surface::new(
            skin_at.clone(),
            skin_normals.clone(),
            body.triangles.clone(),
        );
        let mut worst: BTreeMap<u8, f32> = BTreeMap::new();
        for (corner, in_regions) in &corners {
            let k = *corner as usize;
            let out = outside_cloth(&garment, &skin_surface, skin_at[k], skin_normals[k]);
            if out > HOLE {
                for region in in_regions {
                    let w = worst.entry(*region).or_insert(0.0);
                    *w = w.max(out);
                }
            }
        }
        worst
    });
    let mut dropped: BTreeMap<u8, f32> = BTreeMap::new();
    for worst in worst_by_case {
        for (region, out) in worst {
            let w = dropped.entry(region).or_insert(0.0);
            *w = w.max(out);
        }
    }
    let id_of: BTreeMap<u8, &str> = index_of.iter().map(|(id, k)| (*k, *id)).collect();
    let kept = hides
        .into_iter()
        .filter(|id| {
            index_of
                .get(id.as_str())
                .is_none_or(|k| !dropped.contains_key(k))
        })
        .collect();
    let dropped = dropped
        .into_iter()
        .map(|(k, out)| (id_of.get(&k).copied().unwrap_or("?").to_owned(), out))
        .collect();
    (kept, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tube round the Z axis: `radius`, from `z0` to `z1`, open at both
    /// ends, its normals out.
    fn tube(radius: f32, z0: f32, z1: f32) -> Surface {
        let (around, along) = (24u32, 12u32);
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        for j in 0..=along {
            let z = z0 + (z1 - z0) * j as f32 / along as f32;
            for i in 0..around {
                let a = std::f32::consts::TAU * i as f32 / around as f32;
                let n = Vec3::new(a.cos(), a.sin(), 0.0);
                positions.push(n * radius + Vec3::Z * z);
                normals.push(n);
            }
        }
        let mut triangles = Vec::new();
        for j in 0..along {
            for i in 0..around {
                let a = j * around + i;
                let b = j * around + (i + 1) % around;
                let (c, d) = (a + around, b + around);
                triangles.push([a, b, d]);
                triangles.push([a, d, c]);
            }
        }
        Surface::new(positions, normals, triangles)
    }

    /// Skin inside a sleeve is covered; skin past its hem (the sleeve
    /// ridden up the arm) and skin through the cloth are holes by how far
    /// they are out.
    #[test]
    fn hidden_skin_past_a_hem_or_through_the_cloth_is_a_hole() {
        let arm = tube(0.04, -0.3, 0.3);
        let sleeve = [tube(0.045, 0.0, 0.1)];
        let out = Vec3::X;
        // Under the sleeve, 5 mm in.
        let under = outside_cloth(&sleeve, &arm, Vec3::new(0.04, 0.0, 0.05), out);
        assert!((under + 0.005).abs() < 1e-3, "{under}");
        // 3 cm past its hem: nothing of the sleeve is over it.
        let past = outside_cloth(&sleeve, &arm, Vec3::new(0.04, 0.0, 0.13), out);
        assert!(past > 0.02, "{past}");
        // Through the cloth by 4 mm.
        let through = outside_cloth(&sleeve, &arm, Vec3::new(0.049, 0.0, 0.05), out);
        assert!((through - 0.004).abs() < 1e-3, "{through}");
        // Far from any cloth.
        let bare = outside_cloth(&sleeve, &arm, Vec3::new(0.04, 0.0, -0.25), out);
        assert_eq!(bare, HOLE_REACH);
    }

    /// Skin in a crease far from the cloth beside it is shut in when its
    /// ray out meets the cloth from inside, or the skin across the crease
    /// first; open when the ray meets the cloth from outside.
    #[test]
    fn skin_shut_in_a_crease_is_no_hole() {
        // A wide garment tube the skin point sits deep inside, its ray
        // along +X meeting the tube's wall from inside 12 cm away.
        let wide = [tube(0.13, -0.2, 0.2)];
        let nowhere = tube(1.0, 5.0, 5.1);
        let deep = outside_cloth(&wide, &nowhere, Vec3::new(0.01, 0.0, 0.0), Vec3::X);
        assert!(deep <= HOLE, "{deep}");
        // Past the tube's end, 13 cm from its rim: the ray meets nothing
        // from inside.
        let p = Vec3::new(-0.05, 0.0, 0.3);
        let open = outside_cloth(&wide, &nowhere, p, Vec3::X);
        assert!(open > HOLE, "{open}");
        // The same point, with skin facing back across the crease 3 cm
        // along its ray (a heavy arm against the side).
        let across = tube(0.02, 0.2, 0.4);
        let shut = outside_cloth(&wide, &across, p, Vec3::X);
        assert!(shut <= HOLE, "{shut}");
    }
}
