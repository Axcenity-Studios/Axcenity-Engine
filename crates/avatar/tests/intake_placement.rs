//! The licensed-asset intake places an item on the realistic body itself
//! (`assets/characters/realistic/import/conform.py`: a hat lowered onto
//! the head, tilted and scaled by the spec), so its import keeps that
//! place: `ImportOptions::keep_placement` skips the alignment
//! (`fit::align`, made for garments authored on a body with an even ease,
//! which would lift a brimmed hat off the head), and everything else of
//! the import (binding, weights, shapes, push-out, regions) is the same.

use std::path::{Path, PathBuf};

use axcenity_avatar::fit::{ImportOptions, ImportRequest, import_garment_with};
use axcenity_avatar::{Catalogue, CatalogueBase, CatalogueItem, Library, Source};
use glam::Vec3;

fn shipped() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

/// A scratch library with the realistic base and its underwear.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch_library(name: &str) -> Scratch {
    let root = std::env::temp_dir().join(format!("axcenity-intake-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (library, _, problems) = Library::read(&shipped());
    assert!(problems.is_empty(), "{problems:?}");
    let base = library.base("axcenity/realistic").unwrap();
    let base_path = library.manifest_path("axcenity/realistic").unwrap();
    let files = [
        base_path.to_owned(),
        library.path_of(&base.id, &base.model.file).unwrap(),
        library
            .path_of(&base.id, base.model.regions_map.as_deref().unwrap())
            .unwrap(),
    ];
    for file in &files {
        let target = root.join(file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(shipped().join(file), &target).unwrap();
    }
    let catalogue = Catalogue {
        format_version: 1,
        bases: vec![CatalogueBase::of(base_path, base)],
        items: Vec::<CatalogueItem>::new(),
        presets: Vec::new(),
    };
    std::fs::write(root.join("catalogue.json"), catalogue.to_json()).unwrap();
    Scratch(root)
}

/// The fixture tee moved up by `lift` metres, as an OBJ in `folder`.
fn lifted_tee(folder: &Path, lift: f32) -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test-tee/TestTee.obj");
    let text = std::fs::read_to_string(fixture).unwrap();
    let moved: Vec<String> = text
        .lines()
        .map(|line| match line.strip_prefix("v ") {
            Some(rest) => {
                let v: Vec<f32> = rest
                    .split_whitespace()
                    .map(|n| n.parse().unwrap())
                    .collect();
                format!("v {} {} {}", v[0], v[1] + lift, v[2])
            }
            None => line.to_owned(),
        })
        .collect();
    let path = folder.join("LiftedTee.obj");
    std::fs::write(&path, moved.join("\n")).unwrap();
    path
}

fn request(root: &Path, source: PathBuf) -> ImportRequest {
    ImportRequest {
        source,
        library: root.to_owned(),
        base: "axcenity/realistic".to_owned(),
        id: "intake/lifted-tee".to_owned(),
        label: "Lifted tee".to_owned(),
        folder: "intake/lifted-tee".to_owned(),
        source_info: Source {
            author: "Axcenity Engine contributors".to_owned(),
            licence: "CC0-1.0".to_owned(),
            ..Source::default()
        },
        slot: Some("top".to_owned()),
        layer: None,
        margin: 0.002,
        under: Vec::new(),
        about: None,
        finish_pose: None,
    }
}

#[test]
fn a_placed_item_keeps_its_place_and_an_unplaced_one_is_aligned() {
    let library = scratch_library("placement");
    let tee = lifted_tee(&library.0, 0.04);

    // As the import has always done: put back on the body. (Its masks of
    // what it hides aren't wanted here: they take a minute, wave 5.)
    let skip = ImportOptions {
        skip_hiding: true,
        ..ImportOptions::default()
    };
    let aligned = import_garment_with(&request(&library.0, tee.clone()), skip).unwrap();
    assert!(
        aligned.alignment.offset.y < -0.02,
        "a tee made 4 cm too high comes down: {:?}",
        aligned.alignment
    );

    // Placed by the intake: left where it is, at its own scale.
    let placed = import_garment_with(
        &request(&library.0, tee),
        ImportOptions {
            keep_placement: true,
            ..skip
        },
    )
    .unwrap();
    assert_eq!(placed.alignment.scale, 1.0);
    assert_eq!(placed.alignment.units, 1.0);
    assert_eq!(placed.alignment.offset, Vec3::ZERO);
    // The rest of the import is the same: a fitted draft with its files.
    let folder = library.0.join("intake/lifted-tee");
    for file in [
        "LiftedTee.glb",
        "LiftedTee.binding.json",
        "LiftedTee.item.json",
    ] {
        assert!(folder.join(file).is_file(), "{file}");
    }
    assert_eq!(placed.manifest.slot, "top");
}
