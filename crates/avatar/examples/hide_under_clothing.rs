//! Works out what an item of a character library hides under it (wave 5,
//! hiding the body under clothing: `axcenity_avatar::fit::hide_under`)
//! and writes the masks beside its manifest:
//!
//! ```text
//! cargo run -p axcenity-avatar --release --example hide_under_clothing -- \
//!     <library folder> <item ID> [--under <item ID>]... [--outfit <item ID>,<item ID>...]...
//!     [--bodies <folder>]...
//! ```
//!
//! The item's model is taken as it ships (fitted), over its base body
//! and the items named with `--under` or `--outfit` (the layers worn
//! under it; an outfit is layers worn together, each checked on its own:
//! the feminine frame's bra and briefs, the masculine frame's boxer
//! briefs), in the
//! fit's poses with the runtime's twist bones, on the body states the fit
//! poses (`pose_states_of`) and the cast and body benchmarks in each
//! `--bodies` folder (`assets/characters/benchmarks`: the fit gate's
//! bodies; their shapes only, not their bone proportions). It writes `<Object>.hide.png` (the body) and
//! `<Object>.hide.<layer>.png` beside the manifest and prints the
//! manifest's `hide_maps` and what the rule kept drawn. The manifest
//! itself isn't rewritten: the item's own build (`clothing/wardrobe.py`,
//! `import/fit.py`) writes it, and new imports (`import_garment`) make
//! their masks themselves.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use axcenity_assets::Assets;
use axcenity_avatar::fit::{
    BodyMesh, HIDE_RULE, HideInput, HideRule, hide_poses, hide_under, is_mask_shape, layer_masks,
    pose_states_of, write_hide_maps,
};
use axcenity_avatar::{Description, Library, body_shape_weights};

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut take = |flag: &str| {
        let mut found = Vec::new();
        while let Some(at) = args.iter().position(|a| a == flag) {
            if at + 1 < args.len() {
                found.push(args.remove(at + 1));
            }
            args.remove(at);
        }
        found
    };
    let mut under = take("--under");
    // `--outfit a,b`: layers worn together (each also under).
    let outfits: Vec<Vec<String>> = take("--outfit")
        .iter()
        .map(|o| o.split(',').map(str::to_owned).collect())
        .collect();
    for id in outfits.iter().flatten() {
        if !under.contains(id) {
            under.push(id.clone());
        }
    }
    let bodies = take("--bodies");
    let [library, item] = match <[String; 2]>::try_from(args) {
        Ok(positional) => positional,
        Err(_) => {
            eprintln!(
                "usage: hide_under_clothing <library> <item ID> [--under <item ID>]... [--bodies <folder>]..."
            );
            return ExitCode::FAILURE;
        }
    };
    match run(Path::new(&library), &item, &under, &outfits, &bodies) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("couldn't work out the masks: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    root: &Path,
    id: &str,
    under: &[String],
    outfits: &[Vec<String>],
    bodies: &[String],
) -> Result<(), String> {
    let (library, _, problems) = Library::read(root);
    if !problems.is_empty() {
        return Err(format!("the library has problems: {problems:?}"));
    }
    let assets = Assets::new(root);
    let mesh_of = |id: &str| -> Result<(BodyMesh, PathBuf, String), String> {
        let item = library.item(id).ok_or(format!("no item `{id}`"))?;
        let model = item.model.as_ref().ok_or(format!("`{id}` has no model"))?;
        let file = library
            .path_of(id, &model.file)
            .ok_or(format!("`{id}`'s model isn't in the library"))?;
        let object = model.object.clone().unwrap_or_default();
        let mesh_id = if object.is_empty() {
            file
        } else {
            format!("{file}#{object}")
        };
        let loaded = assets
            .model_blocking(&mesh_id)
            .ok_or(format!("`{mesh_id}` doesn't load"))?;
        let manifest = library.manifest_path(id).ok_or("no manifest path")?;
        let folder = root.join(Path::new(manifest).parent().unwrap_or(Path::new("")));
        let name = object.rsplit('/').next().unwrap_or(id).to_owned();
        Ok((
            BodyMesh::from_model(&loaded).map_err(|e| e.to_string())?,
            folder,
            name,
        ))
    };
    let item = library.item(id).ok_or(format!("no item `{id}`"))?;
    let base_id = item.bases.first().ok_or("the item fits no base")?;
    let base = library
        .base(base_id)
        .ok_or(format!("no base `{base_id}`"))?;
    let body_file = library
        .path_of(base_id, &base.model.file)
        .ok_or("the base's model isn't in the library")?;
    let body_model = assets
        .model_blocking(&format!("{body_file}#{}", base.model.body))
        .ok_or("the body doesn't load")?;
    let body = BodyMesh::from_model(&body_model).map_err(|e| e.to_string())?;
    let (garment, folder, object) = mesh_of(id)?;
    let layers: Vec<(String, BodyMesh)> = under
        .iter()
        .map(|u| mesh_of(u).map(|(mesh, _, _)| (u.clone(), mesh)))
        .collect::<Result<_, _>>()?;
    let layer_meshes: Vec<&BodyMesh> = layers.iter().map(|(_, m)| m).collect();
    let poses = hide_poses(
        &body.rig,
        Some(|rig, pose| axcenity_anim::twist::drive(rig, pose, &mut [])),
    );
    let mut states = pose_states_of(base, &library);
    // The character descriptions in each `--bodies` folder on this base
    // whose names say they are a cast or body benchmark (`-cast-`,
    // `-body-`): the bodies the fit gate checks.
    for folder in bodies {
        let mut names: Vec<PathBuf> = std::fs::read_dir(folder)
            .map_err(|e| format!("{folder}: {e}"))?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                (name.contains("-cast-") || name.contains("-body-"))
                    && name.ends_with(".character.json")
            })
            .collect();
        names.sort();
        for path in names {
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let description = Description::from_json(&text).map_err(|e| e.to_string())?;
            if description.base != base.id {
                continue;
            }
            let weights: Vec<(String, f32)> = body_shape_weights(base, &description, &library)
                .into_iter()
                .filter(|(name, weight)| *weight != 0.0 && !is_mask_shape(name))
                .collect();
            if !states.contains(&weights) {
                states.push(weights);
            }
        }
    }
    let started = std::time::Instant::now();
    let (hidden, report) = hide_under(&HideInput {
        body: &body,
        garment: &garment,
        layers: &layer_meshes,
        poses: &poses,
        states: &states,
        layer_shapes: &under
            .iter()
            .map(|u| layer_masks(base, &library, std::slice::from_ref(u)))
            .collect::<Vec<_>>(),
        outfits: &outfits
            .iter()
            .map(|o| {
                o.iter()
                    .filter_map(|id| under.iter().position(|u| u == id))
                    .collect()
            })
            .collect::<Vec<_>>(),
        rule: rule(),
    });
    let named: Vec<(&str, &BodyMesh)> = layers.iter().map(|(u, m)| (u.as_str(), m)).collect();
    let maps =
        write_hide_maps(&folder, &object, &body, &named, &hidden).map_err(|e| e.to_string())?;
    println!(
        "{id}: {} cases in {:.1} s; {report:?}",
        report.cases,
        started.elapsed().as_secs_f32()
    );
    println!(
        "\"hide_maps\": {}",
        serde_json::to_string_pretty(&maps).map_err(|e| e.to_string())?
    );
    Ok(())
}

/// The rule, with `AXCENITY_HIDE_RULE=<enclose>,<near>,<opening>`
/// in its place for measuring another (never for a shipped item).
fn rule() -> HideRule {
    let Ok(text) = std::env::var("AXCENITY_HIDE_RULE") else {
        return HIDE_RULE;
    };
    let v: Vec<f32> = text
        .split(',')
        .filter_map(|x| x.trim().parse().ok())
        .collect();
    match v.as_slice() {
        [enclose, near, opening] => HideRule {
            enclose: *enclose,
            near: *near,
            opening: *opening,
        },
        _ => HIDE_RULE,
    }
}
