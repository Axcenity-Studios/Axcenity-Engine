//! Each sleeve and trouser leg centred on the limb inside it.
//!
//! The intake turns a sleeve to the body's rest pose as one straight
//! piece, about the shoulder, so its cuff comes to lie on the arm's line.
//! A generated garment's sleeve is seldom straight: its elbow is bent a
//! little, and a wide one hangs. Between the shoulder and the cuff it then
//! lies beside the arm, not round it: the forearm comes out through its
//! underside, and its cloth is shared with the trunk it lies no farther
//! from. So the cloth of each limb (cloth nearer the limb's line than the
//! trunk's skin or another limb, joined over the garment's own surface,
//! from where it leaves the trunk) is cut into rings along the limb's
//! bones, and each ring is moved across the limb until the limb is at its
//! middle; the moves are smoothed along the limb and fade out toward the
//! trunk.

use std::collections::BTreeMap;

use glam::Vec3;

use super::super::parts;
use super::super::transfer::Welds;
use super::super::{BodyMesh, GarmentMesh};

/// A limb's cloth starts this far along it from its joint, as a share of
/// its length: nearer the joint the cloth is the trunk's too.
const FROM: f32 = 0.2;
/// The moves fade in over this far past [`FROM`] (metres).
const FADE: f32 = 0.08;
/// How far from a limb's line cloth can be its sleeve (metres).
const ROUND: f32 = 0.3;
/// A limb's cloth is every piece of cloth along it whose corners lie
/// this near its line or nearer, taken at their middle (metres): a
/// jacket's side beside a hanging arm is farther.
const NEAR: f32 = 0.16;
/// The rings' width along the limb (metres), and the fewest corners that
/// make one.
const RING: f32 = 0.025;
const FEWEST: usize = 6;
/// The fewest corners a limb's cloth has.
const LEAST: usize = 30;
/// A ring whose middle is farther than this from the limb isn't its
/// sleeve read rightly: the limb is left as it is (metres).
const MOST: f32 = 0.15;
/// How many times the rings' moves are evened along the limb.
const EVEN: usize = 3;
/// How many rounds the corners' moves are smoothed over the cloth.
const SMOOTH: usize = 4;

/// The limbs: the bones from the joint to the end.
const ARMS: [[&str; 3]; 2] = [
    ["RightUpperArm", "RightLowerArm", "RightHand"],
    ["LeftUpperArm", "LeftLowerArm", "LeftHand"],
];
const LEGS: [[&str; 3]; 2] = [
    ["RightUpperLeg", "RightLowerLeg", "RightFoot"],
    ["LeftUpperLeg", "LeftLowerLeg", "LeftFoot"],
];

/// Centres `garment`'s sleeves (and with `legs` its legs) on `body`'s
/// limbs at rest. Returns the farthest a corner moved (metres).
pub(super) fn centre_limbs(garment: &mut GarmentMesh, body: &BodyMesh, legs: bool) -> f32 {
    let rig = &body.rig;
    let world = rig.world(&rig.rest_pose());
    let head = |name: &str| rig.bone(name).map(|bone| world[bone].w_axis.truncate());
    let triangles = garment.triangles();
    let mut positions = garment.positions();
    let welds = Welds::new(&positions, &triangles);
    let mut most = 0.0f32;
    // The trunk's skin, and every limb's line: cloth nearer one of the
    // others than a limb's own line isn't that limb's.
    let trunk = parts::of(body).into_iter().next();
    let lines: Vec<Line> = ARMS
        .iter()
        .chain(LEGS.iter())
        .filter_map(|names| {
            let [Some(a), Some(b), Some(c)] = names.map(head) else {
                return None;
            };
            Some(Line::new([a, b, c]))
        })
        .collect();
    let count = if legs {
        lines.len()
    } else {
        lines.len().min(ARMS.len())
    };
    for own in 0..count {
        let line = &lines[own];
        let other = |p: Vec3| {
            let skin = trunk
                .as_ref()
                .and_then(|trunk| trunk.closest(p, ROUND))
                .map_or(f32::INFINITY, |hit| hit.distance);
            lines
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != own)
                .map(|(_, line)| line.place(p).1.length())
                .fold(skin, f32::min)
        };
        let moves = moves_onto(line, &positions, &triangles, &other);
        if moves.iter().all(|m| *m == Vec3::ZERO) {
            continue;
        }
        // Held where nothing moves, so the trunk stays where it is.
        let shares: Vec<f32> = moves
            .iter()
            .map(|m| if *m == Vec3::ZERO { 0.0 } else { 0.5 })
            .collect();
        let moves = welds.smooth(&moves, &shares, SMOOTH);
        for (p, m) in positions.iter_mut().zip(&moves) {
            *p += *m;
            most = most.max(m.length());
        }
    }
    if most > 0.0 {
        garment.set_positions(&positions);
    }
    most
}

/// A limb's line: its two bones, and on past its end.
struct Line {
    points: [Vec3; 3],
    /// Two directions across the limb, for measuring a ring.
    across: [Vec3; 2],
}

impl Line {
    fn new(points: [Vec3; 3]) -> Line {
        let along = (points[2] - points[0]).normalize_or_zero();
        // Across: toward the sky as nearly as the limb allows (a leg's
        // line is nearly upright: then forward), and square to both.
        let toward = if along.y.abs() < 0.9 {
            Vec3::Y
        } else {
            Vec3::Z
        };
        let up = (toward - along * toward.dot(along)).normalize_or_zero();
        Line {
            points,
            across: [up, along.cross(up)],
        }
    }

    fn length(&self) -> f32 {
        self.points[0].distance(self.points[1]) + self.points[1].distance(self.points[2])
    }

    /// How far along the limb `p` is (from its joint; past its end too),
    /// and `p` from the nearest point of its line.
    fn place(&self, p: Vec3) -> (f32, Vec3) {
        let [a, b, c] = self.points;
        let upper = a.distance(b);
        let on = |from: Vec3, to: Vec3, least: f32, most: f32| {
            let along = to - from;
            let t = ((p - from).dot(along) / along.length_squared().max(1e-12)).clamp(least, most);
            (t * along.length(), from + along * t)
        };
        // The upper bone to the elbow, the lower from it on past the end.
        let (first, at_first) = on(a, b, f32::NEG_INFINITY, 1.0);
        let (second, at_second) = on(b, c, 0.0, f32::INFINITY);
        if p.distance_squared(at_first) <= p.distance_squared(at_second) {
            (first, p - at_first)
        } else {
            (upper + second, p - at_second)
        }
    }
}

/// How each corner of the cloth round `line` must move for the limb to be
/// at the middle of it: nothing for a corner that isn't that limb's.
/// `other` is how far a point is from the nearest other part of the body
/// (the trunk's skin, another limb's line).
fn moves_onto(
    line: &Line,
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    other: &dyn Fn(Vec3) -> f32,
) -> Vec<Vec3> {
    let none = vec![Vec3::ZERO; positions.len()];
    let length = line.length();
    if length < 1e-4 {
        return none;
    }
    let from = FROM * length;
    // Corners at one place are one point.
    let mut places: BTreeMap<[i64; 3], u32> = BTreeMap::new();
    let point_of: Vec<u32> = positions
        .iter()
        .map(|p| {
            let key = (*p * 1e5).round().as_i64vec3().to_array();
            let next = places.len() as u32;
            *places.entry(key).or_insert(next)
        })
        .collect();
    let count = places.len();
    let mut at = vec![Vec3::ZERO; count];
    for (corner, point) in point_of.iter().enumerate() {
        at[*point as usize] = positions[corner];
    }
    let placed: Vec<(f32, Vec3)> = at.iter().map(|p| line.place(*p)).collect();
    // The cloth along the limb, past where it leaves the trunk.
    let along_it: Vec<bool> = placed
        .iter()
        .zip(&at)
        .map(|((along, off), p)| {
            *along >= from && off.length() <= ROUND && off.length() < other(*p)
        })
        .collect();
    // Its pieces, joined over the garment's own surface.
    let mut piece: Vec<u32> = (0..count as u32).collect();
    fn root(piece: &mut [u32], mut at: u32) -> u32 {
        while piece[at as usize] != at {
            piece[at as usize] = piece[piece[at as usize] as usize];
            at = piece[at as usize];
        }
        at
    }
    for t in triangles {
        let t = t.map(|v| point_of[v as usize]);
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if along_it[a as usize] && along_it[b as usize] {
                let (ra, rb) = (root(&mut piece, a), root(&mut piece, b));
                piece[ra as usize] = rb;
            }
        }
    }
    let mut pieces: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for point in 0..count as u32 {
        if along_it[point as usize] {
            let r = root(&mut piece, point);
            pieces.entry(r).or_default().push(point);
        }
    }
    // The limb's own: every piece near its line (a sleeve quilted in
    // bands, or cut at the elbow, is several).
    let middle_off = |points: &[u32]| {
        let mut offs: Vec<f32> = points
            .iter()
            .map(|point| placed[*point as usize].1.length())
            .collect();
        offs.sort_by(f32::total_cmp);
        offs[offs.len() / 2]
    };
    let own: Vec<u32> = pieces
        .into_values()
        .filter(|points| points.len() >= LEAST && middle_off(points) <= NEAR)
        .flatten()
        .collect();
    if own.is_empty() {
        return none;
    }
    // Rings along the limb: each one's middle, from the limb's line.
    let ring_of = |along: f32| ((along - from) / RING).floor().max(0.0) as usize;
    let rings = own
        .iter()
        .map(|point| ring_of(placed[*point as usize].0))
        .max()
        .unwrap_or(0)
        + 1;
    let mut spans = vec![[(f32::INFINITY, f32::NEG_INFINITY); 2]; rings];
    let mut counts = vec![0usize; rings];
    for point in &own {
        let (along, off) = placed[*point as usize];
        let ring = ring_of(along);
        counts[ring] += 1;
        for (span, across) in spans[ring].iter_mut().zip(line.across) {
            let at = off.dot(across);
            *span = (span.0.min(at), span.1.max(at));
        }
    }
    let mut middles: Vec<Option<Vec3>> = spans
        .iter()
        .zip(&counts)
        .map(|(span, count)| {
            (*count >= FEWEST).then(|| {
                line.across[0] * (span[0].0 + span[0].1) / 2.0
                    + line.across[1] * (span[1].0 + span[1].1) / 2.0
            })
        })
        .collect();
    if middles.iter().all(Option::is_none) {
        return none;
    }
    // A ring with too few corners takes its neighbour's.
    for ring in 0..rings {
        if middles[ring].is_none() {
            middles[ring] = (1..rings)
                .flat_map(|step| [ring.checked_sub(step), Some(ring + step)])
                .flatten()
                .filter(|other| *other < rings)
                .find_map(|other| middles[other]);
        }
    }
    let mut middles: Vec<Vec3> = middles.into_iter().map(Option::unwrap_or_default).collect();
    if middles.iter().any(|middle| middle.length() > MOST) {
        return none;
    }
    for _ in 0..EVEN {
        let before = middles.clone();
        for ring in 0..rings {
            let (low, high) = (ring.saturating_sub(1), (ring + 1).min(rings - 1));
            middles[ring] = (before[low] + before[ring] * 2.0 + before[high]) / 4.0;
        }
    }
    let middle_at = |along: f32| {
        // Between the rings' own middles.
        let at = ((along - from) / RING - 0.5).clamp(0.0, (rings - 1) as f32);
        let low = at.floor() as usize;
        let high = (low + 1).min(rings - 1);
        middles[low].lerp(middles[high], at - low as f32)
    };
    let mut of_point = vec![Vec3::ZERO; count];
    for point in &own {
        let along = placed[*point as usize].0;
        let fade = ((along - from) / FADE).clamp(0.0, 1.0);
        let fade = fade * fade * (3.0 - 2.0 * fade);
        of_point[*point as usize] = -middle_at(along) * fade;
    }
    point_of
        .iter()
        .map(|point| of_point[*point as usize])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::super::GarmentPiece;
    use super::*;

    /// A tube of cloth round the line from `start` along +X, 0.5 m long,
    /// its middle `sag(x)` under the line.
    fn tube(radius: f32, sag: &dyn Fn(f32) -> f32) -> GarmentMesh {
        let (sides, rings) = (16u32, 26u32);
        let mut positions = Vec::new();
        for ring in 0..rings {
            let x = 0.5 * ring as f32 / (rings - 1) as f32;
            for side in 0..sides {
                let angle = side as f32 / sides as f32 * std::f32::consts::TAU;
                positions.push(Vec3::new(
                    x,
                    radius * angle.cos() - sag(x),
                    radius * angle.sin(),
                ));
            }
        }
        let mut indices = Vec::new();
        for ring in 0..rings - 1 {
            for side in 0..sides {
                let a = ring * sides + side;
                let b = ring * sides + (side + 1) % sides;
                let (c, d) = (a + sides, b + sides);
                indices.extend([a, c, b, b, c, d]);
            }
        }
        GarmentMesh {
            pieces: vec![GarmentPiece {
                material: "Cloth".to_owned(),
                colour: [1.0; 4],
                roughness: 0.8,
                normals: Vec::new(),
                uvs: Vec::new(),
                positions,
                indices,
            }],
        }
    }

    fn line() -> Line {
        Line::new([
            Vec3::ZERO,
            Vec3::new(0.25, 0.0, 0.0),
            Vec3::new(0.5, 0.0, 0.0),
        ])
    }

    /// The farthest the tube's middle is from the line, past `from`.
    fn off_most(positions: &[Vec3], from: f32) -> f32 {
        let mut most = 0.0f32;
        for ring in positions.chunks(16) {
            if ring[0].x < from {
                continue;
            }
            let (low, high) = ring
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), p| {
                    (low.min(p.y), high.max(p.y))
                });
            most = most.max(((low + high) / 2.0).abs());
        }
        most
    }

    #[test]
    fn a_sleeve_that_hangs_beside_the_arm_is_brought_round_it() {
        // Its middle sags 9 cm under the arm past the elbow: the arm is
        // outside a tube 7 cm in radius.
        let sag = |x: f32| 0.09 * ((x - 0.1) / 0.2).clamp(0.0, 1.0);
        let garment = tube(0.07, &sag);
        let mut positions = garment.positions();
        assert!(off_most(&positions, 0.3) > 0.08);
        let moves = moves_onto(&line(), &positions, &garment.triangles(), &|_| {
            f32::INFINITY
        });
        for (p, m) in positions.iter_mut().zip(&moves) {
            *p += *m;
        }
        assert!(
            off_most(&positions, 0.3) < 0.01,
            "{}",
            off_most(&positions, 0.3)
        );
        // By the shoulder nothing moved: the trunk's cloth stays.
        for (p, m) in garment.positions().iter().zip(&moves) {
            if p.x < FROM * 0.5 {
                assert_eq!(*m, Vec3::ZERO);
            }
        }
        // It kept its width.
        let ring = &positions[16 * 20..16 * 21];
        let (low, high) = ring
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), p| {
                (low.min(p.y), high.max(p.y))
            });
        assert!((high - low - 0.14).abs() < 0.01);
    }

    #[test]
    fn cloth_that_is_not_round_the_limb_stays() {
        // A jacket's side: a tube of its own, 22 cm under the arm's line.
        let garment = tube(0.04, &|_| 0.22);
        let moves = moves_onto(&line(), &garment.positions(), &garment.triangles(), &|_| {
            f32::INFINITY
        });
        assert!(moves.iter().all(|m| *m == Vec3::ZERO));
    }

    #[test]
    fn cloth_nearer_the_trunk_than_the_arm_is_not_the_sleeve_s() {
        // A sleeve 9 cm under the arm, and the trunk's skin 3 cm from
        // every corner of it: it is the trunk's cloth.
        let sag = |x: f32| 0.09 * ((x - 0.1) / 0.2).clamp(0.0, 1.0);
        let garment = tube(0.07, &sag);
        let moves = moves_onto(&line(), &garment.positions(), &garment.triangles(), &|_| {
            0.03
        });
        // Only the corners within 3 cm of the arm's line could be the
        // sleeve's, and they are too few to be one.
        assert!(moves.iter().all(|m| *m == Vec3::ZERO));
    }

    #[test]
    fn a_sleeve_already_round_the_arm_stays() {
        let garment = tube(0.07, &|_| 0.0);
        let moves = moves_onto(&line(), &garment.positions(), &garment.triangles(), &|_| {
            f32::INFINITY
        });
        let most = moves.iter().map(|m| m.length()).fold(0.0, f32::max);
        assert!(most < 0.002, "{most}");
    }
}
