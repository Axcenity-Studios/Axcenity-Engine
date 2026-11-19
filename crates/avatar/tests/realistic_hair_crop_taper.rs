//! The realistic family's hairstyle made from scratch against reference
//! photographs (Character Studio wave 5, decision 14): a short textured
//! crop with a tapered nape, groomed from guide curves by zone in Blender
//! (`assets/characters/realistic/hair/`, `guides.py`), its cards made from
//! the groom's clumps and its atlas drawn from the same strands.
//!
//! These tests hold what the files must say and carry: a card groom for
//! the realistic base, following the head's shapes on both frames; Hair
//! shading, cut out, two-sided, a shell over the head, within the budgets;
//! card tiles that end in tapered tips (no straight alpha cut: the
//! wave-4 crop's defect); references recorded with accepted licences; and
//! a state that never claims more than its measurements
//! (asset-quality.md §1, §4 and §5).

use std::path::{Path, PathBuf};

use axcenity_assets::{AlphaMode, Model, ModelFormat, ModelShading, Pick, load_models};
use axcenity_avatar::{Fit, ItemKind, ItemManifest, Library, Quality};

const STYLE: &str = "axcenity/realistic-hair-crop-taper";
const WAVE4: &str = "axcenity/realistic-hair-short-crop";
const BASE: &str = "axcenity/realistic";

/// The plan's hair budgets (section 7): hero cards, gameplay LOD0, crowd.
const HERO_TRIANGLES: usize = 40_000;
const GAMEPLAY_TRIANGLES: usize = 25_000;
const CROWD_TRIANGLES: usize = 3_000;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
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

/// What the build wrote beside the model (`hair/CropTaper.build.json`).
fn build_report() -> serde_json::Value {
    let path = root().join("realistic/hair/CropTaper.build.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    serde_json::from_str(&text).unwrap()
}

/// The style's measurements (`qa_hair.py` and the engine's captures).
fn measurements() -> serde_json::Value {
    let path = repository()
        .join("docs/development/character-studio/refinement/wave5/hair-scratch/measurements.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    serde_json::from_str(&text).unwrap()
}

#[test]
fn the_crop_is_a_new_card_groom_for_the_realistic_base_that_follows_the_head() {
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
    assert!(groom.strands.is_none(), "no strand tier");
    // Both frames and the head's identity shapes (fit by the head's own
    // shapes, as the first style).
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
    // Made here, from scratch: no third-party file in it.
    let source = item.source.as_ref().expect("a source");
    assert_eq!(source.licence, "CC0-1.0");
    // A draft until the review and Paul's sign-off (decision 12); the
    // wave-4 crop stays the draft it is.
    assert_eq!(item.quality, Some(Quality::Draft));
    let wave4 = library.item(WAVE4).expect("the wave-4 crop is kept");
    assert!(wave4.is_draft(), "the wave-4 crop stays a draft");
}

#[test]
fn its_model_is_hair_cut_out_two_sided_a_shell_and_within_the_budgets() {
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
        // Every layer lies on the head (a crop): its light fades over the
        // head's terminator, so a head lit from behind doesn't glow
        // through (docs/rendering/hair.md, shells).
        assert!(material.hair_shell, "the cards are marked a shell");
        assert!(material.base_color_texture.is_some(), "the atlas's colour");
        assert!(material.hair_data_texture.is_some(), "the atlas's data map");
        assert!(material.hair_flow_texture.is_some(), "the atlas's flow map");
        // The normal map gives the engine its tangents (hair.md).
        assert!(material.normal_texture.is_some(), "a normal map");
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

/// The atlas's alpha (0 to 255), with its size.
fn atlas_alpha(model: &Model) -> (Vec<u8>, usize, usize) {
    let colour = model.primitives[0]
        .material
        .base_color_texture
        .as_ref()
        .expect("a colour map");
    let rgba = colour.rgba();
    let alpha = rgba.chunks_exact(4).map(|p| p[3]).collect();
    (alpha, colour.width as usize, colour.height as usize)
}

/// Every card tile of the atlas ends in tapered tips, not a straight cut
/// (asset-quality.md §4, "no visible card edge or straight alpha cut"):
/// the strands drawn in a tile don't touch its sides (so no card's side
/// is an alpha edge), its tip rows cover much less than its middle, and
/// where the strands end varies across the tile (a straight cut would
/// end them all on one row). The tiles' places come from the build's
/// report (`atlas.tiles`, pixels, root at the top).
#[test]
fn its_atlas_tiles_end_in_tapered_tips_not_straight_cuts() {
    let library = library();
    let model = cards(&library, style(&library));
    let (alpha, width, _) = atlas_alpha(&model);
    let report = build_report();
    let tiles = report["atlas"]["tiles"].as_array().expect("atlas.tiles");
    assert!(tiles.len() >= 20, "{} tiles", tiles.len());
    let at = |x: usize, y: usize| alpha[y * width + x];
    for tile in tiles {
        let kind = tile["kind"].as_str().unwrap();
        let [x, y, w, h] = ["x", "y", "w", "h"].map(|k| tile[k].as_u64().unwrap() as usize);
        // The sides: the outermost two columns are clear.
        let side: usize = (y..y + h)
            .map(|row| usize::from(at(x, row) > 0) + usize::from(at(x + w - 1, row) > 0))
            .sum();
        assert!(
            side == 0,
            "{kind} tile at ({x}, {y}): {side} side texels covered"
        );
        // Coverage by row: the tip tenth against the middle fifth.
        let row_cover =
            |row: usize| (x..x + w).filter(|&c| at(c, row) >= 128).count() as f64 / w as f64;
        let mean = |rows: std::ops::Range<usize>| {
            let n = rows.len().max(1) as f64;
            rows.map(row_cover).sum::<f64>() / n
        };
        let middle = mean(y + 2 * h / 5..y + 3 * h / 5);
        let tip = mean(y + 9 * h / 10..y + h);
        assert!(
            tip < 0.5 * middle,
            "{kind} tile at ({x}, {y}): tips cover {tip:.3}, the middle {middle:.3}"
        );
        // Where each covered column's strands end.
        let ends: Vec<f64> = (x..x + w)
            .filter_map(|c| (y..y + h).rev().find(|&r| at(c, r) >= 128))
            .map(|r| (r - y) as f64)
            .collect();
        if ends.len() < 4 {
            continue;
        }
        let m = ends.iter().sum::<f64>() / ends.len() as f64;
        let spread =
            (ends.iter().map(|e| (e - m) * (e - m)).sum::<f64>() / ends.len() as f64).sqrt();
        assert!(
            spread > 0.04 * h as f64,
            "{kind} tile at ({x}, {y}): the strands end within {spread:.1} px of one row"
        );
    }
}

/// The references (decision 14) are recorded beside the other hair
/// references, by link, author and licence, with accepted licences only
/// (CC0, public domain, CC BY): at least eight, each from Openverse.
#[test]
fn its_references_are_recorded_with_accepted_licences() {
    let path = repository().join("docs/development/character-studio/references/hair/README.md");
    let text = std::fs::read_to_string(&path).unwrap();
    let start = text
        .find("crop-taper")
        .expect("a section for the crop's references");
    let section = &text[start..];
    let end = section[3..].find("\n## ").map_or(section.len(), |e| e + 3);
    let rows: Vec<&str> = section[..end]
        .lines()
        .filter(|line| line.contains("openverse.org/image/"))
        .collect();
    assert!(rows.len() >= 8, "{} references", rows.len());
    for row in rows {
        let lower = row.to_lowercase();
        assert!(
            lower.contains("cc0") || lower.contains("cc by ") || lower.contains("public domain"),
            "an accepted licence: {row}"
        );
        for refused in ["-sa", "by-nc", "-nd", "noncommercial"] {
            assert!(!lower.contains(refused), "{refused} refused: {row}");
        }
    }
}

/// The checks of `asset_quality_2` and `asset_quality_4` that failed.
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

/// Nothing marks the style production over its own evidence, and every
/// check of asset-quality §4 has a measurement with a verdict.
#[test]
fn it_stays_a_draft_while_a_measured_check_fails() {
    let library = library();
    let item = style(&library);
    let measured = measurements();
    assert_eq!(measured["style"], STYLE);
    for check in [
        "budgets",
        "volume",
        "scalp_coverage_geometric",
        "scalp_show_through_capture",
        "hairline_ramp",
        "taa_stability",
        "inside_head",
    ] {
        let verdict = &measured["asset_quality_4"][check]["pass"];
        assert!(
            verdict.is_boolean() || verdict == "review",
            "{check} has a verdict ({verdict})"
        );
    }
    let failed = failed_checks(&measured);
    if !failed.is_empty() {
        assert!(
            item.is_draft(),
            "production with failing checks: {failed:?}"
        );
    }
}

/// The browser's notes (plan 11.3) say only what was measured: where a
/// hair vertex went more than 1 mm inside the head (at a parameter's
/// ends, or in motion for the frame row), the note neither claims none
/// did nor hides the depth.
#[test]
fn its_limitations_claim_only_what_its_measurements_show() {
    let library = library();
    let item = style(&library);
    let measured = measurements();
    let inside = &measured["asset_quality_4"]["inside_head"];
    let ends = inside["rest_and_parameters"].as_object().unwrap();
    let motion = inside["motion"].as_object().unwrap();
    let worst = |entries: Vec<&serde_json::Value>| {
        let (mut count, mut deepest) = (0u64, 0f64);
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
    };
    for limitation in &item.limitations {
        let note = limitation.note.to_lowercase();
        let parameter = limitation.parameter.as_deref().unwrap_or("");
        let (count, deepest) = if parameter == "frame" {
            worst(motion.values().collect())
        } else {
            worst(
                ends.iter()
                    .filter(|(key, _)| {
                        key.split('/').nth(1).unwrap_or("").split('=').next() == Some(parameter)
                    })
                    .map(|(_, levels)| levels)
                    .collect(),
            )
        };
        if count > 0 {
            for claim in [
                "no hair inside",
                "no hair vertex",
                "no vertex",
                "none inside",
            ] {
                assert!(
                    !note.contains(claim),
                    "{parameter}: {count} vertices inside (deepest {deepest} mm), but {note:?}"
                );
            }
            let numbers: Vec<f64> = note
                .split(|c: char| !(c.is_ascii_digit() || c == '.'))
                .filter_map(|t| t.trim_end_matches('.').parse().ok())
                .collect();
            assert!(
                numbers.iter().any(|n| (n - deepest).abs() < 1.0),
                "{parameter}: the note doesn't give the depth {deepest} mm: {note:?}"
            );
        }
        if item.is_draft() && parameter == "frame" {
            assert!(!note.starts_with("fits"), "a draft doesn't claim it fits");
        }
    }
}
