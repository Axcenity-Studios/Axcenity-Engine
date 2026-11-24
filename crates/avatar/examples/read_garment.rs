//! Prints how the intake reads a garment file (its cross-sections, trunk
//! and limbs), for looking into an import that went wrong:
//!
//! ```text
//! cargo run -p axcenity-avatar --release --example read_garment -- <garment.glb> [every nth plane]
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use axcenity_avatar::fit::{GarmentMesh, describe_reading};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(file) = args.next() else {
        eprintln!("usage: read_garment <garment> [every nth plane]");
        return ExitCode::FAILURE;
    };
    let every = args.next().and_then(|n| n.parse().ok()).unwrap_or(12);
    match axcenity_assets::load_model_file(&PathBuf::from(&file)) {
        Ok(model) => {
            print!(
                "{}",
                describe_reading(&GarmentMesh::from_model(&model), every)
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("`{file}`: {err}");
            ExitCode::FAILURE
        }
    }
}
