//! Environment presets from scripts: `Lighting:TransitionToPreset(name,
//! seconds, clock)` eases the sky and the weather from how they are to a
//! preset (a built-in such as `"Storm"`, or the project's
//! `assets/environments/<name>.environment.json`), and
//! `Lighting:StopPresetTransition()` leaves them where they've got to.
//!
//! One transition runs at a time, shared by both sides: another call takes
//! over from where the world is at that moment. [`ScriptRuntime::step`]
//! moves it on after the scripts and before the Sequences, so a playing
//! sequence's keys win over it. The blending is `anim::environment`'s,
//! which a sequence's Environment track uses too.
//!
//! [`ScriptRuntime::step`]: crate::ScriptRuntime::step

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use axcenity_anim::environment::{
    ClockTurn, EnvironmentTarget, EnvironmentTransition, find_preset,
};
use axcenity_assets::Assets;
use axcenity_core::{ClassName, World};
use mlua::{Function, Lua};

use crate::instance::LuaInstance;
use crate::lent::world_mut;

/// The transition under way, if any.
#[derive(Default)]
pub struct Environment {
    transition: Option<EnvironmentTransition>,
    /// What Lighting was found not to have, so each is said once.
    warned: BTreeSet<EnvironmentTarget>,
}

#[derive(Clone, Default)]
pub(crate) struct SharedEnvironment(pub Rc<RefCell<Environment>>);

fn shared(lua: &Lua) -> mlua::Result<Rc<RefCell<Environment>>> {
    lua.app_data_ref::<SharedEnvironment>()
        .map(|shared| shared.0.clone())
        .ok_or_else(|| mlua::Error::runtime("environment presets aren't ready"))
}

impl Environment {
    /// Moves the transition on by `seconds`, and forgets it once it has
    /// arrived.
    pub(crate) fn step(&mut self, world: &mut World, seconds: f64) {
        if let Some(transition) = &mut self.transition
            && transition.step(world, seconds)
        {
            self.transition = None;
        }
    }
}

/// What to insert for a preset's values of `target` to show.
fn insert_hint(target: EnvironmentTarget) -> Option<&'static str> {
    match target {
        EnvironmentTarget::Atmosphere => Some("an Atmosphere"),
        EnvironmentTarget::Clouds => Some("Clouds"),
        EnvironmentTarget::Weather => Some("a Weather"),
        // A world with no ColorCorrectionEffect just isn't graded.
        _ => None,
    }
}

/// The methods of Lighting.
pub(crate) fn methods(lua: &Lua) -> mlua::Result<Vec<(ClassName, &'static str, Function)>> {
    let transition = lua.create_function(
        |lua, (_, name, seconds, clock): (LuaInstance, mlua::Value, Option<f64>, Option<String>)| {
            let name = match &name {
                mlua::Value::String(name) => name.to_str()?.to_owned(),
                other => {
                    return Err(mlua::Error::runtime(format!(
                        "TransitionToPreset takes a preset's name first, such as \"Storm\", not a {}",
                        crate::types::type_name(lua, other)
                    )));
                }
            };
            let seconds = seconds.unwrap_or(0.0);
            if !seconds.is_finite() || seconds < 0.0 {
                return Err(mlua::Error::runtime(format!(
                    "TransitionToPreset takes a number of seconds 0 or more second, not {seconds}"
                )));
            }
            let turn = match clock.as_deref() {
                None => ClockTurn::Shortest,
                Some(name) => ClockTurn::from_name(name).ok_or_else(|| {
                    mlua::Error::runtime(format!(
                        "TransitionToPreset's third argument is which way the clock turns: \
                         \"Shortest\", \"Forward\" or \"Backward\", not \"{name}\""
                    ))
                })?,
            };
            let assets = lua
                .app_data_ref::<Assets>()
                .map(|assets| assets.clone())
                .unwrap_or_default();
            let preset = find_preset(&assets, &name).map_err(mlua::Error::runtime)?;
            let shared = shared(lua)?;
            let missing = {
                let mut world = world_mut(lua)?;
                let mut transition = EnvironmentTransition::start(&world, preset, seconds, turn);
                // With no seconds the world is there before the call
                // returns; otherwise this first step leaves it as it is.
                let arrived = transition.step(&mut world, 0.0);
                let mut shared = shared.borrow_mut();
                let missing: Vec<EnvironmentTarget> = transition
                    .missing
                    .iter()
                    .copied()
                    .filter(|&target| shared.warned.insert(target))
                    .collect();
                shared.transition = (!arrived).then_some(transition);
                missing
            };
            for hint in missing.into_iter().filter_map(insert_hint) {
                crate::system::warn(
                    lua,
                    format!(
                        "Lighting:TransitionToPreset(\"{name}\"): the preset sets values of {hint}, \
                         and Lighting has none; insert one for them to show"
                    ),
                );
            }
            Ok(())
        },
    )?;
    let stop = lua.create_function(|lua, _: LuaInstance| {
        shared(lua)?.borrow_mut().transition = None;
        Ok(())
    })?;
    Ok(vec![
        (ClassName::Lighting, "TransitionToPreset", transition),
        (ClassName::Lighting, "StopPresetTransition", stop),
    ])
}
