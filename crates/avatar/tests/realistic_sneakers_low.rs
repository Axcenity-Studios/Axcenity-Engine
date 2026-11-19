//! The first sneakers made from scratch (wave 5 stage 3, Paul's decisions
//! 13 and 14): `axcenity/realistic-sneakers-low`, a plain low-top modelled
//! in Blender over a last made from the realistic foot
//! (`assets/characters/realistic/clothing/build_sneakers_low.py`). These
//! tests hold the shipped files to the parts of
//! `docs/development/character-studio/asset-quality.md` §2 and §3 that can
//! be read from them, with **no records**: the manifest, the budget and
//! the levels, the texture coordinates, a sound mesh with no single-layer
//! edge, and skinning that keeps a shoe a shoe (only the foot's bones; the
//! sole and the toe box rigid on the foot and the toes). The fit on the
//! cast, in motion and on the feet is the player's
//! (`sneakers_low_tests.rs`).

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axcenity_assets::{Assets, Model, ModelShading, Primitive};
use axcenity_avatar::{
    ContentTier, Fit, ItemKind, ItemManifest, Library, MaterialRole, Origin, Quality,
};

const REALISTIC: &str = "axcenity/realistic";
const ID: &str = "axcenity/realistic-sneakers-low";
const OBJECT: &str = "Shoes_LowSneakers";
/// §3 "Budgets": shoes 4k triangles a pair at LOD0.
const BUDGET: usize = 4000;
/// §2 "UVs": stretch under 10% on 95% of the area; texel density at least
/// 8 px/cm at 2K on all of it.
const STRETCH: f32 = 0.10;
const STRETCH_SHARE: f32 = 0.95;
const DENSITY: f32 = 800.0;
/// §3 "Construction": a raw edge lies inside the shell it is turned from,
/// 2 to 25 mm, its fold at least three rings of corners away.
const HEM_THICK: f32 = 0.0020;
const HEM_REACH: f32 = 0.025;
const HEM_RINGS: usize = 3;
/// The bones a shoe may follow: the foot, the toes, and the shin's twist
/// for the collar round the ankle (each side its own).
const FOOT_BONES: [&str; 4] = ["Foot", "Toes", "LowerLeg", "LowerLegTwist"];
/// The toe box: the front of each shoe from this share of its length (as
/// the player's footwear check, `garment_checks::TOE_BOX_FROM`).
const TOE_BOX_FROM: f32 = 0.75;
/// The materials of the sole, whose front half must not bend with the
/// ankle: from this far forward (the file's z, metres; the ankle is at
/// about 0).
const SOLE: [&str; 2] = ["Midsole", "Outsole"];
const SOLE_FRONT: f32 = 0.02;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn library() -> Library {
    let (library, _, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{problems:?}");
    library
}

fn item(library: &Library) -> &ItemManifest {
    library
        .item(ID)
        .unwrap_or_else(|| panic!("the library has no `{ID}`"))
}

/// The item's model, as the engine reads it.
fn model(library: &Library) -> Arc<Model> {
    let manifest = item(library);
    let model = manifest.model.as_ref().expect("a model");
    let file = library.path_of(ID, &model.file).expect("its file");
    let id = format!("{file}#{}", model.object.as_deref().expect("an object"));
    Assets::new(root())
        .model_blocking(&id)
        .unwrap_or_else(|| panic!("`{id}` doesn't load"))
}

/// The JSON chunk of the item's `.glb`.
fn gltf_json(library: &Library) -> serde_json::Value {
    let manifest = item(library);
    let file = library
        .path_of(ID, &manifest.model.as_ref().unwrap().file)
        .unwrap();
    let bytes = std::fs::read(root().join(file)).unwrap();
    assert_eq!(&bytes[0..4], b"glTF");
    let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    serde_json::from_slice(&bytes[20..20 + length]).unwrap()
}

/// A primitive's corners in the file's own space (metres; the engine reads
/// a rigged model into a fitted space, which this undoes).
fn positions(model: &Model, primitive: &Primitive) -> Vec<[f32; 3]> {
    let unfit = model
        .rig
        .as_ref()
        .map_or(glam::Mat4::IDENTITY, |rig| rig.fit.inverse());
    primitive
        .positions
        .iter()
        .map(|p| unfit.transform_point3(glam::Vec3::from(*p)).to_array())
        .collect()
}

fn triangles(primitive: &Primitive) -> usize {
    primitive.indices.len() / 3
}

#[test]
fn the_sneakers_are_shipped_as_a_fitted_built_in_draft_of_original_work() {
    let library = library();
    let manifest = item(&library);
    assert_eq!(manifest.kind, ItemKind::Garment);
    assert_eq!(manifest.slot, "shoes");
    assert_eq!(manifest.bases, vec![REALISTIC.to_owned()]);
    assert_eq!(manifest.tier, ContentTier::BuiltIn);
    assert_eq!(manifest.layer, Some(1));
    // A draft until the review and Paul's sign-off (decision 13).
    assert_eq!(manifest.quality, Some(Quality::Draft));
    assert!(manifest.about.is_some());
    let model = manifest.model.as_ref().expect("a model");
    assert_eq!(
        model.object.as_deref(),
        Some(format!("AxcenityRealistic/{OBJECT}").as_str())
    );
    match &manifest.fit {
        Fit::Binding {
            data: Some(data), ..
        } => {
            let path = library.path_of(ID, data).unwrap();
            assert!(root().join(path).is_file(), "no binding file");
        }
        other => panic!("fitted by {other:?}"),
    }
    let thumbnail = manifest.thumbnail.as_deref().expect("a thumbnail");
    assert!(
        root()
            .join(library.path_of(ID, thumbnail).unwrap())
            .is_file()
    );
    // Modelled from scratch (decision 14): original work released CC0,
    // its maps from CC0 material photographs; no imported base, so no
    // provenance record. The reference photographs are looked at, never
    // shipped: they are listed in references/clothing/README.md.
    let source = manifest.source.as_ref().expect("a source");
    assert_eq!(source.licence, "CC0-1.0");
    assert_ne!(source.origin, Some(Origin::Imported));
    assert!(source.provenance.is_none());
    assert!(source.url.is_some());
    assert!(source.author.contains("CC0"), "{source:?}");
    assert!(manifest.validate().is_empty(), "{:?}", manifest.validate());
    let base = library.base(REALISTIC).unwrap();
    assert!(
        manifest.check_against(base).is_empty(),
        "{:?}",
        manifest.check_against(base)
    );
}

#[test]
fn the_sneakers_materials_are_on_their_shading_models() {
    let library = library();
    let manifest = item(&library);
    let model = model(&library);
    for primitive in &model.primitives {
        assert!(
            manifest.material(&primitive.material.name).is_some(),
            "`{}` isn't a slot",
            primitive.material.name
        );
    }
    let roles: Vec<_> = manifest.materials.iter().filter_map(|m| m.role).collect();
    assert!(roles.contains(&MaterialRole::Primary), "{roles:?}");
    assert!(roles.contains(&MaterialRole::Secondary), "{roles:?}");
    assert!(manifest.variants.len() >= 3);
    for variant in &manifest.variants {
        for name in variant.materials.keys() {
            assert!(manifest.material(name).is_some(), "{name}");
        }
    }
    let shading = |name: &str| {
        model
            .primitives
            .iter()
            .find(|p| p.material.name == name)
            .map(|p| p.material.shading)
            .unwrap_or_else(|| panic!("no `{name}` material"))
    };
    // Leather and rubber are Standard; the textiles are Cloth with a sheen.
    assert_eq!(shading("Upper"), ModelShading::Standard);
    assert_eq!(shading("Outsole"), ModelShading::Standard);
    assert_eq!(shading("Midsole"), ModelShading::Standard);
    for cloth in ["Lining", "Laces"] {
        assert_eq!(shading(cloth), ModelShading::Cloth, "{cloth}");
    }
    for primitive in &model.primitives {
        let material = &primitive.material;
        if material.shading == ModelShading::Cloth {
            assert!(material.sheen.is_some(), "{} has no sheen", material.name);
        }
        let colour = material
            .base_color_texture
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no colour map", material.name));
        assert!(colour.width >= 2048, "{}: {}", material.name, colour.width);
        assert!(
            material.normal_texture.is_some(),
            "{}: no normal map",
            material.name
        );
    }
}

#[test]
fn the_sneakers_keep_to_the_budget_with_three_levels_of_their_own() {
    let library = library();
    let model = model(&library);
    let lod0: usize = model.primitives.iter().map(triangles).sum();
    println!("{lod0} triangles at LOD0");
    assert!(lod0 <= BUDGET, "{lod0} > {BUDGET}");
    let mut before = lod0;
    for level in 1..=2 {
        let count: usize = model
            .primitives
            .iter()
            .map(|p| p.lods.get(level - 1).map_or(usize::MAX / 8, triangles))
            .sum();
        println!("{count} triangles at LOD{level}");
        assert!(count * 10 <= before * 6, "LOD{level} {count} of {before}");
        before = count;
    }
    let json = gltf_json(&library);
    let names: BTreeSet<&str> = json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["name"].as_str())
        .collect();
    for level in 1..=2 {
        let name = format!("{OBJECT}_LOD{level}");
        assert!(names.contains(name.as_str()), "no {name} in {names:?}");
    }
}

/// A triangle's area in metres, and the two singular values of the map
/// from its surface to the texture (texels per metre).
fn texel_map(p: [[f32; 3]; 3], uv: [[f32; 2]; 3], size: [f32; 2]) -> (f32, f32, f32) {
    use glam::{Vec2, Vec3};
    let (a, b, c) = (Vec3::from(p[0]), Vec3::from(p[1]), Vec3::from(p[2]));
    let e1 = b - a;
    let e2 = c - a;
    let area = e1.cross(e2).length() / 2.0;
    if area < 1e-12 {
        return (0.0, 0.0, 0.0);
    }
    let x = e1.normalize();
    let y = e1.cross(e2).cross(e1).normalize();
    let (p1, p2) = (Vec2::new(e1.dot(x), 0.0), Vec2::new(e2.dot(x), e2.dot(y)));
    let s = Vec2::from(size);
    let t0 = Vec2::from(uv[0]) * s;
    let (t1, t2) = (Vec2::from(uv[1]) * s - t0, Vec2::from(uv[2]) * s - t0);
    let det = p1.x * p2.y - p2.x * p1.y;
    let inv = [[p2.y / det, -p2.x / det], [-p1.y / det, p1.x / det]];
    let j = [
        [
            t1.x * inv[0][0] + t2.x * inv[1][0],
            t1.x * inv[0][1] + t2.x * inv[1][1],
        ],
        [
            t1.y * inv[0][0] + t2.y * inv[1][0],
            t1.y * inv[0][1] + t2.y * inv[1][1],
        ],
    ];
    let (a, b, c, d) = (j[0][0], j[0][1], j[1][0], j[1][1]);
    let s1 = a * a + b * b + c * c + d * d;
    let s2 = ((a * a + b * b - c * c - d * d).powi(2) + 4.0 * (a * c + b * d).powi(2)).sqrt();
    let high = ((s1 + s2) / 2.0).max(0.0).sqrt();
    let low = ((s1 - s2) / 2.0).max(0.0).sqrt();
    (area, high, low)
}

/// A draft's recorded shortfall of §2's texture limits on one material at
/// one level (open defects, not accepted behaviour: `sneakers-scratch.md`
/// says where each is): the least share even and dense it reaches, and the
/// most share of its texels drawn twice. A share better than its record by
/// more than `SLACK` fails too (lower the record); a production item has
/// none.
struct UvRecord {
    material: &'static str,
    level: usize,
    even: Option<f32>,
    dense: Option<f32>,
    twice: Option<f32>,
}

const SLACK: f32 = 0.01;

const fn uv(
    material: &'static str,
    level: usize,
    even: Option<f32>,
    dense: Option<f32>,
    twice: Option<f32>,
) -> UvRecord {
    UvRecord {
        material,
        level,
        even,
        dense,
        twice,
    }
}

// The full level: a few sliver triangles under 8 px/cm where the vamp's
// columns converge on the throat and at the bow's knot. The simpler
// levels: each unwrapped on its own and laid onto the full level's
// islands (so they read the same texels), which leaves some islands
// overlapping their neighbours, and stretch on the coarsest upper.
const UV_RECORDS: [UvRecord; 14] = [
    uv("Upper", 0, None, Some(0.9993), None),
    uv("Upper", 1, None, Some(0.9984), Some(0.0255)),
    uv("Upper", 2, Some(0.905), Some(0.9975), Some(0.1092)),
    uv("Lining", 0, None, Some(0.9995), None),
    uv("Lining", 1, None, Some(0.9952), Some(0.0144)),
    uv("Lining", 2, None, None, Some(0.0134)),
    uv("Tongue", 2, None, None, Some(0.0974)),
    uv("Laces", 0, None, Some(0.9989), None),
    uv("Laces", 1, None, None, Some(0.2605)),
    uv("Laces", 2, None, None, Some(0.3038)),
    uv("Eyelets", 1, None, None, Some(0.0325)),
    uv("Eyelets", 2, None, None, Some(0.0671)),
    uv("Midsole", 1, None, None, Some(0.0098)),
    uv("Midsole", 2, None, None, Some(0.1954)),
];

/// A share against its limit (`at_least`: a floor, else a ceiling), or the
/// draft's record of it.
fn judge(
    label: &str,
    share: f32,
    limit: f32,
    record: Option<f32>,
    at_least: bool,
) -> Option<String> {
    let worse = |a: f32, b: f32| if at_least { a < b } else { a > b };
    match record {
        None if worse(share, limit) => Some(format!(
            "{label} {:.2}% (limit {:.1}%)",
            share * 100.0,
            limit * 100.0
        )),
        Some(r) if worse(share, r) => Some(format!(
            "{label} {:.2}% past its record {:.2}%",
            share * 100.0,
            r * 100.0
        )),
        Some(_) if !worse(share, limit) => Some(format!(
            "{label} {:.2}% passes: remove the record",
            share * 100.0
        )),
        Some(r) if (share - r).abs() > SLACK => Some(format!(
            "{label} {:.2}%: move the record from {:.2}%",
            share * 100.0,
            r * 100.0
        )),
        _ => None,
    }
}

/// §2 "UVs" on every level: even (stretch under 10% on 95% of a
/// material's area), dense (8 px/cm at 2K on all of it) and not
/// overlapping (under 0.5% of the texels used drawn twice at 512 px).
#[test]
fn the_sneakers_texture_coordinates_are_even_dense_and_do_not_overlap() {
    let library = library();
    let model = model(&library);
    let mut failures: Vec<String> = Vec::new();
    for top in &model.primitives {
        let levels = std::iter::once(top).chain(top.lods.iter());
        for (level, primitive) in levels.enumerate() {
            let name = format!("{} LOD{level}", top.material.name);
            let colour = top
                .material
                .base_color_texture
                .as_ref()
                .expect("a colour map");
            assert_eq!(primitive.uvs.len(), primitive.positions.len(), "{name}");
            let size = [colour.width as f32, colour.height as f32];
            let world = positions(&model, primitive);
            let mut rows = Vec::new();
            for t in primitive.indices.chunks_exact(3) {
                let p = [0, 1, 2].map(|k| world[t[k] as usize]);
                let uv = [0, 1, 2].map(|k| primitive.uvs[t[k] as usize]);
                let (area, high, low) = texel_map(p, uv, size);
                if area > 0.0 {
                    rows.push((area, high, low));
                }
            }
            let total: f32 = rows.iter().map(|r| r.0).sum();
            let mut scales: Vec<f32> = rows.iter().map(|r| (r.1 * r.2).sqrt()).collect();
            scales.sort_by(f32::total_cmp);
            let median = scales[scales.len() / 2];
            let even: f32 = rows
                .iter()
                .filter(|r| r.1 / median - 1.0 < STRETCH && 1.0 - r.2 / median < STRETCH)
                .map(|r| r.0)
                .sum();
            let dense: f32 = rows.iter().filter(|r| r.2 >= DENSITY).map(|r| r.0).sum();
            println!(
                "{name}: median {median:.0} px/m, even {:.1}%, dense {:.1}%",
                100.0 * even / total,
                100.0 * dense / total
            );
            let record = UV_RECORDS
                .iter()
                .find(|r| r.material == top.material.name && r.level == level);
            if record.is_some() {
                assert!(item(&library).is_draft(), "a production item has no record");
            }
            failures.extend(judge(
                &format!("{name}: even"),
                even / total,
                STRETCH_SHARE,
                record.and_then(|r| r.even),
                true,
            ));
            failures.extend(judge(
                &format!("{name}: dense"),
                dense / total,
                1.0,
                record.and_then(|r| r.dense),
                true,
            ));
            const N: usize = 512;
            let mut hits = vec![0u8; N * N];
            for t in primitive.indices.chunks_exact(3) {
                let px = [0, 1, 2].map(|k| {
                    let q = primitive.uvs[t[k] as usize];
                    [q[0] * N as f32, q[1] * N as f32]
                });
                let x0 = px
                    .iter()
                    .map(|q| q[0])
                    .fold(f32::MAX, f32::min)
                    .floor()
                    .max(0.0) as usize;
                let x1 = px
                    .iter()
                    .map(|q| q[0])
                    .fold(f32::MIN, f32::max)
                    .ceil()
                    .min(N as f32) as usize;
                let y0 = px
                    .iter()
                    .map(|q| q[1])
                    .fold(f32::MAX, f32::min)
                    .floor()
                    .max(0.0) as usize;
                let y1 = px
                    .iter()
                    .map(|q| q[1])
                    .fold(f32::MIN, f32::max)
                    .ceil()
                    .min(N as f32) as usize;
                let edge = |a: [f32; 2], b: [f32; 2], x: f32, y: f32| {
                    (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
                };
                let whole = edge(px[0], px[1], px[2][0], px[2][1]);
                if whole.abs() < 1e-9 {
                    continue;
                }
                for y in y0..y1 {
                    for x in x0..x1 {
                        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                        let w = [
                            edge(px[1], px[2], fx, fy) / whole,
                            edge(px[2], px[0], fx, fy) / whole,
                            edge(px[0], px[1], fx, fy) / whole,
                        ];
                        if w.iter().all(|w| *w > 0.0) {
                            hits[y * N + x] = hits[y * N + x].saturating_add(1);
                        }
                    }
                }
            }
            let used = hits.iter().filter(|h| **h > 0).count();
            let twice = hits.iter().filter(|h| **h > 1).count();
            println!("{name}: {twice} of {used} texels drawn twice");
            failures.extend(judge(
                &format!("{name}: drawn twice"),
                twice as f32 / used as f32,
                0.005,
                record.and_then(|r| r.twice),
                false,
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Corners at one place are one: the index of the first corner at each
/// corner's place.
fn welded(positions: &[[f32; 3]]) -> Vec<usize> {
    let mut first: HashMap<[i64; 3], usize> = HashMap::new();
    positions
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let key = p.map(|c| (c * 1e5).round() as i64);
            *first.entry(key).or_insert(k)
        })
        .collect()
}

/// §2 "Geometry" on every level: no degenerate triangle, no edge in more
/// than two triangles, consistent normals (two triangles on an edge run
/// it opposite ways), weights normalized.
#[test]
fn the_sneakers_meshes_are_sound() {
    let library = library();
    let model = model(&library);
    for top in &model.primitives {
        for (level, primitive) in std::iter::once(top).chain(top.lods.iter()).enumerate() {
            let name = format!("{} LOD{level}", top.material.name);
            let world = positions(&model, primitive);
            for t in primitive.indices.chunks_exact(3) {
                let [a, b, c] = [0, 1, 2].map(|k| glam::Vec3::from(world[t[k] as usize]));
                let area = (b - a).cross(c - a).length() / 2.0;
                assert!(area > 1e-9, "{name}: a degenerate triangle {t:?}");
            }
            assert_eq!(primitive.weights.len(), primitive.positions.len(), "{name}");
            for w in &primitive.weights {
                let sum: f32 = w.iter().sum();
                assert!((sum - 1.0).abs() < 2e-3, "{name}: weights add to {sum}");
            }
            let weld = welded(&world);
            let mut edges: HashMap<(usize, usize), Vec<bool>> = HashMap::new();
            for t in primitive.indices.chunks_exact(3) {
                for k in 0..3 {
                    let (a, b) = (weld[t[k] as usize], weld[t[(k + 1) % 3] as usize]);
                    edges.entry((a.min(b), a.max(b))).or_default().push(a < b);
                }
            }
            for (edge, ways) in &edges {
                assert!(
                    ways.len() <= 2,
                    "{name}: edge {edge:?} in {} triangles",
                    ways.len()
                );
                if ways.len() == 2 {
                    assert_ne!(ways[0], ways[1], "{name}: a flipped face at {edge:?}");
                }
            }
        }
    }
}

/// §3 "Construction": every open edge is the raw edge of a turned edge or
/// a lining, lying 2 to 25 mm inside the shell, on every level.
#[test]
fn no_edge_of_the_sneakers_is_a_single_layer() {
    let library = library();
    let model = model(&library);
    let levels = 1 + model.primitives[0].lods.len();
    for level in 0..levels {
        let mut all = Vec::new();
        let mut tris = Vec::new();
        for top in &model.primitives {
            let primitive = if level == 0 {
                top
            } else {
                &top.lods[level - 1]
            };
            let start = all.len() as u32;
            all.extend(positions(&model, primitive));
            tris.extend(
                primitive
                    .indices
                    .chunks_exact(3)
                    .map(|t| [t[0] + start, t[1] + start, t[2] + start]),
            );
        }
        let weld = welded(&all);
        let mut edges: HashMap<(usize, usize), usize> = HashMap::new();
        let mut neighbours: HashMap<usize, BTreeSet<usize>> = HashMap::new();
        for t in &tris {
            for k in 0..3 {
                let (a, b) = (weld[t[k] as usize], weld[t[(k + 1) % 3] as usize]);
                *edges.entry((a.min(b), a.max(b))).or_default() += 1;
                neighbours.entry(a).or_default().insert(b);
                neighbours.entry(b).or_default().insert(a);
            }
        }
        let open: BTreeSet<usize> = edges
            .iter()
            .filter(|(_, n)| **n == 1)
            .flat_map(|((a, b), _)| [*a, *b])
            .collect();
        let corners: Vec<usize> = neighbours.keys().copied().collect();
        let mut bad = Vec::new();
        for &edge in &open {
            let mut near: BTreeSet<usize> = BTreeSet::from([edge]);
            let mut ring = vec![edge];
            for _ in 0..HEM_RINGS {
                ring = ring
                    .iter()
                    .flat_map(|c| neighbours[c].iter().copied())
                    .filter(|c| near.insert(*c))
                    .collect();
            }
            let p = glam::Vec3::from(all[edge]);
            let nearest = corners
                .iter()
                .filter(|c| !near.contains(c))
                .map(|c| p.distance(glam::Vec3::from(all[*c])))
                .fold(f32::MAX, f32::min);
            if !(HEM_THICK..=HEM_REACH).contains(&nearest) {
                bad.push((all[edge], nearest));
            }
        }
        println!(
            "LOD{level}: {} open corners, {} not inside a shell",
            open.len(),
            bad.len()
        );
        assert!(
            bad.is_empty(),
            "LOD{level}: raw-edge corners not 2 to 25 mm inside a shell: {:?}",
            &bad[..bad.len().min(8)]
        );
    }
}

/// A shoe is skinned as a shoe (§2 "Skinning", §3 "Footwear"): only the
/// foot's bones on its own side, at most four normalized influences; the
/// sole's front half never follows the shin (it would bend with the
/// ankle); the toe box follows the toes alone, so it bends only at the toe
/// joint behind it.
#[test]
fn the_sneakers_are_skinned_to_the_feet_with_a_rigid_sole_and_toe_box() {
    let library = library();
    let model = model(&library);
    let rig = model.rig.as_ref().expect("a rig");
    let bone_of = |joint: u16| rig.bones[rig.joints[joint as usize].bone].name.as_str();
    for top in &model.primitives {
        for (level, primitive) in std::iter::once(top).chain(top.lods.iter()).enumerate() {
            let name = format!("{} LOD{level}", top.material.name);
            let world = positions(&model, primitive);
            for ((p, joints), weights) in
                world.iter().zip(&primitive.joints).zip(&primitive.weights)
            {
                let side = if p[0] > 0.0 { "Left" } else { "Right" };
                for (joint, weight) in joints.iter().zip(weights) {
                    if *weight <= 0.0 {
                        continue;
                    }
                    let bone = bone_of(*joint);
                    assert!(
                        FOOT_BONES.iter().any(|b| bone == format!("{side}{b}")),
                        "{name}: a corner at {p:?} follows {bone}"
                    );
                    // The sole's front half bends with the toes only (round
                    // the heel the whole shoe follows the heel's skin, which
                    // the body's skinning carries partly with the shin).
                    if SOLE.contains(&top.material.name.as_str()) && p[2] > SOLE_FRONT {
                        assert!(
                            bone.ends_with("Foot") || bone.ends_with("Toes"),
                            "{name}: the sole's front at {p:?} follows {bone}"
                        );
                    }
                }
            }
        }
    }
    // The toe box at full detail: each shoe's front quarter on the toes.
    for side in [1.0f32, -1.0] {
        let mut corners = Vec::new();
        for primitive in &model.primitives {
            for (k, p) in positions(&model, primitive).iter().enumerate() {
                if p[0] * side > 0.0 {
                    corners.push((*p, primitive.joints[k], primitive.weights[k]));
                }
            }
        }
        let z: Vec<f32> = corners.iter().map(|c| c.0[2]).collect();
        let (back, front) = (
            z.iter().copied().fold(f32::MAX, f32::min),
            z.iter().copied().fold(f32::MIN, f32::max),
        );
        let mut toe_box = 0;
        for (p, joints, weights) in &corners {
            if (p[2] - back) / (front - back) < TOE_BOX_FROM {
                continue;
            }
            toe_box += 1;
            let on_toes: f32 = joints
                .iter()
                .zip(weights)
                .filter(|(j, _)| bone_of(**j).ends_with("Toes"))
                .map(|(_, w)| *w)
                .sum();
            assert!(
                on_toes > 0.999,
                "a toe-box corner at {p:?} is {on_toes} on the toes"
            );
        }
        assert!(toe_box > 50, "{toe_box} toe-box corners");
    }
}

/// The pair carries the body's frame shape (the feminine foot is shorter
/// and narrower), so it fits both frames.
#[test]
fn the_sneakers_follow_the_frame() {
    let library = library();
    let model = model(&library);
    let names: Vec<&str> = model
        .morph_targets
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert!(names.contains(&"Frame_Feminine"), "{names:?}");
}
