//! Material Slots & Face Material Painting for Model Studio.
//!
//! Painting a face with a material assigns that face's indices to a Material
//! Slot (`face.material = slot`) rather than creating per-face shader
//! instances, so the renderer emits one draw primitive per material slot.
//!
//! Supports:
//! - Material slot badges in the outliner hierarchy (swatch, name, face count)
//! - Assigning selected faces (or painting hovered faces directly in the 3D
//!   viewport when Face Paint is active)
//! - Drag-and-drop of material assets (`.material.json` / `.vmat`) onto a
//!   hierarchy slot (replaces the material for all faces in that slot) or onto
//!   a viewport face (reuses an existing matching slot or appends one slot and
//!   assigns the face(s)).

use axcenity_assets::ModelMaterial;
use eframe::egui;

use super::{Mode, StudioDoc};
use crate::theme::{self, icons};
use crate::widgets::{self, Role};

/// Where a dragged material asset was dropped in Model Studio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialDropTarget {
    /// Dropped onto a hierarchy material slot: replaces that slot's material
    /// for all faces using it.
    Slot { object: usize, slot: usize },
    /// Dropped onto viewport face(s): reuses a matching slot by name or
    /// appends a new slot and assigns those faces to it.
    Faces { object: usize, faces: Vec<u32> },
}

/// Active face-painting & material-slot UI state.
#[derive(Default)]
pub struct MaterialState {
    /// Currently selected material slot index on the active object.
    pub active_slot: usize,
    /// When true in Object/Edit mode, clicking or dragging in the 3D viewport
    /// paints `active_slot` directly onto hovered faces.
    pub face_paint: bool,
    /// Snapshot captured when a face-paint drag stroke begins so the whole
    /// stroke records as a single undo step.
    stroke_before: Option<super::Snapshot>,
    stroke_changed: bool,
}

impl MaterialState {
    pub fn busy(&self) -> bool {
        self.stroke_before.is_some()
    }

    pub fn forget(&mut self) {
        self.active_slot = 0;
        self.face_paint = false;
        self.stroke_before = None;
        self.stroke_changed = false;
    }

    /// Starts a batched face-paint stroke (one undo step).
    pub(super) fn begin_stroke(&mut self, doc: &StudioDoc) {
        if self.stroke_before.is_none() {
            self.stroke_before = Some(doc.snapshot());
            self.stroke_changed = false;
        }
    }

    /// Paints `face` on `doc.active` with `self.active_slot` during an active
    /// stroke.
    pub(super) fn paint_face(&mut self, doc: &mut StudioDoc, face: u32) {
        let Some(object) = doc.model.objects.get_mut(doc.active) else {
            return;
        };
        if self.active_slot >= object.materials.len() {
            return;
        }
        if let Some(mesh) = &mut object.mesh
            && mesh.set_face_material(face, self.active_slot as u32)
        {
            doc.revision += 1;
            self.stroke_changed = true;
        }
    }

    /// Finishes the face-paint stroke and records it as one undo step if any
    /// face changed.
    pub(super) fn end_stroke(&mut self, doc: &mut StudioDoc) {
        if let Some(before) = self.stroke_before.take()
            && self.stroke_changed
        {
            doc.record("Paint Face Material", before);
        }
        self.stroke_changed = false;
    }
}

impl StudioDoc {
    /// Counts how many faces of `object` use each material slot.
    pub fn slot_face_counts(&self, object: usize) -> Vec<usize> {
        let Some(obj) = self.model.objects.get(object) else {
            return Vec::new();
        };
        let mut counts = vec![0usize; obj.materials.len()];
        if let Some(mesh) = &obj.mesh {
            for face in 0..mesh.face_count() as u32 {
                let slot = mesh.face(face).material as usize;
                if let Some(c) = counts.get_mut(slot) {
                    *c += 1;
                }
            }
        }
        counts
    }

    /// Appends a new material slot to `object` as one undo step and returns its
    /// slot index.
    pub fn add_material_slot(
        &mut self,
        object: usize,
        mut material: ModelMaterial,
    ) -> Option<usize> {
        let obj = self.model.objects.get(object)?;
        obj.mesh.as_ref()?;
        let base = if material.name.trim().is_empty() {
            "Material".to_owned()
        } else {
            material.name.clone()
        };
        let mut name = base.clone();
        let mut n = 2usize;
        while obj.materials.iter().any(|m| m.name == name) {
            name = format!("{base}_{n}");
            n += 1;
        }
        material.name = name;
        let before = self.snapshot();
        let slot = self.model.objects[object].materials.len();
        self.model.objects[object].materials.push(material);
        self.record("Add Material Slot", before);
        Some(slot)
    }

    /// Replaces `object`'s material at `slot` with `material` (updating every
    /// face using that slot without creating new draw calls), as one undo step.
    pub fn replace_material_slot(
        &mut self,
        object: usize,
        slot: usize,
        material: ModelMaterial,
    ) -> bool {
        let Some(obj) = self.model.objects.get(object) else {
            return false;
        };
        if slot >= obj.materials.len() {
            return false;
        }
        let before = self.snapshot();
        self.model.objects[object].materials[slot] = material;
        self.record("Replace Material Slot", before);
        true
    }

    /// Assigns `faces` on `object` to material `slot` as one undo step.
    pub fn assign_faces_material(&mut self, object: usize, faces: &[u32], slot: u32) -> bool {
        let Some(obj) = self.model.objects.get(object) else {
            return false;
        };
        if (slot as usize) >= obj.materials.len() || obj.mesh.is_none() || faces.is_empty() {
            return false;
        }
        let before = self.snapshot();
        let mut changed = false;
        if let Some(mesh) = &mut self.model.objects[object].mesh {
            for &face in faces {
                if mesh.set_face_material(face, slot) {
                    changed = true;
                }
            }
        }
        if changed {
            self.record("Assign Material", before);
        }
        changed
    }

    /// Paints `faces` on `object` with `material`: reuses an existing slot
    /// whose name matches `material.name` (avoiding draw-call explosion), or
    /// appends a new slot, then assigns `faces` to that slot as one undo step.
    pub fn paint_faces_with_material(
        &mut self,
        object: usize,
        faces: &[u32],
        material: ModelMaterial,
    ) -> Option<usize> {
        let obj = self.model.objects.get(object)?;
        if obj.mesh.is_none() || faces.is_empty() {
            return None;
        }
        let before = self.snapshot();
        let slot = if let Some(existing) = self.model.objects[object]
            .materials
            .iter()
            .position(|m| m.name == material.name)
        {
            self.model.objects[object].materials[existing] = material;
            existing
        } else {
            let idx = self.model.objects[object].materials.len();
            self.model.objects[object].materials.push(material);
            idx
        };
        if let Some(mesh) = &mut self.model.objects[object].mesh {
            for &face in faces {
                mesh.set_face_material(face, slot as u32);
            }
        }
        self.record("Paint Material onto Faces", before);
        Some(slot)
    }
}

/// Converts a linear/sRGB `[f32; 4]` base color to an `egui::Color32` swatch.
pub(super) fn swatch_color(rgba: [f32; 4]) -> egui::Color32 {
    let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    egui::Color32::from_rgb(c(rgba[0]), c(rgba[1]), c(rgba[2]))
}

/// Material Slots & Face Painting panel (shown in the Materials & LODs tab).
pub(super) fn panel(
    ui: &mut egui::Ui,
    doc: &mut StudioDoc,
    state: &mut MaterialState,
    editable: bool,
) {
    let Some(object) = doc.model.objects.get(doc.active) else {
        return;
    };
    if object.mesh.is_none() || doc.model.is_collision_piece(doc.active) {
        return;
    }
    let palette = theme::palette(ui.ctx());
    let active = doc.active;
    let slot_count = object.materials.len();
    if slot_count > 0 && state.active_slot >= slot_count {
        state.active_slot = slot_count - 1;
    }
    let counts = doc.slot_face_counts(active);

    widgets::section_label(ui, "Material Slots & Face Paint");

    // Face Paint toggle
    ui.add_enabled_ui(editable && slot_count > 0, |ui| {
        ui.horizontal(|ui| {
            if widgets::icon_button(
                ui,
                icons::PAINT_BUCKET,
                "Face Paint: click or drag on faces in the 3D viewport to assign the active material slot",
                state.face_paint,
            )
            .clicked()
            {
                state.face_paint = !state.face_paint;
            }
            ui.label(if state.face_paint {
                egui::RichText::new("Face Paint Active (click/drag faces)").color(palette.accent)
            } else {
                egui::RichText::new("Face Paint Brush")
            });
        });
    });

    ui.add_space(4.0);
    let mut assign_slot: Option<usize> = None;
    let mut add_slot = false;
    let mut edited_mat: Option<(usize, ModelMaterial)> = None;

    for slot in 0..slot_count {
        let mat = doc.model.objects[active].materials[slot].clone();
        let faces = counts.get(slot).copied().unwrap_or(0);
        let selected = state.active_slot == slot;
        ui.horizontal(|ui| {
            let (swatch_rect, _) =
                ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(swatch_rect, 2.0, swatch_color(mat.base_color));
            ui.painter().rect_stroke(
                swatch_rect,
                2.0,
                egui::Stroke::new(1.0, palette.border),
                egui::StrokeKind::Inside,
            );
            let label = format!("{} ({faces} faces)", mat.name);
            if ui.selectable_label(selected, label).clicked() {
                state.active_slot = slot;
            }
            if editable && doc.mode == Mode::Edit && !doc.selection.faces.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("Assign")
                        .on_hover_text("Assign selected faces to this material slot")
                        .clicked()
                    {
                        assign_slot = Some(slot);
                    }
                });
            }
        });
    }

    // Active slot quick properties (Base Color, Roughness, Metalness)
    if let Some(mat) = doc.model.objects[active].materials.get(state.active_slot) {
        let mut edited = mat.clone();
        let mut changed = false;
        ui.add_space(4.0);
        ui.add_enabled_ui(editable, |ui| {
            egui::Grid::new("studio material slot edit")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Color");
                    let mut rgb = [
                        edited.base_color[0],
                        edited.base_color[1],
                        edited.base_color[2],
                    ];
                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                        edited.base_color[0] = rgb[0];
                        edited.base_color[1] = rgb[1];
                        edited.base_color[2] = rgb[2];
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Roughness");
                    if ui
                        .add(
                            egui::DragValue::new(&mut edited.roughness)
                                .speed(0.01)
                                .range(0.0..=1.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Metalness");
                    if ui
                        .add(
                            egui::DragValue::new(&mut edited.metalness)
                                .speed(0.01)
                                .range(0.0..=1.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();
                });
        });
        if changed {
            edited_mat = Some((state.active_slot, edited));
        }
    }

    ui.add_space(4.0);
    if ui
        .add_enabled(
            editable,
            widgets::button(
                Role::Secondary,
                Some(icons::PLUS),
                "Add Material Slot",
                &palette,
            ),
        )
        .on_hover_text(
            "Add a new material slot to this mesh (no extra draw call until faces use it)",
        )
        .clicked()
    {
        add_slot = true;
    }

    if let Some(slot) = assign_slot {
        let faces: Vec<u32> = doc.selection.faces.iter().copied().collect();
        doc.assign_faces_material(active, &faces, slot as u32);
    }
    if let Some((slot, mat)) = edited_mat {
        doc.replace_material_slot(active, slot, mat);
    }
    if add_slot {
        let palette_colors = [
            [0.75, 0.35, 0.30, 1.0],
            [0.30, 0.60, 0.78, 1.0],
            [0.35, 0.70, 0.42, 1.0],
            [0.82, 0.68, 0.28, 1.0],
            [0.62, 0.42, 0.74, 1.0],
        ];
        let color = palette_colors[slot_count % palette_colors.len()];
        let mat = ModelMaterial {
            name: format!("Material_{}", slot_count + 1),
            base_color: color,
            roughness: 0.5,
            ..ModelMaterial::default()
        };
        if let Some(slot) = doc.add_material_slot(active, mat) {
            state.active_slot = slot;
        }
    }
}
