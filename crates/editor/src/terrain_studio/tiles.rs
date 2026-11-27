//! The Tiles section: grow the landscape with more Terrains of the same
//! size, edge to edge (`axcenity_terrain::tiles`, `docs/terrain.md`).

use axcenity_terrain::tiles::{self, Side};

use super::*;

impl TerrainStudio {
    /// How many tiles the landscape has, how a new one is filled, and an
    /// Add button for each side of this tile (off where one is already).
    pub(super) fn tiles_section(
        &mut self,
        ui: &mut egui::Ui,
        world: &World,
        terrain: InstanceId,
        actions: &mut Vec<TerrainAction>,
    ) {
        let palette = theme::palette(ui.ctx());
        let count = tiles::connected(world, terrain).len();
        let free = tiles::free_sides(world, terrain);
        section(ui, "Tiles", |ui| {
            hint(
                ui,
                &match count {
                    1 => "One tile. Add another beside it to grow the landscape: same size, edge to edge.".to_owned(),
                    n => format!("This landscape has {n} tiles."),
                },
            );
            responsive::form(ui, "terrain tile fill", |ui| {
                field(ui, "New ground");
                widgets::segmented(
                    ui,
                    &mut self.tile_fill,
                    &[
                        (TileFill::Continue, None, "Continues it"),
                        (TileFill::Flat, None, "Flat"),
                    ],
                );
                ui.end_row();
            });
            let busy = self.job.is_some();
            let mut button = |ui: &mut egui::Ui, side: Side| {
                let open = free.contains(&side) && !busy;
                let tip = if busy {
                    "Wait for the job to finish.".to_owned()
                } else if open {
                    format!("Add a tile on this tile's {} side.", side.label())
                } else {
                    format!("There's already a terrain on the {} side.", side.label())
                };
                let clicked = ui
                    .add_enabled(
                        open,
                        widgets::button(Role::Secondary, Some(icons::PLUS), side.label(), &palette),
                    )
                    .on_hover_text(tip)
                    .on_disabled_hover_text(if busy {
                        "Wait for the job to finish."
                    } else {
                        "That side already has a terrain."
                    })
                    .clicked();
                if clicked {
                    actions.push(TerrainAction::AddTile(side));
                }
            };
            // Laid out as seen from above, -Z away from the viewer.
            egui::Grid::new(("terrain tiles", terrain))
                .num_columns(3)
                .spacing([4.0, 4.0])
                .show(ui, |ui| {
                    ui.label("");
                    button(ui, Side::NegZ);
                    ui.label("");
                    ui.end_row();
                    button(ui, Side::NegX);
                    ui.label(
                        egui::RichText::new("This tile")
                            .size(theme::type_scale::SMALL)
                            .color(palette.text_weak),
                    );
                    button(ui, Side::PosX);
                    ui.end_row();
                    ui.label("");
                    button(ui, Side::PosZ);
                    ui.label("");
                    ui.end_row();
                });
        });
    }
}
