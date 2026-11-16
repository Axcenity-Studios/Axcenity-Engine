//! The Character Runtime's applier (Character Studio overhaul CO3,
//! `docs/development/character-studio/architecture.md` sections 3, 7 and
//! 17): makes a character in the World look like a description.
//!
//! [`apply_description`] resolves a description against the project's
//! character library (`assets/characters/`, [`library`]) and writes the
//! result onto the character:
//!
//! - its `Body` MeshPart draws the base model's body with the description's
//!   **identity baked in** (a derived model, `assets`' `identity.rs`), and
//!   so does every part, so characters with the same description share
//!   their meshes;
//! - the **parts** it wears are MeshParts inside the `Body` (the base
//!   model's eyes, teeth, brows and lashes, then one per item), which
//!   follow its pose and live shapes (`anim`'s followers); a part on a
//!   socket hangs from it (`anim`'s sockets);
//! - **attributes** on the `Body` (`BodyHeight`, …) are `anim`'s body
//!   sliders; a `Body` with no Animator gets one, which poses it at rest
//!   so its parts are placed on it even when nothing plays;
//! - **material values** are attributes on the part whose model has the
//!   material (`core::material_value_attribute`), which the scene reads;
//! - the description itself is kept as a `HumanoidDescription` inside the
//!   character's Humanoid, which is what `GetAppliedDescription` reads and
//!   what [`DescriptionSync`] applies again wherever the world is loaded
//!   or replicated: each device builds the meshes itself.
//!
//! Plain World functions: the script API (`description_system.rs`), Character
//! Studio's preview and the visual QA capture all call them.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::SystemTime;

use axcenity_assets::Assets;
use axcenity_avatar::{
    Description, LIBRARY_FOLDER, Library, MaterialValue, MaterialValues, PartFit, Problem,
    ResolvedPart,
};
use axcenity_core::{
    ClassName, Color3, InstanceId, MATERIAL_VALUE_PREFIX, Value, World, material_value_attribute,
};

/// The attribute on a MeshPart the applier owns: on a part inside a
/// `Body`, the item it is (an ID such as `axcenity/tee`) or, for the base
/// model's own parts and the `Body` itself, `base:` and its model in the
/// library. Parts without it are the creator's own and are left alone.
pub const DESCRIPTION_PART: &str = "DescriptionPart";

/// What an applied description is called when its character has no
/// Humanoid to hold it.
pub const APPLIED_DESCRIPTION: &str = "AppliedDescription";

/// What [`apply_description`] did.
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    /// The character's skinned body.
    pub body: InstanceId,
    /// The parts it wears now, in the description's order.
    pub parts: Vec<InstanceId>,
    /// The HumanoidDescription kept in the character.
    pub description: InstanceId,
    /// What was left out and why (items that can't be worn).
    pub problems: Vec<Problem>,
}

/// The project's character library (`assets/characters/` in `assets`'
/// folder), read once and again when its catalogue changes. Empty when the
/// project has none.
pub fn library(assets: &Assets) -> Arc<Library> {
    type Kept = HashMap<PathBuf, (Option<SystemTime>, Arc<Library>)>;
    static LIBRARIES: OnceLock<Mutex<Kept>> = OnceLock::new();
    let Some(folder) = assets.folder() else {
        return Arc::default();
    };
    let root = folder.join(LIBRARY_FOLDER);
    let stamp = std::fs::metadata(root.join("catalogue.json"))
        .and_then(|file| file.modified())
        .ok();
    let libraries = LIBRARIES.get_or_init(Mutex::default);
    let mut kept = libraries.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((kept_stamp, library)) = kept.get(&root)
        && *kept_stamp == stamp
    {
        return Arc::clone(library);
    }
    let library = match stamp {
        Some(_) => Arc::new(Library::read(&root).0),
        None => Arc::default(),
    };
    kept.insert(root, (stamp, Arc::clone(&library)));
    library
}

/// The skinned body of `character`: `character` itself if it's a MeshPart,
/// else the MeshPart named `Body` inside it, else the first MeshPart
/// inside it that holds an Animator.
pub fn body_of(world: &World, character: InstanceId) -> Option<InstanceId> {
    let instance = world.get(character)?;
    if instance.class() == ClassName::MeshPart {
        return Some(character);
    }
    let is_mesh_part = |id: InstanceId| {
        world
            .get(id)
            .is_some_and(|child| child.class() == ClassName::MeshPart)
    };
    world
        .find_first_child(character, "Body")
        .filter(|body| is_mesh_part(*body))
        .or_else(|| {
            instance.children().iter().copied().find(|child| {
                is_mesh_part(*child)
                    && world.get(*child).is_some_and(|part| {
                        part.children().iter().any(|inside| {
                            world
                                .get(*inside)
                                .is_some_and(|inside| inside.class() == ClassName::Animator)
                        })
                    })
            })
        })
}

/// The Humanoid of `character` (a Model), if it has one.
fn humanoid_of(world: &World, character: InstanceId) -> Option<InstanceId> {
    world.get(character)?.children().iter().copied().find(|id| {
        world
            .get(*id)
            .is_some_and(|child| child.class() == ClassName::Humanoid)
    })
}

/// The HumanoidDescription `character` has applied: the one inside its
/// Humanoid, or the one named [`APPLIED_DESCRIPTION`] inside the character
/// when it has no Humanoid.
pub fn applied_instance(world: &World, character: InstanceId) -> Option<InstanceId> {
    let is_description = |id: &InstanceId| {
        world
            .get(*id)
            .is_some_and(|child| child.class() == ClassName::HumanoidDescription)
    };
    match humanoid_of(world, character) {
        Some(humanoid) => world
            .get(humanoid)?
            .children()
            .iter()
            .copied()
            .find(is_description),
        None => world
            .find_first_child(character, APPLIED_DESCRIPTION)
            .filter(is_description),
    }
}

/// The description a HumanoidDescription Instance holds: its `Data`, or
/// the default character of its `Base` when `Data` is empty.
pub fn description_of(
    world: &World,
    instance: InstanceId,
    library: &Library,
) -> Result<Description, String> {
    let held = world
        .get(instance)
        .filter(|held| held.class() == ClassName::HumanoidDescription)
        .ok_or("that isn't a HumanoidDescription")?;
    let text = |name| {
        held.property(name)
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    let data = text("Data");
    if !data.trim().is_empty() {
        return Description::from_json(data).map_err(|err| err.to_string());
    }
    let base = text("Base");
    let base = if base.is_empty() {
        axcenity_avatar::V1_BASE
    } else {
        base
    };
    Ok(default_description(library, base))
}

/// A new character on the base model `base`: its default items, every
/// parameter at its default.
pub fn default_description(library: &Library, base: &str) -> Description {
    let mut description = Description::new("New Character", base);
    if let Some(manifest) = library.base(base) {
        description.items = manifest
            .defaults
            .items
            .iter()
            .map(axcenity_avatar::WornItem::new)
            .collect();
    }
    description
}

/// Writes `description` into the HumanoidDescription `instance` (`Base`
/// and `Data`), changing only what differs.
pub fn write_description(world: &mut World, instance: InstanceId, description: &Description) {
    for (name, text) in [
        ("Base", description.base.clone()),
        ("Data", description.to_json()),
    ] {
        let same = world
            .get(instance)
            .and_then(|held| held.property(name))
            .and_then(Value::as_str)
            == Some(text.as_str());
        if !same {
            let _ = world.set_property(instance, name, Value::String(text));
        }
    }
}

/// Makes `character` (a character's Model, or its skinned MeshPart) look
/// like `description`, resolved against the project's character library:
/// see the module documentation. Applying the same description again
/// changes nothing. Fails, changing nothing, when the character has no
/// skinned body or the library hasn't the description's base model.
pub fn apply_description(
    world: &mut World,
    assets: &Assets,
    character: InstanceId,
    description: &Description,
) -> Result<Applied, String> {
    let library = library(assets);
    let body = body_of(world, character)
        .ok_or("the character has no skinned body: a MeshPart named `Body` inside its Model")?;
    let resolved = axcenity_avatar::resolve(&library, description).map_err(|problem| {
        if library.bases.is_empty() {
            format!(
                "the project has no character library (`{LIBRARY_FOLDER}/catalogue.json`), so `{}` can't be found; Insert > Character in the editor adds it",
                description.base
            )
        } else {
            problem.message
        }
    })?;
    let base = library
        .base(&resolved.base)
        .ok_or("the base model is gone")?;
    let identity = Arc::new(resolved.identity.clone());
    let mesh_id = |model: &str| assets.derive(&format!("{LIBRARY_FOLDER}/{model}"), &identity);

    // The body: its model, kept where the old one's skeleton stood.
    let wanted = mesh_id(&resolved.body.model);
    let current = text_property(world, body, "MeshId");
    if current != wanted {
        let fit = |mesh_id: &str| {
            assets
                .model_blocking(mesh_id)
                .and_then(|model| model.rig.as_ref().map(|rig| rig.fit))
        };
        if let (Some(old), Some(new)) = (fit(&current), fit(&wanted)) {
            axcenity_anim::ragdoll::refit(world, body, old, new);
        }
        let _ = world.set_property(body, "MeshId", Value::String(wanted));
    }
    set_marker(world, body, &resolved.body);

    // Body sliders. One at 1 changes nothing, so it isn't written unless
    // it has to undo an earlier value; one this base doesn't drive (left
    // by another base model) goes back to 1.
    let mut sliders = resolved.attributes.clone();
    for name in axcenity_anim::BODY_SLIDERS {
        let set = world
            .get(body)
            .is_some_and(|instance| instance.attribute(name).is_some());
        if set {
            sliders.entry(name.to_owned()).or_insert(1.0);
        }
    }
    for (name, value) in &sliders {
        let now = world
            .get(body)
            .and_then(|instance| instance.attribute(name))
            .and_then(Value::as_number);
        let value = f64::from(*value);
        let neutral = (value - 1.0).abs() < 1e-6;
        match now {
            Some(now) if (now - value).abs() < 1e-6 => {}
            None if neutral => {}
            _ => {
                let _ = world.set_attribute(body, name, Some(Value::Number(value)));
            }
        }
    }

    // Something must pose the body for its parts to be placed on it: an
    // Animator, which poses it at rest when it has nothing to play.
    let animated = children_of(world, body).into_iter().any(|child| {
        world
            .get(child)
            .is_some_and(|child| child.class() == ClassName::Animator)
    });
    if !animated && let Ok(animator) = world.create(ClassName::Animator) {
        let _ = world.set_name(animator, "Animator");
        let _ = world.set_parent(animator, Some(body));
    }

    // Sockets the base model has and the body hasn't yet.
    for socket in &base.sockets {
        if world.find_first_child(body, &socket.name).is_none()
            && let Ok(attachment) = world.create(ClassName::Attachment)
        {
            let _ = world.set_name(attachment, &socket.name);
            let _ = world.set_attribute(
                attachment,
                axcenity_anim::SOCKET_BONE,
                Some(Value::String(socket.bone.clone())),
            );
            let _ = world.set_parent(attachment, Some(body));
        }
    }

    // The parts: the ones already worn are kept (and keep their pose),
    // the others go, new ones are put on.
    let known = known_part_names(&library, &resolved.base);
    let mut worn: Vec<(InstanceId, String)> = Vec::new();
    for child in children_of(world, body) {
        let Some(instance) = world.get(child) else {
            continue;
        };
        if instance.class() != ClassName::MeshPart {
            continue;
        }
        match instance.attribute(DESCRIPTION_PART).and_then(Value::as_str) {
            Some(key) => worn.push((child, key.to_owned())),
            // A part put on by hand that is one of the library's (the
            // default humanoid's own parts) is taken over.
            None if known.contains(instance.name()) => {
                worn.push((child, format!("name:{}", instance.name())));
            }
            None => {}
        }
    }
    let mut parts = Vec::with_capacity(resolved.parts.len());
    for part in &resolved.parts {
        let key = part_key(part);
        let by_name = format!("name:{}", part.name);
        let found = worn
            .iter()
            .position(|(_, worn_key)| *worn_key == key)
            .or_else(|| worn.iter().position(|(_, worn_key)| *worn_key == by_name));
        let instance = match found {
            Some(index) => worn.remove(index).0,
            None => match new_part(world, body) {
                Some(instance) => instance,
                None => continue,
            },
        };
        let wanted = mesh_id(&part.model);
        if text_property(world, instance, "MeshId") != wanted {
            let _ = world.set_property(instance, "MeshId", Value::String(wanted));
        }
        if world.get(instance).map(|held| held.name()) != Some(part.name.as_str()) {
            let _ = world.set_name(instance, &part.name);
        }
        set_marker(world, instance, part);
        if let PartFit::Socket(socket) = &part.fit
            && world.find_first_child(instance, socket).is_none()
            && let Ok(attachment) = world.create(ClassName::Attachment)
        {
            let _ = world.set_name(attachment, socket);
            let _ = world.set_parent(attachment, Some(instance));
        }
        parts.push(instance);
    }
    for (leftover, _) in worn {
        let _ = world.destroy(leftover);
    }

    // Material values: the base model's on the body and its own parts
    // (whichever has the material reads it), each item's on its part.
    for (index, part) in resolved.parts.iter().enumerate() {
        let Some(instance) = parts.get(index) else {
            continue;
        };
        let values = if part.item.is_some() {
            &part.materials
        } else {
            &resolved.materials
        };
        set_material_values(world, *instance, values);
    }
    set_material_values(world, body, &resolved.materials);

    // What's applied, kept in the character.
    let holder = humanoid_of(world, character).unwrap_or(character);
    let kept = match applied_instance(world, character) {
        Some(kept) => kept,
        None => {
            let kept = world
                .create(ClassName::HumanoidDescription)
                .map_err(|err| err.to_string())?;
            let name = if holder == character {
                APPLIED_DESCRIPTION
            } else {
                "HumanoidDescription"
            };
            let _ = world.set_name(kept, name);
            let _ = world.set_parent(kept, Some(holder));
            kept
        }
    };
    write_description(world, kept, description);
    Ok(Applied {
        body,
        parts,
        description: kept,
        problems: resolved.problems,
    })
}

/// How a worn part is told apart from the others: its item, or its model
/// for the base's own parts.
fn part_key(part: &ResolvedPart) -> String {
    match &part.item {
        Some(item) => item.clone(),
        None => format!("base:{}", part.model),
    }
}

fn set_marker(world: &mut World, instance: InstanceId, part: &ResolvedPart) {
    let key = part_key(part);
    let same = world
        .get(instance)
        .and_then(|held| held.attribute(DESCRIPTION_PART))
        .and_then(Value::as_str)
        == Some(key.as_str());
    if !same {
        let _ = world.set_attribute(instance, DESCRIPTION_PART, Some(Value::String(key)));
    }
}

/// The names of every part the library has for `base`: its own parts and
/// its items' models.
fn known_part_names(library: &Library, base: &str) -> BTreeSet<String> {
    let last = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
    let mut names = BTreeSet::new();
    if let Some(manifest) = library.base(base) {
        names.extend(manifest.model.parts.iter().map(|part| last(part)));
    }
    for item in library.items.values() {
        if !item.bases.iter().any(|fits| fits == base) {
            continue;
        }
        for model in item.models() {
            names.insert(match &model.object {
                Some(object) => last(object),
                None => last(&item.id),
            });
        }
    }
    names
}

fn children_of(world: &World, parent: InstanceId) -> Vec<InstanceId> {
    world
        .get(parent)
        .map(|instance| instance.children().to_vec())
        .unwrap_or_default()
}

fn text_property(world: &World, instance: InstanceId, name: &str) -> String {
    world
        .get(instance)
        .and_then(|held| held.property(name))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A MeshPart inside `body` for a worn part: it takes its pose and place
/// from the body, and doesn't collide, so the controller's root part stays
/// the body.
fn new_part(world: &mut World, body: InstanceId) -> Option<InstanceId> {
    let part = world.create(ClassName::MeshPart).ok()?;
    for name in ["CanCollide", "CanQuery", "CanTouch"] {
        let _ = world.set_property(part, name, Value::Bool(false));
    }
    let _ = world.set_property(part, "Massless", Value::Bool(true));
    world.set_parent(part, Some(body)).ok()?;
    Some(part)
}

/// Sets `instance`'s material value attributes to `values` (by material
/// slot), removing the ones it had that aren't among them.
fn set_material_values(
    world: &mut World,
    instance: InstanceId,
    values: &BTreeMap<String, MaterialValues>,
) {
    let mut wanted: BTreeMap<String, Value> = BTreeMap::new();
    for (slot, values) in values {
        for (name, value) in values {
            let value = match value {
                MaterialValue::Number(number) => Value::Number(f64::from(*number)),
                MaterialValue::Colour(colour) => {
                    let [r, g, b] = colour.0.map(|channel| f32::from(channel) / 255.0);
                    Value::Color3(Color3::new(r, g, b))
                }
            };
            wanted.insert(material_value_attribute(slot, name), value);
        }
    }
    let Some(held) = world.get(instance) else {
        return;
    };
    let stale: Vec<String> = held
        .attributes()
        .keys()
        .filter(|name| name.starts_with(MATERIAL_VALUE_PREFIX) && !wanted.contains_key(*name))
        .cloned()
        .collect();
    let changed: Vec<(String, Value)> = wanted
        .into_iter()
        .filter(|(name, value)| held.attribute(name) != Some(value))
        .collect();
    for name in stale {
        let _ = world.set_attribute(instance, &name, None);
    }
    for (name, value) in changed {
        let _ = world.set_attribute(instance, &name, Some(value));
    }
}

/// Applies each character's kept description wherever it hasn't been in
/// this run, or has changed since: after a world is loaded, when a
/// character is replicated to this device, when a script or the editor
/// changes a kept description's `Data`. Hosts step it once a frame.
#[derive(Default)]
pub struct DescriptionSync {
    /// Each kept HumanoidDescription and what it held when it was applied.
    applied: HashMap<InstanceId, (String, String)>,
}

impl DescriptionSync {
    /// Notes that `description` (a kept HumanoidDescription) is applied as
    /// it is now, so [`step`](Self::step) doesn't apply it again.
    pub fn note(&mut self, world: &World, description: InstanceId) {
        self.applied.insert(
            description,
            (
                text_property(world, description, "Base"),
                text_property(world, description, "Data"),
            ),
        );
    }

    /// Applies what isn't applied yet. Returns what went wrong, each as a
    /// sentence.
    pub fn step(&mut self, world: &mut World, assets: &Assets) -> Vec<String> {
        let kept = world.all_below(world.workspace(), |class| {
            class == ClassName::HumanoidDescription
        });
        let mut errors = Vec::new();
        let mut alive = BTreeSet::new();
        for description in kept {
            let Some(parent) = world.get(description).and_then(|held| held.parent()) else {
                continue;
            };
            let Some(holder) = world.get(parent) else {
                continue;
            };
            // Only a description kept in a character is applied: one
            // inside its Humanoid, or named `AppliedDescription`.
            let character = if holder.class() == ClassName::Humanoid {
                holder.parent()
            } else if world.get(description).map(|held| held.name()) == Some(APPLIED_DESCRIPTION) {
                Some(parent)
            } else {
                None
            };
            let Some(character) = character else {
                continue;
            };
            alive.insert(description);
            let now = (
                text_property(world, description, "Base"),
                text_property(world, description, "Data"),
            );
            if self.applied.get(&description) == Some(&now) {
                continue;
            }
            let library = library(assets);
            let result = description_of(world, description, &library)
                .and_then(|read| apply_description(world, assets, character, &read));
            if let Err(why) = result {
                let name = world.full_name(description).unwrap_or_default();
                errors.push(format!(
                    "The HumanoidDescription `{name}` wasn't applied: {why}."
                ));
            }
            // Noted either way, so a description that can't be applied is
            // said once, not every frame. Applying wrote `Data` in its
            // canonical form.
            self.note(world, description);
        }
        self.applied.retain(|id, _| alive.contains(id));
        errors
    }
}
