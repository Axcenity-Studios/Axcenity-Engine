//! An imported model's files, shown as one folder in the Assets panel.
//!
//! Importing a textured or animated model writes many files: the model and
//! its textures in `assets/models/<name>/`, and its clips and state machine
//! in `assets/animations/<name>/`. Listed one by one they bury everything
//! else, so the panel shows them as one folder tile named after the model,
//! which opens to show them. A search still finds each file on its own.
//!
//! Inserting a character does the same in `assets/characters/<family>/`:
//! its body, hair, clothing, skins and their pictures. Each family is one
//! folder tile too ("Human character").
//!
//! This module decides what's in each folder and what the grid shows
//! ([`shown`]), and draws the folder tile and the row above an open folder.

use eframe::egui;

use super::origin::{Origin, Origins};
use super::{
    AssetBrowser, AssetEntry, AssetKind, AssetPayload, ItemMenu, LIST_PICTURE, LIST_ROW,
    global_badge,
};
use crate::theme::{self, icons};
use crate::widgets;

const MODELS: &str = "assets/models/";
const ANIMATIONS: &str = "assets/animations/";
const CHARACTERS: &str = "assets/characters/";

/// The files of one imported model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    /// The model's name, as its folder (or file) is called: `Walking`.
    pub name: String,
    /// Its model file, which dragging the folder places.
    pub model: AssetEntry,
    /// Every file in it: models, then state machines and clips, then
    /// textures, each in path order.
    pub members: Vec<AssetEntry>,
    /// A character family's files from the character library
    /// (`assets/characters/<family>/`), not an imported model's. Its
    /// "model" is the body's parts, which isn't placed on its own, so its
    /// tile isn't dragged, deleted or made global.
    pub character: bool,
}

impl Package {
    /// What's in it, such as "1 model, 1 clip, 1 state machine, 25 textures".
    pub fn summary(&self) -> String {
        let count = |kinds: &[AssetKind]| {
            self.members
                .iter()
                .filter(|entry| kinds.contains(&entry.kind))
                .count()
        };
        let parts = [
            (count(&[AssetKind::Model]), "model"),
            (count(&[AssetKind::Animation]), "clip"),
            (count(&[AssetKind::Controller]), "state machine"),
            (
                count(&[AssetKind::Texture, AssetKind::TextureSet, AssetKind::Image]),
                "texture",
            ),
        ];
        let mut text: Vec<String> = parts
            .iter()
            .filter(|(n, _)| *n > 0)
            .map(|&(n, thing)| match n {
                1 => format!("1 {thing}"),
                n => format!("{n} {thing}s"),
            })
            .collect();
        let others = self.members.len() - parts.iter().map(|(n, _)| n).sum::<usize>();
        if others > 0 {
            text.push(format!("{others} more"));
        }
        text.join(", ")
    }

    /// True if `entry` is one of its files.
    pub fn holds(&self, entry: &AssetEntry) -> bool {
        self.members.iter().any(|member| member.path == entry.path)
    }

    /// True if the project folder `dir` (`assets/...`, `/` between
    /// folders) is this package's or inside it, so the panel needn't offer
    /// it as a folder of its own.
    pub fn covers(&self, dir: &str) -> bool {
        let dir = dir.to_lowercase();
        let roots: Vec<String> = if self.character {
            match character_family(&self.model.path) {
                Some(family) => vec![format!("{CHARACTERS}{family}")],
                None => Vec::new(),
            }
        } else {
            let name = self.name.to_lowercase();
            vec![format!("{MODELS}{name}"), format!("{ANIMATIONS}{name}")]
        };
        roots
            .iter()
            .any(|root| dir == *root || dir.starts_with(&format!("{root}/")))
    }
}

/// The character family folder `path` is in, in lower case: `human` for
/// `assets/characters/human/skins/young_asian_male.jpg`. None for a file
/// right in `assets/characters` or outside it.
fn character_family(path: &str) -> Option<String> {
    let lower = path.to_lowercase();
    let rest = lower.strip_prefix(CHARACTERS)?;
    let (family, _) = rest.split_once('/')?;
    Some(family.to_owned())
}

/// A character family's folder tile's name: `human` is "Human character".
fn character_name(folder: &str) -> String {
    let mut letters = folder.chars();
    let first: String = letters
        .next()
        .into_iter()
        .flat_map(char::to_uppercase)
        .collect();
    format!("{first}{} character", letters.as_str())
}

/// The package `path` (a file in the project) would belong to, by name in
/// lower case: the folder it's in under `assets/models` or
/// `assets/animations`, or for a model file right in `assets/models`, its
/// own name.
fn key(path: &str, kind: AssetKind) -> Option<String> {
    // A character family's key can't be a model folder's: it has a `/`.
    if let Some(family) = character_family(path) {
        return Some(format!("characters/{family}"));
    }
    let lower = path.to_lowercase();
    for root in [MODELS, ANIMATIONS] {
        if let Some(rest) = lower.strip_prefix(root) {
            return match rest.split_once('/') {
                Some((folder, _)) => Some(folder.to_owned()),
                None if root == MODELS && kind == AssetKind::Model => {
                    rest.rsplit_once('.').map(|(stem, _)| stem.to_owned())
                }
                None => None,
            };
        }
    }
    None
}

/// Groups `files` into packages: one per model in `assets/models` (in a
/// folder of its own, or a file on its own), holding every file in its
/// folder and in `assets/animations/<name>/`, and one per character family
/// in `assets/characters/<family>/`, holding every file under it. Only
/// packages of more than one file are made; a model on its own stays a
/// plain tile.
pub fn packages(files: &[AssetEntry]) -> Vec<Package> {
    let mut packages: Vec<(String, Package)> = Vec::new();
    for entry in files.iter().filter(|entry| entry.kind == AssetKind::Model) {
        let Some(key) = key(&entry.path, entry.kind) else {
            continue;
        };
        if packages.iter().any(|(known, _)| *known == key) {
            continue;
        }
        let character = entry.path.to_lowercase().starts_with(CHARACTERS);
        let name = if character {
            let rest = &entry.path[CHARACTERS.len()..];
            character_name(rest.split_once('/').map_or(rest, |(folder, _)| folder))
        } else {
            match entry.path[MODELS.len()..].split_once('/') {
                Some((folder, _)) => folder.to_owned(),
                None => entry.name.clone(),
            }
        };
        packages.push((
            key,
            Package {
                name,
                model: entry.clone(),
                members: Vec::new(),
                character,
            },
        ));
    }
    // A character family's model is its body: a model right in its folder
    // (`human/HumanParts.glb`) rather than the first hair or garment.
    for entry in files.iter().filter(|entry| entry.kind == AssetKind::Model) {
        let Some(family) = character_family(&entry.path) else {
            continue;
        };
        let in_family_folder = entry.path[CHARACTERS.len()..].matches('/').count() == 1;
        if !in_family_folder {
            continue;
        }
        let key = format!("characters/{family}");
        if let Some((_, package)) = packages.iter_mut().find(|(known, _)| *known == key)
            && package.model.path[CHARACTERS.len()..].matches('/').count() > 1
        {
            package.model = entry.clone();
        }
    }
    for entry in files {
        let Some(key) = key(&entry.path, entry.kind) else {
            continue;
        };
        if let Some((_, package)) = packages.iter_mut().find(|(known, _)| *known == key) {
            package.members.push(entry.clone());
        }
    }
    let mut packages: Vec<Package> = packages
        .into_iter()
        .map(|(_, package)| package)
        .filter(|package| package.members.len() > 1)
        .collect();
    for package in &mut packages {
        // Stable, so each kind keeps path order.
        package.members.sort_by_key(|entry| order(entry.kind));
    }
    packages.sort_by_key(|package| package.name.to_lowercase());
    packages
}

/// Where a kind of file comes in a package: what's used most first.
pub fn order(kind: AssetKind) -> u8 {
    match kind {
        AssetKind::Model | AssetKind::Prefab | AssetKind::Prop => 0,
        AssetKind::Controller => 1,
        AssetKind::Animation | AssetKind::Sequence => 2,
        AssetKind::TextureSet | AssetKind::Material => 3,
        AssetKind::Texture | AssetKind::Image | AssetKind::Particle => 4,
        AssetKind::Audio | AssetKind::BehaviorTree | AssetKind::DataTable | AssetKind::Sky => 5,
    }
}

/// The heading for a kind's files inside an open package.
pub fn section(kind: AssetKind) -> &'static str {
    match order(kind) {
        0 => "Model",
        1 | 2 => "Animation",
        3 | 4 => "Textures",
        _ => "Other",
    }
}

/// One thing in the panel's grid or list.
pub(super) enum Shown {
    Asset(AssetEntry),
    /// An imported model's files, as one folder.
    Folder(Package),
}

/// A run of things in the grid, under a heading: a kind of file inside an
/// open folder, or a source's section.
pub(super) struct Block {
    pub(super) heading: Option<String>,
    /// The source this block is the section of, when grouped by source.
    pub(super) origin: Option<Origin>,
    pub(super) items: Vec<Shown>,
}

/// What the panel shows, in order, from the `entries` the category and
/// search let through. With a search, every match on its own. Inside the
/// `open` folder, its files under headings. Otherwise the props, then a
/// folder for each imported model holding anything shown, then the files
/// not in one.
pub(super) fn shown(
    entries: Vec<AssetEntry>,
    packages: &[Package],
    open: Option<&Package>,
    searching: bool,
) -> Vec<Block> {
    let block = |items| Block {
        heading: None,
        origin: None,
        items,
    };
    if searching {
        return vec![block(entries.into_iter().map(Shown::Asset).collect())];
    }
    if let Some(package) = open {
        let mut blocks: Vec<Block> = Vec::new();
        for member in &package.members {
            if !entries.iter().any(|entry| entry.path == member.path) {
                continue;
            }
            let heading = section(member.kind);
            match blocks.last_mut() {
                Some(last) if last.heading.as_deref() == Some(heading) => {
                    last.items.push(Shown::Asset(member.clone()));
                }
                _ => blocks.push(Block {
                    heading: Some(heading.to_owned()),
                    origin: None,
                    items: vec![Shown::Asset(member.clone())],
                }),
            }
        }
        return blocks;
    }
    let (props, files): (Vec<AssetEntry>, Vec<AssetEntry>) = entries
        .into_iter()
        .partition(|entry| entry.kind == AssetKind::Prop);
    let mut shown: Vec<Shown> = props.into_iter().map(Shown::Asset).collect();
    for package in packages {
        if files.iter().any(|entry| package.holds(entry)) {
            shown.push(Shown::Folder(package.clone()));
        }
    }
    shown.extend(
        files
            .into_iter()
            .filter(|entry| !packages.iter().any(|package| package.holds(entry)))
            .map(Shown::Asset),
    );
    if shown.is_empty() {
        Vec::new()
    } else {
        vec![block(shown)]
    }
}

/// [`shown`], in a section for each source the `entries` came from (in
/// [`Origin`]'s order: built-in, made in Axcenity, imported, each Toolbox
/// source, global assets). Each section holds its model folders and files
/// as `shown` would, with an asset made from another (a material from its
/// texture set) right after it. Inside the `open` folder, its files under
/// their kinds' headings as before.
pub(super) fn grouped(
    entries: Vec<AssetEntry>,
    packages: &[Package],
    open: Option<&Package>,
    searching: bool,
    origins: &Origins,
) -> Vec<Block> {
    if open.is_some() && !searching {
        return shown(entries, packages, open, searching);
    }
    let mut blocks = Vec::new();
    for origin in origins.present(&entries) {
        let mine: Vec<AssetEntry> = entries
            .iter()
            .filter(|entry| origins.of(entry) == origin)
            .cloned()
            .collect();
        let folders: Vec<Package> = packages
            .iter()
            .filter(|package| origins.of(&package.model) == origin)
            .cloned()
            .collect();
        let mut items: Vec<Shown> = shown(mine, &folders, None, searching)
            .into_iter()
            .flat_map(|block| block.items)
            .collect();
        follow_parents(&mut items, origins);
        if !items.is_empty() {
            blocks.push(Block {
                heading: Some(origin.label()),
                origin: Some(origin),
                items,
            });
        }
    }
    blocks
}

/// Moves each asset listed under another (a material under its texture
/// set) to just after it, when both are in `items`; the rest keep their
/// order.
fn follow_parents(items: &mut Vec<Shown>, origins: &Origins) {
    let path = |item: &Shown| match item {
        Shown::Asset(entry) => Some(entry.path.clone()),
        Shown::Folder(_) => None,
    };
    let children: Vec<(String, String)> = items
        .iter()
        .filter_map(|item| {
            let child = path(item)?;
            let parent = origins.parent(&child)?.to_owned();
            items
                .iter()
                .any(|other| path(other).as_deref() == Some(parent.as_str()))
                .then_some((child, parent))
        })
        .collect();
    for (child, parent) in children {
        let Some(from) = items
            .iter()
            .position(|item| path(item).as_deref() == Some(child.as_str()))
        else {
            continue;
        };
        let moved = items.remove(from);
        // After the parent and any children already moved behind it.
        let mut at = items
            .iter()
            .position(|item| path(item).as_deref() == Some(parent.as_str()))
            .map_or(items.len(), |at| at + 1);
        while at < items.len()
            && path(&items[at]).is_some_and(|next| origins.parent(&next) == Some(parent.as_str()))
        {
            at += 1;
        }
        items.insert(at, moved);
    }
}

impl AssetBrowser {
    /// The row above an open folder's files: back to every asset, then the
    /// folder's name and what's in it. True when back is clicked.
    pub(super) fn folder_header(&self, ui: &mut egui::Ui, package: &Package) -> bool {
        let palette = theme::palette(ui.ctx());
        let back = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let back = ui
                    .add(
                        widgets::button(
                            widgets::Role::Ghost,
                            Some(icons::ARROW_LEFT),
                            "All assets",
                            &palette,
                        )
                        .min_size(egui::vec2(0.0, 24.0)),
                    )
                    .on_hover_text("Back to every asset")
                    .clicked();
                ui.label(egui::RichText::new(icons::CARET_RIGHT).color(palette.text_muted));
                ui.label(
                    egui::RichText::new(theme::with_icon(icons::FOLDER_SIMPLE, &package.name))
                        .color(palette.text_strong),
                );
                ui.label(
                    egui::RichText::new(package.summary())
                        .size(theme::type_scale::SMALL)
                        .color(palette.text_weak),
                );
                back
            })
            .inner;
        ui.add_space(4.0);
        back
    }

    /// Draws an imported model's folder: a tile, or a row of the list when
    /// `list`. Clicking opens it, and dragging it places its model. True
    /// when it's opened, and what its right-click menu asked for.
    pub(super) fn folder_item(
        &mut self,
        ui: &mut egui::Ui,
        package: &Package,
        can_edit: bool,
        list: bool,
    ) -> (bool, Option<ItemMenu>) {
        let (tile, picture_side) = (self.tile_size.tile(), self.tile_size.picture());
        let size = if list {
            egui::vec2(ui.available_width(), LIST_ROW)
        } else {
            tile
        };
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let id = response.id;
        let global = !package.character && self.is_global(&package.model.path);
        let response = response.on_hover_text(if package.character {
            format!(
                "{}\n{}: the character library's body, hair, clothing and skins.\nClick to open.",
                package.name,
                package.summary(),
            )
        } else {
            format!(
                "{}\n{}, from one import.\nClick to open. Drag into the viewport to place the model.{}",
                package.name,
                package.summary(),
                if global {
                    "\nGlobal asset: in every project."
                } else {
                    ""
                }
            )
        });
        let mut menu = None;
        let local = self.is_local(&package.model.path);
        let paths: Vec<String> = package
            .members
            .iter()
            .filter(|member| self.is_local(&member.path))
            .map(|member| member.path.clone())
            .collect();
        response.context_menu(|ui| {
            if !package.character {
                menu = self.global_checkbox(ui, &package.model.path);
            }
            if local && let Some(chosen) = super::export_package_button(ui, &package.name, &paths) {
                menu = Some(chosen);
            }
            // Characters wear what's in a family's folder, so it isn't
            // deleted from here.
            if !can_edit || !local || package.character {
                return;
            }
            ui.separator();
            // The model's own files are the whole folder: its folder in
            // `assets/models` and its clips in `assets/animations`.
            let model = [package.model.path.clone()];
            if let Some(chosen) = super::delete_button(ui, &package.name, &model) {
                menu = Some(chosen);
            }
        });
        if can_edit && !package.character && response.drag_started() {
            egui::DragAndDrop::set_payload(ui.ctx(), AssetPayload(package.model.clone()));
        }
        let palette = theme::palette(ui.ctx());
        let hover = theme::ease(
            ui.ctx(),
            id.with("hover"),
            response.hovered() || response.dragged(),
        );
        let painter = ui.painter();
        painter.rect_filled(rect, 6.0, palette.control_hover.gamma_multiply(hover));
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                6.0,
                egui::Stroke::new(1.0_f32, palette.accent),
                egui::StrokeKind::Inside,
            );
        }
        let (picture, side) = if list {
            let picture = egui::Rect::from_center_size(
                egui::pos2(rect.min.x + 6.0 + LIST_PICTURE / 2.0, rect.center().y),
                egui::vec2(LIST_PICTURE, LIST_PICTURE),
            );
            (picture, LIST_PICTURE)
        } else {
            let picture = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.min.y + 6.0 + picture_side / 2.0),
                egui::vec2(picture_side, picture_side),
            );
            (picture, picture_side)
        };
        let color = AssetKind::Model.color(ui.visuals());
        painter.rect(
            picture,
            5.0,
            palette.inset,
            egui::Stroke::new(1.0_f32, palette.border),
            egui::StrokeKind::Inside,
        );
        painter.text(
            picture.center(),
            egui::Align2::CENTER_CENTER,
            icons::FOLDER_SIMPLE,
            egui::FontId::proportional(side * 0.6),
            color,
        );
        // The model's icon on the folder, so it reads as "a model's files"
        // (a person's for a character's).
        painter.text(
            picture.center() + egui::vec2(0.0, side * 0.06),
            egui::Align2::CENTER_CENTER,
            if package.character {
                icons::PERSON
            } else {
                icons::CUBE_TRANSPARENT
            },
            egui::FontId::proportional(side * 0.26),
            color,
        );
        if global {
            global_badge(ui, picture);
        }
        if !list {
            // How many files, in a badge at the bottom right.
            let count = package.members.len().to_string();
            let galley = painter.layout_no_wrap(
                count,
                egui::FontId::proportional(theme::type_scale::CAPTION),
                palette.text_strong,
            );
            let badge = egui::Rect::from_min_size(
                picture.right_bottom() - galley.size() - egui::vec2(10.0, 6.0),
                galley.size() + egui::vec2(8.0, 2.0),
            );
            painter.rect_filled(badge, 6.0, palette.control_active);
            painter.galley(
                badge.min + egui::vec2(4.0, 1.0),
                galley,
                palette.text_strong,
            );
        }
        let text_color = if response.hovered() {
            palette.text_strong
        } else {
            palette.text
        };
        if list {
            let mut name_right = rect.max.x - 6.0;
            if rect.width() >= 200.0 {
                let kind = ui.painter().text(
                    rect.right_center() - egui::vec2(8.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    format!("Folder · {}", package.members.len()),
                    egui::FontId::proportional(theme::type_scale::SMALL),
                    palette.text_weak,
                );
                name_right = kind.min.x - 8.0;
            }
            let left = picture.max.x + 8.0;
            let galley = widgets::elided(
                ui,
                package.name.clone(),
                egui::FontId::proportional(theme::type_scale::BODY),
                text_color,
                (name_right - left).max(1.0),
                1,
            );
            ui.painter().galley(
                egui::pos2(left, rect.center().y - galley.size().y / 2.0),
                galley,
                text_color,
            );
        } else {
            let galley = widgets::elided_name(
                ui,
                &package.name,
                egui::FontId::proportional(theme::type_scale::SMALL),
                text_color,
                tile.x - 8.0,
                2,
            );
            ui.painter().galley(
                egui::pos2(rect.center().x, picture.max.y + 5.0),
                galley,
                text_color,
            );
        }
        // While dragged, the model's icon follows the mouse.
        if response.dragged()
            && !package.character
            && let Some(pointer) = ui.ctx().pointer_latest_pos()
        {
            let layer = egui::LayerId::new(egui::Order::Tooltip, id.with("dragged"));
            let ghost = egui::Rect::from_center_size(
                pointer + egui::vec2(18.0, 18.0),
                egui::vec2(40.0, 40.0),
            );
            let painter = ui.ctx().layer_painter(layer);
            painter.rect_filled(ghost.expand(3.0), 6.0, ui.visuals().window_fill);
            painter.text(
                ghost.center(),
                egui::Align2::CENTER_CENTER,
                AssetKind::Model.icon(),
                egui::FontId::proportional(28.0),
                color,
            );
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        let opened = !response.dragged() && (response.clicked() || response.double_clicked());
        (opened, menu)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::{display_name, kind_of, scan};
    use super::*;

    fn entry(path: &str) -> AssetEntry {
        AssetEntry {
            kind: kind_of(path).unwrap(),
            path: path.to_owned(),
            name: display_name(path),
        }
    }

    #[test]
    fn an_imported_models_files_are_one_package() {
        let files: Vec<AssetEntry> = [
            "assets/animations/Door/Open.anim.json",
            "assets/animations/Walking/Walking.anim.json",
            "assets/animations/Walking/Walking.statemachine.json",
            "assets/audio/ding.wav",
            "assets/models/chair.glb",
            "assets/models/noodle.fbx",
            "assets/animations/noodle/Idle.anim.json",
            "assets/models/Walking/Walking.fbx",
            "assets/models/Walking/textures/Body_Diffuse.png",
            "assets/models/Walking/textures/Body_Normal.png",
        ]
        .iter()
        .map(|path| entry(path))
        .collect();
        let packages = packages(&files);
        let names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
        // The chair is on its own, and Door's clip has no model.
        assert_eq!(names, ["noodle", "Walking"]);

        let walking = &packages[1];
        assert_eq!(walking.model.path, "assets/models/Walking/Walking.fbx");
        let paths: Vec<&str> = walking.members.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "assets/models/Walking/Walking.fbx",
                "assets/animations/Walking/Walking.statemachine.json",
                "assets/animations/Walking/Walking.anim.json",
                "assets/models/Walking/textures/Body_Diffuse.png",
                "assets/models/Walking/textures/Body_Normal.png",
            ]
        );
        assert_eq!(
            walking.summary(),
            "1 model, 1 clip, 1 state machine, 2 textures"
        );
        assert!(walking.holds(&files[1]));
        assert!(!walking.holds(&files[0]));

        // A model file on its own in assets/models, with clips of its name.
        assert_eq!(packages[0].members.len(), 2);
        assert_eq!(section(walking.members[1].kind), "Animation");
    }

    /// Inserting a character copies its family's body, hair, clothing and
    /// skins into `assets/characters/<family>/`: one folder tile per
    /// family, named so it can't be taken for a model folder of the same
    /// name, whose model is the body.
    #[test]
    fn a_character_familys_files_are_one_package() {
        let files: Vec<AssetEntry> = [
            "assets/audio/ding.wav",
            "assets/characters/human/HumanParts.glb",
            "assets/characters/human/HumanRegions.png",
            "assets/characters/human/hair/short02/HairShort02.glb",
            "assets/characters/human/hair/short02/HairShort02.png",
            "assets/characters/human/items/bandeau/Bandeau.cover.png",
            "assets/characters/human/items/bandeau/Bandeau.glb",
            "assets/characters/human/skins/young_asian_male.jpg",
            "assets/characters/humanoid/HumanoidParts.glb",
            "assets/characters/humanoid/HumanoidRegions.png",
            "assets/models/Humanoid/Humanoid.glb",
            "assets/models/Humanoid/HumanoidAnimations.glb",
        ]
        .iter()
        .map(|path| entry(path))
        .collect();
        let packages = packages(&files);
        let names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Human character", "Humanoid", "Humanoid character"]);

        let human = &packages[0];
        assert!(human.character);
        assert!(!packages[1].character);
        assert_eq!(human.model.path, "assets/characters/human/HumanParts.glb");
        assert_eq!(human.members.len(), 7);
        assert_eq!(human.summary(), "3 models, 4 textures");
        assert!(!human.holds(&files[0]));
        assert_eq!(packages[2].members.len(), 2);

        // Its folders, and the folders inside them, are the tile's.
        assert!(human.covers("assets/characters/human"));
        assert!(human.covers("assets/characters/Human/items/bandeau"));
        assert!(!human.covers("assets/characters/humanoid"));
        assert!(!human.covers("assets/characters"));
        assert!(packages[1].covers("assets/models/Humanoid"));
        assert!(packages[1].covers("assets/animations/humanoid"));
        assert!(!packages[1].covers("assets/characters/humanoid"));

        // The All grid: the three folders and the sound, nothing loose.
        let shown: Vec<String> = shown(files.clone(), &packages, None, false)
            .into_iter()
            .flat_map(|block| block.items)
            .map(|item| match item {
                Shown::Asset(entry) => entry.name,
                Shown::Folder(package) => format!("[{}]", package.name),
            })
            .collect();
        assert_eq!(
            shown,
            [
                "[Human character]",
                "[Humanoid]",
                "[Humanoid character]",
                "ding"
            ]
        );
    }

    /// An imported Hero (model, two textures, a clip and a state machine)
    /// and a sound: the grid shows one Hero folder and the sound; opened,
    /// the folder shows Hero's files under headings, and a search finds
    /// them one by one. Both are drawn (with `AXCENITY_SNAPSHOTS`, saved as
    /// `assets-panel-<view>.png`).
    #[test]
    fn an_imported_models_files_show_as_one_folder_that_opens() {
        let folder =
            std::env::temp_dir().join(format!("axcenity-asset-folders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        for file in [
            "assets/models/Hero/Hero.fbx",
            "assets/models/Hero/textures/Hero_Diffuse.png",
            "assets/models/Hero/textures/Hero_Normal.png",
            "assets/animations/Hero/Walk.anim.json",
            "assets/animations/Hero/Hero.statemachine.json",
            "assets/audio/ding.wav",
        ] {
            let path = folder.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"").unwrap();
        }
        let files = scan(&folder);
        let packages = packages(&files);
        let describe = |blocks: Vec<Block>| -> Vec<String> {
            let mut lines = Vec::new();
            for block in blocks {
                lines.extend(block.heading.map(|heading| format!("# {heading}")));
                for item in block.items {
                    match item {
                        Shown::Asset(entry) if entry.kind == AssetKind::Prop => {}
                        Shown::Asset(entry) => lines.push(entry.name),
                        Shown::Folder(package) => lines.push(format!("[{}]", package.name)),
                    }
                }
            }
            lines
        };
        assert_eq!(
            describe(shown(files.clone(), &packages, None, false)),
            ["[Hero]", "ding"]
        );
        assert_eq!(
            describe(shown(files.clone(), &packages, Some(&packages[0]), false)),
            [
                "# Model",
                "Hero",
                "# Animation",
                "Hero",
                "Walk",
                "# Textures",
                "Hero_Diffuse",
                "Hero_Normal"
            ]
        );
        let found: Vec<AssetEntry> = files
            .iter()
            .filter(|entry| entry.path.contains("Normal"))
            .cloned()
            .collect();
        assert_eq!(
            describe(shown(found, &packages, None, true)),
            ["Hero_Normal"]
        );

        let mut browser = AssetBrowser::default();
        browser.open(Some(&folder));
        let ctx = theme::test_context();
        let size = egui::vec2(720.0, 320.0);
        let draw = |browser: &mut AssetBrowser, ui: &mut egui::Ui| {
            egui::CentralPanel::default().show(ui, |ui| browser.show(ui, true, false, None));
        };
        for view in ["all", "folder"] {
            if view == "folder" {
                browser.open_package = Some("hero".to_owned());
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            };
            let _ = crate::theme::run(&ctx, input, |ui| draw(&mut browser, ui));
            if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS") {
                crate::snapshot::snapshot(
                    &PathBuf::from(snapshots).join(format!("assets-panel-{view}.png")),
                    size,
                    vec![Vec::new(); 4],
                    |ui| draw(&mut browser, ui),
                );
            }
        }
        // The open folder is found by its name in any case.
        assert_eq!(browser.open_package.as_deref(), Some("hero"));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
