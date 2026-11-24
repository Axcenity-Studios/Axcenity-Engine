//! Which body regions a garment covers: what it hides when worn (an item
//! manifest's `hides`) and which slot it fills.

use std::collections::{BTreeMap, BTreeSet};

use glam::Vec3;

use super::BodyMesh;
use super::surface::Surface;
use crate::BaseManifest;

/// A share of a region's triangles a garment must cover to hide it: all of
/// them but a stray few at a seam.
pub const HIDE_SHARE: f32 = 0.995;
/// How far a garment may stand off the skin and still cover it.
const COVER_REACH: f32 = 0.08;
/// How far a ray out of a crease looks for the skin across it or the
/// garment closing over it ([`closed_creases`]).
pub(super) const CREASE_REACH: f32 = 0.15;

/// A body's region map: a region index per texel over its first texture
/// coordinates (0 where no skin is), as `RealisticRegions.png` holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct RegionMap {
    pub width: u32,
    pub height: u32,
    /// One index per texel, rows from the top.
    pub texels: Vec<u8>,
}

impl RegionMap {
    /// The region at texture coordinates `uv` ((0, 0) is the top left).
    pub fn at(&self, uv: [f32; 2]) -> u8 {
        if self.width == 0 || self.height == 0 {
            return 0;
        }
        let x = ((uv[0].rem_euclid(1.0)) * self.width as f32) as u32;
        let y = ((uv[1].rem_euclid(1.0)) * self.height as f32) as u32;
        let (x, y) = (x.min(self.width - 1), y.min(self.height - 1));
        self.texels[(y * self.width + x) as usize]
    }

    /// The map in an image's red channel (`rgba`: 4 bytes a texel).
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> RegionMap {
        RegionMap {
            width,
            height,
            texels: rgba.chunks_exact(4).map(|px| px[0]).collect(),
        }
    }
}

/// Each body triangle's region: the map's value at the middle of the
/// triangle's texture coordinates (0 when the body has none).
pub fn triangle_regions(body: &BodyMesh, map: &RegionMap) -> Vec<u8> {
    body.triangles
        .iter()
        .map(|t| {
            if body.uvs.is_empty() {
                return 0;
            }
            let uv = t
                .iter()
                .map(|&v| body.uvs[v as usize])
                .fold([0.0, 0.0], |a, b| [a[0] + b[0] / 3.0, a[1] + b[1] / 3.0]);
            map.at(uv)
        })
        .collect()
}

/// How much of each region a garment covers: the share of its triangles
/// whose middle has the garment over it (within 8 cm along the skin's
/// normal, not past one of the garment's open edges), by region index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Coverage {
    pub shares: BTreeMap<u8, f32>,
}

impl Coverage {
    pub fn share(&self, region: u8) -> f32 {
        self.shares.get(&region).copied().unwrap_or(0.0)
    }
}

/// The coverage of the garment `surface` (at rest, over the body at
/// rest) of `body`'s regions (`regions`, one per body triangle).
pub fn coverage(body: &BodyMesh, regions: &[u8], surface: &Surface) -> Coverage {
    let under = under_cloth(body, regions, surface, true);
    let mut covered: BTreeMap<u8, (usize, usize)> = BTreeMap::new();
    for (&region, under) in regions.iter().zip(&under) {
        if region == 0 {
            continue;
        }
        let entry = covered.entry(region).or_default();
        entry.1 += 1;
        if under.is_some() {
            entry.0 += 1;
        }
    }
    Coverage {
        shares: covered
            .into_iter()
            .map(|(region, (yes, all))| (region, yes as f32 / all.max(1) as f32))
            .collect(),
    }
}

/// A body triangle under a garment: how far the cloth stands off its
/// middle (metres), and the cloth's triangle and the point on it nearest
/// the middle (`None` for one only shut in a crease: no cloth is its own).
#[derive(Clone, Copy, Debug)]
pub(super) struct Under {
    pub gap: f32,
    pub cloth: Option<(u32, Vec3)>,
}

/// Which body triangles lie under the garment `surface` (both at rest):
/// for each, how far the cloth stands off its middle (metres) and the
/// cloth's triangle and the point on it nearest the middle, or `None`
/// when it isn't covered. A triangle with no region (0) never is. With
/// `creases`, one in a crease the garment closes over
/// ([`closed_creases`]) is too, at 0 and with no cloth of its own.
pub(super) fn under_cloth(
    body: &BodyMesh,
    regions: &[u8],
    surface: &Surface,
    creases: bool,
) -> Vec<Option<Under>> {
    let weld = welded(&surface.positions);
    let open = open_edges(&weld, &surface.triangles);
    let middles: Vec<(Vec3, Vec3)> = body
        .triangles
        .iter()
        .map(|t| {
            let corners = t.map(|v| body.positions[v as usize]);
            let normal = t
                .iter()
                .map(|&v| body.normals[v as usize])
                .sum::<Vec3>()
                .normalize_or_zero();
            ((corners[0] + corners[1] + corners[2]) / 3.0, normal)
        })
        .collect();
    let mut gaps: Vec<Option<Under>> = middles
        .iter()
        .zip(regions)
        .map(|((middle, normal), &region)| {
            if region == 0 {
                return None;
            }
            covers(surface, &weld, &open, *middle, *normal).map(|(gap, triangle, bary)| Under {
                gap,
                cloth: Some((triangle, bary)),
            })
        })
        .collect();
    if creases {
        let under: Vec<bool> = gaps.iter().map(Option::is_some).collect();
        for (t, closed) in closed_creases(body, surface, regions, &middles, &under)
            .into_iter()
            .enumerate()
        {
            if closed && gaps[t].is_none() {
                gaps[t] = Some(Under {
                    gap: 0.0,
                    cloth: None,
                });
            }
        }
    }
    gaps
}

/// How far the garment stands off the skin at `middle` when it covers
/// it, and the cloth's triangle and the point on it nearest `middle`.
fn covers(
    surface: &Surface,
    weld: &[u32],
    open: &BTreeSet<(u32, u32)>,
    middle: Vec3,
    normal: Vec3,
) -> Option<(f32, u32, Vec3)> {
    let hit = surface.closest(middle, COVER_REACH)?;
    let away = hit.point - middle;
    // The garment must be out along the skin's normal from here, not
    // beside it.
    if away.length() > 1e-4 && away.normalize().dot(normal) < 0.5 {
        return None;
    }
    // On one of the garment's open edges: past its hem.
    let tri = surface.triangles[hit.triangle as usize].map(|v| weld[v as usize]);
    let b = hit.bary.to_array();
    for k in 0..3 {
        if b[k] < 1e-4 {
            let (u, v) = (tri[(k + 1) % 3], tri[(k + 2) % 3]);
            if open.contains(&(u.min(v), u.max(v))) {
                return None;
            }
        }
    }
    Some((away.length(), hit.triangle, hit.bary))
}

/// Skin in a crease the garment closes over (the armpit under a sleeve):
/// each patch of body triangles the garment doesn't cover by [`covers`]
/// (joined across their edges) whose every way out is shut, a ray out
/// along each triangle's normal meeting, within [`CREASE_REACH`], the
/// body's skin facing back across the crease or the garment from inside.
/// Such a patch lies between covered skin on every side, and no cloth can
/// lie along most of its normals, which point at the crease's other wall.
/// A patch with one triangle whose way out is open (skin under a hole in
/// the garment, or the bare skin past its hems, which the patch joins) is
/// not one. Returns which triangles are in one.
fn closed_creases(
    body: &BodyMesh,
    garment: &Surface,
    regions: &[u8],
    middles: &[(Vec3, Vec3)],
    under: &[bool],
) -> Vec<bool> {
    let skin = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    // Is the way out along the triangle's normal shut: by skin facing
    // back across a crease, or by the garment, met from inside?
    let shut = |t: usize| {
        let (middle, normal) = middles[t];
        if normal == Vec3::ZERO {
            return false;
        }
        let from = middle + normal * 1e-3;
        let skin_hit = skin
            .cast(from, normal, CREASE_REACH)
            .filter(|hit| hit.triangle as usize != t && hit.normal.dot(normal) < 0.0);
        let cloth_hit = garment
            .cast(from, normal, CREASE_REACH)
            .filter(|hit| hit.normal.dot(normal) > 0.0);
        skin_hit.is_some() || cloth_hit.is_some()
    };
    // Triangles sharing an edge (corners at one place are one).
    let weld = welded(&body.positions);
    let mut by_edge: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (k, t) in body.triangles.iter().enumerate() {
        let t = t.map(|v| weld[v as usize]);
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            by_edge.entry((u.min(v), u.max(v))).or_default().push(k);
        }
    }
    let mut out = vec![false; body.triangles.len()];
    let mut seen = vec![false; body.triangles.len()];
    for start in 0..body.triangles.len() {
        if seen[start] || under[start] {
            continue;
        }
        // The patch of uncovered triangles joined to `start`.
        let mut patch = vec![start];
        seen[start] = true;
        let mut next = 0;
        while next < patch.len() {
            let k = patch[next];
            next += 1;
            let t = body.triangles[k].map(|v| weld[v as usize]);
            for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                for &other in &by_edge[&(u.min(v), u.max(v))] {
                    if !seen[other] && !under[other] {
                        seen[other] = true;
                        patch.push(other);
                    }
                }
            }
        }
        if patch.iter().all(|&k| regions[k] != 0 && shut(k)) {
            for k in patch {
                out[k] = true;
            }
        }
    }
    out
}

/// Edges with one triangle: a garment's hems and openings. Corners at the
/// same place count as one, so a seam between pieces isn't an opening.
fn open_edges(weld: &[u32], triangles: &[[u32; 3]]) -> BTreeSet<(u32, u32)> {
    let mut uses: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for t in triangles {
        let t = t.map(|v| weld[v as usize]);
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *uses.entry((u.min(v), u.max(v))).or_default() += 1;
        }
    }
    uses.into_iter()
        .filter(|(_, n)| *n == 1)
        .map(|(edge, _)| edge)
        .collect()
}

/// Each corner's first corner at the same place (within a hundredth of a
/// millimetre).
pub(super) fn welded(positions: &[Vec3]) -> Vec<u32> {
    let mut first: BTreeMap<[i64; 3], u32> = BTreeMap::new();
    positions
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let key = (*p * 1e5).round().as_i64vec3().to_array();
            *first.entry(key).or_insert(k as u32)
        })
        .collect()
}

/// The IDs of `base`'s map regions the coverage hides: those covered at
/// least [`HIDE_SHARE`].
pub fn hidden_regions(base: &BaseManifest, coverage: &Coverage) -> Vec<String> {
    base.regions
        .iter()
        .filter_map(|region| {
            let index = region.map_index?;
            (coverage.share(index) >= HIDE_SHARE).then(|| region.id.clone())
        })
        .collect()
}

/// A slot (and layer) for a garment, from what it covers.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotGuess {
    pub slot: String,
    /// Other slots it fills (a dress fills `bottom` too).
    pub also_fills: Vec<String>,
    pub layer: u8,
}

/// Guesses the slot of a garment from the share of each region it covers
/// on `base` (by region ID): the feet make shoes; the torso a top (and a
/// dress when it also covers the thighs); the hips or thighs a bottom.
pub fn classify(base: &BaseManifest, coverage: &Coverage) -> Option<SlotGuess> {
    let share = |id: &str| {
        base.region(id)
            .and_then(|r| r.map_index)
            .map_or(0.0, |index| coverage.share(index))
    };
    let has_slot = |id: &str| base.slots.iter().any(|s| s.id == id);
    let upper = share("torso").max(share("waist"));
    let lower = share("hips").max(share("thighs_top"));
    let guess = |slot: &str, also: &[&str]| SlotGuess {
        slot: slot.to_owned(),
        also_fills: also
            .iter()
            .filter(|s| has_slot(s))
            .map(|s| (*s).to_owned())
            .collect(),
        layer: 1,
    };
    if share("feet") > 0.5 && upper < 0.2 && lower < 0.2 {
        return has_slot("shoes").then(|| guess("shoes", &[]));
    }
    if upper > 0.5 {
        let dress = lower > 0.5 && share("thighs_low") > 0.3;
        return has_slot("top").then(|| guess("top", if dress { &["bottom"] } else { &[] }));
    }
    if lower > 0.5 {
        return has_slot("bottom").then(|| guess("bottom", &[]));
    }
    None
}
