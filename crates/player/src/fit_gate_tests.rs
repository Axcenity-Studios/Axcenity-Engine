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
//! poses and a walk frame. Wearing the tee hides the body regions it
//! covers whole, live.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use axcenity_assets::{Assets, Model};
use axcenity_avatar::fit::{ImportRequest, Surface, import_garment};
use axcenity_avatar::{Catalogue, CatalogueBase, CatalogueItem, Description, Library, WornItem};
use axcenity_core::{ClassName, InstanceId, Value, World};
use axcenity_render::glam::{Mat3, Vec3};

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};

/// A base model's always-worn parts: not garments.
const FACE_PARTS: [&str; 6] = ["Eyes", "Teeth", "EyeShell", "Lashes", "Brows", "LipSeal"];
/// The worst a body vertex may be outside a garment that covers it.
const POKE: f32 = 0.001;
const TEE: &str = "imported/test-tee";

/// A library of the realistic base, its underwear and the test tee
/// imported over the sports bra. It is made once per build of the tests
/// and shared by every test process: kept under the target folder
/// (`target/<profile>/fit-gate/<key>`), keyed by this test binary, the
/// tee's file and the shipped files it is made from, and made by one
/// process while the others wait. Older ones are removed.
fn gate_library() -> &'static Path {
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
            import_garment(&ImportRequest {
                source: tee.clone(),
                library: made.to_owned(),
                base: "axcenity/realistic".to_owned(),
                id: TEE.to_owned(),
                label: "Test tee".to_owned(),
                folder: "imported/test-tee".to_owned(),
                source_info: axcenity_avatar::Source {
                    author: "Axcenity Engine contributors".to_owned(),
                    licence: "CC0-1.0".to_owned(),
                    url: None,
                },
                slot: None,
                layer: None,
                margin: 0.002,
                under: vec!["axcenity/realistic-sports-bra".to_owned()],
                about: None,
                finish_pose: Some(|rig, pose| axcenity_anim::twist::drive(rig, pose, &mut [])),
            })
            .expect("the test tee imports");
        });
        cache.join(&key)
    })
}

fn tee_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../avatar/tests/fixtures/test-tee/TestTee.obj")
}

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
fn bodies() -> Vec<Description> {
    benchmarks("realistic-")
}

/// The cast and body benchmarks whose names start with `family`.
fn benchmarks(family: &str) -> Vec<Description> {
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
struct Posed {
    name: String,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
}

fn posed(
    world: &World,
    assets: &Assets,
    animators: &axcenity_anim::Animators,
    part: InstanceId,
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
    };
    for (index, primitive) in model.primitives.iter().enumerate() {
        let start = out.positions.len() as u32;
        let moved = pose.primitive(&model, index, 0);
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
        out.triangles.extend(
            primitive
                .indices
                .chunks_exact(3)
                .map(|t| [t[0] + start, t[1] + start, t[2] + start]),
        );
    }
    out
}

/// The body and its garments, as the stage posed them.
fn worn(assets: &Assets, description: &Description, stance: Stance) -> (Posed, Vec<Posed>) {
    let (world, animators) = stage_world(assets, description, stance, &[]).unwrap();
    let parts = world.all_below(world.workspace(), |class| class == ClassName::MeshPart);
    let body = *parts
        .iter()
        .find(|part| world.get(**part).unwrap().name() == "Body")
        .expect("a body");
    let garments = world
        .get(body)
        .unwrap()
        .children()
        .iter()
        .copied()
        .filter(|child| {
            let name = world
                .get(*child)
                .map(|c| c.name().to_owned())
                .unwrap_or_default();
            !FACE_PARTS.contains(&name.as_str()) && !name.starts_with("Hair_")
        })
        // Only skinned meshes the Animator posed (not an accessory's
        // Attachment or a part without a model).
        .filter(|part| {
            animators.poses().contains_key(part)
                && matches!(
                    world.get(*part).and_then(|p| p.property("MeshId")),
                    Some(Value::String(id)) if !id.is_empty()
                )
        })
        .map(|part| posed(&world, assets, &animators, part))
        .collect();
    (posed(&world, assets, &animators, body), garments)
}

/// Corners at the same place are one; a corner on an edge with one
/// triangle is on a hem. Returns which corners are on (or next to) one.
fn hem_corners(mesh: &Posed) -> Vec<bool> {
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
fn covered(under: &Posed, garment: &Posed, reach: f32) -> Vec<bool> {
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
    under
        .positions
        .iter()
        .zip(&under.normals)
        .map(|(p, n)| {
            surface
                .closest(*p, reach)
                .is_some_and(|hit| hit.is_over(*p) && n.dot(hit.normal) > 0.5)
        })
        .collect()
}

/// How many of `under`'s covered corners (`covered`, from
/// [`covered`] at rest) are more than `POKE` outside `garment` in this
/// pose, and the worst. A corner counts only where it is drawn.
fn poke_through(label: &str, under: &Posed, garment: &Posed, covered: &[bool]) -> (usize, f32) {
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
    let mut count = 0;
    let mut worst = f32::NEG_INFINITY;
    for (k, (p, n)) in under.positions.iter().zip(&under.normals).enumerate() {
        if !drawn[k] || !covered.get(k).copied().unwrap_or(false) {
            continue;
        }
        let Some(hit) = surface.closest(*p, 0.05) else {
            continue;
        };
        // The garment's inside (a shoe's lining, a jacket's inner shell)
        // faces the skin: being past it is being inside the garment.
        if n.dot(hit.normal) <= 0.0 {
            continue;
        }
        let out = hit.signed(*p);
        worst = worst.max(out);
        if out > POKE {
            count += 1;
            if std::env::var_os("AXCENITY_FIT_GATE_DEBUG").is_some() {
                println!(
                    "  {label} {} out {:.1} mm at {p}; cloth {} normal {}",
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

/// The cover each garment of `garments` gives at rest: (garment name,
/// covered body corners), and the bra's corners the tee covers.
struct Cover {
    body: Vec<(String, Vec<bool>)>,
    bra_under_tee: Option<Vec<bool>>,
}

fn cover_at_rest(body: &Posed, garments: &[Posed]) -> Cover {
    let find = |name: &str| garments.iter().find(|g| g.name == name);
    Cover {
        body: garments
            .iter()
            .map(|g| (g.name.clone(), covered(body, g, 0.04)))
            .collect(),
        bra_under_tee: match (find("TestTee"), find("Underwear_Bra")) {
            (Some(tee), Some(bra)) => Some(covered(bra, tee, 0.02)),
            _ => None,
        },
    }
}

/// One garment (or a lower layer through an upper one) on one body in one
/// pose with body vertices more than `POKE` out.
struct Failure {
    garment: String,
    pose: String,
    body: String,
    count: usize,
    worst: f32,
}

/// A pose's name in the gate's records.
fn pose_name(stance: Stance) -> String {
    match stance {
        Stance::Rest => "rest".to_owned(),
        Stance::Idle => "idle".to_owned(),
        Stance::Clip { name, frames } => format!("{name}@{frames}"),
        Stance::Held(pose) => format!("{pose:?}"),
    }
}

/// The name a lower layer coming through the tee is recorded under.
const BRA_THROUGH_TEE: &str = "Underwear_Bra through TestTee";

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
fn measure(
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
    // The tee over the sports bra: the lower layer never comes through
    // the upper one.
    if let (Some(tee), Some(bra), Some(covered)) = (
        garments.iter().find(|g| g.name == "TestTee"),
        garments.iter().find(|g| g.name == "Underwear_Bra"),
        &cover.bra_under_tee,
    ) {
        let label = format!("{name} {}", pose_name(stance));
        let (count, worst) = poke_through(&label, bra, tee, covered);
        checked += 1;
        fail(BRA_THROUGH_TEE, count, worst);
    }
    checked
}

/// A recorded defect of a draft garment the gate still finds, per garment
/// and pose: at most `cases` bodies failing, none worse than `worst`
/// metres. A garment and pose not listed must have none. Lower these as
/// items improve; never raise them to pass one, and never list a
/// `production` item.
struct Known {
    garment: &'static str,
    pose: &'static str,
    cases: usize,
    worst: f32,
}

/// Fails on any garment and pose with more failing bodies, or a worse
/// one, than `known` records (none, when it records nothing).
fn check(label: &str, failures: &[Failure], known: &[Known]) {
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
        let cover = cover_at_rest(&body, &garments);
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

// What the realistic gate still finds (review fixes 2, 2026-11-17), per
// garment and pose. Every item here is a draft or an open defect in
// `docs/development/character-studio/refinement/wave3/co9-binding.md`
// (Review fixes 2), not accepted behaviour. The test tee was remade in
// review fixes 2 (a closed underarm and sleeves past the armpit, a crew
// neck over the bra's straps): it covers more skin than before, so it is
// counted in more places. It fits at rest, in the T-pose and with the
// elbows or knees bent.

/// The boxer briefs on the masculine heavy-soft body: one body vertex
/// under the gusset, 14 mm out (17 mm in the walk), which the build's
/// push-out (`underwear.py`) doesn't reach.
const BOXER_GUSSET: f32 = 0.0143;

fn boxer(pose: &'static str) -> Known {
    Known {
        garment: "Underwear_BoxerBriefs",
        pose,
        cases: 1,
        worst: if pose == "Walk@18" {
            0.0172
        } else {
            BOXER_GUSSET
        },
    }
}

#[test]
fn garments_cover_the_cast_at_rest() {
    gate(Stance::Rest, &[boxer("rest")]);
}

/// The walk: the front of the armpit, where the arm swings back and the
/// skin there folds out of the tee's armpit (on every body, up to 31 mm),
/// and the bra's side through the tee there. The posed fit's correctives
/// don't reach it: no body corrective leans on a swing.
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
                cases: 54,
                worst: 0.0314,
            },
            Known {
                garment: BRA_THROUGH_TEE,
                pose: "Walk@18",
                cases: 20,
                worst: 0.0172,
            },
            boxer("Walk@18"),
        ],
    );
}

#[test]
fn garments_cover_the_cast_with_the_arms_out() {
    gate(Stance::Held(HeldPose::TPose), &[boxer("TPose")]);
}

/// Arms raised: the base of the neck rises into the tee's close crew neck
/// (on every body, up to 16 mm), and the bra's straps with it.
#[test]
fn garments_cover_the_cast_with_the_arms_raised() {
    gate(
        Stance::Held(HeldPose::ArmsRaised),
        &[
            Known {
                garment: "TestTee",
                pose: "ArmsRaised",
                cases: 54,
                worst: 0.0161,
            },
            Known {
                garment: BRA_THROUGH_TEE,
                pose: "ArmsRaised",
                cases: 5,
                worst: 0.0037,
            },
            boxer("ArmsRaised"),
        ],
    );
}

#[test]
fn garments_cover_the_cast_with_the_elbows_bent() {
    gate(Stance::Held(HeldPose::ElbowsBent), &[boxer("ElbowsBent")]);
}

#[test]
fn garments_cover_the_cast_with_the_knees_bent() {
    gate(Stance::Held(HeldPose::KneesBent), &[boxer("KneesBent")]);
}

/// Wearing the tee leaves the regions it covers whole out of the body
/// drawn (a model with hidden regions, made live), and taking it off puts
/// them back.
#[test]
fn wearing_the_tee_hides_the_regions_it_covers_and_taking_it_off_shows_them() {
    let stage = Stage::new("fit-gate-hiding", gate_library()).expect("the stage");
    let library = Library::read(gate_library()).0;
    let tee = library.item(TEE).unwrap();
    // The realistic `torso` region runs up the neck, past a crew neck:
    // the tee hides the waist, which it covers whole.
    assert!(tee.hides.contains(&"waist".to_owned()), "{:?}", tee.hides);
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
    assert!(hidden >= 200, "{hidden} triangles hidden");
    // The corners stay the same: only what is drawn changes.
    assert_eq!(bare.positions.len(), dressed.positions.len());
    description.items.retain(|worn| worn.id != TEE);
    let (again, _) = worn(&stage.assets, &description, Stance::Rest);
    assert_eq!(again.triangles.len(), bare.triangles.len());
}

/// The stylized family's own clothes (fitted by their copies of the body's
/// shapes, ADR-0030's way): an outfit on the stylized cast and body
/// extremes, measured with the same rule as the realistic items, at rest
/// and in the T-pose.
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
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        let cover = cover_at_rest(&body, &garments);
        checked += measure(
            &description.name,
            Stance::Rest,
            &body,
            &garments,
            &cover,
            &mut failures,
        );
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Held(HeldPose::TPose));
        checked += measure(
            &description.name,
            Stance::Held(HeldPose::TPose),
            &body,
            &garments,
            &cover,
            &mut failures,
        );
    }
    println!(
        "stylized: {checked} garment checks, {} failing",
        failures.len()
    );
    // The stylized clothes are fitted by ADR-0030's shape copies. What this
    // finds is not seen in close captures of the cast (`co9-binding.md`,
    // Review fixes 1): the sneakers' cases are the foot over the shoe's
    // inner floor, which faces it from below; the tee's are at the top of
    // the back and the shoulders, the cargo pants' under the buttocks.
    // Recorded, per garment and pose, as open defects of drafts.
    let known = [
        ("Shoes_Sneakers", "rest", 16, 0.0339),
        ("Shoes_Sneakers", "TPose", 16, 0.0339),
        ("Shirt_Tee", "rest", 7, 0.0247),
        ("Shirt_Tee", "TPose", 8, 0.0249),
        ("Pants_Cargo", "rest", 7, 0.0339),
        ("Pants_Cargo", "TPose", 7, 0.0339),
        ("Underwear_Briefs", "rest", 3, 0.0176),
        ("Underwear_Briefs", "TPose", 3, 0.0176),
    ]
    .map(|(garment, pose, cases, worst)| Known {
        garment,
        pose,
        cases,
        worst,
    });
    check("stylized", &failures, &known);
}
