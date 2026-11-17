//! Parts kept out of the world for the way back: the parts a
//! UnionOperation was made of (Phase 50), kept inside it for Separate, and
//! the parts a grouped MeshPart was made of, kept in its GroupedParts for
//! Ungroup Mesh. They're out of the world: not drawn, not colliding and
//! not hit by queries. The union's or MeshPart's own model stands for them.

use crate::{ClassName, InstanceId, World};

/// True when `id` is inside a UnionOperation (not the union itself) or a
/// GroupedParts, so it's one of the parts kept for Separate or Ungroup
/// Mesh.
pub fn in_union(world: &World, id: InstanceId) -> bool {
    inside(world, id, keeps_parts)
}

/// True when `id` is inside a GroupedParts: one of the parts a grouped
/// MeshPart was made of.
pub fn in_group(world: &World, id: InstanceId) -> bool {
    inside(world, id, |class| class == ClassName::GroupedParts)
}

/// True when an ancestor of `id` is of a class `kind` accepts.
fn inside(world: &World, id: InstanceId, kind: impl Fn(ClassName) -> bool) -> bool {
    let mut at = world.get(id).and_then(|instance| instance.parent());
    while let Some(current) = at {
        let Some(instance) = world.get(current) else {
            return false;
        };
        if kind(instance.class()) {
            return true;
        }
        at = instance.parent();
    }
    false
}

/// A UnionOperation or a GroupedParts: what's inside it is out of the
/// world.
pub fn keeps_parts(class: ClassName) -> bool {
    matches!(class, ClassName::UnionOperation | ClassName::GroupedParts)
}

/// The GroupedParts inside the MeshPart `id`, if it was made by Group as
/// Mesh and still has its parts.
pub fn grouped_parts(world: &World, id: InstanceId) -> Option<InstanceId> {
    let instance = world.get(id)?;
    if instance.class() != ClassName::MeshPart {
        return None;
    }
    instance.children().iter().copied().find(|&child| {
        world
            .get(child)
            .is_some_and(|child| child.class() == ClassName::GroupedParts)
    })
}

/// True when `id` is out of the world: in a put-away Tool (Phase 45), or
/// kept inside a UnionOperation or a GroupedParts.
pub fn out_of_world(world: &World, id: InstanceId) -> bool {
    in_union(world, id) || crate::tool::is_stored(world, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_inside_a_union_are_out_of_the_world() {
        let mut world = World::new();
        let union = world.create(ClassName::UnionOperation).unwrap();
        let part = world.create(ClassName::Part).unwrap();
        world.set_parent(part, Some(union)).unwrap();
        world.set_parent(union, Some(world.workspace())).unwrap();
        assert!(in_union(&world, part));
        assert!(out_of_world(&world, part));
        assert!(!in_union(&world, union));
        assert!(!in_group(&world, part));
    }

    #[test]
    fn a_grouped_mesh_parts_kept_parts_are_out_of_the_world_and_it_is_not() {
        let mut world = World::new();
        let mesh = world.create(ClassName::MeshPart).unwrap();
        let kept = world.create(ClassName::GroupedParts).unwrap();
        let model = world.create(ClassName::Model).unwrap();
        let part = world.create(ClassName::Part).unwrap();
        world.set_parent(part, Some(model)).unwrap();
        world.set_parent(model, Some(kept)).unwrap();
        world.set_parent(kept, Some(mesh)).unwrap();
        world.set_parent(mesh, Some(world.workspace())).unwrap();
        assert!(out_of_world(&world, part), "deep inside counts too");
        assert!(in_group(&world, part));
        assert!(!out_of_world(&world, mesh));
        assert!(!out_of_world(&world, kept));
        assert_eq!(grouped_parts(&world, mesh), Some(kept));
        assert_eq!(grouped_parts(&world, part), None);
    }
}
