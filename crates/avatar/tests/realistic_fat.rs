//! Character Studio refinement R3 (RA3): fat that looks like fat on the
//! realistic body. `fat` at 1 has to read as a heavier person (a belly in
//! profile, a softer and wider waist), with the fat placed by frame as
//! people carry it (more at the belly on the masculine frame; more on the
//! hips, thighs and buttocks on the feminine), and the slight masculine
//! body (`fat` and `muscle` both -1) has to lose its small belly.
//!
//! Measured on the baked rest body (`axcenity_assets::Model::bake`, what a
//! character's MeshPart shows), in glTF axes: Y up, the figure facing +Z.
//! Bands are shares of the default body's height, measured from its feet.

use std::path::{Path, PathBuf};

use axcenity_assets::{Model, ModelFormat, Pick, load_models};
use axcenity_avatar::{Description, Library, resolve};

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

/// The full-detail body's rest positions for these parameter values, in
/// metres. (A model's positions are its box scaled to one on each axis;
/// `size` is the base's box in the file, in metres for this one, and a
/// baked model keeps its base's scale.)
fn baked(library: &Library, model: &Model, values: &[(&str, f32)]) -> Vec<[f32; 3]> {
    let mut description = Description::new("Build", BASE);
    for (id, value) in values {
        description.parameters.insert((*id).to_owned(), *value);
    }
    let resolved = resolve(library, &description).unwrap();
    assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
    let size = model.size;
    let model = model.bake(&resolved.identity);
    let rig = model.rig.as_ref().unwrap();
    let posed = model.pose_shaped(&rig.rest_pose(), &[]).unwrap();
    posed
        .primitive(&model, 0, 0)
        .positions
        .iter()
        .map(|p| [p[0] * size[0], p[1] * size[1], p[2] * size[2]])
        .collect()
}

/// Where the trunk's bands lie, from one body's feet and height.
struct Bands {
    feet: f32,
    height: f32,
}

impl Bands {
    fn of(body: &[[f32; 3]]) -> Self {
        let feet = body.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        let top = body.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        Self {
            feet,
            height: top - feet,
        }
    }

    /// The vertices between two shares of the height, within `half` of
    /// the middle side to side (the hanging arms lie farther out).
    fn band<'a>(
        &self,
        body: &'a [[f32; 3]],
        low: f32,
        high: f32,
        half: f32,
    ) -> impl Iterator<Item = &'a [f32; 3]> {
        let (low, high) = (
            self.feet + low * self.height,
            self.feet + high * self.height,
        );
        body.iter()
            .filter(move |p| p[1] > low && p[1] < high && p[0].abs() < half)
    }

    /// How far forward the belly reaches (between navel and hip crest).
    fn belly(&self, body: &[[f32; 3]]) -> f32 {
        self.band(body, 0.55, 0.61, 0.08)
            .map(|p| p[2])
            .fold(f32::MIN, f32::max)
    }

    /// How far back the buttocks reach.
    fn buttocks(&self, body: &[[f32; 3]]) -> f32 {
        -self
            .band(body, 0.49, 0.52, 0.26)
            .map(|p| p[2])
            .fold(f32::MAX, f32::min)
    }

    fn width(&self, body: &[[f32; 3]], low: f32, high: f32, half: f32) -> f32 {
        let (min, max) = self
            .band(body, low, high, half)
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                (lo.min(p[0]), hi.max(p[0]))
            });
        max - min
    }

    fn waist(&self, body: &[[f32; 3]]) -> f32 {
        self.width(body, 0.58, 0.62, 0.24)
    }

    /// One thigh's width, a little below the crotch.
    fn thigh(&self, body: &[[f32; 3]]) -> f32 {
        let (low, high) = (
            self.feet + 0.40 * self.height,
            self.feet + 0.43 * self.height,
        );
        let (min, max) = body
            .iter()
            .filter(|p| p[1] > low && p[1] < high && p[0] > 0.0)
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                (lo.min(p[0]), hi.max(p[0]))
            });
        max - min
    }
}

/// How `fat` 1 changes one frame's body against its default: (belly
/// forward, waist wider, buttocks back, thigh wider), in metres.
fn heavier(library: &Library, model: &Model, frame: f32) -> [f32; 4] {
    let default = baked(library, model, &[("frame", frame)]);
    let heavy = baked(library, model, &[("frame", frame), ("fat", 1.0)]);
    let bands = Bands::of(&default);
    [
        bands.belly(&heavy) - bands.belly(&default),
        bands.waist(&heavy) - bands.waist(&default),
        bands.buttocks(&heavy) - bands.buttocks(&default),
        bands.thigh(&heavy) - bands.thigh(&default),
    ]
}

/// The baseline's defect H6: at `fat` 1 the waist and limbs thickened a
/// little and there was no belly in the side view (8 mm forward on the
/// masculine frame). A heavier person's belly comes forward by centimetres
/// and the waist widens with it.
#[test]
fn fat_puts_on_a_belly_and_widens_the_waist_on_both_frames() {
    let library = library();
    let model = body(&library);
    for frame in [0.0, 1.0] {
        let [belly, waist, _, _] = heavier(&library, &model, frame);
        assert!(
            belly > 0.025,
            "frame {frame}: the belly comes forward {belly} m"
        );
        assert!(waist > 0.04, "frame {frame}: the waist widens {waist} m");
    }
}

/// Fat is placed by frame as people carry it (references/body B12): more
/// at the belly on the masculine frame, more on the buttocks and thighs on
/// the feminine.
#[test]
fn fat_goes_where_each_frame_carries_it() {
    let library = library();
    let model = body(&library);
    let [m_belly, _, m_buttocks, m_thigh] = heavier(&library, &model, 0.0);
    let [f_belly, _, f_buttocks, f_thigh] = heavier(&library, &model, 1.0);
    assert!(
        m_belly > f_belly + 0.01,
        "belly forward: masculine {m_belly} m, feminine {f_belly} m"
    );
    assert!(
        f_buttocks > m_buttocks + 0.005,
        "buttocks back: masculine {m_buttocks} m, feminine {f_buttocks} m"
    );
    assert!(
        f_thigh > m_thigh + 0.005,
        "thigh wider: masculine {m_thigh} m, feminine {f_thigh} m"
    );
}

/// OPEN-08: the slight masculine body (`muscle` and `fat` both -1) had a
/// small belly under a narrow chest (MakeHuman's least muscled body). Its
/// belly sits well behind the slim body's (`fat` -1 alone), not level
/// with it.
#[test]
fn the_slight_masculine_body_has_a_flat_belly() {
    let library = library();
    let model = body(&library);
    let default = baked(&library, &model, &[("frame", 0.0)]);
    let slim = baked(&library, &model, &[("frame", 0.0), ("fat", -1.0)]);
    let slight = baked(
        &library,
        &model,
        &[("frame", 0.0), ("fat", -1.0), ("muscle", -1.0)],
    );
    let bands = Bands::of(&default);
    let behind = bands.belly(&slim) - bands.belly(&slight);
    assert!(
        behind > 0.015,
        "the slight belly is {behind} m behind the slim one"
    );
}
