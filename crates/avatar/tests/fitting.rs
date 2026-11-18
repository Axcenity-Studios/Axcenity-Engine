//! Garment fitting on the realistic body (Character Studio CO9, ADR-0035,
//! ADR-0041 decision 4): the shipped underwear already carries what the
//! transfer gives; a garment not made from the body (the test tee,
//! `fixtures/test-tee`) imports into a library, carries every identity and
//! corrective shape, is weighted to the body and stays outside it and the
//! sports bra at every extreme.

use std::path::{Path, PathBuf};

use axcenity_assets::Assets;
use axcenity_avatar::fit::{
    BodyMesh, FitOptions, GarmentMesh, ImportRequest, Surface, align, carried_shapes, extremes_of,
    fit_garment, import_garment,
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
        url: None,
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
    }
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
/// height found from how evenly it stands off the body.
#[test]
fn alignment_puts_a_garment_made_too_high_and_too_small_back_on_the_body() {
    let assets = Assets::new(shipped());
    let body = BodyMesh::from_model(&assets.model_blocking(BODY).unwrap()).unwrap();
    let source = axcenity_assets::load_model_file(&tee()).unwrap();
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
