//! A garment's pose correctives baked from cloth simulation (wave 5,
//! `refinement/wave5/pose-correctives.md`): the rules a garment on the
//! realistic body gets, the runs the bake simulates, and the solve that
//! turns the simulated cloth into shapes.

use std::path::{Path, PathBuf};

use axcenity_assets::{Assets, BoneTransform, Rig};
use axcenity_avatar::PoseCorrective;
use axcenity_avatar::fit::{
    BodyMesh, CLOTH_PREFIX, CORRECTIVE_RULES, ClothRead, ClothRun, FitError, MOST_MOVE, Solving,
    cloth_rules, cloth_runs, corrective_weights, rule_weight, skin, skin_matrices, solve_with,
};
use glam::{Quat, Vec3};

const BODY: &str = "realistic/RealisticParts.glb#AxcenityRealistic/Body";

fn shipped() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn finish_pose(rig: &Rig, pose: &mut Vec<BoneTransform>) {
    axcenity_anim::twist::drive(rig, pose, &mut []);
}

fn body_and(object: &str) -> (BodyMesh, BodyMesh) {
    let assets = Assets::new(shipped());
    let body = BodyMesh::from_model(&assets.model_blocking(BODY).unwrap()).unwrap();
    let item = assets
        .model_blocking(&format!(
            "realistic/RealisticParts.glb#AxcenityRealistic/{object}"
        ))
        .unwrap();
    (body, BodyMesh::from_model(&item).unwrap())
}

fn rule<'a>(rules: &'a [PoseCorrective], shape: &str) -> &'a PoseCorrective {
    rules
        .iter()
        .find(|r| r.shape == shape)
        .unwrap_or_else(|| panic!("no rule {shape}"))
}

/// `rule`'s bone turned by `degrees` about its axis from rest.
fn turned(rig: &Rig, rule: &PoseCorrective, degrees: f32) -> Vec<BoneTransform> {
    let mut bones = rig.rest_pose();
    let bone = rig.bone(&rule.bone).unwrap();
    let axis = Vec3::from(rule.axis).normalize();
    bones[bone].rotation = Quat::from_axis_angle(axis, degrees.to_radians()) * bones[bone].rotation;
    bones
}

/// The garment rules: the body's ten shoulder rules under cloth names,
/// and each side's elbow, thigh (forward and back) and knee, each full at
/// its full angle and turning its limb the way its name says.
#[test]
fn a_garment_gets_the_bodys_shoulder_rules_and_its_limbs_bends() {
    let (body, _) = body_and("Underwear_Bra");
    let rig = &body.rig;
    let rules = cloth_rules(rig);
    // The four swings are split in two halves.
    assert_eq!(rules.len(), CORRECTIVE_RULES.len() + 4 + 8);
    for (shape, bone, axis, start, full) in CORRECTIVE_RULES {
        let ours = rule(&rules, &shape.replacen("Corrective_", CLOTH_PREFIX, 1));
        assert_eq!((ours.bone.as_str(), ours.axis), (bone, axis));
        if shape.contains("_Forward_") || shape.contains("_Back_") {
            let half = (start + full) / 2.0;
            assert_eq!(ours.angles, [half, full]);
            let first = rule(
                &rules,
                &ours
                    .shape
                    .replacen("_Forward_", "_ForwardHalf_", 1)
                    .replacen("_Back_", "_BackHalf_", 1),
            );
            assert_eq!((first.bone.as_str(), first.axis), (bone, axis));
            assert_eq!(first.angles, [start, half]);
        } else {
            assert_eq!(ours.angles, [start, full]);
        }
    }
    let world_head = |bones: &[BoneTransform], bone: &str| {
        rig.world(bones)[rig.bone(bone).unwrap()].w_axis.truncate()
    };
    for side in ["Left", "Right"] {
        // (shape, the bone whose head should move, which way: +1 forward,
        // -1 back.)
        for (kind, tip, way) in [
            ("Elbow_Bent", "Hand", 1.0),
            ("Hip_Forward", "LowerLeg", 1.0),
            ("Hip_Back", "LowerLeg", -1.0),
            ("Knee_Bent", "Foot", -1.0),
        ] {
            let ours = rule(&rules, &format!("{CLOTH_PREFIX}{kind}_{side}"));
            assert!(
                (Vec3::from(ours.axis).length() - 1.0).abs() < 1e-3,
                "{}",
                ours.shape
            );
            let full = ours.angles[1];
            let bones = turned(rig, ours, full);
            assert!((rule_weight(rig, &bones, ours) - 1.0).abs() < 1e-3);
            assert!(rule_weight(rig, &turned(rig, ours, -full), ours) < 1e-3);
            let tip = format!("{side}{tip}");
            let moved = world_head(&bones, &tip).z - world_head(&rig.rest_pose(), &tip).z;
            assert!(moved * way > 0.03, "{}: {tip} moved {moved} m", ours.shape);
        }
    }
}

/// The runs start at rest and read it there; each key pose leans its
/// solved shapes fully and leans no shape that isn't solved there or
/// before (so each shape is solved on top of only what it should be);
/// every read is at the end of a hold.
#[test]
fn each_run_solves_its_shapes_at_full_weight_on_top_of_those_before() {
    let (body, _) = body_and("Underwear_Bra");
    let rig = &body.rig;
    let rules = cloth_rules(rig);
    let runs = cloth_runs(rig, &rules, Some(finish_pose));
    let names: Vec<&str> = runs.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "hang",
            "swing-left",
            "swing-right",
            "level",
            "raised",
            "elbows",
            "hips-left",
            "hips-right",
            "knees"
        ]
    );
    let mut solved: Vec<String> = Vec::new();
    for run in &runs {
        assert!(
            matches!(run.reads[0], ClothRead::Rest { .. }),
            "{}",
            run.name
        );
        for read in &run.reads {
            let frame = read.frame();
            assert!(frame > 0 && frame < run.frames.len(), "{}", run.name);
            let (a, b) = (&run.frames[frame - 1], &run.frames[frame]);
            assert!(
                a.iter()
                    .zip(b)
                    .all(|(a, b)| a.rotation.dot(b.rotation).abs() > 1.0 - 1e-6),
                "{}: read at {frame} while moving: {:?}",
                run.name,
                a.iter()
                    .zip(b)
                    .enumerate()
                    .filter(|(_, (a, b))| a.rotation.dot(b.rotation).abs() <= 1.0 - 1e-6)
                    .map(|(k, (a, b))| (rig.bones[k].name.clone(), a.rotation, b.rotation))
                    .collect::<Vec<_>>()
            );
            let leaning = |shape: &str| rule_weight(rig, b, rule(&rules, shape));
            match read {
                ClothRead::Rest { .. } => {
                    for r in &rules {
                        assert!(leaning(&r.shape) < 1e-4, "{}: {}", run.name, r.shape);
                    }
                }
                ClothRead::Check { .. } => {}
                ClothRead::Solve { shapes, .. } => {
                    assert!(!shapes.is_empty());
                    for shape in shapes {
                        assert!(leaning(shape) > 0.99, "{}: {shape}", run.name);
                    }
                    for r in &rules {
                        let known = shapes.contains(&r.shape) || solved.contains(&r.shape);
                        assert!(
                            known || leaning(&r.shape) < 0.01,
                            "{}: {} leans {}",
                            run.name,
                            r.shape,
                            leaning(&r.shape)
                        );
                    }
                    solved.extend(shapes.iter().cloned());
                }
            }
        }
    }
    let mut every: Vec<String> = rules.iter().map(|r| r.shape.clone()).collect();
    every.sort();
    solved.sort();
    assert_eq!(solved, every, "every rule is solved once");
}

/// The garment as the runtime poses it in `bones` with cloth shapes
/// `solved` (by rule), and each corner's linear part.
fn skinned(
    garment: &BodyMesh,
    rules: &[PoseCorrective],
    bones: &[BoneTransform],
    solved: &[(&str, Vec<Vec3>)],
) -> (Vec<Vec3>, Vec<glam::Mat3>) {
    let (mut rest, _) = garment.shaped(&corrective_weights(&garment.rig, bones));
    for (shape, offsets) in solved {
        let w = rule_weight(&garment.rig, bones, rule(rules, shape));
        for (p, o) in rest.iter_mut().zip(offsets) {
            *p += *o * w;
        }
    }
    let matrices = skin_matrices(&garment.rig, bones);
    let empty = vec![Vec3::ZERO; rest.len()];
    let (posed, _, linear) = skin(&matrices, &garment.joints, &garment.weights, &rest, &empty);
    (posed, linear)
}

/// The bake's first run (the arms let down to hang) on the bra, solved
/// from a simulation that moves the cloth as skinning does, but everywhere
/// by a settle `drift` at rest, by `moved(rest)` at the key pose and by
/// `at_check(w) * moved(rest)` half way, where `w` is the Lowered rules'
/// weight there (all taken through each corner's skinning), solved as
/// `solving` says (`Solving::SIDECAR` as the committed bake). Returns the
/// shapes and `w`.
fn hang_bake(
    garment: &BodyMesh,
    rules: &[PoseCorrective],
    rig: &Rig,
    moved: &dyn Fn(Vec3) -> Vec3,
    at_check: &dyn Fn(f32) -> f32,
    solving: Solving,
) -> (axcenity_avatar::fit::PoseShapes, f32) {
    let runs: Vec<ClothRun> = cloth_runs(rig, rules, Some(finish_pose))
        .into_iter()
        .take(1)
        .collect();
    let lowered = rule(rules, "Corrective_Cloth_Shoulder_Lowered_Left");
    let check_weight = runs[0]
        .reads
        .iter()
        .find(|read| matches!(read, ClothRead::Check { .. }))
        .map(|read| rule_weight(rig, &runs[0].frames[read.frame()], lowered))
        .unwrap();
    let drift = |p: Vec3| Vec3::new(0.0, -0.001, 0.0) * (p.y - 1.0).abs();
    let simulated = |r: usize, k: usize| -> Result<Vec<Vec3>, FitError> {
        let run = &runs[r];
        let read = &run.reads[k];
        let bones = &run.frames[read.frame()];
        let (posed, linear) = skinned(garment, rules, bones, &[]);
        let share = match read {
            ClothRead::Rest { .. } => 0.0,
            ClothRead::Check { .. } => at_check(check_weight),
            ClothRead::Solve { .. } => 1.0,
        };
        Ok(posed
            .iter()
            .zip(&linear)
            .zip(&garment.positions)
            .map(|((p, l), rest)| *p + *l * (drift(*rest) + moved(*rest) * share))
            .collect())
    };
    (
        solve_with(garment, rules, &runs, &simulated, solving).unwrap(),
        check_weight,
    )
}

/// The half-way check's figures (median, most; mm).
fn check_mm(shapes: &axcenity_avatar::fit::PoseShapes) -> (f64, f64) {
    report_mm(shapes, "checks")
}

/// The one entry of the report's `list` (median, most; mm).
fn report_mm(shapes: &axcenity_avatar::fit::PoseShapes, list: &str) -> (f64, f64) {
    let entries = shapes.report[list]
        .as_array()
        .unwrap_or_else(|| panic!("the report has no `{list}`: {}", shapes.report));
    assert_eq!(entries.len(), 1, "{list}");
    let mm = |key: &str| entries[0][key].as_f64().unwrap();
    (mm("median_mm"), mm("most_mm"))
}

/// A simulation that moves all the cloth by `moved` at the hanging arms'
/// key pose (and by a settle drift at rest, everywhere): the solve gives
/// `moved` back split between the two sides' Lowered shapes (each the
/// whole of it well on its own side, none well on the other, the two
/// adding up to it everywhere), and takes the drift off. Half way the
/// simulation doesn't move, so the check finds the blend (the shapes at
/// the rules' weight there) off by that weight's share of `moved`.
#[test]
fn a_simulated_move_comes_back_as_its_poses_shapes() {
    let (body, garment) = body_and("Underwear_Bra");
    let rig = &body.rig;
    let rules = cloth_rules(rig);
    let moved = Vec3::new(0.0, 0.002, 0.004);
    let (shapes, w) = hang_bake(
        &garment,
        &rules,
        rig,
        &|_| moved,
        &|_| 0.0,
        Solving::SIDECAR,
    );
    let names: Vec<&str> = shapes.shapes.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Corrective_Cloth_Shoulder_Lowered_Left",
            "Corrective_Cloth_Shoulder_Lowered_Right"
        ]
    );
    let count = garment.positions.len();
    let (left, right) = (shapes.shapes[0].dense(count), shapes.shapes[1].dense(count));
    for ((l, r), rest) in left.iter().zip(&right).zip(&garment.positions) {
        // (Within what the file keeps: a corner moved under 0.05 mm is left out.)
        assert!((*l + *r).distance(moved) < 6e-5, "at {rest}: {l} + {r}");
        if rest.x > 0.12 {
            assert!(l.distance(moved) < 2e-4, "at {rest}: {l}");
        }
        if rest.x < -0.12 {
            assert!(r.distance(moved) < 2e-4, "at {rest}: {r}");
        }
    }
    assert_eq!(shapes.kept_rules().len(), 2);
    // Half way the rule leans part way (smoothstep at mid-range: a half).
    assert!((w - 0.5).abs() < 0.01, "the check's weight {w}");
    let expected = f64::from(w * moved.length() * 1000.0);
    let (median, most) = check_mm(&shapes);
    assert!(
        (median - expected).abs() < 0.05 && (most - expected).abs() < 0.1,
        "the blend is off by {expected:.3} mm half way; the check says median {median:.4}, most {most:.4}"
    );
}

/// A simulation whose cloth half way has moved by the rules' weight there
/// times the key pose's move is what the blend of the solved shapes gives:
/// the check finds it on the cloth (about 0 mm).
#[test]
fn the_half_way_check_measures_the_blend_of_the_solved_shapes() {
    let (body, garment) = body_and("Underwear_Bra");
    let rig = &body.rig;
    let rules = cloth_rules(rig);
    let moved = Vec3::new(0.0, 0.002, 0.004);
    let (shapes, w) = hang_bake(&garment, &rules, rig, &|_| moved, &|w| w, Solving::SIDECAR);
    let (median, most) = check_mm(&shapes);
    assert!(
        median < 0.02 && most < 0.1,
        "the cloth moved {w} of the key pose's move half way, as the blend does; the check says median {median:.4}, most {most:.4} mm"
    );
}

/// Beside each half-way check the report gives the key pose's own: the
/// garment posed with every solved shape at its weight there, against the
/// simulation. A move under the hold comes back whole, so nothing is left
/// at the key pose.
#[test]
fn the_report_measures_what_the_shapes_leave_at_each_key_pose() {
    let (body, garment) = body_and("Underwear_Bra");
    let rig = &body.rig;
    let rules = cloth_rules(rig);
    let moved = Vec3::new(0.0, 0.002, 0.004);
    let (shapes, _) = hang_bake(&garment, &rules, rig, &|_| moved, &|w| w, Solving::SIDECAR);
    let (median, most) = report_mm(&shapes, "keys");
    assert!(
        median < 0.02 && most < 0.1,
        "a 4.47 mm move is solved whole; the key pose's check says median {median:.4}, most {most:.4} mm"
    );
}

/// A move larger than the hold (22.4 mm against 6 mm): held, the key pose
/// keeps the rest (16.4 mm) and a linear blend half way keeps about half
/// of it. Solved whole (no hold, for measuring only), the key pose is met
/// and a simulation that moves by the rule's weight half way is met too;
/// one that doesn't move half way is off by the weight's share of the
/// whole move. So the whole-move solve separates what blending loses from
/// what the hold loses. (A uniform move: smoothing keeps it.)
#[test]
fn a_whole_move_solve_separates_the_blend_from_the_hold() {
    let (body, garment) = body_and("Underwear_Bra");
    let rig = &body.rig;
    let rules = cloth_rules(rig);
    let moved = Vec3::new(0.0, 0.01, 0.02);
    let whole = f64::from(moved.length() * 1000.0);
    let held = whole - f64::from(MOST_MOVE * 1000.0);
    let near = |got: f64, want: f64, slack: f64, what: &str| {
        assert!(
            (got - want).abs() < slack,
            "{what}: {got:.3} mm, expected {want:.3}"
        );
    };

    let (shapes, w) = hang_bake(&garment, &rules, rig, &|_| moved, &|w| w, Solving::SIDECAR);
    let w = f64::from(w);
    let (median, most) = report_mm(&shapes, "keys");
    near(median, held, 0.2, "held, key pose median");
    near(most, held, 0.3, "held, key pose most");
    let (median, _) = check_mm(&shapes);
    near(median, w * held, 0.2, "held, half way median");

    let (shapes, _) = hang_bake(
        &garment,
        &rules,
        rig,
        &|_| moved,
        &|w| w,
        Solving::whole(Solving::SIDECAR.smooth_rounds),
    );
    let (median, most) = report_mm(&shapes, "keys");
    assert!(
        median < 0.05 && most < 0.2,
        "whole, key pose: {median:.4} / {most:.4} mm"
    );
    let (median, most) = check_mm(&shapes);
    assert!(
        median < 0.05 && most < 0.2,
        "whole, half way: {median:.4} / {most:.4} mm"
    );

    let (shapes, _) = hang_bake(
        &garment,
        &rules,
        rig,
        &|_| moved,
        &|_| 0.0,
        Solving::whole(Solving::SIDECAR.smooth_rounds),
    );
    let (median, most) = check_mm(&shapes);
    near(median, w * whole, 0.1, "whole, still half way median");
    near(most, w * whole, 0.2, "whole, still half way most");
}

/// A move crumpled at a few millimetres (a checkerboard of 5 mm cells, up
/// and down by 4 mm, under the hold): the sidecar's smoothing irons it
/// out, so the key pose is left about 4 mm off; solved whole and
/// unsmoothed (for measuring only) the key pose is met exactly, so what
/// is left half way is what blending alone loses.
#[test]
fn an_unsmoothed_whole_solve_meets_each_key_pose() {
    let (body, garment) = body_and("Underwear_Bra");
    let rig = &body.rig;
    let rules = cloth_rules(rig);
    let crumpled = |rest: Vec3| {
        let cell = (rest.x * 200.0).floor() as i32 + (rest.y * 200.0).floor() as i32;
        Vec3::new(
            0.0,
            if cell.rem_euclid(2) == 0 {
                0.004
            } else {
                -0.004
            },
            0.0,
        )
    };
    let (shapes, _) = hang_bake(&garment, &rules, rig, &crumpled, &|w| w, Solving::SIDECAR);
    let (median, _) = report_mm(&shapes, "keys");
    assert!(median > 2.0, "smoothed, the key pose keeps {median:.3} mm");
    let (shapes, _) = hang_bake(&garment, &rules, rig, &crumpled, &|w| w, Solving::whole(0));
    let (median, most) = report_mm(&shapes, "keys");
    assert!(
        median < 0.02 && most < 0.1,
        "whole and unsmoothed, the key pose keeps {median:.4} / {most:.4} mm"
    );
    let (median, most) = check_mm(&shapes);
    assert!(
        median < 0.02 && most < 0.1,
        "and half way a cloth that moves by the weight is met: {median:.4} / {most:.4} mm"
    );
}
