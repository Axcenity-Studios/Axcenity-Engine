//! Imports a garment into a character library (CO9's import flow,
//! `axcenity_avatar::fit::import_garment`) from the command line:
//!
//! ```text
//! cargo run -p axcenity-avatar --release --example import_garment -- \
//!     <garment.obj|.glb> <library folder> <base ID> <item ID> <label> <folder in the library> \
//!     [--under <item ID>]... [--slot <slot>] [--layer <0-3>] [--margin <metres>] \
//!     [--also-fills <slot>]... [--tag <tag>]... \
//!     [--limit <parameter>=<low>..<high>=<note>]... \
//!     [--author <name>] [--licence <SPDX ID>] \
//!     [--intake] [--kind top|trousers|skirt|one_piece] \
//!     [--ease snug|regular|loose|oversized] [--scale <metres a unit>] \
//!     [--conform none|structured|partial|moderate|body|strong] \
//!     [--cut-for <shape>=<weight>]... [--keep-inner-wall] \
//!     [--textures <pixels a side>] [--triangles <budget>] \
//!     [--worn-with <item ID>]... [--tucked] [--worn-most <metres>]
//! ```
//!
//! `--worn-with` names a bottom of the library the garment will be worn
//! with: its cloth below the waist is moved outside that bottom's
//! (`axcenity_avatar::fit::Hang`), by `--worn-most` at the most (0.03),
//! or with `--tucked` drawn in under it.
//!
//! `--also-fills` names another slot the item takes (a jumpsuit in the
//! top slot: `bottom`), `--tag` a tag for the browser, and `--limit` a
//! known limit on a base parameter (a garment cut for the feminine frame:
//! `frame=0.5..1=Cut for the feminine frame`).
//!
//! `--intake` (or any of the flags after it) is for a garment that was not
//! made on the body: a generated or downloaded one in its own size, place
//! and pose (`axcenity_avatar::fit::prepare`).
//!
//! It aligns the garment to the base body, fits it (binding, weights,
//! the body's identity and corrective shapes, push-out from the body and
//! the items named with `--under`), finds its regions and slot, draws a
//! thumbnail, writes the model, binding, thumbnail and item manifest into
//! the folder and lists the item in the library's catalogue. The item is a
//! draft until reviewed. Prints the fitting report.

use std::path::PathBuf;
use std::process::ExitCode;

use axcenity_avatar::fit::{
    Conform, Ease, GarmentKind, ImportRequest, IntakeOptions, Worn, WornWithRequest, import_garment,
};
use axcenity_avatar::{Limitation, Source};

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
    let worn_with = take("--worn-with");
    let worn_most = take("--worn-most")
        .pop()
        .and_then(|m| m.parse().ok())
        .unwrap_or(0.03);
    let also_fills = take("--also-fills");
    let tags = take("--tag");
    let limitations: Vec<Limitation> = take("--limit")
        .iter()
        .filter_map(|limit| {
            let (parameter, rest) = limit.split_once('=')?;
            let (range, note) = rest.split_once('=')?;
            let (low, high) = range.split_once("..")?;
            Some(Limitation {
                note: note.to_owned(),
                parameter: Some(parameter.to_owned()),
                range: Some([low.parse().ok()?, high.parse().ok()?]),
            })
        })
        .collect();
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
    let texture_size = take("--textures").pop().and_then(|s| s.parse().ok());
    let triangles: Option<usize> = take("--triangles").pop().and_then(|s| s.parse().ok());
    let kind = take("--kind").pop();
    let ease = take("--ease").pop();
    let conform = take("--conform").pop();
    let scale = take("--scale").pop().and_then(|s| s.parse().ok());
    let cut_for: Vec<(String, f32)> = take("--cut-for")
        .iter()
        .filter_map(|pair| {
            let (shape, weight) = pair.split_once('=')?;
            Some((shape.to_owned(), weight.parse().ok()?))
        })
        .collect();
    let mut flag = |name: &str| -> bool {
        let found = args.iter().position(|a| a == name);
        if let Some(at) = found {
            args.remove(at);
        }
        found.is_some()
    };
    let keep_inner_wall = flag("--keep-inner-wall");
    let tucked = flag("--tucked");
    let intake = flag("--intake")
        || kind.is_some()
        || ease.is_some()
        || conform.is_some()
        || scale.is_some()
        || !cut_for.is_empty()
        || triangles.is_some()
        || keep_inner_wall;
    let intake = intake.then(|| IntakeOptions {
        kind: kind.as_deref().and_then(|kind| match kind {
            "top" => Some(GarmentKind::Top),
            "trousers" => Some(GarmentKind::Trousers),
            "skirt" => Some(GarmentKind::Skirt),
            "one_piece" => Some(GarmentKind::OnePiece),
            _ => None,
        }),
        ease: match ease.as_deref() {
            Some("snug") => Ease::Snug,
            Some("loose") => Ease::Loose,
            Some("oversized") => Ease::Oversized,
            _ => Ease::Regular,
        },
        conform: conform.as_deref().and_then(|conform| match conform {
            "none" => Some(Conform::None),
            "structured" => Some(Conform::Structured),
            "partial" => Some(Conform::Partial),
            "moderate" => Some(Conform::Moderate),
            "body" => Some(Conform::Body),
            "strong" => Some(Conform::Strong),
            _ => None,
        }),
        scale,
        cut_for,
        keep_inner_wall,
        triangles,
        ..IntakeOptions::default()
    });
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
            url: None,
        },
        slot,
        layer,
        margin,
        under,
        about: None,
        also_fills,
        tags,
        limitations,
        intake,
        texture_size,
        worn_with: (!worn_with.is_empty()).then_some(WornWithRequest {
            items: worn_with,
            worn: if tucked { Worn::Tucked } else { Worn::Over },
            most: worn_most,
        }),
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
            if let Some(intake) = &imported.intake {
                println!("intake {intake:#?}");
            }
            println!("coverage {:?}", imported.coverage);
            println!("cover map {:?}", imported.cover);
            println!("{:#?}", imported.report);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("couldn't import: {err}");
            ExitCode::FAILURE
        }
    }
}
