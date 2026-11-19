//! Trims a shipped item's hide masks to the runtime (wave 5, hiding the
//! body under clothing; `docs/development/character-studio/refinement/
//! wave5/body-hiding.md`). The masks are made by the enclosure rule in
//! the fit's poses (`avatar::fit::hide_under`, `examples/hide_under_clothing.rs`);
//! the runtime's walk, run and idle move the cloth a little differently,
//! and the fit gate's hole check, which looks in the runtime's poses,
//! finds the few hidden places that then show. This plays what the gate
//! and the motion check play, on the bodies they check, collects every
//! hidden triangle whose place shows ([`holes_found`]), and draws them
//! again: drawing a triangle only covers more, so what is left shows
//! nowhere in those poses. Not a test: a step of making an item, run by
//! hand after the masks are made:
//!
//! ```text
//! [AXCENITY_TRIM_VIEWS=gate] [AXCENITY_TRIM_ONLY=<body>] AXCENITY_TRIM=tee-crew|wardrobe|test-tee cargo test -p axcenity-player --release -- --ignored trim_hide_maps --nocapture
//! ```

use std::collections::{HashMap, HashSet};

use axcenity_assets::{Assets, RegionHiding};
use axcenity_avatar::fit::{BODY_MASK, BodyMesh, LAYER_MASK, write_mask};
use axcenity_avatar::{Description, LIBRARY_FOLDER, Library, WornItem};

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};
use super::fit_gate_tests::{FoundHole, bodies, hidden_triangles, holes_found, undressed, worn_in};

/// The outfits the gate checks, by name: the items worn over the
/// underwear each frame wears by default.
fn outfit(name: &str) -> Vec<&'static str> {
    match name {
        "tee-crew" => vec![super::tee_crew_tests::TEE],
        "test-tee" => vec![super::fit_gate_tests::TEE],
        "wardrobe" => vec![
            super::wardrobe_tests::TEE,
            super::wardrobe_tests::JOGGERS,
            super::wardrobe_tests::SNEAKERS,
        ],
        other => panic!("no outfit `{other}`"),
    }
}

/// A model's full-detail triangles as the gate's `Posed` numbers them.
fn triangles_of(model: &axcenity_assets::Model) -> Vec<[u32; 3]> {
    let mut out = Vec::new();
    let mut start = 0u32;
    for primitive in &model.primitives {
        out.extend(
            primitive
                .indices
                .chunks_exact(3)
                .map(|t| [t[0] + start, t[1] + start, t[2] + start]),
        );
        start += primitive.positions.len() as u32;
    }
    out
}

/// Every hidden place that shows on `dressed` in `stance`, played on to
/// each frame of `frames` for a clip.
fn seen_in(
    assets: &Assets,
    dressed: &Description,
    bare: &Description,
    stance: Stance,
    frames: std::ops::Range<u32>,
    out: &mut Vec<FoundHole>,
) {
    let (rest_world, rest_animators) =
        stage_world(assets, bare, Stance::Rest, &[]).expect("the bare body");
    let (bare_body, _, _) = worn_in(&rest_world, assets, &rest_animators);
    let (world_at_rest, animators_at_rest) =
        stage_world(assets, dressed, Stance::Rest, &[]).expect("the dressed body");
    let (dressed_body, _, _) = worn_in(&world_at_rest, assets, &animators_at_rest);
    let hidden = hidden_triangles(&bare_body, &dressed_body);
    let (mut world, mut animators) = stage_world(assets, dressed, stance, &[]).expect("the pose");
    for frame in frames.clone() {
        let (body, garments, _) = worn_in(&world, assets, &animators);
        out.extend(holes_found(&body, &hidden, &garments, fine_views()));
        if frame + 1 < frames.end {
            animators.update(&mut world, assets, 1.0 / 60.0);
        }
    }
}

/// Whether to look from `Views::fine` (the default) or the gate's own
/// views (`AXCENITY_TRIM_VIEWS=gate`: an outfit of several garments has
/// so many openings that the fine views take hours a body).
fn fine_views() -> bool {
    std::env::var("AXCENITY_TRIM_VIEWS").as_deref() != Ok("gate")
}

#[test]
#[ignore = "a step of making an item: run by hand (see the module's notes)"]
fn trim_hide_maps_to_the_runtime() {
    let Ok(which) = std::env::var("AXCENITY_TRIM") else {
        println!("AXCENITY_TRIM names no outfit: nothing trimmed");
        return;
    };
    for name in which.split(',') {
        // The test tee is the gate's own import, in the gate's library:
        // its masks are kept beside its source (`fit_gate_tests::TEE_MASKS`).
        let shipped = if name == "test-tee" {
            super::fit_gate_tests::gate_library().to_owned()
        } else {
            repository_library()
        };
        let (library, _, _) = Library::read(&shipped);
        let items = outfit(name);
        let stage = Stage::new(&format!("hide-trim-{name}"), &shipped).expect("the stage");
        let mut seen: Vec<FoundHole> = Vec::new();
        let started = std::time::Instant::now();
        // `AXCENITY_TRIM_ONLY`: the bodies whose names hold this text.
        let only = std::env::var("AXCENITY_TRIM_ONLY").unwrap_or_default();
        for description in bodies().into_iter().filter(|d| d.name.contains(&only)) {
            let mut dressed = description.clone();
            for id in &items {
                dressed.items.push(WornItem::new(*id));
            }
            let bare = undressed(&description);
            let before = seen.len();
            let held = [
                Stance::Rest,
                Stance::Idle,
                Stance::Held(HeldPose::TPose),
                Stance::Held(HeldPose::ArmsRaised),
                Stance::Held(HeldPose::ElbowsBent),
                Stance::Held(HeldPose::KneesBent),
                Stance::Held(HeldPose::ArmsForward),
            ];
            for stance in held {
                seen_in(&stage.assets, &dressed, &bare, stance, 0..1, &mut seen);
            }
            // The walk's whole stride on every body; the idle and run as
            // the motion check plays them, on the cast.
            let mut clips = vec![("Walk", 2..45)];
            if description.name.contains("-cast-") {
                clips.push(("Run", 2..35));
                clips.push(("Idle", 2..62));
            }
            for (clip, frames) in clips {
                let stance = Stance::Clip {
                    name: clip,
                    frames: frames.start,
                };
                seen_in(&stage.assets, &dressed, &bare, stance, frames, &mut seen);
            }
            println!(
                "{name} {}: {} places seen ({:.0} s)",
                description.name,
                seen.len() - before,
                started.elapsed().as_secs_f32()
            );
        }
        // The triangles to draw again, per mesh.
        let mut redraw: HashMap<String, HashSet<[u32; 3]>> = HashMap::new();
        for hole in &seen {
            redraw
                .entry(hole.mesh.clone())
                .or_default()
                .extend(hole.triangles.iter().copied());
        }
        for (mesh, triangles) in &redraw {
            println!("{name}: {mesh}: {} triangles drawn again", triangles.len());
        }
        trim(&stage.assets, &library, &shipped, &items, &redraw);
        if name == "test-tee" {
            let folder = shipped.join(super::fit_gate_tests::TEE);
            for mask in super::fit_gate_tests::TEE_MASKS {
                if folder.join(mask).is_file() {
                    std::fs::copy(
                        folder.join(mask),
                        super::fit_gate_tests::tee_fixtures().join(mask),
                    )
                    .unwrap();
                }
            }
        }
    }
}

/// Writes again each mask of `items` without the triangles of `redraw`
/// (by mesh name: `Body`, or a lower layer's part).
fn trim(
    assets: &Assets,
    library: &Library,
    shipped: &std::path::Path,
    items: &[&str],
    redraw: &HashMap<String, HashSet<[u32; 3]>>,
) {
    let base = library.base("axcenity/realistic").unwrap();
    let body_file = library.path_of(&base.id, &base.model.file).unwrap();
    for id in items {
        let item = library.item(id).unwrap();
        for hide in &item.hide_maps {
            let (target, mesh_id) = match &hide.under {
                None => (
                    "Body".to_owned(),
                    format!("{LIBRARY_FOLDER}/{body_file}#{}", base.model.body),
                ),
                Some(under) => {
                    let lower = library.item(under).unwrap();
                    let model = lower.model.as_ref().unwrap();
                    let object = model.object.clone().unwrap_or_default();
                    let file = library.path_of(under, &model.file).unwrap();
                    let part = object.rsplit('/').next().unwrap_or(&object).to_owned();
                    (part, format!("{LIBRARY_FOLDER}/{file}#{object}"))
                }
            };
            let Some(redrawn) = redraw.get(&target) else {
                continue;
            };
            let path = library.path_of(id, &hide.map).unwrap();
            let full = assets.model_blocking(&mesh_id).unwrap();
            let masked = assets.hide_regions(
                &mesh_id,
                &std::sync::Arc::new(RegionHiding {
                    map: String::new(),
                    hidden: Vec::new(),
                    masks: vec![format!("{LIBRARY_FOLDER}/{path}")],
                }),
            );
            let drawn: HashSet<[u32; 3]> = triangles_of(&assets.model_blocking(&masked).unwrap())
                .into_iter()
                .collect();
            let all = triangles_of(&full);
            let hidden: Vec<bool> = all
                .iter()
                .map(|t| !drawn.contains(t) && !redrawn.contains(t))
                .collect();
            let before = all.iter().filter(|t| !drawn.contains(*t)).count();
            let after = hidden.iter().filter(|h| **h).count();
            let mesh = BodyMesh::from_model(&full).unwrap();
            let size = if hide.under.is_none() {
                BODY_MASK
            } else {
                LAYER_MASK
            };
            write_mask(
                &shipped.join(&path),
                size,
                &mesh.uvs,
                &mesh.triangles,
                &hidden,
            )
            .unwrap();
            println!("{id}: {path}: {before} hidden, {after} after trimming");
        }
    }
}
