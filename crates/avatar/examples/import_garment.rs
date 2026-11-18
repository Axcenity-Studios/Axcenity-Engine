//! Imports a garment into a character library (CO9's import flow,
//! `axcenity_avatar::fit::import_garment`) from the command line:
//!
//! ```text
//! cargo run -p axcenity-avatar --release --example import_garment -- \
//!     <garment.obj|.glb> <library folder> <base ID> <item ID> <label> <folder in the library> \
//!     [--under <item ID>]... [--slot <slot>] [--layer <0-3>] [--margin <metres>] \
//!     [--author <name>] [--licence <SPDX ID>]
//! ```
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
use axcenity_avatar::fit::{ImportRequest, import_garment};

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
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
                "usage: import_garment <garment> <library> <base ID> <item ID> <label> <folder> [--under <item ID>]... [--slot <slot>] [--layer <n>] [--margin <m>] [--author <name>] [--licence <SPDX>]"
            );
            return ExitCode::FAILURE;
        }
    };
    let request = ImportRequest {
        source: PathBuf::from(source),
        library: PathBuf::from(library),
        base,
        id,
        label,
        folder,
        source_info: Source {
            author,
            licence,
            url: None,
        },
        slot,
        layer,
        margin,
        under,
        about: None,
    };
    let started = std::time::Instant::now();
    match import_garment(&request) {
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
