//! Character Studio refinement, wave 3, R4's second pass (RA2): brows that
//! read as hair, a more feminine feminine face, a heavier masculine brow
//! ridge in profile.
//!
//! The references: `docs/development/character-studio/references/face/
//! README.md` (population data; F-29, eyebrow heights and length by sex;
//! F-30, the nasofrontal angle by sex) and `references/hair/README.md`
//! (brows are hair).
//!
//! These read the shipped package (`assets/characters/realistic/`):
//! rebuild it after changing the face (its README).

use std::path::{Path, PathBuf};

use axcenity_assets::{Model, ModelFormat, Pick, load_models};
use axcenity_avatar::{Description, Library, resolve};

const BASE: &str = "axcenity/realistic";
const BROWS: &str = "AxcenityRealistic/Brows";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn library() -> Library {
    let (library, _, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{problems:?}");
    library
}

fn object(library: &Library, path: &str) -> Model {
    let base = library.base(BASE).unwrap();
    let bytes = std::fs::read(root().join("realistic").join(&base.model.file)).unwrap();
    let picks = [Pick::Objects(vec![path.to_owned()])];
    load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
        .unwrap()
        .remove(0)
}

fn build_data() -> serde_json::Value {
    let text = std::fs::read_to_string(root().join("realistic/Realistic.build.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Every vertex of `model` (all primitives, in order) and its triangles,
/// at rest on `frame` (0 masculine, 1 feminine), in metres (glTF: Y up,
/// the face toward +Z).
fn posed(library: &Library, model: &Model, frame: f32) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    posed_with(library, model, frame, &[])
}

/// As [`posed`], with parameters set (`(id, value)`).
fn posed_with(
    library: &Library,
    model: &Model,
    frame: f32,
    parameters: &[(&str, f32)],
) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    let mut description = Description::new("Frame", BASE);
    description.parameters.insert("frame".into(), frame);
    for &(id, value) in parameters {
        description.parameters.insert(id.into(), value);
    }
    posed_as(library, model, &description)
}

/// As [`posed`], for a whole description.
fn posed_as(
    library: &Library,
    model: &Model,
    description: &Description,
) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    let resolved = resolve(library, description).unwrap();
    let baked = model.bake(&resolved.identity);
    let rig = baked.rig.as_ref().unwrap();
    let posed = baked.pose_shaped(&rig.rest_pose(), &[]).unwrap();
    let size = model.size;
    let mut points = Vec::new();
    let mut tris = Vec::new();
    for k in 0..baked.primitives.len() {
        let base = points.len();
        points.extend(
            posed
                .primitive(&baked, k, 0)
                .positions
                .iter()
                .map(|p| [p[0] * size[0], p[1] * size[1], p[2] * size[2]]),
        );
        tris.extend(baked.primitives[k].indices.chunks_exact(3).map(|t| {
            [
                base + t[0] as usize,
                base + t[1] as usize,
                base + t[2] as usize,
            ]
        }));
    }
    (points, tris)
}

/// The brow texture: its pixels (RGBA, 8 bits) and size.
fn brow_texture(library: &Library) -> (Vec<u8>, usize, usize) {
    let model = object(library, BROWS);
    let image = model.primitives[0]
        .material
        .base_color_texture
        .clone()
        .expect("the brows' card has a hair texture");
    (
        image.rgba().to_vec(),
        image.width as usize,
        image.height as usize,
    )
}

/// The lead's review of wave 2: the brows were heavy dark cards. A brow is
/// hair: graded density (sparse at the inner end, densest in the body,
/// thinning to the tail), soft edges with single hairs at them (most of
/// the covered texels part-covered, not a solid band), and a natural dark
/// brown from the hair's melanin, not black, with hairs varying in colour.
#[test]
fn the_brow_texture_is_hair_not_a_card() {
    let library = library();
    let (rgba, w, h) = brow_texture(&library);
    let alpha = |x: usize, y: usize| f64::from(rgba[(y * w + x) * 4 + 3]) / 255.0;
    // Coverage by column (the card runs from the inner end, u 0, to the
    // tail, u 1).
    let column = |x: usize| (0..h).map(|y| alpha(x, y)).sum::<f64>() / h as f64;
    let span = |from: f64, to: f64| {
        let (a, b) = ((from * w as f64) as usize, (to * w as f64) as usize);
        (a..b).map(column).sum::<f64>() / (b - a) as f64
    };
    let body = (2..14)
        .map(|k| span(k as f64 * 0.05, k as f64 * 0.05 + 0.1))
        .fold(0.0, f64::max);
    let head = span(0.0, 0.08);
    let tail = span(0.85, 1.0);
    assert!(
        head < 0.75 * body,
        "the inner end is as dense as the body: {head:.3} against {body:.3}"
    );
    assert!(
        tail < 0.30 * body,
        "the tail doesn't thin out: {tail:.3} against {body:.3}"
    );
    // Not a solid band: at least a fifth of the covered texels are
    // part-covered (single hairs, soft edges; wave 2's solid band 7%), and
    // the densest part covers under half the card. (Review fixes 1: the
    // first pass's sparser brow, 0.34 of the card and 32% part-covered,
    // read as a faint line at face distance; these bounds are set from
    // the denser brow that reads in the engine, 0.44 and 24%.)
    assert!(body < 0.50, "the body covers {body:.3} of the card");
    let covered: Vec<f64> = (0..w * h)
        .map(|i| f64::from(rgba[i * 4 + 3]) / 255.0)
        .filter(|&a| a > 0.05)
        .collect();
    let partial = covered.iter().filter(|&&a| a < 0.5).count() as f64 / covered.len() as f64;
    assert!(
        partial > 0.20,
        "only {:.0}% of the covered texels are part-covered",
        partial * 100.0
    );
    // Soft edges across the brow: in the body (u 0.2 to 0.6) the rows'
    // coverage rises from a tenth of its most to nine tenths over at least
    // an eighth of the card's height on each side.
    let rows: Vec<f64> = (0..h)
        .map(|y| {
            let (a, b) = (w / 5, 3 * w / 5);
            (a..b).map(|x| alpha(x, y)).sum::<f64>() / (b - a) as f64
        })
        .collect();
    let most = rows.iter().cloned().fold(0.0, f64::max);
    let first = |level: f64| rows.iter().position(|&r| r > level * most).unwrap();
    let last = |level: f64| h - 1 - rows.iter().rev().position(|&r| r > level * most).unwrap();
    // The body's core reads as hair at face distance: the densest rows
    // cover most of the card there (the first pass's 0.24 to 0.34 drew a
    // faint line the review could barely see).
    assert!(
        most > 0.7,
        "the body's densest rows cover only {most:.2} of the card"
    );
    // No card edge: nothing drawn on the card's border texels.
    let border = (0..w)
        .flat_map(|x| [alpha(x, 0), alpha(x, 1), alpha(x, h - 2), alpha(x, h - 1)])
        .chain((0..h).flat_map(|y| [alpha(0, y), alpha(w - 1, y)]))
        .fold(0.0, f64::max);
    assert!(
        border < 0.1,
        "the hair reaches the card's edge ({border:.2})"
    );
    let upper = (first(0.9) - first(0.1)) as f64 / h as f64;
    let lower = (last(0.1) - last(0.9)) as f64 / h as f64;
    assert!(
        upper > 0.125 && lower > 0.125,
        "hard edges: the coverage rises over {upper:.3} and {lower:.3} of the card"
    );
    // The colour, where the hair is dense (alpha over a half): dark brown
    // (sRGB red 55 to 110, red over blue by 22 or more), hairs varying.
    let dense: Vec<[f64; 3]> = (0..w * h)
        .filter(|&i| rgba[i * 4 + 3] > 127)
        .map(|i| std::array::from_fn(|c| f64::from(rgba[i * 4 + c])))
        .collect();
    let n = dense.len() as f64;
    let mean: [f64; 3] = std::array::from_fn(|c| dense.iter().map(|p| p[c]).sum::<f64>() / n);
    assert!(
        (55.0..=110.0).contains(&mean[0]) && mean[0] - mean[2] >= 22.0 && mean[0] > mean[1],
        "the brows' colour is {mean:?} (sRGB), not a dark brown"
    );
    let spread = (dense.iter().map(|p| (p[0] - mean[0]).powi(2)).sum::<f64>() / n).sqrt();
    assert!(
        spread > 4.0,
        "every hair the same colour (red SD {spread:.1})"
    );
}

/// The brow card of each side seen from the front, cut into twelve
/// columns from the inner end to the tail: each column's height (the
/// card's extent up and down) and its middle's height, in metres.
fn brow_columns(points: &[[f32; 3]], left: bool) -> Vec<(f32, f32)> {
    let side: Vec<&[f32; 3]> = points.iter().filter(|p| (p[0] > 0.0) == left).collect();
    let inner = side.iter().map(|p| p[0].abs()).fold(f32::MAX, f32::min);
    let outer = side.iter().map(|p| p[0].abs()).fold(f32::MIN, f32::max);
    (0..12)
        .filter_map(|k| {
            let from = inner + (outer - inner) * k as f32 / 12.0;
            let to = inner + (outer - inner) * (k + 1) as f32 / 12.0;
            let ys: Vec<f32> = side
                .iter()
                .filter(|p| p[0].abs() >= from && p[0].abs() <= to)
                .map(|p| p[1])
                .collect();
            if ys.is_empty() {
                return None;
            }
            let low = ys.iter().cloned().fold(f32::MAX, f32::min);
            let high = ys.iter().cloned().fold(f32::MIN, f32::max);
            Some((high - low, 0.5 * (high + low)))
        })
        .collect()
}

/// The card's height across its own line at each of its segments, from the
/// inner end to the tail (metres): the distance between its lower-edge
/// and upper-edge vertices (the texture's v 0 and 1) at each u, on one
/// side (`left`: the body's left, +x).
fn card_heights(points: &[[f32; 3]], uvs: &[[f32; 2]], left: bool) -> Vec<f32> {
    let mut by_u: std::collections::BTreeMap<i64, Vec<(f32, [f32; 3])>> = Default::default();
    for (p, uv) in points.iter().zip(uvs) {
        if (p[0] > 0.0) == left {
            // (u runs in steps of a segment; v from one edge to the other.)
            let key = (uv[0] * 1000.0).round() as i64;
            by_u.entry(key).or_default().push((uv[1], *p));
        }
    }
    by_u.values()
        .map(|column| {
            let low = column.iter().min_by(|a, b| a.0.total_cmp(&b.0)).unwrap().1;
            let high = column.iter().max_by(|a, b| a.0.total_cmp(&b.0)).unwrap().1;
            ((high[0] - low[0]).powi(2) + (high[1] - low[1]).powi(2) + (high[2] - low[2]).powi(2))
                .sqrt()
        })
        .collect()
}

/// How far the middle of the brow's card rises over the straight line
/// between its two ends (front view), in metres.
fn arch(columns: &[(f32, f32)]) -> f32 {
    let (first, last) = (columns[0].1, columns[columns.len() - 1].1);
    let n = (columns.len() - 1) as f32;
    columns
        .iter()
        .enumerate()
        .map(|(k, &(_, mid))| mid - (first + (last - first) * k as f32 / n))
        .fold(f32::MIN, f32::max)
}

fn area(points: &[[f32; 3]], tris: &[[usize; 3]]) -> f32 {
    tris.iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| points[i]);
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let x = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            0.5 * (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt()
        })
        .sum()
}

/// The card tapers from head to tail on both frames; the feminine brows
/// are thinner (a smaller card) and more arched, the masculine fuller and
/// flatter (F-29: the brows' lower edge stands about as high over the
/// inner eye corner in men and women, 2.6 mm higher over the outer one in
/// women).
#[test]
fn the_brows_taper_and_differ_by_frame() {
    let library = library();
    let model = object(&library, BROWS);
    let (m, tris) = posed(&library, &model, 0.0);
    let (f, _) = posed(&library, &model, 1.0);
    let uvs = &model.primitives[0].uvs;
    assert_eq!(model.primitives.len(), 1, "the brows are one card");
    let mut arches = [0.0f32; 2];
    for (k, points) in [&m, &f].into_iter().enumerate() {
        for left in [true, false] {
            // The card's height across its own line at each segment (the
            // review of the first pass: a front view's columns let the
            // foreshortening round the temple pass for a taper, and
            // wave 2's square-ended card passed).
            let heights = card_heights(points, uvs, left);
            let head = heights[1];
            let tail = heights[heights.len() - 1];
            println!(
                "frame {k} {}: card {:.1} mm high at its head, {:.1} at its tail",
                if left { "left" } else { "right" },
                head * 1000.0,
                tail * 1000.0
            );
            assert!(
                tail < 0.6 * head,
                "frame {k}: the card's tail is {:.1} mm high against {:.1} mm at its head",
                tail * 1000.0,
                head * 1000.0
            );
            arches[k] += 0.5 * arch(&brow_columns(points, left));
        }
    }
    let (am, af) = (area(&m, &tris), area(&f, &tris));
    println!(
        "brow cards: {:.0} and {:.0} mm2, arches {:.1} and {:.1} mm",
        am * 1e6,
        af * 1e6,
        arches[0] * 1000.0,
        arches[1] * 1000.0
    );
    assert!(
        af < 0.8 * am,
        "the feminine brows' card is {:.0} mm2 against the masculine {:.0}",
        af * 1e6,
        am * 1e6
    );
    assert!(
        arches[1] > arches[0] + 0.0015,
        "the feminine brows arch {:.1} mm, the masculine {:.1} mm",
        arches[1] * 1000.0,
        arches[0] * 1000.0
    );
}

/// The brows' place against the eyes (the build measures it on the
/// hair: where the texture's coverage begins under the brow), within two
/// SD of F-29's young Caucasian adults: the lower edge over the inner eye
/// corner (men 17.4 +- 2.0 mm, women 17.5 +- 2.3) and over the outer one
/// (men 15.9 +- 2.9, women 18.5 +- 2.9), the feminine at least 1.5 mm
/// higher over the outer corner than the masculine. The length, as F-29
/// measures it (a path along the hair's lower edge from the first hair to
/// the last; the build's columns with 4% of the most hair), within two SD
/// of F-29 (men 69.5 +- 5.8, women 61.4 +- 6.1), the feminine 0.80 to
/// 0.96 of the masculine (F-29: 0.88). The hair's height across the brow
/// (where a column's coverage is over 30% of its most): F-29 has no
/// thickness, but F-12's upper edge over the inner eye corner (women
/// 26.4 +- 3.1 mm) less F-29's lower edge there (17.5) puts a woman's
/// brow about 8.9 mm high at its head; the feminine head within 3 mm of
/// that, the masculine brow fuller (at least 1.15 times as high at its
/// head and in its body), both thinning to the tail.
#[test]
fn the_brows_sit_where_the_references_put_them() {
    let data = build_data();
    let brows = &data["face"]["brows"];
    let get = |frame: &str, key: &str| {
        brows[frame][key]
            .as_f64()
            .unwrap_or_else(|| panic!("no brows.{frame}.{key} in the build data"))
    };
    for (frame, en, ex) in [
        ("masculine", (17.36, 1.99), (15.92, 2.89)),
        ("feminine", (17.52, 2.32), (18.53, 2.86)),
    ] {
        for (key, (mean, sd)) in [("over_inner_corner_mm", en), ("over_outer_corner_mm", ex)] {
            let value = get(frame, key);
            assert!(
                (value - mean).abs() <= 2.0 * sd,
                "{frame} brows: {key} {value:.1} (reference {mean} +- {sd})"
            );
        }
    }
    let rise = get("feminine", "over_outer_corner_mm") - get("masculine", "over_outer_corner_mm");
    assert!(rise >= 1.5, "the feminine tail is only {rise:.1} mm higher");
    for (frame, (mean, sd)) in [("masculine", (69.5, 5.8)), ("feminine", (61.4, 6.1))] {
        let value = get(frame, "length_mm");
        assert!(
            (value - mean).abs() <= 2.0 * sd,
            "{frame} brows: {value:.1} mm long (F-29 {mean} +- {sd})"
        );
    }
    let ratio = get("feminine", "length_mm") / get("masculine", "length_mm");
    assert!(
        (0.80..=0.96).contains(&ratio),
        "the feminine brow is {ratio:.2} of the masculine's length"
    );
    let tall = |frame: &str| -> Vec<f64> {
        brows[frame]["hair_height_mm"]
            .as_array()
            .unwrap_or_else(|| panic!("no brows.{frame}.hair_height_mm in the build data"))
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    };
    let (m, f) = (tall("masculine"), tall("feminine"));
    println!("hair height (head, body, tail): masculine {m:?}, feminine {f:?} mm");
    assert!(
        (f[0] - 8.9).abs() <= 3.0,
        "the feminine brow's hair is {:.1} mm high at its head (about 8.9)",
        f[0]
    );
    for k in 0..2 {
        assert!(
            m[k] >= 1.15 * f[k],
            "the masculine brow is not fuller: {:.1} mm against {:.1} ({})",
            m[k],
            f[k],
            ["head", "body"][k]
        );
    }
    for (frame, t) in [("masculine", &m), ("feminine", &f)] {
        assert!(
            t[2] < 0.7 * t[0],
            "the {frame} brow's hair doesn't thin to the tail: {t:?}"
        );
    }
}

/// The face's midline profile (front-most point per millimetre of height),
/// from 4 cm under the nose's tip to 8 cm over it: (height, forward).
fn profile(points: &[[f32; 3]]) -> Vec<(f32, f32)> {
    let top = points.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    let head: Vec<&[f32; 3]> = points
        .iter()
        .filter(|p| p[1] > top - 0.24 && p[1] < top - 0.04 && p[0].abs() < 0.005)
        .collect();
    let tip = **head.iter().max_by(|a, b| a[2].total_cmp(&b[2])).unwrap();
    // (Only the face's front: the back of the head is on the midline too.)
    let head: Vec<&[f32; 3]> = head.into_iter().filter(|p| p[2] > tip[2] - 0.06).collect();
    let bins: Vec<(f32, f32)> = (0..120)
        .filter_map(|k| {
            let y = tip[1] - 0.04 + k as f32 * 0.001;
            head.iter()
                .filter(|p| (p[1] - y).abs() < 0.001)
                .map(|p| p[2])
                .fold(None, |most: Option<f32>, z| {
                    Some(most.map_or(z, |m| m.max(z)))
                })
                .map(|z| (y, z))
        })
        .collect();
    // (A height with no vertex of the skin's front near it finds one
    // inside the nose or the mouth: dropped where it lies more than 4 mm
    // behind the front within 3 mm above and below.)
    bins.iter()
        .cloned()
        .filter(|&(y, z)| {
            let front = bins
                .iter()
                .filter(|b| (b.0 - y).abs() < 0.003)
                .map(|b| b.1)
                .fold(f32::MIN, f32::max);
            z > front - 0.004
        })
        .collect()
}

/// (Sellion, glabella, the nasofrontal angle in degrees): sellion the
/// deepest point of the profile over the nose's tip, glabella the most
/// forward one over sellion (within 3 cm), the angle at sellion between
/// glabella and the dorsum a third of the way down to the tip.
fn nasion_and_glabella(profile: &[(f32, f32)]) -> ((f32, f32), (f32, f32), f32) {
    let tip = *profile.iter().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    // Over the tip, the profile goes back to sellion, forward to glabella.
    let above: Vec<(f32, f32)> = profile.iter().cloned().filter(|p| p.0 > tip.0).collect();
    let mut sellion = above[0];
    for &p in &above {
        if p.0 > tip.0 + 0.06 {
            break;
        }
        if p.1 < sellion.1 {
            sellion = p;
        }
    }
    let glabella = above
        .iter()
        .cloned()
        .filter(|p| p.0 > sellion.0 && p.0 < sellion.0 + 0.03)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    let dorsum_y = sellion.0 - (sellion.0 - tip.0) / 3.0;
    let dorsum = *profile
        .iter()
        .min_by(|a, b| (a.0 - dorsum_y).abs().total_cmp(&(b.0 - dorsum_y).abs()))
        .unwrap();
    let to = |p: (f32, f32)| (p.0 - sellion.0, p.1 - sellion.1);
    let (g, d) = (to(glabella), to(dorsum));
    let cos = (g.0 * d.0 + g.1 * d.1) / ((g.0.hypot(g.1)) * (d.0.hypot(d.1)));
    (sellion, glabella, cos.clamp(-1.0, 1.0).acos().to_degrees())
}

/// The lead's choice for wave 3: a moderately heavier masculine brow ridge
/// in profile. F-30 (142 adults): the nasofrontal angle is 138.5 +- 9.3
/// degrees in men and 141.6 +- 8.1 in women. Glabella stands at least
/// 5 mm forward of sellion on the masculine frame (3.7 mm in wave 2) and
/// at most 1.5 mm on the feminine; each angle is within its sex's
/// mean +- 1.5 SD (moderate: wave 2's masculine angle was 131.3, 0.8 SD
/// under the mean), the feminine at least 3 degrees the more open.
#[test]
fn the_masculine_brow_ridge_stands_forward_in_profile() {
    let library = library();
    let base = library.base(BASE).unwrap();
    let model = object(&library, &base.model.body);
    let (m, _) = posed(&library, &model, 0.0);
    let (f, _) = posed(&library, &model, 1.0);
    let (ms, mg, ma) = nasion_and_glabella(&profile(&m));
    let (fs, fg, fa) = nasion_and_glabella(&profile(&f));
    println!(
        "sellion, glabella, angle: masculine {ms:?} {mg:?} {ma:.1}; feminine {fs:?} {fg:?} {fa:.1}"
    );
    let ridge_m = (mg.1 - ms.1) * 1000.0;
    let ridge_f = (fg.1 - fs.1) * 1000.0;
    assert!(
        ridge_m >= 5.0,
        "masculine glabella {ridge_m:.1} mm forward of sellion"
    );
    assert!(
        ridge_f <= 1.5,
        "feminine glabella {ridge_f:.1} mm forward of sellion"
    );
    assert!(
        (138.49 - 1.5 * 9.29..=138.49 + 1.5 * 9.29).contains(&ma),
        "masculine nasofrontal angle {ma:.1}"
    );
    assert!(
        (141.64 - 1.5 * 8.10..=141.64 + 1.5 * 8.10).contains(&fa),
        "feminine nasofrontal angle {fa:.1}"
    );
    assert!(fa >= ma + 3.0, "nasofrontal angles {ma:.1} and {fa:.1}");
}

/// Where the references say women's and men's faces differ (population
/// data, F-01): the face narrower (zy-zy 135.1 against 143.5, 0.94) and
/// shorter (se-me 0.92), the jaw narrower (go-go 0.91), the nose smaller
/// (al-al 0.91, se-sn 0.93), the eyes held, so larger against the face.
/// The build measures both frames' faces on the skin (`face.rest.frames`);
/// the feminine against the masculine must reach at least these ratios
/// (zy-zy 0.965, se-me 0.95, go-go 0.95, al-al 0.94), and its eyes'
/// spacing against its face width must be no less than the masculine's.
/// (The references' own ratio is about 1.02: women's interpupillary
/// distance is 0.96 of men's, F-01 and F-05. The second review: the bar of
/// 1.03 rewarded a wide feminine eye spacing; the shipped 1.08 is an open
/// issue, wave 3's hand-back.)
#[test]
fn the_feminine_face_differs_where_the_references_say() {
    let data = build_data();
    let frames = &data["face"]["rest"]["frames"];
    let get = |frame: &str, key: &str| {
        frames[frame][key]
            .as_f64()
            .unwrap_or_else(|| panic!("no face.rest.frames.{frame}.{key} in the build data"))
    };
    for (key, most) in [
        ("zy_zy_mm", 0.965),
        ("se_me_mm", 0.95),
        ("go_go_mm", 0.95),
        ("al_al_mm", 0.94),
    ] {
        let ratio = get("feminine", key) / get("masculine", key);
        assert!(
            ratio <= most,
            "{key}: feminine / masculine {ratio:.3} (at most {most})"
        );
    }
    let eyes = |frame: &str| get(frame, "eye_centres_mm") / get(frame, "zy_zy_mm");
    println!(
        "eyes against the face's width: {:.3} and {:.3}",
        eyes("masculine"),
        eyes("feminine")
    );
    assert!(
        eyes("feminine") >= eyes("masculine"),
        "eyes against the face's width: {:.3} and {:.3}",
        eyes("masculine"),
        eyes("feminine")
    );
}

/// The face's midline (x = 0) as segments in profile: where each triangle
/// meets it, (height, forward) at both ends. (The mesh has a loop of
/// vertices on the midline, so most segments are its edges.)
fn midline(points: &[[f32; 3]], tris: &[[usize; 3]]) -> Vec<[(f32, f32); 2]> {
    const ON: f32 = 2e-5;
    let mut out = Vec::new();
    for t in tris {
        let mut ends: Vec<(f32, f32)> = t
            .iter()
            .map(|&i| points[i])
            .filter(|p| p[0].abs() < ON)
            .map(|p| (p[1], p[2]))
            .collect();
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (p, q) = (points[a], points[b]);
            if p[0].abs() >= ON && q[0].abs() >= ON && (p[0] < 0.0) != (q[0] < 0.0) {
                let s = p[0] / (p[0] - q[0]);
                ends.push((p[1] + s * (q[1] - p[1]), p[2] + s * (q[2] - p[2])));
            }
        }
        if ends.len() >= 2 && ends[0] != ends[1] {
            out.push([ends[0], ends[1]]);
        }
    }
    out
}

/// Where `segment` crosses `at` along its axis `k` (0 height, 1 forward):
/// the other coordinate there.
fn crossing(segment: &[(f32, f32); 2], k: usize, at: f32) -> Option<f32> {
    let get = |p: (f32, f32)| if k == 0 { (p.0, p.1) } else { (p.1, p.0) };
    let (a, b) = (get(segment[0]), get(segment[1]));
    if (a.0 - at) * (b.0 - at) > 0.0 || a.0 == b.0 {
        return None;
    }
    Some(a.1 + (at - a.0) / (b.0 - a.0) * (b.1 - a.1))
}

/// The face's outline in profile: the front-most point of the midline at
/// each quarter millimetre of height from `from` to `to` (metres), among
/// the points no more than 4 cm behind the nose's tip (so not the back of
/// the head): (height, forward).
fn outline(line: &[[(f32, f32); 2]], tip: (f32, f32), from: f32, to: f32) -> Vec<(f32, f32)> {
    let step = 0.00025;
    let bins = ((to - from) / step) as usize;
    (0..bins)
        .filter_map(|k| {
            let y = from + (k as f32 + 0.5) * step;
            line.iter()
                .filter_map(|s| crossing(s, 0, y))
                .filter(|&z| z > tip.1 - 0.04)
                .fold(None, |most: Option<f32>, z| {
                    Some(most.map_or(z, |m| m.max(z)))
                })
                .map(|z| (y, z))
        })
        .collect()
}

/// The nose's tip (pronasale): the front-most midline point of the face.
fn pronasale(line: &[[(f32, f32); 2]]) -> (f32, f32) {
    let ends = || line.iter().flat_map(|s| s.iter().cloned());
    let top = ends().map(|p| p.0).fold(f32::MIN, f32::max);
    ends()
        .filter(|p| p.0 > top - 0.24 && p.0 < top - 0.04)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap()
}

/// The nasolabial angle in degrees, in profile on the midline (the
/// references' rule, `references/face/nose/README.md`: columella point,
/// subnasale, labrale superius). Subnasale is the deepest point of the
/// outline between the nose's tip and the lip; the columella's line is the
/// tangent from subnasale to the nose's lower edge (the chord to it that
/// rises least, over the edge from 3 mm forward of subnasale to 1 mm short
/// of the tip); the lip's line runs to labrale superius, the front-most
/// point of the outline 5 to 20 mm under subnasale.
fn nasolabial(points: &[[f32; 3]], tris: &[[usize; 3]], at: Option<f32>) -> (f32, f32) {
    let line = midline(points, tris);
    let tip = pronasale(&line);
    let face = outline(&line, tip, tip.0 - 0.045, tip.0 + 0.002);
    let deepest = || {
        *face
            .iter()
            .filter(|p| p.0 > tip.0 - 0.025 && p.0 < tip.0 - 0.002)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
    };
    let sn = match at {
        Some(y) => *face
            .iter()
            .min_by(|a, b| (a.0 - y).abs().total_cmp(&(b.0 - y).abs()))
            .unwrap(),
        None => deepest(),
    };
    // The nose's lower edge: the lowest midline point over subnasale and
    // under the tip, every quarter millimetre forward.
    let steps = ((tip.1 - sn.1 - 0.004) / 0.00025).max(1.0) as usize;
    let column = (0..steps)
        .filter_map(|k| {
            let z = sn.1 + 0.003 + k as f32 * 0.00025;
            line.iter()
                .filter_map(|s| crossing(s, 1, z))
                .filter(|&y| y > sn.0 - 0.0005 && y < tip.0 + 0.001)
                .fold(None, |low: Option<f32>, y| {
                    Some(low.map_or(y, |m| m.min(y)))
                })
                .map(|y| (y, z))
        })
        .min_by(|a, b| ((a.0 - sn.0) / (a.1 - sn.1)).total_cmp(&((b.0 - sn.0) / (b.1 - sn.1))))
        .expect("the nose's lower edge");
    let lip = *face
        .iter()
        .filter(|p| p.0 < sn.0 - 0.005 && p.0 > sn.0 - 0.020)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    let (c, l) = (
        (column.0 - sn.0, column.1 - sn.1),
        (lip.0 - sn.0, lip.1 - sn.1),
    );
    let cos = (c.0 * l.0 + c.1 * l.1) / (c.0.hypot(c.1) * l.0.hypot(l.1));
    (cos.clamp(-1.0, 1.0).acos().to_degrees(), sn.0)
}

/// Brief item 4 and the second review: `nose_tip_rotation` reaches the
/// references' range at both ends. The nose README sets the nasolabial
/// angle's range at +-20 degrees (105 +- 9.6, about 2 SD); the brief's
/// 2.5 SD is 24. On both frames the angle (columella point, subnasale,
/// labrale superius) opens by 20 to 24 degrees at +1 and closes by 15 to
/// 24 at -1. The ends are separate shapes (`faceparams.SPLIT`): the
/// drooped end is held where the thin line under the columella stayed
/// within a level of the neutral face's (`face.NOSE_TURNS_DEG`; wave 3's
/// hand-back, review fixes 1), and its upper bound guards that line.
#[test]
fn the_nose_tip_turns_by_the_references_range() {
    let library = library();
    let base = library.base(BASE).unwrap();
    let model = object(&library, &base.model.body);
    let mut short = Vec::new();
    for frame in [0.0, 1.0] {
        let shape =
            |value: f32| posed_with(&library, &model, frame, &[("nose_tip_rotation", value)]);
        let (points, tris) = shape(0.0);
        let (rest, sn) = nasolabial(&points, &tris, None);
        // (The turn holds subnasale: the ends are measured there, not at
        // their own deepest point, which climbs the columella as the angle
        // opens.)
        let angle = |value: f32| {
            let (points, tris) = shape(value);
            nasolabial(&points, &tris, Some(sn)).0
        };
        let (down, up) = (angle(-1.0), angle(1.0));
        println!(
            "frame {frame}: nasolabial angle {down:.1} / {rest:.1} / {up:.1} ({:+.1} / {:+.1})",
            down - rest,
            up - rest
        );
        if !(20.0..=24.0).contains(&(up - rest)) {
            short.push(format!("frame {frame}: opens by {:.1} at +1", up - rest));
        }
        if !(15.0..=24.0).contains(&(rest - down)) {
            short.push(format!("frame {frame}: closes by {:.1} at -1", rest - down));
        }
    }
    assert!(short.is_empty(), "nasolabial angle: {short:?}");
}

/// How far the forehead's outline dips behind a straight line from
/// glabella (the most forward point within 3 cm over sellion) to the
/// outline 4 cm over it, in millimetres: 0 for a forehead that rounds back
/// evenly, more for a shelf with a hollow over it.
fn forehead_hollow(points: &[[f32; 3]], tris: &[[usize; 3]]) -> f32 {
    let line = midline(points, tris);
    let tip = pronasale(&line);
    let face = outline(&line, tip, tip.0 - 0.01, tip.0 + 0.11);
    let (_, glabella, _) = nasion_and_glabella(&face);
    let at = |y: f32| {
        *face
            .iter()
            .min_by(|a, b| (a.0 - y).abs().total_cmp(&(b.0 - y).abs()))
            .unwrap()
    };
    let top = at(glabella.0 + 0.04);
    face.iter()
        .filter(|p| p.0 > glabella.0 && p.0 < top.0)
        .map(|p| {
            let s = (p.0 - glabella.0) / (top.0 - glabella.0);
            let chord = glabella.1 + s * (top.1 - glabella.1);
            (chord - p.1) * 1000.0
        })
        .fold(0.0, f32::max)
}

/// The second review: the square preset's strong brow ridge on a forehead
/// tipped back read as a hard band across the forehead, a shelf with a
/// hollow over it in profile. No face preset hollows the forehead more
/// than 0.5 mm deeper than the frame's default face does.
#[test]
fn no_face_preset_puts_a_shelf_on_the_forehead() {
    let library = library();
    let base = library.base(BASE).unwrap();
    let model = object(&library, &base.model.body);
    let mut deep = Vec::new();
    for frame in [0.0, 1.0] {
        let (points, tris) = posed(&library, &model, frame);
        let plain = forehead_hollow(&points, &tris);
        println!("frame {frame} default: hollow {plain:.2} mm");
        for name in [
            "round", "oval", "square", "heart", "long", "broad", "narrow", "soft", "angular",
        ] {
            let id = format!("axcenity/realistic-face-{name}");
            let preset = library.preset(&id).unwrap_or_else(|| panic!("no `{id}`"));
            let mut description = Description::new("Frame", BASE);
            description.parameters.insert("frame".into(), frame);
            axcenity_avatar::apply_preset(&mut description, preset, &library).unwrap();
            let (points, tris) = posed_as(&library, &model, &description);
            let hollow = forehead_hollow(&points, &tris);
            println!("frame {frame} {name}: hollow {hollow:.2} mm");
            if hollow > plain + 0.5 {
                deep.push(format!(
                    "frame {frame} {name} {hollow:.2} mm (default {plain:.2})"
                ));
            }
        }
    }
    assert!(deep.is_empty(), "forehead hollows: {deep:?}");
}
