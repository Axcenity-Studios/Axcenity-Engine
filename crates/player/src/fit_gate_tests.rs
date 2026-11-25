//! The fit gate (`docs/development/character-studio/asset-quality.md`
//! §3): garments worn by the human base's cast, applied and posed by the
//! player as a game poses them (`character_capture`'s stage), with no
//! body vertex more than 1 mm outside a garment where the garment covers
//! it, no lower layer through an upper one, and no skin a garment hides
//! out from under its cloth. The garments: the underwear cut from the
//! body, MakeHuman's two system outfits on the whole cast, and every
//! garment of the repository's wardrobe on its own. The poses: rest, the
//! T-pose and a frame of the walk.

use std::path::PathBuf;

use axcenity_assets::{Assets, Model};
use axcenity_avatar::fit::{Surface, uncovered};
use axcenity_avatar::{Description, WornItem};
use axcenity_core::{ClassName, InstanceId, Value, World};
use axcenity_render::glam::{Mat3, Vec3};

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};

/// A base model's always-worn parts: not garments.
const FACE_PARTS: [&str; 7] = [
    "Eyes", "Teeth", "Tongue", "EyeShell", "Lashes", "Brows", "LipSeal",
];
/// The worst a body vertex may be outside a garment that covers it.
pub(super) const POKE: f32 = 0.001;

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
/// pose, and the worst. A corner counts only where it is drawn.
pub(super) fn poke_through(
    label: &str,
    under: &Posed,
    garment: &Posed,
    covered: &[bool],
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
                    "  {label} {} out {:.1} mm at {p} (corner {k}); cloth {} normal {}",
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

/// Holes: how many corners of the body's `hidden` triangles (left out of
/// the drawing by the worn items, [`hidden_triangles`]) have no cloth
/// over them in this pose, by more than `POKE` (`avatar::fit::uncovered`),
/// and the worst. Hiding skin takes it out of [`poke_through`]'s count,
/// so this is what keeps hiding honest: hidden skin with nothing over it
/// is a hole a player sees through.
///
/// Since ADR-0045 (a garment's cover map hides the skin under it,
/// triangle by triangle, set back from its openings) hidden skin that the
/// cloth has sunk a little under is no hole: cloth is what is seen there,
/// which is what hiding the skin is for. Before, the rule was
/// `outside_cloth`, which counted it; an import still hides whole
/// *regions* by that stricter rule (`pose_safe_hides`).
pub(super) fn holes(
    label: &str,
    body: &Posed,
    hidden: &[[u32; 3]],
    garments: &[Posed],
) -> (usize, f32) {
    let surfaces: Vec<Surface> = garments
        .iter()
        .filter(|g| !is_footwear(g))
        .map(|g| Surface::new(g.positions.clone(), g.normals.clone(), g.triangles.clone()))
        .collect();
    let mut all = body.triangles.clone();
    all.extend_from_slice(hidden);
    let skin = Surface::new(body.positions.clone(), body.normals.clone(), all);
    let mut corners: Vec<u32> = hidden.iter().flatten().copied().collect();
    corners.sort_unstable();
    corners.dedup();
    let mut count = 0;
    let mut worst = f32::NEG_INFINITY;
    for k in corners {
        let (p, n) = (body.positions[k as usize], body.normals[k as usize]);
        let out = uncovered(&surfaces, &skin, p, n);
        worst = worst.max(out);
        if out > POKE {
            count += 1;
            if std::env::var_os("AXCENITY_FIT_GATE_DEBUG").is_some() {
                println!(
                    "  {label} hole: hidden corner {k} out {:.1} mm at {p} (normal {n})",
                    out * 1000.0
                );
            }
        }
    }
    (count, worst)
}

/// The name holes are recorded under in the gate.
pub(super) const HOLES: &str = "hidden skin showing";

/// The lower layers the gate checks under whatever else is worn: the
/// human base's cut underwear. A lower layer never comes through an upper
/// one where the upper one covers it at rest.
const UNDER: [&str; 2] = ["BoxerBriefs", "Bandeau"];

/// The cover each garment of `garments` gives at rest: (garment name,
/// covered body corners), and for each lower layer worn ([`UNDER`]) and
/// each garment over it, the lower layer's corners the upper one covers.
pub(super) struct Cover {
    pub body: Vec<(String, Vec<bool>)>,
    layers: Vec<(String, String, Vec<bool>)>,
    /// The body triangles the worn items hide ([`hidden_triangles`]),
    /// checked for holes in every pose.
    hidden: Vec<[u32; 3]>,
}

/// `bare` is the same body at rest with nothing worn, to find the skin
/// the worn items hide.
pub(super) fn cover_at_rest(body: &Posed, garments: &[Posed], bare: &Posed) -> Cover {
    let mut layers = Vec::new();
    for lower in garments.iter().filter(|g| UNDER.contains(&g.name.as_str())) {
        for upper in garments {
            if UNDER.contains(&upper.name.as_str()) || is_footwear(upper) {
                continue;
            }
            let covered = covered(lower, upper, 0.02);
            if covered.contains(&true) {
                layers.push((lower.name.clone(), upper.name.clone(), covered));
            }
        }
    }
    Cover {
        hidden: hidden_triangles(bare, body),
        body: garments
            .iter()
            .filter(|g| !is_footwear(g))
            .map(|g| (g.name.clone(), covered(body, g, 0.04)))
            .collect(),
        layers,
    }
}

/// `description` with nothing worn: its whole body drawn.
pub(super) fn undressed(description: &Description) -> Description {
    let mut bare = description.clone();
    bare.items.clear();
    bare
}

/// Shoes made for the footwear check (`garment_checks`, which knows a
/// closed shoe's inside: the foot over the shoe's inner floor is inside
/// it, not through it) are named `Shoes_…` and left out of the cover rule
/// here. MakeHuman's shoes (`Shoes01`…) aren't: they are measured as any
/// garment is.
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
    // Holes: skin the worn items hide that their cloth doesn't cover here.
    if !cover.hidden.is_empty() {
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
    // now more than `SLACK` under it (lower it). Records follow an item
    // down, and a gate that stopped finding a recorded case is caught.
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
            Some(_) => {}
        }
    }
    assert!(problems.is_empty(), "{label}: {problems:#?}");
}

/// The ratchet: a garment and pose recorded in `known` passes at its
/// record and fails past it, or when its defect is gone or well under the
/// record (the record must come down); one not recorded fails on any case.
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

/// The walk at its widest stride.
const WALK: Stance = Stance::Clip {
    name: "Walk",
    frames: 18,
};

/// The human base (ADR-0047: MakeHuman's body and clothes) in the gate:
/// the underwear cut from the body, alone, and a MakeHuman outfit with
/// shoes over it, on the human cast, measured with the same rule as the
/// other bases' garments, at rest, in the walk and in the T-pose. Its
/// garments follow the body by their own copies of its shapes, so nothing
/// here is fitted by `avatar::fit`.
#[test]
fn human_clothes_cover_the_human_cast() {
    let stage = Stage::new("fit-gate-human", &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    let cast = benchmarks("human-")
        .into_iter()
        .filter(|d| d.name.contains("-cast-"));
    for description in cast {
        let feminine = description.name.contains("-feminine-");
        let outfit: [&str; 2] = if feminine {
            ["makehuman/female-casualsuit01", "makehuman/shoes04"]
        } else {
            ["makehuman/male-casualsuit01", "makehuman/shoes01"]
        };
        let mut dressed = description.clone();
        for id in outfit {
            if !dressed.items.iter().any(|worn| worn.id == id) {
                dressed.items.push(WornItem::new(id));
            }
        }
        let (bare, _) = worn(&stage.assets, &undressed(&description), Stance::Rest);
        let outfits = [
            (
                format!("{} underwear", description.name),
                description.clone(),
            ),
            (description.name.clone(), dressed),
        ];
        for (name, outfit) in &outfits {
            let (body, garments) = worn(&stage.assets, outfit, Stance::Rest);
            let cover = cover_at_rest(&body, &garments, &bare);
            checked += measure(name, Stance::Rest, &body, &garments, &cover, &mut failures);
            for stance in [WALK, Stance::Held(HeldPose::TPose)] {
                let (body, garments) = worn(&stage.assets, outfit, stance);
                checked += measure(name, stance, &body, &garments, &cover, &mut failures);
            }
        }
    }
    println!(
        "human: {checked} garment checks, {} failing",
        failures.len()
    );
    // First measured on 2026-11-24, as open defects of drafts. The cut
    // underwear (boxer briefs, bandeau) has none in any pose. The MakeHuman
    // outfits are as their authors fitted them: the women's T-shirt and
    // jeans let a few corners of skin through, 2 mm at rest on the heavy
    // and muscular bodies and up to 9 mm in the stride; the men's shirt
    // and jeans one corner by 1 mm on the muscular body. "Hidden skin
    // showing" is skin the outfits' files hide (`delete_verts`, their cover
    // maps) that their cloth doesn't cover: one corner at the men's
    // collar at rest (34 mm), and the ankles and insteps between the jeans
    // and the shoes in the stride.
    // 2026-11-25: the two walk records fell (8.87 to 7.03 mm, 55.13 to
    // 48.57 mm) when each body shape was brought into the pose about its
    // own joints (`build.py` `shape_moves`), and the underwear under each
    // outfit joined the gate ([`UNDER`]): no corner of it through either
    // outfit in any of the three poses. The walk records were measured
    // again the same day in the library's jog (ADR-0049): the women's
    // 7.03 to 5.29 mm, the men's 1 body and 1.06 mm to 4 bodies and
    // 1.77 mm; and once more when the takes were baked on the human
    // base's own skeleton (ADR-0050; the arms and legs now point as the
    // library's do): the men's 1.77 to 4.25 mm. And when the collarbones
    // were turned against the chest instead of the world (`mocap.SWUNG`,
    // the motion review of 2026-11-25: in the jog they had been left 51
    // degrees behind the leaning chest): hidden skin showing in the
    // stride, 48.57 to 34.72 mm.
    let known = [
        ("FemaleCasualsuit01", "rest", 3, 0.00208),
        ("FemaleCasualsuit01", "TPose", 5, 0.00229),
        ("FemaleCasualsuit01", "Walk@18", 12, 0.00529),
        ("MaleCasualsuit01", "rest", 1, 0.00100),
        ("MaleCasualsuit01", "TPose", 1, 0.00100),
        ("MaleCasualsuit01", "Walk@18", 4, 0.00425),
        ("hidden skin showing", "rest", 8, 0.03450),
        ("hidden skin showing", "Walk@18", 24, 0.03472),
    ]
    .map(|(garment, pose, cases, worst)| Known {
        garment,
        pose,
        cases,
        worst,
    });
    check("human", &failures, &known);
}

/// The bodies each garment of the wardrobe is measured on, of each body
/// type it is cut for.
const WARDROBE_BODIES: [&str; 3] = ["default", "heavy", "muscular"];

/// The whole wardrobe in the gate (ADR-0050: every CC0 garment of the
/// converted MakeHuman library): each garment worn on its own over the
/// cut underwear, on the default, heavy and muscular bodies of each body
/// type it is cut for (one tagged `male` or `female` on that type alone),
/// at rest, in the walk and in the T-pose. A garment's records are under
/// its part's name: `Part` for skin through its cloth, `Part: hidden skin
/// showing` for skin its file hides that its cloth doesn't cover, `Part:
/// Other` for skin through another worn part while it is on, and `Lower
/// through Part` for the underwear through it.
#[test]
fn every_garment_of_the_human_wardrobe_is_in_the_gate() {
    let stage = Stage::new("fit-gate-wardrobe", &repository_library()).expect("the stage");
    let (library, catalogue, problems) = axcenity_avatar::Library::read(&repository_library());
    assert!(problems.is_empty(), "{problems:?}");
    let bodies = benchmarks("human-");
    let mut failures: Vec<Failure> = Vec::new();
    let (mut garments_measured, mut checked) = (0, 0);
    for entry in &catalogue.expect("the catalogue").items {
        let item = library.item(&entry.id).expect("a listed item");
        let Some(part) = item
            .model
            .as_ref()
            .and_then(|model| model.object.as_deref())
            .and_then(|object| object.rsplit('/').next())
        else {
            continue;
        };
        if item.kind == axcenity_avatar::ItemKind::Groom || UNDER.contains(&part) {
            continue;
        }
        garments_measured += 1;
        let tagged = |tag: &str| item.tags.iter().any(|t| t == tag);
        for frame in ["masculine", "feminine"] {
            if (frame == "masculine" && tagged("female")) || (frame == "feminine" && tagged("male"))
            {
                continue;
            }
            for body in WARDROBE_BODIES {
                let name = format!("human-{frame}-body-{body}");
                let description = bodies
                    .iter()
                    .find(|d| d.name == name)
                    .unwrap_or_else(|| panic!("no benchmark {name}"));
                let mut dressed = description.clone();
                dressed.items.push(WornItem::new(&item.id));
                let label = format!("{name} in {part}");
                let (bare, _) = worn(&stage.assets, &undressed(description), Stance::Rest);
                let from = failures.len();
                let (body_at_rest, garments) = worn(&stage.assets, &dressed, Stance::Rest);
                assert!(
                    garments.iter().any(|g| g.name == part),
                    "{label}: the garment isn't worn"
                );
                let cover = cover_at_rest(&body_at_rest, &garments, &bare);
                checked += measure(
                    &label,
                    Stance::Rest,
                    &body_at_rest,
                    &garments,
                    &cover,
                    &mut failures,
                );
                for stance in [WALK, Stance::Held(HeldPose::TPose)] {
                    let (body, garments) = worn(&stage.assets, &dressed, stance);
                    checked += measure(&label, stance, &body, &garments, &cover, &mut failures);
                }
                for failure in &mut failures[from..] {
                    if !failure.garment.contains(part) {
                        failure.garment = format!("{part}: {}", failure.garment);
                    }
                }
            }
        }
    }
    println!(
        "wardrobe: {garments_measured} garments, {checked} garment checks, {} failing",
        failures.len()
    );
    assert!(garments_measured >= 51, "{garments_measured} garments");
    if std::env::var_os("AXCENITY_FIT_GATE_RECORDS").is_some() {
        records(&failures);
    }
    check("the human wardrobe", &failures, &WARDROBE_KNOWN);
}

/// Prints `failures` as records, a line a garment and pose, to write a
/// first record list from (`AXCENITY_FIT_GATE_RECORDS`).
fn records(failures: &[Failure]) {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<(&str, &str), (usize, f32)> = BTreeMap::new();
    for failure in failures {
        let group = groups
            .entry((failure.garment.as_str(), failure.pose.as_str()))
            .or_default();
        group.0 += 1;
        group.1 = group.1.max(failure.worst);
    }
    for ((garment, pose), (cases, worst)) in groups {
        println!("    known({garment:?}, {pose:?}, {cases}, {worst:.5}),");
    }
}

/// A record of [`WARDROBE_KNOWN`].
const fn known(garment: &'static str, pose: &'static str, cases: usize, worst: f32) -> Known {
    Known {
        garment,
        pose,
        cases,
        worst,
    }
}

/// What the gate finds in the wardrobe, first measured on 2026-11-25:
/// open defects of drafts, as MakeHuman's authors fitted them and as the
/// converter binds them to the body's shapes (nothing here is fitted by
/// `avatar::fit`). 6 of the 51 garments have none. The largest: the
/// elegant dress hides skin its skirt stands clear of (10 cm, the most
/// the measure looks), the men's third casual suit and elegant suit let
/// the heavy body through (34 and 29 mm), the women's second casual suit
/// all three bodies (24 to 29 mm) and the panties 23 mm. The arm sleeves
/// and stockings are one mesh in three textures each, so their records
/// are the same. Measured with the underwear not drawn under a top or a
/// bottom (`Slot::hidden_by`): before that rule the same day, 145
/// records, 25 of them the underwear through what was worn over it.
/// The stride's records were measured again the same day when the
/// collarbones were turned against the chest instead of the world
/// (`mocap.SWUNG`, the motion review; ADR-0049, decision 5): 28 of them
/// moved. Hidden skin showing fell on thirteen garments (the men's first
/// casual suit 46.31 to 16.69 mm, the fifth 46.37 to 30.00 mm, the elegant
/// suit 35.99 to 21.82 mm) and two records went; skin through the cloth
/// rose on seven (the camisole 4.92 to 12.61 mm, the women's first casual
/// suit 1.76 to 8.44 mm, the sport suit 1.76 to 7.43 mm) and two garments
/// gained a record of 1 and 2 mm. The shoulders now sit on the chest as
/// the library's do; the garments are as they were.
/// Lower these as items improve; never raise one to pass an item.
#[rustfmt::skip]
const WARDROBE_KNOWN: [Known; 120] = [
    known("ArmsleeveFishnetLarge", "Walk@18", 5, 0.00284),
    known("ArmsleeveFishnetMedium", "Walk@18", 5, 0.00284),
    known("ArmsleeveFishnetSmall", "Walk@18", 5, 0.00284),
    known("ArmsleeveWhiteL", "Walk@18", 5, 0.00284),
    known("ArmsleeveWhiteM", "Walk@18", 5, 0.00284),
    known("ArmsleeveWhiteS", "Walk@18", 5, 0.00284),
    known("BodiceStyleTop: hidden skin showing", "rest", 1, 0.00145),
    known("BoxerBriefs through SweaterFisherman", "TPose", 3, 0.00769),
    known("BoxerBriefs through SweaterFisherman", "Walk@18", 5, 0.00790),
    known("BoxerBriefs through SweaterFisherman", "rest", 3, 0.00769),
    known("CamisoleTop", "TPose", 4, 0.00754),
    known("CamisoleTop", "Walk@18", 6, 0.01261),
    known("CamisoleTop", "rest", 2, 0.00345),
    known("CamisoleTop: hidden skin showing", "TPose", 1, 0.02869),
    known("CamisoleTop: hidden skin showing", "Walk@18", 5, 0.02452),
    known("CamisoleTop: hidden skin showing", "rest", 1, 0.02869),
    known("CargoPants", "TPose", 1, 0.00140),
    known("CargoPants", "Walk@18", 6, 0.00991),
    known("CargoPants", "rest", 1, 0.00140),
    known("CrudeMaleShirt", "TPose", 1, 0.00104),
    known("CrudeMaleShirt", "Walk@18", 5, 0.00302),
    known("CrudeMaleShirt: hidden skin showing", "Walk@18", 6, 0.00731),
    known("Crudefemaletshirt: hidden skin showing", "Walk@18", 6, 0.00598),
    known("FemaleCasualsuit01", "TPose", 2, 0.00206),
    known("FemaleCasualsuit01", "Walk@18", 3, 0.00844),
    known("FemaleCasualsuit01", "rest", 2, 0.00181),
    known("FemaleCasualsuit01: hidden skin showing", "Walk@18", 3, 0.02173),
    known("FemaleCasualsuit02", "TPose", 3, 0.02364),
    known("FemaleCasualsuit02", "Walk@18", 3, 0.02882),
    known("FemaleCasualsuit02", "rest", 3, 0.02364),
    known("FemaleCasualsuit02: hidden skin showing", "Walk@18", 3, 0.00811),
    known("FemaleElegantsuit01", "TPose", 3, 0.00330),
    known("FemaleElegantsuit01", "Walk@18", 3, 0.00281),
    known("FemaleElegantsuit01", "rest", 3, 0.00337),
    known("FemaleElegantsuit01: hidden skin showing", "TPose", 3, 0.10000),
    known("FemaleElegantsuit01: hidden skin showing", "Walk@18", 3, 0.04490),
    known("FemaleElegantsuit01: hidden skin showing", "rest", 3, 0.10000),
    known("FemalePanties01", "TPose", 3, 0.02260),
    known("FemalePanties01", "Walk@18", 3, 0.01808),
    known("FemalePanties01", "rest", 3, 0.02260),
    known("FemaleSportsuit01", "TPose", 2, 0.00206),
    known("FemaleSportsuit01", "Walk@18", 2, 0.00743),
    known("FemaleSportsuit01", "rest", 2, 0.00181),
    known("FemaleSportsuit01: hidden skin showing", "Walk@18", 3, 0.02858),
    known("FemaleTop01", "TPose", 3, 0.01935),
    known("FemaleTop01", "Walk@18", 3, 0.01574),
    known("FemaleTop01", "rest", 3, 0.01612),
    known("JeanShorts", "Walk@18", 2, 0.00136),
    known("LegWarmerSocks", "TPose", 3, 0.00183),
    known("LegWarmerSocks", "Walk@18", 3, 0.00183),
    known("LegWarmerSocks", "rest", 3, 0.00183),
    known("MaleCasualsuit01", "TPose", 1, 0.00179),
    known("MaleCasualsuit01", "Walk@18", 1, 0.00175),
    known("MaleCasualsuit01", "rest", 1, 0.00179),
    known("MaleCasualsuit01: hidden skin showing", "Walk@18", 3, 0.01669),
    known("MaleCasualsuit01: hidden skin showing", "rest", 2, 0.03440),
    known("MaleCasualsuit02: hidden skin showing", "Walk@18", 3, 0.01244),
    known("MaleCasualsuit03", "TPose", 2, 0.03431),
    known("MaleCasualsuit03", "Walk@18", 1, 0.03200),
    known("MaleCasualsuit03", "rest", 1, 0.03431),
    known("MaleCasualsuit03: hidden skin showing", "Walk@18", 3, 0.01656),
    known("MaleCasualsuit04", "Walk@18", 1, 0.00104),
    known("MaleCasualsuit04: hidden skin showing", "Walk@18", 3, 0.01246),
    known("MaleCasualsuit05", "TPose", 2, 0.00480),
    known("MaleCasualsuit05", "Walk@18", 2, 0.00371),
    known("MaleCasualsuit05", "rest", 2, 0.00364),
    known("MaleCasualsuit05: hidden skin showing", "TPose", 3, 0.03880),
    known("MaleCasualsuit05: hidden skin showing", "Walk@18", 3, 0.03000),
    known("MaleCasualsuit05: hidden skin showing", "rest", 3, 0.04003),
    known("MaleCasualsuit06: hidden skin showing", "TPose", 1, 0.00754),
    known("MaleCasualsuit06: hidden skin showing", "Walk@18", 3, 0.01187),
    known("MaleElegantsuit01", "TPose", 1, 0.02895),
    known("MaleElegantsuit01", "Walk@18", 1, 0.02836),
    known("MaleElegantsuit01", "rest", 1, 0.02895),
    known("MaleElegantsuit01: hidden skin showing", "Walk@18", 3, 0.02182),
    known("MaleWorksuit01: hidden skin showing", "Walk@18", 6, 0.02198),
    known("MaleWorksuit01: hidden skin showing", "rest", 3, 0.01791),
    known("PantsHarem", "TPose", 5, 0.00507),
    known("PantsHarem", "Walk@18", 5, 0.00759),
    known("PantsHarem", "rest", 5, 0.00507),
    known("PantsWool", "TPose", 1, 0.00135),
    known("PantsWool", "Walk@18", 3, 0.01324),
    known("PantsWool", "rest", 1, 0.00135),
    known("PoloTShirt", "TPose", 1, 0.00619),
    known("PoloTShirt", "Walk@18", 3, 0.01337),
    known("Shoes02: hidden skin showing", "TPose", 2, 0.00468),
    known("Shoes02: hidden skin showing", "Walk@18", 2, 0.00498),
    known("Shoes02: hidden skin showing", "rest", 2, 0.00468),
    known("Shoes03", "TPose", 3, 0.00794),
    known("Shoes03", "Walk@18", 3, 0.00794),
    known("Shoes03", "rest", 3, 0.00794),
    known("Shoes05: hidden skin showing", "TPose", 2, 0.00468),
    known("Shoes05: hidden skin showing", "Walk@18", 2, 0.00498),
    known("Shoes05: hidden skin showing", "rest", 2, 0.00468),
    known("Shoes06: hidden skin showing", "TPose", 2, 0.00468),
    known("Shoes06: hidden skin showing", "Walk@18", 2, 0.00498),
    known("Shoes06: hidden skin showing", "rest", 2, 0.00468),
    known("Sockslacefrill", "TPose", 3, 0.00226),
    known("Sockslacefrill", "Walk@18", 3, 0.00226),
    known("Sockslacefrill", "rest", 3, 0.00226),
    known("StockingsFishnetLarge", "Walk@18", 4, 0.00260),
    known("StockingsFishnetMedium", "Walk@18", 4, 0.00260),
    known("StockingsFishnetSmall", "Walk@18", 4, 0.00260),
    known("StockingsWhiteL", "Walk@18", 4, 0.00260),
    known("StockingsWhiteM", "Walk@18", 4, 0.00260),
    known("StockingsWhiteS", "Walk@18", 4, 0.00260),
    known("SweaterFisherman", "TPose", 6, 0.00443),
    known("SweaterFisherman", "Walk@18", 6, 0.01608),
    known("SweaterFisherman", "rest", 5, 0.00376),
    known("TShirtBasicTucked", "TPose", 3, 0.00286),
    known("TShirtBasicTucked", "Walk@18", 6, 0.00411),
    known("TShirtBasicTucked", "rest", 2, 0.00290),
    known("TankKeyholeNeck", "Walk@18", 2, 0.00171),
    known("TankKeyholeNeck: hidden skin showing", "Walk@18", 2, 0.01621),
    known("TurtleneckHalter", "TPose", 6, 0.00997),
    known("TurtleneckHalter", "Walk@18", 6, 0.00897),
    known("TurtleneckHalter", "rest", 1, 0.00245),
    known("TurtleneckHalter: hidden skin showing", "TPose", 2, 0.02221),
    known("TurtleneckHalter: hidden skin showing", "Walk@18", 2, 0.02698),
    known("TurtleneckHalter: hidden skin showing", "rest", 2, 0.02221),
];

/// Paul's test of 2026-11-25: the human base's underwear came through its
/// outfits. Worn alone the briefs and the bandeau are drawn whole; under
/// an outfit neither is drawn at all, on every body of the human cast (a
/// top takes the bandeau's place and a bottom the briefs': `Slot::hidden_by`),
/// and the socks of the shoes are cut where the jeans cover them.
#[test]
fn the_human_bases_underwear_is_not_drawn_under_an_outfit() {
    let stage = Stage::new("fit-gate-human-layers", &repository_library()).expect("the stage");
    let cast = benchmarks("human-")
        .into_iter()
        .filter(|d| d.name.contains("-cast-") || d.name.contains("-body-"));
    let mut checked = 0;
    for description in cast {
        let feminine = description.name.contains("-feminine-");
        let (suit, shoes) = if feminine {
            ("makehuman/female-casualsuit01", "makehuman/shoes04")
        } else {
            ("makehuman/male-casualsuit01", "makehuman/shoes01")
        };
        let triangles = |items: &[&str]| {
            let mut worn_now = description.clone();
            for id in items {
                if !worn_now.items.iter().any(|worn| worn.id == *id) {
                    worn_now.items.push(WornItem::new(*id));
                }
            }
            let (_, garments) = worn(&stage.assets, &worn_now, Stance::Rest);
            move |name: &str| {
                garments
                    .iter()
                    .find(|garment| garment.name.starts_with(name))
                    .map(|garment| garment.triangles.len())
            }
        };
        let alone = triangles(&[shoes]);
        let dressed = triangles(&[shoes, suit]);
        for name in ["BoxerBriefs", "Bandeau"] {
            let Some(whole) = alone(name) else {
                continue;
            };
            assert!(whole > 1000, "{}: {name} alone", description.name);
            assert_eq!(
                dressed(name),
                None,
                "{}: {name} under {suit}",
                description.name
            );
            checked += 1;
        }
        let (bare, covered) = (alone("Shoes").unwrap(), dressed("Shoes").unwrap());
        assert!(
            covered < bare,
            "{}: the socks under the jeans ({covered} of {bare})",
            description.name
        );
    }
    assert!(checked >= 50, "{checked} checked");
}
