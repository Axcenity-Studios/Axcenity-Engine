//! The first licensed joggers (wave 5, Paul's decisions 11 and 13):
//! BlenderKit's Grey sweat pants (CC0, made from MakeHuman's CC0
//! male_casualsuit06), refined and fitted by the licensed-asset intake
//! (`assets/characters/realistic/import/`). These tests hold the shipped
//! item to what `docs/development/character-studio/asset-quality.md` §2,
//! §3 and §5 ask that can be read from its files, with **no records**:
//! its provenance (the licence, the hashes, the evidence kept beside the
//! record), the manifest (a draft until the review, colour roles,
//! variants, cloth with sheen and 2K maps), the budget and three levels,
//! even and dense texture coordinates that don't overlap, a sound mesh
//! with no face flipped or folded over, and no single-layer edge. The fit on the cast, in motion and at the
//! knee and waist is the player's (`fleece_joggers_tests.rs`).

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axcenity_assets::{Assets, Model, ModelShading, Primitive};
use axcenity_avatar::{
    ContentTier, Fit, ItemKind, ItemManifest, Library, MaterialRole, Origin, check_item_provenance,
    read_provenance,
};

const ID: &str = "axcenity/realistic-fleece-joggers";
const OBJECT: &str = "Pants_FleeceJoggers";
const REALISTIC: &str = "axcenity/realistic";
/// §3 "Budgets": a bottom's LOD0.
const BUDGET: usize = 6000;
/// §2 "UVs": stretch under 10% on 95% of the area; at least 8 px/cm at 2K
/// on all of it.
const STRETCH: f32 = 0.10;
const STRETCH_SHARE: f32 = 0.95;
const DENSITY: f32 = 800.0;
/// §3 "Construction": a raw edge lies inside the shell it is turned from,
/// 2 mm to a hem's allowance away, three or more rings along the cloth
/// from it (as `realistic_wardrobe.rs` holds the wave-4 items).
const HEM_THICK: f32 = 0.0020;
const HEM_REACH: f32 = 0.025;
const HEM_RINGS: usize = 3;
/// The SHA-256 of the download (`content-sourcing.md` §5).
const DOWNLOAD_SHA256: &str = "04abe82b560270bc7612f0bbc8ae05209917bbf3476949c9dd3588804f1fa235";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn library() -> Library {
    let (library, _, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{problems:?}");
    library
}

fn manifest(library: &Library) -> &ItemManifest {
    library
        .item(ID)
        .unwrap_or_else(|| panic!("`{ID}` isn't in the library"))
}

/// The item's model, as the engine reads it.
fn model(library: &Library) -> Arc<Model> {
    let model = manifest(library).model.as_ref().expect("a model");
    let file = library.path_of(ID, &model.file).expect("its file");
    let id = format!("{file}#{}", model.object.as_deref().expect("an object"));
    Assets::new(root())
        .model_blocking(&id)
        .unwrap_or_else(|| panic!("`{id}` doesn't load"))
}

/// A primitive's corners in the file's own space (metres): the engine
/// reads a rigged model into a fitted space (`Rig::fit`), undone here.
fn positions(model: &Model, primitive: &Primitive) -> Vec<[f32; 3]> {
    let unfit = model.rig.as_ref().map(|r| r.fit.inverse());
    primitive
        .positions
        .iter()
        .map(|p| {
            let p = glam::Vec3::from(*p);
            unfit.map_or(p, |m| m.transform_point3(p)).to_array()
        })
        .collect()
}

fn triangles(primitive: &Primitive) -> usize {
    primitive.indices.len() / 3
}

#[test]
fn the_joggers_are_an_imported_draft_with_a_sound_provenance_record() {
    let library = library();
    let manifest = manifest(&library);
    assert_eq!(manifest.kind, ItemKind::Garment);
    assert_eq!(manifest.slot, "bottom");
    assert_eq!(manifest.layer, Some(1));
    assert_eq!(manifest.bases, vec![REALISTIC.to_owned()]);
    assert_eq!(manifest.tier, ContentTier::BuiltIn);
    // Nothing becomes Production without the lead's review and Paul's
    // sign-off of the first bottom (asset-quality.md §1, §5).
    assert!(manifest.is_draft(), "the licensed joggers stay a draft");
    assert!(manifest.about.is_some());
    assert_eq!(
        manifest.model.as_ref().unwrap().object.as_deref(),
        Some(format!("AxcenityRealistic/{OBJECT}").as_str())
    );
    match &manifest.fit {
        Fit::Binding {
            data: Some(data), ..
        } => assert!(root().join(library.path_of(ID, data).unwrap()).is_file()),
        other => panic!("fitted by {other:?}"),
    }
    assert!(manifest.validate().is_empty(), "{:?}", manifest.validate());
    let base = library.base(REALISTIC).unwrap();
    assert!(manifest.check_against(base).is_empty());
    // Imported, CC0, with its record beside it and sound.
    let source = manifest.source.as_ref().expect("a source");
    assert_eq!(source.origin, Some(Origin::Imported));
    assert_eq!(source.licence, "CC0-1.0");
    let record_path = root().join(
        library
            .path_of(ID, source.provenance.as_deref().expect("a record"))
            .unwrap(),
    );
    let record = read_provenance(&root(), &library, ID)
        .expect("the record reads")
        .expect("the record is there");
    assert!(
        check_item_provenance(manifest, Some(&record)).is_empty(),
        "{:?}",
        check_item_provenance(manifest, Some(&record))
    );
    assert!(record.items.contains(&ID.to_owned()));
    // The download, by its hash as downloaded.
    assert!(
        record.files.iter().any(|f| f.sha256 == DOWNLOAD_SHA256),
        "{:?}",
        record.files
    );
    // What it was made from (MakeHuman's CC0 male_casualsuit06) and the
    // open question of who made the upload are said, not left out.
    let evidence = record.evidence.as_deref().unwrap_or_default();
    for said in [
        "male_casualsuit06",
        "MakeHuman",
        "CC0",
        "could not be confirmed",
    ] {
        assert!(evidence.contains(said), "the evidence doesn't say `{said}`");
    }
    // The page as retrieved is kept beside the record, by its hash.
    let folder = record_path.parent().unwrap();
    let saved: Vec<PathBuf> = std::fs::read_dir(folder)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains(".source-page."))
        })
        .collect();
    assert!(!saved.is_empty(), "no saved source page beside the record");
    for page in saved {
        let text = std::fs::read_to_string(&page).unwrap();
        assert!(
            text.contains("2026-11-18"),
            "{}: no retrieval date",
            page.display()
        );
        assert!(text.contains("Creative commons zero"), "{}", page.display());
    }
    // Its changes say what the scripted refinement did.
    let changes = record.changes.join("; ");
    for said in ["ankle", "knee", "waistband", "drawcord"] {
        assert!(
            changes.contains(said),
            "the changes don't say `{said}`: {changes}"
        );
    }
}

#[test]
fn the_joggers_have_colour_roles_variants_and_cloth_with_sheen() {
    let library = library();
    let manifest = manifest(&library);
    let model = model(&library);
    for primitive in &model.primitives {
        assert!(
            manifest.material(&primitive.material.name).is_some(),
            "`{}` isn't a slot",
            primitive.material.name
        );
    }
    let role = |name: &str| manifest.material(name).and_then(|m| m.role);
    assert_eq!(role("Fleece"), Some(MaterialRole::Primary));
    assert_eq!(role("Rib"), Some(MaterialRole::Secondary));
    assert_eq!(role("Cord"), Some(MaterialRole::Accent));
    assert!(manifest.variants.len() >= 3);
    for variant in &manifest.variants {
        for name in variant.materials.keys() {
            assert!(manifest.material(name).is_some(), "{name}");
        }
    }
    let fabrics: Vec<_> = model
        .primitives
        .iter()
        .filter(|p| p.material.shading == ModelShading::Cloth)
        .collect();
    assert!(fabrics.len() >= 3, "fleece, rib and cord are cloth");
    for primitive in fabrics {
        let material = &primitive.material;
        assert!(material.sheen.is_some(), "{} has no sheen", material.name);
        let colour = material.base_color_texture.as_ref().expect("a colour map");
        assert!(colour.width >= 2048, "{}", colour.width);
        assert!(material.normal_texture.is_some(), "no normal map");
    }
    // One set of maps, not a copy per material: the file stays small.
    let size = std::fs::metadata(
        root().join(
            library
                .path_of(ID, &manifest.model.as_ref().unwrap().file)
                .unwrap(),
        ),
    )
    .unwrap()
    .len();
    println!("{ID}: {:.1} MB", size as f64 / 1e6);
    assert!(size < 16_000_000, "{size} bytes");
}

#[test]
fn the_joggers_keep_to_the_budget_with_three_levels_of_their_own() {
    let library = library();
    let model = model(&library);
    let lod0: usize = model.primitives.iter().map(triangles).sum();
    println!("{ID}: {lod0} triangles at LOD0");
    assert!(lod0 <= BUDGET, "{lod0} > {BUDGET}");
    let mut before = lod0;
    for level in 1..=2 {
        let count: usize = model
            .primitives
            .iter()
            .map(|p| p.lods.get(level - 1).map_or(usize::MAX / 8, triangles))
            .sum();
        println!("{ID}: {count} triangles at LOD{level}");
        assert!(count * 10 <= before * 6, "LOD{level} {count} of {before}");
        before = count;
    }
}

/// A triangle's area and the two singular values of its map from the
/// surface to the texture (texels per metre).
fn texel_map(p: [[f32; 3]; 3], uv: [[f32; 2]; 3], size: [f32; 2]) -> (f32, f32, f32) {
    use glam::{Vec2, Vec3};
    let (a, b, c) = (Vec3::from(p[0]), Vec3::from(p[1]), Vec3::from(p[2]));
    let (e1, e2) = (b - a, c - a);
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
    let (a, b) = (
        t1.x * inv[0][0] + t2.x * inv[1][0],
        t1.x * inv[0][1] + t2.x * inv[1][1],
    );
    let (c, d) = (
        t1.y * inv[0][0] + t2.y * inv[1][0],
        t1.y * inv[0][1] + t2.y * inv[1][1],
    );
    let s1 = a * a + b * b + c * c + d * d;
    let s2 = ((a * a + b * b - c * c - d * d).powi(2) + 4.0 * (a * c + b * d).powi(2)).sqrt();
    let high = ((s1 + s2) / 2.0).max(0.0).sqrt();
    let low = ((s1 - s2) / 2.0).max(0.0).sqrt();
    (area, high, low)
}

/// §2 "UVs", on every textured material: 95% of the area within 10% of
/// the material's median texel scale both ways, all of it at 8 px/cm or
/// more, and no two triangles on one texel.
#[test]
fn the_joggers_texture_coordinates_are_even_dense_and_do_not_overlap() {
    let library = library();
    let model = model(&library);
    let mut failures = Vec::new();
    for primitive in &model.primitives {
        let material = &primitive.material;
        let Some(colour) = material.base_color_texture.as_ref() else {
            continue;
        };
        assert_eq!(primitive.uvs.len(), primitive.positions.len());
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
            "{}: median {:.0} px/m, even {:.1}%, dense {:.2}%",
            material.name,
            median,
            100.0 * even / total,
            100.0 * dense / total
        );
        if even / total < STRETCH_SHARE {
            failures.push(format!(
                "{}: even {:.1}%",
                material.name,
                100.0 * even / total
            ));
        }
        if dense < total {
            failures.push(format!(
                "{}: dense {:.2}%",
                material.name,
                100.0 * dense / total
            ));
        }
        // Overlap: drawn at 512 px, under 0.5% of the texels used drawn
        // twice (a texel counts when its middle is inside a triangle).
        const N: usize = 512;
        let mut hits = vec![0u8; N * N];
        for t in primitive.indices.chunks_exact(3) {
            let px = [0, 1, 2].map(|k| {
                let q = primitive.uvs[t[k] as usize];
                [q[0] * N as f32, q[1] * N as f32]
            });
            let lo = |i: usize| {
                px.iter()
                    .map(|q| q[i])
                    .fold(f32::MAX, f32::min)
                    .floor()
                    .max(0.0) as usize
            };
            let hi = |i: usize| {
                px.iter()
                    .map(|q| q[i])
                    .fold(f32::MIN, f32::max)
                    .ceil()
                    .min(N as f32) as usize
            };
            let edge = |a: [f32; 2], b: [f32; 2], x: f32, y: f32| {
                (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
            };
            let whole = edge(px[0], px[1], px[2][0], px[2][1]);
            if whole.abs() < 1e-9 {
                continue;
            }
            for y in lo(1)..hi(1) {
                for x in lo(0)..hi(0) {
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
        println!("{}: {twice} of {used} texels drawn twice", material.name);
        if twice * 200 > used {
            failures.push(format!("{}: overlapping", material.name));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Corners at one place are one.
fn welded(positions: &[[f32; 3]]) -> Vec<usize> {
    let mut first: HashMap<[i64; 3], usize> = HashMap::new();
    positions
        .iter()
        .enumerate()
        .map(|(k, p)| {
            *first
                .entry(p.map(|c| (c * 1e5).round() as i64))
                .or_insert(k)
        })
        .collect()
}

/// §2 "Geometry" and "Skinning": no degenerate triangle, at most four
/// normalized weights, no edge in three triangles, neighbours wound the
/// same way (whether faces fold over is
/// `no_face_of_the_joggers_is_flipped_or_folded_over_at_rest`).
#[test]
fn the_joggers_mesh_is_sound() {
    let library = library();
    let model = model(&library);
    for primitive in &model.primitives {
        let name = &primitive.material.name;
        let world = positions(&model, primitive);
        for t in primitive.indices.chunks_exact(3) {
            let [a, b, c] = [0, 1, 2].map(|k| glam::Vec3::from(world[t[k] as usize]));
            let area = (b - a).cross(c - a).length() / 2.0;
            assert!(area > 1e-9, "{name}: a degenerate triangle {t:?}");
        }
        assert_eq!(primitive.weights.len(), primitive.positions.len());
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

/// How far two faces sharing an edge may turn from each other (degrees)
/// before the edge counts as a fold-over: a fin, two faces back to back.
/// The turned waistband and cuffs bend round their rounded fold over
/// several rings, so no edge of a sound turn comes near this.
const FIN: f32 = 140.0;

/// §2 "Geometry": "normals consistent (no flipped faces)", at rest, at
/// every level. `the_joggers_mesh_is_sound` checks only that neighbouring
/// triangles wind the same way; a strip can wind consistently and still
/// fold over, so this checks the surface itself: no triangle faces
/// against its own corners' normals (the shading's), and no edge folds
/// back on itself (a fin, its two faces turned more than [`FIN`] apart).
#[test]
fn no_face_of_the_joggers_is_flipped_or_folded_over_at_rest() {
    use glam::Vec3;
    let library = library();
    let model = model(&library);
    let unfit = model.rig.as_ref().map(|r| r.fit.inverse());
    let mut failures = Vec::new();
    for primitive in &model.primitives {
        let levels = std::iter::once(primitive).chain(primitive.lods.iter());
        for (level, mesh) in levels.enumerate() {
            let name = format!("{} LOD{level}", primitive.material.name);
            let world: Vec<Vec3> = positions(&model, mesh)
                .into_iter()
                .map(Vec3::from)
                .collect();
            let normals: Vec<Vec3> = mesh
                .normals
                .iter()
                .map(|n| {
                    let n = Vec3::from(*n);
                    unfit
                        .map_or(n, |m| m.transform_vector3(n))
                        .normalize_or_zero()
                })
                .collect();
            assert_eq!(normals.len(), world.len(), "{name}: a normal per corner");
            // Triangles facing against their corners' normals.
            let (mut flipped, mut flipped_area) = (Vec::new(), 0.0f32);
            let mut faces = Vec::new();
            for t in mesh.indices.chunks_exact(3) {
                let [a, b, c] = [0, 1, 2].map(|k| world[t[k] as usize]);
                let cross = (b - a).cross(c - a);
                let face = cross.normalize_or_zero();
                faces.push(face);
                let shading = [0, 1, 2].map(|k| normals[t[k] as usize]);
                let mean = (shading[0] + shading[1] + shading[2]).normalize_or_zero();
                if face.dot(mean) < 0.0 {
                    flipped_area += cross.length() / 2.0;
                    flipped.push(((a + b + c) / 3.0, face.dot(mean), cross.length() * 5e5));
                }
            }
            // Fins: edges whose two faces turn more than `FIN` apart.
            let weld = welded(&world.iter().map(|p| p.to_array()).collect::<Vec<_>>());
            let mut edges: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
            for (f, t) in mesh.indices.chunks_exact(3).enumerate() {
                for k in 0..3 {
                    let (a, b) = (weld[t[k] as usize], weld[t[(k + 1) % 3] as usize]);
                    edges.entry((a.min(b), a.max(b))).or_default().push(f);
                }
            }
            let limit = FIN.to_radians().cos();
            let mut fins: Vec<(f32, Vec3)> = edges
                .iter()
                .filter(|(_, f)| f.len() == 2)
                .map(|((a, b), f)| {
                    let turn = faces[f[0]].dot(faces[f[1]]).clamp(-1.0, 1.0);
                    (turn, (world[*a] + world[*b]) / 2.0)
                })
                .filter(|(turn, _)| *turn < limit)
                .map(|(turn, at)| (turn.acos().to_degrees(), at))
                .collect();
            fins.sort_by(|x, y| y.0.total_cmp(&x.0));
            let round = |p: Vec3| (p * 1000.0).round() / 1000.0;
            println!(
                "{name}: {} triangles against their normals ({:.0} mm²), {} fins over {FIN} degrees",
                flipped.len(),
                flipped_area * 1e6,
                fins.len()
            );
            for (at, dot, area) in &flipped {
                println!(
                    "  flipped at {}: face against normal {dot:.2}, {area:.0} mm²",
                    round(*at)
                );
            }
            for (turn, at) in &fins {
                println!("  fin {turn:.0} degrees at {}", round(*at));
            }
            if !flipped.is_empty() || !fins.is_empty() {
                failures.push(format!(
                    "{name}: {} flipped ({:.0} mm²), {} fins",
                    flipped.len(),
                    flipped_area * 1e6,
                    fins.len()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// §3 "Construction": no single-layer edge. Every open edge is the raw
/// edge of a turned hem, band or facing, inside the shell it was turned
/// from, `HEM_THICK` to `HEM_REACH` from it, `HEM_RINGS` rings or more
/// along the cloth from the fold. (The pockets and the drawcord are
/// closed.)
#[test]
fn no_edge_of_the_joggers_is_a_single_layer() {
    let library = library();
    let model = model(&library);
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
    assert!(!open.is_empty(), "the raw edges of the facings");
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
        "{} open corners, {} not inside a shell",
        open.len(),
        bad.len()
    );
    assert!(bad.is_empty(), "{:?}", &bad[..bad.len().min(10)]);
}
