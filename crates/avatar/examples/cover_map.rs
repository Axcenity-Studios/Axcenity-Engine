//! Makes the cover maps of items in a character library again from their
//! fitted models (ADR-0045, `axcenity_avatar::fit::remake_cover_map`),
//! without fitting them again:
//!
//! ```text
//! cargo run -p axcenity-avatar --release --example cover_map -- \
//!     <library folder> [<item ID or part of one>]...
//! ```
//!
//! With no IDs, every garment fitted by binding. For items imported before
//! cover maps, or after the rule for them changed.

use std::path::PathBuf;
use std::process::ExitCode;

use axcenity_avatar::fit::remake_cover_map;
use axcenity_avatar::{Fit, ItemKind, Library};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("usage: cover_map <library> [<item ID or part of one>]...");
        return ExitCode::FAILURE;
    };
    let wanted: Vec<String> = args.collect();
    let (library, _, _) = Library::read(&root);
    let ids: Vec<String> = library
        .items
        .values()
        .filter(|item| item.kind == ItemKind::Garment && matches!(item.fit, Fit::Binding { .. }))
        .filter(|item| wanted.is_empty() || wanted.iter().any(|text| item.id.contains(text)))
        .map(|item| item.id.clone())
        .collect();
    let mut failed = false;
    for id in ids {
        // The runtime's twist and helper bones, so the poses the hidden
        // skin is tried in are the runtime's.
        let finish: axcenity_avatar::fit::FinishPose =
            |rig, pose| axcenity_anim::twist::drive(rig, pose, &mut []);
        match remake_cover_map(&root, &id, Some(finish)) {
            Ok(report) => println!(
                "{id}: covers {} body triangles, hides {} of skin and {} of cloth beneath",
                report.covered, report.skin, report.cloth
            ),
            Err(err) => {
                failed = true;
                eprintln!("{id}: {err}");
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
