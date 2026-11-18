//! The first realistic wardrobe (Character Studio R7, milestone CO10): a
//! crew-neck T-shirt, joggers and a pair of sneakers, made by the garment
//! pipeline in `assets/characters/realistic/clothing/` and fitted to the
//! realistic body by `avatar::fit`. These tests hold the shipped items to
//! the parts of `docs/development/character-studio/asset-quality.md` §2
//! and §3 that can be read from the files: what the manifest declares,
//! the budgets and levels of detail, the texture coordinates, a sound mesh
//! with no single-layer edge, and that a draft is never offered by a
//! preset or by Randomize (§5). The fit on the cast, in motion and on the
//! feet is the player's (`wardrobe_tests.rs`).

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axcenity_assets::{Assets, Model, Primitive};
use axcenity_avatar::{
    ContentTier, Description, Fit, ItemKind, ItemManifest, Library, MaterialRole, Scope, randomize,
};

const REALISTIC: &str = "axcenity/realistic";

/// One of the three items: its ID, slot, object in its file, its LOD0
/// triangle budget (§3 "Budgets") and the pattern it was drafted after
/// (FreeSewing designs; none for a shoe, which is a shell).
struct Garment {
    id: &'static str,
    slot: &'static str,
    object: &'static str,
    budget: usize,
    pattern: Option<&'static str>,
}

const GARMENTS: [Garment; 3] = [
    Garment {
        id: "axcenity/realistic-crew-tee",
        slot: "top",
        object: "Shirt_CrewTee",
        budget: 6000,
        pattern: Some("teagan"),
    },
    Garment {
        id: "axcenity/realistic-joggers",
        slot: "bottom",
        object: "Pants_Joggers",
        budget: 6000,
        pattern: Some("paco"),
    },
    Garment {
        id: "axcenity/realistic-sneakers",
        slot: "shoes",
        object: "Shoes_CourtSneakers",
        budget: 4000,
        pattern: None,
    },
];

/// §2 "UVs": stretch under 10% on 95% of the area; texel density at least
/// 8 px/cm at 2K.
const STRETCH: f32 = 0.10;
const STRETCH_SHARE: f32 = 0.95;
const DENSITY: f32 = 800.0;
/// The share of a material's area that must meet the density: all of it,
/// as §2 asks (no allowance for slivers; a material that falls short is a
/// draft's record).
const DENSITY_SHARE: f32 = 1.0;
/// §3 "Construction": a hem turned with at least 2 mm thickness. A raw
/// edge lies inside the shell it is turned from, at least this far (the
/// full 2 mm, also where the turn rounds) and no farther than a hem's
/// allowance.
const HEM_THICK: f32 = 0.0020;
const HEM_REACH: f32 = 0.025;
/// A turned hem's raw edge is this many rings of corners or more from the
/// shell it lies inside (the fold between).
const HEM_RINGS: usize = 3;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn library() -> Library {
    let (library, _, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{problems:?}");
    library
}

fn item<'a>(library: &'a Library, garment: &Garment) -> &'a ItemManifest {
    library
        .item(garment.id)
        .unwrap_or_else(|| panic!("the library has no `{}`", garment.id))
}

/// The item's model, as the engine reads it.
fn model(library: &Library, garment: &Garment) -> Arc<Model> {
    let manifest = item(library, garment);
    let model = manifest.model.as_ref().expect("a model");
    let file = library.path_of(garment.id, &model.file).expect("its file");
    let id = format!("{file}#{}", model.object.as_deref().expect("an object"));
    Assets::new(root())
        .model_blocking(&id)
        .unwrap_or_else(|| panic!("`{id}` doesn't load"))
}

/// The JSON chunk of the item's `.glb`: what the file itself holds.
fn gltf_json(library: &Library, garment: &Garment) -> serde_json::Value {
    let manifest = item(library, garment);
    let file = library
        .path_of(garment.id, &manifest.model.as_ref().unwrap().file)
        .unwrap();
    let bytes = std::fs::read(root().join(file)).unwrap();
    assert_eq!(&bytes[0..4], b"glTF");
    let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    serde_json::from_slice(&bytes[20..20 + length]).unwrap()
}

/// A primitive's corners in the file's own space (metres): the engine
/// reads a rigged model into a fitted space (`Rig::fit`), which this
/// undoes, as the fit gate does.
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
fn the_first_garments_are_shipped_as_fitted_built_in_items() {
    let library = library();
    for garment in &GARMENTS {
        let manifest = item(&library, garment);
        let at = garment.id;
        assert_eq!(manifest.kind, ItemKind::Garment, "{at}");
        assert_eq!(manifest.slot, garment.slot, "{at}");
        assert_eq!(manifest.bases, vec![REALISTIC.to_owned()], "{at}");
        assert_eq!(manifest.tier, ContentTier::BuiltIn, "{at}");
        assert_eq!(manifest.layer, Some(1), "{at}");
        // Its quality is said, not left to the default.
        assert!(manifest.quality.is_some(), "{at}: quality isn't declared");
        assert!(manifest.about.is_some(), "{at}: no `about`");
        let model = manifest.model.as_ref().expect("a model");
        assert_eq!(
            model.object.as_deref(),
            Some(format!("AxcenityRealistic/{}", garment.object).as_str()),
            "{at}"
        );
        // Fitted by binding, with the binding kept.
        match &manifest.fit {
            Fit::Binding {
                data: Some(data), ..
            } => {
                let path = library.path_of(at, data).unwrap();
                assert!(root().join(path).is_file(), "{at}: no binding file");
            }
            other => panic!("{at}: fitted by {other:?}"),
        }
        let thumbnail = manifest.thumbnail.as_deref().expect("a thumbnail");
        assert!(
            root()
                .join(library.path_of(at, thumbnail).unwrap())
                .is_file(),
            "{at}: no thumbnail"
        );
        // Where it came from: our make, the pattern (FreeSewing, MIT) and
        // the fabrics (CC0: Poly Haven or ambientCG), with a link.
        let source = manifest.source.as_ref().expect("a source");
        assert!(source.url.is_some(), "{at}: no link");
        assert!(
            source.author.contains("Poly Haven") || source.author.contains("ambientCG"),
            "{at}: {source:?}"
        );
        assert!(source.author.contains("CC0"), "{at}: {source:?}");
        if let Some(pattern) = garment.pattern {
            assert!(
                source.author.contains("FreeSewing") && source.author.contains(pattern),
                "{at}: {source:?}"
            );
        }
        assert!(manifest.validate().is_empty(), "{:?}", manifest.validate());
        let base = library.base(REALISTIC).unwrap();
        assert!(
            manifest.check_against(base).is_empty(),
            "{:?}",
            manifest.check_against(base)
        );
    }
}

#[test]
fn their_materials_have_colour_roles_and_variants() {
    let library = library();
    for garment in &GARMENTS {
        let manifest = item(&library, garment);
        let at = garment.id;
        let model = model(&library, garment);
        // Every material the model draws is a slot of the manifest.
        for primitive in &model.primitives {
            assert!(
                manifest.material(&primitive.material.name).is_some(),
                "{at}: `{}` isn't a slot",
                primitive.material.name
            );
        }
        let roles: Vec<_> = manifest.materials.iter().filter_map(|m| m.role).collect();
        assert!(roles.contains(&MaterialRole::Primary), "{at}: {roles:?}");
        assert!(
            roles.contains(&MaterialRole::Secondary) || roles.contains(&MaterialRole::Accent),
            "{at}: {roles:?}"
        );
        // Colour and material variants: at least three, each naming only
        // the item's own slots (the manifest's checks hold the values).
        assert!(manifest.variants.len() >= 3, "{at}");
        for variant in &manifest.variants {
            for name in variant.materials.keys() {
                assert!(manifest.material(name).is_some(), "{at}: {name}");
            }
        }
        // The fabrics are drawn as cloth, with a sheen (cloth.md's
        // contract), and their maps are 2K.
        let fabrics: Vec<_> = model
            .primitives
            .iter()
            .filter(|p| p.material.shading == axcenity_assets::ModelShading::Cloth)
            .collect();
        assert!(!fabrics.is_empty(), "{at}: no cloth");
        for primitive in fabrics {
            let material = &primitive.material;
            assert!(
                material.sheen.is_some(),
                "{at}: {} has no sheen",
                material.name
            );
            let colour = material.base_color_texture.as_ref().expect("a colour map");
            assert!(colour.width >= 2048, "{at}: {}", colour.width);
            assert!(material.normal_texture.is_some(), "{at}: no normal map");
        }
    }
}

#[test]
fn they_keep_to_their_budgets_with_three_levels_of_their_own() {
    let library = library();
    for garment in &GARMENTS {
        let at = garment.id;
        let model = model(&library, garment);
        let lod0: usize = model.primitives.iter().map(triangles).sum();
        println!("{at}: {lod0} triangles at LOD0");
        assert!(lod0 <= garment.budget, "{at}: {lod0} > {}", garment.budget);
        for level in 1..=2 {
            let count: usize = model
                .primitives
                .iter()
                .map(|p| p.lods.get(level - 1).map_or(usize::MAX / 8, triangles))
                .sum();
            let before: usize = if level == 1 {
                lod0
            } else {
                model.primitives.iter().map(|p| triangles(&p.lods[0])).sum()
            };
            println!("{at}: {count} triangles at LOD{level}");
            assert!(
                count * 10 <= before * 6,
                "{at}: LOD{level} {count} of {before}"
            );
        }
        // The levels are the file's own (made from the pattern and fitted),
        // not simplified on load.
        let json = gltf_json(&library, garment);
        let names: BTreeSet<&str> = json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n["name"].as_str())
            .collect();
        for level in 1..=2 {
            let name = format!("{}_LOD{level}", garment.object);
            assert!(
                names.contains(name.as_str()),
                "{at}: no {name} in {names:?}"
            );
        }
    }
}

/// A triangle's area in metres and in texels, and how its texture
/// coordinates stretch it: the two singular values of the map from the
/// surface to the texture (texels per metre).
fn texel_map(p: [[f32; 3]; 3], uv: [[f32; 2]; 3], size: [f32; 2]) -> (f32, f32, f32) {
    use glam::{Vec2, Vec3};
    let (a, b, c) = (Vec3::from(p[0]), Vec3::from(p[1]), Vec3::from(p[2]));
    let e1 = b - a;
    let e2 = c - a;
    let area = e1.cross(e2).length() / 2.0;
    if area < 1e-12 {
        return (0.0, 0.0, 0.0);
    }
    // An orthonormal frame on the triangle.
    let x = e1.normalize();
    let y = e1.cross(e2).cross(e1).normalize();
    let (p1, p2) = (Vec2::new(e1.dot(x), 0.0), Vec2::new(e2.dot(x), e2.dot(y)));
    let s = Vec2::from(size);
    let t0 = Vec2::from(uv[0]) * s;
    let (t1, t2) = (Vec2::from(uv[1]) * s - t0, Vec2::from(uv[2]) * s - t0);
    // J maps the frame's (x, y) to texels: [t1 t2] = J [p1 p2].
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
    // Singular values of a 2x2 matrix.
    let (a, b, c, d) = (j[0][0], j[0][1], j[1][0], j[1][1]);
    let s1 = a * a + b * b + c * c + d * d;
    let s2 = ((a * a + b * b - c * c - d * d).powi(2) + 4.0 * (a * c + b * d).powi(2)).sqrt();
    let high = ((s1 + s2) / 2.0).max(0.0).sqrt();
    let low = ((s1 - s2) / 2.0).max(0.0).sqrt();
    (area, high, low)
}

/// Open defects of drafts, recorded so they can only get better (as the
/// fit gate's `Known`): the least share a draft's material reaches of
/// §2's UV limits (even, dense), and the most raw-edge corners an item has
/// outside §3's fold thickness. A production item has none; a record no
/// longer earned (the item now passes, or does better by more than
/// `SLACK`) fails, so it is lowered or removed.
struct UvRecord {
    item: &'static str,
    material: &'static str,
    even: Option<f32>,
    dense: Option<f32>,
}

const SLACK: f32 = 0.02;

const fn uv(
    item: &'static str,
    material: &'static str,
    even: Option<f32>,
    dense: Option<f32>,
) -> UvRecord {
    UvRecord {
        item,
        material,
        even,
        dense,
    }
}

const TEE: &str = "axcenity/realistic-crew-tee";
const JOGGERS: &str = "axcenity/realistic-joggers";
const SNEAKERS: &str = "axcenity/realistic-sneakers";

// What the drafts still fail (`refinement/wave4/r7-wardrobe-first.md`,
// open issues): the share of each material's area within 10% of its
// median texel scale ("even") and at 8 px/cm or more ("dense").
// - The tee's jersey: the drape keeps 95% of its edges within 10% of the
//   pattern, but the fit pushes the tee out over the sports bra and the
//   joggers' waistband (up to a centimetre), stretching the shoulders,
//   sleeve caps and hem. Its rib band leans in to the neck, so its rows
//   are shorter than its seam (a rectangle in the texture).
// - The joggers: the waist and cuffs gathered on elastic, and the bands
//   pushed clear of the belly and the heel, stretch the cloth near them;
//   the aglets are short tubes. The drawcord is a thin tube whose ends,
//   tapered into the aglets, fall under 8 px/cm (0.9% of its area).
// - The sneakers: the upper is one ring of quads from the sole to the
//   collar, laid out by each row's length round and each column's up, which
//   isn't conformal (rows of very different lengths); the sole's caps are
//   projected from above, and its side wall is a strip.
const UV_RECORDS: [UvRecord; 12] = [
    uv(TEE, "Jersey", Some(0.51), Some(0.96)),
    uv(TEE, "Rib", Some(0.045), None),
    uv(JOGGERS, "Fleece", Some(0.39), Some(0.96)),
    uv(JOGGERS, "Rib", Some(0.36), Some(0.92)),
    uv(JOGGERS, "Cord", None, Some(0.99)),
    uv(JOGGERS, "Aglet", Some(0.65), Some(0.92)),
    uv(SNEAKERS, "Upper", Some(0.03), Some(0.79)),
    uv(SNEAKERS, "Lining", Some(0.045), None),
    uv(SNEAKERS, "Midsole", Some(0.13), Some(0.71)),
    uv(SNEAKERS, "Outsole", Some(0.69), Some(0.97)),
    uv(SNEAKERS, "Tongue", Some(0.18), Some(0.95)),
    uv(SNEAKERS, "Laces", Some(0.75), Some(0.97)),
];

/// Raw-edge corners under §3's 2 mm: the tee's hem at the side, where the
/// fit pushes it out over the joggers' waistband and its turn-up is
/// squeezed to 1.4 mm; and one raw-edge corner of a joggers' cuff (at
/// [0.136, 0.187, 0.004] m in the file), 1.92 mm from the shell.
const THIN_FOLDS: [(&str, usize); 2] = [(TEE, 1), (JOGGERS, 1)];

fn draft_floor(
    library: &Library,
    garment: &Garment,
    material: &str,
    records: &[UvRecord],
) -> Option<(Option<f32>, Option<f32>)> {
    let record = records
        .iter()
        .find(|r| r.item == garment.id && r.material == material)?;
    assert!(
        item(library, garment).is_draft(),
        "{} is production: it has no record",
        garment.id
    );
    Some((record.even, record.dense))
}

/// A share measured against its limit, or a draft's record of it.
fn within(label: &str, share: f32, limit: f32, record: Option<f32>) -> Option<String> {
    match record {
        None if share < limit => Some(format!(
            "{label} {:.1}% < {:.0}%",
            share * 100.0,
            limit * 100.0
        )),
        Some(r) if share < r => Some(format!(
            "{label} {:.1}% < recorded {:.1}%",
            share * 100.0,
            r * 100.0
        )),
        Some(_) if share >= limit => Some(format!(
            "{label} {:.1}% passes: remove the record",
            share * 100.0
        )),
        Some(r) if share > r + SLACK => Some(format!(
            "{label} {:.1}%: raise the record from {:.1}%",
            share * 100.0,
            r * 100.0
        )),
        _ => None,
    }
}

#[test]
fn their_texture_coordinates_are_even_dense_and_do_not_overlap() {
    let library = library();
    let mut failures: Vec<String> = Vec::new();
    for garment in &GARMENTS {
        let at = garment.id;
        let model = model(&library, garment);
        for primitive in &model.primitives {
            let material = &primitive.material;
            let Some(colour) = material.base_color_texture.as_ref() else {
                continue;
            };
            assert_eq!(primitive.uvs.len(), primitive.positions.len(), "{at}");
            let size = [colour.width as f32, colour.height as f32];
            // Each triangle's stretch against the piece's median scale, and
            // its density (the smaller singular value: the sparser way).
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
                "{at} {}: median {:.0} px/m, even {:.1}%, dense {:.1}%",
                material.name,
                median,
                100.0 * even / total,
                100.0 * dense / total
            );
            let floor = draft_floor(&library, garment, &material.name, &UV_RECORDS);
            failures.extend(within(
                &format!("{at} {}: even", material.name),
                even / total,
                STRETCH_SHARE,
                floor.and_then(|r| r.0),
            ));
            failures.extend(within(
                &format!("{at} {}: dense", material.name),
                dense / total,
                DENSITY_SHARE,
                floor.and_then(|r| r.1),
            ));
            // No two triangles on one texel (no deliberate mirroring in
            // these items): drawn at 512 px, under 0.5% of the texels used
            // are drawn twice (edges shared by neighbours aren't counted:
            // a texel counts when its middle is inside a triangle).
            const N: usize = 512;
            let mut hits = vec![0u8; N * N];
            for t in primitive.indices.chunks_exact(3) {
                let uv = [0, 1, 2].map(|k| primitive.uvs[t[k] as usize]);
                let px = uv.map(|q| [q[0] * N as f32, q[1] * N as f32]);
                let (x0, x1) = (
                    px.iter()
                        .map(|q| q[0])
                        .fold(f32::MAX, f32::min)
                        .floor()
                        .max(0.0) as usize,
                    px.iter()
                        .map(|q| q[0])
                        .fold(f32::MIN, f32::max)
                        .ceil()
                        .min(N as f32) as usize,
                );
                let (y0, y1) = (
                    px.iter()
                        .map(|q| q[1])
                        .fold(f32::MAX, f32::min)
                        .floor()
                        .max(0.0) as usize,
                    px.iter()
                        .map(|q| q[1])
                        .fold(f32::MIN, f32::max)
                        .ceil()
                        .min(N as f32) as usize,
                );
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
            println!(
                "{at} {}: {twice} of {used} texels drawn twice",
                material.name
            );
            if twice * 200 > used {
                failures.push(format!("{at} {}: overlapping", material.name));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Corners at one place are one (texture seams split corners): the index
/// of the first corner at each corner's place.
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

#[test]
fn their_meshes_are_sound() {
    let library = library();
    for garment in &GARMENTS {
        let at = garment.id;
        let model = model(&library, garment);
        for primitive in &model.primitives {
            let name = &primitive.material.name;
            let world = positions(&model, primitive);
            // No degenerate triangles.
            for t in primitive.indices.chunks_exact(3) {
                let [a, b, c] = [0, 1, 2].map(|k| glam::Vec3::from(world[t[k] as usize]));
                let area = (b - a).cross(c - a).length() / 2.0;
                assert!(area > 1e-9, "{at} {name}: a degenerate triangle {t:?}");
            }
            // Skinned with at most four normalized influences.
            assert_eq!(primitive.weights.len(), primitive.positions.len(), "{at}");
            for w in &primitive.weights {
                let sum: f32 = w.iter().sum();
                assert!(
                    (sum - 1.0).abs() < 2e-3,
                    "{at} {name}: weights add to {sum}"
                );
            }
            // Each edge (corners welded) has at most two triangles, and two
            // that share one run it opposite ways (consistent normals).
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
                    "{at} {name}: edge {edge:?} has {} triangles",
                    ways.len()
                );
                if ways.len() == 2 {
                    assert_ne!(ways[0], ways[1], "{at} {name}: a flipped face at {edge:?}");
                }
            }
        }
    }
}

/// No single-layer edge (§3 "Construction"): every open edge of a garment
/// is the raw edge of a turned hem, band or lining, lying inside the shell
/// it was turned from, at least `HEM_THICK` and at most `HEM_REACH` from
/// it, with the fold `HEM_RINGS` rings or more away along the cloth.
#[test]
fn no_edge_is_a_single_layer() {
    let library = library();
    for garment in &GARMENTS {
        let at = garment.id;
        let model = model(&library, garment);
        // The whole item, corners welded across its pieces.
        let mut all = Vec::new();
        let mut tris = Vec::new();
        for primitive in &model.primitives {
            let start = all.len() as u32;
            all.extend(positions(&model, primitive));
            tris.extend(
                primitive
                    .indices
                    .chunks_exact(3)
                    .map(|t| [t[0] + start, t[1] + start, t[2] + start]),
            );
        }
        let positions = all;
        let weld = welded(&positions);
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
        let mut bad = 0;
        let mut worst = (f32::MAX, 0usize);
        for &edge in &open {
            // Corners within `HEM_RINGS` rings along the cloth are the
            // edge's own strip.
            let mut near: BTreeSet<usize> = BTreeSet::from([edge]);
            let mut ring = vec![edge];
            for _ in 0..HEM_RINGS {
                ring = ring
                    .iter()
                    .flat_map(|c| neighbours[c].iter().copied())
                    .filter(|c| near.insert(*c))
                    .collect();
            }
            let p = glam::Vec3::from(positions[edge]);
            let nearest = corners
                .iter()
                .filter(|c| !near.contains(c))
                .map(|c| p.distance(glam::Vec3::from(positions[*c])))
                .fold(f32::MAX, f32::min);
            if !(HEM_THICK..=HEM_REACH).contains(&nearest) {
                bad += 1;
                if nearest < worst.0 || worst.0 == f32::MAX {
                    worst = (nearest, edge);
                }
            }
        }
        println!(
            "{at}: {} open corners, {bad} not inside a shell (e.g. {:?} at {:.2} mm)",
            open.len(),
            positions[worst.1],
            worst.0 * 1000.0
        );
        let recorded = THIN_FOLDS.iter().find(|(id, _)| *id == at).map(|(_, n)| *n);
        match recorded {
            None => assert_eq!(
                bad,
                0,
                "{at}: raw-edge corners not {:.1} to {:.0} mm inside a shell",
                HEM_THICK * 1000.0,
                HEM_REACH * 1000.0
            ),
            Some(n) => {
                assert!(item(&library, garment).is_draft(), "{at} is production");
                assert!(bad <= n, "{at}: {bad} thin folds, recorded {n}");
                assert!(
                    bad == n,
                    "{at}: {bad} thin folds, recorded {n}: lower the record"
                );
            }
        }
    }
}

/// §5: a draft is never in a shipped preset, and Randomize never draws it;
/// an outfit preset of the three exists only when all three are
/// production.
#[test]
fn drafts_are_in_no_preset_and_never_drawn_at_random() {
    let library = library();
    let drafts: Vec<&str> = GARMENTS
        .iter()
        .map(|g| item(&library, g))
        .filter(|i| i.is_draft())
        .map(|i| i.id.as_str())
        .collect();
    println!("drafts: {drafts:?}");
    for (id, preset) in &library.presets {
        for worn in &preset.items {
            assert!(
                !drafts.contains(&worn.id.as_str()),
                "the preset `{id}` wears the draft `{}`",
                worn.id
            );
        }
    }
    let start = Description::from_json(
        &std::fs::read_to_string(
            root().join("benchmarks/realistic-masculine-cast-average.character.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut drawn = BTreeSet::new();
    for seed in 0..400 {
        let d = randomize(&start, &Scope::All, &BTreeSet::new(), seed, &library);
        for worn in &d.items {
            drawn.insert(worn.id.clone());
        }
    }
    println!("drawn at random: {drawn:?}");
    for draft in &drafts {
        assert!(
            !drawn.contains(*draft),
            "Randomize drew the draft `{draft}`"
        );
    }
    // A production item is offered: Randomize draws it.
    for garment in &GARMENTS {
        if !item(&library, garment).is_draft() {
            assert!(drawn.contains(garment.id), "{} is never drawn", garment.id);
        }
    }
    let outfit = library.presets.values().any(|p| {
        GARMENTS
            .iter()
            .all(|g| p.items.iter().any(|w| w.id == g.id))
    });
    assert_eq!(
        outfit,
        drafts.is_empty(),
        "an outfit preset only when all three are production"
    );
}
