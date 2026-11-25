//! Imports a canonical A-pose single-shell garment into a character library
//! (ADR-0046, `axcenity_avatar::fit::import_garment`) from the command line:
//!
//! ```text
//! cargo run -p axcenity-avatar --release --example import_garment -- \
//!     <garment.obj|.glb> <library folder> <base ID> <item ID> <label> <folder in the library> \
//!     [--slot <slot>] [--layer <0-3>] [--margin <metres>] \
//!     [--frame masculine|feminine|both] [--proxy body|skirt] \
//!     [--no-shrink-under] [--strict-validate] \
//!     [--also-fills <slot>]... [--tag <tag>]... \
//!     [--limit <parameter>=<low>..<high>=<note>]... \
//!     [--author <name>] [--licence <SPDX ID>] \
//!     [--textures <pixels a side>]
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use axcenity_avatar::fit::{HomeFrame, ImportRequest, ProxyKind, import_garment};
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
    let home_frame = match take("--frame").pop().as_deref() {
        Some("feminine") => HomeFrame::Feminine,
        Some("both") => HomeFrame::Both,
        _ => HomeFrame::Masculine,
    };
    let proxy_kind = match take("--proxy").pop().as_deref() {
        Some("skirt") => ProxyKind::Skirt,
        _ => ProxyKind::Body,
    };
    let mut flag = |name: &str| -> bool {
        let found = args.iter().position(|a| a == name);
        if let Some(at) = found {
            args.remove(at);
        }
        found.is_some()
    };
    let generate_shrink_under = !flag("--no-shrink-under");
    let strict_validate = flag("--strict-validate");
    let sleeveless = flag("--sleeveless");
    let skip_align = flag("--canonical");
    let [source, library, base, id, label, folder] = match <[String; 6]>::try_from(args) {
        Ok(positional) => positional,
        Err(_) => {
            eprintln!(
                "usage: import_garment <garment> <library> <base ID> <item ID> <label> <folder> [--slot <slot>] [--layer <0-3>] [--margin <m>] [--frame masculine|feminine|both] [--proxy body|skirt] [--no-shrink-under] [--strict-validate] [--author <name>] [--licence <SPDX>] [--textures <px>]"
            );
            return ExitCode::FAILURE;
        }
    };
    let request = ImportRequest {
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
        home_frame,
        proxy_kind,
        sleeveless,
        skip_align,
        generate_shrink_under,
        strict_validate,
        texture_size,
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
                "  slot: {}, layer: {:?}, confident: {}, inpainted: {}, max_morph: {:.2} mm, shrink_under: {:.2} mm",
                imported.manifest.slot,
                imported.manifest.layer,
                imported.report.confident_vertices,
                imported.report.inpainted_vertices,
                imported.report.max_morph_displacement * 1000.0,
                imported.report.shrink_under_reach * 1000.0,
            );
            if !imported.report.validation.warnings.is_empty() {
                for w in &imported.report.validation.warnings {
                    eprintln!("  validation warning: {w}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("import failed: {err}");
            ExitCode::FAILURE
        }
    }
}
