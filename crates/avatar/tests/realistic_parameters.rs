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
//! `tests/realistic_proportions.rs`. The hips below 1 move no bone (R3): a
//! shape narrows them, checked on the baked body like the shape sliders.

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

/// The rest positions of `model`'s first primitive at each level of detail,
/// in metres. (A model's positions are its box scaled to one on each axis;
/// `size` is the box in the file, in metres for this one. Until R4 this
/// compared the scaled positions, which made a move up count 1.75 times
/// less than it is and a move forward three times more.)
fn rest(model: &Model) -> Vec<Vec<[f32; 3]>> {
    let rig = model.rig.as_ref().unwrap();
    let posed = model.pose_shaped(&rig.rest_pose(), &[]).unwrap();
    let size = model.size;
    (0..=model.primitives[0].lods.len())
        .map(|level| {
            posed
                .primitive(model, 0, level)
                .positions
                .iter()
                .map(|p| [p[0] * size[0], p[1] * size[1], p[2] * size[2]])
                .collect()
        })
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
            // Regional body adjustments are the detailed level (brief §3.3).
            assert!(!parameter.advanced, "{id}");
            assert_eq!(parameter.level(), axcenity_avatar::Level::Detailed, "{id}");
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
    let mut proportions = 0;
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
            // A proportion changes its body slider, except where a shape
            // does its work instead (the hips below 1, R3): then it has to
            // move the baked body, which a body slider alone doesn't.
            // (The nearest end is the feet's, 0.96: a 4% smaller foot.)
            let slider_moves = parameter.attributes.iter().any(|driver| {
                let was = resolved_neutral.attributes[&driver.attribute];
                (resolved.attributes[&driver.attribute] - was).abs() > 0.03
            });
            for driver in &parameter.attributes {
                assert!(
                    BODY_SLIDERS.contains(&driver.attribute.as_str()),
                    "{at}: {}",
                    driver.attribute
                );
            }
            if slider_moves {
                proportions += 1;
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
    // (both ends: the hips narrow too since R3), and the face's 61
    // parameters (CO6's 45, the brows' length, and R4's 14: nose tip
    // projection and rotation, nose width, philtrum length, and a pair each
    // of jaw angle, upper lid fold, lower lid, eye corner and earlobe).
    assert_eq!(checked, 1 + 2 * (2 + 16 + 9) + 2 * 61);
    // Every proportion's end moves its body slider but the hips' low end,
    // which moves the body by a shape.
    assert_eq!(proportions, 2 * 9 - 1);
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
    // Three more in R3: `Masculine_Fat_Soft`, `Feminine_Fat_Soft` (fat that
    // is fat) and `Masculine_Belly_Flat` (the slight body's belly); two more
    // for the wider ranges, `<Frame>_Hips_Narrow` (the hips below 1).
    // Fourteen more in R4: the face's new controls, one shape each. Seven
    // more in wave 3: R3's `<Frame>_Waist_Wide` and `_Narrow` (the waist
    // slider's shape) and `<Frame>_Fat_Over_Muscle` (heavy and muscular),
    // and R4's `Face_Nose_Tip_Rotation_Up` (`nose_tip_rotation`'s upturned
    // end, `faceparams.SPLIT`).
    assert_eq!(model.morph_targets.len(), 181);
}

/// The realistic body's underwear comes with Male or Female and is always
/// worn: a description that leaves it out, swaps it or wears the other
/// frame's still resolves to the chosen frame's underwear.
#[test]
fn the_underwear_is_always_worn_and_follows_the_frame() {
    let library = library();
    let worn = |description: &Description| -> Vec<String> {
        let resolved = resolve(&library, description).unwrap();
        let mut items: Vec<String> = resolved
            .parts
            .iter()
            .filter_map(|p| p.item.clone())
            .collect();
        items.sort();
        items
    };
    assert_eq!(
        axcenity_avatar::chosen_slots(&library, BASE),
        ["underwear_bottom", "underwear_top"]
    );

    let mut male = Description::new("Bare", BASE);
    assert_eq!(worn(&male), ["axcenity/realistic-boxer-briefs"]);
    male.items = vec![axcenity_avatar::WornItem::new(
        "axcenity/realistic-sports-bra",
    )];
    assert_eq!(worn(&male), ["axcenity/realistic-boxer-briefs"]);

    let mut female = Description::new("Bare", BASE);
    female.parameters.insert("frame".to_owned(), 1.0);
    assert_eq!(
        worn(&female),
        ["axcenity/realistic-briefs", "axcenity/realistic-sports-bra"]
    );
    female.items = vec![axcenity_avatar::WornItem::new(
        "axcenity/realistic-boxer-briefs",
    )];
    assert_eq!(
        worn(&female),
        ["axcenity/realistic-briefs", "axcenity/realistic-sports-bra"]
    );

    // Already dressed, nothing changes.
    let mut dressed = female.clone();
    axcenity_avatar::wear_choices(&mut dressed, &library);
    let again = dressed.clone();
    axcenity_avatar::wear_choices(&mut dressed, &library);
    assert_eq!(dressed, again);
}

/// The skin's normal map shows its defined (lean, muscular) layer only on
/// lean, muscular bodies: `muscle` drives the Skin's `definition` and
/// `fat` its `softness`, and the renderer shows `definition × (1 −
/// softness)` of it (R5, OPEN-07). Heavy bellies get none.
#[test]
fn muscle_and_fat_drive_how_much_of_the_skins_definition_shows() {
    use axcenity_avatar::MaterialValue;
    let library = library();
    let shown = |muscle: f32, fat: f32| {
        let mut description = Description::new("Build", BASE);
        description.parameters.insert("muscle".into(), muscle);
        description.parameters.insert("fat".into(), fat);
        let resolved = resolve(&library, &description).unwrap();
        assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
        let skin = &resolved.materials["Skin"];
        let value = |name: &str| match skin.get(name) {
            Some(MaterialValue::Number(n)) => *n,
            other => panic!("Skin {name}: {other:?}"),
        };
        value("definition") * (1.0 - value("softness"))
    };
    // The benchmark cast's bodies (muscle, fat).
    let athletic = shown(0.7, -0.4);
    let heavyset = shown(0.2, 0.8);
    let average = shown(0.0, 0.0);
    let slim = shown(-0.2, -0.6);
    let heavy_muscular = shown(1.0, 1.0);
    assert!(athletic > 0.7, "athletic: {athletic}");
    assert!(heavyset < 0.05, "heavyset: {heavyset}");
    assert!(
        heavy_muscular < 0.05,
        "heavy and muscular: {heavy_muscular}"
    );
    assert!(average < 0.3, "average: {average}");
    assert!(slim < 0.3, "slim, not muscular: {slim}");
}
