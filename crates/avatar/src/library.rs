//! A library read into memory: base models, items and presets by ID, each
//! with the path of its file in the library (paths inside a manifest are
//! relative to it). The content library on disk is a folder with a
//! `catalogue.json` at its root (`docs/development/character-studio/
//! architecture.md`, Content library).

use std::collections::BTreeMap;
use std::path::Path;

use crate::{BaseManifest, Catalogue, Description, ItemManifest, Problem};

/// The character library's folder in a project, from the project's folder:
/// its `catalogue.json` is at the top of it.
pub const LIBRARY_FOLDER: &str = "assets/characters";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Library {
    pub bases: BTreeMap<String, BaseManifest>,
    pub items: BTreeMap<String, ItemManifest>,
    /// Presets (`*.preset.json`) by their `preset` block's ID.
    pub presets: BTreeMap<String, Description>,
    /// Each manifest's path in the library, by ID.
    manifest_paths: BTreeMap<String, String>,
    /// Each preset file's path in the library, by ID.
    preset_paths: BTreeMap<String, String>,
}

impl Library {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a base model whose manifest is at `path` in the library.
    pub fn add_base(&mut self, path: impl Into<String>, base: BaseManifest) {
        self.manifest_paths.insert(base.id.clone(), path.into());
        self.bases.insert(base.id.clone(), base);
    }

    /// Adds an item whose manifest is at `path` in the library.
    pub fn add_item(&mut self, path: impl Into<String>, item: ItemManifest) {
        self.manifest_paths.insert(item.id.clone(), path.into());
        self.items.insert(item.id.clone(), item);
    }

    /// Adds a preset whose file is at `path` in the library, by its
    /// `preset` block's ID; a description without one isn't a preset.
    pub fn add_preset(
        &mut self,
        path: impl Into<String>,
        preset: Description,
    ) -> Result<(), Problem> {
        let path = path.into();
        let Some(block) = preset.preset() else {
            return Err(Problem::new(path, "has no `preset` block"));
        };
        self.preset_paths.insert(block.id.clone(), path);
        self.presets.insert(block.id, preset);
        Ok(())
    }

    pub fn base(&self, id: &str) -> Option<&BaseManifest> {
        self.bases.get(id)
    }

    pub fn item(&self, id: &str) -> Option<&ItemManifest> {
        self.items.get(id)
    }

    pub fn preset(&self, id: &str) -> Option<&Description> {
        self.presets.get(id)
    }

    /// Where the manifest of the base model or item `id` is in the library.
    pub fn manifest_path(&self, id: &str) -> Option<&str> {
        self.manifest_paths.get(id).map(String::as_str)
    }

    /// Where the preset `id`'s file is in the library.
    pub fn preset_path(&self, id: &str) -> Option<&str> {
        self.preset_paths.get(id).map(String::as_str)
    }

    /// The library path of `relative`, written in the manifest of `id`;
    /// `None` when there is no such manifest or the path leaves the
    /// library.
    pub fn path_of(&self, id: &str, relative: &str) -> Option<String> {
        crate::resolve_path(self.manifest_path(id)?, relative)
    }

    /// Reads the library in the folder `root`: its `catalogue.json` and
    /// every manifest and preset it lists. Files that can't be read, IDs
    /// that don't match their entries and duplicates are problems; what
    /// reads is kept.
    pub fn read(root: &Path) -> (Library, Option<Catalogue>, Vec<Problem>) {
        let mut library = Library::new();
        let mut problems = Vec::new();
        let catalogue = match read_text(root, "catalogue.json")
            .and_then(|text| Catalogue::from_json(&text).map_err(|err| err.to_string()))
        {
            Ok(catalogue) => catalogue,
            Err(err) => {
                problems.push(Problem::new("catalogue.json", err));
                return (library, None, problems);
            }
        };
        for entry in &catalogue.bases {
            let read = read_manifest(root, &entry.manifest, BaseManifest::from_json);
            match read {
                Err(problem) => problems.push(problem),
                Ok(base) if base.id != entry.id => problems.push(Problem::new(
                    &entry.manifest,
                    format!("is `{}`; the catalogue says `{}`", base.id, entry.id),
                )),
                Ok(base) if library.manifest_path(&base.id).is_some() => problems.push(
                    Problem::new(&entry.manifest, format!("`{}` is listed twice", base.id)),
                ),
                Ok(base) => library.add_base(entry.manifest.clone(), base),
            }
        }
        for entry in &catalogue.items {
            let read = read_manifest(root, &entry.manifest, ItemManifest::from_json);
            match read {
                Err(problem) => problems.push(problem),
                Ok(item) if item.id != entry.id => problems.push(Problem::new(
                    &entry.manifest,
                    format!("is `{}`; the catalogue says `{}`", item.id, entry.id),
                )),
                Ok(item) if library.manifest_path(&item.id).is_some() => problems.push(
                    Problem::new(&entry.manifest, format!("`{}` is listed twice", item.id)),
                ),
                Ok(item) => library.add_item(entry.manifest.clone(), item),
            }
        }
        for entry in &catalogue.presets {
            let read = read_manifest(root, &entry.file, Description::from_json);
            let preset = match read {
                Err(problem) => {
                    problems.push(problem);
                    continue;
                }
                Ok(preset) => preset,
            };
            match preset.preset().map(|block| block.id) {
                Some(id) if id == entry.id && library.preset(&id).is_none() => {
                    let _ = library.add_preset(entry.file.clone(), preset);
                }
                Some(id) if id == entry.id => {
                    problems.push(Problem::new(&entry.file, format!("`{id}` is listed twice")))
                }
                found => problems.push(Problem::new(
                    &entry.file,
                    format!(
                        "is `{}`; the catalogue says `{}`",
                        found.unwrap_or_else(|| "no preset".to_owned()),
                        entry.id
                    ),
                )),
            }
        }
        (library, Some(catalogue), problems)
    }

    /// Checks every manifest on its own, every item against each base
    /// model it fits (which must be in the library) and the items it
    /// requires or excludes, every preset ([`crate::check_preset`]), and
    /// that every path a manifest names stays inside the library.
    pub fn validate(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        for (id, base) in &self.bases {
            problems.extend(prefixed(id, base.validate()));
            self.check_inside(&mut problems, id, "model.file", &base.model.file);
        }
        for (id, item) in &self.items {
            problems.extend(prefixed(id, item.validate()));
            for base_id in &item.bases {
                match self.base(base_id) {
                    None => problems.push(Problem::new(
                        format!("{id}: bases"),
                        format!("the library has no base model `{base_id}`"),
                    )),
                    Some(base) => problems.extend(prefixed(
                        &format!("{id} on {base_id}"),
                        item.check_against(base),
                    )),
                }
            }
            for (list, ids) in [("requires", &item.requires), ("excludes", &item.excludes)] {
                for other in ids {
                    if self.item(other).is_none() {
                        problems.push(Problem::new(
                            format!("{id}: {list}"),
                            format!("the library has no item `{other}`"),
                        ));
                    }
                }
            }
            for model in item.models() {
                self.check_inside(&mut problems, id, "model", &model.file);
            }
            if let Some(strands) = item.groom.as_ref().and_then(|g| g.strands.as_deref()) {
                self.check_inside(&mut problems, id, "groom.strands", strands);
            }
            if let Some(thumbnail) = &item.thumbnail {
                self.check_inside(&mut problems, id, "thumbnail", thumbnail);
            }
            for (k, hide) in item.hide_maps.iter().enumerate() {
                self.check_inside(&mut problems, id, &format!("hide_maps[{k}].map"), &hide.map);
            }
            for variant in &item.variants {
                if let Some(thumbnail) = &variant.thumbnail {
                    let at = format!("variants.{}.thumbnail", variant.id);
                    self.check_inside(&mut problems, id, &at, thumbnail);
                }
            }
        }
        for (id, preset) in &self.presets {
            problems.extend(prefixed(id, crate::check_preset(preset, self)));
            let thumbnail = preset.preset().and_then(|block| block.thumbnail);
            if let Some(thumbnail) = thumbnail
                && self
                    .preset_path(id)
                    .and_then(|path| crate::resolve_path(path, &thumbnail))
                    .is_none()
            {
                problems.push(Problem::new(
                    format!("{id}: preset.thumbnail"),
                    format!("`{thumbnail}` leads out of the library"),
                ));
            }
        }
        problems
    }

    /// A path written in the manifest of `id` must stay in the library.
    fn check_inside(&self, problems: &mut Vec<Problem>, id: &str, at: &str, path: &str) {
        if self.path_of(id, path).is_none() {
            problems.push(Problem::new(
                format!("{id}: {at}"),
                format!("`{path}` leads out of the library"),
            ));
        }
    }
}

/// `problems` with the file's ID in front of where each is.
fn prefixed(id: &str, problems: Vec<Problem>) -> Vec<Problem> {
    problems
        .into_iter()
        .map(|problem| Problem::new(format!("{id}: {}", problem.at), problem.message))
        .collect()
}

fn read_text(root: &Path, path: &str) -> Result<String, String> {
    if !crate::is_valid_path(path) || path.split('/').any(|part| part == "..") {
        return Err(format!("`{path}` isn't a path inside the library"));
    }
    std::fs::read_to_string(root.join(path)).map_err(|err| err.to_string())
}

fn read_manifest<T>(
    root: &Path,
    path: &str,
    parse: impl Fn(&str) -> Result<T, crate::FormatError>,
) -> Result<T, Problem> {
    let text = read_text(root, path).map_err(|err| Problem::new(path, err))?;
    parse(&text).map_err(|err| Problem::new(path, err.to_string()))
}
