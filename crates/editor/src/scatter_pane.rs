//! The Scatter pane: a palette of plants, props and models painted onto
//! Terrain as a weighted random mix (`docs/editor/scatter.md`).
//!
//! At the top, the palette picker with New, Rename and Delete. Then Paint,
//! which arms the brush in the viewport (drag scatters, Shift+drag erases,
//! `[` and `]` size it, Escape stops), the brush's size, density and
//! spacing, and the items: each with its share of the mix, size range,
//! collide, turn, lean and sink. Add from Assets makes a click on a prop or
//! model in the Assets panel add it.
//!
//! Palettes are the project's `presets/*.axscatter` files
//! (`project::scatter_palettes`). The panel edits the palette in place and
//! marks it [`ScatterPane::dirty`]; the app (`app/scatter_pane.rs`) saves
//! it, carries out the [`ScatterAction`]s and runs strokes, each one undo
//! step.

use std::path::PathBuf;

use axcenity_core::{FoliagePlant, FoliageProp, FoliageSource};
use axcenity_project::ScatterPalette;
use axcenity_terrain::brush::{MAX_ITEM_SCALE, MIN_ITEM_SCALE, ScatterItem, shares};
use eframe::egui;

use crate::responsive;
use crate::terrain_studio::{ActiveStroke, card, field, hint, section};
use crate::theme::{self, icons};
use crate::widgets::{self, Role};

/// The name a project's first palette gets.
pub const FIRST_PALETTE: &str = "Palette";
/// The brush's smallest and largest radius, in studs.
pub const RADIUS_RANGE: (f32, f32) = (0.5, 256.0);
/// Density's range: items per 100 square studs.
pub const DENSITY_RANGE: (f32, f32) = (0.1, 200.0);

/// A palette and the file it's saved in (`None` until it first is).
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteEntry {
    pub file: Option<PathBuf>,
    pub palette: ScatterPalette,
}

/// What the app should do, from the pane.
#[derive(Clone, Debug, PartialEq)]
pub enum ScatterAction {
    /// Arm (true) or put down the brush.
    Paint(bool),
    /// Start or stop adding what's clicked in Assets.
    Pick(bool),
    /// Show palette N.
    Select(usize),
    /// A new empty palette.
    New,
    /// Rename the palette shown.
    Rename(String),
    /// Delete the palette shown (and its file).
    Delete,
    /// Open the Terrain Studio (there's no terrain to paint on).
    OpenTerrain,
}

/// What the pane needs to know about the editor.
pub struct ScatterContext {
    /// False during Play, or with no project open: nothing can be changed.
    pub editing: bool,
    /// The terrain strokes paint onto, by name, if there is one.
    pub terrain: Option<String>,
}

/// The Scatter pane's palettes, brush and what's under way.
pub struct ScatterPane {
    /// The project's palettes, in name order. Never empty once read.
    pub palettes: Vec<PaletteEntry>,
    /// The palette shown and painted with.
    pub current: usize,
    /// The project folder the palettes were read for (`Some(None)`: no project).
    pub read_for: Option<Option<PathBuf>>,
    /// Studs from the brush's middle to its rim.
    pub radius: f32,
    /// Shift erases any painted foliage, not only the palette's items.
    pub erase_everything: bool,
    /// The brush is armed in the viewport.
    pub painting: bool,
    /// A click on a prop or model in Assets adds it to the palette.
    pub picking: bool,
    /// A stroke under way.
    pub stroke: Option<ActiveStroke>,
    /// How much painted foliage the terrain had when the stroke began, to
    /// name its undo step.
    pub stroke_start: usize,
    /// The palette shown changed since it was saved.
    pub dirty: bool,
    /// The next stroke's seed; each stroke takes the next one.
    pub seed: u64,
    /// Rename's text, while renaming.
    renaming: Option<String>,
}

impl Default for ScatterPane {
    fn default() -> Self {
        Self {
            palettes: Vec::new(),
            current: 0,
            read_for: None,
            radius: 12.0,
            erase_everything: false,
            painting: false,
            picking: false,
            stroke: None,
            stroke_start: 0,
            dirty: false,
            seed: crate::terrain_studio::random_seed(),
            renaming: None,
        }
    }
}

impl ScatterPane {
    /// The palette shown, if any are read.
    pub fn palette(&self) -> Option<&ScatterPalette> {
        self.palettes.get(self.current).map(|entry| &entry.palette)
    }

    pub fn palette_mut(&mut self) -> Option<&mut ScatterPalette> {
        self.palettes
            .get_mut(self.current)
            .map(|entry| &mut entry.palette)
    }

    /// The sources the palette holds, for the Assets panel to highlight.
    pub fn sources(&self) -> Vec<FoliageSource> {
        self.palette()
            .map(|palette| palette.items.iter().map(|i| i.source.clone()).collect())
            .unwrap_or_default()
    }

    /// Adds `source` to the palette shown, unless it's there already.
    /// True if it was added.
    pub fn add(&mut self, source: FoliageSource) -> bool {
        let Some(palette) = self.palette_mut() else {
            return false;
        };
        if palette.items.iter().any(|item| item.source == source) {
            return false;
        }
        palette.items.push(ScatterItem::new(source));
        self.dirty = true;
        true
    }

    /// The brush's color in the viewport: the first item's, else `accent`.
    pub fn brush_color(&self, accent: egui::Color32) -> egui::Color32 {
        let Some(item) = self
            .palette()
            .and_then(|palette| palette.items.iter().find(|item| item.places()))
        else {
            return accent;
        };
        let color = axcenity_scene::foliage_brush_color(&item.source);
        egui::Color32::from_rgb(
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
        )
    }

    /// Draws the pane and returns what was asked for.
    pub fn show(&mut self, ui: &mut egui::Ui, cx: &ScatterContext) -> Vec<ScatterAction> {
        let mut actions = Vec::new();
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_enabled_ui(cx.editing, |ui| {
                    self.header(ui);
                    if cx.terrain.is_none() {
                        no_terrain(ui, &mut actions);
                    }
                    self.palette_row(ui, &mut actions);
                    if self.palette().is_none() {
                        return;
                    }
                    self.paint_row(ui, cx, &mut actions);
                    let before = self.palette().cloned();
                    self.brush_section(ui);
                    self.items_section(ui, &mut actions);
                    if self.palette().cloned() != before {
                        self.dirty = true;
                    }
                });
            });
        actions
    }

    fn header(&self, ui: &mut egui::Ui) {
        let palette = theme::palette(ui.ctx());
        ui.label(
            egui::RichText::new(theme::with_icon(icons::TREE, "Scatter"))
                .family(theme::heading_family())
                .color(palette.text_strong),
        );
        hint(
            ui,
            "Paint a mix of plants, props and models onto terrain. Pick them from Assets.",
        );
        ui.add_space(4.0);
    }

    /// The palette picker, with New, Rename and Delete (or Rename's box).
    fn palette_row(&mut self, ui: &mut egui::Ui, actions: &mut Vec<ScatterAction>) {
        let colors = theme::palette(ui.ctx());
        if let Some(text) = &mut self.renaming {
            let mut done = None;
            ui.horizontal(|ui| {
                let edit = ui.add(
                    egui::TextEdit::singleline(text)
                        .hint_text("Palette name")
                        .desired_width(ui.available_width() - 70.0),
                );
                edit.request_focus();
                let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if widgets::icon_button(ui, icons::CHECK, "Rename", false).clicked() || enter {
                    done = Some(true);
                }
                if widgets::icon_button(ui, icons::X, "Cancel", false).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    done = Some(false);
                }
            });
            match done {
                Some(true) => {
                    let name = text.trim().to_owned();
                    if !name.is_empty() {
                        actions.push(ScatterAction::Rename(name));
                    }
                    self.renaming = None;
                }
                Some(false) => self.renaming = None,
                None => {}
            }
            return;
        }
        ui.horizontal(|ui| {
            let names: Vec<String> = self
                .palettes
                .iter()
                .map(|entry| entry.palette.name.clone())
                .collect();
            let mut chosen = self.current;
            egui::ComboBox::from_id_salt("scatter palette")
                .selected_text(names.get(self.current).cloned().unwrap_or_default())
                .width((ui.available_width() - 120.0).max(60.0))
                .show_ui(ui, |ui| {
                    for (index, name) in names.iter().enumerate() {
                        ui.selectable_value(&mut chosen, index, name);
                    }
                });
            if chosen != self.current {
                actions.push(ScatterAction::Select(chosen));
            }
            if widgets::icon_button(ui, icons::PLUS, "New palette", false).clicked() {
                actions.push(ScatterAction::New);
            }
            if widgets::icon_button(ui, icons::PENCIL_SIMPLE, "Rename this palette", false)
                .clicked()
            {
                self.renaming = Some(names.get(self.current).cloned().unwrap_or_default());
            }
            if widgets::icon_button(ui, icons::TRASH, "Delete this palette", false).clicked() {
                actions.push(ScatterAction::Delete);
            }
        });
        let saved = if self.dirty {
            "Saving…"
        } else {
            "Saved in the project's presets folder."
        };
        ui.label(
            egui::RichText::new(saved)
                .size(theme::type_scale::SMALL)
                .color(colors.text_weak),
        );
        ui.add_space(4.0);
    }

    /// Paint, which arms the brush, and how to use it.
    fn paint_row(
        &mut self,
        ui: &mut egui::Ui,
        cx: &ScatterContext,
        actions: &mut Vec<ScatterAction>,
    ) {
        let colors = theme::palette(ui.ctx());
        let placing = self
            .palette()
            .is_some_and(|palette| palette.items.iter().any(ScatterItem::places));
        let can_paint = cx.terrain.is_some();
        responsive::toolbar(ui, |ui| {
            let role = if self.painting {
                Role::Secondary
            } else {
                Role::Primary
            };
            let label = if self.painting {
                "Stop painting"
            } else {
                "Paint"
            };
            let button = ui
                .add_enabled(
                    can_paint,
                    widgets::button(role, Some(icons::PAINT_BRUSH), label, &colors)
                        .selected(self.painting),
                )
                .on_hover_text("Drag on the terrain to scatter the palette. Escape stops.")
                .on_disabled_hover_text("Scatter paints onto Terrain: make one first.");
            if button.clicked() {
                actions.push(ScatterAction::Paint(!self.painting));
            }
        });
        if self.painting {
            let text = match &cx.terrain {
                Some(name) if placing => format!(
                    "Drag on {name} to scatter. Shift+drag erases. [ and ] change the size."
                ),
                Some(_) => {
                    "Add an item, or turn one on, to scatter. Shift+drag still erases.".to_owned()
                }
                None => String::new(),
            };
            hint(ui, &text);
        }
    }

    fn brush_section(&mut self, ui: &mut egui::Ui) {
        let radius = &mut self.radius;
        let erase_everything = &mut self.erase_everything;
        let Some(palette) = self
            .palettes
            .get_mut(self.current)
            .map(|entry| &mut entry.palette)
        else {
            return;
        };
        section(ui, "Brush", |ui| {
            responsive::form(ui, "scatter brush", |ui| {
                field(ui, "Size").on_hover_text("The brush's radius. [ and ] change it.");
                ui.add(
                    egui::Slider::new(radius, RADIUS_RANGE.0..=RADIUS_RANGE.1)
                        .logarithmic(true)
                        .suffix(" studs"),
                );
                ui.end_row();
                field(ui, "Density").on_hover_text(
                    "How many items a stroke puts on each 100 square studs (a 10 × 10 stud patch).",
                );
                ui.add(
                    egui::DragValue::new(&mut palette.density)
                        .range(DENSITY_RANGE.0..=DENSITY_RANGE.1)
                        .speed(0.1)
                        .max_decimals(1)
                        .suffix(" per 100 studs²"),
                );
                ui.end_row();
                field(ui, "Spacing").on_hover_text(
                    "The least distance between a new item and any painted foliage, so strokes \
                     over the same ground don't pile items up. 0 lets them touch.",
                );
                ui.add(
                    egui::DragValue::new(&mut palette.spacing)
                        .range(0.0..=64.0)
                        .speed(0.1)
                        .max_decimals(2)
                        .suffix(" studs"),
                );
                ui.end_row();
                field(ui, "Erase").on_hover_text(
                    "What Shift+drag takes away: the palette's items, or any painted foliage.",
                );
                ui.checkbox(erase_everything, "Erase everything");
                ui.end_row();
            });
        });
    }

    fn items_section(&mut self, ui: &mut egui::Ui, actions: &mut Vec<ScatterAction>) {
        let picking = self.picking;
        let Some(palette) = self
            .palettes
            .get_mut(self.current)
            .map(|entry| &mut entry.palette)
        else {
            return;
        };
        section(ui, "Items", |ui| {
            let colors = theme::palette(ui.ctx());
            let add = ui
                .add(
                    widgets::button(
                        if picking { Role::Primary } else { Role::Secondary },
                        Some(icons::CURSOR_CLICK),
                        if picking {
                            "Adding from Assets…"
                        } else {
                            "Add from Assets"
                        },
                        &colors,
                    )
                    .selected(picking),
                )
                .on_hover_text(
                    "While on, clicking a prop or model in the Assets panel adds it to this palette.",
                );
            if add.clicked() {
                actions.push(ScatterAction::Pick(!picking));
            }
            if picking {
                hint(ui, "Click props or models in the Assets panel to add them.");
            }
            if palette.items.is_empty() {
                hint(
                    ui,
                    "No items yet. Turn on Add from Assets, then click the props or models to scatter.",
                );
                return;
            }
            let share = shares(&palette.items);
            let mut remove = None;
            for (index, item) in palette.items.iter_mut().enumerate() {
                ui.push_id(("scatter item", index), |ui| {
                    card(ui, |ui| {
                        if item_row(ui, item, share[index]) {
                            remove = Some(index);
                        }
                        item_settings(ui, item);
                    });
                });
            }
            if let Some(index) = remove {
                palette.items.remove(index);
            }
        });
    }
}

/// The card shown while there's no terrain to paint on.
fn no_terrain(ui: &mut egui::Ui, actions: &mut Vec<ScatterAction>) {
    let colors = theme::palette(ui.ctx());
    card(ui, |ui| {
        ui.label(
            egui::RichText::new("Scatter paints onto Terrain")
                .family(theme::medium_family())
                .color(colors.text_strong),
        );
        hint(
            ui,
            "This project has no terrain yet. Make one in the Terrain Studio, then paint here.",
        );
        if ui
            .add(widgets::button(
                Role::Secondary,
                Some(icons::MOUNTAINS),
                "Open Terrain Studio",
                &colors,
            ))
            .clicked()
        {
            actions.push(ScatterAction::OpenTerrain);
        }
    });
}

/// An item's on switch, icon, name and share, and its remove button.
/// True if remove was clicked.
fn item_row(ui: &mut egui::Ui, item: &mut ScatterItem, share: f32) -> bool {
    let colors = theme::palette(ui.ctx());
    let mut remove = false;
    ui.horizontal(|ui| {
        ui.checkbox(&mut item.enabled, "")
            .on_hover_text("Off, strokes don't place it (or erase it).");
        ui.label(
            egui::RichText::new(theme::with_icon(
                source_icon(&item.source),
                item.source.name(),
            ))
            .color(colors.text_strong),
        )
        .on_hover_text(item.source.entry());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            remove = widgets::icon_button(ui, icons::TRASH, "Take it out of the palette", false)
                .clicked();
            ui.label(
                egui::RichText::new(format!("{:.0}%", share * 100.0)).color(if share > 0.0 {
                    colors.text_secondary
                } else {
                    colors.text_weak
                }),
            )
            .on_hover_text("Its share of the mix: its weight against the other items that are on.");
        });
    });
    remove
}

/// An item's weight, size, collide, turn, lean and sink.
fn item_settings(ui: &mut egui::Ui, item: &mut ScatterItem) {
    responsive::form(ui, "scatter item settings", |ui| {
        field(ui, "Weight").on_hover_text(
            "Its share of the mix: an item of weight 2 is placed twice as often as one of 1.",
        );
        ui.add(
            egui::DragValue::new(&mut item.weight)
                .range(0.0..=100.0)
                .speed(0.05)
                .max_decimals(2),
        );
        ui.end_row();
        field(ui, "Size").on_hover_text(
            "The smallest and largest scale it's placed at, picked at random between (1 is its own size).",
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut item.min_scale)
                    .range(MIN_ITEM_SCALE..=MAX_ITEM_SCALE)
                    .speed(0.01)
                    .max_decimals(2)
                    .prefix("× "),
            );
            ui.label("to");
            ui.add(
                egui::DragValue::new(&mut item.max_scale)
                    .range(MIN_ITEM_SCALE..=MAX_ITEM_SCALE)
                    .speed(0.01)
                    .max_decimals(2)
                    .prefix("× "),
            );
        });
        ui.end_row();
        field(ui, "Collide").on_hover_text(
            "Solid: parts and characters rest on it. Off, it's visual only and they pass through.",
        );
        ui.checkbox(&mut item.collide, "Solid");
        ui.end_row();
        field(ui, "Rotation").on_hover_text(
            "How far each copy is turned at random about its up axis: 0 faces them all the same way.",
        );
        ui.add(egui::Slider::new(&mut item.rotation, 0.0..=360.0).suffix("°"));
        ui.end_row();
        field(ui, "Align to slope")
            .on_hover_text("0 stands it upright; 1 tilts it square to the ground under it.");
        ui.add(egui::Slider::new(&mut item.align_to_slope, 0.0..=1.0));
        ui.end_row();
        field(ui, "Sink").on_hover_text(
            "How far it's lowered into the ground, in studs, so it doesn't float on slopes.",
        );
        ui.add(
            egui::DragValue::new(&mut item.sink)
                .range(-16.0..=16.0)
                .speed(0.02)
                .max_decimals(2)
                .suffix(" studs"),
        );
        ui.end_row();
    });
    // Size's two ends stay in order.
    if item.min_scale > item.max_scale {
        std::mem::swap(&mut item.min_scale, &mut item.max_scale);
    }
}

/// The icon a plant, prop or model shows.
pub fn source_icon(source: &FoliageSource) -> &'static str {
    match source {
        FoliageSource::Plant(FoliagePlant::Grass) => icons::LEAF,
        FoliageSource::Plant(FoliagePlant::Bush) => icons::PLANT,
        FoliageSource::Plant(FoliagePlant::Flower) => icons::FLOWER,
        FoliageSource::Plant(FoliagePlant::Tree) | FoliageSource::Prop(FoliageProp::Tree) => {
            icons::TREE
        }
        FoliageSource::Prop(FoliageProp::Crate) => icons::PACKAGE,
        FoliageSource::Prop(FoliageProp::Lamp) => icons::LAMP,
        FoliageSource::Prop(FoliageProp::Table) => icons::TABLE,
        FoliageSource::Prop(FoliageProp::Floor) => icons::SQUARE,
        FoliageSource::Prop(FoliageProp::Wall) => icons::WALL,
        FoliageSource::Prop(FoliageProp::Stairs) => icons::STEPS,
        FoliageSource::Prop(FoliageProp::Ramp) => icons::TRIANGLE,
        FoliageSource::Prop(FoliageProp::Rock) => icons::POLYGON,
        FoliageSource::Prop(FoliageProp::Cliff) => icons::MOUNTAINS,
        FoliageSource::Model(_) => icons::CUBE_TRANSPARENT,
    }
}

/// `name`, or `name 2`, `name 3`… whichever no palette in `palettes` has
/// (ignoring case, as file names on Windows and macOS do), leaving out
/// palette `except`.
pub fn unique_name(palettes: &[PaletteEntry], name: &str, except: Option<usize>) -> String {
    let taken = |candidate: &str| {
        palettes.iter().enumerate().any(|(index, entry)| {
            Some(index) != except
                && axcenity_project::scatter_palette_file_name(&entry.palette.name)
                    .eq_ignore_ascii_case(&axcenity_project::scatter_palette_file_name(candidate))
        })
    };
    let name = name.trim();
    let name = if name.is_empty() { FIRST_PALETTE } else { name };
    if !taken(name) {
        return name.to_owned();
    }
    (2..)
        .map(|n| format!("{name} {n}"))
        .find(|candidate| !taken(candidate))
        .expect("some number is free")
}

#[cfg(test)]
mod tests;
