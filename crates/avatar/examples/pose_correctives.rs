//! A garment's pose correctives, baked from cloth simulation (wave 5;
//! `axcenity_avatar::fit::cloth`), from the command line. The intake's
//! `assets/characters/realistic/import/correctives.py` runs it around
//! Blender's cloth simulation:
//!
//! ```text
//! cargo run -p axcenity-avatar --release --example pose_correctives -- dump <library> <item ID> <work folder>
//! cargo run -p axcenity-avatar --release --example pose_correctives -- solve <library> <item ID> <work folder> <out .correctives.json>
//! cargo run -p axcenity-avatar --release --example pose_correctives -- add <library> <item ID> <.correctives.json>
//! ```
//!
//! - `dump` writes the body, the garment and each run's frames (the skin
//!   posed, the garment as skinning poses it) into the work folder.
//! - `solve` reads the simulation's results (`<run>.sim.f32`, the cloth at
//!   each of the run's reads in order) and writes the shapes.
//! - `add` writes the shapes into the item's model and its manifest's
//!   `correctives` (the item's model must be as exported, without any).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use axcenity_avatar::fit::{
    FitError, PoseShapes, add_to_item, cloth_rules, cloth_runs, item_meshes, solve,
    write_bake_input,
};
use glam::Vec3;

fn finish(rig: &axcenity_assets::Rig, pose: &mut Vec<axcenity_assets::BoneTransform>) {
    axcenity_anim::twist::drive(rig, pose, &mut []);
}

fn run(args: &[String]) -> Result<(), FitError> {
    let usage = || {
        FitError::Model("usage: pose_correctives dump|solve|add <library> <item ID> ...".to_owned())
    };
    let (command, library, id) = match args {
        [command, library, id, ..] => (command.as_str(), PathBuf::from(library), id.as_str()),
        _ => return Err(usage()),
    };
    match (command, &args[3..]) {
        ("dump", [work]) => {
            let (body, garment) = item_meshes(&library, id)?;
            let rules = cloth_rules(&body.rig);
            let runs = cloth_runs(&body.rig, &rules, Some(finish));
            write_bake_input(Path::new(work), &body, &garment, &rules, &runs)?;
            println!(
                "{} runs, {} frames, {} rules; body {} corners, garment {}",
                runs.len(),
                runs.iter().map(|r| r.frames.len()).sum::<usize>(),
                rules.len(),
                body.positions.len(),
                garment.positions.len()
            );
            Ok(())
        }
        ("solve", [work, out]) => {
            let (body, garment) = item_meshes(&library, id)?;
            let rules = cloth_rules(&body.rig);
            let runs = cloth_runs(&body.rig, &rules, Some(finish));
            let count = garment.positions.len();
            let work = PathBuf::from(work);
            let simulated = |r: usize, k: usize| -> Result<Vec<Vec3>, FitError> {
                let path = work.join(format!("{}.sim.f32", runs[r].name));
                let bytes = std::fs::read(&path)
                    .map_err(|e| FitError::Model(format!("{}: {e}", path.display())))?;
                let floats: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                    .collect();
                let start = k * count * 3;
                let read = floats
                    .get(start..start + count * 3)
                    .ok_or_else(|| FitError::Model(format!("{}: no read {k}", path.display())))?;
                Ok(read
                    .chunks_exact(3)
                    .map(|v| Vec3::new(v[0], v[1], v[2]))
                    .collect())
            };
            let shapes = solve(&garment, &rules, &runs, &simulated)?;
            std::fs::write(out, shapes.to_json()).map_err(|e| FitError::Write {
                path: out.clone(),
                message: e.to_string(),
            })?;
            println!(
                "{}",
                serde_json::to_string_pretty(&shapes.report).unwrap_or_default()
            );
            Ok(())
        }
        ("add", [shapes]) => {
            let text = std::fs::read_to_string(shapes)
                .map_err(|e| FitError::Model(format!("{shapes}: {e}")))?;
            let shapes = PoseShapes::from_json(&text)?;
            let added = add_to_item(&library, id, &shapes)?;
            println!("added {added} pose correctives to `{id}`");
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("pose_correctives: {err}");
            ExitCode::FAILURE
        }
    }
}
