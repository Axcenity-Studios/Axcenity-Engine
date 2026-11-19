//! CO9's fit gate (`docs/development/character-studio/asset-quality.md`
//! §3): garments worn by the realistic cast, applied and posed by the
//! player as a game poses them (`character_capture`'s stage), with no
//! body vertex more than 1 mm outside a garment where the garment covers
//! it, and no lower layer through an upper one. The garments: every
//! shipped realistic item (the underwear, each on the frame that wears
//! it) and the imported test tee (`avatar`'s `tests/fixtures/test-tee`,
//! not made from the body) on both frames, over the underwear. The
//! bodies: the cast of both frames and the body benchmarks' composition
//! and proportion extremes. The poses: rest, the body suite's four held
//! poses, the arms raised forward, a walk frame and (wave 5, the walking
//! fit test) the whole stride, every third frame. Wearing the tee hides
//! the body regions it covers whole, live.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use axcenity_assets::{Assets, Model};
use axcenity_avatar::fit::{
    Hit, ImportOptions, ImportRequest, Openings, Surface, Views, holes_seen, import_garment_with,
};
use axcenity_avatar::{Catalogue, CatalogueBase, CatalogueItem, Description, Library, WornItem};
use axcenity_core::{ClassName, InstanceId, Value, World};
use axcenity_render::glam::{Mat3, Vec3};

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};

/// A base model's always-worn parts: not garments.
const FACE_PARTS: [&str; 6] = ["Eyes", "Teeth", "EyeShell", "Lashes", "Brows", "LipSeal"];
/// The worst a body vertex may be outside a garment that covers it.
pub(super) const POKE: f32 = 0.001;
pub(super) const TEE: &str = "imported/test-tee";

/// A library of the realistic base, its underwear and the test tee
/// imported over the sports bra. It is made once per build of the tests
/// and shared by every test process: kept under the target folder
/// (`target/<profile>/fit-gate/<key>`), keyed by this test binary, the
/// tee's file and the shipped files it is made from, and made by one
/// process while the others wait. Older ones are removed.
pub(super) fn gate_library() -> &'static Path {
    static LIBRARY: OnceLock<PathBuf> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let shipped = repository_library();
        let (library, _, problems) = Library::read(&shipped);
        assert!(problems.is_empty(), "{problems:?}");
        let base = library.base("axcenity/realistic").unwrap();
        let base_path = library.manifest_path(&base.id).unwrap();
        let mut files = vec![
            base_path.to_owned(),
            library.path_of(&base.id, &base.model.file).unwrap(),
            library
                .path_of(&base.id, base.model.regions_map.as_deref().unwrap())
                .unwrap(),
        ];
        let mut catalogue = Catalogue {
            format_version: 1,
            bases: vec![CatalogueBase::of(base_path, base)],
            items: Vec::new(),
            presets: Vec::new(),
        };
        for (id, item) in &library.items {
            if item.bases.contains(&base.id) {
                let path = library.manifest_path(id).unwrap();
                files.push(path.to_owned());
                catalogue.items.push(CatalogueItem::of(path, item));
            }
        }
        files.sort();
        let tee = tee_source();
        let key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            let exe = std::env::current_exe().unwrap();
            let made = std::fs::metadata(&exe).unwrap();
            (made.len(), made.modified().ok()).hash(&mut hasher);
            std::fs::read(&tee).unwrap().hash(&mut hasher);
            for mask in TEE_MASKS {
                std::fs::read(tee_fixtures().join(mask))
                    .ok()
                    .hash(&mut hasher);
            }
            for file in &files {
                file.hash(&mut hasher);
                std::fs::read(shipped.join(file)).unwrap().hash(&mut hasher);
            }
            format!("{:016x}", hasher.finish())
        };
        let cache = gate_cache();
        shared_once(&cache, &key, |made| {
            for file in &files {
                let target = made.join(file);
                std::fs::create_dir_all(target.parent().unwrap()).unwrap();
                std::fs::copy(shipped.join(file), &target).unwrap();
            }
            std::fs::write(made.join("catalogue.json"), catalogue.to_json()).unwrap();
            // Its masks trimmed to the runtime (wave 5, `hide_trim_tests`),
            // when they are kept beside it, in place of the import's own
            // (which take minutes to make: the import skips them).
            let kept = TEE_MASKS.iter().any(|m| tee_fixtures().join(m).is_file());
            let imported = import_garment_with(
                &ImportRequest {
                    source: tee.clone(),
                    library: made.to_owned(),
                    base: "axcenity/realistic".to_owned(),
                    id: TEE.to_owned(),
                    label: "Test tee".to_owned(),
                    folder: "imported/test-tee".to_owned(),
                    source_info: axcenity_avatar::Source {
                        author: "Axcenity Engine contributors".to_owned(),
                        licence: "CC0-1.0".to_owned(),
                        ..axcenity_avatar::Source::default()
                    },
                    slot: None,
                    layer: None,
                    margin: 0.002,
                    under: vec!["axcenity/realistic-sports-bra".to_owned()],
                    about: None,
                    finish_pose: Some(|rig, pose| axcenity_anim::twist::drive(rig, pose, &mut [])),
                },
                ImportOptions {
                    skip_hiding: kept,
                    ..ImportOptions::default()
                },
            )
            .expect("the test tee imports");
            if kept {
                let path = made.join(&imported.manifest_path);
                let mut manifest = imported.manifest.clone();
                for (mask, under) in TEE_MASKS
                    .iter()
                    .zip([None, Some("axcenity/realistic-sports-bra")])
                {
                    if tee_fixtures().join(mask).is_file() {
                        std::fs::copy(tee_fixtures().join(mask), made.join(TEE).join(mask))
                            .unwrap();
                        manifest.hide_maps.push(axcenity_avatar::HideMap {
                            under: under.map(str::to_owned),
                            map: (*mask).to_owned(),
                        });
                    }
                }
                std::fs::write(path, manifest.to_json()).unwrap();
            }
            // What it hides, and what it covers at rest but leaves drawn
            // whole.
            println!(
                "the test tee hides {:?} and {:?}; not whole {:?}",
                imported.manifest.hides,
                imported.hiding,
                imported
                    .unhidden
                    .iter()
                    .map(|(id, out)| format!("{id} ({:.1} mm out)", out * 1000.0))
                    .collect::<Vec<_>>()
            );
        });
        cache.join(&key)
    })
}

fn tee_source() -> PathBuf {
    tee_fixtures().join("TestTee.obj")
}

/// The test tee's folder (`avatar`'s `tests/fixtures/test-tee`).
pub(super) fn tee_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../avatar/tests/fixtures/test-tee")
}

/// The test tee's masks (its import's, trimmed to the runtime by
/// `hide_trim_tests` and kept beside it).
pub(super) const TEE_MASKS: [&str; 2] =
    ["TestTee.hide.png", "TestTee.hide.realistic-sports-bra.png"];

/// Where the gate's libraries are kept: beside this test binary's
/// profile folder (`target/debug/fit-gate`), not the system's temporary
/// folder, which nothing cleans.
fn gate_cache() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    // target/<profile>/deps/<binary>
    let profile = exe.parent().and_then(Path::parent).unwrap();
    profile.join("fit-gate")
}

/// Makes `cache/key` once with `make` (given an empty folder to fill),
/// shared by every process that asks: the first takes a lock file and
/// makes it in a folder of its own, then renames that into place; the
/// others wait for it. A lock older than ten minutes is a crashed
/// process's and is taken over. Folders of other keys more than an hour
/// old (earlier builds) are removed.
fn shared_once(cache: &Path, key: &str, make: impl FnOnce(&Path)) {
    use std::time::{Duration, SystemTime};
    let age = |path: &Path| {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| SystemTime::now().duration_since(m).ok())
            .unwrap_or_default()
    };
    std::fs::create_dir_all(cache).unwrap();
    let done = cache.join(key);
    let lock = cache.join(format!("{key}.lock"));
    let started = std::time::Instant::now();
    let mut make = Some(make);
    while !done.is_dir() {
        let taken = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
            .is_ok();
        if taken {
            let making = cache.join(format!("{key}.{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&making);
            std::fs::create_dir_all(&making).unwrap();
            (make.take().expect("made once"))(&making);
            if std::fs::rename(&making, &done).is_err() {
                // Another process (past a stale lock) finished first.
                let _ = std::fs::remove_dir_all(&making);
            }
            let _ = std::fs::remove_file(&lock);
            break;
        }
        if age(&lock) > Duration::from_secs(600) {
            let _ = std::fs::remove_file(&lock);
            continue;
        }
        assert!(
            started.elapsed() < Duration::from_secs(600),
            "waited ten minutes for {}",
            done.display()
        );
        std::thread::sleep(Duration::from_millis(250));
    }
    for entry in std::fs::read_dir(cache).into_iter().flatten().flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(key) || age(&path) < Duration::from_secs(3600) {
            continue;
        }
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(path);
        } else {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// The benchmark descriptions of the gate: the cast of both frames and
/// the body extremes (`assets/characters/benchmarks`).
pub(super) fn bodies() -> Vec<Description> {
    benchmarks("realistic-")
}

/// The cast and body benchmarks whose names start with `family`.
pub(super) fn benchmarks(family: &str) -> Vec<Description> {
    let folder = repository_library().join("benchmarks");
    let mut names: Vec<String> = std::fs::read_dir(&folder)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.starts_with(family)
                && (name.contains("-cast-") || name.contains("-body-"))
                && name.ends_with(".character.json")
        })
        .collect();
    names.sort();
    names
        .iter()
        .map(|name| {
            Description::from_json(&std::fs::read_to_string(folder.join(name)).unwrap()).unwrap()
        })
        .collect()
}

/// A posed mesh in its file's space: corners, their normals, and the
/// triangles drawn.
#[derive(Clone)]
pub(super) struct Posed {
    pub name: String,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    /// The triangles its model leaves out (hidden regions, or what the
    /// items over it hide: wave 5), over the same corners.
    pub hidden: Vec<[u32; 3]>,
}

fn posed(
    world: &World,
    assets: &Assets,
    animators: &axcenity_anim::Animators,
    part: InstanceId,
) -> Posed {
    posed_at(world, assets, animators, part, 0)
}

/// `part` as `animators` last posed it, at its level of detail `level`
/// (0 the full model; a garment's simpler copies are checked for
/// clearance too: `wardrobe_tests`).
pub(super) fn posed_at(
    world: &World,
    assets: &Assets,
    animators: &axcenity_anim::Animators,
    part: InstanceId,
    level: usize,
) -> Posed {
    let mesh_id = match world.get(part).and_then(|p| p.property("MeshId")) {
        Some(Value::String(id)) => id.clone(),
        _ => panic!("a MeshPart without a MeshId"),
    };
    let model: std::sync::Arc<Model> = assets.model_blocking(&mesh_id).unwrap();
    let pose = animators.poses()[&part].clone();
    let fit = model.rig.as_ref().unwrap().fit;
    let unfit = fit.inverse();
    let turn = Mat3::from_mat4(fit).transpose();
    let mut out = Posed {
        name: world.get(part).unwrap().name().to_owned(),
        positions: Vec::new(),
        normals: Vec::new(),
        triangles: Vec::new(),
        hidden: Vec::new(),
    };
    // The model it is made from with nothing left out, when it leaves
    // some triangles out.
    let whole: Option<std::sync::Arc<Model>> = match axcenity_assets::split_hidden(&mesh_id) {
        (base, Some(_)) => assets.model_blocking(base),
        _ => None,
    };
    for (index, primitive) in model.primitives.iter().enumerate() {
        let start = out.positions.len() as u32;
        let moved = pose.primitive(&model, index, level);
        out.positions.extend(
            moved
                .positions
                .iter()
                .map(|p| unfit.transform_point3(Vec3::from(*p))),
        );
        out.normals.extend(
            moved
                .normals
                .iter()
                .map(|n| (turn * Vec3::from(*n)).normalize_or_zero()),
        );
        let indices = if level == 0 {
            &primitive.indices
        } else {
            &primitive.lods[level - 1].indices
        };
        out.triangles.extend(
            indices
                .chunks_exact(3)
                .map(|t| [t[0] + start, t[1] + start, t[2] + start]),
        );
        if let Some(whole) = whole.as_ref().and_then(|w| w.primitives.get(index)) {
            let all = if level == 0 {
                &whole.indices
            } else {
                &whole.lods[level - 1].indices
            };
            let drawn: std::collections::HashSet<[u32; 3]> = indices
                .chunks_exact(3)
                .map(|t| [t[0], t[1], t[2]])
                .collect();
            out.hidden.extend(
                all.chunks_exact(3)
                    .map(|t| [t[0], t[1], t[2]])
                    .filter(|t| !drawn.contains(t))
                    .map(|t| t.map(|v| v + start)),
            );
        }
    }
    out
}

/// The body and its garments, as the stage posed them.
pub(super) fn worn(
    assets: &Assets,
    description: &Description,
    stance: Stance,
) -> (Posed, Vec<Posed>) {
    let (world, animators) = stage_world(assets, description, stance, &[]).unwrap();
    let (body, garments, _) = worn_in(&world, assets, &animators);
    (body, garments)
}

/// The body, its garments and its hair (`Hair_*` parts) as `animators`
/// last posed them in `world`.
pub(super) fn worn_in(
    world: &World,
    assets: &Assets,
    animators: &axcenity_anim::Animators,
) -> (Posed, Vec<Posed>, Vec<Posed>) {
    let parts = world.all_below(world.workspace(), |class| class == ClassName::MeshPart);
    let body = *parts
        .iter()
        .find(|part| world.get(**part).unwrap().name() == "Body")
        .expect("a body");
    let name_of = |part: InstanceId| {
        world
            .get(part)
            .map(|c| c.name().to_owned())
            .unwrap_or_default()
    };
    // Only skinned meshes the Animator posed (not an accessory's
    // Attachment or a part without a model).
    let posed_parts: Vec<InstanceId> = world
        .get(body)
        .unwrap()
        .children()
        .iter()
        .copied()
        .filter(|part| {
            animators.poses().contains_key(part)
                && matches!(
                    world.get(*part).and_then(|p| p.property("MeshId")),
                    Some(Value::String(id)) if !id.is_empty()
                )
        })
        .collect();
    let garments = posed_parts
        .iter()
        .copied()
        .filter(|part| {
            let name = name_of(*part);
            !FACE_PARTS.contains(&name.as_str()) && !name.starts_with("Hair_")
        })
        .map(|part| posed(world, assets, animators, part))
        .collect();
    let hair = posed_parts
        .iter()
        .copied()
        .filter(|part| name_of(*part).starts_with("Hair_"))
        .map(|part| posed(world, assets, animators, part))
        .collect();
    (posed(world, assets, animators, body), garments, hair)
}

/// Corners at the same place are one; a corner on an edge with one
/// triangle is on a hem. Returns which corners are on (or next to) one.
pub(super) fn hem_corners(mesh: &Posed) -> Vec<bool> {
    use std::collections::HashMap;
    let mut first: HashMap<[i64; 3], u32> = HashMap::new();
    let weld: Vec<u32> = mesh
        .positions
        .iter()
        .enumerate()
        .map(|(k, p)| {
            *first
                .entry((*p * 1e5).round().as_i64vec3().to_array())
                .or_insert(k as u32)
        })
        .collect();
    let mut uses: HashMap<(u32, u32), u32> = HashMap::new();
    for t in &mesh.triangles {
        let t = t.map(|v| weld[v as usize]);
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    let mut on = vec![false; mesh.positions.len()];
    let mut hem_welds = std::collections::HashSet::new();
    for ((a, b), n) in uses {
        if n == 1 {
            hem_welds.insert(a);
            hem_welds.insert(b);
        }
    }
    for (k, w) in weld.iter().enumerate() {
        on[k] = hem_welds.contains(w);
    }
    on
}

/// Which corners of `under` (a body or a lower layer) `garment` covers,
/// both at rest: those whose closest point on the garment, within
/// `reach`, is away from its hems (two rings of triangles in from an
/// open edge, which on the underwear is the rim folded under the skin),
/// lies along the garment's normal from them and faces the way they do.
/// Past a hem, or beside a garment (an arm by a shirt's side), is not
/// covered.
pub(super) fn covered(under: &Posed, garment: &Posed, reach: f32) -> Vec<bool> {
    let mut hem = hem_corners(garment);
    for _ in 0..2 {
        let mut grown = hem.clone();
        for t in &garment.triangles {
            if t.iter().any(|v| hem[*v as usize]) {
                for v in t {
                    grown[*v as usize] = true;
                }
            }
        }
        hem = grown;
    }
    let inner: Vec<[u32; 3]> = garment
        .triangles
        .iter()
        .copied()
        .filter(|t| t.iter().all(|v| !hem[*v as usize]))
        .collect();
    let surface = Surface::new(garment.positions.clone(), garment.normals.clone(), inner);
    let whole = Surface::new(
        garment.positions.clone(),
        garment.normals.clone(),
        garment.triangles.clone(),
    );
    under
        .positions
        .iter()
        .zip(&under.normals)
        .map(|(p, n)| {
            surface.closest(*p, reach).is_some_and(|hit| {
                hit.is_over(*p)
                    && n.dot(hit.normal) > 0.5
                    // Skin nearer a hem (or the rim folded under it) than
                    // the cloth over it by more than `BESIDE_HEM` is
                    // beside the hem, not under the cloth (review fixes
                    // 2: the boxer briefs' gusset edge).
                    && whole
                        .closest(*p, reach)
                        .is_none_or(|near| hit.distance <= near.distance + BESIDE_HEM)
            })
        })
        .collect()
}

/// How much farther than the garment's nearest point (a hem's, or its
/// rim's) the cloth over a corner may be for the corner to be under it.
const BESIDE_HEM: f32 = 0.01;

/// How many of `under`'s covered corners (`covered`, from
/// [`covered`] at rest) are more than `POKE` outside `garment` in this
/// pose, and the worst. A corner counts only where it is drawn. Out is
/// past the nearest cloth along its normal, unless the garment encloses
/// the corner ([`enclosed`]).
pub(super) fn poke_through(
    label: &str,
    under: &Posed,
    garment: &Posed,
    covered: &[bool],
) -> (usize, f32) {
    poke_through_counting(label, under, garment, covered, count_hidden())
}

/// [`poke_through`], counting hidden corners too when `hidden_too`.
fn poke_through_counting(
    label: &str,
    under: &Posed,
    garment: &Posed,
    covered: &[bool],
    hidden_too: bool,
) -> (usize, f32) {
    let surface = Surface::new(
        garment.positions.clone(),
        garment.normals.clone(),
        garment.triangles.clone(),
    );
    let mut drawn = vec![false; under.positions.len()];
    for t in &under.triangles {
        for v in t {
            drawn[*v as usize] = true;
        }
    }
    // Hidden skin (or a hidden part of a lower layer) can't be seen, so it
    // isn't counted (wave 5: hiding under clothing; the hole check holds
    // it). For comparison with the count before, which counted it,
    // `AXCENITY_FIT_GATE_COUNT_HIDDEN=1`: never for a record.
    if hidden_too {
        for t in &under.hidden {
            for v in t {
                drawn[*v as usize] = true;
            }
        }
    }
    // For comparing with the measure before wave 5 (the closest point
    // alone), `AXCENITY_FIT_GATE_CLOSEST_ONLY=1`: never for a record.
    let closest_only = std::env::var_os("AXCENITY_FIT_GATE_CLOSEST_ONLY").is_some();

    let mut count = 0;
    let mut worst = f32::NEG_INFINITY;
    for (k, (p, n)) in under.positions.iter().zip(&under.normals).enumerate() {
        if !drawn[k] || !covered.get(k).copied().unwrap_or(false) {
            continue;
        }
        let Some(hit) = surface.closest(*p, POKE_REACH) else {
            continue;
        };
        // The garment's inside (a shoe's lining, a jacket's inner shell)
        // faces the skin: being past it is being inside the garment.
        if n.dot(hit.normal) <= 0.0 {
            continue;
        }
        let nearest = hit.signed(*p);
        let out = if nearest > POKE && !closest_only {
            enclosed(&surface, *p, *n).unwrap_or(nearest)
        } else {
            nearest
        };
        worst = worst.max(out);
        if out > POKE {
            count += 1;
            if std::env::var_os("AXCENITY_FIT_GATE_DEBUG").is_some() {
                // Where the way out along the normal first meets the
                // garment, and from which side (`enclosed`).
                let way_out = surface.cast(*p, *n, POKE_REACH).map_or_else(
                    || "nothing".to_owned(),
                    |ray| {
                        // "flipped": the smooth normal alone would say
                        // inside, but the posed triangle faces the skin.
                        let side = if from_inside(&surface, &ray, *n) {
                            "inside"
                        } else if ray.normal.dot(*n) > 0.0 {
                            "outside, flipped"
                        } else {
                            "outside"
                        };
                        format!("{:.1} mm, from {side}", ray.distance * 1000.0)
                    },
                );
                println!(
                    "  {label} {} through {} out {:.1} mm at {p} (corner {k}); nearest cloth {} normal {}; way out meets {way_out}",
                    under.name,
                    garment.name,
                    out * 1000.0,
                    hit.point,
                    hit.normal,
                );
            }
        }
    }
    (count, worst)
}

/// How far from a corner the gate looks for the garment over it.
const POKE_REACH: f32 = 0.05;

/// Whether poke-through counts hidden skin too (the count before wave 5's
/// hiding under clothing), for comparison: `AXCENITY_FIT_GATE_COUNT_HIDDEN`.
fn count_hidden() -> bool {
    std::env::var_os("AXCENITY_FIT_GATE_COUNT_HIDDEN").is_some()
}

/// Where the way out from a corner at `p` along its normal `n` first meets
/// `garment` (within [`POKE_REACH`]) from the garment's inside, the corner
/// is enclosed there: how far it is out along that cloth's normal
/// (negative: inside). `None` where the way out meets the garment from
/// outside first, or nothing.
///
/// The closest point alone can't tell this where a loose garment is
/// nearer some other part of itself than the cloth over a corner (wave
/// 5, `refinement/wave5/tee-walk.md`): a sleeve swung beside the chest, 23
/// mm from the chest's skin with its outside towards it, while the panel
/// over the chest is 25 mm out along the skin's normal; or the torso's
/// panel 26 mm beside the arm inside its sleeve, whose cloth is 30 mm off.
/// The skin there is inside the garment and nothing of it shows; the
/// closest point took it as 23 and 26 mm through. Skin truly through the
/// cloth over it is outside that cloth, so its way out meets nothing or
/// another fold from outside, and it keeps the closest point's measure.
/// (The same test the gate's holes take as "shut in", `outside_cloth`,
/// there within 15 cm; here within the 5 cm the closest point is.)
///
/// "From inside" takes both the cloth's smooth normal and its posed
/// triangle's own facing (review fixes 2). The smooth normal is the
/// corners' skinned rest normals mixed; where a fold flips a triangle
/// (the armpit's, in the walk) it still points out while the triangle
/// faces the skin, and the renderer culls that back face: the skin shows.
fn enclosed(garment: &Surface, p: Vec3, n: Vec3) -> Option<f32> {
    let hit = garment.cast(p, n, POKE_REACH)?;
    from_inside(garment, &hit, n).then(|| hit.signed(p))
}

/// Whether a ray along `n` meets `garment` at `hit` from its inside:
/// both the smooth normal there and the hit triangle's posed face normal
/// point along `n`.
fn from_inside(garment: &Surface, hit: &Hit, n: Vec3) -> bool {
    let [a, b, c] = garment.triangles[hit.triangle as usize].map(|v| garment.positions[v as usize]);
    hit.normal.dot(n) > 0.0 && (b - a).cross(c - a).dot(n) > 0.0
}

/// The body triangles the worn items hide (review fixes 2): those `bare`
/// (the body with nothing worn) draws and `dressed` doesn't. Both are the
/// same body at rest, so they share their corners.
pub(super) fn hidden_triangles(bare: &Posed, dressed: &Posed) -> Vec<[u32; 3]> {
    let drawn: std::collections::HashSet<[u32; 3]> = dressed.triangles.iter().copied().collect();
    bare.triangles
        .iter()
        .copied()
        .filter(|t| !drawn.contains(t))
        .collect()
}

/// Holes (wave 5, hiding under clothing): how many places of the body's
/// `hidden` triangles (left out of the drawing by the worn items,
/// [`hidden_triangles`]), and of the triangles each garment's own model
/// leaves out (a lower layer the items over it hide), show in this pose:
/// seen from any view direction round the character, up and down, and up
/// and down each garment's openings, their place isn't behind cloth or
/// drawn skin facing the viewer (`avatar::fit::holes_seen`: the
/// background, or the inside of a garment, would show where skin should
/// be). Places are each hidden triangle's corners and middle. The worst
/// is the farthest such place from the nearest cloth. Hidden skin isn't
/// counted by [`poke_through`], so this is what keeps hiding honest.
pub(super) fn holes(
    label: &str,
    body: &Posed,
    hidden: &[[u32; 3]],
    garments: &[Posed],
) -> (usize, f32) {
    let seen = holes_found(body, hidden, garments, false);
    let mut worst = f32::NEG_INFINITY;
    for hole in &seen {
        worst = worst.max(hole.depth);
        if std::env::var_os("AXCENITY_FIT_GATE_DEBUG").is_some() {
            println!(
                "  {label} hole: hidden {} at {} seen from {} ({:.1} mm from the cloth)",
                hole.mesh,
                hole.seen.at,
                hole.seen.toward,
                hole.depth * 1000.0
            );
        }
    }
    (seen.len(), worst)
}

/// A hidden place that shows ([`holes_found`]): the mesh it is on (the
/// body or a garment, by name), where and how it shows, the hidden
/// triangles (of that mesh's) whose place it is (a corner's every hidden
/// triangle, or the one whose middle it is), and how far it is from the
/// nearest cloth.
pub(super) struct FoundHole {
    pub mesh: String,
    pub seen: axcenity_avatar::fit::SeenHole,
    pub triangles: Vec<[u32; 3]>,
    pub depth: f32,
}

/// The hidden places of [`holes`] that show, each with its triangles;
/// with `fine`, from more views than the gate's (`Views::fine`).
pub(super) fn holes_found(
    body: &Posed,
    hidden: &[[u32; 3]],
    garments: &[Posed],
    fine: bool,
) -> Vec<FoundHole> {
    let cloth: Vec<Surface> = garments
        .iter()
        .map(|g| Surface::new(g.positions.clone(), g.normals.clone(), g.triangles.clone()))
        .collect();
    let skin = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    let mut drawn: Vec<&Surface> = cloth.iter().collect();
    drawn.push(&skin);
    let cloth_refs: Vec<&Surface> = cloth.iter().collect();
    let openings: Vec<Openings> = garments
        .iter()
        .map(|g| {
            let mut whole = g.triangles.clone();
            whole.extend_from_slice(&g.hidden);
            Openings::new(&g.positions, &whole)
        })
        .collect();
    let refs: Vec<&Openings> = openings.iter().collect();
    let views = if fine {
        Views::fine(&refs)
    } else {
        Views::new(&refs)
    };
    let mut seen = Vec::new();
    let layers = garments
        .iter()
        .map(|g| (g.name.as_str(), g, g.hidden.as_slice()));
    for (name, posed, hidden) in std::iter::once((body.name.as_str(), body, hidden)).chain(layers) {
        if hidden.is_empty() {
            continue;
        }
        let found = holes_seen(
            hidden,
            &posed.positions,
            &posed.normals,
            &drawn,
            &cloth_refs,
            &views,
        );
        for hole in found {
            let triangles = match (hole.corner, hole.middle) {
                (Some(k), _) => hidden.iter().copied().filter(|t| t.contains(&k)).collect(),
                (None, Some(t)) => vec![hidden[t]],
                _ => Vec::new(),
            };
            let depth = cloth_refs
                .iter()
                .filter_map(|c| c.closest(hole.at, 1.0))
                .map(|hit| hit.distance)
                .fold(f32::INFINITY, f32::min);
            seen.push(FoundHole {
                mesh: name.to_owned(),
                seen: hole,
                triangles,
                depth,
            });
        }
    }
    seen
}

/// What wearing `dressed` hides at rest (wave 5, hiding under clothing):
/// how many body triangles `bare` (the same body with nothing worn over its
/// underwear) draws and `dressed` doesn't, and each worn part's own hidden
/// triangles (a lower layer under a garment), by name.
pub(super) fn hidden_at_rest(
    assets: &Assets,
    bare: &Description,
    dressed: &Description,
) -> (usize, Vec<(String, usize)>) {
    let (body, _) = worn(assets, bare, Stance::Rest);
    let (worn_body, garments) = worn(assets, dressed, Stance::Rest);
    (
        hidden_triangles(&body, &worn_body).len(),
        garments
            .iter()
            .map(|g| (g.name.clone(), g.hidden.len()))
            .collect(),
    )
}

/// The name holes are recorded under in the gate.
pub(super) const HOLES: &str = "hidden skin showing";

/// Layered pairs the gate checks: (the lower layer, the upper one). A
/// lower layer never comes through the upper one where the upper one
/// covers it at rest.
const LAYERS: [(&str, &str); 10] = [
    // The realistic sports bra under the test tee.
    ("Underwear_Bra", "TestTee"),
    // The stylized cargo pants' waistband under the tee's hem, and the
    // sports top under the tee.
    ("Pants_Cargo", "Shirt_Tee"),
    ("Underwear_Top", "Shirt_Tee"),
    // The first realistic wardrobe (`wardrobe_tests`): the joggers'
    // waistband under the tee's hem, the underwear under each.
    ("Pants_Joggers", "Shirt_CrewTee"),
    ("Underwear_Bra", "Shirt_CrewTee"),
    ("Underwear_BoxerBriefs", "Pants_Joggers"),
    ("Underwear_Briefs", "Pants_Joggers"),
    // The first licensed tee (wave 5, `tee_crew_tests`): every piece of
    // the realistic underwear it is worn over.
    ("Underwear_Bra", "Shirt_TeeCrew"),
    ("Underwear_BoxerBriefs", "Shirt_TeeCrew"),
    ("Underwear_Briefs", "Shirt_TeeCrew"),
];

/// The cover each garment of `garments` gives at rest: (garment name,
/// covered body corners), and for each layered pair worn
/// ([`LAYERS`]), the lower layer's corners the upper one covers.
pub(super) struct Cover {
    pub body: Vec<(String, Vec<bool>)>,
    layers: Vec<(&'static str, &'static str, Vec<bool>)>,
    /// The body triangles the worn items hide ([`hidden_triangles`]),
    /// checked for holes in every pose.
    pub hidden: Vec<[u32; 3]>,
    /// Whether [`measure`] checks holes (an outfit that hides much checks
    /// them in tests of their own, `wardrobe_tests`: too slow beside the
    /// rest in a test's two minutes).
    pub holes: bool,
}

/// `bare` is the same body at rest with nothing worn, to find the skin
/// the worn items hide.
pub(super) fn cover_at_rest(body: &Posed, garments: &[Posed], bare: &Posed) -> Cover {
    let find = |name: &str| garments.iter().find(|g| g.name == name);
    Cover {
        holes: true,
        hidden: hidden_triangles(bare, body),
        body: garments
            .iter()
            .filter(|g| !is_footwear(g))
            .map(|g| (g.name.clone(), covered(body, g, 0.04)))
            .collect(),
        layers: LAYERS
            .iter()
            .filter_map(|&(lower, upper)| match (find(lower), find(upper)) {
                (Some(l), Some(u)) => Some((lower, upper, covered(l, u, 0.02))),
                _ => None,
            })
            .collect(),
    }
}

/// `description` with nothing worn: its whole body drawn.
pub(super) fn undressed(description: &Description) -> Description {
    let mut bare = description.clone();
    bare.items.clear();
    bare
}

/// Shoes are measured by the footwear check (`garment_checks`), which
/// knows a closed shoe's inside (the foot over the shoe's inner floor is
/// inside it, not through it): not by the cover rule here.
pub(super) fn is_footwear(garment: &Posed) -> bool {
    garment.name.starts_with("Shoes_")
}

/// One garment (or a lower layer through an upper one) on one body in one
/// pose with body vertices more than `POKE` out.
pub(super) struct Failure {
    pub garment: String,
    pub pose: String,
    pub body: String,
    pub count: usize,
    pub worst: f32,
}

/// A pose's name in the gate's records.
pub(super) fn pose_name(stance: Stance) -> String {
    match stance {
        Stance::Rest => "rest".to_owned(),
        Stance::Idle => "idle".to_owned(),
        Stance::Clip { name, frames } => format!("{name}@{frames}"),
        Stance::Held(pose) => format!("{pose:?}"),
    }
}

/// For looking at a case: with `AXCENITY_FIT_GATE_DUMP` set to a folder,
/// writes the posed body (its covered corners marked) and garments of the
/// bodies whose names contain `AXCENITY_FIT_GATE_DUMP_ONLY` as OBJ files.
fn dump(name: &str, stance: Stance, body: &Posed, garments: &[Posed], cover: &Cover) {
    use std::fmt::Write;
    let Some(folder) = std::env::var_os("AXCENITY_FIT_GATE_DUMP") else {
        return;
    };
    let only = std::env::var("AXCENITY_FIT_GATE_DUMP_ONLY").unwrap_or_default();
    if !name.contains(&only) {
        return;
    }
    let folder = PathBuf::from(folder);
    let _ = std::fs::create_dir_all(&folder);
    let write = |mesh: &Posed, file: String, marks: Option<&[bool]>| {
        let mut text = String::new();
        for (k, p) in mesh.positions.iter().enumerate() {
            // A covered corner is coloured red.
            let red = marks.is_some_and(|m| m.get(k).copied().unwrap_or(false));
            let colour = if red { "1 0 0" } else { "0.8 0.8 0.8" };
            let _ = writeln!(text, "v {} {} {} {colour}", p.x, p.y, p.z);
        }
        for t in &mesh.triangles {
            let _ = writeln!(text, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1);
        }
        let _ = std::fs::write(folder.join(file), text);
    };
    let pose = pose_name(stance);
    let mut any = vec![false; body.positions.len()];
    for (_, covered) in &cover.body {
        for (a, c) in any.iter_mut().zip(covered) {
            *a |= *c;
        }
    }
    write(body, format!("{name}-{pose}-body.obj"), Some(&any));
    for garment in garments {
        write(garment, format!("{name}-{pose}-{}.obj", garment.name), None);
    }
}

/// Measures every garment of a posed character against the cover it
/// gives at rest; returns how many checks ran and what failed.
pub(super) fn measure(
    name: &str,
    stance: Stance,
    body: &Posed,
    garments: &[Posed],
    cover: &Cover,
    failures: &mut Vec<Failure>,
) -> usize {
    dump(name, stance, body, garments, cover);
    let mut checked = 0;
    let mut fail = |garment: &str, count: usize, worst: f32| {
        if count > 0 {
            failures.push(Failure {
                garment: garment.to_owned(),
                pose: pose_name(stance),
                body: name.to_owned(),
                count,
                worst,
            });
        }
    };
    for garment in garments {
        let Some((_, covered)) = cover.body.iter().find(|(n, _)| *n == garment.name) else {
            continue;
        };
        let label = format!("{name} {}", pose_name(stance));
        let (count, worst) = poke_through(&label, body, garment, covered);
        checked += 1;
        fail(&garment.name, count, worst);
    }
    // Layered pairs: the lower layer never comes through the upper one.
    for (lower, upper, covered) in &cover.layers {
        let (Some(l), Some(u)) = (
            garments.iter().find(|g| g.name == *lower),
            garments.iter().find(|g| g.name == *upper),
        ) else {
            continue;
        };
        let label = format!("{name} {}", pose_name(stance));
        let (count, worst) = poke_through(&label, l, u, covered);
        checked += 1;
        fail(&format!("{lower} through {upper}"), count, worst);
    }
    // Holes: skin (or a lower layer) the worn items hide that shows here.
    if cover.holes && (!cover.hidden.is_empty() || garments.iter().any(|g| !g.hidden.is_empty())) {
        let label = format!("{name} {}", pose_name(stance));
        let (count, worst) = holes(&label, body, &cover.hidden, garments);
        checked += 1;
        fail(HOLES, count, worst);
    }
    checked
}

/// A recorded defect of a draft garment the gate still finds, per garment
/// and pose: at most `cases` bodies failing, none worse than `worst`
/// metres. A garment and pose not listed must have none. Lower these as
/// items improve; never raise them to pass one, and never list a
/// `production` item.
pub(super) struct Known {
    pub garment: &'static str,
    pub pose: &'static str,
    pub cases: usize,
    pub worst: f32,
}

/// Fails on any garment and pose with more failing bodies, or a worse
/// one, than `known` records (none, when it records nothing).
pub(super) fn check(label: &str, failures: &[Failure], known: &[Known]) {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<(&str, &str), (usize, f32)> = BTreeMap::new();
    for failure in failures {
        println!(
            "  {} {} {}: {} vertices out, worst {:.2} mm",
            failure.body,
            failure.pose,
            failure.garment,
            failure.count,
            failure.worst * 1000.0
        );
        let group = groups
            .entry((failure.garment.as_str(), failure.pose.as_str()))
            .or_default();
        group.0 += 1;
        group.1 = group.1.max(failure.worst);
    }
    for ((garment, pose), (cases, worst)) in &groups {
        println!(
            "  {label} measured: {garment} {pose}: {cases} bodies, worst {:.2} mm",
            worst * 1000.0
        );
    }
    let mut problems = Vec::new();
    for ((garment, pose), (cases, worst)) in &groups {
        let allowed = known
            .iter()
            .find(|k| k.garment == *garment && k.pose == *pose)
            .map_or((0, 0.0), |k| (k.cases, k.worst));
        if *cases > allowed.0 || *worst > allowed.1 + 1e-4 {
            problems.push(format!(
                "{garment} {pose}: {cases} bodies failing, worst {:.2} mm (recorded {}, {:.2} mm)",
                worst * 1000.0,
                allowed.0,
                allowed.1 * 1000.0
            ));
        }
    }
    // A record no longer earned: its defect is gone (remove it), or is
    // now more than `SLACK` under it or on fewer bodies (lower it).
    // Records follow an item down, and a gate that stopped finding a
    // recorded case is caught.
    for k in known {
        match groups.get(&(k.garment, k.pose)) {
            None => problems.push(format!(
                "{} {}: recorded {} bodies, {:.2} mm, now none: remove the record",
                k.garment,
                k.pose,
                k.cases,
                k.worst * 1000.0
            )),
            Some((_, worst)) if *worst < k.worst - super::garment_checks::SLACK => {
                problems.push(format!(
                    "{} {}: recorded {:.2} mm, now {:.2} mm: lower the record",
                    k.garment,
                    k.pose,
                    k.worst * 1000.0,
                    worst * 1000.0
                ))
            }
            // Fewer bodies than recorded (review fixes 2): lower the count.
            Some((cases, worst)) if *cases < k.cases => problems.push(format!(
                "{} {}: recorded {} bodies, now {cases} ({:.2} mm): lower the record",
                k.garment,
                k.pose,
                k.cases,
                worst * 1000.0
            )),
            Some(_) => {}
        }
    }
    assert!(problems.is_empty(), "{label}: {problems:#?}");
}

/// Runs the gate for `stance` on every body and reports every case.
fn gate(stance: Stance, known: &[Known]) {
    let label = pose_name(stance);
    let stage = Stage::new(&format!("fit-gate-{label}"), gate_library()).expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    let started = std::time::Instant::now();
    for description in bodies() {
        let mut dressed = description.clone();
        dressed.items.push(WornItem::new(TEE));
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        let names: Vec<&str> = garments.iter().map(|g| g.name.as_str()).collect();
        assert!(
            names.contains(&"TestTee"),
            "{}: {names:?}",
            description.name
        );
        let (bare, _) = worn(&stage.assets, &undressed(&description), Stance::Rest);
        let cover = cover_at_rest(&body, &garments, &bare);
        let (body, garments) = if stance == Stance::Rest {
            (body, garments)
        } else {
            worn(&stage.assets, &dressed, stance)
        };
        checked += measure(
            &description.name,
            stance,
            &body,
            &garments,
            &cover,
            &mut failures,
        );
    }
    println!(
        "fit gate {label}: {checked} garment checks in {:.1} s, {} failing",
        started.elapsed().as_secs_f32(),
        failures.len()
    );
    check(&label, &failures, known);
}

/// The walk's frames the gate samples (sixtieths of a second from the
/// clip's start): one whole stride, as `garment_checks::motion` plays it
/// (frames 2 to 44: each arm swings forward and back once), every third
/// frame. Split in three, one test each, so each stays well inside the
/// test runner's two minutes under load.
pub(super) const WALK_FRAMES: [&[u32]; 3] = [
    &[2, 5, 8, 11, 14],
    &[17, 20, 23, 26, 29],
    &[32, 35, 38, 41, 44],
];

/// The pose a walk's failures are recorded under: the worst frame of the
/// stride for each body and garment.
pub(super) const WALK: &str = "Walk";

/// Plays the walk on `dressed` (one character) and measures the frames
/// `frames` against `cover` (its cover at rest), as [`measure`] measures
/// one pose. Each garment's failures on this body are kept once, at the
/// frame with the worst corner (counted under the pose [`WALK`], with the
/// frame in the body's name), so a record's body count is bodies, not
/// frames.
pub(super) fn measure_walk(
    assets: &Assets,
    name: &str,
    dressed: &Description,
    cover: &Cover,
    frames: &[u32],
    failures: &mut Vec<Failure>,
) -> usize {
    use std::collections::BTreeMap;
    let first = frames.iter().copied().min().unwrap_or(2).max(2);
    let last = frames.iter().copied().max().unwrap_or(first);
    let (mut world, mut animators) = stage_world(
        assets,
        dressed,
        Stance::Clip {
            name: "Walk",
            frames: first,
        },
        &[],
    )
    .expect("the walk");
    let mut checked = 0;
    let mut worst: BTreeMap<String, Failure> = BTreeMap::new();
    for frame in first..=last {
        if frames.contains(&frame) {
            let (body, garments, _) = worn_in(&world, assets, &animators);
            let mut here = Vec::new();
            let stance = Stance::Clip {
                name: "Walk",
                frames: frame,
            };
            checked += measure(name, stance, &body, &garments, cover, &mut here);
            for failure in here {
                let keep = worst
                    .get(&failure.garment)
                    .is_none_or(|kept| failure.worst > kept.worst);
                if keep {
                    worst.insert(failure.garment.clone(), failure);
                }
            }
        }
        animators.update(&mut world, assets, 1.0 / 60.0);
    }
    for (_, failure) in worst {
        failures.push(Failure {
            body: format!("{} ({})", failure.body, failure.pose),
            pose: WALK.to_owned(),
            ..failure
        });
    }
    checked
}

/// Runs the gate through the walk's `frames` ([`WALK_FRAMES`]) on every
/// body: the walking fit test (Paul's decision 13).
fn gate_walk(frames: &[u32], known: &[Known]) {
    let label = format!("{WALK} {frames:?}");
    let stage =
        Stage::new(&format!("fit-gate-walk-{}", frames[0]), gate_library()).expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    let started = std::time::Instant::now();
    for description in bodies() {
        let mut dressed = description.clone();
        dressed.items.push(WornItem::new(TEE));
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        let (bare, _) = worn(&stage.assets, &undressed(&description), Stance::Rest);
        let cover = cover_at_rest(&body, &garments, &bare);
        checked += measure_walk(
            &stage.assets,
            &description.name,
            &dressed,
            &cover,
            frames,
            &mut failures,
        );
    }
    println!(
        "fit gate {label}: {checked} garment checks in {:.1} s, {} failing",
        started.elapsed().as_secs_f32(),
        failures.len()
    );
    check(&label, &failures, known);
}

// What the realistic gate still finds (wave 5, `refinement/wave5/
// tee-walk.md`), per garment and pose: open defects of drafts, not
// accepted behaviour. Lower these as items improve; never raise one to
// pass an item.
//
// Wave 5 corrected the measure, not the fit (`enclosed`): skin the
// garment encloses (its way out along its normal meets the cloth from
// inside) was counted out when another part of the same loose garment
// came nearer than the cloth over it. The fit is wave 4's.
//
// Review fixes 2 made "from inside" stricter: the hit triangle's posed
// face must agree with the smooth normal (`from_inside`). With the smooth
// normal alone, skin under a fold flipped in the walk (its back face
// towards the skin, culled when drawn) counted enclosed; the first
// commit's records came down partly by that. They are set back up here
// to what the stricter measure finds. That undoes a wrong reduction; it
// loosens nothing.
//
// - The test tee in the walk: at Walk@18 38 bodies, 10.2 mm (54 bodies,
//   30.2 mm, with the closest point alone; 16 bodies, 9.7 mm, with the
//   smooth normal alone). Through the whole stride (`WALK_FRAMES`) every
//   body in each third, up to 16.5 mm: 2,395 corners, almost all at the
//   armpit, the arm's inner skin and the torso's side under it, where the
//   hanging arm closes on the side and the bridge of cloth between them
//   can't fold into the crease; 1,220 of them under a flipped fold. The
//   bra through it: 6 bodies, 15.2 mm at Walk@18 (smooth normal: 4); 20
//   bodies a third, up to 17.0 mm (smooth normal: 9 to 17).
// - Arms forward (new in wave 5): 44 bodies, 9.4 mm, the same places
//   (smooth normal: 39); the bra 15 bodies, 3.6 mm (smooth normal: 4).
// - Arms raised: the test tee on 1 body (feminine heavy-soft), 2.9 mm:
//   4 corners at the neck's sides under the collar, whose way out meets
//   the shoulder's cloth flipped towards them (wave 4: 2 bodies, 5.2 mm;
//   the smooth normal alone found none, so the first commit removed the
//   record).
//   The bra through it: none (wave 4: 2 bodies, 3.6 mm).
// - Holes (`HOLES`): none. Every pose of the gate checks them.
// - The boxer briefs: none (`BESIDE_HEM`, wave 4).

/// The name a lower layer coming through the tee is recorded under.
const BRA_THROUGH_TEE: &str = "Underwear_Bra through TestTee";

#[test]
fn garments_cover_the_cast_at_rest() {
    gate(Stance::Rest, &[]);
}

/// The walk at its widest swing (the right arm back 43 degrees, the left
/// forward 25).
#[test]
fn garments_cover_the_cast_walking() {
    gate(
        Stance::Clip {
            name: "Walk",
            frames: 18,
        },
        &[
            Known {
                garment: "TestTee",
                pose: "Walk@18",
                cases: 38,
                worst: 0.0103,
            },
            Known {
                garment: BRA_THROUGH_TEE,
                pose: "Walk@18",
                cases: 6,
                worst: 0.0152,
            },
        ],
    );
}

/// The test tee's and the bra's records in a third of the walk.
const fn walk_records(tee: (usize, f32), bra: (usize, f32)) -> [Known; 2] {
    [
        Known {
            garment: "TestTee",
            pose: WALK,
            cases: tee.0,
            worst: tee.1,
        },
        Known {
            garment: BRA_THROUGH_TEE,
            pose: WALK,
            cases: bra.0,
            worst: bra.1,
        },
    ]
}

/// The walking fit test (Paul's decision 13, wave 5): the whole stride,
/// every third frame, on every body. A tee passes it only with no record:
/// the test tee's are open defects (`refinement/wave5/tee-walk.md`).
#[test]
fn garments_cover_the_cast_through_the_walk_first_third() {
    gate_walk(WALK_FRAMES[0], &walk_records((54, 0.0136), (20, 0.0108)));
}

#[test]
fn garments_cover_the_cast_through_the_walk_second_third() {
    gate_walk(WALK_FRAMES[1], &walk_records((54, 0.0166), (20, 0.0171)));
}

#[test]
fn garments_cover_the_cast_through_the_walk_last_third() {
    gate_walk(WALK_FRAMES[2], &walk_records((54, 0.0147), (20, 0.0131)));
}

/// Arm movement (decision 13): both arms raised forward to level.
#[test]
fn garments_cover_the_cast_with_the_arms_forward() {
    gate(
        Stance::Held(HeldPose::ArmsForward),
        &[
            Known {
                garment: "TestTee",
                pose: "ArmsForward",
                cases: 44,
                worst: 0.0094,
            },
            Known {
                garment: BRA_THROUGH_TEE,
                pose: "ArmsForward",
                cases: 15,
                worst: 0.0036,
            },
        ],
    );
}

#[test]
fn garments_cover_the_cast_with_the_arms_out() {
    gate(Stance::Held(HeldPose::TPose), &[]);
}

#[test]
fn garments_cover_the_cast_with_the_arms_raised() {
    gate(
        Stance::Held(HeldPose::ArmsRaised),
        &[Known {
            garment: "TestTee",
            pose: "ArmsRaised",
            cases: 1,
            worst: 0.0029,
        }],
    );
}

#[test]
fn garments_cover_the_cast_with_the_elbows_bent() {
    gate(Stance::Held(HeldPose::ElbowsBent), &[]);
}

#[test]
fn garments_cover_the_cast_with_the_knees_bent() {
    gate(Stance::Held(HeldPose::KneesBent), &[]);
}

/// The ratchet: a garment and pose recorded in `known` passes at its
/// record and fails past it, or when its defect is gone or well under the
/// record, or on fewer bodies than recorded (the record must come down);
/// one not recorded fails on any case.
#[test]
fn the_gate_holds_garments_to_their_records() {
    let failure = |count, worst| Failure {
        garment: "Draft".to_owned(),
        pose: "rest".to_owned(),
        body: "a body".to_owned(),
        count,
        worst,
    };
    let known = [Known {
        garment: "Draft",
        pose: "rest",
        cases: 1,
        worst: 0.005,
    }];
    check("at its record", &[failure(3, 0.0045)], &known);
    let two = [Known {
        cases: 2,
        ..known[0]
    }];
    for (label, failures, known) in [
        ("gone", vec![], &known[..]),
        ("well under", vec![failure(1, 0.003)], &known[..]),
        ("worse", vec![failure(1, 0.006)], &known[..]),
        (
            "more bodies",
            vec![failure(1, 0.0045), failure(1, 0.0045)],
            &known[..],
        ),
        ("not recorded", vec![failure(1, 0.004)], &[]),
        // Review fixes 2: fewer bodies than recorded is a record no
        // longer earned, as a worst well under it is.
        ("fewer bodies", vec![failure(1, 0.0045)], &two[..]),
    ] {
        let caught = std::panic::catch_unwind(|| check(label, &failures, known));
        assert!(caught.is_err(), "{label}");
    }
}

/// A flat square of cloth (`size` across, `cells` a side, at height `z`,
/// facing up), named `name`.
fn sheet(name: &str, size: f32, cells: u32, z: f32) -> Posed {
    let mut out = Posed {
        name: name.to_owned(),
        positions: Vec::new(),
        normals: Vec::new(),
        triangles: Vec::new(),
        hidden: Vec::new(),
    };
    for j in 0..=cells {
        for i in 0..=cells {
            let at = |k: u32| (k as f32 / cells as f32 - 0.5) * size;
            out.positions.push(Vec3::new(at(i), at(j), z));
            out.normals.push(Vec3::Z);
        }
    }
    let row = cells + 1;
    for j in 0..cells {
        for i in 0..cells {
            let a = j * row + i;
            out.triangles.push([a, a + 1, a + row + 1]);
            out.triangles.push([a, a + row + 1, a + row]);
        }
    }
    out
}

/// Skin under the middle of a sheet of cloth is covered; with a hem's
/// rim (a strip that is all hem) much nearer it than the cloth over it,
/// it is beside the hem, not under the cloth (`BESIDE_HEM`).
#[test]
fn skin_nearer_a_hem_than_the_cloth_over_it_is_not_covered() {
    let skin = Posed {
        name: "Body".to_owned(),
        positions: vec![Vec3::new(0.0, 0.0, -0.025)],
        normals: vec![Vec3::Z],
        triangles: Vec::new(),
        hidden: Vec::new(),
    };
    let cloth = sheet("Cloth", 0.2, 10, 0.0);
    assert_eq!(covered(&skin, &cloth, 0.04), [true]);
    // A small rim 5 mm over the skin, 20 mm under the cloth: one cell,
    // all of it hem.
    let mut with_rim = cloth.clone();
    let rim = sheet("Rim", 0.01, 1, -0.02);
    let start = with_rim.positions.len() as u32;
    with_rim.positions.extend(rim.positions);
    with_rim.normals.extend(rim.normals);
    with_rim
        .triangles
        .extend(rim.triangles.iter().map(|t| t.map(|v| v + start)));
    assert_eq!(covered(&skin, &with_rim, 0.04), [false]);
    // The rim 18 mm off, the cloth 25 mm: within `BESIDE_HEM`, covered.
    let mut far_rim = cloth.clone();
    let rim = sheet("Rim", 0.01, 1, -0.007);
    let start = far_rim.positions.len() as u32;
    far_rim.positions.extend(rim.positions);
    far_rim.normals.extend(rim.normals);
    far_rim
        .triangles
        .extend(rim.triangles.iter().map(|t| t.map(|v| v + start)));
    assert_eq!(covered(&skin, &far_rim, 0.04), [true]);
}

/// One posed corner (skin, or a lower layer's), drawn.
fn corner(at: Vec3, normal: Vec3) -> Posed {
    Posed {
        name: "Body".to_owned(),
        positions: vec![at],
        normals: vec![normal.normalize()],
        triangles: vec![[0, 0, 0]],
        hidden: Vec::new(),
    }
}

/// `a` and `b` as one garment.
fn joined(name: &str, a: &Posed, b: &Posed) -> Posed {
    let mut out = a.clone();
    out.name = name.to_owned();
    let start = out.positions.len() as u32;
    out.positions.extend(&b.positions);
    out.normals.extend(&b.normals);
    out.triangles
        .extend(b.triangles.iter().map(|t| t.map(|v| v + start)));
    out
}

/// Skin a loose garment encloses is inside it, even where another part of
/// the same garment comes nearer than the cloth over it with its outside
/// towards the skin (a sleeve swung beside the chest, the torso's panel
/// beside the arm inside its sleeve: wave 5, `tee-walk.md`); skin through
/// the cloth over it is out by as much as before, whatever else is near.
#[test]
fn skin_a_loose_garment_encloses_is_inside_and_skin_through_it_is_out() {
    // The panel 25 mm over the skin, facing out (+Z); a sleeve's wall 12
    // mm to the side (the plane x = 12 mm, below the panel), its outside
    // (-X) towards the skin.
    let panel = sheet("Panel", 0.2, 10, 0.025);
    let mut wall = sheet("Wall", 0.1, 4, 0.0);
    for p in &mut wall.positions {
        *p = Vec3::new(0.012, p.y, -0.0175 + p.x * 0.65);
    }
    wall.normals.fill(-Vec3::X);
    let tee = joined("Tee", &panel, &wall);
    // Skin under the panel, its normal leaning away from the wall: the
    // wall is the nearest cloth (12 mm, the skin on its outside), but the
    // skin's way out along its normal meets the panel from inside.
    let under = corner(Vec3::ZERO, Vec3::new(-0.3, 0.0, 0.954));
    let (count, worst) = poke_through("enclosed", &under, &tee, &[true]);
    assert_eq!(count, 0, "enclosed skin counted out by {worst}");
    assert!(worst < 0.0, "{worst}");
    // Skin 5 mm through the panel: out, by 5 mm.
    let through = corner(Vec3::new(0.0, 0.0, 0.030), Vec3::Z);
    let (count, worst) = poke_through("through", &through, &tee, &[true]);
    assert_eq!(count, 1);
    assert!((worst - 0.005).abs() < 1e-4, "{worst}");
    // Through it with another fold of the cloth over it facing back: the
    // way out meets cloth from outside, so it is still out.
    let mut fold = sheet("Fold", 0.2, 10, 0.05);
    fold.normals.fill(-Vec3::Z);
    let folded = joined("Tee", &tee, &fold);
    let (count, worst) = poke_through("folded", &through, &folded, &[true]);
    assert_eq!(count, 1);
    assert!((worst - 0.005).abs() < 1e-4, "{worst}");
    // Review fixes 2: a fold whose corner normals still face out (+Z, the
    // skinned rest normals) but whose posed triangles face the skin (a
    // triangle flipped in the armpit's fold). Seen from the skin it is
    // the cloth's outside, so the way out meets it from outside: still
    // out, by 5 mm.
    let mut flipped = sheet("Flipped", 0.2, 10, 0.05);
    for t in &mut flipped.triangles {
        t.swap(1, 2);
    }
    let flipped = joined("Tee", &tee, &flipped);
    let (count, worst) = poke_through("flipped", &through, &flipped, &[true]);
    assert_eq!(
        count, 1,
        "skin under a flipped fold counted inside by {worst}"
    );
    assert!((worst - 0.005).abs() < 1e-4, "{worst}");
}

/// Wave 5, hiding under clothing: skin a garment hides isn't counted
/// through it (it can't be seen), unless the count before is asked for;
/// hidden skin under cloth facing out shows nowhere, and under cloth
/// flipped towards it (back faces aren't drawn) it shows: a hole.
#[test]
fn hidden_skin_isnt_counted_through_the_cloth_and_shows_only_through_a_hole() {
    // Skin: a 10 cm square, its middle cell's two triangles hidden, its
    // middle corner 5 mm out through the cloth 1 cm over it.
    let mut skin = sheet("Body", 0.1, 2, 0.0);
    skin.positions[4].z = 0.015;
    let middle: Vec<[u32; 3]> = skin
        .triangles
        .iter()
        .copied()
        .filter(|t| t.contains(&4))
        .collect();
    skin.triangles.retain(|t| !t.contains(&4));
    skin.hidden = middle.clone();
    let cloth = sheet("Cloth", 0.3, 6, 0.01);
    let covered = vec![true; skin.positions.len()];
    let (count, _) = poke_through_counting("hidden", &skin, &cloth, &covered, false);
    assert_eq!(count, 0);
    let (count, worst) = poke_through_counting("counted", &skin, &cloth, &covered, true);
    assert_eq!(count, 1);
    assert!((worst - 0.005).abs() < 1e-4, "{worst}");

    // Holes: none under the cloth facing out.
    let (count, _) = holes("cloth", &skin, &middle, std::slice::from_ref(&cloth));
    assert_eq!(count, 0);
    // The cloth flipped towards the skin isn't drawn from above: its
    // places show.
    let mut flipped = cloth.clone();
    for t in &mut flipped.triangles {
        t.swap(1, 2);
    }
    flipped.normals.fill(-Vec3::Z);
    let (count, _) = holes("flipped", &skin, &middle, std::slice::from_ref(&flipped));
    assert!(count > 0);
    // A lower layer's own hidden triangles (the bra under a tee) are
    // checked too: the skin drawn whole, the cloth as a layer whose
    // middle is hidden, under nothing.
    let mut layer = cloth.clone();
    layer.name = "Underwear".to_owned();
    let hidden_layer: Vec<[u32; 3]> = layer.triangles.drain(..4).collect();
    layer.hidden = hidden_layer;
    let whole = sheet("Body", 0.1, 2, 0.0);
    let (count, _) = holes("layer", &whole, &[], std::slice::from_ref(&layer));
    assert!(count > 0);
}

/// Wearing the tee leaves the regions it covers whole, in every fit pose,
/// out of the body drawn (a model with hidden regions, made live), and
/// taking it off puts them back.
#[test]
fn wearing_the_tee_hides_the_skin_it_encloses_and_taking_it_off_shows_it() {
    let stage = Stage::new("fit-gate-hiding", gate_library()).expect("the stage");
    let library = Library::read(gate_library()).0;
    let tee = library.item(TEE).unwrap();
    // The tee covers the torso, the waist and the upper arm under its
    // sleeve whole at rest, but hides no whole region (wave 5: the waist,
    // which the pose-safe rule hid, is seen from below up the hem); it
    // hides what it encloses and can't be seen triangle by triangle, the
    // body's and the sports bra's (`hide_maps`).
    assert!(tee.hides.is_empty(), "{:?}", tee.hides);
    let unders: Vec<Option<&str>> = tee.hide_maps.iter().map(|m| m.under.as_deref()).collect();
    assert_eq!(unders, [None, Some("axcenity/realistic-sports-bra")]);
    let mut description = bodies()
        .into_iter()
        .find(|d| d.name == "realistic-masculine-cast-average")
        .unwrap();
    let (bare, _) = worn(&stage.assets, &description, Stance::Rest);
    description.items.push(WornItem::new(TEE));
    let (dressed, _) = worn(&stage.assets, &description, Stance::Rest);
    let hidden = bare.triangles.len() - dressed.triangles.len();
    println!(
        "the body draws {} triangles bare, {} in the tee ({hidden} hidden)",
        bare.triangles.len(),
        dressed.triangles.len()
    );
    // What its masks hide, trimmed to the runtime (wave 5): 11 triangles.
    // They change only when the masks are made again.
    assert!(hidden >= 11, "{hidden} triangles hidden");
    // The corners stay the same: only what is drawn changes.
    assert_eq!(bare.positions.len(), dressed.positions.len());
    description.items.retain(|worn| worn.id != TEE);
    let (again, _) = worn(&stage.assets, &description, Stance::Rest);
    assert_eq!(again.triangles.len(), bare.triangles.len());
}

/// The walk at its widest stride.
const STYLIZED_WALK: Stance = Stance::Clip {
    name: "Walk",
    frames: 18,
};

/// The stylized family's own clothes (fitted by their copies of the body's
/// shapes, ADR-0030's way): the underwear alone and an outfit over it on
/// the stylized cast, measured with the same rule as the realistic items,
/// at rest, in the walk and in the T-pose.
#[test]
fn stylized_clothes_cover_the_stylized_cast() {
    let stage = Stage::new("fit-gate-stylized", &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    // The cast only: the body extremes take this past the test time limit.
    let cast = benchmarks("stylized-")
        .into_iter()
        .filter(|d| d.name.contains("-cast-"));
    for description in cast {
        let mut dressed = description.clone();
        for id in ["axcenity/tee", "axcenity/cargo-pants", "axcenity/sneakers"] {
            if !dressed.items.iter().any(|worn| worn.id == id) {
                dressed.items.push(WornItem::new(id));
            }
        }
        let (bare, _) = worn(&stage.assets, &undressed(&description), Stance::Rest);
        // The underwear alone as well as under the outfit (review fixes
        // 2): the cargo pants hide the hips and thighs, and their mask
        // shape sinks the skin there, under the briefs too, so dressed the
        // briefs can't show what they do worn alone.
        let underwear = description.clone();
        let outfits = [
            (format!("{} underwear", description.name), underwear),
            (description.name.clone(), dressed),
        ];
        for (name, outfit) in &outfits {
            let (body, garments) = worn(&stage.assets, outfit, Stance::Rest);
            let cover = cover_at_rest(&body, &garments, &bare);
            checked += measure(name, Stance::Rest, &body, &garments, &cover, &mut failures);
            // The T-pose, and the walk at its widest stride, where the
            // tee's hem lies over the cargo pants' waistband (`LAYERS`).
            for stance in [STYLIZED_WALK, Stance::Held(HeldPose::TPose)] {
                let (body, garments) = worn(&stage.assets, outfit, stance);
                checked += measure(name, stance, &body, &garments, &cover, &mut failures);
            }
        }
    }
    println!(
        "stylized: {checked} garment checks, {} failing",
        failures.len()
    );
    // The stylized clothes are fitted by ADR-0030's shape copies, and not
    // refitted in wave 4 (`co9-fit-2.md`, open issues). Recorded per
    // garment and pose, as open defects of drafts, each at or under its
    // wave-3 record: the tee's case is the neck's side under the collar's
    // rolled-in band (the rule takes the band's inner face as covering
    // it), and on the muscular feminine body the top of the shoulder 9 mm
    // through; the cargo pants at the crotch, 2.4 mm (34 mm at wave 3,
    // when the shoes' insides counted). The sports top through the tee
    // (new in wave 4: layered pairs are checked) at the sides of the
    // chest. The shoes are the footwear check's (`garment_checks`).
    let known = [
        ("Shirt_Tee", "rest", 6, 0.0247),
        ("Shirt_Tee", "TPose", 6, 0.0249),
        ("Pants_Cargo", "rest", 7, 0.0024),
        ("Pants_Cargo", "TPose", 7, 0.0024),
        ("Underwear_Top through Shirt_Tee", "rest", 7, 0.0047),
        ("Underwear_Top through Shirt_Tee", "TPose", 8, 0.0082),
        // The walk (new in review fixes 1): the cargo pants at the crotch,
        // the tee as at rest plus the armpit, the sports top at the side
        // of the chest. Lowered in wave 5 by the corrected measure
        // (`enclosed`), not a refit: the cargo pants 15 bodies at 27.2 mm
        // to 7 at 5.3, the tee 16 bodies to 8, the sports top alone (1
        // body, 5.3 mm) to none and through the tee 12 bodies at 2.5 mm to
        // 2 at 1.6. (With the smooth normal alone the cargo pants were 4
        // at 1.0 here and 6 at rest and in the T-pose; tee-walk review
        // fixes 2 set them back to the face-consistent 7, 7 and 7.)
        ("Pants_Cargo", "Walk@18", 7, 0.0053),
        ("Shirt_Tee", "Walk@18", 8, 0.0247),
        ("Underwear_Top through Shirt_Tee", "Walk@18", 2, 0.0017),
        // The underwear alone (new in review fixes 2; dressed, the cargo
        // pants' mask sank the skin under the briefs): the feminine briefs
        // in the walk, the right buttock's skin 1.2 to 1.9 mm through on
        // every feminine body (the patch the full-detail capture shows);
        // the masculine briefs at rest and in the T-pose, one corner by
        // the front of the crotch 28 mm out, where small notches of skin
        // show under the hem.
        ("Underwear_Briefs", "Walk@18", 8, 0.0019),
        ("Underwear_Briefs", "rest", 4, 0.0280),
        ("Underwear_Briefs", "TPose", 4, 0.0280),
    ]
    .map(|(garment, pose, cases, worst)| Known {
        garment,
        pose,
        cases,
        worst,
    });
    check("stylized", &failures, &known);
}
