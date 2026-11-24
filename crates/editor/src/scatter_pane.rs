//! The Scatter pane: a palette of plants, props and models painted onto
//! Terrain as a weighted random mix (`docs/editor/scatter.md`).
//!
//! At the top, the palette picker with New, Rename and Delete. Then Paint,
//! which arms the brush in the viewport (drag scatters, Shift+drag erases,
//! `[` and `]` size it, Escape stops), the brush's size, density and
//! spacing, and the items: each with its share of the mix, size range,
//! collide, turn, lean and sink, and under Masks where it's placed more or
//! less (noise patches, slope, height, moisture). Add from Assets makes a
//! click on a prop or model in the Assets panel add it. The Decals section
//! is the palette's pictures and Paint decals, which arms the same brush
//! to lay them on the ground as ProjectedDecals instead
//! (`app/decal_scatter.rs`).
//!
//! Palettes are the project's `presets/*.axscatter` files
//! (`project::scatter_palettes`). The panel edits the palette in place and
//! marks it [`ScatterPane::dirty`]; the app (`app/scatter_pane.rs`) saves
//! it, carries out the [`ScatterAction`]s and runs strokes, each one undo
//! step.

use std::path::PathBuf;

use axcenity_core::{FoliagePlant, FoliageProp, FoliageSource};
use axcenity_project::{DECAL_SIZE_RANGE, ScatterDecal, ScatterPalette};
use axcenity_render::Quality;
use axcenity_terrain::brush::{
    MAX_ITEM_SCALE, MIN_ITEM_SCALE, MOISTURE_REACH, NOISE_SCALE, OPEN_HEIGHT, ScatterItem,
    ScatterMask, shares,
};
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
/// The decal brush's density range: decals per 100 square studs.
pub const DECAL_DENSITY_RANGE: (f32, f32) = (0.05, 50.0);

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
    /// Arm (true) or put down the brush, laying the palette's decals.
    PaintDecals(bool),
    /// Start or stop adding what's clicked in Assets.
    Pick(bool),
    /// Start or stop adding the pictures clicked in Assets as decals.
    PickDecals(bool),
    /// Show palette N.
    Select(usize),
    /// A new empty palette.
    New,
    /// A new palette from the ground clutter preset.
    NewGroundClutter,
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
    /// That terrain's lowest and highest possible ground (world Y), which
    /// a height mask starts from when it's switched on.
    pub heights: Option<(f32, f32)>,
}

/// A decal stroke under way: the brush dragged with Paint decals armed.
/// The world is changed as it goes; the stroke becomes one undo step when
/// it ends (`app/decal_scatter.rs`).
pub struct DecalStroke {
    /// The world as it was when the stroke began, for undo.
    pub before: axcenity_core::World,
    /// Spaces the stamps along the mouse's path.
    pub stroke: axcenity_terrain::brush::Stroke,
    /// The brush's settings for this stroke.
    pub brush: axcenity_procedural::decal_scatter::DecalBrush,
    /// Shift was held: it takes scattered decals away.
    pub erase: bool,
    /// Where every projected decal is, kept up as the stroke adds its own.
    pub existing: Vec<axcenity_render::glam::Vec3>,
    /// What decals land on with Land on parts: the terrains and colliding
    /// parts as they were when the stroke began.
    pub surfaces: Option<axcenity_procedural::Surfaces>,
    /// How many it has placed, taken away, and left out for the budget.
    pub placed: usize,
    pub erased: usize,
    pub over_budget: usize,
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
    /// The armed brush lays the palette's decals, not its items.
    pub decals: bool,
    /// A click on a picture in Assets adds it to the palette's decals.
    pub picking_decals: bool,
    /// Decals drop onto parts too (porches, rocks, roofs), not only the
    /// terrain under the brush.
    pub decals_on_parts: bool,
    /// The decal brush stops where Medium quality would start leaving
    /// decals out; off, where Ultra would.
    pub decals_for_medium: bool,
    /// A decal stroke under way.
    pub decal_stroke: Option<DecalStroke>,
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
            decals: false,
            picking_decals: false,
            decals_on_parts: false,
            decals_for_medium: true,
            decal_stroke: None,
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

    /// The pictures the palette's decals use, for the Assets panel to
    /// highlight.
    pub fn decal_textures(&self) -> Vec<String> {
        self.palette()
            .map(|palette| palette.decals.iter().map(|d| d.texture.clone()).collect())
            .unwrap_or_default()
    }

    /// Adds the picture `texture` (`aspect`: its width over its height) to
    /// the palette's decals, unless it's there already. True if it was
    /// added.
    pub fn add_decal(&mut self, texture: &str, aspect: f32) -> bool {
        let Some(palette) = self.palette_mut() else {
            return false;
        };
        if palette.decals.iter().any(|decal| decal.texture == texture) {
            return false;
        }
        palette.decals.push(ScatterDecal {
            aspect,
            ..ScatterDecal::new(texture)
        });
        self.dirty = true;
        true
    }

    /// The most projected decals the decal brush lets lie within a
    /// distance of one another: Medium quality's limits (so a camera
    /// standing at one draws every one near it, at Medium and above), or
    /// Ultra's, the most any draws.
    pub fn decal_budget(&self) -> axcenity_procedural::decal_scatter::DecalBudget {
        let quality = if self.decals_for_medium {
            Quality::Medium
        } else {
            Quality::Ultra
        };
        let limits = quality.decal_limits();
        axcenity_procedural::decal_scatter::DecalBudget {
            count: limits.count as usize,
            distance: limits.distance,
        }
    }

    /// The brush's color in the viewport: the first item's, else `accent`.
    pub fn brush_color(&self, accent: egui::Color32) -> egui::Color32 {
        if self.decals {
            return accent;
        }
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
                    self.items_section(ui, cx, &mut actions);
                    self.decals_section(ui, cx, &mut actions);
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
                .width((ui.available_width() - 150.0).max(60.0))
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
            if widgets::icon_button(
                ui,
                icons::POLYGON,
                "New ground clutter palette: gravel, pebbles and stones that lie to the slope \
                 and sink in a little",
                false,
            )
            .clicked()
            {
                actions.push(ScatterAction::NewGroundClutter);
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
            let items = self.painting && !self.decals;
            let role = if items {
                Role::Secondary
            } else {
                Role::Primary
            };
            let label = if items { "Stop painting" } else { "Paint" };
            let button = ui
                .add_enabled(
                    can_paint,
                    widgets::button(role, Some(icons::PAINT_BRUSH), label, &colors).selected(items),
                )
                .on_hover_text("Drag on the terrain to scatter the palette. Escape stops.")
                .on_disabled_hover_text("Scatter paints onto Terrain: make one first.");
            if button.clicked() {
                actions.push(ScatterAction::Paint(!items));
            }
        });
        if self.painting && !self.decals {
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

    fn items_section(
        &mut self,
        ui: &mut egui::Ui,
        cx: &ScatterContext,
        actions: &mut Vec<ScatterAction>,
    ) {
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
                        mask_settings(ui, &mut item.mask, cx.heights);
                    });
                });
            }
            if let Some(index) = remove {
                palette.items.remove(index);
            }
        });
    }
}

impl ScatterPane {
    /// The palette's decals: Paint decals, the decal brush's settings and
    /// each picture's.
    fn decals_section(
        &mut self,
        ui: &mut egui::Ui,
        cx: &ScatterContext,
        actions: &mut Vec<ScatterAction>,
    ) {
        let picking = self.picking_decals;
        let armed = self.painting && self.decals;
        let on_parts = &mut self.decals_on_parts;
        let for_medium = &mut self.decals_for_medium;
        let Some(palette) = self
            .palettes
            .get_mut(self.current)
            .map(|entry| &mut entry.palette)
        else {
            return;
        };
        section(ui, "Decals", |ui| {
            let colors = theme::palette(ui.ctx());
            hint(
                ui,
                "Lay pictures on the ground as projected decals: stains, tire tracks, grit \
                 round a wall's foot or under a vehicle.",
            );
            let placing = palette.decals.iter().any(ScatterDecal::places);
            responsive::toolbar(ui, |ui| {
                let paint = ui
                    .add_enabled(
                        cx.terrain.is_some(),
                        widgets::button(
                            if armed {
                                Role::Secondary
                            } else {
                                Role::Primary
                            },
                            Some(icons::IMAGE_SQUARE),
                            if armed {
                                "Stop painting decals"
                            } else {
                                "Paint decals"
                            },
                            &colors,
                        )
                        .selected(armed),
                    )
                    .on_hover_text(
                        "Drag on the terrain to lay this palette's decals. Shift+drag takes \
                         scattered decals away. Escape stops.",
                    )
                    .on_disabled_hover_text("The decal brush follows Terrain: make one first.");
                if paint.clicked() {
                    actions.push(ScatterAction::PaintDecals(!armed));
                }
                let add = ui
                    .add(
                        widgets::button(
                            if picking {
                                Role::Primary
                            } else {
                                Role::Secondary
                            },
                            Some(icons::CURSOR_CLICK),
                            if picking {
                                "Adding pictures…"
                            } else {
                                "Add pictures from Assets"
                            },
                            &colors,
                        )
                        .selected(picking),
                    )
                    .on_hover_text(
                        "While on, clicking a picture (an image or texture) in the Assets panel \
                         adds it to this palette's decals.",
                    );
                if add.clicked() {
                    actions.push(ScatterAction::PickDecals(!picking));
                }
            });
            if armed {
                hint(
                    ui,
                    if placing {
                        "Drag to lay decals. Shift+drag takes scattered decals away. [ and ] \
                         change the size."
                    } else {
                        "Add a picture, or turn one on, to lay decals. Shift+drag still takes \
                         them away."
                    },
                );
            }
            if picking {
                hint(ui, "Click pictures in the Assets panel to add them.");
            }
            responsive::form(ui, "scatter decal brush", |ui| {
                field(ui, "Density").on_hover_text(
                    "How many decals a stroke lays on each 100 square studs (a 10 × 10 stud patch).",
                );
                ui.add(
                    egui::DragValue::new(&mut palette.decal_density)
                        .range(DECAL_DENSITY_RANGE.0..=DECAL_DENSITY_RANGE.1)
                        .speed(0.05)
                        .max_decimals(2)
                        .suffix(" per 100 studs²"),
                );
                ui.end_row();
                field(ui, "Spacing").on_hover_text(
                    "The least distance between a new decal's middle and any other projected \
                     decal's, so strokes over the same ground don't pile them up. 0 lets them \
                     overlap freely.",
                );
                ui.add(
                    egui::DragValue::new(&mut palette.decal_spacing)
                        .range(0.0..=64.0)
                        .speed(0.1)
                        .max_decimals(2)
                        .suffix(" studs"),
                );
                ui.end_row();
                field(ui, "Surface").on_hover_text(
                    "Off, decals lie on the terrain under the brush, even under a vehicle or a \
                     porch. On, each drops from 20 studs up onto the first terrain or colliding \
                     part below: porches, rocks, roofs.",
                );
                ui.checkbox(on_parts, "Land on parts");
                ui.end_row();
                field(ui, "Limit").on_hover_text(
                    "The renderer draws only the nearest projected decals: none at Low quality, \
                     128 within 150 studs at Medium, 512 within 300 at High, 1,024 within 600 at \
                     Ultra. On, the brush stops where 128 already lie within 150 studs of a new decal, \
                     so a camera standing there draws them all at Medium and above. Off, it stops \
                     at 1,024 within 600.",
                );
                ui.checkbox(for_medium, "For Medium quality");
                ui.end_row();
            });
            if palette.decals.is_empty() {
                hint(
                    ui,
                    "No pictures yet. Turn on Add pictures from Assets, then click the images or \
                     textures to lay.",
                );
                return;
            }
            let total: f32 = palette
                .decals
                .iter()
                .filter(|decal| decal.places())
                .map(|decal| decal.weight)
                .sum();
            let mut remove = None;
            for (index, decal) in palette.decals.iter_mut().enumerate() {
                let share = if decal.places() && total > 0.0 {
                    decal.weight / total
                } else {
                    0.0
                };
                ui.push_id(("scatter decal", index), |ui| {
                    card(ui, |ui| {
                        if decal_row(ui, decal, share) {
                            remove = Some(index);
                        }
                        decal_settings(ui, decal);
                    });
                });
            }
            if let Some(index) = remove {
                palette.decals.remove(index);
            }
        });
    }
}

/// A decal's on switch, name and share, and its remove button. True if
/// remove was clicked.
fn decal_row(ui: &mut egui::Ui, decal: &mut ScatterDecal, share: f32) -> bool {
    let colors = theme::palette(ui.ctx());
    let mut remove = false;
    ui.horizontal(|ui| {
        ui.checkbox(&mut decal.enabled, "")
            .on_hover_text("Off, strokes don't lay it.");
        ui.label(
            egui::RichText::new(theme::with_icon(
                icons::IMAGE_SQUARE,
                &decal_name(&decal.texture),
            ))
            .color(colors.text_strong),
        )
        .on_hover_text(&decal.texture);
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
            .on_hover_text(
                "Its share of the mix: its weight against the other pictures that are on.",
            );
        });
    });
    remove
}

/// A picture's file name without its folder or extension.
pub fn decal_name(texture: &str) -> String {
    std::path::Path::new(texture)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(texture)
        .to_owned()
}

/// A decal's weight, size, opacity and turn.
fn decal_settings(ui: &mut egui::Ui, decal: &mut ScatterDecal) {
    responsive::form(ui, "scatter decal settings", |ui| {
        field(ui, "Weight").on_hover_text(
            "Its share of the mix: a picture of weight 2 is laid twice as often as one of 1.",
        );
        ui.add(
            egui::DragValue::new(&mut decal.weight)
                .range(0.0..=100.0)
                .speed(0.05)
                .max_decimals(2),
        );
        ui.end_row();
        field(ui, "Size").on_hover_text(
            "The smallest and largest its longer side is, in studs, picked at random between.",
        );
        ui.horizontal(|ui| {
            for size in [&mut decal.min_size, &mut decal.max_size] {
                ui.add(
                    egui::DragValue::new(size)
                        .range(DECAL_SIZE_RANGE.0..=DECAL_SIZE_RANGE.1)
                        .speed(0.05)
                        .max_decimals(2),
                );
            }
            ui.label("studs");
        });
        ui.end_row();
        field(ui, "Opacity").on_hover_text(
            "The faintest and strongest it's drawn, picked at random between: 0 is invisible, \
             1 solid.",
        );
        ui.horizontal(|ui| {
            for opacity in [&mut decal.min_opacity, &mut decal.max_opacity] {
                ui.add(
                    egui::DragValue::new(opacity)
                        .range(0.0..=1.0)
                        .speed(0.01)
                        .max_decimals(2),
                );
            }
        });
        ui.end_row();
        field(ui, "Rotation").on_hover_text(
            "How far each copy is turned at random on the surface: 0 lays them all the same way \
             (tracks along one line), 360 any way.",
        );
        ui.add(egui::Slider::new(&mut decal.rotation, 0.0..=360.0).suffix("°"));
        ui.end_row();
    });
    if decal.min_size > decal.max_size {
        std::mem::swap(&mut decal.min_size, &mut decal.max_size);
    }
    if decal.min_opacity > decal.max_opacity {
        std::mem::swap(&mut decal.min_opacity, &mut decal.max_opacity);
    }
}

/// An item's masks, folded away until they're opened: where
/// it's placed more or less (noise patches, slope, height, moisture).
/// `heights` is the terrain's lowest and highest ground, which the height
/// mask starts from.
fn mask_settings(ui: &mut egui::Ui, mask: &mut ScatterMask, heights: Option<(f32, f32)>) {
    // How many masks are on, in the title, so a folded item still says
    // it's masked (their names would run past a narrow pane).
    let on = [
        mask.has_noise(),
        mask.slope_range().is_some(),
        mask.height_range().is_some(),
        mask.has_moisture(),
    ]
    .into_iter()
    .filter(|&on| on)
    .count();
    let title = match on {
        0 => "Masks".to_owned(),
        n => format!("Masks ({n} on)"),
    };
    egui::CollapsingHeader::new(title)
        .id_salt("scatter item masks")
        .default_open(false)
        .show(ui, |ui| {
            responsive::form(ui, "scatter item masks grid", |ui| {
                field(ui, "Noise").on_hover_text(
                    "Gathers it into patches: 0 spreads it evenly, 1 leaves none between the \
                     patches.",
                );
                ui.add(egui::Slider::new(&mut mask.noise, 0.0..=1.0));
                ui.end_row();
                if mask.has_noise() {
                    field(ui, "Scale").on_hover_text("How far across a patch is, in studs.");
                    ui.add(
                        egui::DragValue::new(&mut mask.noise_scale)
                            .range(NOISE_SCALE.0..=NOISE_SCALE.1)
                            .speed(0.5)
                            .max_decimals(1)
                            .suffix(" studs"),
                    );
                    ui.end_row();
                    field(ui, "Threshold").on_hover_text(
                        "How much of the ground the patches leave bare: 0.5 is about half, \
                         higher is fewer, smaller patches.",
                    );
                    ui.add(egui::Slider::new(&mut mask.noise_threshold, 0.0..=1.0));
                    ui.end_row();
                    field(ui, "Contrast").on_hover_text(
                        "How sharp the patches' edges are: 0 fades across the whole patch, 1 is \
                         a hard edge.",
                    );
                    ui.add(egui::Slider::new(&mut mask.noise_contrast, 0.0..=1.0));
                    ui.end_row();
                    field(ui, "Seed")
                        .on_hover_text("Which patches: another number moves them all.");
                    ui.add(egui::DragValue::new(&mut mask.noise_seed).speed(1));
                    ui.end_row();
                }
                field(ui, "Slope").on_hover_text(
                    "The gentlest and steepest ground it's placed on, in degrees: 0 is flat, 90 \
                     a wall. 0 to 90 is anywhere.",
                );
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut mask.min_slope)
                            .range(0.0..=90.0)
                            .speed(0.25)
                            .max_decimals(1)
                            .suffix("°"),
                    );
                    ui.label("to");
                    ui.add(
                        egui::DragValue::new(&mut mask.max_slope)
                            .range(0.0..=90.0)
                            .speed(0.25)
                            .max_decimals(1)
                            .suffix("°"),
                    );
                });
                ui.end_row();
                field(ui, "Height")
                    .on_hover_text("Only between two heights (the ground's world Y, in studs).");
                let mut limited = mask.height_range().is_some();
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut limited, "").changed() {
                        (mask.min_height, mask.max_height) = if limited {
                            heights.unwrap_or((0.0, 100.0))
                        } else {
                            (-OPEN_HEIGHT, OPEN_HEIGHT)
                        };
                        // The terrain's whole range is still a limit, so
                        // the two numbers show.
                        if limited && mask.height_range().is_none() {
                            mask.max_height = OPEN_HEIGHT - 1.0;
                        }
                    }
                    if limited {
                        let limit = OPEN_HEIGHT - 1.0;
                        ui.add(
                            egui::DragValue::new(&mut mask.min_height)
                                .range(-limit..=limit)
                                .speed(0.25)
                                .max_decimals(1),
                        );
                        ui.label("to");
                        ui.add(
                            egui::DragValue::new(&mut mask.max_height)
                                .range(-limit..=limit)
                                .speed(0.25)
                                .max_decimals(1),
                        );
                    } else {
                        ui.label("Any");
                    }
                });
                ui.end_row();
                field(ui, "Moisture").on_hover_text(
                    "Worked out from the terrain's shape: hollows (washes, ditches, the foot of \
                     a slope) are wet, rises (ridges, knolls) dry. Above 0 it gathers in the \
                     wet, below 0 on the dry; 0 doesn't look.",
                );
                ui.add(egui::Slider::new(&mut mask.moisture, -1.0..=1.0));
                ui.end_row();
                if mask.has_moisture() {
                    field(ui, "Reach").on_hover_text(
                        "How far round a spot the ground is compared with, in studs: about half \
                         the width of the washes or ridges to pick out.",
                    );
                    ui.add(
                        egui::DragValue::new(&mut mask.moisture_reach)
                            .range(MOISTURE_REACH.0..=MOISTURE_REACH.1)
                            .speed(0.25)
                            .max_decimals(1)
                            .suffix(" studs"),
                    );
                    ui.end_row();
                }
            });
        });
    // Each range's two ends stay in order.
    if mask.min_slope > mask.max_slope {
        std::mem::swap(&mut mask.min_slope, &mut mask.max_slope);
    }
    if mask.min_height > mask.max_height {
        std::mem::swap(&mut mask.min_height, &mut mask.max_height);
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
