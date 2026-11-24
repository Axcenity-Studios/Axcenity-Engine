//! A top and a bottom worn together (ADR-0045): which is outside round
//! the hips.
//!
//! Every garment is fitted to the body and the underwear alone, so two
//! worn together have no order where they overlap. A top's cover map
//! hides the trousers under it, but not within its setback of its hem,
//! and there trousers standing farther off the skin than the top are
//! drawn in front of it, cut along their own triangles: what looks like
//! bites out of the top's hem. [`hang`] gives the order to the cloth
//! itself, at rest and before the fit, against the garments it will be
//! worn with ([`WornWith`], the library's bottoms):
//!
//! - [`Worn::Over`] (a sweater, a jacket, a coat): its cloth below the
//!   waist is moved out along the skin's normal until it clears the
//!   outside of every one of them;
//! - [`Worn::Tucked`] (a tank top, a polo: cloth that lies on the body):
//!   its cloth below the waist is drawn in under the inside of every one
//!   of them, and the item is given a layer under the bottoms', so their
//!   cover maps hide it from the waistband down.
//!
//! **Only round the hips, never far, and smoothly.** The garments worn
//! with it are measured over the skin as an envelope: at each corner of
//! the skin from the waist to the top of the thighs ([`Hang::below`]),
//! how far along the skin's normal their cloth begins and ends. A corner of the garment is moved
//! only when the skin nearest it is that skin (a sleeve hanging beside
//! the hip is nearest the arm), by at most [`Hang::most`], and the moves
//! are smoothed over the cloth, so a hem opens a little and evenly
//! rather than in a step. Where the cap stops a corner short, the report
//! says so ([`Hung::short`]).
//!
//! **Not for what it can't clear.** A skirt or a wide trouser leg stands
//! several centimetres off the thigh; a sweater's hem moved out as far as
//! the cap allows would still be inside it, and would only have opened
//! into a ruffle for nothing. So each of the garments is taken on its
//! own: one a corner would need more than [`HOPELESS`] times the cap to
//! clear is left out for that corner, and the corner clears the rest.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::surface::{Surface, vertex_normals};
use super::transfer::Welds;

/// How far off the skin cloth worn with the garment is looked for, and
/// how far under it (metres).
const REACH: f32 = 0.10;
const UNDER: f32 = 0.01;
/// Cloth met again within this of where a garment's cloth begins is the
/// same garment's (a pocket, a fold, a waistband turned over); farther
/// is something else (the other leg's).
const FOLD: f32 = 0.025;
/// How far from the skin a corner of the garment is still placed on it.
const NEAR: f32 = 0.3;
/// How many rounds the moves are smoothed over the cloth.
const SMOOTH: usize = 6;
/// A garment a corner would have to move more than this many times the
/// cap to clear isn't hung over at that corner.
const HOPELESS: f32 = 2.0;

/// How a garment is worn with the bottoms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Worn {
    /// Hanging outside them.
    #[default]
    Over,
    /// Tucked into them.
    Tucked,
}

/// A garment worn with the one being fitted: its cloth at rest.
#[derive(Clone, Debug)]
pub struct WornWith {
    pub name: String,
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

/// What [`hang`] is given.
#[derive(Clone, Debug)]
pub struct Hang {
    pub worn: Worn,
    /// The garments it will be worn with (the bottoms).
    pub with: Vec<WornWith>,
    /// Which of the body's triangles are round the hips (from the waist
    /// to the top of the thighs): only cloth nearest these is moved.
    pub below: Vec<bool>,
    /// The clearance kept from the other garments' cloth (metres).
    pub clear: f32,
    /// The farthest a corner is moved (metres).
    pub most: f32,
}

/// What [`hang`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Hung {
    /// How many garments it was hung with.
    pub with: usize,
    /// Corners moved more than a tenth of a millimetre, and the farthest.
    pub moved: usize,
    pub most: f32,
    /// Corners the cap stopped short of clearing the other garments, and
    /// by how much at the most (metres).
    pub short: usize,
    pub short_most: f32,
}

/// One garment's cloth over the skin: at each of the body's corners, how
/// far along the normal it begins and ends (`NAN` where it has none).
struct Envelope {
    inner: Vec<f32>,
    outer: Vec<f32>,
}

fn envelopes(skin: &Surface, hang: &Hang) -> Vec<Envelope> {
    let count = skin.positions.len();
    let mut wanted = vec![false; count];
    for (t, below) in skin.triangles.iter().zip(&hang.below) {
        if *below {
            for &v in t {
                wanted[v as usize] = true;
            }
        }
    }
    let mut all = Vec::new();
    for other in &hang.with {
        let mut inner = vec![f32::NAN; count];
        let mut outer = vec![f32::NAN; count];
        let cloth = Surface::new(
            other.positions.clone(),
            vertex_normals(&other.positions, &other.triangles),
            other.triangles.clone(),
        );
        for v in (0..count).filter(|v| wanted[*v]) {
            let (p, n) = (skin.positions[v], skin.normals[v]);
            // Out from just under the skin: where its cloth begins, and
            // then on through its own folds.
            let Some(first) = cloth.cast(p - n * UNDER, n, UNDER + REACH) else {
                continue;
            };
            let begins = first.distance - UNDER;
            let mut ends = begins;
            for _ in 0..8 {
                let from = ends + 1e-4;
                let left = (begins + FOLD).min(REACH) - from;
                if left <= 0.0 {
                    break;
                }
                match cloth.cast(p + n * from, n, left) {
                    Some(next) => ends = from + next.distance,
                    None => break,
                }
            }
            inner[v] = begins;
            outer[v] = ends;
        }
        all.push(Envelope { inner, outer });
    }
    all
}

/// Moves `positions` (a garment's corners at rest, welded by `welds`) to
/// hang over, or tuck under, the garments of `hang` on `skin` (the body
/// at rest): see the module documentation.
pub(super) fn hang(positions: &mut [Vec3], welds: &Welds, skin: &Surface, hang: &Hang) -> Hung {
    let mut hung = Hung {
        with: hang.with.len(),
        ..Hung::default()
    };
    if hang.with.is_empty() || hang.most <= 0.0 {
        return hung;
    }
    let cloth = envelopes(skin, hang);
    // Each corner's move along the skin's normal under it: out (positive)
    // or in.
    let mut out = vec![Vec3::ZERO; positions.len()];
    let mut need = vec![0.0f32; positions.len()];
    for (k, p) in positions.iter().enumerate() {
        let Some(hit) = skin.closest(*p, NEAR) else {
            continue;
        };
        out[k] = hit.normal;
        if !hang
            .below
            .get(hit.triangle as usize)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }
        let height = hit.signed(*p);
        let corners = skin.triangles[hit.triangle as usize];
        // What each of the other garments asks of this corner: its cloth
        // over this point of the skin, from the triangle's corners that
        // have any.
        let mut wanted = 0.0f32;
        let mut short = 0.0f32;
        for other in &cloth {
            let field = match hang.worn {
                Worn::Over => &other.outer,
                Worn::Tucked => &other.inner,
            };
            let (mut sum, mut total) = (0.0, 0.0);
            for (v, w) in corners.iter().zip(hit.bary.to_array()) {
                let value = field[*v as usize];
                if value.is_finite() {
                    sum += value * w;
                    total += w;
                }
            }
            if total < 1e-3 {
                continue;
            }
            let there = sum / total;
            let asks = match hang.worn {
                Worn::Over => (there + hang.clear - height).max(0.0),
                Worn::Tucked => (there - hang.clear - height).min(0.0),
            };
            if asks.abs() > HOPELESS * hang.most {
                // Out of reach: left as it is.
                short = short.max(asks.abs());
                continue;
            }
            short = short.max(asks.abs() - hang.most);
            if asks.abs() > wanted.abs() {
                wanted = asks;
            }
        }
        need[k] = wanted.clamp(-hang.most, hang.most);
        if short > 1e-4 {
            hung.short += 1;
            hung.short_most = hung.short_most.max(short);
        }
    }
    // Spread to the cloth round each corner and smoothed, so the cloth
    // opens evenly and not corner by corner; and no corner less than it
    // needs.
    let as_field: Vec<Vec3> = need.iter().map(|n| Vec3::new(*n, 0.0, 0.0)).collect();
    let smooth = welds.smooth(&welds.spread(&as_field), &[0.5], SMOOTH);
    for (k, p) in positions.iter_mut().enumerate() {
        let by = match hang.worn {
            Worn::Over => smooth[k].x.max(need[k]).clamp(0.0, hang.most),
            Worn::Tucked => smooth[k].x.min(need[k]).clamp(-hang.most, 0.0),
        };
        if by.abs() > 1e-4 {
            hung.moved += 1;
            hung.most = hung.most.max(by.abs());
        }
        *p += out[k] * by;
    }
    hung
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square sheet of `n` by `n` cells from (0, 0) to (`size`, `size`)
    /// at height `z`, facing +Z.
    fn sheet(n: usize, size: f32, z: f32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let mut positions = Vec::new();
        for j in 0..=n {
            for i in 0..=n {
                positions.push(Vec3::new(
                    size * i as f32 / n as f32,
                    size * j as f32 / n as f32,
                    z,
                ));
            }
        }
        let mut triangles = Vec::new();
        let row = (n + 1) as u32;
        for j in 0..n as u32 {
            for i in 0..n as u32 {
                let a = j * row + i;
                triangles.push([a, a + 1, a + row]);
                triangles.push([a + 1, a + row + 1, a + row]);
            }
        }
        (positions, triangles)
    }

    /// The skin: a sheet at height 0; "below the waist" is its half with
    /// `y` under 0.5.
    fn skin() -> (Surface, Vec<bool>) {
        let (positions, triangles) = sheet(20, 1.0, 0.0);
        let below = triangles
            .iter()
            .map(|t| t.iter().all(|&v| positions[v as usize].y <= 0.5))
            .collect();
        let normals = vec![Vec3::Z; positions.len()];
        (Surface::new(positions, normals, triangles), below)
    }

    /// Trousers: a sheet over the skin's lower half, `off` above it.
    fn trousers(off: f32) -> WornWith {
        let (mut positions, triangles) = sheet(10, 1.0, off);
        for p in &mut positions {
            p.y *= 0.45;
        }
        WornWith {
            name: "trousers".to_owned(),
            positions,
            triangles,
        }
    }

    fn top(off: f32) -> (Vec<Vec3>, Welds) {
        // Off the skin's own grid, so no corner is over a skin corner.
        let (mut positions, triangles) = sheet(24, 0.9, off);
        for p in &mut positions {
            *p += Vec3::new(0.05, 0.05, 0.0);
        }
        let welds = Welds::new(&positions, &triangles);
        (positions, welds)
    }

    fn hang_with(worn: Worn, with: Vec<WornWith>, most: f32) -> Hang {
        let (_, below) = skin();
        Hang {
            worn,
            with,
            below,
            clear: 0.003,
            most,
        }
    }

    #[test]
    fn a_top_under_the_trousers_cloth_is_moved_out_over_it_below_the_waist_only() {
        let (skin, _) = skin();
        let (mut positions, welds) = top(0.01);
        let made = positions.clone();
        let hung = hang(
            &mut positions,
            &welds,
            &skin,
            &hang_with(Worn::Over, vec![trousers(0.02)], 0.03),
        );
        assert!(hung.moved > 0 && hung.short == 0, "{hung:?}");
        for (p, was) in positions.iter().zip(&made) {
            // Only ever out, and never past the cap.
            assert!(p.z >= was.z - 1e-6 && p.z <= was.z + 0.03 + 1e-6);
            if p.y < 0.4 {
                // Over the trousers: clear of them.
                assert!(p.z >= 0.02 + 0.003 - 1e-4, "{p:?}");
            }
            if p.y > 0.8 {
                // The chest: as it was made.
                assert!((p.z - was.z).abs() < 1e-4, "{p:?}");
            }
        }
    }

    #[test]
    fn the_cap_holds_and_the_report_says_what_it_left() {
        let (skin, _) = skin();
        let (mut positions, welds) = top(0.01);
        let hung = hang(
            &mut positions,
            &welds,
            &skin,
            &hang_with(Worn::Over, vec![trousers(0.06)], 0.03),
        );
        // 0.06 + 0.003 - 0.01 is 0.053 wanted: 0.023 short of the cap.
        assert!(hung.short > 0);
        assert!((hung.short_most - 0.023).abs() < 1e-3, "{hung:?}");
        assert!((hung.most - 0.03).abs() < 1e-5);
        assert!(positions.iter().all(|p| p.z <= 0.04 + 1e-5));
    }

    #[test]
    fn a_garment_out_of_reach_is_left_and_the_others_are_still_cleared() {
        let (skin, _) = skin();
        let (mut positions, welds) = top(0.01);
        // A skirt 9 cm off the skin can't be cleared with 3 cm; the
        // trousers at 2 cm can.
        let hung = hang(
            &mut positions,
            &welds,
            &skin,
            &hang_with(Worn::Over, vec![trousers(0.09), trousers(0.02)], 0.03),
        );
        assert!(hung.short > 0 && hung.short_most > 0.08, "{hung:?}");
        for p in positions.iter().filter(|p| p.y < 0.4) {
            // Over the trousers and no farther.
            assert!(p.z >= 0.023 - 1e-4 && p.z < 0.03, "{p:?}");
        }
    }

    #[test]
    fn a_tucked_top_is_drawn_in_under_the_nearest_of_them() {
        let (skin, _) = skin();
        let (mut positions, welds) = top(0.03);
        let made = positions.clone();
        let hung = hang(
            &mut positions,
            &welds,
            &skin,
            &hang_with(Worn::Tucked, vec![trousers(0.05), trousers(0.02)], 0.03),
        );
        assert!(hung.moved > 0);
        for (p, was) in positions.iter().zip(&made) {
            assert!(p.z <= was.z + 1e-6);
            if p.y < 0.4 {
                assert!(p.z <= 0.02 - 0.003 + 1e-4, "{p:?}");
            }
            if p.y > 0.8 {
                assert!((p.z - was.z).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn cloth_already_clear_and_cloth_with_nothing_worn_under_it_stay() {
        let (skin, _) = skin();
        let (mut positions, welds) = top(0.05);
        let made = positions.clone();
        let hung = hang(
            &mut positions,
            &welds,
            &skin,
            &hang_with(Worn::Over, vec![trousers(0.02)], 0.03),
        );
        assert_eq!((hung.moved, hung.short), (0, 0));
        assert_eq!(positions, made);
        let none = hang(
            &mut positions,
            &welds,
            &skin,
            &hang_with(Worn::Over, Vec::new(), 0.03),
        );
        assert_eq!(none, Hung::default());
    }

    #[test]
    fn a_pocket_on_the_trousers_counts_and_cloth_far_past_them_does_not() {
        let (skin, _) = skin();
        // The trousers at 2 cm with a pocket 1.5 cm over them, and
        // something else 8 cm off (the other leg's cloth).
        let mut pocket = trousers(0.035);
        pocket.name = "pocket".to_owned();
        let both = WornWith {
            name: "trousers".to_owned(),
            positions: [trousers(0.02).positions, pocket.positions.clone()].concat(),
            triangles: {
                let base = trousers(0.02);
                let shift = base.positions.len() as u32;
                let mut all = base.triangles.clone();
                all.extend(pocket.triangles.iter().map(|t| t.map(|v| v + shift)));
                all
            },
        };
        let far = trousers(0.08);
        let merged = WornWith {
            name: "trousers".to_owned(),
            positions: [both.positions.clone(), far.positions.clone()].concat(),
            triangles: {
                let shift = both.positions.len() as u32;
                let mut all = both.triangles.clone();
                all.extend(far.triangles.iter().map(|t| t.map(|v| v + shift)));
                all
            },
        };
        let (mut positions, welds) = top(0.01);
        hang(
            &mut positions,
            &welds,
            &skin,
            &hang_with(Worn::Over, vec![merged], 0.05),
        );
        let over: Vec<f32> = positions
            .iter()
            .filter(|p| p.y < 0.4)
            .map(|p| p.z)
            .collect();
        let (low, high) = over
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, h), z| (l.min(*z), h.max(*z)));
        // Over the pocket (3.5 cm and the clearance), not the far cloth.
        assert!(low >= 0.038 - 1e-4 && high < 0.045, "{low} {high}");
    }
}
