//! One-Click "Game Ready" validation bar for Model Studio.
//!
//! Checks the open model before saving to the level editor:
//! - Collision hulls (`_COL` pieces present for visual meshes)
//! - LODs (`_LOD1` / `_LOD2` levels present)
//! - Mesh normals (no degenerate zero-length face normals)
//! - Skin weights (when rigged, all weighted vertices sum to ~1.0)
//!
//! Renders a compact status bar at the bottom of Model Studio with one-click
//! quick-fix actions for any flagged issue.

use axcenity_mesh_edit::MeshOp;
use eframe::egui;

use super::collision::CollisionShape;
use super::rig::WeightAction;
use super::{ModelStudio, StudioDoc};
use crate::theme::{self, icons};

/// Summary of game-readiness checks for the active object / model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameReadyReport {
    /// Number of `_COL` pieces on the active visual mesh (or total across visual meshes).
    pub collision_pieces: usize,
    /// Total LOD levels on the active visual mesh (1 = LOD 0 only; 3 = LOD 0, 1, 2).
    pub lod_levels: usize,
    /// Number of faces with degenerate (near-zero) normals.
    pub bad_normals: usize,
    /// True if the model has an armature and the active object has skin weights.
    pub is_rigged: bool,
    /// Number of skinned vertices whose total weight deviates from 1.0 by > 0.02.
    pub unnormalized_weights: usize,
}

impl GameReadyReport {
    pub fn collision_ok(&self) -> bool {
        self.collision_pieces > 0
    }

    pub fn lods_ok(&self) -> bool {
        self.lod_levels >= 2
    }

    pub fn normals_ok(&self) -> bool {
        self.bad_normals == 0
    }

    pub fn weights_ok(&self) -> bool {
        !self.is_rigged || self.unnormalized_weights == 0
    }

    pub fn issue_count(&self) -> usize {
        usize::from(!self.collision_ok())
            + usize::from(!self.lods_ok())
            + usize::from(!self.normals_ok())
            + usize::from(!self.weights_ok())
    }

    pub fn all_ok(&self) -> bool {
        self.issue_count() == 0
    }
}

impl StudioDoc {
    /// Runs lightweight game-readiness checks on the active visual mesh (or
    /// first visual mesh if a collision piece / LOD copy is selected).
    pub fn validate_game_ready(&self) -> GameReadyReport {
        let target = if self
            .model
            .objects
            .get(self.active)
            .is_some_and(|o| o.mesh.is_some())
            && !self.model.is_lod_copy(self.active)
            && !self.model.is_collision_piece(self.active)
        {
            self.active
        } else {
            self.model
                .objects
                .iter()
                .enumerate()
                .find(|&(k, o)| {
                    o.mesh.is_some()
                        && !self.model.is_lod_copy(k)
                        && !self.model.is_collision_piece(k)
                })
                .map_or(self.active, |(k, _)| k)
        };
        let Some(object) = self.model.objects.get(target) else {
            return GameReadyReport::default();
        };
        let collision_pieces = self.model.collision_pieces(target).len();
        let lod_levels = self.model.lod_copies(target).len() + 1;
        let bad_normals = object.mesh.as_ref().map_or(0, |mesh| {
            (0..mesh.face_count() as u32)
                .filter(|&f| mesh.face_normal(f).length_squared() < 1e-6)
                .count()
        });
        let is_rigged = self.model.armature.is_some() && object.skin.is_some();
        let unnormalized_weights = if is_rigged {
            object.skin.as_ref().map_or(0, |skin| {
                skin.weights
                    .iter()
                    .filter(|list| {
                        let sum: f32 = list.iter().map(|&(_, w)| w).sum();
                        !list.is_empty() && (sum - 1.0).abs() > 0.02
                    })
                    .count()
            })
        } else {
            0
        };
        GameReadyReport {
            collision_pieces,
            lod_levels,
            bad_normals,
            is_rigged,
            unnormalized_weights,
        }
    }
}

impl ModelStudio {
    /// Compact bottom status bar showing Game Ready validation pills and
    /// one-click fix actions.
    pub(super) fn validation_bar(&mut self, ui: &mut egui::Ui, editable: bool) {
        let Some(doc) = &self.doc else {
            return;
        };
        let report = doc.validate_game_ready();
        let palette = theme::palette(ui.ctx());
        let ok_color = egui::Color32::from_rgb(64, 217, 102);
        let warn_color = palette.warning;

        let mut fix_collision = false;
        let mut fix_lods = false;
        let mut fix_normals = false;
        let mut fix_weights = false;

        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            // Overall status pill
            if report.all_ok() {
                ui.colored_label(
                    ok_color,
                    theme::with_icon(icons::CHECK_CIRCLE, "Game Ready"),
                );
            } else {
                let n = report.issue_count();
                ui.colored_label(
                    warn_color,
                    theme::with_icon(
                        icons::WARNING,
                        &format!("{n} check{} to review", if n == 1 { "" } else { "s" }),
                    ),
                );
            }

            // Collision chip
            if report.collision_ok() {
                ui.weak(format!("✓ Valid Collision ({})", report.collision_pieces));
            } else if ui
                .add_enabled(
                    editable,
                    egui::Button::new(egui::RichText::new("⚠ Missing Collision").color(warn_color))
                        .frame(false),
                )
                .on_hover_text("Click to auto-generate a Convex Hull collision mesh")
                .clicked()
            {
                fix_collision = true;
            }

            // LODs chip
            if report.lods_ok() {
                ui.weak(format!("✓ {} LODs", report.lod_levels));
            } else if ui
                .add_enabled(
                    editable,
                    egui::Button::new(egui::RichText::new("⚠ 1 LOD only").color(warn_color))
                        .frame(false),
                )
                .on_hover_text("Click to generate LOD 1 and LOD 2 copies")
                .clicked()
            {
                fix_lods = true;
            }

            // Normals chip
            if report.normals_ok() {
                ui.weak("✓ Normals OK");
            } else if ui
                .add_enabled(
                    editable,
                    egui::Button::new(
                        egui::RichText::new(format!("⚠ {} Bad Normals", report.bad_normals))
                            .color(warn_color),
                    )
                    .frame(false),
                )
                .on_hover_text("Click to recalculate outward normals")
                .clicked()
            {
                fix_normals = true;
            }

            // Weights chip (only when rigged)
            if report.is_rigged {
                if report.weights_ok() {
                    ui.weak("✓ Weights OK");
                } else if ui
                    .add_enabled(
                        editable,
                        egui::Button::new(
                            egui::RichText::new(format!(
                                "⚠ {} Un-normalized Weights",
                                report.unnormalized_weights
                            ))
                            .color(warn_color),
                        )
                        .frame(false),
                    )
                    .on_hover_text("Click to normalize all skin weights to sum to 1.0")
                    .clicked()
                {
                    fix_weights = true;
                }
            }
        });

        if fix_collision
            && let Some(doc) = &mut self.doc
            && let Err(why) = doc.generate_collision(CollisionShape::Hull)
        {
            self.notices.push(why);
        }
        if fix_lods
            && let Some(doc) = &mut self.doc
            && let Err(why) = doc.generate_lods()
        {
            self.notices.push(why);
        }
        if fix_normals
            && let Some(doc) = &mut self.doc
            && let Err(why) = doc.run(MeshOp::RecalculateNormals { inside: false })
        {
            self.notices.push(why);
        }
        if fix_weights {
            self.weight_action(WeightAction::Normalize);
        }
    }
}
