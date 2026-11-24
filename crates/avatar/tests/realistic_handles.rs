//! Character Studio refinement, R2 (direct manipulation, brief §3.4,
//! §4.3 and §5.3): the realistic base model's handles, written by
//! `assets/characters/realistic/manifests.py` (`handles.py`).
//!
//! These read the shipped package (`assets/characters/realistic/`): run
//! `manifests.py` again after changing a handle.

use std::path::Path;

use axcenity_avatar::{BaseManifest, Library};

/// The features with handles, each feature's together, in the order
/// `handles.py` lists them: the ring round the face's outline (the
/// `face_shape` feature, 2026-11-24), then the brief's first seven in its
/// order (nose, eyes, jaw, the mouth as the `lips` feature, shoulders,
/// waist, hips) with the face's other features among them where the
/// studio lists them (cheeks, chin, forehead).
const FEATURES: [&str; 11] = [
    "face_shape",
    "nose",
    "eyes",
    "cheeks",
    "jaw",
    "chin",
    "lips",
    "forehead",
    "shoulders",
    "waist",
    "hips",
];

/// The feature whose handles are the ring round the face's outline. They
/// drive the outline's own parameters and those of the features the
/// outline passes (the forehead's height, the temples' width, the jaw's
/// angle, the chin's width and shape), so selecting Face shape shows one
/// ring of markers that reshapes the whole outline.
const OUTLINE: &str = "face_shape";

/// The framings a handle's `views` and a drive's `screen` name.
const VIEWS: [&str; 4] = ["front", "three_quarter", "side", "back"];

fn realistic() -> BaseManifest {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters");
    let (library, _, problems) = Library::read(&root);
    assert!(problems.is_empty(), "{problems:?}");
    library.base("axcenity/realistic").unwrap().clone()
}

#[test]
fn the_seven_features_have_handles_in_the_brief_s_order() {
    let base = realistic();
    assert!(base.validate().is_empty(), "{:?}", base.validate());
    let mut order: Vec<&str> = Vec::new();
    for handle in &base.handles {
        if order.last() != Some(&handle.feature.as_str()) {
            order.push(&handle.feature);
        }
    }
    assert_eq!(order, FEATURES, "each feature's handles together, in order");
    for feature in FEATURES {
        assert!(base.feature(feature).is_some(), "{feature} is a feature");
    }
}

#[test]
fn every_handle_drives_its_own_feature_s_parameters_and_pairs_name_each_other() {
    let base = realistic();
    for handle in &base.handles {
        assert!(!handle.drives.is_empty(), "{}", handle.id);
        assert!(
            handle.drives.len() <= 3,
            "{}: a few meaningful drives",
            handle.id
        );
        for drive in &handle.drives {
            let parameter = base
                .parameter(&drive.parameter)
                .unwrap_or_else(|| panic!("{}: no {}", handle.id, drive.parameter));
            let feature = base.feature_of(parameter).expect("a feature");
            if handle.feature == OUTLINE {
                // The outline's ring: any of the face's parameters.
                assert_eq!(
                    feature.category,
                    axcenity_avatar::Category::Face,
                    "{}",
                    drive.parameter
                );
            } else {
                assert_eq!(feature.id, handle.feature, "{}", drive.parameter);
            }
        }
        let Some(mirror) = &handle.mirror else {
            continue;
        };
        let twin = base.handle(mirror).unwrap();
        assert_eq!(twin.mirror.as_deref(), Some(handle.id.as_str()));
        assert_eq!(twin.feature, handle.feature);
        assert_eq!(twin.drives.len(), handle.drives.len());
        // Each side drives its own side's parameter, or the one both share.
        for (ours, theirs) in handle.drives.iter().zip(&twin.drives) {
            let mirrored = base
                .parameter(&ours.parameter)
                .and_then(|p| p.mirror.clone())
                .unwrap_or_else(|| ours.parameter.clone());
            assert_eq!(theirs.parameter, mirrored, "{} and {}", handle.id, twin.id);
            // The twin's anchor and axis are this one's mirrored across
            // the middle (the character's left is +X).
            assert!(
                (ours.axis[0] + theirs.axis[0]).abs() < 0.05,
                "{}",
                handle.id
            );
            assert!(
                (ours.axis[1] - theirs.axis[1]).abs() < 0.05,
                "{}",
                handle.id
            );
        }
        let (a, b) = (handle.anchor.offset, twin.anchor.offset);
        assert!((a[0] + b[0]).abs() < 0.002 && (a[1] - b[1]).abs() < 0.002);
    }
}

/// A drive moves the surface by `travel` metres along `axis` over its
/// parameter's range, measured on the model: a unit axis and a travel of
/// a few millimetres (a face) to a few centimetres (the body), so a drag
/// of one centimetre moves the skin about one centimetre. Each framing it
/// works in has a screen direction, and no two drives of one handle share
/// a line on the screen there.
#[test]
fn drives_have_a_measured_axis_and_travel_and_one_line_each_per_view() {
    let base = realistic();
    for handle in &base.handles {
        assert!(!handle.views.is_empty(), "{} works in some view", handle.id);
        for view in &handle.views {
            assert!(VIEWS.contains(&view.as_str()), "{}: {view}", handle.id);
            assert!(
                handle.drives.iter().any(|d| d.screen.contains_key(view)),
                "{}: a drive works in {view}",
                handle.id
            );
        }
        for drive in &handle.drives {
            let length = drive.axis.iter().map(|v| v * v).sum::<f32>().sqrt();
            assert!(
                (length - 1.0).abs() < 0.01,
                "{}: axis {length}",
                drive.parameter
            );
            assert!(
                (0.002..=0.08).contains(&drive.travel),
                "{}: {} m",
                drive.parameter,
                drive.travel
            );
            for (view, direction) in &drive.screen {
                assert!(VIEWS.contains(&view.as_str()));
                assert!(handle.views.contains(view), "{}: {view}", handle.id);
                let length = direction[0].hypot(direction[1]);
                assert!((length - 1.0).abs() < 0.01);
            }
        }
        for view in VIEWS {
            let lines: Vec<[f32; 2]> = handle
                .drives
                .iter()
                .filter_map(|drive| drive.screen.get(view).copied())
                .collect();
            for (k, a) in lines.iter().enumerate() {
                for b in &lines[k + 1..] {
                    let along = (a[0] * b[0] + a[1] * b[1]).abs();
                    assert!(
                        along < 0.91,
                        "{} in {view}: two drives on one line",
                        handle.id
                    );
                }
            }
        }
    }
}

/// The eye-corner handles sit on the outer corner of the eye, not on the
/// cheek (where they were, 10 mm below): within 4 mm of the eye's height
/// and out to the side of it, within 2 cm. The eye's height is the eyes
/// feature's anchor on that side (both are offsets from the head bone);
/// that anchor, the eye region's middle, is about 6 mm under the eyeball's
/// centre and the outer corner about 3 mm under it, so they are 3 mm apart.
#[test]
fn the_eye_corner_handles_sit_at_the_eye_s_outer_corner() {
    let base = realistic();
    let eyes = base.feature("eyes").unwrap();
    for id in ["eye_corner_left", "eye_corner_right"] {
        let handle = base.handle(id).unwrap();
        let [x, y, _] = handle.anchor.offset;
        let eye = eyes
            .anchors
            .iter()
            .find(|anchor| anchor.offset[0].signum() == x.signum())
            .unwrap();
        assert_eq!(handle.anchor.bone, eye.bone, "{id}");
        let below = (y - eye.offset[1]) * 1000.0;
        assert!(
            below.abs() <= 4.0,
            "{id}: {below:.1} mm off the eye's height"
        );
        let out = (x.abs() - eye.offset[0].abs()) * 1000.0;
        assert!(
            (0.0..=20.0).contains(&out),
            "{id}: {out:.1} mm out to the side of the eye"
        );
    }
}
