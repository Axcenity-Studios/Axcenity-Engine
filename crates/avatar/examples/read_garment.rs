//! Validates a canonical garment mesh against ADR-0046 source rules
//! (manifoldness, degenerate triangles, open boundary loops, and opening
//! chord-sag budget `L < sqrt(8 * r * m)`):
//!
//! ```text
//! cargo run -p axcenity-avatar --release --example read_garment -- <garment.glb|.obj> [margin_m]
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use axcenity_avatar::fit::{GarmentMesh, validate_garment};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(file) = args.next() else {
        eprintln!("usage: read_garment <garment> [margin_m]");
        return ExitCode::FAILURE;
    };
    let margin = args.next().and_then(|n| n.parse().ok()).unwrap_or(0.002);
    match axcenity_assets::load_model_file(&PathBuf::from(&file)) {
        Ok(model) => {
            let garment = GarmentMesh::from_model(&model);
            let report = validate_garment(&garment, margin);
            println!(
                "{file}: verts={} tris={} non_manifold={} degen={} loops={} bnd_edges={} max_open_mm={:.2} limit_mm={:.2} violations={} clean={}",
                garment.len(),
                garment.triangles().len(),
                report.non_manifold_edges,
                report.degenerate_triangles,
                report.boundary_loops,
                report.boundary_edges,
                report.max_opening_edge_m * 1000.0,
                report.chord_sag_limit_m * 1000.0,
                report.chord_sag_violations,
                report.is_clean(),
            );
            for w in &report.warnings {
                println!("  warning: {w}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("`{file}`: {err}");
            ExitCode::FAILURE
        }
    }
}
