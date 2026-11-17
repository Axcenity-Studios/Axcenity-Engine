//! Character Studio refinement, R4 (RA2): the realistic face's controls
//! added from the face references' high-value list
//! (`docs/development/character-studio/references/face/README.md`, §2),
//! and the two frames' faces made more different.
//!
//! These read the shipped package (`assets/characters/realistic/`):
//! rebuild it after changing the face (its README).

use std::path::{Path, PathBuf};

use axcenity_assets::{Model, ModelFormat, Pick, load_models};
use axcenity_avatar::{Description, Level, Library, resolve};

const BASE: &str = "axcenity/realistic";

/// The new controls: (id stem, group, paired). A paired control has a
/// `_left` and a `_right` parameter, each naming the other as its mirror.
const CONTROLS: [(&str, &str, bool); 9] = [
    ("nose_tip_projection", "nose", false),
    ("nose_tip_rotation", "nose", false),
    ("nose_width", "nose", false),
    ("philtrum_length", "lips", false),
    ("jaw_angle", "jaw", true),
    ("upper_lid_fold", "eyes", true),
    ("lower_lid", "eyes", true),
    ("eye_corner", "eyes", true),
    ("earlobe", "ears", true),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn library() -> Library {
    let (library, _, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{problems:?}");
    library
}

fn ids(stem: &str, paired: bool) -> Vec<String> {
    if paired {
        vec![format!("{stem}_left"), format!("{stem}_right")]
    } else {
        vec![stem.to_owned()]
    }
}

#[test]
fn the_face_has_the_high_value_controls_with_their_metadata() {
    let library = library();
    let base = library.base(BASE).unwrap();
    for (stem, group, paired) in CONTROLS {
        for id in ids(stem, paired) {
            let p = base
                .parameter(&id)
                .unwrap_or_else(|| panic!("no parameter `{id}`"));
            assert_eq!(p.group, group, "{id}");
            assert_eq!(p.feature.as_deref(), Some(group), "{id}");
            assert_eq!(p.range, [-1.0, 1.0], "{id}");
            assert_eq!(p.level(), Level::Detailed, "{id}");
            let about = p.about.as_deref().unwrap_or_default();
            assert!(about.len() > 20 && about.ends_with('.'), "{id}: {about:?}");
            assert!(!about.contains("{side}"), "{id}: {about:?}");
            assert!(!p.shapes.is_empty(), "{id} drives no shape");
            if paired {
                let other = if id.ends_with("_left") {
                    id.replace("_left", "_right")
                } else {
                    id.replace("_right", "_left")
                };
                assert_eq!(p.mirror.as_deref(), Some(other.as_str()), "{id}");
                assert!(about.contains(if id.ends_with("_left") {
                    "left"
                } else {
                    "right"
                }));
            } else {
                assert!(p.mirror.is_none(), "{id}");
            }
        }
    }
    // The tilt and the new corner control say what tells them apart.
    let tilt = base.parameter("eye_tilt_left").unwrap();
    assert!(
        tilt.about.as_deref().unwrap().contains("inner"),
        "{:?}",
        tilt.about
    );
}

/// Each new control moves the face by its population range (the build's
/// largest move per shape, `Realistic.build.json`): at least what is seen
/// (2 mm) and no more than a shape stretches cleanly with its neighbours.
#[test]
fn each_new_control_moves_the_face_within_its_range() {
    let text = std::fs::read_to_string(root().join("realistic/Realistic.build.json")).unwrap();
    let data: serde_json::Value = serde_json::from_str(&text).unwrap();
    let largest = &data["face"]["largest_move_mm"];
    let library = library();
    let base = library.base(BASE).unwrap();
    for (stem, _, paired) in CONTROLS {
        for id in ids(stem, paired) {
            let p = base.parameter(&id).unwrap();
            for driver in &p.shapes {
                let mm = largest[&driver.shape]
                    .as_f64()
                    .unwrap_or_else(|| panic!("{id}: no largest move for `{}`", driver.shape));
                assert!(
                    (2.0..=12.0).contains(&mm),
                    "{id}: `{}` moves {mm} mm",
                    driver.shape
                );
            }
        }
    }
}

fn body(library: &Library) -> Model {
    let base = library.base(BASE).unwrap();
    let bytes = std::fs::read(root().join("realistic").join(&base.model.file)).unwrap();
    let picks = [Pick::Objects(vec![base.model.body.clone()])];
    load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
        .unwrap()
        .remove(0)
}

fn face_at(library: &Library, model: &Model, frame: f32) -> Vec<[f32; 3]> {
    let mut description = Description::new("Frame", BASE);
    description.parameters.insert("frame".into(), frame);
    let resolved = resolve(library, &description).unwrap();
    let baked = model.bake(&resolved.identity);
    let rig = baked.rig.as_ref().unwrap();
    let posed = baked.pose_shaped(&rig.rest_pose(), &[]).unwrap();
    // Every primitive's vertices (the skin and the mouth's lining), in order.
    (0..baked.primitives.len())
        .flat_map(|k| posed.primitive(&baked, k, 0).positions.clone())
        .collect()
}

/// M6, the baseline's C1: the two frames had nearly the same face. The
/// feminine frame's face (its shape's face part, less the move of the head
/// as a whole) now differs from the masculine one by more than 4 mm
/// (root mean square over the front of the face; it was 3.1 mm).
#[test]
fn the_frames_faces_differ() {
    let library = library();
    let model = body(&library);
    let m = face_at(&library, &model, 0.0);
    let f = face_at(&library, &model, 1.0);
    assert_eq!(m.len(), f.len());
    // (A model's positions are its box scaled to one on each axis; `size`
    // is the box in the file, in metres.)
    let size = model.size;
    let metres = |p: &[f32; 3]| [p[0] * size[0], p[1] * size[1], p[2] * size[2]];
    let m: Vec<[f32; 3]> = m.iter().map(metres).collect();
    let f: Vec<[f32; 3]> = f.iter().map(metres).collect();
    // The front of the head (glTF: Y up, the face toward +Z), the same
    // vertices on both frames: from the brows (6 cm under the top) to under
    // the chin (24 cm), and within 7 cm of the nose's tip, in front of the
    // ears.
    let top = m.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    let head: Vec<usize> = (0..m.len())
        .filter(|&i| m[i][1] > top - 0.24 && m[i][1] < top - 0.06)
        .collect();
    let front = head.iter().map(|&i| m[i][2]).fold(f32::MIN, f32::max);
    let face: Vec<usize> = head
        .into_iter()
        .filter(|&i| m[i][2] > front - 0.07)
        .collect();
    assert!(face.len() > 500, "{} face vertices", face.len());
    let n = face.len() as f32;
    let mean: [f32; 3] =
        std::array::from_fn(|k| face.iter().map(|&i| f[i][k] - m[i][k]).sum::<f32>() / n);
    let rms = (face
        .iter()
        .map(|&i| {
            (0..3)
                .map(|k| (f[i][k] - m[i][k] - mean[k]).powi(2))
                .sum::<f32>()
        })
        .sum::<f32>()
        / n)
        .sqrt();
    assert!(
        rms > 0.0040,
        "the frames' faces differ by {:.2} mm",
        rms * 1000.0
    );
}
