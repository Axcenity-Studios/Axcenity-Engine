//! The realistic family's first hairstyle (Character Studio R6, CO11): a
//! card groom made by `assets/characters/realistic/hair/` and the item
//! manifest it writes. It is a groom of cards for the realistic base that
//! follows the head's shapes; its model marks the strands' material Hair,
//! cut out, two-sided and within the plan's triangle budgets; it offers
//! only the colour values its atlas carries; its colour presets run from
//! black to platinum and grey; and it says which frames and head ranges
//! it holds on (asset-quality.md §4, "a style declares only what it
//! supports").

use std::path::{Path, PathBuf};

use axcenity_assets::{AlphaMode, Model, ModelFormat, ModelShading, Pick, load_models};
use axcenity_avatar::{
    Fit, ItemKind, ItemManifest, Library, MaterialValue, MaterialValues, ShadingModel,
};

const STYLE: &str = "axcenity/realistic-hair-short-crop";
const BASE: &str = "axcenity/realistic";

/// The plan's hair budgets (section 7): hero cards, gameplay LOD0, crowd.
const HERO_TRIANGLES: usize = 40_000;
const GAMEPLAY_TRIANGLES: usize = 25_000;
const CROWD_TRIANGLES: usize = 3_000;

/// The colour presets every realistic style carries (brief §7.2): by
/// melanin and redness, darkest first.
const COLOURS: [&str; 8] = [
    "black",
    "dark-brown",
    "brown",
    "auburn",
    "ginger",
    "blonde",
    "platinum",
    "grey",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn library() -> Library {
    let (library, _, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{problems:?}");
    library
}

fn style(library: &Library) -> &ItemManifest {
    library
        .item(STYLE)
        .unwrap_or_else(|| panic!("the library has no `{STYLE}`"))
}

/// The groom's cards model, read with the engine's own loader.
fn cards(library: &Library, item: &ItemManifest) -> Model {
    let groom = item.groom.as_ref().expect("a groom");
    let model = groom.cards.as_ref().expect("cards");
    let file = library
        .path_of(&item.id, &model.file)
        .expect("in the library");
    let bytes = std::fs::read(root().join(&file)).unwrap_or_else(|err| panic!("{file}: {err}"));
    let pick = match &model.object {
        Some(object) => Pick::Objects(vec![object.clone()]),
        None => Pick::Whole,
    };
    load_models(&bytes, Path::new("."), ModelFormat::Gltf, &[pick])
        .unwrap_or_else(|err| panic!("{file}: {err}"))
        .remove(0)
}

fn triangles(model: &Model, level: usize) -> usize {
    model
        .primitives
        .iter()
        .map(|primitive| {
            if level == 0 {
                primitive.indices.len() / 3
            } else {
                primitive
                    .lods
                    .get(level - 1)
                    .map_or(0, |lod| lod.indices.len() / 3)
            }
        })
        .sum()
}

fn number(values: &MaterialValues, name: &str) -> f32 {
    match values.get(name) {
        Some(MaterialValue::Number(value)) => *value,
        _ => 0.0,
    }
}

#[test]
fn the_first_style_is_a_card_groom_for_the_realistic_base_that_follows_the_head() {
    let library = library();
    let item = style(&library);
    assert_eq!(item.kind, ItemKind::Groom);
    assert_eq!(item.slot, "hair");
    assert_eq!(item.bases, [BASE]);
    assert!(item.validate().is_empty(), "{:?}", item.validate());
    let base = library.base(BASE).unwrap();
    assert!(
        item.check_against(base).is_empty(),
        "{:?}",
        item.check_against(base)
    );
    let groom = item.groom.as_ref().unwrap();
    assert!(groom.cards.is_some(), "the card tier draws it");
    assert!(groom.strands.is_none(), "no strand tier yet");
    // It carries the head's identity shapes and the frame (fit by shapes,
    // as the realistic underwear), so a head parameter moves it.
    let Fit::Shapes { follows } = &item.fit else {
        panic!("fits by the head's shapes, not {:?}", item.fit)
    };
    for shape in [
        "Frame_Feminine",
        "Face_Head_Width",
        "Face_Forehead_Height",
        "Face_Forehead_Slope",
    ] {
        assert!(follows.iter().any(|s| s == shape), "follows {shape}");
    }
    // A draft until Paul signs it off (asset-quality.md §1): it says so.
    assert!(item.quality.is_some(), "states its quality");
    let source = item.source.as_ref().expect("a source");
    assert_eq!(source.licence, "CC0-1.0");
}

#[test]
fn its_model_is_hair_cut_out_two_sided_and_within_the_budgets() {
    let library = library();
    let item = style(&library);
    let model = cards(&library, item);
    assert!(!model.primitives.is_empty());
    assert!(model.rig.is_some(), "skinned to the body's skeleton");
    for primitive in &model.primitives {
        let material = &primitive.material;
        assert_eq!(material.shading, ModelShading::Hair, "{}", material.name);
        assert!(
            matches!(material.alpha_mode, AlphaMode::Mask { .. }),
            "{} is a cut-out",
            material.name
        );
        assert!(material.double_sided, "cards are seen from both sides");
        assert!(material.base_color_texture.is_some(), "the atlas's colour");
        assert!(material.hair_data_texture.is_some(), "the atlas's data map");
        // Lit along its strands: texture coordinates run root to tip.
        assert_eq!(primitive.uvs.len(), primitive.positions.len());
        assert!(!primitive.weights.is_empty(), "skinned");
        assert_eq!(primitive.lods.len(), 2, "its gameplay and crowd copies");
    }
    let (hero, gameplay, crowd) = (
        triangles(&model, 0),
        triangles(&model, 1),
        triangles(&model, 2),
    );
    eprintln!("triangles: hero {hero}, gameplay {gameplay}, crowd {crowd}");
    assert!(hero <= HERO_TRIANGLES && hero > gameplay, "hero {hero}");
    assert!(
        gameplay <= GAMEPLAY_TRIANGLES && gameplay > crowd,
        "gameplay {gameplay}"
    );
    assert!(crowd <= CROWD_TRIANGLES, "crowd {crowd}");
    // Every shape it follows is in the file.
    let Fit::Shapes { follows } = &item.fit else {
        unreachable!()
    };
    for shape in follows {
        assert!(
            model.morph_targets.iter().any(|t| &t.name == shape),
            "the cards carry {shape}"
        );
    }
}

/// What a groom's atlas carries, read from its data map over the texels
/// its colour map covers: whether the root-to-tip channel runs from root
/// to tip, and whether the streak mask marks some strands and not all.
fn atlas_carries(model: &Model) -> (bool, bool) {
    let material = &model.primitives[0].material;
    let (Some(colour), Some(data)) = (&material.base_color_texture, &material.hair_data_texture)
    else {
        return (false, false);
    };
    assert_eq!(
        (colour.width, colour.height),
        (data.width, data.height),
        "the data map is laid out as the colour map"
    );
    let (colour, data) = (colour.rgba(), data.rgba());
    let (mut low, mut high, mut streak, mut covered) = (255u8, 0u8, 0usize, 0usize);
    for (c, d) in colour.chunks_exact(4).zip(data.chunks_exact(4)) {
        if c[3] < 128 {
            continue;
        }
        covered += 1;
        low = low.min(d[0]);
        high = high.max(d[0]);
        if d[2] > 128 {
            streak += 1;
        }
    }
    let root_to_tip = low < 26 && high > 230;
    let streaks = streak > covered / 50 && streak < covered / 2;
    (root_to_tip, streaks)
}

#[test]
fn a_realistic_style_offers_only_the_colour_values_its_atlas_carries() {
    let library = library();
    let grooms: Vec<&ItemManifest> = library
        .items
        .values()
        .filter(|item| item.kind == ItemKind::Groom && item.bases.iter().any(|b| b == BASE))
        .collect();
    assert!(!grooms.is_empty(), "the realistic family has a hairstyle");
    for item in grooms {
        let model = cards(&library, item);
        let (root_to_tip, streaks) = atlas_carries(&model);
        for slot in &item.materials {
            assert_eq!(slot.shading, ShadingModel::Hair, "{}", item.id);
            // Declared, not "every value" by default.
            assert!(
                !slot.values.is_empty(),
                "{}: `{}` lists the values it offers",
                item.id,
                slot.name
            );
            for value in &slot.values {
                let needs_roots = matches!(
                    value.as_str(),
                    "root_melanin" | "root_length" | "tip_lightening"
                );
                let needs_streaks = matches!(value.as_str(), "streaks" | "streak_melanin");
                assert!(
                    !needs_roots || root_to_tip,
                    "{} offers `{value}` without a root-to-tip channel",
                    item.id
                );
                assert!(
                    !needs_streaks || streaks,
                    "{} offers `{value}` without a streak mask",
                    item.id
                );
            }
            // Every value a variant or a parameter sets is offered.
            for variant in &item.variants {
                for (name, values) in &variant.materials {
                    let slot = item.material(name).unwrap();
                    for value in values.keys() {
                        assert!(
                            slot.values.iter().any(|v| v == value),
                            "{}: variant `{}` sets `{value}`, which `{name}` doesn't offer",
                            item.id,
                            variant.id
                        );
                    }
                }
            }
        }
        // An item parameter must drive something the model has (no
        // control that does nothing, brief §14.1).
        for parameter in &item.parameters {
            for driver in &parameter.shapes {
                assert!(
                    model.morph_targets.iter().any(|t| t.name == driver.shape),
                    "{}: `{}` drives a shape the cards don't have",
                    item.id,
                    parameter.id
                );
            }
        }
    }
    // The first style carries both channels and offers roots and streaks.
    let item = style(&library);
    let (root_to_tip, streaks) = atlas_carries(&cards(&library, item));
    assert!(root_to_tip && streaks);
    let offered = &item.materials[0].values;
    for value in ["melanin", "redness", "root_melanin", "streaks"] {
        assert!(offered.iter().any(|v| v == value), "offers {value}");
    }
}

#[test]
fn its_colour_presets_run_from_black_to_platinum_and_grey() {
    let library = library();
    let item = style(&library);
    let hair = &item.materials[0].name;
    let ids: Vec<&str> = item.variants.iter().map(|v| v.id.as_str()).collect();
    for colour in COLOURS {
        assert!(ids.contains(&colour), "a `{colour}` preset ({ids:?})");
    }
    let values = |id: &str| item.variant(id).unwrap().materials[hair].clone();
    let melanin = |id: &str| number(&values(id), "melanin");
    let redness = |id: &str| number(&values(id), "redness");
    // Darkest to lightest along the melanin curve.
    let natural = ["black", "dark-brown", "brown", "blonde", "platinum"];
    for pair in natural.windows(2) {
        assert!(
            melanin(pair[0]) > melanin(pair[1]),
            "{} is darker than {}",
            pair[0],
            pair[1]
        );
    }
    // The red ones are red by pheomelanin, not by a dye.
    for red in ["auburn", "ginger"] {
        assert!(redness(red) >= 0.5, "{red} redness {}", redness(red));
        assert_eq!(number(&values(red), "dye_amount"), 0.0, "{red}");
    }
    assert!(redness("platinum") < 0.0 && redness("grey") < 0.0, "ash");
}

#[test]
fn it_says_which_frames_and_head_ranges_it_holds_on() {
    let library = library();
    let item = style(&library);
    let base = library.base(BASE).unwrap();
    let held = |id: &str| {
        item.limitations
            .iter()
            .find(|l| l.parameter.as_deref() == Some(id))
            .and_then(|l| l.range)
            .unwrap_or_else(|| panic!("a range for `{id}`"))
    };
    assert_eq!(held("frame"), [0.0, 1.0], "both frames");
    for id in [
        "head_size",
        "temple_width",
        "forehead_height",
        "forehead_slope",
    ] {
        let [low, high] = held(id);
        let range = base.parameter(id).unwrap().range;
        assert!(
            range[0] <= low && low < high && high <= range[1],
            "{id}: {low} to {high} inside {range:?}"
        );
    }
}

/// Stubble's density map (R6, ADR-0042's spare channel): the body's
/// second mask holds it in its alpha, over the beard's region (the chin
/// and jaw, the upper lip) and nowhere on the forehead or the scalp.
#[test]
fn the_spare_mask_channel_holds_stubble_over_the_beard_only() {
    let bytes = std::fs::read(root().join("realistic/RealisticParts.glb")).unwrap();
    let body = load_models(
        &bytes,
        Path::new("."),
        ModelFormat::Gltf,
        &[Pick::Objects(vec!["AxcenityRealistic/Body".into()])],
    )
    .unwrap()
    .remove(0);
    let skin = body
        .primitives
        .iter()
        .find(|p| p.material.name == "Skin")
        .expect("the skin");
    let mask = &skin.material.skin_masks[1];
    let pixels = mask.rgba();
    let (w, h) = (mask.width as usize, mask.height as usize);
    // The mean alpha at the corners inside a box (the model's space: Y
    // up, the face toward +Z; it is fitted, so distances are measured as
    // shares of the body's own height).
    let top = skin.positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    let bottom = skin.positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
    let mean_in = |x: f32, low: f32, high: f32, front: f32| {
        let (mut sum, mut count) = (0.0, 0usize);
        for (p, uv) in skin.positions.iter().zip(&skin.uvs) {
            let below = top - p[1];
            if p[0].abs() < x && below >= low && below <= high && p[2] > front {
                let px = ((uv[0].rem_euclid(1.0)) * w as f32) as usize % w;
                let py = ((uv[1].rem_euclid(1.0)) * h as f32) as usize % h;
                sum += f32::from(pixels[(py * w + px) * 4 + 3]) / 255.0;
                count += 1;
            }
        }
        assert!(count > 3, "corners in the box");
        sum / count as f32
    };
    // Heights below the top of the head, as shares of the body's height
    // (the realistic body is 1.75 m): the chin about 0.23 m down, the
    // forehead about 0.07, the crown the top 0.02.
    let scale = (top - bottom) / 1.75;
    let chin = mean_in(0.03 * scale, 0.215 * scale, 0.245 * scale, 0.0);
    let forehead = mean_in(0.03 * scale, 0.05 * scale, 0.08 * scale, 0.0);
    let crown = mean_in(0.04 * scale, 0.0, 0.02 * scale, f32::MIN);
    eprintln!("stubble: chin {chin:.3}, forehead {forehead:.3}, crown {crown:.3}");
    assert!(chin > 0.2, "the chin has stubble: {chin}");
    assert!(
        forehead < 0.01 && crown < 0.01,
        "none above: {forehead}, {crown}"
    );
}

/// The style's measurements (`qa_hair.py` and the captures), committed
/// with its evidence.
fn measurements() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../docs/development/character-studio/refinement/wave4/r6-hair-first/measurements.json",
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    serde_json::from_str(&text).unwrap()
}

/// Every check of `asset_quality_2` and `asset_quality_4` that failed.
fn failed_checks(measured: &serde_json::Value) -> Vec<String> {
    let mut failed = Vec::new();
    for section in ["asset_quality_2", "asset_quality_4"] {
        for (name, check) in measured[section].as_object().unwrap() {
            if check["pass"] == serde_json::Value::Bool(false) {
                failed.push(format!("{section}.{name}"));
            }
        }
    }
    failed
}

/// A style stays a draft while any measured check fails (asset-quality
/// §1): nothing may mark it production over its own evidence.
#[test]
fn it_stays_a_draft_while_a_measured_check_fails() {
    let library = library();
    let item = style(&library);
    let failed = failed_checks(&measurements());
    if !failed.is_empty() {
        assert!(
            item.is_draft(),
            "production with failing checks: {failed:?}"
        );
    }
}

/// The worst of some `inside_head` entries (each maps a level, such as
/// `Hair_ShortCrop_LOD1`, to `over_1mm` and `deepest_mm`): how many
/// vertices sit over 1 mm inside, and the deepest of those, in mm.
fn worst_inside<'a>(entries: impl Iterator<Item = &'a serde_json::Value>) -> (u64, f64) {
    let mut count = 0;
    let mut deepest: f64 = 0.0;
    for levels in entries {
        for level in levels.as_object().unwrap().values() {
            let over = level["over_1mm"].as_u64().unwrap();
            if over > 0 {
                count += over;
                deepest = deepest.max(level["deepest_mm"].as_f64().unwrap());
            }
        }
    }
    (count, deepest)
}

/// Every number written in a note ("6 to 7 mm", "7.2 mm", "5.7%").
fn numbers_in(note: &str) -> Vec<f64> {
    note.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter_map(|token| token.trim_end_matches('.').parse().ok())
        .collect()
}

/// A note that says no hair goes inside the head, in any of the wordings
/// the build has used ("no hair inside", "no hair vertex more than 1 mm
/// inside", ...).
fn claims_none_inside(note: &str) -> bool {
    [
        "no hair inside",
        "no hair vertex",
        "no vertex",
        "nothing inside",
        "none inside",
    ]
    .iter()
    .any(|claim| note.contains(claim))
}

/// The limitations the browser shows (plan 11.3) say only what was
/// measured. Where a vertex was measured more than 1 mm inside the head
/// (at a parameter's ends, or in motion for the frame row), the note may
/// not claim none is, in any wording, and it must give the measured depth
/// (a number within 1 mm of the deepest). No note claims "no scalp
/// through" while the captured show-through fails, and the frame row
/// doesn't say the style fits while it is a draft.
#[test]
fn its_limitations_claim_only_what_its_measurements_show() {
    let library = library();
    let item = style(&library);
    let measured = measurements();
    let quality = &measured["asset_quality_4"];
    let ends = quality["inside_head"]["rest_and_parameters"]
        .as_object()
        .unwrap();
    let motion = quality["inside_head"]["motion"].as_object().unwrap();
    for limitation in &item.limitations {
        let note = limitation.note.to_lowercase();
        let parameter = limitation.parameter.as_deref().unwrap_or("");
        // The frame row speaks for motion; a parameter row for its ends,
        // on either frame and at any level. Keys are "frame/what".
        let (inside, deepest) = if parameter == "frame" {
            worst_inside(motion.values())
        } else {
            worst_inside(
                ends.iter()
                    .filter(|(key, _)| {
                        key.split('/').nth(1).unwrap_or("").split('=').next() == Some(parameter)
                    })
                    .map(|(_, levels)| levels),
            )
        };
        if inside > 0 {
            assert!(
                !claims_none_inside(&note),
                "{parameter}: {inside} vertices inside (deepest {deepest} mm), but the note says {note:?}"
            );
            assert!(
                numbers_in(&note).iter().any(|n| (n - deepest).abs() < 1.0),
                "{parameter}: the note doesn't give the measured depth {deepest} mm: {note:?}"
            );
        }
        if quality["scalp_show_through_capture"]["pass"] == serde_json::Value::Bool(false) {
            assert!(
                !note.contains("no scalp"),
                "{parameter}: show-through fails, but the note says {note:?}"
            );
        }
        if item.is_draft() && parameter == "frame" {
            assert!(
                !note.starts_with("fits"),
                "a draft doesn't claim it fits: {note:?}"
            );
        }
    }
}

/// Randomize never wears a draft (asset-quality.md §5) on either
/// realistic frame: 1,000 seeds from each frame's average cast member.
#[test]
fn randomize_never_wears_a_draft_on_either_realistic_frame() {
    use axcenity_avatar::{Description, Quality, Scope, randomize};
    let library = library();
    let none = std::collections::BTreeSet::new();
    for frame in ["masculine", "feminine"] {
        let path = root().join(format!(
            "benchmarks/realistic-{frame}-cast-average.character.json"
        ));
        let start = Description::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for seed in 0..1000 {
            let character = randomize(&start, &Scope::All, &none, seed, &library);
            for worn in &character.items {
                let item = library.item(&worn.id).unwrap();
                assert_ne!(
                    item.quality,
                    Some(Quality::Draft),
                    "{frame} seed {seed} wears the draft {}",
                    worn.id
                );
            }
        }
    }
}
