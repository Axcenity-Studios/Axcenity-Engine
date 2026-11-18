//! Character Studio refinement R3 (RA3, wave 3): `muscle` and `fat` both
//! at 1 is a strong, heavy person, not a heavyweight bodybuilder. MakeHuman's
//! most muscular and heaviest corner carries a bodybuilder's definition
//! (separate deltoids, biceps and trapezius) and, on the feminine frame,
//! narrow hips over thighs that flare out below them;
//! `<Frame>_Fat_Over_Muscle` covers the muscle with the fat (shapes.py,
//! `fat_over_muscle`).
//!
//! Measured on the baked rest body (`axcenity_assets::Model::bake`), in
//! metres and glTF axes: Y up, X side to side. Bands are shares of the
//! body's height from its feet.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use axcenity_assets::{Model, ModelFormat, Pick, load_models};
use axcenity_avatar::{Description, Library, resolve};

const BASE: &str = "axcenity/realistic";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn library() -> Library {
    let (library, _, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{problems:?}");
    library
}

fn body(library: &Library) -> Model {
    let base = library.base(BASE).unwrap();
    let bytes = std::fs::read(root().join("realistic").join(&base.model.file)).unwrap();
    let picks = [Pick::Objects(vec![base.model.body.clone()])];
    load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
        .unwrap()
        .remove(0)
}

/// The skin's rest positions for these parameter values, in metres.
fn baked(library: &Library, model: &Model, values: &[(&str, f32)]) -> Vec<[f32; 3]> {
    baked_with_hips(library, model, values).0
}

/// `baked`, and the height of the hip joints (the `LeftUpperLeg` bone's
/// head) in the same space.
fn baked_with_hips(
    library: &Library,
    model: &Model,
    values: &[(&str, f32)],
) -> (Vec<[f32; 3]>, f32) {
    let mut description = Description::new("Build", BASE);
    for (id, value) in values {
        description.parameters.insert((*id).to_owned(), *value);
    }
    let resolved = resolve(library, &description).unwrap();
    assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
    // The skin itself: the underwear (always worn) sinks the skin it
    // covers by its mask shape, which would read as a narrower hip.
    let mut identity = resolved.identity;
    for layer in &mut identity.layers {
        for (shape, weight) in &mut layer.shapes {
            if shape.starts_with("Mask_") {
                *weight = 0.0;
            }
        }
    }
    let size = model.size;
    let model = model.bake(&identity);
    let rig = model.rig.as_ref().unwrap();
    let posed = model.pose_shaped(&rig.rest_pose(), &[]).unwrap();
    let hip = rig.bone("LeftUpperLeg").expect("a LeftUpperLeg bone");
    // (The rig is in the file's space; the positions are fitted round the
    // model's centre.)
    let hip = rig.world(&rig.rest_pose())[hip].w_axis.y - model.center[1];
    let positions = posed
        .primitive(&model, 0, 0)
        .positions
        .iter()
        .map(|p| [p[0] * size[0], p[1] * size[1], p[2] * size[2]])
        .collect();
    (positions, hip)
}

/// The skin as one surface: each vertex's welded index (the file splits
/// vertices along texture seams) and each welded vertex's neighbours.
struct Surface {
    weld: Vec<usize>,
    neighbours: Vec<Vec<usize>>,
}

impl Surface {
    fn of(model: &Model, rest: &[[f32; 3]]) -> Self {
        let mut ids: HashMap<[u32; 3], usize> = HashMap::new();
        let weld: Vec<usize> = rest
            .iter()
            .map(|p| {
                let key = p.map(f32::to_bits);
                let next = ids.len();
                *ids.entry(key).or_insert(next)
            })
            .collect();
        let mut neighbours = vec![Vec::new(); ids.len()];
        for tri in model.primitives[0].indices.chunks(3) {
            for k in 0..3 {
                let (a, b) = (weld[tri[k] as usize], weld[tri[(k + 1) % 3] as usize]);
                if a != b {
                    if !neighbours[a].contains(&b) {
                        neighbours[a].push(b);
                    }
                    if !neighbours[b].contains(&a) {
                        neighbours[b].push(a);
                    }
                }
            }
        }
        Self { weld, neighbours }
    }

    /// How defined a change of shape is: the root mean square, in metres,
    /// of what's left of each vertex's move once the move is smoothed over
    /// about 10 cm (30 rounds of halfway to its neighbours' mean), over the
    /// vertices `keep` picks.
    fn definition(
        &self,
        from: &[[f32; 3]],
        to: &[[f32; 3]],
        keep: impl Fn(&[f32; 3]) -> bool,
    ) -> f32 {
        let n = self.neighbours.len();
        let mut d = vec![[0.0_f32; 3]; n];
        let mut kept = vec![false; n];
        for (i, (a, b)) in from.iter().zip(to).enumerate() {
            let w = self.weld[i];
            d[w] = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            kept[w] = keep(a);
        }
        let mut s = d.clone();
        for _ in 0..30 {
            let mean: Vec<[f32; 3]> = (0..n)
                .map(|i| {
                    let around = &self.neighbours[i];
                    if around.is_empty() {
                        return s[i];
                    }
                    let mut m = [0.0; 3];
                    for &j in around {
                        for k in 0..3 {
                            m[k] += s[j][k];
                        }
                    }
                    m.map(|v| v / around.len() as f32)
                })
                .collect();
            for i in 0..n {
                for k in 0..3 {
                    s[i][k] = (s[i][k] + mean[i][k]) / 2.0;
                }
            }
        }
        let (mut sum, mut count) = (0.0_f32, 0);
        for i in (0..n).filter(|&i| kept[i]) {
            sum += (0..3).map(|k| (d[i][k] - s[i][k]).powi(2)).sum::<f32>();
            count += 1;
        }
        (sum / count as f32).sqrt()
    }
}

/// A body's feet and height.
fn feet_and_height(body: &[[f32; 3]]) -> (f32, f32) {
    let feet = body.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
    let top = body.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    (feet, top - feet)
}

#[test]
fn heavy_and_muscular_is_not_a_bodybuilder() {
    let library = library();
    let model = body(&library);
    for (frame, name) in [(0.0, "masculine"), (1.0, "feminine")] {
        let default = baked(&library, &model, &[("frame", frame)]);
        let heavy = baked(&library, &model, &[("frame", frame), ("fat", 1.0)]);
        let both = baked(
            &library,
            &model,
            &[("frame", frame), ("fat", 1.0), ("muscle", 1.0)],
        );
        let surface = Surface::of(&model, &default);
        let (feet, height) = feet_and_height(&default);
        // The upper body: the trunk from the waist up, the shoulders and the
        // arms, under the head.
        let upper = |p: &[f32; 3]| (0.55..0.85).contains(&((p[1] - feet) / height));
        let heavy_definition = surface.definition(&default, &heavy, upper);
        let both_definition = surface.definition(&default, &both, upper);
        eprintln!(
            "{name}: definition {:.2} mm heavy and muscular, {:.2} mm heavy",
            both_definition * 1000.0,
            heavy_definition * 1000.0
        );
        assert!(
            both_definition <= 1.5 * heavy_definition,
            "{name}: heavy and muscular is {:.1} mm defined, heavy {:.1} mm",
            both_definition * 1000.0,
            heavy_definition * 1000.0
        );
        // Still strong: the muscle adds mass to the heavy body's upper half.
        let mass = |body: &[[f32; 3]]| -> f32 {
            let moved: Vec<f32> = default
                .iter()
                .zip(body)
                .filter(|(p, _)| upper(p))
                .map(|(p, q)| (0..3).map(|k| (q[k] - p[k]).powi(2)).sum::<f32>().sqrt())
                .collect();
            moved.iter().sum::<f32>() / moved.len() as f32
        };
        assert!(
            mass(&both) > mass(&heavy),
            "{name}: heavy and muscular moves the upper body {:.1} mm, heavy {:.1} mm",
            mass(&both) * 1000.0,
            mass(&heavy) * 1000.0
        );
    }
}

/// The outer silhouette from the front: per level from `low` to `high`
/// (metres, 5 mm apart), the farthest point side to side where a level
/// plane cuts the skin's triangles, the arms left out (in the A-pose they
/// hang more than 0.3 m out at the thighs' and hips' height).
fn silhouette(model: &Model, body: &[[f32; 3]], low: f32, high: f32) -> Vec<f32> {
    let indices = &model.primitives[0].indices;
    let mut out = Vec::new();
    let mut y = low;
    while y <= high {
        let mut widest = 0.0_f32;
        for tri in indices.chunks(3) {
            for k in 0..3 {
                let (a, b) = (body[tri[k] as usize], body[tri[(k + 1) % 3] as usize]);
                if (a[1] - y) * (b[1] - y) > 0.0 || a[1] == b[1] {
                    continue;
                }
                let t = (y - a[1]) / (b[1] - a[1]);
                let x = (a[0] + (b[0] - a[0]) * t).abs();
                if x < 0.3 {
                    widest = widest.max(x);
                }
            }
        }
        out.push(widest);
        y += 0.005;
    }
    out
}

#[test]
fn heavy_and_muscular_thighs_do_not_flare_below_the_hips() {
    let library = library();
    let model = body(&library);
    for (frame, name) in [(0.0, "masculine"), (1.0, "feminine")] {
        let (both, hip) = baked_with_hips(
            &library,
            &model,
            &[("frame", frame), ("fat", 1.0), ("muscle", 1.0)],
        );
        let widest = |low: f32, high: f32| -> f32 {
            silhouette(&model, &both, hip + low, hip + high)
                .into_iter()
                .fold(0.0, f32::max)
        };
        // Mid-thigh (14 to 30 cm under the hip joints) against the hips (12 cm
        // under them to 4 cm over): a thigh wider than the hips is the
        // jodhpur outline.
        let flare = widest(-0.30, -0.14) - widest(-0.12, 0.04);
        eprintln!(
            "{name}: the thigh's flare past the hips {:.1} mm",
            flare * 1000.0
        );
        assert!(
            flare <= 0.003,
            "{name}: the thigh stands {:.1} mm out past the hips",
            flare * 1000.0
        );
    }
}
