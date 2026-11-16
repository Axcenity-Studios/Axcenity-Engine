//! Character Studio overhaul, CO5: every parameter of the realistic base
//! model's manifest moves geometry, on the body and on both of its simpler
//! copies (levels of detail), at both ends of its range.
//!
//! A parameter that drives shapes or bones is checked on the model its
//! identity bakes (the brows' length on the `Brows` part, which is all it
//! moves) (`axcenity_assets::Model::bake`, what a character's
//! MeshPart shows at rest). A parameter that drives an attribute (the nine
//! proportions) is checked to change that attribute, which is one of
//! `anim`'s body sliders; that each of those moves the skin is `anim`'s
//! `tests/realistic_proportions.rs`.

use std::path::{Path, PathBuf};

use axcenity_assets::{Model, ModelFormat, Pick, load_models};
use axcenity_avatar::{Description, Library, resolve};

const BASE: &str = "axcenity/realistic";
/// `anim::BODY_SLIDERS` (`avatar` doesn't depend on `anim`).
const BODY_SLIDERS: [&str; 9] = [
    "BodyHeight",
    "LegLength",
    "ArmLength",
    "ShoulderWidth",
    "WaistWidth",
    "HipWidth",
    "HeadSize",
    "HandSize",
    "FootSize",
];
/// Less than this isn't a change anyone would see, in metres.
const SEEN: f32 = 0.002;

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
    object(library, &base.model.body)
}

/// One object of the base model's file, such as a part (`…/Brows`).
fn object(library: &Library, path: &str) -> Model {
    let base = library.base(BASE).unwrap();
    let bytes = std::fs::read(root().join("realistic").join(&base.model.file)).unwrap();
    let picks = [Pick::Objects(vec![path.to_owned()])];
    load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
        .unwrap()
        .remove(0)
}

/// Parameters that move a part the body carries an empty shape for (the
/// brows' card), not the body: the part they move.
const PART_PARAMETERS: [(&str, &str); 2] = [
    ("brow_length_left", "AxcenityRealistic/Brows"),
    ("brow_length_right", "AxcenityRealistic/Brows"),
];

/// The rest positions of `model`'s first primitive at each level of detail.
fn rest(model: &Model) -> Vec<Vec<[f32; 3]>> {
    let rig = model.rig.as_ref().unwrap();
    let posed = model.pose_shaped(&rig.rest_pose(), &[]).unwrap();
    (0..=model.primitives[0].lods.len())
        .map(|level| posed.primitive(model, 0, level).positions.clone())
        .collect()
}

fn farthest(a: &[[f32; 3]], b: &[[f32; 3]]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(p, q)| (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f32>().sqrt())
        .fold(0.0, f32::max)
}

#[test]
fn the_realistic_base_has_its_body_parameters() {
    let library = library();
    let base = library.base(BASE).unwrap();
    for id in ["frame", "muscle", "fat"] {
        assert!(base.parameter(id).is_some(), "{id}");
    }
    for region in [
        "chest",
        "shoulders",
        "arms",
        "abdomen",
        "back",
        "hips",
        "thighs",
        "calves",
    ] {
        for kind in ["fat", "muscle"] {
            let id = format!("{kind}_{region}");
            let parameter = base.parameter(&id).unwrap_or_else(|| panic!("{id}"));
            assert!(parameter.advanced, "{id}");
        }
    }
    let proportions = base
        .parameters
        .iter()
        .filter(|parameter| !parameter.attributes.is_empty())
        .count();
    assert_eq!(proportions, BODY_SLIDERS.len());
}

#[test]
fn every_parameter_moves_the_body_and_its_simpler_copies() {
    let library = library();
    let base = library.base(BASE).unwrap();
    let model = body(&library);
    assert_eq!(model.primitives[0].lods.len(), 2, "two simpler copies");
    let neutral = Description::new("Neutral", BASE);
    let resolved_neutral = resolve(&library, &neutral).unwrap();
    let at_rest = rest(&model.bake(&resolved_neutral.identity));
    let mut checked = 0;
    for parameter in &base.parameters {
        for end in parameter.range {
            if (end - parameter.default).abs() < 1e-6 {
                continue;
            }
            let mut description = Description::new("One slider", BASE);
            description.parameters.insert(parameter.id.clone(), end);
            let resolved = resolve(&library, &description).unwrap();
            assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
            let at = format!("{} at {end}", parameter.id);
            if !parameter.attributes.is_empty() {
                for driver in &parameter.attributes {
                    assert!(
                        BODY_SLIDERS.contains(&driver.attribute.as_str()),
                        "{at}: {}",
                        driver.attribute
                    );
                    let was = resolved_neutral.attributes[&driver.attribute];
                    let now = resolved.attributes[&driver.attribute];
                    assert!((now - was).abs() > 0.05, "{at}: {was} to {now}");
                }
            } else if let Some((_, part)) =
                PART_PARAMETERS.iter().find(|(id, _)| *id == parameter.id)
            {
                let part = object(&library, part);
                let far = farthest(
                    &rest(&part.bake(&resolved.identity))[0],
                    &rest(&part.bake(&resolved_neutral.identity))[0],
                );
                assert!(far > SEEN, "{at}: the part moved {far} m");
            } else {
                let moved = rest(&model.bake(&resolved.identity));
                for (level, (a, b)) in moved.iter().zip(&at_rest).enumerate() {
                    let far = farthest(a, b);
                    assert!(far > SEEN, "{at}, level of detail {level}: moved {far} m");
                }
            }
            checked += 1;
        }
    }
    // Frame (one end), muscle and fat, sixteen regions, nine proportions
    // (the hips only widen: one end), and the face's 47 parameters (CO6's
    // 45 and the brows' length).
    assert_eq!(checked, 1 + 2 * (2 + 16 + 9) - 1 + 2 * 47);
}

/// The two whole-body sliders are not one slider twice: each corner of
/// the pair is its own body.
#[test]
fn muscle_and_fat_make_four_different_corners() {
    let library = library();
    let model = body(&library);
    let corner = |muscle: f32, fat: f32, frame: f32| {
        let mut description = Description::new("Corner", BASE);
        description.parameters.insert("muscle".into(), muscle);
        description.parameters.insert("fat".into(), fat);
        description.parameters.insert("frame".into(), frame);
        let resolved = resolve(&library, &description).unwrap();
        rest(&model.bake(&resolved.identity)).remove(0)
    };
    for frame in [0.0, 1.0] {
        let corners = [
            corner(-1.0, -1.0, frame),
            corner(-1.0, 1.0, frame),
            corner(1.0, -1.0, frame),
            corner(1.0, 1.0, frame),
        ];
        for a in 0..4 {
            for b in a + 1..4 {
                let far = farthest(&corners[a], &corners[b]);
                assert!(
                    far > 0.01,
                    "frame {frame}: corners {a} and {b} are {far} m apart"
                );
            }
        }
        // And the sum of the two alone isn't the corner: the pair has its own shape.
        let (both, muscle, fat, none) = (
            &corners[3],
            corner(1.0, 0.0, frame),
            corner(0.0, 1.0, frame),
            corner(0.0, 0.0, frame),
        );
        let off = both
            .iter()
            .zip(&muscle)
            .zip(&fat)
            .zip(&none)
            .map(|(((b, m), f), n)| {
                (0..3)
                    .map(|k| (b[k] - (m[k] + f[k] - n[k])).powi(2))
                    .sum::<f32>()
                    .sqrt()
            })
            .fold(0.0, f32::max);
        assert!(off > SEEN, "frame {frame}: the corner adds {off} m");
    }
}

/// What a description drives is baked and gone from the model's blend
/// shapes; what is left is live: the pose-driven corrective shapes (`anim`'s
/// `correctives.rs` sets them every pose) and the face's expression shapes
/// (CO6: `FaceControls` sets them), and nothing else.
#[test]
fn a_baked_body_keeps_only_its_live_shapes() {
    let library = library();
    let model = body(&library);
    let resolved = resolve(&library, &Description::new("Neutral", BASE)).unwrap();
    let baked = model.bake(&resolved.identity);
    let left: Vec<&str> = baked
        .morph_targets
        .iter()
        .map(|target| target.name.as_str())
        .collect();
    // Nothing the identity names is left.
    for layer in &resolved.identity.layers {
        for (shape, _) in &layer.shapes {
            assert!(
                !left.contains(&shape.as_str()),
                "`{shape}` is identity and was not baked"
            );
        }
    }
    let mut correctives: Vec<&str> = left
        .iter()
        .copied()
        .filter(|name| name.starts_with("Corrective_"))
        .collect();
    correctives.sort_unstable();
    assert_eq!(
        correctives,
        [
            "Corrective_Shoulder_Level_Left",
            "Corrective_Shoulder_Level_Right",
            "Corrective_Shoulder_Lowered_Left",
            "Corrective_Shoulder_Lowered_Right",
            "Corrective_Shoulder_Raised_Left",
            "Corrective_Shoulder_Raised_Right",
        ]
    );
    // The rest are the face's: no body or identity shape by its name.
    for name in &left {
        let identity_name = ["Frame_", "Masculine_", "Feminine_", "Face_"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
            || *name == "Pupil_Size";
        assert!(!identity_name, "`{name}` is left on the baked body");
    }
    // 6 corrective shapes and the 44 expression shapes (the 48 face
    // controls less the four eye looks, which are bones).
    assert_eq!(left.len(), 50, "{left:?}");
    // (One less than at the merge: the body no longer carries `Pupil_Size`,
    // which is the eyes' own shape and no slider's. Four more since: the
    // brows' height and arch have a raising shape each side besides the
    // lowering one, `faceparams.SPLIT`. Two more for the lowered shoulder,
    // `Corrective_Shoulder_Lowered_*`. Seven more on 2026-11-16: the
    // brows' length, four shapes the body carries empty for the `Brows`
    // card, and a mask shape under each of the three underwear items.)
    assert_eq!(model.morph_targets.len(), 155);
}
