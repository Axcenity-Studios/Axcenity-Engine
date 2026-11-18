//! Character Studio refinement R3 (RA3, wave 3): the realistic body's
//! waist slider changes the waist. `anim`'s `WaistWidth` alone scales only
//! the skin weighted to the Spine bone, which moved the waist's breadth by
//! about a quarter of the slider; `<Frame>_Waist_Wide` and `_Narrow` carry
//! the rest (`assets/characters/realistic/shapes.py`, `waist`).
//!
//! Measured as a game draws the character: the description resolved
//! (`avatar`), its identity baked into the body (`assets`), and the body
//! slider applied to the pose by `anim`'s own rule (`BodyShape`). Positions
//! are in metres, glTF axes: Y up, X side to side, Z front to back.

use std::path::{Path, PathBuf};

use axcenity_anim::BodyShape;
use axcenity_assets::{Model, ModelFormat, Pick, load_models};
use axcenity_avatar::{Description, Library, resolve};
use axcenity_core::{ClassName, Value, World};

const BASE: &str = "axcenity/realistic";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn library() -> Library {
    let (library, _, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{problems:?}");
    library
}

fn body(library: &Library) -> Model {
    let base = library.base(BASE).unwrap();
    let bytes = std::fs::read(root().join("realistic").join(&base.model.file)).unwrap();
    let picks = [Pick::Objects(vec![base.model.body.clone()])];
    load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
        .unwrap()
        .remove(0)
}

/// The full-detail body at rest for these parameter values, with its
/// identity baked and its body sliders applied as `anim` applies them.
fn posed(library: &Library, model: &Model, values: &[(&str, f32)]) -> Vec<[f32; 3]> {
    let mut description = Description::new("Waist", BASE);
    for (id, value) in values {
        description.parameters.insert((*id).to_owned(), *value);
    }
    let resolved = resolve(library, &description).unwrap();
    assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
    let size = model.size;
    let baked = model.bake(&resolved.identity);
    let rig = baked.rig.clone().unwrap();
    let mut world = World::new();
    let part = world.create(ClassName::MeshPart).unwrap();
    for (attribute, value) in &resolved.attributes {
        world
            .set_attribute(part, attribute, Some(Value::Number(f64::from(*value))))
            .unwrap();
    }
    let mut pose = rig.rest_pose();
    let posed = match BodyShape::read(&world, part) {
        Some(shape) => {
            let scales = shape.apply(&rig, &mut pose);
            baked.pose_shaped(&pose, &scales).unwrap()
        }
        None => baked.pose_shaped(&pose, &[]).unwrap(),
    };
    posed
        .primitive(&baked, 0, 0)
        .positions
        .iter()
        .map(|p| [p[0] * size[0], p[1] * size[1], p[2] * size[2]])
        .collect()
}

/// The trunk's slab at a height (within 1.5 cm), arms left out (in the
/// A-pose they hang more than 0.3 m out at the waist's height).
fn slab(body: &[[f32; 3]], y: f32) -> Vec<[f32; 3]> {
    body.iter()
        .filter(|p| (p[1] - y).abs() < 0.015 && p[0].abs() < 0.3)
        .copied()
        .collect()
}

fn extent(points: &[[f32; 3]], axis: usize) -> f32 {
    let low = points.iter().map(|p| p[axis]).fold(f32::MAX, f32::min);
    let high = points.iter().map(|p| p[axis]).fold(f32::MIN, f32::max);
    high - low
}

/// The natural waist: the height of the trunk's narrowest slab between
/// 0.56 and 0.70 of the body's height.
fn natural_waist(body: &[[f32; 3]]) -> f32 {
    let feet = body.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
    let top = body.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    let height = top - feet;
    (0..=28)
        .map(|k| feet + height * (0.56 + 0.005 * k as f32))
        .map(|y| (y, extent(&slab(body, y), 0)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap()
        .0
}

#[test]
fn the_waist_slider_changes_the_waists_breadth_by_most_of_its_value() {
    let library = library();
    let base = library.base(BASE).unwrap();
    let range = base.parameter("waist_width").unwrap().range;
    let model = body(&library);
    for (frame, name) in [(0.0, "masculine"), (1.0, "feminine")] {
        let neutral = posed(&library, &model, &[("frame", frame)]);
        let y = natural_waist(&neutral);
        let at_one = slab(&neutral, y);
        let (breadth, depth) = (extent(&at_one, 0), extent(&at_one, 2));
        for value in range {
            let body = posed(
                &library,
                &model,
                &[("frame", frame), ("waist_width", value)],
            );
            let here = slab(&body, y);
            // The share of the slider's change the waist's breadth (front
            // view) and depth (side view) change by.
            let breadth_share = (extent(&here, 0) / breadth - 1.0) / (value - 1.0);
            let depth_share = (extent(&here, 2) / depth - 1.0) / (value - 1.0);
            eprintln!(
                "{name} at {value}: breadth {breadth_share:.3}, depth {depth_share:.3} of the slider's change"
            );
            assert!(
                breadth_share >= 0.6,
                "{name} at {value}: the waist's breadth changes by {breadth_share:.3} of the slider \
                 ({breadth:.4} m at 1)"
            );
            // Breadth and depth go together in people (r = 0.87): the
            // side view widens too, less.
            assert!(
                (0.15..=breadth_share).contains(&depth_share),
                "{name} at {value}: the waist's depth changes by {depth_share:.3} of the slider \
                 ({depth:.4} m at 1)"
            );
        }
    }
}

#[test]
fn the_waist_shapes_are_driven_from_one_on_both_sides() {
    // Wide above 1, narrow below, none at 1: the slider has no kink and
    // the bone keeps its own part.
    let library = library();
    for (frame, name) in [(0.0, "Masculine"), (1.0, "Feminine")] {
        for (value, wide, narrow) in [(1.0, 0.0, 0.0), (1.1, 0.2, 0.0), (0.9, 0.0, 0.2)] {
            let mut description = Description::new("Waist", BASE);
            description.parameters.insert("frame".into(), frame);
            description.parameters.insert("waist_width".into(), value);
            let resolved = resolve(&library, &description).unwrap();
            let weight = |shape: &str| -> f32 {
                resolved
                    .identity
                    .layers
                    .iter()
                    .flat_map(|layer| layer.shapes.iter())
                    .filter(|(name, _)| name.as_str() == shape)
                    .map(|(_, weight)| *weight)
                    .sum()
            };
            let got = (
                weight(&format!("{name}_Waist_Wide")),
                weight(&format!("{name}_Waist_Narrow")),
            );
            assert!(
                (got.0 - wide).abs() < 1e-4 && (got.1 - narrow).abs() < 1e-4,
                "{name} at {value}: {got:?}"
            );
            assert!(
                (resolved.attributes["WaistWidth"] - value).abs() < 1e-6,
                "{name} at {value}: the bone"
            );
        }
    }
}
