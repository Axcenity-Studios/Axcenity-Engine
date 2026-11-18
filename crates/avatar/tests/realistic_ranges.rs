//! Character Studio refinement R3 (RA3), wider ranges (plan section 0,
//! decision 8): each proportion slider of the realistic body reaches as
//! far as the body still deforms cleanly or the population's 0.5th and
//! 99.5th percentiles, whichever is nearer, and the hips narrow as well
//! as widen.
//!
//! The ends are found by the sweep (`assets/characters/humanoid/qa/ranges.py`)
//! and recorded with what set each one in
//! `docs/development/character-studio/refinement/r3/ranges.md`; the
//! manifest ships that table. Narrower hips are a shape, not a bone: below
//! 1 `hip_width` leaves `anim`'s `HipWidth` at 1 (the legs' roots pulled
//! in cross the thighs at the crotch) and drives `<Frame>_Hips_Narrow`.
//!
//! Positions are the baked rest body's (`axcenity_assets::Model::bake`), in
//! glTF axes: Y up, X across the body.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use axcenity_assets::{Model, ModelFormat, Pick, load_models};
use axcenity_avatar::{Description, Library, resolve};

const BASE: &str = "axcenity/realistic";
/// The nine proportions and the body slider each drives.
const PROPORTIONS: [(&str, &str); 9] = [
    ("height", "BodyHeight"),
    ("leg_length", "LegLength"),
    ("arm_length", "ArmLength"),
    ("shoulder_width", "ShoulderWidth"),
    ("waist_width", "WaistWidth"),
    ("hip_width", "HipWidth"),
    ("head_size", "HeadSize"),
    ("hand_size", "HandSize"),
    ("foot_size", "FootSize"),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn library() -> Library {
    let (library, _, problems) = Library::read(&root().join("assets/characters"));
    assert!(problems.is_empty(), "{problems:?}");
    library
}

/// The "Shipped ranges" table of ranges.md: parameter -> [low, high].
fn recorded() -> BTreeMap<String, [f32; 2]> {
    recorded_under("Shipped ranges")
}

/// The table under the heading `## {heading}` of ranges.md.
fn recorded_under(heading: &str) -> BTreeMap<String, [f32; 2]> {
    let path = root().join("docs/development/character-studio/refinement/r3/ranges.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let table = text
        .split(&format!("\n## {heading}"))
        .nth(1)
        .unwrap_or_else(|| panic!("a \"{heading}\" section"));
    let mut out = BTreeMap::new();
    for line in table.lines().skip(1) {
        if line.starts_with('#') {
            break;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // | `id` | low | set by | high | set by |
        if cells.len() < 6 || !cells[1].starts_with('`') {
            continue;
        }
        let id = cells[1].trim_matches('`').to_owned();
        let number = |cell: &str| -> f32 {
            cell.parse()
                .unwrap_or_else(|_| panic!("{id}: {cell} is not a number"))
        };
        out.insert(id.clone(), [number(cells[2]), number(cells[4])]);
    }
    out
}

#[test]
fn the_shipped_proportion_ranges_are_the_sweeps() {
    let library = library();
    let base = library.base(BASE).unwrap();
    let table = recorded();
    assert_eq!(table.len(), PROPORTIONS.len(), "{table:?}");
    for (id, attribute) in PROPORTIONS {
        let parameter = base.parameter(id).unwrap_or_else(|| panic!("{id}"));
        let range = table[id];
        assert!(
            (parameter.range[0] - range[0]).abs() < 1e-6
                && (parameter.range[1] - range[1]).abs() < 1e-6,
            "{id}: the manifest ships {:?}, ranges.md says {range:?}",
            parameter.range
        );
        assert!(range[0] < 1.0 && range[1] > 1.0, "{id}: {range:?}");
        let driver = &parameter.attributes[0];
        assert_eq!(driver.attribute, attribute, "{id}");
        // The slider's value is its attribute's, except that the hips below
        // 1 move no bone.
        let expected = if id == "hip_width" {
            [1.0, range[1]]
        } else {
            range
        };
        assert_eq!(driver.from, expected, "{id}");
        assert_eq!(driver.to, None, "{id}");
    }
}

/// R3 wave 3: the stylized family is held to the same body checks, so its
/// proportion sliders end where its own sweep found them (`ranges.py
/// --package stylized`), not at `anim`'s clamp. Its hips narrow by the
/// bone (it has no narrowing shape), so every driver spans its range.
#[test]
fn the_stylized_proportion_ranges_are_its_sweeps() {
    let library = library();
    let base = library.base("axcenity/stylized").unwrap();
    let table = recorded_under("Stylized shipped ranges");
    assert_eq!(table.len(), PROPORTIONS.len(), "{table:?}");
    for (id, attribute) in PROPORTIONS {
        let parameter = base.parameter(id).unwrap_or_else(|| panic!("{id}"));
        let range = table[id];
        assert!(
            (parameter.range[0] - range[0]).abs() < 1e-6
                && (parameter.range[1] - range[1]).abs() < 1e-6,
            "stylized {id}: the manifest ships {:?}, ranges.md says {range:?}",
            parameter.range
        );
        assert!(
            range[0] <= 1.0 && range[1] >= 1.0,
            "stylized {id}: {range:?}"
        );
        let driver = &parameter.attributes[0];
        assert_eq!(driver.attribute, attribute, "stylized {id}");
        assert_eq!(driver.from, range, "stylized {id}");
    }
}

#[test]
fn the_ranges_stay_inside_anims_clamp() {
    // `anim::BodyShape` clamps every body slider to 0.5 to 1.5.
    for (id, [low, high]) in recorded()
        .into_iter()
        .chain(recorded_under("Stylized shipped ranges"))
    {
        assert!(
            (0.5..=1.5).contains(&low) && (0.5..=1.5).contains(&high),
            "{id}"
        );
    }
}

fn body(library: &Library) -> Model {
    let base = library.base(BASE).unwrap();
    let bytes = std::fs::read(
        root()
            .join("assets/characters/realistic")
            .join(&base.model.file),
    )
    .unwrap();
    let picks = [Pick::Objects(vec![base.model.body.clone()])];
    load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
        .unwrap()
        .remove(0)
}

/// A description's body as resolved and baked.
struct Baked {
    /// The rest positions, in metres.
    positions: Vec<[f32; 3]>,
    /// `anim`'s body sliders.
    attributes: BTreeMap<String, f32>,
    /// Each identity shape's summed weight.
    weights: BTreeMap<String, f32>,
}

fn baked(library: &Library, model: &Model, values: &[(&str, f32)]) -> Baked {
    let mut description = Description::new("Hips", BASE);
    for (id, value) in values {
        description.parameters.insert((*id).to_owned(), *value);
    }
    let resolved = resolve(library, &description).unwrap();
    assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
    let mut weights = BTreeMap::new();
    for layer in &resolved.identity.layers {
        for (shape, weight) in &layer.shapes {
            *weights.entry(shape.clone()).or_insert(0.0) += weight;
        }
    }
    let size = model.size;
    let model = model.bake(&resolved.identity);
    let rig = model.rig.as_ref().unwrap();
    let posed = model.pose_shaped(&rig.rest_pose(), &[]).unwrap();
    let positions = posed
        .primitive(&model, 0, 0)
        .positions
        .iter()
        .map(|p| [p[0] * size[0], p[1] * size[1], p[2] * size[2]])
        .collect();
    Baked {
        positions,
        attributes: resolved.attributes,
        weights,
    }
}

#[test]
fn hips_narrow_below_one_by_a_shape_not_a_bone() {
    let library = library();
    let base = library.base(BASE).unwrap();
    let hips = base.parameter("hip_width").unwrap();
    let low = hips.range[0];
    assert!(low < 1.0, "the hips go below 1: {:?}", hips.range);
    let model = body(&library);
    for (frame, name) in [(0.0, "Masculine"), (1.0, "Feminine")] {
        let neutral = baked(&library, &model, &[("frame", frame)]).positions;
        let Baked {
            positions: narrow,
            attributes,
            weights,
        } = baked(&library, &model, &[("frame", frame), ("hip_width", low)]);
        assert_eq!(attributes["HipWidth"], 1.0, "{name}: no bone narrows");
        let shape = format!("{name}_Hips_Narrow");
        assert!(
            weights.get(&shape).copied().unwrap_or(0.0) > 0.5,
            "{name}: {weights:?}"
        );
        // The hips: the widest skin of the trunk between the crotch and the
        // waist (a fifth of the height either side of 0.5, within 0.26 m of
        // the middle: the hands hang farther out).
        let feet = neutral.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        let top = neutral.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        let height = top - feet;
        let band = |p: &[f32; 3]| {
            let up = (p[1] - feet) / height;
            (0.45..0.6).contains(&up) && p[0].abs() < 0.26
        };
        let widest = |body: &[[f32; 3]]| {
            body.iter()
                .filter(|p| band(p))
                .map(|p| p[0].abs())
                .fold(0.0_f32, f32::max)
        };
        let (before, after) = (widest(&neutral), widest(&narrow));
        assert!(
            after < before * 0.95,
            "{name}: hips {before} m to {after} m from the middle at {low}"
        );
        // Nothing moves out, and nothing crosses the middle.
        for (a, b) in neutral.iter().zip(&narrow) {
            if band(a) {
                assert!(b[0].abs() <= a[0].abs() + 1e-4, "{name}: {a:?} to {b:?}");
                assert!(
                    a[0] * b[0] >= 0.0 || a[0].abs() < 1e-4,
                    "{name}: {a:?} to {b:?}"
                );
            }
        }
    }
}
