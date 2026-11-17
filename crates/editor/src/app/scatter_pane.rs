//! The Scatter pane's side of the app (`scatter_pane.rs` is the panel):
//! reading and saving the project's palettes, carrying out the pane's
//! actions, adding what's clicked in Assets, and the brush's strokes, each
//! one undo step named by what it did ("Scatter 12 Items", "Erase
//! Scatter").
//!
//! The brush paints onto the Terrain Studio's terrain (the selected one, or
//! the first) through the same viewport plumbing as a terrain brush: while
//! it's armed, `scatter_brush_view` gives the viewport its circle and the
//! mouse goes to `scatter_pointer`. Arming it puts a terrain tool down, and
//! picking a terrain tool puts it down.

use axcenity_core::FoliageSource;
use axcenity_project::{
    ScatterPalette, WORLD_PRESETS_FOLDER, list_scatter_palettes, save_scatter_palette,
};
use axcenity_render::glam::Vec3;
use axcenity_terrain::brush::{Brush, EditRecorder, Operation, Scatter, ScatterErase, Stroke};

use super::*;
use crate::scatter_pane::{
    FIRST_PALETTE, PaletteEntry, RADIUS_RANGE, ScatterAction, ScatterContext, unique_name,
};
use crate::terrain_studio::ActiveStroke;
use crate::viewport::{TerrainBrush, TerrainPointer};

/// How much `[` and `]` change the brush's size.
const BRUSH_STEP: f32 = 1.15;
/// How much of the brush's radius fades toward its rim, thinning items there.
const FALLOFF: f32 = 0.3;

impl EditorApp {
    /// The Scatter panel.
    pub(super) fn scatter_pane(&mut self, ui: &mut egui::Ui) {
        self.refresh_scatter_palettes();
        let terrain = self
            .scatter_terrain()
            .and_then(|id| self.world.get(id))
            .map(|instance| instance.name().to_owned());
        let cx = ScatterContext {
            editing: self.project.is_some() && !self.playing(),
            terrain,
        };
        let actions = self.scatter.show(ui, &cx);
        for action in actions {
            self.scatter_action(action);
        }
    }

    /// Opens the Scatter pane (the ribbon's Models group, universal search).
    pub(super) fn open_scatter(&mut self) {
        self.layout.open(Pane::Scatter);
    }

    /// Carries out one of the pane's actions.
    pub(in crate::app) fn scatter_action(&mut self, action: ScatterAction) {
        if self.playing() {
            return;
        }
        match action {
            ScatterAction::Paint(on) => {
                self.finish_scatter_stroke();
                self.scatter.painting = on && self.scatter_terrain().is_some();
                if self.scatter.painting {
                    // One brush at a time.
                    self.terrain.pick(None);
                }
            }
            ScatterAction::Pick(on) => self.scatter.picking = on,
            ScatterAction::Select(index) => {
                self.save_scatter_palette();
                if index < self.scatter.palettes.len() {
                    self.scatter.current = index;
                }
            }
            ScatterAction::New => {
                self.save_scatter_palette();
                let name = unique_name(&self.scatter.palettes, FIRST_PALETTE, None);
                self.scatter.palettes.push(PaletteEntry {
                    file: None,
                    palette: ScatterPalette::new(&name),
                });
                self.scatter.current = self.scatter.palettes.len() - 1;
                self.scatter.dirty = true;
                self.save_scatter_palette();
            }
            ScatterAction::Rename(name) => {
                let current = self.scatter.current;
                let name = unique_name(&self.scatter.palettes, &name, Some(current));
                if let Some(palette) = self.scatter.palette_mut() {
                    palette.name = name;
                    self.scatter.dirty = true;
                }
                self.save_scatter_palette();
            }
            ScatterAction::Delete => self.delete_scatter_palette(),
            ScatterAction::OpenTerrain => self.layout.open(Pane::Terrain),
        }
    }

    /// Reads the project's palettes again if the project changed since they
    /// were read (saving the one shown first). A project without any starts
    /// with an empty one, saved once it's edited.
    pub(in crate::app) fn refresh_scatter_palettes(&mut self) {
        let folder = self.project.as_ref().map(|project| project.folder.clone());
        if self.scatter.read_for.as_ref() == Some(&folder) {
            return;
        }
        self.save_scatter_palette();
        let mut palettes = Vec::new();
        if let Some(folder) = &folder {
            let (found, problems) = list_scatter_palettes(&folder.join(WORLD_PRESETS_FOLDER));
            for (file, problem) in problems {
                self.output.warning(format!(
                    "Couldn't read the scatter palette {}: {problem}",
                    file.display()
                ));
            }
            palettes.extend(found.into_iter().map(|(file, palette)| PaletteEntry {
                file: Some(file),
                palette,
            }));
        }
        if palettes.is_empty() {
            palettes.push(PaletteEntry {
                file: None,
                palette: ScatterPalette::new(FIRST_PALETTE),
            });
        }
        self.finish_scatter_stroke();
        self.scatter.palettes = palettes;
        self.scatter.current = 0;
        self.scatter.dirty = false;
        self.scatter.read_for = Some(folder);
    }

    /// Writes the palette shown to its file if it changed (renamed: to its
    /// new file, removing the old one).
    pub(in crate::app) fn save_scatter_palette(&mut self) {
        if !self.scatter.dirty {
            return;
        }
        self.scatter.dirty = false;
        // The folder the palettes were read for, which is the project's
        // unless it was just closed or switched.
        let Some(Some(project)) = self.scatter.read_for.clone() else {
            return;
        };
        let folder = project.join(WORLD_PRESETS_FOLDER);
        let current = self.scatter.current;
        let Some(entry) = self.scatter.palettes.get(current).cloned() else {
            return;
        };
        // Renamed: move its file first, so a change of case alone (one file
        // on Windows and macOS) keeps it.
        let file = folder.join(axcenity_project::scatter_palette_file_name(
            &entry.palette.name,
        ));
        if let Some(old) = &entry.file
            && *old != file
            && old.is_file()
        {
            let _ = std::fs::create_dir_all(&folder);
            let _ = std::fs::rename(old, &file);
        }
        match save_scatter_palette(&folder, &entry.palette) {
            Ok(file) => self.scatter.palettes[current].file = Some(file),
            Err(error) => self.output.error(format!(
                "Couldn't save the scatter palette {}: {error}",
                entry.palette.name
            )),
        }
    }

    /// Deletes the palette shown and its file; the last one left becomes an
    /// empty "Palette" again.
    fn delete_scatter_palette(&mut self) {
        let current = self.scatter.current;
        if current >= self.scatter.palettes.len() {
            return;
        }
        let entry = self.scatter.palettes.remove(current);
        if let Some(file) = &entry.file
            && let Err(error) = std::fs::remove_file(file)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            self.output.error(format!(
                "Couldn't delete the scatter palette {}: {error}",
                file.display()
            ));
        }
        self.scatter.dirty = false;
        if self.scatter.palettes.is_empty() {
            self.scatter.palettes.push(PaletteEntry {
                file: None,
                palette: ScatterPalette::new(FIRST_PALETTE),
            });
        }
        self.scatter.current = current.min(self.scatter.palettes.len() - 1);
        self.output.info(format!(
            "Deleted the scatter palette {}.",
            entry.palette.name
        ));
    }

    /// Adds `source` (clicked in Assets) to the palette shown.
    pub(in crate::app) fn add_scatter_item(&mut self, source: FoliageSource) {
        self.refresh_scatter_palettes();
        if !self.scatter.add(source.clone()) {
            self.output
                .info(format!("{} is already in this palette.", source.name()));
        }
    }

    /// Whether the Assets panel adds what's clicked to the palette.
    pub(in crate::app) fn scatter_picking(&self) -> bool {
        self.scatter.picking && !self.playing() && self.layout.is_open(Pane::Scatter)
    }

    /// The terrain strokes paint onto: the Terrain Studio's (the selected
    /// one, or the first), if there's one.
    pub(in crate::app) fn scatter_terrain(&self) -> Option<InstanceId> {
        let all = crate::terrain_studio::terrains(&self.world);
        self.terrain
            .target
            .filter(|target| all.contains(target))
            .or_else(|| all.first().copied())
    }

    /// True while the brush has the viewport's left button: armed, and no
    /// terrain brush held instead.
    pub(in crate::app) fn scatter_brushing(&self) -> bool {
        self.scatter.painting
            && !self.playing()
            && !self.terrain.brushing()
            && self.scatter_terrain().is_some()
    }

    /// The brush to show in the viewport, while it's armed.
    pub(in crate::app) fn scatter_brush_view(&self, ctx: &egui::Context) -> Option<TerrainBrush> {
        if !self.scatter_brushing() {
            return None;
        }
        Some(TerrainBrush {
            terrain: self.scatter_terrain()?,
            radius: self.scatter.radius,
            falloff: FALLOFF,
            square: false,
            color: self.scatter.brush_color(theme::palette(ctx).accent),
        })
    }

    /// Runs every frame: puts the brush down when it can't be used, `[`,
    /// `]` and Escape while it's armed, and saves an edited palette once
    /// the mouse is let go.
    pub(super) fn scatter_frame(&mut self, ctx: &egui::Context) {
        let open = self.layout.is_open(Pane::Scatter);
        if self.playing() || self.project.is_none() || !open {
            self.scatter.painting = false;
            self.scatter.picking = false;
            self.finish_scatter_stroke();
        }
        // A terrain tool picked since the brush was armed puts it down.
        if self.scatter.painting && self.terrain.tool.is_some() {
            self.scatter.painting = false;
            self.finish_scatter_stroke();
        }
        if self.scatter.painting && self.scatter_terrain().is_none() {
            self.scatter.painting = false;
        }
        if self
            .scatter
            .stroke
            .as_ref()
            .is_some_and(|stroke| self.world.get(stroke.terrain).is_none())
        {
            self.scatter.stroke = None;
        }
        if self.scatter.painting && !ctx.egui_wants_keyboard_input() {
            let (smaller, bigger, escape) = ctx.input_mut(|input| {
                (
                    crate::shortcuts::consume(input, crate::shortcuts::Action::BrushSmaller),
                    crate::shortcuts::consume(input, crate::shortcuts::Action::BrushBigger),
                    crate::shortcuts::consume(input, crate::shortcuts::Action::PutBrushDown),
                )
            });
            let radius = &mut self.scatter.radius;
            if smaller {
                *radius = (*radius / BRUSH_STEP).max(RADIUS_RANGE.0);
            }
            if bigger {
                *radius = (*radius * BRUSH_STEP).min(RADIUS_RANGE.1);
            }
            if escape {
                self.scatter.painting = false;
                self.finish_scatter_stroke();
            }
        }
        if self.scatter.dirty && !ctx.input(|input| input.pointer.any_down()) {
            self.save_scatter_palette();
        }
    }

    /// The brush the strokes use: the pane's size, full strength, a soft rim.
    fn scatter_brush(&self) -> Brush {
        Brush {
            radius: self.scatter.radius,
            strength: 1.0,
            falloff: FALLOFF,
            ..Brush::default()
        }
    }

    /// The mouse over the terrain while the brush is armed: starts,
    /// continues and ends strokes. Shift erases.
    pub(in crate::app) fn scatter_pointer(&mut self, pointer: TerrainPointer) {
        if self.playing() {
            return;
        }
        let Some(terrain) = self.scatter_terrain() else {
            return;
        };
        if pointer.pressed
            && self.scatter.stroke.is_none()
            && let Some(point) = pointer.point
            && let Some(palette) = self.scatter.palette()
        {
            let erase = pointer
                .modifiers
                .shift
                .then_some(if self.scatter.erase_everything {
                    ScatterErase::Everything
                } else {
                    ScatterErase::Palette
                });
            let operation = Operation::Scatter(Scatter {
                items: palette.items.clone(),
                // The pane's density is per 100 square studs.
                density: palette.density / 100.0,
                spacing: palette.spacing,
                erase,
                seed: self.scatter.seed,
            });
            self.scatter.seed = self.scatter.seed.wrapping_add(1);
            self.scatter.stroke_start = self
                .world
                .terrain(terrain)
                .map_or(0, |data| data.foliage().len());
            self.scatter.stroke = Some(ActiveStroke {
                terrain,
                operation,
                stroke: Stroke::new(),
                recorder: EditRecorder::new(),
                label: if erase.is_some() {
                    "Erase Scatter".to_owned()
                } else {
                    "Scatter".to_owned()
                },
                once: false,
            });
            self.scatter_stroke_to(point, pointer.seconds);
        } else if pointer.down
            && let Some(point) = pointer.point
        {
            self.scatter_stroke_to(point, pointer.seconds);
        }
        if pointer.released || !pointer.down {
            self.finish_scatter_stroke();
        }
    }

    fn scatter_stroke_to(&mut self, point: Vec3, seconds: f32) {
        let Some(mut active) = self.scatter.stroke.take() else {
            return;
        };
        let brush = self.scatter_brush();
        super::terrain::stroke_active(&mut self.world, &mut active, &brush, point, seconds);
        self.scatter.stroke = Some(active);
    }

    /// Ends the stroke under way, as one undo step if it changed anything,
    /// named by how many items it placed.
    pub(in crate::app) fn finish_scatter_stroke(&mut self) {
        let Some(mut active) = self.scatter.stroke.take() else {
            return;
        };
        if let Operation::Scatter(scatter) = &active.operation
            && scatter.erase.is_none()
        {
            let now = self
                .world
                .terrain(active.terrain)
                .map_or(0, |data| data.foliage().len());
            let placed = now.saturating_sub(self.scatter.stroke_start);
            active.label = match placed {
                1 => "Scatter 1 Item".to_owned(),
                n => format!("Scatter {n} Items"),
            };
        }
        self.record_stroke(active);
    }
}

#[cfg(test)]
#[path = "scatter_pane_tests.rs"]
mod tests;
