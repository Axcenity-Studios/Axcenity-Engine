//! The first licensed joggers (wave 5, Paul's decision 13: "clean cuffs,
//! consistent geometry and proper leg deformation"): BlenderKit's Grey
//! sweat pants (CC0), refined and fitted by the intake
//! (`assets/characters/realistic/import/`), worn over the underwear by
//! every realistic cast and body benchmark, as a game poses them. Held to
//! `docs/development/character-studio/asset-quality.md` §2 and §3 with
//! **no records**: what a production item must pass.
//!
//! - **Poke-through** (the fit gate's rule, `fit_gate_tests`): no body
//!   corner more than 1 mm outside the joggers where they cover it at
//!   rest, and no underwear through them, at rest, in Idle, in every held
//!   pose (the hips flexed too), a walk frame and the whole stride.
//! - **Levels of detail**: the same at rest for each simpler copy.
//! - **Stability in motion**: Idle, Walk and Run (`garment_checks`).
//! - **The knee wraps the bent knee's front**: with the knees bent, no
//!   point of the cloth's surface over the front of the knee lies more
//!   than 1 mm inside the skin (sampled inside the triangles, not only at
//!   corners). The measure fails the wave-4 joggers, so it can fail. It
//!   doesn't judge the back of the knee (see `knee_front_failures`).
//! - **The waistband stays closed**: its top edge stays near the skin all
//!   round in the walk and with the hips flexed (§3 "Sleeves and waist").

use axcenity_avatar::fit::Surface;
use axcenity_avatar::{Description, WornItem};
use axcenity_core::{ClassName, Value};
use axcenity_render::glam::Vec3;

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};
use super::fit_gate_tests::{
    Failure, Posed, WALK_FRAMES, bodies, check, cover_at_rest, measure, measure_walk, poke_through,
    pose_name, posed_at, undressed, worn,
};
use super::garment_checks::check_motion;

pub(super) const JOGGERS: &str = "axcenity/realistic-fleece-joggers";
/// The part the joggers draw as.
const PART: &str = "Pants_FleeceJoggers";

/// `description` in the joggers, over the underwear its frame wears.
fn dressed(description: &Description) -> Description {
    let mut out = description.clone();
    out.items.push(WornItem::new(JOGGERS));
    out
}

/// A failure that is the joggers' (or a layer through them).
fn ours(failure: &Failure) -> bool {
    failure.garment.contains(PART)
}

/// The gate for `stance` on every realistic cast and body benchmark in
/// the joggers, on the shipped library. Nothing is recorded: any failure
/// fails.
fn joggers_gate(stance: Stance) {
    let label = pose_name(stance);
    let stage =
        Stage::new(&format!("joggers-gate-{label}"), &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        let names: Vec<&str> = garments.iter().map(|g| g.name.as_str()).collect();
        assert!(names.contains(&PART), "{}: {names:?}", description.name);
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
    failures.retain(ours);
    println!(
        "joggers gate {label}: {checked} garment checks, {} failing",
        failures.len()
    );
    check(&label, &failures, &[]);
}

/// The walking fit test (decision 13) through the walk's `frames`.
fn joggers_gate_walk(frames: &[u32]) {
    let label = format!("Walk {frames:?}");
    let stage = Stage::new(
        &format!("joggers-gate-walk-{}", frames[0]),
        &repository_library(),
    )
    .expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        // A walk without the joggers would pass: they must be worn.
        let names: Vec<&str> = garments.iter().map(|g| g.name.as_str()).collect();
        assert!(names.contains(&PART), "{}: {names:?}", description.name);
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
    failures.retain(ours);
    println!(
        "joggers gate {label}: {checked} garment checks, {} failing",
        failures.len()
    );
    check(&label, &failures, &[]);
}

#[test]
fn the_fleece_joggers_cover_the_cast_at_rest() {
    joggers_gate(Stance::Rest);
}

#[test]
fn the_fleece_joggers_cover_the_cast_in_idle() {
    joggers_gate(Stance::Idle);
}

#[test]
fn the_fleece_joggers_cover_the_cast_walking() {
    joggers_gate(Stance::Clip {
        name: "Walk",
        frames: 18,
    });
}

#[test]
fn the_fleece_joggers_cover_the_cast_through_the_walk_first_third() {
    joggers_gate_walk(WALK_FRAMES[0]);
}

#[test]
fn the_fleece_joggers_cover_the_cast_through_the_walk_second_third() {
    joggers_gate_walk(WALK_FRAMES[1]);
}

#[test]
fn the_fleece_joggers_cover_the_cast_through_the_walk_last_third() {
    joggers_gate_walk(WALK_FRAMES[2]);
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_knees_bent() {
    joggers_gate(Stance::Held(HeldPose::KneesBent));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_hips_flexed() {
    joggers_gate(Stance::Held(HeldPose::HipFlex));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_arms_raised() {
    joggers_gate(Stance::Held(HeldPose::ArmsRaised));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_arms_out() {
    joggers_gate(Stance::Held(HeldPose::TPose));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_arms_forward() {
    joggers_gate(Stance::Held(HeldPose::ArmsForward));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_elbows_bent() {
    joggers_gate(Stance::Held(HeldPose::ElbowsBent));
}

/// The body and the joggers at rest, the joggers at level of detail
/// `level`.
fn worn_at_level(stage: &Stage, description: &Description, level: usize) -> (Posed, Vec<Posed>) {
    let (world, animators) =
        stage_world(&stage.assets, description, Stance::Rest, &[]).expect("the stage world");
    let parts = world.all_below(world.workspace(), |class| class == ClassName::MeshPart);
    let named = |name: &str| {
        parts.iter().copied().find(|part| {
            world.get(*part).is_some_and(|p| p.name() == name)
                && matches!(
                    world.get(*part).and_then(|p| p.property("MeshId")),
                    Some(Value::String(id)) if !id.is_empty()
                )
        })
    };
    let body = posed_at(&world, &stage.assets, &animators, named("Body").unwrap(), 0);
    let garments = named(PART)
        .map(|part| posed_at(&world, &stage.assets, &animators, part, level))
        .into_iter()
        .collect();
    (body, garments)
}

/// §2 "LODs": each simpler copy stays clear of the body where the full
/// one covers it, at rest, on every cast and body benchmark.
#[test]
fn the_fleece_joggers_levels_of_detail_stay_clear_of_the_body() {
    let stage = Stage::new("joggers-lods", &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, full) = worn_at_level(&stage, &dressed, 0);
        assert_eq!(full.len(), 1, "{}", description.name);
        let (bare, _) = worn_at_level(&stage, &undressed(&description), 0);
        let cover = cover_at_rest(&body, &full, &bare);
        let covered = &cover.body.iter().find(|(n, _)| n == PART).unwrap().1;
        for level in 1..=2 {
            let (_, simpler) = worn_at_level(&stage, &dressed, level);
            let label = format!("{} LOD{level}", description.name);
            let (count, worst) = poke_through(&label, &body, &simpler[0], covered);
            println!("{label}: {count} out, worst {:.2} mm", worst * 1000.0);
            if count > 0 {
                failures.push(Failure {
                    garment: format!("{PART} LOD{level}"),
                    pose: "rest".to_owned(),
                    body: description.name.clone(),
                    count,
                    worst,
                });
            }
        }
    }
    check("levels of detail", &failures, &[]);
}

/// The realistic cast (not the extremes) in the joggers.
fn cast() -> Vec<Description> {
    bodies()
        .into_iter()
        .filter(|d| d.name.contains("-cast-"))
        .map(|d| dressed(&d))
        .collect()
}

#[test]
fn the_fleece_joggers_stay_steady_in_the_idle_walk_and_run() {
    let stage = Stage::new("joggers-motion", &repository_library()).expect("the stage");
    let cast = cast();
    for description in &cast {
        let (_, garments) = worn(&stage.assets, description, Stance::Rest);
        assert!(
            garments.iter().any(|g| g.name == PART),
            "{}: no joggers",
            description.name
        );
    }
    for clip in ["Idle", "Walk", "Run"] {
        check_motion(&stage, &cast, clip, &[]);
    }
}

/// The rest pose's knee joint height (m, the masculine and feminine frames
/// share the skeleton: `LeftLowerLeg`'s head).
const KNEE_HEIGHT: f32 = 0.50;
/// The knee's band: cloth whose rest height is within this of the knee.
const KNEE_BAND: f32 = 0.08;
/// How far inside the skin the cloth's surface may lie: the fit gate's
/// poke-through limit (`fit_gate_tests`).
const KNEE_CUT: f32 = 0.001;

/// Points across a triangle, as barycentric weights: its middle, its
/// edges' middles, and one between the middle and each corner. The gate
/// checks corners; a long triangle across a bend cuts the corner of the
/// bend between its corners, so this samples inside the triangles.
const SAMPLES: [[f32; 3]; 7] = [
    [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
    [0.5, 0.5, 0.0],
    [0.0, 0.5, 0.5],
    [0.5, 0.0, 0.5],
    [0.6, 0.2, 0.2],
    [0.2, 0.6, 0.2],
    [0.2, 0.2, 0.6],
];

/// Over the front of one knee (`side` +1 or -1: the cloth whose rest
/// height is within [`KNEE_BAND`] of the knee and whose rest face looks
/// forward), the cloth's surface as `posed` against the posed `body`:
/// how many of its sample points lie more than [`KNEE_CUT`] inside the
/// skin, of how many, and the deepest (m).
fn knee_front_cut(rest: &Posed, posed: &Posed, body: &Posed, side: f32) -> (usize, usize, f32) {
    let skin = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    let (mut total, mut inside, mut deepest) = (0, 0, 0.0f32);
    for t in &rest.triangles {
        let r = t.map(|k| rest.positions[k as usize]);
        let middle = (r[0] + r[1] + r[2]) / 3.0;
        let forward = (r[1] - r[0]).cross(r[2] - r[0]).z > 0.0;
        if (middle.y - KNEE_HEIGHT).abs() > KNEE_BAND || middle.x * side < 0.03 || !forward {
            continue;
        }
        let p = t.map(|k| posed.positions[k as usize]);
        for [a, b, c] in SAMPLES {
            let q = p[0] * a + p[1] * b + p[2] * c;
            total += 1;
            if let Some(hit) = skin.closest(q, 0.05) {
                let depth = -hit.signed(q);
                if depth > KNEE_CUT {
                    inside += 1;
                    deepest = deepest.max(depth);
                }
            }
        }
    }
    (total, inside, deepest)
}

/// "Proper leg deformation" at the knee for `item` (drawn as `part`): with
/// the knees bent 135 degrees (the body suite's pose), on every cast body,
/// the cloth wraps the front of each knee: no sample of its surface lies
/// more than [`KNEE_CUT`] inside the skin. Returns the knees that fail.
///
/// This is the side a bend stretches: cloth that cuts the kneecap's corner
/// between its corners shows there. (Whether the joggers' knee loops are
/// what passes it is not shown: they have not been rebuilt without them.)
/// The back of the bent knee
/// is not judged: there the calf meets the thigh and the body's own skin
/// closes the crease, so any cloth in it lies inside the skin (up to 15 mm
/// for these joggers and the wave-4 ones alike) and none of it shows.
fn knee_front_failures(item: &str, part: &str) -> Vec<String> {
    let stage =
        Stage::new(&format!("knee-front-{part}"), &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    for description in bodies().into_iter().filter(|d| d.name.contains("-cast-")) {
        let mut dressed = description.clone();
        dressed.items.push(WornItem::new(item));
        let (_, rest) = worn(&stage.assets, &dressed, Stance::Rest);
        let (bent_body, bent) = worn(&stage.assets, &dressed, Stance::Held(HeldPose::KneesBent));
        let rest = rest.iter().find(|g| g.name == part).unwrap();
        let bent = bent.iter().find(|g| g.name == part).unwrap();
        for side in [1.0f32, -1.0] {
            let (total, inside, deepest) = knee_front_cut(rest, bent, &bent_body, side);
            println!(
                "{part} on {} side {side}: {inside} of {total} samples over the knee's front more than 1 mm inside the skin, deepest {:.1} mm",
                description.name,
                deepest * 1000.0
            );
            assert!(total > 100, "the knee's front has cloth");
            if inside > 0 {
                failures.push(format!(
                    "{} side {side}: {inside} samples, {:.1} mm",
                    description.name,
                    deepest * 1000.0
                ));
            }
        }
    }
    failures
}

#[test]
fn the_fleece_joggers_wrap_the_front_of_the_bent_knee() {
    let failures = knee_front_failures(JOGGERS, PART);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The knee measure can fail: the wave-4 joggers (`axcenity/realistic-
/// joggers`, a Draft kept as it is) cut the front of the bent right knee
/// on every cast body (their left knee passes, so this shows a fault of
/// theirs, not that missing knee loops fail). If they are ever remade,
/// point this at another garment that fails it rather than deleting it.
#[test]
fn the_knee_measure_catches_the_wave_4_joggers() {
    let failures = knee_front_failures("axcenity/realistic-joggers", "Pants_Joggers");
    assert!(
        !failures.is_empty(),
        "the measure passed the wave-4 joggers, which cut the bent right knee"
    );
}

/// How far the waistband's top edge may stand off the skin in a pose
/// (§3 "Sleeves and waist": the band stays closed round the waist, no gap
/// to skin): the boxer briefs' band under it (up to 9 mm off the skin) and
/// the band's own ease, and no more.
const WAIST_GAP: f32 = 0.012;

/// The waistband's top edge: the joggers' corners within 4 mm of their
/// highest at rest.
fn waist_edge(stage: &Stage, description: &Description) -> Vec<usize> {
    let (_, rest) = worn(&stage.assets, description, Stance::Rest);
    let rest = rest.iter().find(|g| g.name == PART).unwrap();
    let top = rest.positions.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    (0..rest.positions.len())
        .filter(|k| rest.positions[*k].y > top - 0.004)
        .collect()
}

/// How far the farthest corner of `edge` is from the skin in `stance`,
/// and where.
fn waist_gap(
    stage: &Stage,
    description: &Description,
    edge: &[usize],
    stance: Stance,
) -> (f32, Vec3) {
    let (body, garments) = worn(&stage.assets, description, stance);
    let posed = garments.iter().find(|g| g.name == PART).unwrap();
    let skin = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    let mut worst = (0.0f32, Vec3::ZERO);
    for &k in edge {
        let p = posed.positions[k];
        let gap = skin.closest(p, 0.05).map_or(0.05, |hit| hit.distance);
        if gap > worst.0 {
            worst = (gap, p);
        }
    }
    worst
}

/// The waistband stays closed round the waist (§3 "Sleeves and waist"):
/// in the walk's frames and with the hips flexed, on every cast body, its
/// top edge is never more than [`WAIST_GAP`] off the skin.
#[test]
fn the_fleece_joggers_waistband_stays_closed_in_the_walk_and_the_hip_flex() {
    let stage = Stage::new("joggers-waist", &repository_library()).expect("the stage");
    let mut problems = Vec::new();
    for description in bodies().into_iter().filter(|d| d.name.contains("-cast-")) {
        let dressed = dressed(&description);
        let mut stances: Vec<Stance> = WALK_FRAMES
            .iter()
            .flat_map(|frames| frames.iter())
            .map(|frames| Stance::Clip {
                name: "Walk",
                frames: *frames,
            })
            .collect();
        stances.push(Stance::Rest);
        stances.push(Stance::Held(HeldPose::HipFlex));
        let edge = waist_edge(&stage, &dressed);
        let mut worst = (0.0f32, String::new(), Vec3::ZERO);
        for stance in stances {
            let (gap, at) = waist_gap(&stage, &dressed, &edge, stance);
            if gap > worst.0 {
                worst = (gap, pose_name(stance), at);
            }
        }
        println!(
            "{}: the waistband's top edge is at most {:.1} mm off the skin ({} at {})",
            description.name,
            worst.0 * 1000.0,
            worst.1,
            worst.2
        );
        if worst.0 > WAIST_GAP {
            problems.push(format!(
                "{}: {:.1} mm in {}",
                description.name,
                worst.0 * 1000.0,
                worst.1
            ));
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// A posed triangle's face (unit, by its winding), the mean of its
/// corners' shading normals (unit) and its area.
fn face_and_shading(posed: &Posed, t: [u32; 3]) -> (Vec3, Vec3, f32) {
    let p = t.map(|k| posed.positions[k as usize]);
    let cross = (p[1] - p[0]).cross(p[2] - p[0]);
    let mean = t
        .map(|k| posed.normals[k as usize])
        .iter()
        .fold(Vec3::ZERO, |s, n| s + *n);
    (
        cross.normalize_or_zero(),
        mean.normalize_or_zero(),
        cross.length() / 2.0,
    )
}

/// A triangle a ray crosses: its distance (studs), part, material,
/// whether it faces the camera, and its middle at rest (m).
type Seen = (f32, String, String, bool, Vec3);

/// What a capture's camera sees at pixel `pixel` of a `size` picture
/// (`character_capture`'s `view`: the eye and target in studs, the
/// vertical field of view in degrees), on `description` in `stance`:
/// every triangle of every MeshPart the ray through the pixel's middle
/// crosses, nearest first, as (distance in studs, part, material, whether
/// the triangle faces the camera, its middle at rest in the part's own
/// metres), and how many studs a metre of the body is. Materials are
/// single-sided, so the first triangle that faces the camera is drawn.
fn seen_at(
    stage: &Stage,
    description: &Description,
    stance: Stance,
    view: ([f32; 3], [f32; 3], f32),
    size: (f32, f32),
    pixel: (f32, f32),
) -> (Vec<Seen>, f32) {
    use axcenity_render::glam::{Mat4, Quat};
    let (world, animators) = stage_world(&stage.assets, description, stance, &[]).unwrap();
    let (eye, target) = (Vec3::from(view.0), Vec3::from(view.1));
    let forward = (target - eye).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward);
    let tan = (view.2.to_radians() / 2.0).tan();
    let x = ((pixel.0 + 0.5) / size.0 * 2.0 - 1.0) * tan * size.0 / size.1;
    let y = (1.0 - (pixel.1 + 0.5) / size.1 * 2.0) * tan;
    let ray = (forward + right * x + up * y).normalize();
    let (mut hits, mut studs_per_metre) = (Vec::new(), 0.0);
    for part in world.all_below(world.workspace(), |class| class == ClassName::MeshPart) {
        let instance = world.get(part).unwrap();
        let Some(Value::String(mesh_id)) = instance.property("MeshId") else {
            continue;
        };
        if mesh_id.is_empty() || !animators.poses().contains_key(&part) {
            continue;
        }
        let vector = |name: &str| instance.property(name).and_then(Value::as_vector3);
        let (Some(position), Some(orientation), Some(scale)) =
            (vector("Position"), vector("Orientation"), vector("Size"))
        else {
            continue;
        };
        let rotation: Quat = axcenity_scene::rotation_from_orientation(orientation);
        // As the draw list places a MeshPart (`part_placement`).
        let placement = Mat4::from_scale_rotation_translation(
            Vec3::new(scale.x, scale.y, scale.z),
            rotation,
            Vec3::new(position.x, position.y, position.z),
        );
        let model = stage.assets.model_blocking(mesh_id).unwrap();
        let pose = animators.poses()[&part].clone();
        let fit = model.rig.as_ref().unwrap().fit;
        let unfit = fit.inverse();
        if instance.name() == "Body" {
            studs_per_metre = (placement * fit).transform_vector3(Vec3::Y).length();
        }
        for (index, primitive) in model.primitives.iter().enumerate() {
            let moved = pose.primitive(&model, index, 0);
            let placed: Vec<Vec3> = moved
                .positions
                .iter()
                .map(|p| placement.transform_point3(Vec3::from(*p)))
                .collect();
            for t in primitive.indices.chunks_exact(3) {
                let [a, b, c] = [0, 1, 2].map(|k| placed[t[k] as usize]);
                // Möller and Trumbore's ray-triangle test.
                let (e1, e2) = (b - a, c - a);
                let h = ray.cross(e2);
                let det = e1.dot(h);
                if det.abs() < 1e-12 {
                    continue;
                }
                let s = eye - a;
                let u = s.dot(h) / det;
                let q = s.cross(e1);
                let v = ray.dot(q) / det;
                if !(0.0..=1.0).contains(&u) || v < 0.0 || u + v > 1.0 {
                    continue;
                }
                let distance = e2.dot(q) / det;
                if distance <= 0.0 {
                    continue;
                }
                let facing = e1.cross(e2).dot(ray) < 0.0;
                let rest = [0, 1, 2].map(|k| {
                    unfit.transform_point3(Vec3::from(primitive.positions[t[k] as usize]))
                });
                hits.push((
                    distance,
                    instance.name().to_owned(),
                    primitive.material.name.clone(),
                    facing,
                    (rest[0] + rest[1] + rest[2]) / 3.0,
                ));
            }
        }
    }
    hits.sort_by(|x, y| x.0.total_cmp(&y.0));
    (hits, studs_per_metre)
}

/// Measurements behind the hand-back's account of the seat and the front
/// crotch (Review fixes 2), printed, not judged. Run with
/// `cargo nextest run -p axcenity-player --run-ignored only -E
/// 'test(/measure_the_joggers_seat/)' --no-capture`.
///
/// The body faces +z here, so the back camera stands at -z and draws a
/// face whose normal has z < 0; the front camera stands at +z and draws a
/// face whose normal has z > 0 (single-sided materials: the others are
/// culled, and what is behind them shows).
///
/// - With the hips flexed (the masculine average, slim, heavyset and the
///   feminine heavyset): the length of the edges across the seat, and how
///   sharply the cloth turns at the edges down it (the corners of the
///   seat's outline), at rest and flexed; and the triangles the back
///   camera draws whose shading normals face away from it (drawn dark).
/// - In the walk at 0.8 s (the masculine muscular, `close-11`): the front
///   crotch's triangles the front camera culls.
#[test]
#[ignore = "a measurement for the hand-back, not a check"]
fn measure_the_joggers_seat_and_crotch() {
    let stage = Stage::new("joggers-measure", &repository_library()).expect("the stage");
    let round = |p: Vec3| (p * 1000.0).round();
    let named = |name: &str| {
        bodies()
            .into_iter()
            .find(|d| d.name.ends_with(name))
            .unwrap_or_else(|| panic!("no body {name}"))
    };
    for name in [
        "masculine-cast-average",
        "masculine-cast-slim",
        "masculine-cast-heavyset",
        "feminine-cast-heavyset",
    ] {
        let dressed = dressed(&named(name));
        let (_, rest) = worn(&stage.assets, &dressed, Stance::Rest);
        let (_, flexed) = worn(&stage.assets, &dressed, Stance::Held(HeldPose::HipFlex));
        let rest = rest.iter().find(|g| g.name == PART).unwrap();
        let flexed = flexed.iter().find(|g| g.name == PART).unwrap();
        // The seat, by rest place: the back of the pelvis tube under the
        // pockets' middle and the legs' first rings.
        let seat = |p: Vec3| p.z < -0.04 && p.x.abs() < 0.2 && (0.70..0.86).contains(&p.y);
        let mut edges: std::collections::HashMap<(u32, u32), Vec<usize>> = Default::default();
        for (f, t) in rest.triangles.iter().enumerate() {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                edges.entry((a.min(b), a.max(b))).or_default().push(f);
            }
        }
        // (rest, flexed): lengths of the edges across the seat, and the
        // turn between the two faces of each edge down it (degrees).
        let (mut across, mut turns) = (Vec::new(), Vec::new());
        for ((a, b), faces) in &edges {
            let (ra, rb) = (rest.positions[*a as usize], rest.positions[*b as usize]);
            if !seat((ra + rb) / 2.0) {
                continue;
            }
            let (fa, fb) = (flexed.positions[*a as usize], flexed.positions[*b as usize]);
            let length = ra.distance(rb);
            let dy = (ra.y - rb.y).abs();
            if dy < 0.25 * length {
                across.push((length, fa.distance(fb)));
            } else if dy > 0.7 * length && faces.len() == 2 {
                let turn = |posed: &Posed| {
                    let n0 = face_and_shading(posed, posed.triangles[faces[0]]).0;
                    let n1 = face_and_shading(posed, posed.triangles[faces[1]]).0;
                    n0.dot(n1).clamp(-1.0, 1.0).acos().to_degrees()
                };
                turns.push((turn(rest), turn(flexed)));
            }
        }
        let median = |v: &[(f32, f32)], flexed: bool| {
            let mut s: Vec<f32> = v.iter().map(|x| if flexed { x.1 } else { x.0 }).collect();
            s.sort_by(f32::total_cmp);
            s[s.len() / 2]
        };
        let most = |v: &[(f32, f32)], flexed: bool| {
            v.iter()
                .map(|x| if flexed { x.1 } else { x.0 })
                .fold(0.0f32, f32::max)
        };
        println!(
            "{name}, hips flexed: {} edges across the seat: rest median {:.1} mm (longest {:.1}), flexed median {:.1} mm (longest {:.1}). {} edges down it: the cloth turns at rest median {:.1} degrees (most {:.1}), flexed median {:.1} (most {:.1}); {} over 20 degrees flexed, {} at rest",
            across.len(),
            median(&across, false) * 1000.0,
            most(&across, false) * 1000.0,
            median(&across, true) * 1000.0,
            most(&across, true) * 1000.0,
            turns.len(),
            median(&turns, false),
            most(&turns, false),
            median(&turns, true),
            most(&turns, true),
            turns.iter().filter(|t| t.1 > 20.0).count(),
            turns.iter().filter(|t| t.0 > 20.0).count(),
        );
        for t in &flexed.triangles {
            let (face, shading, area) = face_and_shading(flexed, *t);
            let r = t.map(|k| rest.positions[k as usize]);
            let middle = (r[0] + r[1] + r[2]) / 3.0;
            if face.z < -0.3 && shading.z > 0.0 && middle.z < 0.0 && middle.y > 0.6 {
                let p = t.map(|k| flexed.positions[k as usize]);
                println!(
                    "  drawn dark from behind: rest {} flexed {} ({:.0} mm², face z {:.2}, shading z {:.2})",
                    round(middle),
                    round((p[0] + p[1] + p[2]) / 3.0),
                    area * 1e6,
                    face.z,
                    shading.z
                );
            }
            // Faces the back camera saw at rest and culls flexed.
            let rest_face = face_and_shading(rest, *t).0;
            if rest_face.z < -0.2 && face.z > 0.0 && (0.7..1.0).contains(&middle.y) {
                let p = t.map(|k| flexed.positions[k as usize]);
                println!(
                    "  culled from behind, flexed: rest {} (rest face z {:.2}) flexed {} ({:.0} mm², face z {:.2})",
                    round(middle),
                    rest_face.z,
                    round((p[0] + p[1] + p[2]) / 3.0),
                    area * 1e6,
                    face.z
                );
            }
        }
    }
    // The front crotch in the walk at 0.8 s (48 sixtieths).
    let muscular = dressed(&named("masculine-cast-muscular"));
    let (_, rest) = worn(&stage.assets, &muscular, Stance::Rest);
    let walk = Stance::Clip {
        name: "Walk",
        frames: 48,
    };
    let (_, walk) = worn(&stage.assets, &muscular, walk);
    let rest = rest.iter().find(|g| g.name == PART).unwrap();
    let walk = walk.iter().find(|g| g.name == PART).unwrap();
    let (mut culled, mut culled_area) = (0, 0.0f32);
    for t in &rest.triangles {
        let r = t.map(|k| rest.positions[k as usize]);
        let middle = (r[0] + r[1] + r[2]) / 3.0;
        if middle.x.abs() > 0.06 || !(0.70..0.85).contains(&middle.y) || middle.z < -0.02 {
            continue;
        }
        let rest_face = face_and_shading(rest, *t).0;
        let (face, shading, area) = face_and_shading(walk, *t);
        if face.z < 0.0 && (rest_face.z > 0.0 || shading.z > 0.0) {
            culled += 1;
            culled_area += area;
            let p = t.map(|k| walk.positions[k as usize]);
            println!(
                "  front crotch culled at Walk@0.80: rest {} (rest face z {:.2}) walk {} ({:.0} mm², face z {:.2}, shading z {:.2})",
                round(middle),
                rest_face.z,
                round((p[0] + p[1] + p[2]) / 3.0),
                area * 1e6,
                face.z,
                shading.z
            );
        }
    }
    println!(
        "masculine-cast-muscular, Walk@0.80: {culled} front-crotch triangles culled by the front camera, {:.0} mm²",
        culled_area * 1e6
    );
    // What the studio captures (1920 x 1080) show at the notch on the
    // seat (back view, the masculine average and feminine heavyset, hips
    // flexed) and in the
    // front crotch's hole (front view, the masculine muscular, Walk@0.80),
    // with a pixel of plain cloth beside each.
    const BACK: ([f32; 3], [f32; 3], f32) = ([0.0, 2.6, 9.5], [0.0, 2.6, 0.0], 35.0);
    const FRONT: ([f32; 3], [f32; 3], f32) = ([0.0, 2.6, -9.5], [0.0, 2.6, 0.0], 35.0);
    let looks = [
        (
            "masculine-cast-average",
            Stance::Held(HeldPose::HipFlex),
            BACK,
            [
                ("the notch", 995.0, 585.0),
                ("cloth beside it", 1005.0, 585.0),
            ],
        ),
        (
            "feminine-cast-heavyset",
            Stance::Held(HeldPose::HipFlex),
            BACK,
            [
                ("the notch", 1000.0, 595.0),
                ("cloth beside it", 1012.0, 595.0),
            ],
        ),
        (
            "masculine-cast-muscular",
            Stance::Clip {
                name: "Walk",
                frames: 48,
            },
            FRONT,
            [
                ("the hole, black", 980.0, 592.0),
                ("the hole, skin", 986.0, 598.0),
            ],
        ),
    ];
    for (name, stance, view, pixels) in looks {
        let worn_by = dressed(&named(name));
        for (what, x, y) in pixels {
            println!("{name} {}, {what} at ({x}, {y}):", pose_name(stance));
            let (hits, studs) = seen_at(&stage, &worn_by, stance, view, (1920.0, 1080.0), (x, y));
            let first = hits.first().map_or(0.0, |h| h.0);
            for (distance, part, material, facing, rest) in hits.iter().take(6) {
                println!(
                    "  {distance:.4} studs (+{:.1} mm): {part} {material} {}, rest middle {}",
                    (distance - first) / studs * 1000.0,
                    if *facing {
                        "facing the camera (drawn)"
                    } else {
                        "facing away (culled)"
                    },
                    round(*rest)
                );
            }
        }
    }
}
