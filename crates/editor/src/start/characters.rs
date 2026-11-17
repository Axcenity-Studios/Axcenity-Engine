//! The start screen's Character Studio page (`docs/research/character-studio.md`
//! CS2; `docs/editor/character-studio.md`): make a new character from a
//! preset or edit a saved one, in the Character Studio workspace, with no
//! project open. From the top: Recent (the characters last opened or
//! saved, four and "Show more"), your characters, packs, and the presets.
//! Every character and preset is a card with its picture (drawn by
//! `character_workspace::thumbnails`) and an overview; saved ones can be
//! edited and deleted. `cards.rs` draws the cards, `dialogs.rs` the pack
//! editor and the questions before deleting.

use std::time::{Duration, Instant};

use super::*;
use crate::character_workspace::packs::{self, Pack};
use crate::character_workspace::shelf::{self, Overview, SavedCharacter};
use crate::character_workspace::thumbnails::{self, Subject, Thumbnails};
use crate::character_workspace::{Preset, presets, recent};

mod cards;
mod dialogs;
#[cfg(test)]
mod tests;

use cards::{CardButton, CardView, PackButton, character_card, pack_card};
use dialogs::ShelfDialog;

/// How many recent characters show before "Show more".
pub(super) const RECENT_SHOWN: usize = 4;
/// Character cards are at least this wide; a row fills the page.
const MIN_CHARACTER_CARD_WIDTH: f32 = 150.0;
/// Below this width the header's button goes under its title.
const NARROW_HEADER: f32 = 620.0;
/// Pack cards are at least this wide.
const MIN_PACK_CARD_WIDTH: f32 = 340.0;
/// The files are listed again this often while the page shows, so a
/// character saved elsewhere appears.
const LIST_EVERY: Duration = Duration::from_secs(3);

/// A saved character's card.
struct Card {
    saved: SavedCharacter,
    overview: Overview,
    picture: Option<PathBuf>,
}

/// A preset's card.
struct PresetCard {
    index: usize,
    preset: Preset,
    overview: Overview,
    picture: Option<PathBuf>,
}

/// The page's state: the characters, packs and presets as last listed,
/// their pictures, and its dialog.
#[derive(Default)]
pub(super) struct Shelf {
    thumbnails: Thumbnails,
    cards: Vec<Card>,
    /// The recent characters' files, newest first.
    recent: Vec<PathBuf>,
    packs: Vec<Pack>,
    presets: Vec<PresetCard>,
    /// The folder listed, and when.
    listed: Option<(PathBuf, Instant)>,
    /// Recent shows all its characters, not just the first four.
    more_recent: bool,
    dialog: Option<ShelfDialog>,
    /// What the last action did (or why it didn't), and whether it failed.
    notice: Option<(String, bool)>,
}

/// What a card or button on the page asked for.
enum Pick {
    Edit(PathBuf),
    Delete(PathBuf),
    Preset(usize),
    NewPack,
    ImportPack,
    Pack(usize, PackButton),
}

impl Shelf {
    /// Lists the folder's characters, packs and presets again and asks for
    /// the pictures that are missing.
    fn list(&mut self, folder: &Path) {
        let library = shelf::library_for(folder);
        self.cards = shelf::list(folder)
            .into_iter()
            .map(|saved| {
                let overview = saved
                    .description
                    .as_ref()
                    .map(|description| shelf::overview(description, &library))
                    .unwrap_or_else(|| Overview {
                        body: "Can't be read by this version".into(),
                        look: String::new(),
                    });
                let picture = saved
                    .description
                    .as_ref()
                    .and_then(|_| thumbnails::character_picture(folder, &saved.path, &saved.text));
                if let (Some(description), Some(picture)) = (&saved.description, &picture) {
                    self.thumbnails
                        .request(Subject::Character(description.clone()), picture.clone());
                }
                Card {
                    saved,
                    overview,
                    picture,
                }
            })
            .collect();
        self.recent = recent::recent(folder);
        self.packs = packs::packs(folder);
        self.presets = presets()
            .into_iter()
            .enumerate()
            .filter(|(_, preset)| library.bases.is_empty() || library.base(preset.base).is_some())
            .map(|(index, preset)| {
                let overview =
                    shelf::overview(&preset.description(preset.name, &library), &library);
                let picture = thumbnails::preset_picture(folder, index);
                if let Some(picture) = &picture {
                    self.thumbnails
                        .request(Subject::Preset(index), picture.clone());
                }
                PresetCard {
                    index,
                    preset,
                    overview,
                    picture,
                }
            })
            .collect();
        self.listed = Some((folder.to_owned(), Instant::now()));
    }

    /// Lists again on the next frame.
    fn stale(&mut self) {
        self.listed = None;
    }

    fn card(&self, path: &Path) -> Option<&Card> {
        self.cards.iter().find(|card| card.saved.path == path)
    }

    /// The picture of a card, if it's drawn.
    fn texture(
        &mut self,
        ctx: &egui::Context,
        picture: Option<&Path>,
    ) -> Option<egui::TextureHandle> {
        self.thumbnails.texture(ctx, picture?)
    }

    fn pending(&self, picture: Option<&Path>) -> bool {
        picture.is_some_and(|picture| self.thumbnails.pending(picture))
    }
}

impl StartScreen {
    /// Where Character Studio keeps its files (the page lists the saved
    /// characters there).
    pub fn set_characters_folder(&mut self, folder: Option<PathBuf>) {
        self.shelf.thumbnails.set_folder(folder.clone());
        self.shelf.stale();
        self.characters_folder = folder;
    }

    /// The editor's GPU device, which the page's pictures are drawn on.
    pub fn set_gpu(
        &mut self,
        device: &axcenity_render::wgpu::Device,
        queue: &axcenity_render::wgpu::Queue,
    ) {
        self.shelf.thumbnails.set_gpu(device, queue);
    }

    /// The workspace saved `file`: its picture is drawn again, and the page
    /// lists it when it next shows.
    pub fn character_saved(&mut self, file: &Path) {
        let Some(folder) = self.characters_folder.clone() else {
            return;
        };
        if let Some(saved) = SavedCharacter::read(file)
            && let Some(description) = saved.description
            && let Some(picture) = thumbnails::character_picture(&folder, file, &saved.text)
        {
            self.shelf
                .thumbnails
                .request(Subject::Character(description), picture);
        }
        self.shelf.stale();
    }

    pub(super) fn characters_page(&mut self, ui: &mut egui::Ui) -> Option<StartAction> {
        let ctx = ui.ctx().clone();
        let palette = theme::palette(&ctx);
        self.shelf.thumbnails.poll(&ctx);
        if let Some(folder) = self.characters_folder.clone() {
            let fresh = self
                .shelf
                .listed
                .as_ref()
                .is_some_and(|(listed, at)| *listed == folder && at.elapsed() < LIST_EVERY);
            if !fresh {
                self.shelf.list(&folder);
            }
            ctx.request_repaint_after(LIST_EVERY);
        }
        let mut action = None;
        let mut pick = None;
        let title = |ui: &mut egui::Ui| {
            ui.label(
                RichText::new("Character Studio")
                    .family(theme::display_family())
                    .size(type_scale::DISPLAY)
                    .extra_letter_spacing(theme::DISPLAY_TRACKING)
                    .color(palette.text_strong),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Create a character, dress it and save it, then use it in any game.",
                    )
                    .color(palette.text_secondary),
                )
                .wrap(),
            );
        };
        let new_character = |ui: &mut egui::Ui| {
            let new = widgets::button(Role::Primary, Some(icons::PLUS), "New Character", &palette)
                .min_size(vec2(0.0, 32.0));
            ui.add(new)
                .on_hover_text("Open Character Studio on the default character")
                .clicked()
        };
        // In a narrow window the button goes under the title.
        let clicked = if ui.available_width() < NARROW_HEADER {
            ui.vertical(|ui| {
                title(ui);
                ui.add_space(10.0);
                new_character(ui)
            })
            .inner
        } else {
            ui.horizontal(|ui| {
                ui.vertical(title);
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    new_character,
                )
                .inner
            })
            .inner
        };
        if clicked {
            action = Some(StartAction::NewCharacter(0));
        }
        if let Some((notice, failed)) = &self.shelf.notice {
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(RichText::new(notice).color(if *failed {
                    palette.error
                } else {
                    palette.text_secondary
                }))
                .wrap(),
            );
        }
        ui.add_space(24.0);

        // Recent: the first four, then the rest on "Show more".
        let recent: Vec<PathBuf> = self
            .shelf
            .recent
            .iter()
            .filter(|path| self.shelf.card(path).is_some())
            .cloned()
            .collect();
        if !recent.is_empty() {
            widgets::section_label(ui, "Recent");
            ui.add_space(6.0);
            let shown = if self.shelf.more_recent {
                recent.len()
            } else {
                recent.len().min(RECENT_SHOWN)
            };
            if let Some(picked) = self.character_grid(ui, &recent[..shown], "recent") {
                pick = Some(picked);
            }
            if recent.len() > RECENT_SHOWN {
                ui.add_space(4.0);
                let (icon, label) = if self.shelf.more_recent {
                    (icons::CARET_UP, "Show less".to_owned())
                } else {
                    (
                        icons::CARET_DOWN,
                        format!("Show {} more", recent.len() - RECENT_SHOWN),
                    )
                };
                if ui
                    .add(widgets::button(Role::Ghost, Some(icon), &label, &palette))
                    .clicked()
                {
                    self.shelf.more_recent = !self.shelf.more_recent;
                }
            }
            ui.add_space(24.0);
        }

        widgets::section_label(ui, "Your characters");
        ui.add_space(6.0);
        if self.shelf.cards.is_empty() {
            empty_state(
                ui,
                icons::USER,
                "No characters yet",
                "Characters you save in Character Studio appear here. Start from a preset below.",
            );
        } else {
            let all: Vec<PathBuf> = self
                .shelf
                .cards
                .iter()
                .map(|card| card.saved.path.clone())
                .collect();
            if let Some(picked) = self.character_grid(ui, &all, "all") {
                pick = Some(picked);
            }
        }
        ui.add_space(24.0);

        if let Some(picked) = self.packs_section(ui, &palette) {
            pick = Some(picked);
        }
        ui.add_space(24.0);

        widgets::section_label(ui, "Start from a preset");
        ui.add_space(6.0);
        if let Some(picked) = self.preset_grid(ui) {
            pick = Some(picked);
        }

        if let Some(picked) = pick
            && let Some(asked) = self.follow_pick(picked)
        {
            action = Some(asked);
        }
        if let Some(folder) = self.characters_folder.clone() {
            self.shelf_dialog(&ctx, &folder);
        }
        action
    }

    /// Cards for the saved characters at `paths`, in rows that fill the
    /// page.
    fn character_grid(
        &mut self,
        ui: &mut egui::Ui,
        paths: &[PathBuf],
        section: &str,
    ) -> Option<Pick> {
        let ctx = ui.ctx().clone();
        let (columns, width) = columns(ui.available_width(), MIN_CHARACTER_CARD_WIDTH);
        let mut pick = None;
        let now = SystemTime::now();
        ui.spacing_mut().item_spacing.y = CARD_SPACING;
        for row in paths.chunks(columns) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = CARD_SPACING;
                for path in row {
                    let Some(card) = self.shelf.card(path) else {
                        continue;
                    };
                    let (name, overview, picture) = (
                        card.saved.name.clone(),
                        card.overview.clone(),
                        card.picture.clone(),
                    );
                    let edited = card
                        .saved
                        .modified
                        .map(|time| format!("Edited {}", ago(time, now)));
                    let texture = self.shelf.texture(&ctx, picture.as_deref());
                    let view = CardView {
                        id: egui::Id::new(("character card", section, path)),
                        name: &name,
                        overview: &overview,
                        picture: texture.as_ref(),
                        pending: self.shelf.pending(picture.as_deref()),
                        hover: edited.unwrap_or_else(|| path.display().to_string()),
                    };
                    match character_card(ui, &view, width, false) {
                        Some(CardButton::Main) => pick = Some(Pick::Edit(path.clone())),
                        Some(CardButton::Delete) => pick = Some(Pick::Delete(path.clone())),
                        None => {}
                    }
                }
            });
        }
        pick
    }

    /// The presets' cards: picking one starts a new character from it.
    fn preset_grid(&mut self, ui: &mut egui::Ui) -> Option<Pick> {
        let ctx = ui.ctx().clone();
        let (columns, width) = columns(ui.available_width(), MIN_CHARACTER_CARD_WIDTH);
        let mut pick = None;
        ui.spacing_mut().item_spacing.y = CARD_SPACING;
        let presets: Vec<(usize, Preset, Overview, Option<PathBuf>)> = self
            .shelf
            .presets
            .iter()
            .map(|card| {
                (
                    card.index,
                    card.preset,
                    card.overview.clone(),
                    card.picture.clone(),
                )
            })
            .collect();
        for row in presets.chunks(columns) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = CARD_SPACING;
                for (index, preset, overview, picture) in row {
                    let texture = self.shelf.texture(&ctx, picture.as_deref());
                    let view = CardView {
                        id: egui::Id::new(("preset card", index)),
                        name: preset.name,
                        overview,
                        picture: texture.as_ref(),
                        pending: self.shelf.pending(picture.as_deref()),
                        hover: preset.about.to_owned(),
                    };
                    if character_card(ui, &view, width, true).is_some() {
                        pick = Some(Pick::Preset(*index));
                    }
                }
            });
        }
        pick
    }

    /// The packs, with New Pack and Import Pack.
    fn packs_section(&mut self, ui: &mut egui::Ui, palette: &Palette) -> Option<Pick> {
        let ctx = ui.ctx().clone();
        let mut pick = None;
        ui.horizontal(|ui| {
            widgets::section_label(ui, "Packs");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if ui
                    .add(widgets::button(
                        Role::Secondary,
                        Some(icons::DOWNLOAD_SIMPLE),
                        "Import Pack…",
                        palette,
                    ))
                    .on_hover_text("Add the characters of an exported pack (.characterpack.json)")
                    .clicked()
                {
                    pick = Some(Pick::ImportPack);
                }
                let can_make = !self.shelf.cards.is_empty();
                let new = ui
                    .add_enabled(
                        can_make,
                        widgets::button(Role::Secondary, Some(icons::PLUS), "New Pack…", palette),
                    )
                    .on_hover_text("Group several saved characters as a pack")
                    .on_disabled_hover_text("Save a character first");
                if new.clicked() {
                    pick = Some(Pick::NewPack);
                }
            });
        });
        ui.add_space(6.0);
        if self.shelf.packs.is_empty() {
            empty_state(
                ui,
                icons::PACKAGE,
                "No packs yet",
                "A pack keeps several characters together: export it as one file, or add it to a game project.",
            );
            return pick;
        }
        let folder = self.characters_folder.clone().unwrap_or_default();
        let (columns, width) = columns(ui.available_width(), MIN_PACK_CARD_WIDTH);
        ui.spacing_mut().item_spacing.y = CARD_SPACING;
        let packs = self.shelf.packs.clone();
        for (row, chunk) in packs.chunks(columns).enumerate() {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = CARD_SPACING;
                for (column, pack) in chunk.iter().enumerate() {
                    let members: Vec<(String, Option<egui::TextureHandle>)> = pack
                        .members(&folder)
                        .iter()
                        .map(|path| {
                            let card = self.shelf.card(path);
                            let name =
                                card.map_or_else(String::new, |card| card.saved.name.clone());
                            let picture = card.and_then(|card| card.picture.clone());
                            (name, self.shelf.texture(&ctx, picture.as_deref()))
                        })
                        .collect();
                    if let Some(button) = pack_card(ui, pack, &members, width) {
                        pick = Some(Pick::Pack(row * columns + column, button));
                    }
                }
            });
        }
        pick
    }

    /// Does what a card or button asked for.
    fn follow_pick(&mut self, pick: Pick) -> Option<StartAction> {
        let folder = self.characters_folder.clone()?;
        match pick {
            Pick::Edit(path) => return Some(StartAction::OpenCharacter(path)),
            Pick::Preset(index) => return Some(StartAction::NewCharacter(index)),
            Pick::Delete(path) => {
                let name = self
                    .shelf
                    .card(&path)
                    .map_or_else(String::new, |card| card.saved.name.clone());
                self.shelf.dialog = Some(ShelfDialog::DeleteCharacter { name, path });
            }
            Pick::NewPack => self.shelf.dialog = Some(ShelfDialog::edit_pack(None)),
            Pick::ImportPack => {
                let picked = rfd::FileDialog::new()
                    .set_title("Import a character pack")
                    .add_filter("Character pack", &["json"])
                    .pick_file();
                if let Some(file) = picked {
                    self.import_pack(&folder, &file);
                }
            }
            Pick::Pack(index, button) => {
                let pack = self.shelf.packs.get(index)?.clone();
                match button {
                    PackButton::Edit => {
                        self.shelf.dialog = Some(ShelfDialog::edit_pack(Some(&pack)));
                    }
                    PackButton::Export => {
                        let picked = rfd::FileDialog::new()
                            .set_title("Export the pack")
                            .set_file_name(packs::export_file_name(&pack))
                            .add_filter("Character pack", &["json"])
                            .save_file();
                        if let Some(to) = picked {
                            self.shelf.notice =
                                Some(match packs::export_pack(&folder, &pack, &to) {
                                    Ok(()) => (
                                        format!("Exported \"{}\" to {}.", pack.name, to.display()),
                                        false,
                                    ),
                                    Err(error) => (error, true),
                                });
                        }
                    }
                    PackButton::AddToProject => {
                        let picked = rfd::FileDialog::new()
                            .set_title("Add the pack's characters to a project")
                            .pick_folder();
                        if let Some(project) = picked {
                            self.add_pack_to_project(&folder, &pack, &project);
                        }
                    }
                    PackButton::Delete => self.shelf.dialog = Some(ShelfDialog::DeletePack(pack)),
                }
            }
        }
        None
    }

    /// Imports the pack exported to `file`.
    fn import_pack(&mut self, folder: &Path, file: &Path) {
        self.shelf.notice = Some(match packs::import_pack(folder, file) {
            Ok(pack) => (
                format!(
                    "Imported \"{}\" with {} character{}.",
                    pack.name,
                    pack.characters.len(),
                    if pack.characters.len() == 1 { "" } else { "s" }
                ),
                false,
            ),
            Err(error) => (error, true),
        });
        self.shelf.stale();
    }

    /// Writes `pack`'s characters into the game project at `project`.
    fn add_pack_to_project(&mut self, folder: &Path, pack: &Pack, project: &Path) {
        self.shelf.notice = Some(match packs::add_to_project(folder, pack, project) {
            Ok(written) => (
                format!(
                    "Added {} character{} to {} ({}).",
                    written.len(),
                    if written.len() == 1 { "" } else { "s" },
                    project.display(),
                    packs::PROJECT_FOLDER
                ),
                false,
            ),
            Err(error) => (error, true),
        });
    }
}

/// How many cards at least `min` wide fit in a row `width` wide, and how
/// wide each is so the row fills it.
fn columns(width: f32, min: f32) -> (usize, f32) {
    let columns = ((width + CARD_SPACING) / (min + CARD_SPACING))
        .floor()
        .max(1.0);
    let card = ((width - CARD_SPACING * (columns - 1.0)) / columns)
        .floor()
        .max(1.0);
    (columns as usize, card)
}
