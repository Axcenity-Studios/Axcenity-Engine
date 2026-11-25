//! Cover maps (ADR-0045): what a fitted garment hides when it is worn.
//!
//! A garment covers skin, and anything worn between it and the skin. What
//! it covers is drawn as an image over the **body's** texture coordinates
//! ([`cover_map`], an item manifest's `cover_map`), so it means the same
//! at every level of detail of the body and for every garment bound to
//! that body:
//!
//! - **red**: skin the body leaves out under the garment. Skin that isn't
//!   drawn can't come through the cloth, whatever the pose or the body's
//!   shape;
//! - **green**: skin under which a garment worn beneath this one leaves
//!   its own cloth out (a trouser's waistband under a shirt's hem), found
//!   by the inner garment's corners' places on the skin ([`skin_uvs`]);
//! - **blue**: the same for a top worn **tucked** into this garment (a
//!   bottom). Round the hips it is lit wherever the cloth is over the
//!   skin, with no setback: nothing is seen under a waistband as it is
//!   under a hem. Anywhere else (the bib and the straps of overalls) it
//!   is as green.
//!
//! **Skin is hidden only where the cloth stays over it.** Given the fit's
//! poses ([`Proof`]), that is tried: the garment and the body are posed
//! together in each pose on each body state, and a skin triangle is
//! hidden when cloth lies over each of its corners in all of them
//! ([`uncovered`](super::uncovered), the rule the player's fit gate holds
//! hidden skin to, or cloth met out along the corner's normal within
//! [`OVER_REACH`]: a loose garment stands farther off). The inner arm under a short sleeve is as near the
//! shirt's side as its own sleeve at rest, and comes out from under it
//! when the arm lifts: it stays drawn. Without poses the older stand-in
//! is used: the cloth over the skin must follow the same bones
//! ([`MOVES_WITH`]), which leaves most skin under a loose garment drawn.
//! Skin in a crease the garment only closes over (the armpit) stays drawn
//! either way. Cloth worn beneath is hidden wherever cloth is over it:
//! where that leaves a gap in a pose, the skin under it is still drawn.
//!
//! Neither reaches the garment's openings. From an opening (a hem, a
//! cuff, a neckline) the covered skin stays drawn for a **setback** that
//! grows with how far the cloth stands off the skin there: looking under
//! a loose hem one sees skin, not the place where it stops, and a tight
//! cuff hides almost up to its edge. The cloth's setback is shorter than
//! the skin's, since what is under a hem is nearly always the skin.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};

use glam::{Vec2, Vec3};

use super::BodyMesh;
use super::bind::Binding;
use super::coverage::{under_cloth, welded};
use super::hiding::uncovered;
use super::pose::{FitPose, skin, skin_matrices};
use super::surface::Surface;
use super::transfer::par_map;

/// A cover map's side in pixels.
pub const COVER_SIZE: u32 = 1024;
/// A pixel is lit with this value (the runtime reads 128 and up as lit:
/// `axcenity_assets::COVERED`).
const LIT: u8 = 255;

/// The skin's setback from an opening: this many times the cloth's
/// distance from the skin there, between [`SKIN_LEAST`] and
/// [`SKIN_MOST`] metres. At 2.5 a line of sight under the hem must be
/// within 22 degrees of the skin to reach where it stops.
const SKIN_GAIN: f32 = 2.5;
const SKIN_LEAST: f32 = 0.025;
const SKIN_MOST: f32 = 0.060;
/// How much of a skin triangle's skin weights the cloth over it must
/// share (the sum over the bones of the lesser weight) to cover it.
const MOVES_WITH: f32 = 0.8;
/// How far from under the cloth a skin corner may come in a pose and
/// still be hidden (metres; [`Proof`]).
const BARE: f32 = 0.001;
/// How far out along its normal a skin corner looks for the cloth over
/// it in a pose (metres): a coat stands this far off the back.
const OVER_REACH: f32 = 0.25;
/// The setback of cloth worn beneath, likewise.
const CLOTH_GAIN: f32 = 1.0;
const CLOTH_LEAST: f32 = 0.003;
const CLOTH_MOST: f32 = 0.040;

/// A cover map: `size` × `size` pixels of RGBA, rows from the top, over
/// the body's first texture coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverMap {
    pub size: u32,
    pub rgba: Vec<u8>,
}

/// What a cover map hides, counted in body triangles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct CoverReport {
    /// Under the garment at all.
    pub covered: usize,
    /// Left out of the body.
    pub skin: usize,
    /// Under which a garment worn beneath leaves its cloth out.
    pub cloth: usize,
}

/// A triangle on the way out from the covered skin's edge.
#[derive(PartialEq)]
struct Reached {
    distance: f32,
    triangle: usize,
}

impl Eq for Reached {}

impl Ord for Reached {
    fn cmp(&self, other: &Self) -> Ordering {
        // The nearest first out of a max-heap.
        other
            .distance
            .total_cmp(&self.distance)
            .then_with(|| other.triangle.cmp(&self.triangle))
    }
}

impl PartialOrd for Reached {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A garment's corners at rest with given weights of its carried shapes.
pub type ClothAt<'a> = dyn Fn(&[(String, f32)]) -> Vec<Vec3> + Sync + 'a;

/// What a cover map's hidden skin is tried in: the fit's poses, on each
/// of the body states, with the garment's corners as its carried shapes
/// put them there.
pub struct Proof<'a> {
    pub poses: &'a [FitPose],
    /// Body shape weights (the frames, the build's corners, the body
    /// presets); none means the default body alone.
    pub states: &'a [Vec<(String, f32)>],
    /// The garment's corners at rest with these weights of its shapes.
    pub cloth_at: &'a ClothAt<'a>,
}

/// The cover map of the fitted garment `garment` (at rest, with each of
/// its corners' skin `weights` on the body's joints) over `body` (at
/// rest), whose triangles' regions are `regions` (skin without a region,
/// 0, is never hidden: the head and what the base keeps drawn); `tuck`
/// names the regions round the hips, where a tucked top is under the
/// garment whole (the map's blue). With
/// `proof`, skin is hidden where the cloth stays over it in every pose;
/// without, only under cloth that follows the bones it follows. A body
/// without texture coordinates has an empty map.
pub fn cover_map(
    body: &BodyMesh,
    (regions, tuck): (&[u8], &[u8]),
    garment: &Surface,
    weights: (&[[u16; 4]], &[[f32; 4]]),
    proof: Option<&Proof>,
    size: u32,
) -> (CoverMap, CoverReport) {
    let size = size.max(1);
    let mut map = CoverMap {
        size,
        rgba: [0, 0, 0, 255].repeat((size * size) as usize),
    };
    let mut report = CoverReport::default();
    if body.uvs.len() != body.positions.len() {
        return (map, report);
    }
    // A point's skin weights by bone: of the body, or of the cloth.
    let mix = |joints: &[[u16; 4]], shares: &[[f32; 4]], corners: [u32; 3], bary: Vec3| {
        let mut mix: BTreeMap<u16, f32> = BTreeMap::new();
        for (&corner, at) in corners.iter().zip(bary.to_array()) {
            let (Some(joints), Some(shares)) =
                (joints.get(corner as usize), shares.get(corner as usize))
            else {
                continue;
            };
            for (joint, share) in joints.iter().zip(shares) {
                if *share > 0.0 {
                    *mix.entry(*joint).or_default() += share * at;
                }
            }
        }
        mix
    };
    // Under the cloth at all (what hides cloth worn beneath), and under
    // cloth that follows the bones the skin follows (what hides skin).
    let under = under_cloth(body, regions, garment, false);
    let any: Vec<Option<f32>> = under
        .iter()
        .map(|under| under.as_ref().map(|under| under.gap))
        .collect();
    let stays = proof.map(|proof| stays_covered(body, garment, weights, &any, proof));
    let moving: Vec<Option<f32>> = under
        .iter()
        .zip(&body.triangles)
        .enumerate()
        .map(|(k, (under, skin))| {
            if let Some(stays) = &stays {
                return any[k].filter(|_| stays[k]);
            }
            let under = under.as_ref()?;
            let (triangle, bary) = under.cloth?;
            let of_skin = mix(&body.joints, &body.weights, *skin, Vec3::splat(1.0 / 3.0));
            let of_cloth = mix(
                weights.0,
                weights.1,
                garment.triangles[triangle as usize],
                bary,
            );
            let shared: f32 = of_skin
                .iter()
                .map(|(joint, share)| share.min(of_cloth.get(joint).copied().unwrap_or(0.0)))
                .sum();
            (shared >= MOVES_WITH).then_some(under.gap)
        })
        .collect();
    let middles: Vec<Vec3> = body
        .triangles
        .iter()
        .map(|t| t.iter().map(|&v| body.positions[v as usize]).sum::<Vec3>() / 3.0)
        .collect();
    // Triangles that share an edge (corners at one place are one).
    let weld = welded(&body.positions);
    let mut by_edge: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (k, t) in body.triangles.iter().enumerate() {
        let t = t.map(|v| weld[v as usize]);
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            by_edge.entry((u.min(v), u.max(v))).or_default().push(k);
        }
    }
    let neighbours = |k: usize| {
        let t = body.triangles[k].map(|v| weld[v as usize]);
        [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]
            .into_iter()
            .flat_map(|(u, v)| by_edge[&(u.min(v), u.max(v))].iter().copied())
            .filter(move |&other| other != k)
    };
    // Each covered triangle's way to the nearest edge of the covered
    // skin, over the skin (triangle middle to middle), and how far the
    // cloth stands off there: the opening it would be seen through.
    let ways = |gaps: &[Option<f32>]| -> (Vec<f32>, Vec<f32>) {
        let mut distance = vec![f32::INFINITY; body.triangles.len()];
        let mut opening = vec![0.0f32; body.triangles.len()];
        let mut heap = BinaryHeap::new();
        for k in 0..body.triangles.len() {
            let Some(gap) = gaps[k] else {
                continue;
            };
            // Bare skin beside it, or the body's own edge.
            let edge = neighbours(k)
                .filter(|&other| gaps[other].is_none())
                .map(|other| middles[k].distance(middles[other]) * 0.5)
                .fold(None, |least: Option<f32>, d| {
                    Some(least.map_or(d, |l| l.min(d)))
                });
            if let Some(edge) = edge {
                distance[k] = edge;
                opening[k] = gap;
                heap.push(Reached {
                    distance: edge,
                    triangle: k,
                });
            }
        }
        while let Some(Reached {
            distance: here,
            triangle: k,
        }) = heap.pop()
        {
            if here > distance[k] {
                continue;
            }
            for other in neighbours(k) {
                if gaps[other].is_none() {
                    continue;
                }
                let next = here + middles[k].distance(middles[other]);
                if next < distance[other] {
                    distance[other] = next;
                    opening[other] = opening[k];
                    heap.push(Reached {
                        distance: next,
                        triangle: other,
                    });
                }
            }
        }
        (distance, opening)
    };
    let (skin_way, skin_opening) = ways(&moving);
    let (cloth_way, cloth_opening) = ways(&any);
    for k in 0..body.triangles.len() {
        if any[k].is_none() {
            continue;
        }
        report.covered += 1;
        // Skin: the way is measured to the triangle's middle, and its
        // corners reach that much nearer the edge; the whole triangle
        // must be set back, or a big triangle's corner would be left
        // bare at a neckline.
        let reach = body.triangles[k]
            .iter()
            .map(|&v| body.positions[v as usize].distance(middles[k]))
            .fold(0.0, f32::max);
        let skin = moving[k].is_some()
            && skin_way[k] - reach >= (skin_opening[k] * SKIN_GAIN).clamp(SKIN_LEAST, SKIN_MOST);
        // Cloth worn beneath: under a hole in it the skin is still drawn,
        // so it goes wherever cloth is over it, by its middle.
        let cloth = cloth_way[k] >= (cloth_opening[k] * CLOTH_GAIN).clamp(CLOTH_LEAST, CLOTH_MOST);
        report.skin += usize::from(skin);
        report.cloth += usize::from(cloth);
        let uvs = body.triangles[k].map(|v| Vec2::from(body.uvs[v as usize]));
        // A tucked top goes wherever the cloth is over the skin, but
        // only round the hips: a strap is no waistband.
        let tucked = cloth || tuck.contains(&regions[k]);
        fill(&mut map, uvs, [skin, cloth, tucked]);
    }
    (map, report)
}

/// Which body triangles (of those under the cloth at rest, `any`) have
/// cloth over each of their corners in every pose of `proof` on every
/// one of its states.
fn stays_covered(
    body: &BodyMesh,
    garment: &Surface,
    (joints, weights): (&[[u16; 4]], &[[f32; 4]]),
    any: &[Option<f32>],
    proof: &Proof,
) -> Vec<bool> {
    let mut wanted = vec![false; body.positions.len()];
    for (t, gap) in body.triangles.iter().zip(any) {
        if gap.is_some() {
            for &v in t {
                wanted[v as usize] = true;
            }
        }
    }
    let corners: Vec<usize> = (0..wanted.len()).filter(|k| wanted[*k]).collect();
    let default = [Vec::new()];
    let states = if proof.states.is_empty() {
        &default[..]
    } else {
        proof.states
    };
    let cases: Vec<(usize, usize)> = (0..proof.poses.len())
        .flat_map(|p| (0..states.len()).map(move |s| (p, s)))
        .collect();
    let bare_by_case = par_map(&cases, |&(p, s)| {
        let pose = &proof.poses[p];
        let mut shapes = states[s].clone();
        for (name, weight) in &pose.correctives {
            match shapes.iter_mut().find(|(n, _)| n == name) {
                Some(entry) => entry.1 += *weight,
                None => shapes.push((name.clone(), *weight)),
            }
        }
        let (skin_at, skin_normals) = body.shaped(&shapes);
        let cloth = (proof.cloth_at)(&shapes);
        let matrices = skin_matrices(&body.rig, &pose.bones);
        let (skin_at, skin_normals, _) = skin(
            &matrices,
            &body.joints,
            &body.weights,
            &skin_at,
            &skin_normals,
        );
        let (cloth, cloth_normals, _) = skin(&matrices, joints, weights, &cloth, &garment.normals);
        let cloth = [Surface::new(
            cloth,
            cloth_normals,
            garment.triangles.clone(),
        )];
        let skin_surface = Surface::new(
            skin_at.clone(),
            skin_normals.clone(),
            body.triangles.clone(),
        );
        corners
            .iter()
            .map(|&k| {
                let (p, n) = (skin_at[k], skin_normals[k]);
                // Under the cloth by the gate's rule, or with cloth out
                // along its normal: loose cloth stands farther off the
                // skin than the gate looks, and is over it all the same.
                uncovered(&cloth, &skin_surface, p, n) > BARE
                    && cloth[0].cast(p + n * 0.002, n, OVER_REACH).is_none()
            })
            .collect::<Vec<bool>>()
    });
    let mut bare = vec![false; body.positions.len()];
    for case in bare_by_case {
        for (k, out) in corners.iter().zip(case) {
            if out {
                bare[*k] = true;
            }
        }
    }
    body.triangles
        .iter()
        .zip(any)
        .map(|(t, gap)| gap.is_some() && t.iter().all(|&v| !bare[v as usize]))
        .collect()
}

/// Lights the pixels of `map` under the triangle `uvs` in the channels
/// `lit` names: every pixel whose middle is in it or within three
/// quarters of a pixel of it, so the triangle's own middle and every
/// point of it is on a lit pixel.
fn fill(map: &mut CoverMap, uvs: [Vec2; 3], lit: [bool; 3]) {
    const SLACK: f32 = 0.75;
    let size = map.size as f32;
    let [a, b, c] = uvs.map(|uv| uv * size);
    let lo = a.min(b).min(c) - SLACK;
    let hi = a.max(b).max(c) + SLACK;
    let most = map.size as i64 - 1;
    let (x0, x1) = (
        (lo.x.floor() as i64).clamp(0, most),
        (hi.x.floor() as i64).clamp(0, most),
    );
    let (y0, y1) = (
        (lo.y.floor() as i64).clamp(0, most),
        (hi.y.floor() as i64).clamp(0, most),
    );
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            if away(p, a, b, c) > SLACK {
                continue;
            }
            let at = (y as usize * map.size as usize + x as usize) * 4;
            for (channel, lit) in lit.iter().enumerate() {
                if *lit {
                    map.rgba[at + channel] = LIT;
                }
            }
        }
    }
}

/// How far `p` is from the triangle `a`, `b`, `c` in the plane (0 inside).
fn away(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let side = |u: Vec2, v: Vec2| (v - u).perp_dot(p - u);
    let (d0, d1, d2) = (side(a, b), side(b, c), side(c, a));
    let inside = (d0 >= 0.0 && d1 >= 0.0 && d2 >= 0.0) || (d0 <= 0.0 && d1 <= 0.0 && d2 <= 0.0);
    if inside {
        return 0.0;
    }
    let to_edge = |u: Vec2, v: Vec2| {
        let along = v - u;
        let t = ((p - u).dot(along) / along.length_squared().max(1e-12)).clamp(0.0, 1.0);
        p.distance(u + along * t)
    };
    to_edge(a, b).min(to_edge(b, c)).min(to_edge(c, a))
}

/// Each garment corner's place on the skin: the body's texture
/// coordinates at the point `binding` ties it to (a fitted model's
/// `_AXCENITY_SKIN_UV`, `axcenity_assets::Primitive::skin_uvs`). Empty
/// when the body has no texture coordinates.
pub fn skin_uvs(body: &BodyMesh, binding: &Binding) -> Vec<[f32; 2]> {
    if body.uvs.len() != body.positions.len() {
        return Vec::new();
    }
    binding
        .vertices
        .iter()
        .map(|bound| {
            let Some(triangle) = body.triangles.get(bound.triangle as usize) else {
                return [0.0, 0.0];
            };
            let uv = triangle
                .iter()
                .zip(bound.bary)
                .map(|(&v, weight)| Vec2::from(body.uvs[v as usize]) * weight)
                .sum::<Vec2>();
            uv.to_array()
        })
        .collect()
}
