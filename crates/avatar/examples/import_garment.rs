//! Imports a garment into a character library (CO9's import flow,
//! `axcenity_avatar::fit::import_garment`) from the command line:
//!
//! ```text
//! cargo run -p axcenity-avatar --release --example import_garment -- \
//!     <garment.obj|.glb> <library folder> <base ID> <item ID> <label> <folder in the library> \
//!     [--under <item ID>]... [--slot <slot>] [--layer <0-3>] [--margin <metres>] \
//!     [--author <name>] [--licence <SPDX ID>] [--placed] [--drape]
//! ```
//!
//! `--placed`: the file is already in metres and placed on the body (the
//! licensed-asset intake places it), so it isn't aligned again
//! (`ImportOptions::keep_placement`). `--drape`: it was draped onto the
//! body by the intake's cloth simulation and hangs with ease of its own
//! (`ImportOptions::drape`, `FitOptions::drape`).
//!
//! It aligns the garment to the base body, fits it (binding, weights,
//! the body's identity and corrective shapes, push-out from the body and
//! the items named with `--under`), finds its regions and slot, draws a
//! thumbnail, writes the model, binding, thumbnail and item manifest into
//! the folder and lists the item in the library's catalogue. The item is a
//! draft until reviewed. Prints the fitting report.

use std::path::PathBuf;
use std::process::ExitCode;

use axcenity_avatar::Source;
use axcenity_avatar::fit::{ImportOptions, ImportRequest, import_garment_with};

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let placed = match args.iter().position(|a| a == "--placed") {
        Some(at) => {
            args.remove(at);
            true
        }
        None => false,
    };
    let drape = match args.iter().position(|a| a == "--drape") {
        Some(at) => {
            args.remove(at);
            true
        }
        None => false,
    };
    let mut take = |flag: &str| -> Vec<String> {
        let mut found = Vec::new();
        while let Some(at) = args.iter().position(|a| a == flag) {
            if at + 1 < args.len() {
                found.push(args.remove(at + 1));
            }
            args.remove(at);
        }
        found
    };
    let under = take("--under");
    let slot = take("--slot").pop();
    let layer = take("--layer").pop().and_then(|l| l.parse().ok());
    let margin = take("--margin")
        .pop()
        .and_then(|m| m.parse().ok())
        .unwrap_or(0.002);
    let author = take("--author")
        .pop()
        .unwrap_or_else(|| "unknown".to_owned());
    let licence = take("--licence")
        .pop()
        .unwrap_or_else(|| "NOASSERTION".to_owned());
    let [source, library, base, id, label, folder] = match <[String; 6]>::try_from(args) {
        Ok(positional) => positional,
        Err(_) => {
            eprintln!(
                "usage: import_garment <garment> <library> <base ID> <item ID> <label> <folder> [--under <item ID>]... [--slot <slot>] [--layer <n>] [--margin <m>] [--author <name>] [--licence <SPDX>] [--placed] [--drape]"
            );
            return ExitCode::FAILURE;
        }
    };
    let request = ImportRequest {
        // The runtime's twist and helper bones, so the fit poses are the
        // runtime's.
        finish_pose: Some(|rig, pose| axcenity_anim::twist::drive(rig, pose, &mut [])),
        source: PathBuf::from(source),
        library: PathBuf::from(library),
        base,
        id,
        label,
        folder,
        source_info: Source {
            author,
            licence,
            ..Source::default()
        },
        slot,
        layer,
        margin,
        under,
        about: None,
    };
    let started = std::time::Instant::now();
    let options = ImportOptions {
        keep_placement: placed,
        drape,
        ..ImportOptions::default()
    };
    match import_garment_with(&request, options) {
        Ok(imported) => {
            println!(
                "imported `{}` as {} in {:.1} s",
                imported.manifest.id,
                imported.manifest_path,
                started.elapsed().as_secs_f32()
            );
            println!(
                "slot {}, layer {:?}, hides {:?}",
                imported.manifest.slot, imported.manifest.layer, imported.manifest.hides
            );
            println!(
                "hide_maps {:?}; hiding {:?}",
                imported.manifest.hide_maps, imported.hiding
            );
            println!("alignment {:?}", imported.alignment);
            println!("coverage {:?}", imported.coverage);
            println!("{:#?}", imported.report);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("couldn't import: {err}");
            ExitCode::FAILURE
        }
    }
}
