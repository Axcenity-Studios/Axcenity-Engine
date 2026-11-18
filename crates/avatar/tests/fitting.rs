//! Garment fitting on the realistic body (Character Studio CO9, ADR-0035,
//! ADR-0041 decision 4): the shipped underwear already carries what the
//! transfer gives; a garment not made from the body (the test tee,
//! `fixtures/test-tee`) imports into a library, carries every identity and
//! corrective shape, is weighted to the body and stays outside it and the
//! sports bra at every extreme.

use std::path::{Path, PathBuf};

use axcenity_assets::Assets;
use axcenity_avatar::fit::{
    BodyMesh, CORRECTIVE_RULES, FitOptions, GarmentMesh, ImportRequest, Surface, align,
    carried_shapes, extremes_of, fit_garment, fit_poses, import_garment, pose_states_of, skin,
    skin_matrices,
};
use axcenity_avatar::{
    Catalogue, CatalogueBase, CatalogueItem, Fit, Library, ModelContents, Quality, Source,
};
use glam::Vec3;

const BODY: &str = "realistic/RealisticParts.glb#AxcenityRealistic/Body";

fn shipped() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn tee() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test-tee/TestTee.obj")
}

/// A scratch folder, removed when dropped (also when a test fails).
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<std::ffi::OsStr> for Scratch {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_os_str()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A library of its own in a scratch folder: the realistic base and its
/// underwear, nothing else.
fn scratch_library(name: &str) -> Scratch {
    let root = std::env::temp_dir().join(format!("axcenity-fit-{name}-{}", std::process::id()));
    let (library, _, problems) = Library::read(&shipped());
    assert!(problems.is_empty(), "{problems:?}");
    let base = library.base("axcenity/realistic").unwrap();
    let base_path = library.manifest_path("axcenity/realistic").unwrap();
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
    for file in files {
        let target = root.join(&file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(shipped().join(&file), &target).unwrap();
    }
    std::fs::write(root.join("catalogue.json"), catalogue.to_json()).unwrap();
    Scratch(root)
}

fn source() -> Source {
    Source {
        author: "Axcenity Engine contributors".to_owned(),
        licence: "CC0-1.0".to_owned(),
        ..Source::default()
    }
}

fn request(root: &Path, id: &str, under: &[&str]) -> ImportRequest {
    ImportRequest {
        source: tee(),
        library: root.to_owned(),
        base: "axcenity/realistic".to_owned(),
        id: id.to_owned(),
        label: "Test tee".to_owned(),
        folder: "imported/test-tee".to_owned(),
        source_info: source(),
        slot: None,
        layer: None,
        margin: 0.002,
        under: under.iter().map(|s| (*s).to_owned()).collect(),
        about: Some("A test garment for the import flow (a draft).".to_owned()),
        finish_pose: Some(finish_pose),
    }
}

/// The runtime's last touch to a pose: the twist and helper bones.
fn finish_pose(rig: &axcenity_assets::Rig, pose: &mut Vec<axcenity_assets::BoneTransform>) {
    axcenity_anim::twist::drive(rig, pose, &mut []);
}

fn body_and(assets: &Assets, object: &str) -> (BodyMesh, BodyMesh) {
    let body = BodyMesh::from_model(&assets.model_blocking(BODY).unwrap()).unwrap();
    let item = assets
        .model_blocking(&format!(
            "realistic/RealisticParts.glb#AxcenityRealistic/{object}"
        ))
        .unwrap();
    (body, BodyMesh::from_model(&item).unwrap())
}

/// The underwear is cut from the body, so where its cloth lies on the
/// skin each of its shapes is the body's own mix, and binding it and
/// carrying the body's shapes gives the same shapes. Where it bridges a
/// hollow (the gusset between the thighs, the bra between the breasts on
/// the feminine frame) its own shapes were made on each frame and are
/// right; a transfer by binding would not be (ADR-0035: binding errs
/// with distance from the skin). So it keeps its own shapes (`fit:
/// shapes`), and this measures how closely the transfer agrees.
#[test]
fn the_realistic_underwear_already_carries_what_the_transfer_gives() {
    let assets = Assets::new(shipped());
    for object in ["Underwear_BoxerBriefs", "Underwear_Briefs", "Underwear_Bra"] {
        let (body, item) = body_and(&assets, object);
        let model = assets
            .model_blocking(&format!(
                "realistic/RealisticParts.glb#AxcenityRealistic/{object}"
            ))
            .unwrap();
        let garment = GarmentMesh::from_model(&model);
        // No push-out: only what the binding carries.
        let options = FitOptions {
            margin: -1.0,
            shapes: item.shapes.iter().map(|s| s.name.clone()).collect(),
            rounds: 1,
            ..FitOptions::default()
        };
        let fitted = fit_garment(&body, &garment, &options).unwrap();
        let mut frame = Vec::new();
        let mut others = Vec::new();
        for (own, carried) in item.shapes.iter().zip(&fitted.shapes) {
            assert_eq!(own.name, carried.name);
            let dense = own.dense(item.positions.len());
            let errors = dense
                .iter()
                .zip(&carried.offsets)
                .map(|(a, b)| a.distance(*b));
            if own.name == "Frame_Feminine" {
                frame.extend(errors);
            } else {
                others.extend(errors);
            }
        }
        let share = |errors: &[f32], within: f32| {
            errors.iter().filter(|e| **e <= within).count() as f32 / errors.len().max(1) as f32
        };
        let worst = |errors: &[f32]| errors.iter().copied().fold(0.0, f32::max);
        println!(
            "{object}: {} shapes; the build and composition shapes {:.2}% within 1 mm (worst {:.2} mm); Frame_Feminine {:.1}% within 2 mm (worst {:.1} mm, at the bridges)",
            item.shapes.len(),
            100.0 * share(&others, 0.001),
            1000.0 * worst(&others),
            100.0 * share(&frame, 0.002),
            1000.0 * worst(&frame),
        );
        assert!(share(&others, 0.001) > 0.85, "{object}");

        assert!(share(&frame, 0.002) > 0.8, "{object}");
    }
}

#[test]
fn a_garment_not_made_from_the_body_imports_as_a_fitted_draft() {
    let root = scratch_library("import");
    let imported = import_garment(&request(&root, "imported/test-tee", &[])).unwrap();
    let manifest = &imported.manifest;
    println!("{:#?}", imported.report);
    println!("{:?}", imported.coverage);
    println!("{:?}", imported.alignment);
    assert_eq!(manifest.slot, "top");
    assert_eq!(manifest.quality, Some(Quality::Draft));
    assert!(manifest.is_draft());
    assert!(matches!(manifest.fit, Fit::Binding { .. }));
    // It hides what it covers whole (the waist) and nothing it covers
    // only in part (the torso region runs up the neck, past a crew neck).
    let share = |id: &str| {
        imported
            .coverage
            .iter()
            .find(|(region, _)| region == id)
            .map_or(0.0, |(_, share)| *share)
    };
    assert!(
        manifest.hides.contains(&"waist".to_owned()),
        "{:?}",
        manifest.hides
    );
    for region in &manifest.hides {
        assert!(share(region) >= 0.995, "{region}: {}", share(region));
    }
    assert!(share("torso") > 0.6, "{:?}", imported.coverage);
    // Aligned as it was made: on the body, in metres.
    assert_eq!(imported.alignment.scale, 1.0);
    assert!(imported.alignment.offset.length() < 0.02);

    // The library reads it back, with its files, and it checks against
    // its model and the body's.
    let (library, _, problems) = Library::read(&root);
    assert!(problems.is_empty(), "{problems:?}");
    let item = library.item("imported/test-tee").unwrap();
    assert!(library.validate().is_empty(), "{:?}", library.validate());
    let assets = Assets::new(&root);
    let model = assets
        .model_blocking(&format!(
            "imported/test-tee/{}#{}",
            item.model.as_ref().unwrap().file,
            item.model.as_ref().unwrap().object.as_deref().unwrap()
        ))
        .expect("the fitted model loads");
    let body_model = assets.model_blocking(BODY).unwrap();
    let problems = item.check_model(&ModelContents::of(&model), &ModelContents::of(&body_model));
    assert!(problems.is_empty(), "{problems:?}");
    assert!(root.join("imported/test-tee/TestTee.png").is_file());
    assert!(
        root.join("imported/test-tee/TestTee.binding.json")
            .is_file()
    );

    // It carries every identity and corrective shape of the body, by name.
    let body = BodyMesh::from_model(&body_model).unwrap();
    let base = library.base("axcenity/realistic").unwrap();
    let expected = carried_shapes(base, &body);
    let carried: Vec<String> = model.morph_targets.iter().map(|t| t.name.clone()).collect();
    assert_eq!(carried, expected);
    assert!(expected.iter().any(|s| s.starts_with("Corrective_")));
    assert!(expected.contains(&"Frame_Feminine".to_owned()));
    // Weighted to the body's bones, four at most, adding up to 1.
    let read = BodyMesh::from_model(&model).unwrap();
    for w in &read.weights {
        let sum: f32 = w.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4, "{w:?}");
    }
    // Nothing of it is inside the body at rest, at any shape's full weight
    // or at any extreme.
    assert!(
        imported.report.inside_most <= 1e-4,
        "{} m inside",
        imported.report.inside_most
    );
    // It was made to fit: no push-out deep enough to fold the cloth.
    let body_push = &imported.report.layers[0];
    assert!(body_push.most < 0.01, "{body_push:?}");
}

/// A layered pair: the tee imported over the sports bra stays outside it
/// by its margin at rest and at every extreme the sliders check pairs
/// (both frames; every build parameter alone and with every other one at
/// either end; the body presets).
#[test]
fn a_shirt_over_the_sports_bra_keeps_its_order_at_every_extreme() {
    let root = scratch_library("layers");
    let bra_id = "axcenity/realistic-sports-bra";
    let started = std::time::Instant::now();
    let imported = import_garment(&request(&root, "imported/test-tee", &[bra_id])).unwrap();
    println!("imported in {:.1} s", started.elapsed().as_secs_f32());
    assert!(
        imported.report.layers.iter().any(|l| l.layer == bra_id),
        "the bra is a layer"
    );
    // What the bra pushed is recorded (made with the ease a T-shirt has,
    // the tee clears the bra as it is; `fit::tests` has a layer that must
    // be pushed out over the one under it).
    println!("{:#?}", imported.report.layers);

    let (library, _, _) = Library::read(&root);
    let assets = Assets::new(&root);
    let item = library.item("imported/test-tee").unwrap();
    let model = item.model.as_ref().unwrap();
    let tee = BodyMesh::from_model(
        &assets
            .model_blocking(&format!(
                "imported/test-tee/{}#{}",
                model.file,
                model.object.as_deref().unwrap()
            ))
            .unwrap(),
    )
    .unwrap();
    let bra = BodyMesh::from_model(
        &assets
            .model_blocking("realistic/RealisticParts.glb#AxcenityRealistic/Underwear_Bra")
            .unwrap(),
    )
    .unwrap();
    let base = library.base("axcenity/realistic").unwrap();
    let mut states = extremes_of(base, &library);
    states.push(Vec::new());
    // The states side by side, a share on each thread.
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get().min(8));
    let chunk = states.len().div_ceil(threads);
    let worst = std::thread::scope(|scope| {
        let handles: Vec<_> = states
            .chunks(chunk)
            .map(|part| {
                let (tee, bra) = (&tee, &bra);
                scope.spawn(move || {
                    let mut worst = f32::INFINITY;
                    for state in part {
                        let (bra_at, bra_normals) = bra.shaped(state);
                        let surface = Surface::new(bra_at, bra_normals, bra.triangles.clone());
                        let (tee_at, _) = tee.shaped(state);
                        for p in &tee_at {
                            let Some(hit) = surface.closest(*p, 0.03) else {
                                continue;
                            };
                            if hit.is_over(*p) {
                                worst = worst.min(hit.signed(*p));
                            }
                        }
                    }
                    worst
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .fold(f32::INFINITY, f32::min)
    });
    println!(
        "{} states, the tee at least {:.2} mm over the bra ({:.1} s in all)",
        states.len(),
        worst * 1000.0,
        started.elapsed().as_secs_f32()
    );
    assert!(worst > 0.0015, "{worst} m over the bra");

    // And posed: in every pose it is fitted in (the held poses, the arms
    // hanging, a walk's swings either way round), on both frames and every
    // body preset, no corner of the bra the tee covers at rest is more
    // than 1 mm outside it. The bra and the tee are posed with their own
    // skin weights and the body's corrective weights in that pose, as the
    // runtime poses them.
    let body = BodyMesh::from_model(&assets.model_blocking(BODY).unwrap()).unwrap();
    let poses = fit_poses(&body.rig, Some(finish_pose));
    let pose_states = pose_states_of(base, &library);
    let mut failing = Vec::new();
    let mut worst_posed = 0.0f32;
    for state in &pose_states {
        let (tee_rest, tee_rest_normals) = tee.shaped(state);
        let (bra_rest, bra_rest_normals) = bra.shaped(state);
        let inner = Surface::new(tee_rest, tee_rest_normals, tee.triangles.clone());
        let covered: Vec<bool> = bra_rest
            .iter()
            .zip(&bra_rest_normals)
            .map(|(p, n)| {
                inner
                    .closest(*p, 0.02)
                    .is_some_and(|hit| hit.is_over(*p) && n.dot(hit.normal) > 0.5)
            })
            .collect();
        for pose in &poses {
            let mut weights = state.clone();
            weights.extend(pose.correctives.iter().cloned());
            let matrices = skin_matrices(&body.rig, &pose.bones);
            let (tee_at, tee_normals) = tee.shaped(&weights);
            let (tee_posed, tee_posed_normals, _) =
                skin(&matrices, &tee.joints, &tee.weights, &tee_at, &tee_normals);
            let (bra_at, bra_normals) = bra.shaped(&weights);
            let (bra_posed, bra_posed_normals, _) =
                skin(&matrices, &bra.joints, &bra.weights, &bra_at, &bra_normals);
            let cloth = Surface::new(tee_posed, tee_posed_normals, tee.triangles.clone());
            let mut out = 0;
            let mut most = 0.0f32;
            for (k, (p, n)) in bra_posed.iter().zip(&bra_posed_normals).enumerate() {
                if !covered[k] {
                    continue;
                }
                let Some(hit) = cloth.closest(*p, 0.05) else {
                    continue;
                };
                if n.dot(hit.normal) <= 0.0 {
                    continue;
                }
                let signed = hit.signed(*p);
                if signed > 0.001 {
                    out += 1;
                    most = most.max(signed);
                }
            }
            worst_posed = worst_posed.max(most);
            if out > 0 {
                failing.push(format!(
                    "{} on {state:?}: {out} bra corners out, {:.1} mm",
                    pose.name,
                    most * 1000.0
                ));
            }
        }
    }
    println!(
        "posed: {} poses on {} body states, {} failing, worst {:.1} mm",
        poses.len(),
        pose_states.len(),
        failing.len(),
        worst_posed * 1000.0
    );
    for case in &failing {
        println!("  {case}");
    }
    // Recorded, not accepted (`co9-binding.md`, Review fixes 2): with the
    // arms hanging or swung the bra's side band comes out at the tee's
    // armpit, and with them raised its straps at the neck. Lower these as
    // the fit improves; never raise them. (Wave 4: 58 cases, 19.1 mm to
    // 51, 18.6 mm, the straps off the neck.)
    assert!(
        failing.len() <= 51 && worst_posed <= 0.0187,
        "{} cases, {worst_posed} m: {failing:#?}",
        failing.len()
    );
}

/// The fit poses lean on the body's shoulder correctives as the runtime
/// does (`anim::correctives`): level arms on the level shape, raised arms
/// on both the level and the raised ones, hanging arms on the lowered.
#[test]
fn the_fit_poses_lean_on_the_shoulder_correctives() {
    let assets = Assets::new(shipped());
    let body = BodyMesh::from_model(&assets.model_blocking(BODY).unwrap()).unwrap();
    let poses = fit_poses(&body.rig, Some(finish_pose));
    let weight = |pose: &str, shape: &str| {
        poses
            .iter()
            .find(|p| p.name == pose)
            .unwrap_or_else(|| panic!("no pose {pose}"))
            .correctives
            .iter()
            .find(|(name, _)| name == shape)
            .map_or(0.0, |(_, w)| *w)
    };
    for side in ["Left", "Right"] {
        let shape = |kind: &str| format!("Corrective_Shoulder_{kind}_{side}");
        assert!((weight("t-pose", &shape("Level")) - 1.0).abs() < 0.02);
        assert!(weight("t-pose", &shape("Raised")) < 0.02);
        assert!((weight("arms-raised", &shape("Level")) - 1.0).abs() < 0.02);
        assert!((weight("arms-raised", &shape("Raised")) - 1.0).abs() < 0.02);
        assert!(weight("arms-hanging", &shape("Lowered")) > 0.9);
        assert!(weight("arms-hanging", &shape("Level")) < 0.02);
        assert!(weight("arms-hanging", &shape("Forward")) < 0.02);
        assert!(weight("arms-hanging", &shape("Back")) < 0.02);
        // The walk's swings lean on the swing correctives (the arm that is
        // ahead on Forward, the one behind on Back), on top of the lowered
        // shoulder they hang from.
        let (ahead, behind) = if side == "Left" {
            ("walk-left-40", "walk-right-40")
        } else {
            ("walk-right-40", "walk-left-40")
        };
        assert!(weight(ahead, &shape("Forward")) > 0.8, "{side}");
        assert!(weight(ahead, &shape("Back")) < 0.02, "{side}");
        assert!(weight(behind, &shape("Back")) > 0.7, "{side}");
        assert!(weight(behind, &shape("Forward")) < 0.02, "{side}");
        assert!(weight(behind, &shape("Lowered")) > 0.5, "{side}");
    }
    for pose in ["elbows-bent", "knees-bent"] {
        assert!(
            poses
                .iter()
                .find(|p| p.name == pose)
                .unwrap()
                .correctives
                .iter()
                .all(|(_, w)| *w < 0.02),
            "{pose}"
        );
    }
}

/// The corrective rules the fit poses use are the realistic build's (the
/// same numbers `anim` drives the shapes with at run time).
#[test]
fn the_corrective_rules_are_the_build_datas() {
    let text = std::fs::read_to_string(shipped().join("realistic/Realistic.build.json")).unwrap();
    let data: serde_json::Value = serde_json::from_str(&text).unwrap();
    let rules = data["correctives"].as_array().unwrap();
    assert_eq!(rules.len(), CORRECTIVE_RULES.len());
    for rule in rules {
        let shape = rule["shape"].as_str().unwrap();
        let ours = CORRECTIVE_RULES
            .iter()
            .find(|r| r.0 == shape)
            .unwrap_or_else(|| panic!("no rule for {shape}"));
        assert_eq!(ours.1, rule["bone"].as_str().unwrap());
        for (a, b) in ours.2.iter().zip(rule["axis"].as_array().unwrap()) {
            assert!((a - b.as_f64().unwrap() as f32).abs() < 1e-5, "{shape}");
        }
        let angles = rule["angles"].as_array().unwrap();
        assert!((ours.3 - angles[0].as_f64().unwrap() as f32).abs() < 1e-4);
        assert!((ours.4 - angles[1].as_f64().unwrap() as f32).abs() < 1e-4);
    }
}

/// The extremes a garment is fitted at are the sliders check's
/// (`assets/characters/humanoid/qa/measure.py`, `slider_pairs`) for the
/// body's build: on both frames, each build parameter at either end alone
/// and with every other build parameter at either end.
#[test]
fn the_extremes_are_every_pair_of_build_parameter_ends_on_both_frames() {
    let (library, _, problems) = Library::read(&shipped());
    assert!(problems.is_empty(), "{problems:?}");
    let base = library.base("axcenity/realistic").unwrap();
    let extremes = extremes_of(base, &library);
    let build: Vec<&axcenity_avatar::Parameter> = base
        .parameters
        .iter()
        .filter(|p| matches!(p.group.as_str(), "build" | "build_regions"))
        .collect();
    assert_eq!(build.len(), 18);
    let state = |values: &[(&str, f32)]| {
        let mut description = axcenity_avatar::Description::new("extreme", &base.id);
        for (id, value) in values {
            description.parameters.insert((*id).to_owned(), *value);
        }
        axcenity_avatar::body_shape_weights(base, &description, &library)
            .into_iter()
            .filter(|(name, weight)| *weight != 0.0 && !name.starts_with("Mask_"))
            .collect::<Vec<(String, f32)>>()
    };
    for frame in [0.0, 1.0] {
        for (i, a) in build.iter().enumerate() {
            for b in &build[i + 1..] {
                for ea in a.range {
                    for eb in b.range {
                        let wanted =
                            state(&[("frame", frame), (a.id.as_str(), ea), (b.id.as_str(), eb)]);
                        assert!(
                            extremes.contains(&wanted),
                            "frame {frame}, {}={ea}, {}={eb}",
                            a.id,
                            b.id
                        );
                    }
                }
            }
        }
    }
    // 18 parameters: 153 pairs at 4 combinations of ends and 36 ends
    // alone, on each frame, and the presets (some coincide).
    println!("{} extremes", extremes.len());
    assert!(
        extremes.len() >= 2 * (153 * 4 + 36) - 20,
        "{}",
        extremes.len()
    );
}

/// A garment made too high and too small (a third-party file made for a
/// slightly different body) is put back where it fits: its scale and
/// height found from how evenly it stands off the body. On the wave-3 test
/// tee (`TestTeeEvenEase.obj`), which stands off the body by about the
/// same ease everywhere, as the fine fit assumes: the wave-4 tee lies on
/// the shoulders and its collar on the skin, and is only 13% more even at
/// its true place than 5% small and 3 cm high, under the 20% and 2 mm the
/// fine fit needs to move a garment (a limitation, `co9-fit-2.md`).
#[test]
fn alignment_puts_a_garment_made_too_high_and_too_small_back_on_the_body() {
    let assets = Assets::new(shipped());
    let body = BodyMesh::from_model(&assets.model_blocking(BODY).unwrap()).unwrap();
    let even =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test-tee/TestTeeEvenEase.obj");
    let source = axcenity_assets::load_model_file(&even).unwrap();
    let mut made = GarmentMesh::from_model(&source);
    let as_made = align(&mut made, &body);
    println!("as made: {as_made:?}");
    // Made on the body: next to nothing changes.
    assert!((as_made.scale - 1.0).abs() < 0.011, "{as_made:?}");
    assert!(as_made.offset.length() < 0.01, "{as_made:?}");
    let (lo, hi) = made.bounds();
    let centre = (lo + hi) / 2.0;
    let mut wrong = made.clone();
    // 0.95 of its size about its middle, and 3 cm too high.
    wrong.transform(0.95, centre * 0.05 + Vec3::Y * 0.03);
    let back = align(&mut wrong, &body);
    println!("back: {back:?}");
    // Within 2.5%: the tee's ease is a little more over the torso than
    // round the sleeves, and a size a little bigger stands off more evenly
    // (the fine fit can't tell a bigger size from a more even ease).
    assert!((back.scale - 1.0 / 0.95).abs() < 0.025, "{back:?}");
    let errors: Vec<f32> = wrong
        .positions()
        .iter()
        .zip(made.positions())
        .map(|(a, b)| a.distance(b))
        .collect();
    let mean = errors.iter().sum::<f32>() / errors.len() as f32;
    let worst = errors.iter().copied().fold(0.0, f32::max);
    println!(
        "put back within {:.1} mm on average, {:.1} mm at worst",
        mean * 1000.0,
        worst * 1000.0
    );
    assert!(mean < 0.008 && worst < 0.015, "{mean} m, {worst} m");
}

/// The sports bra's straps sit where a sports bra's do: over the top of
/// the shoulder, between the neck and the shoulder joint, not up the side
/// of the neck (where they came up to 1.50 m and made a T-shirt's crew
/// neck close round the neck to cover them). On both frames: no bra
/// corner lies over skin weighted more than a fifth to the neck or head,
/// and near its top each strap is at least 1.5 cm out from the neck's
/// side.
#[test]
fn the_sports_bras_straps_sit_on_the_shoulders_not_the_neck() {
    let assets = Assets::new(shipped());
    let (body, bra) = body_and(&assets, "Underwear_Bra");
    let rig = &body.rig;
    // Each body corner's weight to the neck and head.
    let neck_weight: Vec<f32> = body
        .joints
        .iter()
        .zip(&body.weights)
        .map(|(joints, weights)| {
            joints
                .iter()
                .zip(weights)
                .filter(|(j, _)| {
                    let bone = &rig.bones[rig.joints[**j as usize].bone].name;
                    bone == "Neck" || bone == "Head"
                })
                .map(|(_, w)| *w)
                .sum()
        })
        .collect();
    for (frame, weights) in [
        ("masculine", vec![]),
        ("feminine", vec![("Frame_Feminine".to_owned(), 1.0)]),
    ] {
        let (skin, _) = body.shaped(&weights);
        let (cloth, _) = bra.shaped(&weights);
        let on_neck = |p: Vec3| {
            let nearest = skin
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.distance_squared(p).total_cmp(&b.1.distance_squared(p)))
                .unwrap()
                .0;
            neck_weight[nearest]
        };
        let worst = cloth.iter().map(|p| on_neck(*p)).fold(0.0f32, f32::max);
        assert!(
            worst <= 0.2,
            "{frame}: a bra corner over skin {worst:.2} weighted to the neck or head"
        );
        let top = cloth.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        // The neck's half width a little below the top of the straps, and
        // the straps' inner edge there.
        let at = top - 0.01;
        let neck_half = skin
            .iter()
            .zip(&neck_weight)
            .filter(|(p, w)| **w > 0.5 && (p.y - at).abs() < 0.006)
            .map(|(p, _)| p.x.abs())
            .fold(0.0f32, f32::max);
        let strap_inner = cloth
            .iter()
            .filter(|p| (p.y - at).abs() < 0.006)
            .map(|p| p.x.abs())
            .fold(f32::MAX, f32::min);
        assert!(
            strap_inner >= neck_half + 0.015,
            "{frame}: the straps' inner edge at {strap_inner:.3} m from the middle, the neck {neck_half:.3} m"
        );
    }
}

/// How far each corner of `positions` stands out of the surface its
/// neighbours make (along its normal, metres; negative: sunk into it).
/// Corners at one place at rest (`welds`: a seam split for its texture)
/// are one, with the neighbours of all of them.
fn bulge(
    positions: &[Vec3],
    normals: &[Vec3],
    triangles: &[[u32; 3]],
    welds: &[usize],
) -> Vec<f32> {
    let mut sum = vec![Vec3::ZERO; positions.len()];
    let mut count = vec![0.0f32; positions.len()];
    for t in triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (a, b) = (welds[a as usize], welds[b as usize]);
            sum[a] += positions[b];
            count[a] += 1.0;
            sum[b] += positions[a];
            count[b] += 1.0;
        }
    }
    (0..positions.len())
        .map(|k| {
            let w = welds[k];
            if count[w] > 0.0 {
                (positions[w] - sum[w] / count[w]).dot(normals[w])
            } else {
                0.0
            }
        })
        .collect()
}

/// Each corner's first corner at the same place.
fn welds_of(positions: &[Vec3]) -> Vec<usize> {
    let mut first = std::collections::HashMap::new();
    positions
        .iter()
        .enumerate()
        .map(|(k, p)| {
            *first
                .entry((*p * 1e5).round().as_ivec3().to_array())
                .or_insert(k)
        })
        .collect()
}

/// The walk's folds at the shoulder (CO9 part 2): in the fit's walk poses
/// at their widest (one arm swung forward 30 degrees, the other back 48,
/// as the starter Walk swings them), the skin round each shoulder (within
/// 15 cm of the joint, below its top) folds where the arm meets the chest
/// or the back: a corner stands out of, or sinks into, the surface its
/// neighbours make, more than it does at rest. The swing correctives
/// (`Corrective_Shoulder_Forward_*`, `_Back_*`, made on the masculine
/// frame) take that fold out over the region: on the masculine frame the
/// 95th percentile of its depth is at most 60% of what it is without them
/// (measured: 10.6 mm to 4.9 and 5.9).
///
/// The rest is held at what the build reaches, as records of open defects
/// (`refinement/wave4/co9-fit-2.md`, review fixes 1), each a ceiling to
/// lower and never raise: the feminine frame folds less and the shapes
/// barely help it (4.2 to 3.2 mm on the left; 4.3 to 4.5 mm on the right,
/// worse); and the deepest single corner, the armpit's apex pinched
/// between the hanging arm and the side, which no corrective reaches
/// (16.0 to 18.4 mm on the masculine left, worse; 14.3 to 17.2 mm
/// elsewhere).
#[test]
fn the_swing_correctives_take_the_walks_fold_out_of_the_armpit() {
    let assets = Assets::new(shipped());
    let body = BodyMesh::from_model(&assets.model_blocking(BODY).unwrap()).unwrap();
    let poses = fit_poses(&body.rig, Some(finish_pose));
    let world = body.rig.world(&body.rig.rest_pose());
    let welds = welds_of(&body.positions);
    for frame in [0.0f32, 1.0] {
        let state = vec![("Frame_Feminine".to_owned(), frame)];
        let (rest, rest_normals) = body.shaped(&state);
        let at_rest = bulge(&rest, &rest_normals, &body.triangles, &welds);
        let near: Vec<bool> = rest
            .iter()
            .map(|p| {
                ["LeftUpperArm", "RightUpperArm"].iter().any(|bone| {
                    let joint = world[body.rig.bone(bone).unwrap()].w_axis.truncate();
                    p.distance(joint) < 0.15 && p.y < joint.y
                })
            })
            .collect();
        for name in ["walk-left-40", "walk-right-40"] {
            let pose = poses.iter().find(|p| p.name == name).unwrap();
            // The fold's depth at each corner of the region: its 95th
            // percentile and its deepest.
            let fold = |swing: bool| {
                let mut weights = state.clone();
                weights.extend(
                    pose.correctives
                        .iter()
                        .filter(|(shape, _)| {
                            swing || !(shape.contains("_Forward_") || shape.contains("_Back_"))
                        })
                        .cloned(),
                );
                let (at, normals) = body.shaped(&weights);
                let matrices = skin_matrices(&body.rig, &pose.bones);
                let (posed, posed_normals, _) =
                    skin(&matrices, &body.joints, &body.weights, &at, &normals);
                let now = bulge(&posed, &posed_normals, &body.triangles, &welds);
                let mut depths: Vec<f32> = now
                    .iter()
                    .zip(&at_rest)
                    .zip(&near)
                    .filter(|(_, near)| **near)
                    .map(|((a, b), _)| (a - b).abs())
                    .collect();
                depths.sort_by(f32::total_cmp);
                (depths[depths.len() * 95 / 100], *depths.last().unwrap())
            };
            let ((without, without_most), (with, with_most)) = (fold(false), fold(true));
            println!(
                "frame {frame} {name}: the fold's 95th percentile {:.1} mm without the swing \
                 correctives, {:.1} mm with (the deepest corner {:.1} and {:.1} mm)",
                without * 1000.0,
                with * 1000.0,
                without_most * 1000.0,
                with_most * 1000.0
            );
            if frame == 0.0 {
                assert!(
                    with <= 0.6 * without,
                    "frame {frame} {name}: {with} m, {without} m without"
                );
            }
            // The records (metres): the 95th percentile and the deepest
            // corner with the swing correctives, by frame and side.
            let (most, deepest) = match (frame == 0.0, name) {
                (true, "walk-left-40") => (0.0049, 0.0184),
                (true, _) => (0.0059, 0.0143),
                (false, "walk-left-40") => (0.0032, 0.0172),
                (false, _) => (0.0045, 0.0165),
            };
            assert!(
                with <= most + 5e-5 && with_most <= deepest + 5e-5,
                "frame {frame} {name}: {with} m (record {most}), the deepest {with_most} m                  (record {deepest})"
            );
        }
    }
}
