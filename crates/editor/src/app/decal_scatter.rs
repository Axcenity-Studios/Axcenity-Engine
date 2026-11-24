//! The Scatter pane's decal brush (`scatter_pane.rs` is the panel,
//! `procedural::decal_scatter` the layout): with Paint decals armed, a
//! drag lays the palette's pictures on the ground as ProjectedDecals in a
//! "Scattered Decals" Folder in the Workspace, and Shift+drag takes the
//! ones in that Folder away. One stroke is one undo step, named by what it
//! did ("Scatter 12 Decals", "Erase 3 Decals").
//!
//! The brush follows the terrain under the mouse, through parts, so it
//! reaches under a vehicle or a porch. Each decal lies on that terrain,
//! or, with Land on parts, on the first terrain or colliding part under a
//! point 20 studs above it. The renderer draws only the nearest projected
//! decals (`docs/rendering/decals.md`), so the brush has a budget
//! (`ScatterPane::decal_budget`) and says in Output what it left out.

use axcenity_core::{ClassName, InstanceId, TerrainData, Value, Vector3, World};
use axcenity_procedural::Surfaces;
use axcenity_procedural::decal_scatter::{DecalBrush, DecalItem, PlacedDecal, stamp};
use axcenity_render::glam::{Vec2, Vec3};
use axcenity_terrain::Placement;
use axcenity_terrain::brush::Stroke;

use crate::app::EditorApp;
use crate::scatter_pane::{DecalStroke, decal_name};
use crate::viewport::TerrainPointer;

/// The Folder in the Workspace the brush's decals go in, and Shift+drag
/// takes them from.
pub(in crate::app) const DECAL_FOLDER: &str = "Scattered Decals";
/// The distance between a stroke's stamps, as a share of the brush's radius.
const STAMP_SPACING: f32 = 0.5;

/// Where every ProjectedDecal in `world` is.
fn decal_positions(world: &World) -> Vec<Vec3> {
    world
        .all_below(world.root(), |class| class == ClassName::ProjectedDecal)
        .into_iter()
        .filter_map(|id| position(world, id))
        .collect()
}

fn position(world: &World, id: InstanceId) -> Option<Vec3> {
    let at = world.get(id)?.property("Position")?.as_vector3()?;
    Some(Vec3::new(at.x, at.y, at.z))
}

/// The Workspace's Folders named [`DECAL_FOLDER`].
fn decal_folders(world: &World) -> Vec<InstanceId> {
    let Some(workspace) = world.get(world.workspace()) else {
        return Vec::new();
    };
    workspace
        .children()
        .iter()
        .copied()
        .filter(|&child| {
            world.get(child).is_some_and(|instance| {
                instance.class() == ClassName::Folder && instance.name() == DECAL_FOLDER
            })
        })
        .collect()
}

/// Makes `decal` as a ProjectedDecal in `folder`.
fn make_decal(world: &mut World, folder: InstanceId, decal: &PlacedDecal) -> Option<InstanceId> {
    let id = world.create(ClassName::ProjectedDecal).ok()?;
    let vector = |v: Vec3| Value::Vector3(Vector3::new(v.x, v.y, v.z));
    let _ = world.set_name(id, decal_name(&decal.texture));
    for (name, value) in [
        ("Texture", Value::String(decal.texture.clone())),
        ("Position", vector(decal.position)),
        ("Orientation", Value::Vector3(decal.orientation)),
        ("Size", vector(decal.size)),
        ("Transparency", Value::Number(f64::from(decal.transparency))),
    ] {
        let _ = world.set_property(id, name, value);
    }
    world.set_parent(id, Some(folder)).ok()?;
    Some(id)
}

impl EditorApp {
    /// Adds the picture at `path` (clicked in Assets) to the palette's
    /// decals, with its shape read from the file.
    pub(in crate::app) fn add_scatter_decal(&mut self, path: &str) {
        self.refresh_scatter_palettes();
        let aspect = self
            .project
            .as_ref()
            .and_then(|project| image::image_dimensions(project.folder.join(path)).ok())
            .filter(|&(width, height)| width > 0 && height > 0)
            .map_or(1.0, |(width, height)| width as f32 / height as f32);
        if !self.scatter.add_decal(path, aspect) {
            self.output.info(format!(
                "{} is already one of this palette's decals.",
                decal_name(path)
            ));
        }
    }

    /// Whether the Assets panel adds the pictures clicked to the palette's
    /// decals.
    pub(in crate::app) fn scatter_decal_picking(&self) -> bool {
        self.scatter.picking_decals
            && !self.playing()
            && self.layout.is_open(crate::app::Pane::Scatter)
    }

    /// The mouse over the terrain while Paint decals is armed: starts,
    /// continues and ends strokes. Shift takes scattered decals away.
    pub(in crate::app) fn decal_pointer(&mut self, pointer: TerrainPointer) {
        if self.playing() || self.scatter_terrain().is_none() {
            return;
        }
        if pointer.pressed
            && self.scatter.decal_stroke.is_none()
            && let Some(point) = pointer.point
            && let Some(palette) = self.scatter.palette()
        {
            let brush = DecalBrush {
                items: palette
                    .decals
                    .iter()
                    .filter(|decal| decal.places())
                    .map(|decal| DecalItem {
                        texture: decal.texture.clone(),
                        weight: decal.weight,
                        min_size: decal.min_size,
                        max_size: decal.max_size,
                        aspect: decal.aspect,
                        min_opacity: decal.min_opacity,
                        max_opacity: decal.max_opacity,
                        rotation: decal.rotation,
                    })
                    .collect(),
                radius: self.scatter.radius,
                falloff: super::FALLOFF,
                // The pane's density is per 100 square studs.
                density: palette.decal_density / 100.0,
                spacing: palette.decal_spacing,
                seed: self.scatter.seed,
            };
            self.scatter.seed = self.scatter.seed.wrapping_add(1);
            let erase = pointer.modifiers.shift;
            self.scatter.decal_stroke = Some(DecalStroke {
                before: self.world.clone(),
                stroke: Stroke::new(),
                brush,
                erase,
                existing: decal_positions(&self.world),
                surfaces: (self.scatter.decals_on_parts && !erase)
                    .then(|| Surfaces::of(&self.world, &self.assets)),
                placed: 0,
                erased: 0,
                over_budget: 0,
            });
            self.decal_stroke_to(point, pointer.seconds);
        } else if pointer.down
            && let Some(point) = pointer.point
        {
            self.decal_stroke_to(point, pointer.seconds);
        }
        if pointer.released || !pointer.down {
            self.finish_decal_stroke();
        }
    }

    /// Moves the stroke under way to `point`, stamping along the way.
    fn decal_stroke_to(&mut self, point: Vec3, seconds: f32) {
        let Some(mut active) = self.scatter.decal_stroke.take() else {
            return;
        };
        let spacing = (active.brush.radius * STAMP_SPACING).max(0.25);
        for center in active.stroke.advance(point, spacing, seconds, None) {
            if active.erase {
                self.erase_decals(&mut active, center);
            } else {
                self.stamp_decals(&mut active, center);
            }
        }
        self.scatter.decal_stroke = Some(active);
    }

    /// One stamp: lays the brush's decals round `center`.
    fn stamp_decals(&mut self, active: &mut DecalStroke, center: Vec3) {
        let Some(terrain) = self.scatter_terrain() else {
            return;
        };
        let (Some(placement), Some(data)) = (
            self.world.get(terrain).and_then(Placement::of),
            self.world.terrain(terrain).cloned(),
        ) else {
            return;
        };
        let budget = self.scatter.decal_budget();
        let surfaces = &active.surfaces;
        let stamped = stamp(
            &active.brush,
            center,
            &mut active.existing,
            budget,
            |spot| lie_on(&data, &placement, surfaces.as_ref(), center, spot),
        );
        active.over_budget += stamped.over_budget;
        if stamped.placed.is_empty() {
            return;
        }
        let folder = match decal_folders(&self.world).first() {
            Some(&folder) => folder,
            None => {
                let Ok(folder) = self.world.create(ClassName::Folder) else {
                    return;
                };
                let workspace = self.world.workspace();
                let _ = self.world.set_name(folder, DECAL_FOLDER.to_owned());
                if self.world.set_parent(folder, Some(workspace)).is_err() {
                    return;
                }
                folder
            }
        };
        for decal in &stamped.placed {
            if make_decal(&mut self.world, folder, decal).is_some() {
                active.placed += 1;
            }
        }
    }

    /// One erasing stamp: takes away the scattered decals whose middles
    /// are under the brush at `center`, seen from above.
    fn erase_decals(&mut self, active: &mut DecalStroke, center: Vec3) {
        let radius = active.brush.radius;
        let gone: Vec<InstanceId> = decal_folders(&self.world)
            .into_iter()
            .flat_map(|folder| {
                self.world
                    .get(folder)
                    .map(|instance| instance.children().to_vec())
                    .unwrap_or_default()
            })
            .filter(|&child| {
                self.world
                    .get(child)
                    .is_some_and(|instance| instance.class() == ClassName::ProjectedDecal)
                    && position(&self.world, child).is_some_and(|at| {
                        Vec2::new(at.x - center.x, at.z - center.z).length() <= radius
                    })
            })
            .collect();
        for id in gone {
            if self.world.destroy(id).is_ok() {
                active.erased += 1;
            }
        }
    }

    /// Ends the decal stroke under way, as one undo step if it changed
    /// anything, named by how many decals it laid or took.
    pub(in crate::app) fn finish_decal_stroke(&mut self) {
        let Some(active) = self.scatter.decal_stroke.take() else {
            return;
        };
        if active.over_budget > 0 {
            let budget = self.scatter.decal_budget();
            let advice = if self.scatter.decals_for_medium {
                " Untick the Limit, For Medium quality, to lay up to 1,024 within 600 studs \
                 (Ultra's), or lay fewer, larger decals."
            } else {
                " That's the most any quality draws: lay fewer, larger decals."
            };
            self.output.warning(format!(
                "Left out {} decals: {} projected decals already lie within {} studs there, \
                 the most the renderer would draw.{advice}",
                active.over_budget, budget.count, budget.distance
            ));
        }
        let count = |n: usize, verb: &str| match n {
            1 => format!("{verb} 1 Decal"),
            n => format!("{verb} {n} Decals"),
        };
        let label = match (active.placed, active.erased) {
            (0, 0) => return,
            (0, erased) => count(erased, "Erase"),
            (placed, _) => count(placed, "Scatter"),
        };
        self.history
            .record_world(&mut self.world, label, active.before);
    }
}

/// Where a decal at `spot` (world X, Z) lies and which way the surface
/// faces: on the terrain, or with `surfaces` on the first terrain or
/// colliding part under a point 20 studs above the terrain there (above
/// the brush's middle `center`, off the terrain).
fn lie_on(
    data: &TerrainData,
    placement: &Placement,
    surfaces: Option<&Surfaces>,
    center: Vec3,
    spot: Vec2,
) -> Option<(Vec3, Vec3)> {
    let ground = placement.ground_y(data, spot.x, spot.y);
    if let Some(surfaces) = surfaces {
        return surfaces.below(Vec3::new(spot.x, ground.unwrap_or(center.y), spot.y));
    }
    let y = ground?;
    let grid = placement.to_grid(data, spot.x, spot.y).round();
    let (size_x, size_z) = data.size();
    let normal = placement.normal(
        data,
        (grid.x.max(0.0) as usize).min(size_x - 1),
        (grid.y.max(0.0) as usize).min(size_z - 1),
    );
    Some((Vec3::new(spot.x, y, spot.y), normal))
}
