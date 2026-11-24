//! Conforming and tidying a placed garment (ADR-0045).
//!
//! Sizing and placing ([`prepare`](super::prepare)) leave a garment the
//! shape it was made: where it was cut for a wider body, or modelled with
//! its arms hanging, it stands off the body unevenly, a sleeve hanging
//! under the arm like a wing. [`conform`] draws it in round the body by
//! how the kind of garment is worn ([`Conform`]):
//!
//! - **Each part of the cloth goes to its own part of the body.** The
//!   body is five parts by its bones (the trunk, each arm, each leg).
//!   Cloth that is clearly nearest one of them belongs to it; the rest
//!   (the underside of a wide sleeve, as near the side as the arm) takes
//!   its shares from the cloth round it, settled over the garment's own
//!   surface. So a sleeve is drawn in round the arm, never to the side
//!   it hangs beside.
//! - **A patch of cloth moves as one.** What is shortened is the patch's
//!   *mean* clearance from the skin (smoothed over the cloth), kept up
//!   to a clearance and shortened past it, and the moves themselves are
//!   smoothed again before they are made. A collar, a cuff, a pocket
//!   flap or a fold standing off the cloth round it moves with that
//!   cloth: the garment's silhouette comes in, its relief doesn't
//!   flatten.
//! - A skirt's cloth under the crotch is left as it hangs.
//!
//! [`tidy`] then cleans what cutting a closed shell open leaves: stray
//! scraps of cloth, teeth along the hems, and the hems' zigzag.

use std::collections::{BTreeMap, BTreeSet};

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::super::parts;
use super::super::transfer::Welds;
use super::super::{BodyMesh, GarmentMesh};
use super::with_triangles;

/// How closely a garment follows the body it is put on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Conform {
    /// As it was made: only sized and placed.
    None,
    /// Keeps its build, loose cloth drawn in a little: jackets, coats,
    /// suits.
    Structured,
    /// Hangs off the body: hoodies, sweaters, sweatpants, wide legs.
    Partial,
    /// Follows the body with room: shirts, jeans, trousers, shorts.
    #[default]
    Moderate,
    /// Lies on the body: T-shirts, tank tops, polos.
    Body,
    /// Second skin: leggings.
    Strong,
}

impl Conform {
    /// The clearance a patch of cloth keeps as made, and how much of
    /// what is past it is kept each time it is drawn in (metres, a
    /// share: over [`PASSES`] passes the share is kept twice over, so
    /// `moderate` keeps a quarter of what stood past its clearance).
    fn rule(self) -> Option<(f32, f32)> {
        match self {
            Conform::None => None,
            Conform::Structured => Some((0.035, 0.80)),
            Conform::Partial => Some((0.025, 0.65)),
            Conform::Moderate => Some((0.015, 0.50)),
            Conform::Body => Some((0.010, 0.40)),
            Conform::Strong => Some((0.004, 0.15)),
        }
    }
}

/// How many times the garment is measured and drawn in.
const PASSES: usize = 2;
/// How many rounds a patch's clearance, and then its moves, are smoothed
/// over the cloth: about a hand's width of a generated garment's mesh.
const SMOOTH: usize = 24;
/// A skirt's cloth is left alone from this far under the crotch, and
/// drawn in fully from this far over it.
const SKIRT_FADE: f32 = 0.06;
/// Draws `garment` (placed on `body`, both at rest) in round the body by
/// `how`. With `skirt_from` (the body's crotch height) cloth under it
/// keeps its shape: a skirt's, a dress's or a coat's that no leg fills.
/// Returns the farthest a corner moved (metres).
pub(super) fn conform(
    garment: &mut GarmentMesh,
    body: &BodyMesh,
    how: Conform,
    skirt_from: Option<f32>,
) -> f32 {
    let Some((keep, share)) = how.rule() else {
        return 0.0;
    };
    let triangles = garment.triangles();
    let parts = parts::of(body);
    let made = garment.positions();
    let mut positions = made.clone();
    let welds = Welds::new(&positions, &triangles);
    let made_normals = welds.normals(&positions, &triangles);
    let scalar = |values: &[f32], rounds: usize| -> Vec<f32> {
        let field: Vec<Vec3> = values.iter().map(|v| Vec3::new(*v, 0.0, 0.0)).collect();
        welds
            .smooth(&field, &[0.7], rounds)
            .into_iter()
            .map(|v| v.x)
            .collect()
    };
    for _ in 0..PASSES {
        // Each corner's place on each part of the body, and its shares of
        // them.
        let (measured, shares) = parts::shares(&parts, &welds, &positions);
        // The clearance of the cloth round each corner, and the way out.
        let mut clearance = vec![0.0f32; positions.len()];
        let mut out = vec![Vec3::ZERO; positions.len()];
        for (k, (near, own)) in measured.iter().zip(&shares).enumerate() {
            let mut total = 0.0;
            for (hit, weight) in near.iter().zip(own) {
                if let Some(hit) = hit {
                    clearance[k] += hit.clearance * weight;
                    out[k] += hit.out * *weight;
                    total += weight;
                }
            }
            if total > 0.0 {
                clearance[k] /= total;
            }
            out[k] = out[k].normalize_or_zero();
        }
        let patch = scalar(&clearance, SMOOTH);
        let moves: Vec<Vec3> = positions
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let over = patch[k] - keep;
                if over <= 0.0 {
                    return Vec3::ZERO;
                }
                let fade = skirt_from.map_or(1.0, |crotch| {
                    ((p.y - (crotch - SKIRT_FADE)) / (2.0 * SKIRT_FADE)).clamp(0.0, 1.0)
                });
                // Never through the skin: a corner comes in no nearer
                // than the clearance kept.
                let most = (clearance[k] - keep).max(0.0);
                -out[k] * (over * (1.0 - share)).min(most) * fade
            })
            .collect();
        let moves = welds.smooth(&moves, &[0.7], SMOOTH);
        for (p, m) in positions.iter_mut().zip(&moves) {
            *p += *m;
        }
    }
    // The garment's own normals turn as far as its surface did.
    let now = welds.normals(&positions, &triangles);
    let turned: Vec<Vec3> = garment
        .normals()
        .iter()
        .zip(made_normals.iter().zip(&now))
        .map(|(n, (was, is))| {
            if *was == Vec3::ZERO || *is == Vec3::ZERO {
                *n
            } else {
                (Quat::from_rotation_arc(*was, *is) * *n).normalize_or_zero()
            }
        })
        .collect();
    let farthest = positions
        .iter()
        .zip(&made)
        .map(|(p, was)| p.distance(*was))
        .fold(0.0, f32::max);
    garment.set_positions(&positions);
    garment.set_normals(&turned);
    farthest
}

/// A piece of cloth with fewer than this share of the garment's triangles
/// that isn't joined to the rest is a scrap, and comes out.
const SCRAP: f32 = 0.01;
/// How many rounds teeth are taken off the edges.
const TEETH: usize = 3;
/// How many rounds an edge's corners are evened along it: each round a
/// step toward the middle of the two neighbours, then a slightly longer
/// one back; and the farthest a corner may move in all (metres).
const EDGE_ROUNDS: usize = 8;
const EDGE_IN: f32 = 0.5;
const EDGE_OUT: f32 = -0.53;
const EDGE_MOST: f32 = 0.02;

/// A piece of cloth no corner of which is within about this of the
/// garment's largest piece floats beside it (metres; the side of the
/// cells they are looked up in).
const FLOATS: f32 = 0.015;

/// What [`tidy`] took off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tidied {
    /// Triangles in scraps of cloth joined to nothing.
    pub scraps: usize,
    /// Teeth: triangles held by one side only.
    pub teeth: usize,
    /// Edge corners evened out.
    pub evened: usize,
}

/// How open a garment's cloth is, as the corners welded see it: its open
/// loops (a neck, a cuff, a hem, and every hole and tear) and the open
/// edges in them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Openings {
    pub loops: usize,
    pub edges: usize,
}

/// Counts `garment`'s open loops and edges.
pub(super) fn openings(garment: &GarmentMesh) -> Openings {
    let (triangles, _) = welded(garment);
    let loops = open_loops(&triangles);
    Openings {
        loops: loops.len(),
        edges: loops.iter().map(Vec::len).sum(),
    }
}

/// The open loops of `triangles` (over welded points): each a list of
/// open edges that share corners. An edge is open when one triangle only
/// has it.
pub(super) fn open_loops(triangles: &[[u32; 3]]) -> Vec<Vec<(u32, u32)>> {
    let mut uses: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for t in triangles {
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            continue;
        }
        for side in sides(t) {
            *uses.entry(side).or_default() += 1;
        }
    }
    let open: Vec<(u32, u32)> = uses
        .into_iter()
        .filter(|(_, count)| *count == 1)
        .map(|(side, _)| side)
        .collect();
    let mut parent: BTreeMap<u32, u32> = BTreeMap::new();
    fn root(parent: &mut BTreeMap<u32, u32>, mut at: u32) -> u32 {
        loop {
            let up = *parent.entry(at).or_insert(at);
            if up == at {
                return at;
            }
            at = up;
        }
    }
    for (a, b) in &open {
        let (ra, rb) = (root(&mut parent, *a), root(&mut parent, *b));
        if ra != rb {
            parent.insert(ra, rb);
        }
    }
    let mut loops: BTreeMap<u32, Vec<(u32, u32)>> = BTreeMap::new();
    for side in open {
        let r = root(&mut parent, side.0);
        loops.entry(r).or_default().push(side);
    }
    loops.into_values().collect()
}

/// Each triangle's corners as welded points, and the points.
fn welded(garment: &GarmentMesh) -> (Vec<[u32; 3]>, Vec<u32>) {
    let mut first: BTreeMap<[i64; 3], u32> = BTreeMap::new();
    let of_corner: Vec<u32> = garment
        .positions()
        .iter()
        .map(|p| {
            let key = (*p * 1e5).round().as_i64vec3().to_array();
            let next = first.len() as u32;
            *first.entry(key).or_insert(next)
        })
        .collect();
    let triangles = garment
        .triangles()
        .iter()
        .map(|t| t.map(|v| of_corner[v as usize]))
        .collect();
    (triangles, of_corner)
}

fn sides(t: &[u32; 3]) -> [(u32, u32); 3] {
    [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])].map(|(a, b)| (a.min(b), a.max(b)))
}

/// Cleans the cut edges of `garment`: scraps of cloth joined to nothing
/// come out, then teeth (a triangle two of whose sides are open edges
/// stands out of the hem as a tooth, or is the tip of a tear), and the
/// open edges' corners are evened along the edge so a hem is a line and
/// not a zigzag (without drawing the hem in). The cloth away from the edges isn't touched.
pub(super) fn tidy(garment: &mut GarmentMesh) -> Tidied {
    let mut tidied = Tidied::default();
    // Scraps.
    let (triangles, _) = welded(garment);
    if triangles.is_empty() {
        return tidied;
    }
    let points = triangles.iter().flatten().copied().max().unwrap_or(0) as usize + 1;
    let mut parent: Vec<u32> = (0..points as u32).collect();
    fn root(parent: &mut [u32], mut at: u32) -> u32 {
        while parent[at as usize] != at {
            parent[at as usize] = parent[parent[at as usize] as usize];
            at = parent[at as usize];
        }
        at
    }
    for t in &triangles {
        let a = root(&mut parent, t[0]);
        for &other in &t[1..] {
            let b = root(&mut parent, other);
            parent[b as usize] = a;
        }
    }
    let mut sizes: BTreeMap<u32, usize> = BTreeMap::new();
    let roots: Vec<u32> = triangles.iter().map(|t| root(&mut parent, t[0])).collect();
    for r in &roots {
        *sizes.entry(*r).or_default() += 1;
    }
    let least = (triangles.len() as f32 * SCRAP) as usize;
    let largest = sizes.values().copied().max().unwrap_or(0);
    // A smaller piece that touches nothing of the largest floats beside
    // the garment (a bit of a pit's floor left past a cuff): it goes too,
    // whatever its size. A button or a flap lies on the cloth, and stays.
    let positions = garment.positions();
    let of_corner = welded(garment).1;
    let mut at = vec![Vec3::ZERO; points];
    for (corner, point) in of_corner.iter().enumerate() {
        at[*point as usize] = positions[corner];
    }
    let main = sizes
        .iter()
        .max_by_key(|(_, size)| **size)
        .map(|(piece, _)| *piece);
    let cell = |p: Vec3| (p / FLOATS).floor().as_ivec3().to_array();
    let mut near_main: BTreeSet<[i32; 3]> = BTreeSet::new();
    for (t, piece) in triangles.iter().zip(&roots) {
        if Some(*piece) == main {
            for &point in t {
                let [x, y, z] = cell(at[point as usize]);
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        for dz in -1..=1 {
                            near_main.insert([x + dx, y + dy, z + dz]);
                        }
                    }
                }
            }
        }
    }
    let mut touches: BTreeSet<u32> = BTreeSet::new();
    for (t, piece) in triangles.iter().zip(&roots) {
        if t.iter()
            .any(|point| near_main.contains(&cell(at[*point as usize])))
        {
            touches.insert(*piece);
        }
    }
    let kept: Vec<u32> = (0..triangles.len() as u32)
        .filter(|k| {
            let piece = roots[*k as usize];
            let size = sizes[&piece];
            size == largest || (size > least && touches.contains(&piece))
        })
        .collect();
    tidied.scraps = triangles.len() - kept.len();
    if tidied.scraps > 0 {
        *garment = with_triangles(garment, &kept);
    }
    // Teeth.
    for _ in 0..TEETH {
        let (triangles, _) = welded(garment);
        let mut uses: BTreeMap<(u32, u32), u32> = BTreeMap::new();
        for t in &triangles {
            for side in sides(t) {
                *uses.entry(side).or_default() += 1;
            }
        }
        let kept: Vec<u32> = (0..triangles.len() as u32)
            .filter(|k| {
                let open = sides(&triangles[*k as usize])
                    .iter()
                    .filter(|side| uses[*side] == 1)
                    .count();
                open < 2
            })
            .collect();
        let teeth = triangles.len() - kept.len();
        if teeth == 0 {
            break;
        }
        tidied.teeth += teeth;
        *garment = with_triangles(garment, &kept);
    }
    // The edges evened: each edge corner with two neighbours along the
    // edge goes half way to their middle, a few times.
    let (triangles, of_corner) = welded(garment);
    let mut uses: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for t in &triangles {
        for side in sides(t) {
            *uses.entry(side).or_default() += 1;
        }
    }
    let mut along: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for ((a, b), count) in &uses {
        if *count == 1 {
            along.entry(*a).or_default().insert(*b);
            along.entry(*b).or_default().insert(*a);
        }
    }
    let mut positions = garment.positions();
    let count = of_corner
        .iter()
        .copied()
        .max()
        .map_or(0, |m| m as usize + 1);
    let mut at = vec![Vec3::ZERO; count];
    for (corner, point) in of_corner.iter().enumerate() {
        at[*point as usize] = positions[corner];
    }
    let made = at.clone();
    // Two steps a round, in and back out (Taubin's smoothing): a zigzag
    // goes, and a loop keeps its size, where drawing each corner toward
    // its neighbours alone would shrink a hem onto the leg inside it.
    for _ in 0..EDGE_ROUNDS {
        for step in [EDGE_IN, EDGE_OUT] {
            let before = at.clone();
            for (point, neighbours) in &along {
                if neighbours.len() != 2 {
                    continue;
                }
                let middle = neighbours.iter().map(|n| before[*n as usize]).sum::<Vec3>() / 2.0;
                let here = before[*point as usize];
                let wanted = here + (middle - here) * step;
                let from = made[*point as usize];
                at[*point as usize] = from + (wanted - from).clamp_length_max(EDGE_MOST);
            }
        }
    }
    tidied.evened = along
        .keys()
        .filter(|point| at[**point as usize].distance(made[**point as usize]) > 1e-5)
        .count();
    for (corner, point) in of_corner.iter().enumerate() {
        positions[corner] = at[*point as usize];
    }
    garment.set_positions(&positions);
    tidied
}
