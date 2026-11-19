//! A garment's pose correctives baked from cloth simulation (wave 5,
//! `refinement/wave5/pose-correctives.md`): the rules a garment on the
//! realistic body gets, the runs the bake simulates, and the solve that
//! turns the simulated cloth into shapes.

use std::path::{Path, PathBuf};

use axcenity_assets::{Assets, BoneTransform, Rig};
use axcenity_avatar::PoseCorrective;
use axcenity_avatar::fit::{
    BodyMesh, CLOTH_PREFIX, CORRECTIVE_RULES, ClothRead, ClothRun, FitError, cloth_rules,
    cloth_runs, corrective_weights, rule_weight, skin, skin_matrices, solve,
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

/// A simulation that moves the cloth as skinning does, but at the hanging
/// arms' key pose moves all of it by `moved` (taken through each corner's
/// skinning), and everywhere by a settle `drift` at rest: the solve gives
/// `moved` back split between the two sides' Lowered shapes (each the
/// whole of it well on its own side, none well on the other, the two
/// adding up to it everywhere), and takes the drift off.
#[test]
fn a_simulated_move_comes_back_as_its_poses_shapes() {
    let (body, garment) = body_and("Underwear_Bra");
    let rig = &body.rig;
    let rules = cloth_rules(rig);
    let runs: Vec<ClothRun> = cloth_runs(rig, &rules, Some(finish_pose))
        .into_iter()
        .take(1)
        .collect();
    let moved = Vec3::new(0.0, 0.002, 0.004);
    let drift = |p: Vec3| Vec3::new(0.0, -0.001, 0.0) * (p.y - 1.0).abs();
    let simulated = |r: usize, k: usize| -> Result<Vec<Vec3>, FitError> {
        let run = &runs[r];
        let read = &run.reads[k];
        let bones = &run.frames[read.frame()];
        let (posed, linear) = skinned(&garment, &rules, bones, &[]);
        let solving = matches!(read, ClothRead::Solve { .. });
        Ok(posed
            .iter()
            .zip(&linear)
            .zip(&garment.positions)
            .map(|((p, l), rest)| {
                let mut back = drift(*rest);
                if solving {
                    back += moved;
                }
                *p + *l * back
            })
            .collect())
    };
    let shapes = solve(&garment, &rules, &runs, &simulated).unwrap();
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
    // The half-way check measured how far the blend is from the cloth.
    let checks = shapes.report["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 1);
    assert!(checks[0]["most_mm"].as_f64().unwrap() > 0.0);
}
