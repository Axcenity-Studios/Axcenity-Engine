//! Scatter palettes: `.axscatter` files in the project's `presets/` folder
//! (beside world presets), each a list of what the editor's Scatter pane
//! places, with each item's weight, size, turn, collide, lean and sink, and
//! the palette's density and spacing (`docs/editor/scatter.md`).
//!
//! An item names its plant, prop or model as a ScatterRule's `Models` entry
//! does (`prop:Crate`, `plant:Grass` or a model's path). An entry this
//! engine doesn't know (a built-in from a newer version) is left out when
//! read, so the rest of the palette still loads. Every setting but the
//! item has a default, so a hand-written file can be short.

use std::io;
use std::path::{Path, PathBuf};

use axcenity_core::FoliageSource;
use axcenity_terrain::brush::ScatterItem;
use serde::{Deserialize, Serialize};

use crate::world_presets::safe_file_stem;

/// A scatter palette file's extension.
pub const SCATTER_PALETTE_EXTENSION: &str = "axscatter";
/// The newest palette format this reads, and the one it writes.
pub const SCATTER_PALETTE_FORMAT: u32 = 1;
/// A new palette's density: items per 100 square studs.
pub const DEFAULT_SCATTER_DENSITY: f32 = 8.0;
/// A new palette's spacing: studs between items.
pub const DEFAULT_SCATTER_SPACING: f32 = 1.0;

/// A scatter palette: what a `.axscatter` file holds.
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterPalette {
    pub name: String,
    /// Items per 100 square studs where the brush is at full strength.
    pub density: f32,
    /// Studs kept between a new item and any painted foliage (0: none).
    pub spacing: f32,
    pub items: Vec<ScatterItem>,
}

impl ScatterPalette {
    /// An empty palette called `name`, at the usual density and spacing.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.trim().to_owned(),
            density: DEFAULT_SCATTER_DENSITY,
            spacing: DEFAULT_SCATTER_SPACING,
            items: Vec::new(),
        }
    }

    pub fn to_json(&self) -> String {
        let file = PaletteFile {
            format: SCATTER_PALETTE_FORMAT,
            name: self.name.clone(),
            density: self.density,
            spacing: self.spacing,
            items: self.items.iter().map(ItemRecord::from).collect(),
        };
        serde_json::to_string_pretty(&file).expect("a palette is always JSON") + "\n"
    }

    pub fn from_json(text: &str) -> Result<ScatterPalette, ScatterPaletteError> {
        let file: PaletteFile = serde_json::from_str(text)?;
        if file.format > SCATTER_PALETTE_FORMAT {
            return Err(ScatterPaletteError::TooNew { found: file.format });
        }
        let finite = |value: f32, default: f32| if value.is_finite() { value } else { default };
        Ok(ScatterPalette {
            name: file.name.trim().to_owned(),
            density: finite(file.density, DEFAULT_SCATTER_DENSITY).max(0.0),
            spacing: finite(file.spacing, DEFAULT_SCATTER_SPACING).max(0.0),
            items: file
                .items
                .into_iter()
                .filter_map(ItemRecord::into_item)
                .collect(),
        })
    }
}

#[derive(Serialize, Deserialize)]
struct PaletteFile {
    format: u32,
    name: String,
    #[serde(default = "default_density")]
    density: f32,
    #[serde(default = "default_spacing")]
    spacing: f32,
    #[serde(default)]
    items: Vec<ItemRecord>,
}

fn default_density() -> f32 {
    DEFAULT_SCATTER_DENSITY
}

fn default_spacing() -> f32 {
    DEFAULT_SCATTER_SPACING
}

/// One item as written: its settings by name, with `item` the source.
#[derive(Serialize, Deserialize)]
struct ItemRecord {
    item: String,
    #[serde(default = "yes")]
    enabled: bool,
    #[serde(default = "one")]
    weight: f32,
    #[serde(default = "default_min_scale")]
    min_scale: f32,
    #[serde(default = "default_max_scale")]
    max_scale: f32,
    #[serde(default)]
    collide: bool,
    /// Degrees of random turn, 0 to 360.
    #[serde(default = "full_turn")]
    rotation: f32,
    #[serde(default)]
    align_to_slope: f32,
    #[serde(default)]
    sink: f32,
}

fn yes() -> bool {
    true
}

fn one() -> f32 {
    1.0
}

fn default_min_scale() -> f32 {
    ScatterItem::new(FoliageSource::Model(String::new())).min_scale
}

fn default_max_scale() -> f32 {
    ScatterItem::new(FoliageSource::Model(String::new())).max_scale
}

fn full_turn() -> f32 {
    360.0
}

impl From<&ScatterItem> for ItemRecord {
    fn from(item: &ScatterItem) -> Self {
        Self {
            item: item.source.entry(),
            enabled: item.enabled,
            weight: item.weight,
            min_scale: item.min_scale,
            max_scale: item.max_scale,
            collide: item.collide,
            rotation: item.rotation,
            align_to_slope: item.align_to_slope,
            sink: item.sink,
        }
    }
}

impl ItemRecord {
    /// The item, or `None` for an empty or unknown entry.
    fn into_item(self) -> Option<ScatterItem> {
        let source = FoliageSource::from_entry(&self.item)?;
        let finite = |value: f32, default: f32| if value.is_finite() { value } else { default };
        Some(ScatterItem {
            source,
            enabled: self.enabled,
            weight: finite(self.weight, 1.0).max(0.0),
            min_scale: finite(self.min_scale, 1.0),
            max_scale: finite(self.max_scale, 1.0),
            collide: self.collide,
            rotation: finite(self.rotation, 360.0).clamp(0.0, 360.0),
            align_to_slope: finite(self.align_to_slope, 0.0).clamp(0.0, 1.0),
            sink: finite(self.sink, 0.0),
        })
    }
}

/// Why a scatter palette couldn't be read or saved.
#[derive(Debug, thiserror::Error)]
pub enum ScatterPaletteError {
    #[error("not a scatter palette: {0}")]
    Json(#[from] serde_json::Error),
    #[error(
        "scatter palette format {found} is newer than this engine understands; update Axcenity to use it"
    )]
    TooNew { found: u32 },
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// A palette file found on disk, and what it holds.
pub type FoundPalette = (PathBuf, ScatterPalette);
/// A palette file that couldn't be read, and why.
pub type PaletteProblem = (PathBuf, ScatterPaletteError);

/// A file name for a palette called `name`, such as `Forest Floor.axscatter`
/// (characters file systems don't allow become `-`).
pub fn scatter_palette_file_name(name: &str) -> String {
    format!(
        "{}.{SCATTER_PALETTE_EXTENSION}",
        safe_file_stem(name, "Palette")
    )
}

/// Saves `palette` in `folder` (made if missing), named after it, replacing
/// a palette file of the same name. Returns the file.
pub fn save_scatter_palette(
    folder: &Path,
    palette: &ScatterPalette,
) -> Result<PathBuf, ScatterPaletteError> {
    std::fs::create_dir_all(folder)?;
    let file = folder.join(scatter_palette_file_name(&palette.name));
    std::fs::write(&file, palette.to_json())?;
    Ok(file)
}

/// The scatter palettes in `folder`, sorted by name, and the files that
/// couldn't be read. A missing folder has none.
pub fn list_scatter_palettes(folder: &Path) -> (Vec<FoundPalette>, Vec<PaletteProblem>) {
    let mut found = Vec::new();
    let mut problems = Vec::new();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return (found, problems);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(SCATTER_PALETTE_EXTENSION) {
            continue;
        }
        match std::fs::read_to_string(&path)
            .map_err(ScatterPaletteError::from)
            .and_then(|text| ScatterPalette::from_json(&text))
        {
            Ok(palette) => found.push((path, palette)),
            Err(problem) => problems.push((path, problem)),
        }
    }
    found.sort_by_key(|(_, palette)| palette.name.to_lowercase());
    (found, problems)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axcenity_core::{FoliagePlant, FoliageProp};

    fn palette() -> ScatterPalette {
        let mut palette = ScatterPalette::new("Forest Floor");
        palette.density = 12.5;
        palette.spacing = 2.0;
        palette.items = vec![
            ScatterItem {
                weight: 3.0,
                min_scale: 0.5,
                max_scale: 2.0,
                collide: true,
                rotation: 45.0,
                align_to_slope: 0.75,
                sink: 0.3,
                ..ScatterItem::new(FoliageSource::Prop(FoliageProp::Rock))
            },
            ScatterItem {
                enabled: false,
                ..ScatterItem::new(FoliageSource::Plant(FoliagePlant::Grass))
            },
            ScatterItem::new(FoliageSource::Model("assets/models/Fern.glb".into())),
        ];
        palette
    }

    #[test]
    fn a_palette_round_trips_through_its_file() {
        let palette = palette();
        let text = palette.to_json();
        assert!(text.contains("\"prop:Rock\""), "{text}");
        assert!(text.contains("\"assets/models/Fern.glb\""), "{text}");
        assert_eq!(ScatterPalette::from_json(&text).unwrap(), palette);
    }

    #[test]
    fn a_short_file_takes_defaults_and_unknown_items_are_left_out() {
        let text = r#"{
  "format": 1,
  "name": "Rocks",
  "items": [
    { "item": "prop:Rock" },
    { "item": "prop:Unicorn" },
    { "item": "" },
    { "item": "plant:Bush", "weight": 2, "rotation": 720, "align_to_slope": 4 }
  ]
}"#;
        let read = ScatterPalette::from_json(text).unwrap();
        assert_eq!(read.density, DEFAULT_SCATTER_DENSITY);
        assert_eq!(read.spacing, DEFAULT_SCATTER_SPACING);
        assert_eq!(read.items.len(), 2);
        assert_eq!(
            read.items[0],
            ScatterItem::new(FoliageSource::Prop(FoliageProp::Rock))
        );
        assert_eq!(read.items[1].weight, 2.0);
        assert_eq!(read.items[1].rotation, 360.0);
        assert_eq!(read.items[1].align_to_slope, 1.0);
        let newer = text.replace("\"format\": 1", "\"format\": 9");
        assert!(matches!(
            ScatterPalette::from_json(&newer),
            Err(ScatterPaletteError::TooNew { found: 9 })
        ));
    }

    #[test]
    fn palettes_save_by_name_and_list_sorted_with_problems_apart() {
        let folder = axcenity_test_dirs::TestDir::adopt(
            std::env::temp_dir().join(format!("axcenity-scatter-palettes-{}", std::process::id())),
        );
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(list_scatter_palettes(&folder).0.len(), 0);
        let forest = palette();
        let file = save_scatter_palette(&folder, &forest).unwrap();
        assert_eq!(file, folder.join("Forest Floor.axscatter"));
        let mut beach = ScatterPalette::new("beach: rocks?");
        beach
            .items
            .push(ScatterItem::new(FoliageSource::Prop(FoliageProp::Rock)));
        let beach_file = save_scatter_palette(&folder, &beach).unwrap();
        assert_eq!(beach_file, folder.join("beach- rocks-.axscatter"));
        std::fs::write(folder.join("Broken.axscatter"), "not json").unwrap();
        std::fs::write(folder.join("Island.axworld"), "{}").unwrap();
        let (found, problems) = list_scatter_palettes(&folder);
        let names: Vec<&str> = found.iter().map(|(_, p)| p.name.as_str()).collect();
        assert_eq!(names, ["beach: rocks?", "Forest Floor"]);
        assert_eq!(found[1].1, forest);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].0.ends_with("Broken.axscatter"));
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
