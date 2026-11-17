//! The Character Studio page's dialogs: the pack editor (new pack, or a
//! pack's name and characters) and the questions before deleting a
//! character or a pack, in the editor's standard dialog.

use super::*;

/// The page's open dialog.
pub(in crate::start) enum ShelfDialog {
    DeleteCharacter {
        name: String,
        path: PathBuf,
    },
    DeletePack(Pack),
    /// A new pack (`editing` is `None`) or a pack being changed.
    EditPack {
        editing: Option<PathBuf>,
        name: String,
        /// The characters picked, by file name, in the pack's order.
        picked: Vec<String>,
        error: Option<String>,
    },
}

impl ShelfDialog {
    /// The pack editor, on `pack` or on a new one.
    pub(in crate::start) fn edit_pack(pack: Option<&Pack>) -> Self {
        Self::EditPack {
            editing: pack.map(|pack| pack.path.clone()),
            name: pack.map_or_else(String::new, |pack| pack.name.clone()),
            picked: pack.map_or_else(Vec::new, |pack| pack.characters.clone()),
            error: None,
        }
    }
}

/// How wide the pack editor is.
const PACK_DIALOG_WIDTH: f32 = 520.0;

impl StartScreen {
    /// Shows the page's dialog, if one is open, and does what it's asked.
    pub(super) fn shelf_dialog(&mut self, ctx: &egui::Context, folder: &Path) {
        let Some(mut open) = self.shelf.dialog.take() else {
            return;
        };
        let close = match &mut open {
            ShelfDialog::DeleteCharacter { name, path } => {
                let message = format!(
                    "\"{name}\" will be deleted from this computer with its picture, and taken \
                     out of Recent and its packs. This can't be undone."
                );
                let outcome = dialog::ask(
                    ctx,
                    &dialog::Question {
                        id: "delete character",
                        icon: icons::TRASH,
                        tone: dialog::Tone::Danger,
                        title: "Delete character?",
                        message: &message,
                        items: vec![(icons::FILE, path.display().to_string())],
                    },
                    &[
                        dialog::Button::cancel(),
                        dialog::Button::new(icons::TRASH, "Delete Character", dialog::Kind::Danger),
                    ],
                );
                if outcome == Some(dialog::Outcome::Picked(1)) {
                    self.delete_character(folder, &path.clone());
                }
                outcome.is_some()
            }
            ShelfDialog::DeletePack(pack) => {
                let message = format!(
                    "The pack \"{}\" will be deleted. Its characters are kept.",
                    pack.name
                );
                let outcome = dialog::ask(
                    ctx,
                    &dialog::Question {
                        id: "delete pack",
                        icon: icons::TRASH,
                        tone: dialog::Tone::Danger,
                        title: "Delete pack?",
                        message: &message,
                        items: Vec::new(),
                    },
                    &[
                        dialog::Button::cancel(),
                        dialog::Button::new(icons::TRASH, "Delete Pack", dialog::Kind::Danger),
                    ],
                );
                if outcome == Some(dialog::Outcome::Picked(1)) {
                    self.shelf.notice = Some(match packs::delete_pack(pack) {
                        Ok(()) => (format!("Deleted the pack \"{}\".", pack.name), false),
                        Err(error) => (error, true),
                    });
                    self.shelf.stale();
                }
                outcome.is_some()
            }
            ShelfDialog::EditPack {
                editing,
                name,
                picked,
                error,
            } => {
                let (saved, dismissed) = dialog::modal(ctx, "edit pack", PACK_DIALOG_WIDTH, |ui| {
                    self.pack_editor(ui, folder, editing.as_deref(), name, picked, error)
                });
                saved || dismissed
            }
        };
        if !close {
            self.shelf.dialog = Some(open);
        }
    }

    /// Deletes the saved character at `path`: its file, picture, Recent
    /// entry and place in packs.
    pub(super) fn delete_character(&mut self, folder: &Path, path: &Path) {
        let picture = self.shelf.card(path).and_then(|card| card.picture.clone());
        let name = self
            .shelf
            .card(path)
            .map_or_else(String::new, |card| card.saved.name.clone());
        self.shelf.notice = Some(match shelf::delete_character(folder, path) {
            Ok(()) => (format!("Deleted \"{name}\"."), false),
            Err(error) => (error, true),
        });
        if let Some(picture) = picture {
            self.shelf.thumbnails.forget(&picture);
        }
        self.shelf.stale();
    }

    /// The pack editor's body: the name, the characters to pick and the
    /// buttons. Returns true once the pack is saved or Cancel is pressed.
    fn pack_editor(
        &mut self,
        ui: &mut egui::Ui,
        folder: &Path,
        editing: Option<&Path>,
        name: &mut String,
        picked: &mut Vec<String>,
        error: &mut Option<String>,
    ) -> bool {
        let palette = theme::palette(ui.ctx());
        let ctx = ui.ctx().clone();
        dialog::header(
            ui,
            icons::PACKAGE,
            dialog::Tone::Question,
            if editing.is_some() {
                "Edit pack"
            } else {
                "New pack"
            },
            Some("Name the pack and pick its characters."),
        );
        ui.label(RichText::new("Name").color(palette.text_secondary));
        ui.add(dialog::text_field(name).hint_text("Villagers"));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Characters").color(palette.text_secondary));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} picked", picked.len()))
                        .size(type_scale::SMALL)
                        .color(palette.text_weak),
                );
            });
        });
        let rows: Vec<(String, String, String, Option<PathBuf>)> = self
            .shelf
            .cards
            .iter()
            .map(|card| {
                (
                    card.saved.file_name(),
                    card.saved.name.clone(),
                    card.overview.body.clone(),
                    card.picture.clone(),
                )
            })
            .collect();
        egui::Frame::new()
            .fill(palette.inset)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .corner_radius(theme::radius::CONTROL)
            .inner_margin(6.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for (file, shown, body, picture) in &rows {
                            let mut on = picked.contains(file);
                            let texture = self.shelf.texture(&ctx, picture.as_deref());
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut on, "").changed() {
                                    if on {
                                        picked.push(file.clone());
                                    } else {
                                        picked.retain(|other| other != file);
                                    }
                                }
                                let (rect, _) =
                                    ui.allocate_exact_size(vec2(32.0, 40.0), Sense::hover());
                                cards::small_picture(ui, rect, shown, texture.as_ref());
                                ui.vertical(|ui| {
                                    ui.add_space(3.0);
                                    ui.label(RichText::new(shown).color(palette.text_strong));
                                    ui.label(
                                        RichText::new(body)
                                            .size(type_scale::SMALL)
                                            .color(palette.text_weak),
                                    );
                                });
                            });
                        }
                    });
            });
        if let Some(error) = error.as_deref() {
            ui.add_space(6.0);
            ui.label(RichText::new(error).color(palette.error));
        }
        let ready = !name.trim().is_empty() && !picked.is_empty();
        let save = if editing.is_some() {
            "Save Pack"
        } else {
            "Make Pack"
        };
        let pressed = dialog::buttons(
            ui,
            &[
                dialog::Button::cancel(),
                dialog::Button::new(icons::CHECK, save, dialog::Kind::Main)
                    .enabled(ready)
                    .tooltip(if ready {
                        "Save the pack"
                    } else {
                        "Name the pack and pick at least one character"
                    }),
            ],
            dialog::Enter::Main,
        );
        match pressed {
            Some(0) => true,
            Some(_) => match packs::save_pack(folder, editing, name, picked.clone()) {
                Ok(pack) => {
                    self.shelf.notice = Some((format!("Saved the pack \"{}\".", pack.name), false));
                    self.shelf.stale();
                    true
                }
                Err(problem) => {
                    *error = Some(problem);
                    false
                }
            },
            None => false,
        }
    }
}
