//! Character descriptions for scripts (Character Studio overhaul CO3,
//! `docs/development/character-studio/architecture.md` section 17): a
//! `HumanoidDescription`'s methods (its parameters, wardrobe and material
//! values), `Humanoid:ApplyDescription` and `GetAppliedDescription`, and
//! `Players:CreateHumanoidModelFromDescription`, over the applier
//! (`description.rs`). The system also steps the applier's
//! [`DescriptionSync`] every frame, so a description kept in a character
//! is applied where the world is loaded or replicated, and again when a
//! script changes it.
//!
//! A HumanoidDescription holds a character file's text (`Data`), so every
//! method reads it, changes the [`Description`] and writes it back
//! (`write_description`, which keeps `Base` in step). The changes
//! themselves are plain functions on a `Description`, checked against the
//! library's manifests ([`set_parameter`], [`set_item`], [`remove_item`],
//! [`set_material_value`]). Behaviour: `docs/scripting/api-character.md`.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use axcenity_assets::Assets;
use axcenity_avatar::{
    BaseManifest, Colour, Description, LIBRARY_FOLDER, Library, MaterialValue, ValueKind, WornItem,
};
use axcenity_core::{ClassName, InstanceId, STARTER_CHARACTER, World};
use axcenity_script::system::{self, GameSystem, LuaColor3, destroyed, type_name};
use axcenity_script::{LuaInstance, lent_world as world, lent_world_mut as world_mut};
use mlua::{FromLua, Function, IntoLua, Lua, ObjectLike};

use crate::description::{self, DescriptionSync};

/// Character descriptions: see the module documentation. One per runtime;
/// [`crate::install`] adds it.
#[derive(Default)]
pub struct DescriptionSystem {
    /// What's applied, which both sides' methods note and the step reads.
    sync: SharedSync,
}

/// The one [`DescriptionSync`] of a runtime, in each side's app data.
/// `Rc<RefCell<…>>` is shared ownership with the borrow checked as it
/// runs: the two interpreters are on one thread and never both inside it.
#[derive(Clone, Default)]
struct SharedSync(Rc<RefCell<DescriptionSync>>);

impl GameSystem for DescriptionSystem {
    fn added(&self, lua: &Lua) {
        lua.set_app_data(self.sync.clone());
    }

    fn methods(&self, lua: &Lua) -> mlua::Result<Vec<(ClassName, &'static str, Function)>> {
        methods(lua)
    }

    /// Applies each kept description that isn't applied as it is now: a
    /// world just loaded or replicated, or one a script changed. It runs
    /// on both sides, so a change is applied in the frame it was made
    /// whichever side made it; the second side finds nothing left to do.
    fn step(&self, lua: &Lua) {
        let assets = assets(lua);
        let errors = match world_mut(lua) {
            Ok(mut world) => self.sync.0.borrow_mut().step(&mut world, &assets),
            Err(_) => return,
        };
        // The World is given back before anything is reported. Each is
        // said once: the sync notes what it couldn't apply.
        for error in errors {
            system::report(lua, error);
        }
    }
}

/// The project's files on `lua`'s side (`ScriptRuntime::set_assets`).
fn assets(lua: &Lua) -> Assets {
    lua.app_data_ref::<Assets>()
        .map(|assets| assets.clone())
        .unwrap_or_default()
}

fn failed(call: &str, why: impl std::fmt::Display) -> mlua::Error {
    mlua::Error::runtime(format!("{call}: {why}"))
}

/// The base model `description` is on, or why there is nothing to check
/// it against.
fn base_of<'a>(
    library: &'a Library,
    description: &Description,
) -> Result<&'a BaseManifest, String> {
    library.base(&description.base).ok_or_else(|| {
        if library.bases.is_empty() {
            format!(
                "the project has no character library (`{LIBRARY_FOLDER}/catalogue.json`), so `{}` can't be found",
                description.base
            )
        } else {
            format!("the library has no base model `{}`", description.base)
        }
    })
}

/// `names` as a list in a sentence: `hair, top, shoes`.
fn listed<'a>(names: impl Iterator<Item = &'a str>) -> String {
    names.collect::<Vec<_>>().join(", ")
}

/// A stored value as scripts read it: the number its file says (`1.2`),
/// not the 32-bit number nearest to it widened (`1.2000000476837158`).
fn as_written(value: f32) -> f64 {
    value
        .to_string()
        .parse()
        .unwrap_or_else(|_| f64::from(value))
}

/// The value of the parameter `id`: `description`'s, or the base
/// manifest's default when it holds none. A parameter the base model
/// hasn't is an error. With a base model the library hasn't, what the
/// description holds can still be read.
pub(crate) fn parameter(
    library: &Library,
    description: &Description,
    id: &str,
) -> Result<f64, String> {
    let held = description.parameters.get(id).copied();
    let base = match base_of(library, description) {
        Ok(base) => base,
        Err(why) => return held.map(as_written).ok_or(why),
    };
    let parameter = base
        .parameter(id)
        .ok_or_else(|| format!("`{}` has no parameter `{id}`", base.id))?;
    Ok(as_written(held.unwrap_or(parameter.default)))
}

/// Sets the parameter `id` to `value`, clamped to the manifest's range;
/// one with choices (Male or Female) also wears what the nearest wears.
pub(crate) fn set_parameter(
    library: &Library,
    description: &mut Description,
    id: &str,
    value: f64,
) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("`{id}` needs a finite number, not {value}"));
    }
    let base = base_of(library, description)?;
    let parameter = base
        .parameter(id)
        .ok_or_else(|| format!("`{}` has no parameter `{id}`", base.id))?;
    let [from, to] = parameter.range;
    let value = (value as f32).clamp(from.min(to), from.max(to));
    description.parameters.insert(id.to_owned(), value);
    // A parameter with choices (Male or Female) wears what the choice does.
    axcenity_avatar::wear_choice(description, library, parameter, value);
    Ok(())
}

/// An error when `slot` is one a parameter's choices fill (the realistic
/// body's underwear, which follows Male or Female): it can't be changed.
fn not_chosen(library: &Library, base: &BaseManifest, slot: &str) -> Result<(), String> {
    if !axcenity_avatar::chosen_slots(library, &base.id)
        .iter()
        .any(|s| s == slot)
    {
        return Ok(());
    }
    let by = base
        .parameters
        .iter()
        .find(|parameter| {
            parameter
                .choices
                .iter()
                .flat_map(|c| &c.items)
                .any(|id| library.item(id).is_some_and(|item| item.slot == slot))
        })
        .map_or("a choice", |parameter| parameter.id.as_str());
    Err(format!(
        "the `{slot}` slot comes with `{by}` and can't be changed or emptied; set `{by}` instead"
    ))
}

/// Checks that `base` has the slot `slot`.
fn known_slot<'a>(base: &'a BaseManifest, slot: &str) -> Result<&'a axcenity_avatar::Slot, String> {
    base.slot(slot).ok_or_else(|| {
        format!(
            "`{}` has no slot `{slot}` (it has {})",
            base.id,
            listed(base.slots.iter().map(|slot| slot.id.as_str()))
        )
    })
}

/// Puts the item `id` in `slot`, as `variant` (`None` is the item as
/// made), in place of what the slot holds unless the slot holds several.
/// The new item takes the place in the list of the slot's first; one worn
/// already stays where it is with its own material values. An error,
/// changing nothing, when the library hasn't the item, it doesn't fit the
/// base model, `slot` isn't its slot, it hasn't the variant, or the slot
/// comes with a parameter's choice (the realistic body's underwear).
pub(crate) fn set_item(
    library: &Library,
    description: &mut Description,
    slot: &str,
    id: &str,
    variant: Option<&str>,
) -> Result<(), String> {
    let base = base_of(library, description)?;
    let item = library
        .item(id)
        .ok_or_else(|| format!("the library has no item `{id}`"))?;
    if !item.bases.contains(&base.id) {
        return Err(format!("`{id}` doesn't fit the base model `{}`", base.id));
    }
    not_chosen(library, base, slot)?;
    if item.slot != slot {
        return Err(format!(
            "`{id}` goes in the `{}` slot, not `{slot}`",
            item.slot
        ));
    }
    let multiple = known_slot(base, slot)?.multiple;
    if let Some(variant) = variant
        && item.variant(variant).is_none()
    {
        return Err(if item.variants.is_empty() {
            format!("`{id}` has no variant `{variant}` (it has none)")
        } else {
            format!(
                "`{id}` has no variant `{variant}` (it has {})",
                listed(item.variants.iter().map(|variant| variant.id.as_str()))
            )
        });
    }
    let in_slot = |worn: &WornItem| {
        library
            .item(&worn.id)
            .is_some_and(|other| other.slot == slot)
    };
    let at = match description.items.iter().position(|worn| worn.id == id) {
        Some(at) => at,
        None => {
            let first = if multiple {
                None
            } else {
                description.items.iter().position(in_slot)
            };
            let at = first.unwrap_or(description.items.len());
            description.items.insert(at, WornItem::new(id));
            at
        }
    };
    description.items[at].variant = variant.map(str::to_owned);
    if !multiple {
        let mut index = 0;
        description.items.retain(|worn| {
            let keep = index == at || !in_slot(worn);
            index += 1;
            keep
        });
    }
    Ok(())
}

/// Takes off every item in `slot`. A slot the base model hasn't is an
/// error (a mistyped name would otherwise do nothing, silently), and so is
/// one that comes with a parameter's choice (the realistic body's
/// underwear can't be taken off).
pub(crate) fn remove_item(
    library: &Library,
    description: &mut Description,
    slot: &str,
) -> Result<(), String> {
    let base = base_of(library, description)?;
    known_slot(base, slot)?;
    not_chosen(library, base, slot)?;
    description
        .items
        .retain(|worn| !library.item(&worn.id).is_some_and(|item| item.slot == slot));
    Ok(())
}

/// Sets the value `name` of the base model's material `slot`, checked
/// against the slot's shading model: its name, its kind (a number or a
/// colour) and, for a number, its range. A number out of range is an
/// error, not clamped: a material value has no slider to stop at.
pub(crate) fn set_material_value(
    library: &Library,
    description: &mut Description,
    slot: &str,
    name: &str,
    value: MaterialValue,
) -> Result<(), String> {
    let base = base_of(library, description)?;
    let material = base.material(slot).ok_or_else(|| {
        format!(
            "`{}` has no material `{slot}` (it has {})",
            base.id,
            listed(base.materials.iter().map(|material| material.name.as_str()))
        )
    })?;
    let spec = material.shading.value(name).ok_or_else(|| {
        format!(
            "the `{slot}` material has no value `{name}` (it takes {})",
            listed(material.shading.values().iter().map(|spec| spec.name))
        )
    })?;
    match (spec.kind, value) {
        (ValueKind::Number { .. }, MaterialValue::Number(number)) if !number.is_finite() => {
            return Err(format!("`{name}` takes a finite number, not {number}"));
        }
        (ValueKind::Number { min, max, .. }, MaterialValue::Number(number)) => {
            if number < min || number > max {
                return Err(format!(
                    "`{name}` takes a number from {min} to {max}, not {}",
                    as_written(number)
                ));
            }
        }
        (ValueKind::Number { min, max, .. }, MaterialValue::Colour(_)) => {
            return Err(format!(
                "`{name}` takes a number from {min} to {max}, not a Color3"
            ));
        }
        (ValueKind::Colour { .. }, MaterialValue::Number(_)) => {
            return Err(format!("`{name}` takes a Color3, not a number"));
        }
        (ValueKind::Colour { .. }, MaterialValue::Colour(_)) => {}
    }
    description
        .materials
        .entry(slot.to_owned())
        .or_default()
        .insert(name.to_owned(), value);
    Ok(())
}

/// Checks that `id` is a HumanoidDescription.
fn check_description(world: &World, id: InstanceId, call: &str) -> mlua::Result<()> {
    match world.get(id).map(|instance| instance.class()) {
        Some(ClassName::HumanoidDescription) => Ok(()),
        Some(class) => Err(mlua::Error::runtime(format!(
            "{call} needs a HumanoidDescription, not a {class}"
        ))),
        None => Err(destroyed()),
    }
}

/// The description the HumanoidDescription `id` holds, and the project's
/// character library to check changes to it against. The World is
/// released again before this returns.
fn held_by(lua: &Lua, id: InstanceId, call: &str) -> mlua::Result<(Arc<Library>, Description)> {
    let library = description::library(&assets(lua));
    let world = world(lua)?;
    check_description(&world, id, call)?;
    let held = description::description_of(&world, id, &library).map_err(|why| {
        mlua::Error::runtime(format!(
            "{call} can't read the HumanoidDescription's Data: {why}"
        ))
    })?;
    Ok((library, held))
}

/// Changes the description the HumanoidDescription `id` holds and writes
/// it back. A change that fails writes nothing.
fn edit(
    lua: &Lua,
    id: InstanceId,
    call: &str,
    change: impl FnOnce(&Library, &mut Description) -> Result<(), String>,
) -> mlua::Result<()> {
    let (library, mut held) = held_by(lua, id, call)?;
    change(&library, &mut held).map_err(|why| failed(call, why))?;
    description::write_description(&mut *world_mut(lua)?, id, &held);
    Ok(())
}

fn text_arg(lua: &Lua, value: &mlua::Value, call: &str, what: &str) -> mlua::Result<String> {
    match value {
        mlua::Value::String(text) => Ok(text.to_string_lossy()),
        other => Err(mlua::Error::runtime(format!(
            "{call}'s {what} must be a string, not {}",
            type_name(lua, other)
        ))),
    }
}

fn number_arg(lua: &Lua, value: &mlua::Value, call: &str, what: &str) -> mlua::Result<f64> {
    match value {
        mlua::Value::Integer(number) => Ok(*number as f64),
        mlua::Value::Number(number) if number.is_finite() => Ok(*number),
        mlua::Value::Number(number) => Err(mlua::Error::runtime(format!(
            "{call}'s {what} must be a finite number, not {number}"
        ))),
        other => Err(mlua::Error::runtime(format!(
            "{call}'s {what} must be a finite number, not {}",
            type_name(lua, other)
        ))),
    }
}

/// A HumanoidDescription given to a method.
fn description_arg(lua: &Lua, value: &mlua::Value, call: &str) -> mlua::Result<InstanceId> {
    let Ok(given) = LuaInstance::from_lua(value.clone(), lua) else {
        return Err(mlua::Error::runtime(format!(
            "{call} needs a HumanoidDescription, not {}",
            type_name(lua, value)
        )));
    };
    check_description(&*world(lua)?, given.0, call)?;
    Ok(given.0)
}

/// A material value from a script: a number, or a `Color3` as the nearest
/// `#rrggbb`.
fn material_value_arg(lua: &Lua, value: &mlua::Value, call: &str) -> mlua::Result<MaterialValue> {
    if let mlua::Value::UserData(data) = value
        && let Ok(colour) = data.borrow::<LuaColor3>()
    {
        let channel = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
        let colour = colour.0;
        return Ok(MaterialValue::Colour(Colour([
            channel(colour.r),
            channel(colour.g),
            channel(colour.b),
        ])));
    }
    match value {
        mlua::Value::Integer(_) | mlua::Value::Number(_) => {
            number_arg(lua, value, call, "value").map(|number| MaterialValue::Number(number as f32))
        }
        other => Err(mlua::Error::runtime(format!(
            "{call}'s value must be a number or a Color3, not {}",
            type_name(lua, other)
        ))),
    }
}

/// Makes `character` look like `description` and notes what it keeps as
/// applied, so the step doesn't apply it a second time. Items left out
/// are warnings; a description that can't be applied is the error.
fn apply(
    lua: &Lua,
    character: InstanceId,
    description: &Description,
    call: &str,
) -> mlua::Result<()> {
    let assets = assets(lua);
    let applied = {
        let mut world = world_mut(lua)?;
        let applied = description::apply_description(&mut world, &assets, character, description)
            .map_err(|why| failed(call, why))?;
        if let Some(sync) = lua.app_data_ref::<SharedSync>() {
            sync.0.borrow_mut().note(&world, applied.description);
        }
        applied
    };
    for problem in &applied.problems {
        system::warn(
            lua,
            format!("{call} left an item out: {}.", problem.message),
        );
    }
    Ok(())
}

/// The methods of a HumanoidDescription, and the Humanoid's and Players'
/// that take one. Both sides have them.
fn methods(lua: &Lua) -> mlua::Result<Vec<(ClassName, &'static str, Function)>> {
    let get_parameter = lua.create_function(|lua, (this, id): (LuaInstance, mlua::Value)| {
        let call = "GetParameter(id)";
        let id = text_arg(lua, &id, call, "id")?;
        let (library, held) = held_by(lua, this.0, call)?;
        parameter(&library, &held, &id).map_err(|why| failed(call, why))
    })?;
    let set_parameter = lua.create_function(
        |lua, (this, id, value): (LuaInstance, mlua::Value, mlua::Value)| {
            let call = "SetParameter(id, value)";
            let id = text_arg(lua, &id, call, "id")?;
            let value = number_arg(lua, &value, call, "value")?;
            edit(lua, this.0, call, |library, held| {
                set_parameter(library, held, &id, value)
            })
        },
    )?;
    let get_items = lua.create_function(|lua, this: LuaInstance| {
        let (library, held) = held_by(lua, this.0, "GetItems()")?;
        let items = lua.create_table()?;
        for worn in &held.items {
            let item = lua.create_table()?;
            item.set("Id", worn.id.as_str())?;
            // Nil for an item the library hasn't.
            let slot = library.item(&worn.id).map(|item| item.slot.as_str());
            item.set("Slot", slot)?;
            item.set("Variant", worn.variant.as_deref())?;
            items.push(item)?;
        }
        Ok(items)
    })?;
    let set_item = lua.create_function(
        |lua, (this, slot, id, variant): (LuaInstance, mlua::Value, mlua::Value, mlua::Value)| {
            let call = "SetItem(slot, id, variant)";
            let slot = text_arg(lua, &slot, call, "slot")?;
            let id = text_arg(lua, &id, call, "id")?;
            let variant = match &variant {
                mlua::Value::Nil => None,
                value => Some(text_arg(lua, value, call, "variant")?),
            };
            edit(lua, this.0, call, |library, held| {
                set_item(library, held, &slot, &id, variant.as_deref())
            })
        },
    )?;
    let remove_item = lua.create_function(|lua, (this, slot): (LuaInstance, mlua::Value)| {
        let call = "RemoveItem(slot)";
        let slot = text_arg(lua, &slot, call, "slot")?;
        edit(lua, this.0, call, |library, held| {
            remove_item(library, held, &slot)
        })
    })?;
    let set_material_value = lua.create_function(
        |lua, (this, slot, name, value): (LuaInstance, mlua::Value, mlua::Value, mlua::Value)| {
            let call = "SetMaterialValue(slot, name, value)";
            let slot = text_arg(lua, &slot, call, "slot")?;
            let name = text_arg(lua, &name, call, "name")?;
            let value = material_value_arg(lua, &value, call)?;
            edit(lua, this.0, call, |library, held| {
                set_material_value(library, held, &slot, &name, value)
            })
        },
    )?;
    let apply_description =
        lua.create_function(|lua, (this, given): (LuaInstance, mlua::Value)| {
            let call = "ApplyDescription(description)";
            let given = description_arg(lua, &given, call)?;
            let character = {
                let world = world(lua)?;
                let humanoid = world.get(this.0).ok_or_else(destroyed)?;
                humanoid
                    .parent()
                    .filter(|_| humanoid.class() == ClassName::Humanoid)
                    .ok_or_else(|| {
                        mlua::Error::runtime(format!(
                            "{call} needs the Humanoid to be in a character's Model"
                        ))
                    })?
            };
            let (_, description) = held_by(lua, given, call)?;
            apply(lua, character, &description, call)
        })?;
    let get_applied = lua.create_function(|lua, this: LuaInstance| {
        let mut world = world_mut(lua)?;
        let humanoid = world.get(this.0).ok_or_else(destroyed)?;
        let kept = humanoid
            .parent()
            .and_then(|character| description::applied_instance(&world, character));
        let Some(kept) = kept else {
            return Ok(None);
        };
        // A copy with no parent: changing it changes nothing until it's
        // applied.
        let copy = world
            .duplicate(kept)
            .map_err(|err| failed("GetAppliedDescription()", err))?;
        Ok(Some(LuaInstance(copy)))
    })?;
    let create_model = lua.create_function(|lua, (_, given): (LuaInstance, mlua::Value)| {
        let call = "CreateHumanoidModelFromDescription(description)";
        let given = description_arg(lua, &given, call)?;
        let (_, description) = held_by(lua, given, call)?;
        let template = {
            let world = world(lua)?;
            world
                .find_first_child(world.starter_player(), STARTER_CHARACTER)
                .filter(|&model| {
                    world
                        .get(model)
                        .is_some_and(|model| model.class() == ClassName::Model)
                })
        };
        let template = template.ok_or_else(|| {
            mlua::Error::runtime(format!(
                "{call} copies the game's StarterCharacter, and there is none: set \
                 StarterPlayer.StarterCharacter to a character prefab (Insert > Character \
                 makes `assets/prefabs/Character.prefab.json`)"
            ))
        })?;
        // Copied as `template:Clone()` copies it (the World isn't borrowed
        // here), so its scripts start once the copy is in the Workspace.
        let copy: LuaInstance = match LuaInstance(template).into_lua(lua)? {
            mlua::Value::UserData(template) => template.call_method("Clone", ())?,
            _ => return Err(destroyed()),
        };
        if let Err(err) = apply(lua, copy.0, &description, call) {
            let _ = system::destroy(lua, copy.0);
            return Err(err);
        }
        Ok(copy)
    })?;
    Ok(vec![
        (
            ClassName::HumanoidDescription,
            "GetParameter",
            get_parameter,
        ),
        (
            ClassName::HumanoidDescription,
            "SetParameter",
            set_parameter,
        ),
        (ClassName::HumanoidDescription, "GetItems", get_items),
        (ClassName::HumanoidDescription, "SetItem", set_item),
        (ClassName::HumanoidDescription, "RemoveItem", remove_item),
        (
            ClassName::HumanoidDescription,
            "SetMaterialValue",
            set_material_value,
        ),
        (ClassName::Humanoid, "ApplyDescription", apply_description),
        (ClassName::Humanoid, "GetAppliedDescription", get_applied),
        (
            ClassName::Players,
            "CreateHumanoidModelFromDescription",
            create_model,
        ),
    ])
}
