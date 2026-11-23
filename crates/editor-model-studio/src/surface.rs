//! Modular Surface Properties, Physics & Decals component for Model Studio
//! Object Mode (Source 2 ModelDoc approach).
//!
//! Picking a [`SurfacePreset`] (e.g. `Wood_Plank`, `Metal_Hollow`, `Concrete`,
//! `Flesh`) auto-computes mass from `bounding_box_volume × density` and links
//! default impact decal and sound sets, while exposing per-property manual
//! overrides when specific tuning is needed.

use axcenity_mesh_edit::{SurfacePreset, SurfaceProperties};
use eframe::egui;

use super::StudioDoc;
use crate::theme::{self, icons};

/// The collapsible header name.
pub const SURFACE_HEADER: &str = "Surface & Physics";

impl StudioDoc {
    /// Replaces the active object's surface properties as one undo step.
    pub fn set_surface_properties(&mut self, surface: SurfaceProperties) -> bool {
        let Some(object) = self.model.objects.get(self.active) else {
            return false;
        };
        if object.surface == surface {
            return false;
        }
        let before = self.snapshot();
        if let Some(object) = self.model.objects.get_mut(self.active) {
            object.surface = surface;
        }
        self.record("Surface Properties", before);
        true
    }
}

/// Renders the collapsible Surface Properties & Decals component card for the
/// active visual mesh object.
pub(super) fn panel(ui: &mut egui::Ui, doc: &mut StudioDoc, editable: bool) {
    let Some(object) = doc.model.objects.get(doc.active) else {
        return;
    };
    if object.mesh.is_none()
        || doc.model.is_lod_copy(doc.active)
        || doc.model.is_collision_piece(doc.active)
    {
        return;
    }
    let volume = object.bounding_box_volume();
    let mut surface = object.surface.clone();
    let original = surface.clone();
    let palette = theme::palette(ui.ctx());

    ui.add_space(6.0);
    egui::CollapsingHeader::new(theme::with_icon(icons::ATOM, SURFACE_HEADER))
        .id_salt("studio surface properties")
        .default_open(true)
        .show(ui, |ui| {
            ui.add_enabled_ui(editable, |ui| {
                // Surface Property Preset selector
                ui.horizontal(|ui| {
                    ui.label("Surface Preset");
                    egui::ComboBox::from_id_salt("studio surface preset")
                        .selected_text(surface.preset.label())
                        .show_ui(ui, |ui| {
                            for preset in SurfacePreset::ALL {
                                ui.selectable_value(&mut surface.preset, preset, preset.label());
                            }
                        });
                });

                // Compact summary pill showing density & computed volume
                let auto_mass = surface.computed_mass(volume);
                ui.horizontal_wrapped(|ui| {
                    ui.weak(format!(
                        "Density {:.0} kg/m³ · Vol {:.2} m³",
                        surface.preset.density(),
                        volume,
                    ));
                });

                ui.add_space(4.0);
                egui::Grid::new("studio surface grid")
                    .num_columns(3)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        // Mass (auto-computed or manual override)
                        ui.label("Mass");
                        let mut override_mass = surface.mass_override.is_some();
                        if ui
                            .checkbox(&mut override_mass, "Override")
                            .on_hover_text("Override auto-calculated mass (volume × density)")
                            .changed()
                        {
                            surface.mass_override = override_mass.then_some(auto_mass);
                        }
                        if let Some(mass) = &mut surface.mass_override {
                            ui.add(
                                egui::DragValue::new(mass)
                                    .speed(0.1)
                                    .range(0.01..=100_000.0)
                                    .suffix(" kg"),
                            );
                        } else {
                            ui.label(format!("{auto_mass:.1} kg (auto)"));
                        }
                        ui.end_row();

                        // Friction
                        ui.label("Friction");
                        let mut override_friction = surface.friction_override.is_some();
                        if ui
                            .checkbox(&mut override_friction, "Override")
                            .on_hover_text("Override the preset's friction coefficient")
                            .changed()
                        {
                            surface.friction_override =
                                override_friction.then_some(surface.preset.friction());
                        }
                        if let Some(friction) = &mut surface.friction_override {
                            ui.add(egui::DragValue::new(friction).speed(0.01).range(0.0..=2.0));
                        } else {
                            ui.label(format!("{:.2} (preset)", surface.preset.friction()));
                        }
                        ui.end_row();

                        // Bounciness / Restitution
                        ui.label("Bounciness");
                        let mut override_bounce = surface.bounciness_override.is_some();
                        if ui
                            .checkbox(&mut override_bounce, "Override")
                            .on_hover_text("Override the preset's restitution / bounciness")
                            .changed()
                        {
                            surface.bounciness_override =
                                override_bounce.then_some(surface.preset.bounciness());
                        }
                        if let Some(bounce) = &mut surface.bounciness_override {
                            ui.add(egui::DragValue::new(bounce).speed(0.01).range(0.0..=1.0));
                        } else {
                            ui.label(format!("{:.2} (preset)", surface.preset.bounciness()));
                        }
                        ui.end_row();
                    });

                ui.add_space(4.0);
                // Impact Decals & Sound Sets (collapsible sub-section)
                egui::CollapsingHeader::new("Impact Decals & Sounds")
                    .id_salt("studio surface reactions")
                    .default_open(false)
                    .show(ui, |ui| {
                        egui::Grid::new("studio surface reactions grid")
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Decal Set");
                                let mut decal = surface.effective_decal_set().to_owned();
                                if ui.text_edit_singleline(&mut decal).changed() {
                                    surface.decal_set_override = if decal
                                        == surface.preset.decal_set()
                                        || decal.is_empty()
                                    {
                                        None
                                    } else {
                                        Some(decal)
                                    };
                                }
                                ui.end_row();

                                ui.label("Sound Set");
                                let mut sound = surface.effective_sound_set().to_owned();
                                if ui.text_edit_singleline(&mut sound).changed() {
                                    surface.sound_set_override = if sound
                                        == surface.preset.sound_set()
                                        || sound.is_empty()
                                    {
                                        None
                                    } else {
                                        Some(sound)
                                    };
                                }
                                ui.end_row();
                            });
                    });

                if !surface.is_default()
                    && ui
                        .small_button("Reset to Default")
                        .on_hover_text("Clear preset and manual overrides")
                        .clicked()
                {
                    surface = SurfaceProperties::default();
                }
            });
            let _ = palette;
        })
        .header_response
        .on_hover_text(
            "Surface property preset (auto-fills mass from volume × density, friction, \
             bounciness, impact decals and sound sets)",
        );

    if surface != original {
        doc.set_surface_properties(surface);
    }
}
