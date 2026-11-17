//! Group as Mesh and Ungroup Mesh (Model ▸ Solid Modeling): the selected
//! parts (MeshParts, Parts, UnionOperations, and those in selected Models)
//! made into one MeshPart whose model is one optimized `.glb`
//! (`mesh_edit::group`), and back.
//!
//! The selection's roots (the selected parts and Models, structure and all)
//! go into a GroupedParts inside the new MeshPart: out of the world
//! (`core::csg::in_union`), listed dimmed in the Explorer, never moved by
//! the move tools. The GroupedParts remembers where the MeshPart was
//! (`Origin`, `OriginSize`), so Ungroup Mesh puts them back moved, turned
//! and stretched as the MeshPart has been since. Each is one undo step
//! (`History::record_world`); the `.glb` stays when it's undone, like a
//! union's.

use axcenity_core::{CFrame, ClassName, InstanceId, Value, Vector3, World};
use axcenity_render::glam::{EulerRot, Quat, Vec3};

use super::EditorApp;
use crate::history::{self, Place};
use crate::tree;

/// The properties the grouped MeshPart takes from the first part.
const FROM_FIRST: [&str; 8] = [
    "Anchored",
    "CastShadow",
    "CanCollide",
    "CanTouch",
    "CanQuery",
    "CollisionGroup",
    "CustomPhysicalProperties",
    "Massless",
];

impl EditorApp {
    /// The parts Group as Mesh would group: the selected parts and the
    /// parts in selected Models and Folders, leaving out negated parts and
    /// parts already kept out of the world.
    pub(in crate::app) fn group_parts(&self) -> Vec<InstanceId> {
        let mut parts = Vec::new();
        for root in self.group_roots() {
            for id in std::iter::once(root).chain(self.world.descendants(root)) {
                let groupable = self.world.get(id).is_some_and(|instance| {
                    instance.class().is_base_part()
                        && instance.class() != ClassName::NegateOperation
                });
                if groupable
                    && !axcenity_core::csg::out_of_world(&self.world, id)
                    && !parts.contains(&id)
                {
                    parts.push(id);
                }
            }
        }
        parts
    }

    /// The selected Instances that aren't inside another selected one and
    /// are parts or hold parts, in the order they were selected.
    fn group_roots(&self) -> Vec<InstanceId> {
        self.selection_roots()
            .into_iter()
            .filter(|&id| {
                self.world.get(id).is_some_and(|instance| {
                    let class = instance.class();
                    (class.is_base_part() || matches!(class, ClassName::Model | ClassName::Folder))
                        && instance.parent().is_some()
                        && !axcenity_core::csg::out_of_world(&self.world, id)
                        && !self.world.is_locked(id)
                })
            })
            .collect()
    }

    /// The selected MeshParts made by Group as Mesh that still have their
    /// parts.
    pub(in crate::app) fn grouped_meshes(&self) -> Vec<InstanceId> {
        self.selection_roots()
            .into_iter()
            .filter(|&id| axcenity_core::csg::grouped_parts(&self.world, id).is_some())
            .collect()
    }

    /// Groups the selected parts into one MeshPart.
    pub(in crate::app) fn group_as_mesh(&mut self) {
        let roots = self.group_roots();
        let parts = self.group_parts();
        if parts.len() < 2 {
            self.output
                .warning("Select two or more parts, or a Model with them, to group as a mesh.");
            return;
        }
        let Some(folder) = self.project.as_ref().map(|project| project.folder.clone()) else {
            self.output
                .warning("Open or save a project first: the grouped mesh is saved in it.");
            return;
        };
        let mut sources = Vec::with_capacity(parts.len());
        let mut notes = Vec::new();
        for &part in &parts {
            match axcenity_mesh_edit::group_source(&self.world, &self.assets, part) {
                Ok((mut source, more)) => {
                    precise_collision(&self.world, &self.assets, part, &mut source);
                    sources.push(source);
                    notes.extend(more);
                }
                Err(problem) => {
                    self.output.warning(format!(
                        "Could not group the parts: {problem}. Nothing was changed."
                    ));
                    return;
                }
            }
        }
        let grouped = match axcenity_mesh_edit::group(&sources) {
            Ok(grouped) => grouped,
            Err(problem) => {
                self.output.warning(format!(
                    "Could not group the parts: {problem}. Nothing was changed."
                ));
                return;
            }
        };
        let Some(place) = roots
            .first()
            .and_then(|&root| history::place_of(&self.world, root))
        else {
            return;
        };
        // One Model grouped is named after it (it moves out of the way);
        // anything else is a Group.
        let name = match roots.as_slice() {
            [only] if self.world.get(*only).map(|i| i.class()) == Some(ClassName::Model) => self
                .world
                .get(*only)
                .map_or("Group".to_owned(), |i| i.name().to_owned()),
            _ => tree::unique_name(&self.world, place.parent, "Group"),
        };
        let path = match axcenity_mesh_edit::save_group(&folder, &self.assets, &name, &grouped) {
            Ok(path) => path,
            Err(problem) => {
                self.output.error(problem);
                return;
            }
        };

        let before = self.world.clone();
        let mesh = self
            .world
            .create(ClassName::MeshPart)
            .expect("a MeshPart can be made");
        let _ = self.world.set_name(mesh, name);
        let vector = |v: Vec3| Value::Vector3(Vector3::new(v.x, v.y, v.z));
        let orientation = grouped.orientation();
        let mut properties = vec![
            ("MeshId", Value::String(path.clone())),
            ("Position", vector(grouped.position)),
            ("Orientation", Value::Vector3(orientation)),
            ("Size", vector(grouped.size)),
        ];
        if let Some(first) = self.world.get(parts[0]) {
            for name in FROM_FIRST {
                if let Some(value) = first.property(name) {
                    properties.push((name, value.clone()));
                }
            }
        }
        for (name, value) in properties {
            let _ = self.world.set_property(mesh, name, value);
        }
        // Where the MeshPart is now, read back as stored, so Ungroup Mesh
        // finds it unmoved exactly.
        let kept = self
            .world
            .create(ClassName::GroupedParts)
            .expect("a GroupedParts can be made");
        let (position, rotation, size) = placement(&self.world, mesh);
        let origin = CFrame {
            position: Vector3::new(position.x, position.y, position.z),
            rotation: rotation.to_array(),
        };
        let _ = self
            .world
            .set_property(kept, "Origin", Value::CFrame(origin));
        let _ = self.world.set_property(kept, "OriginSize", vector(size));
        let _ = history::put(&mut self.world, mesh, Some(place));
        let _ = self.world.set_parent(kept, Some(mesh));
        for &root in &roots {
            let _ = self.world.set_parent(root, Some(kept));
        }
        let label = format!("Group {} parts as a mesh", parts.len());
        self.history.record_world(&mut self.world, label, before);
        self.select_from_viewport(Some(mesh));
        self.output
            .info(format!("{} Saved in {path}.", grouped.stats.summary()));
        for note in notes {
            self.output.warning(note);
        }
    }

    /// Puts each selected grouped MeshPart's parts back where it is, and
    /// takes it out.
    pub(in crate::app) fn ungroup_mesh(&mut self) {
        let meshes = self.grouped_meshes();
        if meshes.is_empty() {
            self.output
                .warning("Select a MeshPart made by Group as Mesh to ungroup.");
            return;
        }
        let before = self.world.clone();
        let mut restored = Vec::new();
        for mesh in meshes {
            let (Some(place), Some(kept)) = (
                history::place_of(&self.world, mesh),
                axcenity_core::csg::grouped_parts(&self.world, mesh),
            ) else {
                continue;
            };
            let moved = Moved::new(&self.world, mesh, kept);
            if let Some(moved) = moved {
                for id in self.world.descendants(kept) {
                    moved.apply(&mut self.world, id);
                }
            }
            let inside = self
                .world
                .get(kept)
                .map(|instance| instance.children().to_vec())
                .unwrap_or_default();
            for (offset, child) in inside.into_iter().enumerate() {
                let at = Place {
                    parent: place.parent,
                    index: place.index + 1 + offset,
                };
                if history::put(&mut self.world, child, Some(at)).is_ok() {
                    restored.push(child);
                }
            }
            let _ = self.world.set_parent(mesh, None);
        }
        let label = format!("Ungroup mesh into {} parts", restored.len());
        self.history.record_world(&mut self.world, label, before);
        self.select_many(restored);
    }
}

/// A UnionOperation that collides with PreciseConvexDecomposition keeps
/// its convex pieces, so a grouped doorway can still be walked through.
fn precise_collision(
    world: &World,
    assets: &axcenity_assets::Assets,
    part: InstanceId,
    source: &mut axcenity_mesh_edit::GroupSource,
) {
    let Some(instance) = world.get(part) else {
        return;
    };
    let precise = matches!(
        instance.property("CollisionFidelity"),
        Some(Value::EnumItem(item)) if item.item == "PreciseConvexDecomposition"
    );
    let collides = instance.property("CanCollide").and_then(Value::as_bool) != Some(false);
    if !precise || !collides {
        return;
    }
    let mesh_id = instance
        .property("MeshId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let (Some(model), Some(size)) = (
        assets.model_blocking(mesh_id),
        instance.property("Size").and_then(Value::as_vector3),
    ) else {
        return;
    };
    source.collision = axcenity_physics::precise_pieces(&model, size)
        .into_iter()
        .map(|piece| piece.into_iter().map(Vec3::from).collect())
        .collect();
}

/// A part's `Position`, rotation (from `Orientation`) and `Size`.
fn placement(world: &World, id: InstanceId) -> (Vec3, Quat, Vec3) {
    let vector = |name| {
        world
            .get(id)
            .and_then(|instance| instance.property(name))
            .and_then(Value::as_vector3)
            .map_or(Vec3::ZERO, |v| Vec3::new(v.x, v.y, v.z))
    };
    (
        vector("Position"),
        rotation_of(vector("Orientation")),
        vector("Size"),
    )
}

/// An `Orientation` (degrees: Z, then X, then Y) as a rotation.
fn rotation_of(orientation: Vec3) -> Quat {
    let [x, y, z] = orientation.to_array().map(f32::to_radians);
    Quat::from_euler(EulerRot::YXZ, y, x, z)
}

/// A rotation as an `Orientation`, rounded to 3 decimals as the editor
/// shows it.
fn orientation_of(rotation: Quat) -> Vector3 {
    let (y, x, z) = rotation.to_euler(EulerRot::YXZ);
    let tidy = |radians: f32| (radians.to_degrees() * 1000.0).round() / 1000.0 + 0.0;
    Vector3::new(tidy(x), tidy(y), tidy(z))
}

/// How a grouped MeshPart has moved, turned and been stretched since it
/// was grouped: what Ungroup Mesh does to its parts.
struct Moved {
    origin: Vec3,
    unturn: Quat,
    position: Vec3,
    rotation: Quat,
    stretch: Vec3,
}

impl Moved {
    /// `None` when the MeshPart `mesh` is just where its GroupedParts
    /// `kept` says it was: the parts go back exactly as they were.
    fn new(world: &World, mesh: InstanceId, kept: InstanceId) -> Option<Moved> {
        let instance = world.get(kept)?;
        let origin = match instance.property("Origin") {
            Some(Value::CFrame(frame)) => *frame,
            _ => CFrame::IDENTITY,
        };
        let origin_size = instance
            .property("OriginSize")
            .and_then(Value::as_vector3)
            .map_or(Vec3::ONE, |v| Vec3::new(v.x, v.y, v.z));
        let (position, rotation, size) = placement(world, mesh);
        let origin_position = Vec3::new(origin.position.x, origin.position.y, origin.position.z);
        let origin_rotation = Quat::from_array(origin.rotation);
        if position == origin_position && rotation == origin_rotation && size == origin_size {
            return None;
        }
        let origin_rotation = origin_rotation.normalize();
        let stretch = Vec3::select(origin_size.cmpgt(Vec3::ZERO), size / origin_size, Vec3::ONE);
        Some(Moved {
            origin: origin_position,
            unturn: origin_rotation.inverse(),
            position,
            rotation,
            stretch,
        })
    }

    /// Moves, turns and stretches `id` (a part, or a light standing free)
    /// with the MeshPart.
    fn apply(&self, world: &mut World, id: InstanceId) {
        let Some(instance) = world.get(id) else {
            return;
        };
        let class = instance.class();
        let free = class.can_stand_free()
            && instance
                .parent()
                .and_then(|parent| world.get(parent))
                .is_none_or(|parent| !parent.class().is_base_part());
        if !class.is_base_part() && !free {
            return;
        }
        let vector = |name| {
            instance
                .property(name)
                .and_then(Value::as_vector3)
                .map(|v| Vec3::new(v.x, v.y, v.z))
        };
        let (Some(position), Some(orientation)) = (vector("Position"), vector("Orientation"))
        else {
            return;
        };
        let size = vector("Size");
        let turn = rotation_of(orientation);
        let local = self.unturn * (position - self.origin);
        let new_position = self.position + self.rotation * (local * self.stretch);
        let new_turn = self.rotation * self.unturn * turn;
        // Each of the part's own axes stretched as the group was along it.
        let own = self.unturn * turn;
        let new_size = size.map(|size| {
            Vec3::new(
                size.x * (own * Vec3::X * self.stretch).length(),
                size.y * (own * Vec3::Y * self.stretch).length(),
                size.z * (own * Vec3::Z * self.stretch).length(),
            )
        });
        let vector = |v: Vec3| Value::Vector3(Vector3::new(v.x, v.y, v.z));
        let _ = world.set_property(id, "Position", vector(new_position));
        let _ = world.set_property(id, "Orientation", Value::Vector3(orientation_of(new_turn)));
        if let Some(size) = new_size.filter(|_| self.stretch != Vec3::ONE) {
            let _ = world.set_property(id, "Size", vector(size));
        }
    }
}
