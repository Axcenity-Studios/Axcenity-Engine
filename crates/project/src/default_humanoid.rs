//! The default humanoid (`docs/research/default-humanoid.md`, stage HD3):
//! the human base's default body (`assets/characters/human/Humanoid.glb`,
//! MakeHuman's, made by `default_model.py`) and the starter animations,
//! built into the engine like the default sky, so a project can have it
//! without importing anything. [`add_default_humanoid`] writes its model,
//! its starter animations (clips linked to the glTF animation file's takes
//! and a state machine) and a `Character` prefab into a project; New
//! Project's character templates set that prefab as
//! `StarterPlayer.StarterCharacter`, and Insert > Character places one.
//! [`default_character`] builds the prefab's Instances.
//!
//! The character is a Model holding an invisible `HumanoidRootPart` (the
//! body the controller measures, walks and collides with: 0.60 m across and
//! 5 studs tall, 1 stud = 0.35 m) and the visible MeshPart `Body`, welded
//! to it and not colliding, whose Animator plays the state machine. The
//! Humanoid plays the Idle, Walk, Run, Jump and Fall states as it moves,
//! and Land when it comes down standing still.

use super::*;
use axcenity_core::{Value, Vector3};

/// The model, in a project.
pub const DEFAULT_HUMANOID_MODEL_PATH: &str = "assets/models/Humanoid/Humanoid.glb";
/// The starter animations (a glTF file of takes), in a project.
pub const DEFAULT_HUMANOID_ANIMATIONS_PATH: &str = "assets/models/Humanoid/HumanoidAnimations.glb";
/// The folder of its clips and state machine, in a project.
pub const DEFAULT_HUMANOID_CLIPS_FOLDER: &str = "assets/animations/Humanoid";
/// Its state machine, in a project.
pub const DEFAULT_HUMANOID_STATE_MACHINE_PATH: &str =
    "assets/animations/Humanoid/Humanoid.statemachine.json";
/// The prefab of the whole character, in a project: what
/// `StarterPlayer.StarterCharacter` names in a new character template.
pub const DEFAULT_CHARACTER_PREFAB_PATH: &str = "assets/prefabs/Character.prefab.json";

/// The default humanoid's model (`Humanoid.glb`): the human base's
/// default body in its underwear, with its face's control shapes and no
/// others. What a character is drawn as before its description is
/// applied, and where a project has no character library.
pub static DEFAULT_HUMANOID_MODEL: &[u8] =
    include_bytes!("../../../assets/characters/human/Humanoid.glb");

/// The default character's sockets (Attachments in its `Body` that follow
/// a bone, `anim`'s sockets) and their bones: an accessory with an
/// Attachment of the same name inside the `Body` hangs from it.
pub const HUMANOID_SOCKETS: [(&str, &str); 7] = [
    ("HeadSocket", "Head"),
    ("BackSocket", "UpperChest"),
    ("HipsSocket", "Hips"),
    ("LeftHandSocket", "LeftHand"),
    ("RightHandSocket", "RightHand"),
    ("LeftFootSocket", "LeftFoot"),
    ("RightFootSocket", "RightFoot"),
];

/// Its 86 starter animations (`HumanoidAnimations.glb`): the 84 takes of
/// Quaternius's Universal Animation Library (CC0), baked onto this
/// skeleton (`assets/characters/human/mocap.py`), and its walk and jog
/// again with root motion.
pub static DEFAULT_HUMANOID_ANIMATIONS: &[u8] =
    include_bytes!("../../../assets/characters/human/HumanoidAnimations.glb");

/// Studs per metre: the 1.75 m humanoid is 5 studs tall, as Axcenity's
/// characters are.
pub const STUDS_PER_METRE: f32 = 5.0 / 1.75;

/// The model's box in its file (metres, glTF axes: Y up, facing +Z), its
/// A-posed hands out to the sides. A test checks it against the file.
pub(crate) const MODEL_MIN: [f32; 3] = [-0.6672, 0.0, -0.1042];
pub(crate) const MODEL_MAX: [f32; 3] = [0.6672, 1.75, 0.2133];

/// The root part's size in studs: 0.30 m round (the spec's capsule radius)
/// and 5 studs tall.
pub const ROOT_PART_SIZE: [f32; 3] = [0.6 * STUDS_PER_METRE, 5.0, 0.6 * STUDS_PER_METRE];

/// A state of the default state machine: its name, the take it plays,
/// its speed and what it does at its end.
struct State {
    name: &'static str,
    take: &'static str,
    speed: f64,
    end: End,
}

#[derive(Clone, Copy, PartialEq)]
enum End {
    /// Goes round.
    Loops,
    /// Holds its last frame (a pose to stay in: an aim, lying dead).
    Holds,
    /// Goes back to Idle.
    ReturnsToIdle,
    /// Goes on to the state named, the one the library made the take to
    /// lead into or to be played from: the last frame of `Jump` is the
    /// first of `Fall`, `SitDown` ends seated as `Sit` starts, and a
    /// pistol shot starts and ends in the pistol stance, not in Idle.
    /// Measured on 2026-11-25: every such pair is within 13 degrees a bone
    /// and 3 mm at the hips, but the spell's cast (23 degrees from its
    /// stance) and the sword's attack (23 mm).
    GoesTo(&'static str),
}

impl End {
    /// The state it goes to when its take has played, if it goes to one.
    fn next(self) -> Option<&'static str> {
        match self {
            End::Loops | End::Holds => None,
            End::ReturnsToIdle => Some("Idle"),
            End::GoesTo(state) => Some(state),
        }
    }
}

/// What the library's forward takes cover on the ground, metres a second
/// (`mocap.SPEEDS`, its own root motion): the jog that the `Run` take is,
/// its sprint and its crouched walk.
const JOG_SPEED: f64 = 5.36;
const SPRINT_SPEED: f64 = 8.25;
const CROUCH_WALK_SPEED: f64 = 0.75;
/// The library's own walk, metres a second (`Stroll`, below).
const STROLL_SPEED: f64 = 0.97;
/// Studs per metre, as a `f64` for the speeds below.
const STUDS: f64 = 5.0 / 1.75;

/// The default state machine's states. Walk and Run play the Run and
/// Sprint takes, a little faster, so the feet keep up with the
/// controller's default `WalkSpeed` (16 studs a second) and `RunSpeed`
/// (32): the Run take covers 5.36 m a second (15.3 studs), Sprint 8.25 m
/// (23.6 studs). The library's own walk (0.97 m a second) is `Stroll`.
/// After the states the Humanoid and Insert > Character play come one for
/// every other take of the file, so each can be played by name. A state
/// whose take doesn't loop says where it goes when the take has played
/// ([`End`]).
const STATES: [State; 84] = {
    use End::*;
    const fn s(name: &'static str, take: &'static str, speed: f64, end: End) -> State {
        State {
            name,
            take,
            speed,
            end,
        }
    }
    /// A state that plays the take of its own name as it is.
    const fn t(name: &'static str, end: End) -> State {
        s(name, name, 1.0, end)
    }
    [
        s("Idle", "Idle", 1.0, Loops),
        s("Walk", "Run", 16.0 / (JOG_SPEED * STUDS), Loops),
        s("Run", "Sprint", 32.0 / (SPRINT_SPEED * STUDS), Loops),
        s("Jump", "Jump", 1.0, GoesTo("Fall")),
        s("Fall", "Fall", 1.0, Loops),
        s("Land", "Land", 1.0, ReturnsToIdle),
        s("Sit", "Sit", 1.0, Loops),
        s("Crouch", "Crouch", 1.0, Loops),
        s(
            "CrouchWalk",
            "CrouchWalk",
            16.0 * CROUCH_SPEED_SCALE / (CROUCH_WALK_SPEED * STUDS),
            Loops,
        ),
        s("Swim", "Swim", 1.0, Loops),
        s("Push", "Push", 1.0, Loops),
        s("PickUp", "PickUp", 1.0, ReturnsToIdle),
        s("Interact", "Interact", 1.0, ReturnsToIdle),
        s("Dance", "Dance", 1.0, Loops),
        s("HitReact", "HitReact", 1.0, ReturnsToIdle),
        // The library's other takes (`mocap.TAKES`), each under its name.
        s("Stroll", "Walk", 1.0, Loops),
        t("WalkFormal", Loops),
        t("WalkCarry", Loops),
        t("IdleTalking", Loops),
        t("IdleFoldArms", Loops),
        t("IdlePhone", Loops),
        t("IdleTorch", Loops),
        t("IdleLantern", Loops),
        t("IdleRail", Loops),
        t("IdleRailCall", GoesTo("IdleRail")),
        t("Yes", ReturnsToIdle),
        t("No", Loops),
        t("SwimIdle", Loops),
        t("SitDown", GoesTo("Sit")),
        t("StandUp", ReturnsToIdle),
        t("SitTalking", Loops),
        t("GetUp", ReturnsToIdle),
        t("Drive", Loops),
        t("Roll", ReturnsToIdle),
        t("SlideStart", GoesTo("Slide")),
        t("Slide", Loops),
        t("SlideEnd", ReturnsToIdle),
        t("ClimbUp", ReturnsToIdle),
        t("HighJump", GoesTo("HighJumpFall")),
        t("HighJumpFall", Loops),
        t("HighJumpLand", ReturnsToIdle),
        t("Death", Holds),
        t("HitHead", ReturnsToIdle),
        t("Knockback", Holds),
        t("Fix", ReturnsToIdle),
        t("OpenChest", ReturnsToIdle),
        t("Consume", ReturnsToIdle),
        t("Harvest", ReturnsToIdle),
        t("PlantSeed", ReturnsToIdle),
        t("Water", ReturnsToIdle),
        t("ChopTree", Loops),
        t("Throw", ReturnsToIdle),
        t("PunchJab", ReturnsToIdle),
        t("PunchCross", ReturnsToIdle),
        t("Hook", GoesTo("HookRecover")),
        t("HookRecover", ReturnsToIdle),
        t("SwordIdle", Loops),
        t("SwordAttack", GoesTo("SwordIdle")),
        t("SwordBlock", ReturnsToIdle),
        t("SwordDash", ReturnsToIdle),
        t("SwordSlashA", GoesTo("SwordSlashARecover")),
        t("SwordSlashARecover", ReturnsToIdle),
        t("SwordSlashB", GoesTo("SwordSlashBRecover")),
        t("SwordSlashBRecover", ReturnsToIdle),
        t("SwordSlashC", ReturnsToIdle),
        t("SwordCombo", ReturnsToIdle),
        t("SwordHeavyCombo", ReturnsToIdle),
        t("ShieldIdle", Loops),
        t("ShieldBreak", GoesTo("ShieldIdle")),
        t("ShieldDash", ReturnsToIdle),
        t("ShieldBash", ReturnsToIdle),
        t("PistolIdle", Loops),
        t("PistolAim", Holds),
        t("PistolAimUp", Holds),
        t("PistolAimDown", Holds),
        t("PistolShoot", GoesTo("PistolIdle")),
        t("PistolReload", GoesTo("PistolIdle")),
        t("SpellStart", GoesTo("SpellIdle")),
        t("SpellIdle", Loops),
        t("SpellCast", GoesTo("SpellIdle")),
        t("SpellEnd", ReturnsToIdle),
        t("ZombieIdle", Loops),
        t("ZombieWalk", Loops),
        t("ZombieScratch", GoesTo("ZombieIdle")),
    ]
};

/// Writes the default humanoid into `project` (its model, animation file,
/// a clip per state, the state machine and the `Character` prefab) and
/// returns the prefab's path. Files already there are kept, so this never
/// writes over a creator's own (a changed clip or prefab stays changed).
/// A character library isn't part of it: [`add_default_game_character`]
/// brings the human base, so the character can be given a description
/// (`Humanoid:ApplyDescription`).
pub fn add_default_humanoid(project: &Path) -> Result<&'static str, SaveError> {
    write_default_humanoid(project, write_missing)
}

/// [`add_default_humanoid`] for a folder the engine keeps for itself
/// (Character Studio's own, which no creator edits): every file of the
/// default humanoid there that isn't this engine's is written again, so
/// the folder never plays an earlier build's animations. Without it a
/// folder made by an earlier build kept that build's animation file and
/// state machine beside this build's clips: takes the file didn't have
/// played nothing, and the others played the old motion at the new
/// lengths (seen in Character Studio on 2026-11-25, with a folder made on
/// the 21st). Returns the prefab's path.
pub fn refresh_default_humanoid(folder: &Path) -> Result<&'static str, SaveError> {
    write_default_humanoid(folder, write_changed)
}

/// Writes the default humanoid's files into `project` through `write`,
/// which decides what happens to a file already there.
fn write_default_humanoid(
    project: &Path,
    write: fn(&Path, &str, &[u8]) -> Result<(), SaveError>,
) -> Result<&'static str, SaveError> {
    let write = |path: &str, bytes: &[u8]| write(project, path, bytes);
    write(DEFAULT_HUMANOID_MODEL_PATH, DEFAULT_HUMANOID_MODEL)?;
    write(
        DEFAULT_HUMANOID_ANIMATIONS_PATH,
        DEFAULT_HUMANOID_ANIMATIONS,
    )?;
    for (path, text) in default_humanoid_animation_files() {
        write(&path, text.as_bytes())?;
    }
    let mut world = World::new();
    let character = default_character(&mut world, Vector3::default());
    write(
        DEFAULT_CHARACTER_PREFAB_PATH,
        world_file::write_prefab(&world, &[character]).as_bytes(),
    )?;
    Ok(DEFAULT_CHARACTER_PREFAB_PATH)
}

/// Writes `bytes` to `path` in `project` unless a file is there already.
pub(crate) fn write_missing(project: &Path, path: &str, bytes: &[u8]) -> Result<(), SaveError> {
    let file = project.join(path);
    if file.is_file() {
        return Ok(());
    }
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder).map_err(SaveError::new(folder))?;
    }
    fs::write(&file, bytes).map_err(SaveError::new(&file))
}

/// Writes `bytes` to `path` in `project` unless the file there holds them
/// already.
fn write_changed(project: &Path, path: &str, bytes: &[u8]) -> Result<(), SaveError> {
    let file = project.join(path);
    if fs::read(&file).is_ok_and(|there| there == bytes) {
        return Ok(());
    }
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder).map_err(SaveError::new(folder))?;
    }
    fs::write(&file, bytes).map_err(SaveError::new(&file))
}

/// A MeshPart's `Size` and its middle's offset from the feet, in studs, for
/// a model whose box in its file is `min` to `max` (metres): turned 180°
/// so its front (+Z in the file) faces -Z, which puts its middle at -x, -z.
fn body_box(min: [f32; 3], max: [f32; 3]) -> (Vector3, Vector3) {
    let size = |axis: usize| (max[axis] - min[axis]) * STUDS_PER_METRE;
    let middle = |axis: usize| (max[axis] + min[axis]) * 0.5 * STUDS_PER_METRE;
    (
        Vector3::new(size(0), size(1), size(2)),
        Vector3::new(-middle(0), middle(1), -middle(2)),
    )
}

/// The default humanoid's clip files and state machine, as `(path, text)`:
/// one clip per state, linked to its take in the animation file over the
/// take's whole length.
pub fn default_humanoid_animation_files() -> Vec<(String, String)> {
    let takes = axcenity_assets::load_takes(
        DEFAULT_HUMANOID_ANIMATIONS,
        axcenity_assets::ModelFormat::Gltf,
    )
    .unwrap_or_default();
    let mut files = Vec::new();
    let mut states = Vec::new();
    for (index, state) in STATES.iter().enumerate() {
        let duration = takes
            .iter()
            .find(|take| take.name == state.take)
            .map_or(1.0, |take| f64::from(take.duration));
        let clip = format!("{DEFAULT_HUMANOID_CLIPS_FOLDER}/{}.anim.json", state.name);
        let text = serde_json::json!({
            "format_version": 2,
            "duration": duration,
            "loop": state.end == End::Loops,
            "link": {
                "source": DEFAULT_HUMANOID_ANIMATIONS_PATH,
                "take": state.take,
                "start": 0.0,
                "end": duration,
            },
            "tracks": [],
            "events": [],
        });
        files.push((clip.clone(), pretty(&text)));
        // Laid out in rows of eight in the editor's graph.
        let (column, row) = (index % 8, index / 8);
        let mut entry = serde_json::json!({
            "name": state.name,
            "motion": { "clip": clip },
            "speed": state.speed,
            "position": [column as f64 * 180.0, row as f64 * 110.0],
        });
        if let Some(next) = state.end.next() {
            entry["returns"] = serde_json::json!({ "to": next, "when": "finished", "blend": 0.2 });
        }
        if state.name == "Walk" {
            entry["motion"] = locomotion();
            entry["speed"] = serde_json::json!(1.0);
        }
        states.push(entry);
    }
    let machine = serde_json::json!({
        "format_version": 2,
        "parameters": [{ "type": "Number", "name": "Speed", "default": 0.0 }],
        "default_state": "Idle",
        "states": states,
        "transitions": [],
    });
    files.push((
        DEFAULT_HUMANOID_STATE_MACHINE_PATH.to_owned(),
        pretty(&machine),
    ));
    files
}

/// The default Walk's motion: a blend tree by `Speed` (the Humanoid keeps
/// it at how fast the feet go) from the library's walk (the Stroll state's
/// take, at its own 2.8 studs a second) through Walk's take at `WalkSpeed`
/// (16) to Run's at `RunSpeed` (32), each played as fast as its state is,
/// so the strides keep up with the ground at every speed between and the
/// character eases from a walk into a run (2026-11-26). All three put the
/// left foot forward at the start of their cycle, so they mix in step.
fn locomotion() -> serde_json::Value {
    let clip = |name: &str| format!("{DEFAULT_HUMANOID_CLIPS_FOLDER}/{name}.anim.json");
    let speed = |name: &str| {
        STATES
            .iter()
            .find(|state| state.name == name)
            .map_or(1.0, |state| state.speed)
    };
    serde_json::json!({ "blend_tree": {
        "parameter": "Speed",
        "children": [
            { "clip": clip("Stroll"), "threshold": STROLL_SPEED * STUDS },
            { "clip": clip("Walk"), "threshold": 16.0, "speed": speed("Walk") },
            { "clip": clip("Run"), "threshold": 32.0, "speed": speed("Run") },
        ],
    }})
}

fn pretty(value: &serde_json::Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

/// The default character's `CrouchSpeedScale`.
const CROUCH_SPEED_SCALE: f64 = 0.25;

/// Builds the default character with no parent: a Model named Character,
/// its feet at `position`, facing -Z (as characters do), holding the
/// invisible `HumanoidRootPart`, the MeshPart `Body` welded to it (with an
/// Animator playing [`DEFAULT_HUMANOID_STATE_MACHINE_PATH`]), a Humanoid,
/// a ThirdPersonController, a Health and an Inventory. Its files are the
/// project's (see [`add_default_humanoid`]).
pub fn default_character(world: &mut World, position: Vector3) -> InstanceId {
    let mut make =
        |class, name: &str, parent: Option<InstanceId>, properties: Vec<(&str, Value)>| {
            let id = world.create(class).expect("not a service");
            world.set_name(id, name).expect("just made");
            for (property, value) in properties {
                world
                    .set_property(id, property, value)
                    .expect("the class has it");
            }
            if let Some(parent) = parent {
                world.set_parent(id, Some(parent)).expect("a new Model");
            }
            id
        };
    let at = |x: f32, y: f32, z: f32| {
        Value::Vector3(Vector3::new(position.x + x, position.y + y, position.z + z))
    };
    let model = make(ClassName::Model, "Character", None, Vec::new());
    let [rx, ry, rz] = ROOT_PART_SIZE;
    let root = make(
        ClassName::Part,
        "HumanoidRootPart",
        Some(model),
        vec![
            ("Size", Value::Vector3(Vector3::new(rx, ry, rz))),
            ("Position", at(0.0, ry * 0.5, 0.0)),
            ("Transparency", Value::Number(1.0)),
            ("CastShadow", Value::Bool(false)),
        ],
    );
    let (size, middle) = body_box(MODEL_MIN, MODEL_MAX);
    let body = make(
        ClassName::MeshPart,
        "Body",
        Some(model),
        vec![
            (
                "MeshId",
                Value::String(DEFAULT_HUMANOID_MODEL_PATH.to_owned()),
            ),
            ("Size", Value::Vector3(size)),
            ("Position", at(middle.x, middle.y, middle.z)),
            ("Orientation", Value::Vector3(Vector3::new(0.0, 180.0, 0.0))),
            ("CanCollide", Value::Bool(false)),
            ("CanQuery", Value::Bool(false)),
            ("CanTouch", Value::Bool(false)),
            ("Massless", Value::Bool(true)),
        ],
    );
    make(
        ClassName::Animator,
        "Animator",
        Some(body),
        vec![(
            "StateMachine",
            Value::String(DEFAULT_HUMANOID_STATE_MACHINE_PATH.to_owned()),
        )],
    );
    let sockets: Vec<(InstanceId, &str)> = HUMANOID_SOCKETS
        .iter()
        .map(|(socket, bone)| {
            (
                make(ClassName::Attachment, socket, Some(body), Vec::new()),
                *bone,
            )
        })
        .collect();
    make(
        ClassName::WeldConstraint,
        "BodyWeld",
        Some(root),
        vec![
            ("Part0", Value::Instance(Some(root))),
            ("Part1", Value::Instance(Some(body))),
        ],
    );
    // A quarter of WalkSpeed crouched (4 studs/s, 1.4 m/s at this scale),
    // so CrouchWalk's steps keep up at about the pace Walk's do.
    let controller_properties = vec![("CrouchSpeedScale", Value::Number(CROUCH_SPEED_SCALE))];
    for (class, name, properties) in [
        (ClassName::Humanoid, "Humanoid", Vec::new()),
        (
            ClassName::ThirdPersonController,
            "ThirdPersonController",
            controller_properties,
        ),
        (ClassName::Health, "Health", Vec::new()),
        (ClassName::Inventory, "Inventory", Vec::new()),
    ] {
        make(class, name, Some(model), properties);
    }
    for (socket, bone) in sockets {
        world
            .set_attribute(
                socket,
                axcenity_anim::SOCKET_BONE,
                Some(Value::String(bone.to_owned())),
            )
            .expect("an Attachment takes attributes");
    }
    model
}

#[cfg(test)]
mod tests {
    use super::*;
    use axcenity_anim::files::Motion;
    use axcenity_anim::{ClipFile, StateMachineFile};

    /// The JSON chunk of a GLB file.
    fn glb_json(bytes: &[u8]) -> serde_json::Value {
        let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        serde_json::from_slice(&bytes[20..20 + length]).unwrap()
    }

    #[test]
    fn the_model_box_matches_the_file() {
        let json = glb_json(DEFAULT_HUMANOID_MODEL);
        let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
        for mesh in json["meshes"].as_array().unwrap() {
            for primitive in mesh["primitives"].as_array().unwrap() {
                let index = primitive["attributes"]["POSITION"].as_u64().unwrap() as usize;
                let accessor = &json["accessors"][index];
                for axis in 0..3 {
                    min[axis] = min[axis].min(accessor["min"][axis].as_f64().unwrap() as f32);
                    max[axis] = max[axis].max(accessor["max"][axis].as_f64().unwrap() as f32);
                }
            }
        }
        for axis in 0..3 {
            assert!((min[axis] - MODEL_MIN[axis]).abs() < 0.002, "{min:?}");
            assert!((max[axis] - MODEL_MAX[axis]).abs() < 0.002, "{max:?}");
        }
    }

    #[test]
    fn every_state_plays_a_take_the_animation_file_has_and_the_files_read() {
        let takes = axcenity_assets::load_takes(
            DEFAULT_HUMANOID_ANIMATIONS,
            axcenity_assets::ModelFormat::Gltf,
        )
        .unwrap();
        let files = default_humanoid_animation_files();
        assert_eq!(files.len(), STATES.len() + 1);
        for (path, text) in &files[..STATES.len()] {
            let clip = ClipFile::from_json(text).unwrap_or_else(|err| panic!("{path}: {err}"));
            let link = clip.link.unwrap();
            assert!(takes.iter().any(|take| take.name == link.take), "{path}");
            assert!(clip.duration > 0.1, "{path}");
        }
        let machine = StateMachineFile::from_json(&files.last().unwrap().1).unwrap();
        // The states the Humanoid plays, and those Insert > Character has.
        for state in ["Idle", "Walk", "Run", "Jump", "Fall", "Sit"] {
            assert!(machine.state(state).is_some(), "{state}");
        }
        // Walk blends walking into running by the Humanoid's Speed.
        let Motion::BlendTree(tree) = &machine.state("Walk").unwrap().motion else {
            panic!("Walk is a blend tree");
        };
        assert_eq!(tree.parameter, "Speed");
        let thresholds: Vec<f64> = tree.children.iter().map(|c| c.threshold).collect();
        assert!((thresholds[0] - 2.77).abs() < 0.01, "{thresholds:?}");
        assert_eq!(thresholds[1..], [16.0, 32.0]);
        assert!(tree.children[2].speed > tree.children[1].speed);
        // Every take of the file can be played by a state, but the
        // versions with root motion (clips of a creator's own).
        assert_eq!(takes.len(), 86);
        let played: std::collections::BTreeSet<&str> =
            STATES.iter().map(|state| state.take).collect();
        let unplayed: Vec<&str> = takes
            .iter()
            .map(|take| take.name.as_str())
            .filter(|name| !played.contains(name))
            .collect();
        assert_eq!(unplayed, ["Run_RootMotion", "Walk_RootMotion"]);
        let names: std::collections::BTreeSet<&str> =
            STATES.iter().map(|state| state.name).collect();
        assert_eq!(names.len(), STATES.len(), "no state twice");
        // A state that goes on to another names one the machine has, and
        // only a state whose take doesn't loop goes anywhere.
        for state in &STATES {
            let made = machine.state(state.name).unwrap();
            match state.end.next() {
                Some(next) => {
                    assert!(machine.state(next).is_some(), "{} to {next}", state.name);
                    assert_eq!(made.returns.as_ref().unwrap().to, next, "{}", state.name);
                }
                None => assert!(made.returns.is_none(), "{}", state.name),
            }
        }
    }

    /// A take the library made to lead into another ends in the pose the
    /// other starts in, and one made to be played from a stance starts and
    /// ends in it: where a state goes when its take has played is where
    /// the body already is, within what the blend covers. (Each of these
    /// held its last frame or went to Idle before 2026-11-25: a pistol
    /// shot dropped the arms, a call from the rail stood away from it.)
    #[test]
    fn a_state_goes_on_to_the_state_its_take_ends_in() {
        let model = axcenity_assets::load_model(
            DEFAULT_HUMANOID_MODEL,
            Path::new(""),
            axcenity_assets::ModelFormat::Gltf,
        )
        .unwrap();
        let rig = model.rig.as_deref().unwrap();
        let takes = axcenity_assets::load_takes(
            DEFAULT_HUMANOID_ANIMATIONS,
            axcenity_assets::ModelFormat::Gltf,
        )
        .unwrap();
        let take = |state: &str| {
            let state = STATES.iter().find(|s| s.name == state).unwrap();
            takes.iter().find(|take| take.name == state.take).unwrap()
        };
        let pose = |take: &axcenity_assets::Take, time: f32| {
            let mut pose = rig.rest_pose();
            take.sample(time, &take.bind(rig), rig, &mut pose);
            pose
        };
        let mut checked = 0;
        for state in &STATES {
            let End::GoesTo(next) = state.end else {
                continue;
            };
            let from = take(state.name);
            let (last, first) = (pose(from, from.duration), pose(take(next), 0.0));
            let mut worst = (0.0_f32, "");
            for (index, bone) in rig.bones.iter().enumerate() {
                let turn = last[index]
                    .rotation
                    .angle_between(first[index].rotation)
                    .to_degrees();
                if turn > worst.0 {
                    worst = (turn, bone.name.as_str());
                }
            }
            assert!(
                worst.0 < 25.0,
                "{} ends {:.1} degrees from where {next} starts ({})",
                state.name,
                worst.0,
                worst.1
            );
            checked += 1;
        }
        assert_eq!(checked, 15);
    }

    #[test]
    fn adding_it_writes_the_prefab_and_its_files_and_keeps_changed_ones() {
        let folder = std::env::temp_dir().join(format!(
            "axcenity-default-humanoid-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        assert_eq!(
            add_default_humanoid(&folder).unwrap(),
            DEFAULT_CHARACTER_PREFAB_PATH
        );
        for path in [
            DEFAULT_HUMANOID_MODEL_PATH,
            DEFAULT_HUMANOID_ANIMATIONS_PATH,
            DEFAULT_HUMANOID_STATE_MACHINE_PATH,
            "assets/animations/Humanoid/Walk.anim.json",
        ] {
            assert!(folder.join(path).is_file(), "{path}");
        }
        // The prefab is one Model: the root part, the welded mesh with its
        // Animator, and the character's pieces.
        let mut world = World::new();
        let workspace = world.workspace();
        let roots = crate::Prefabs::in_folder(&folder)
            .instantiate(DEFAULT_CHARACTER_PREFAB_PATH, &mut world, workspace)
            .unwrap();
        let [model] = roots.as_slice() else {
            panic!("{roots:?}");
        };
        for name in [
            "HumanoidRootPart",
            "Body",
            "Humanoid",
            "ThirdPersonController",
            "Health",
            "Inventory",
        ] {
            assert!(world.find_first_child(*model, name).is_some(), "{name}");
        }
        let body = world.find_first_child(*model, "Body").unwrap();
        assert!(world.find_first_child(body, "Animator").is_some());
        // A creator's change is kept.
        let walk = folder.join("assets/animations/Humanoid/Walk.anim.json");
        fs::write(&walk, "changed").unwrap();
        add_default_humanoid(&folder).unwrap();
        assert_eq!(fs::read_to_string(&walk).unwrap(), "changed");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn refreshing_it_replaces_an_earlier_builds_files_and_adding_it_keeps_them() {
        let folder = std::env::temp_dir().join(format!(
            "axcenity-default-humanoid-refresh-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        // A folder as an earlier build left it: another animation file,
        // another state machine and a clip of another length.
        let stale = [
            DEFAULT_HUMANOID_ANIMATIONS_PATH,
            DEFAULT_HUMANOID_MODEL_PATH,
            DEFAULT_HUMANOID_STATE_MACHINE_PATH,
            "assets/animations/Humanoid/Idle.anim.json",
            DEFAULT_CHARACTER_PREFAB_PATH,
        ];
        for path in stale {
            write_missing(&folder, path, b"an earlier build's").unwrap();
        }
        add_default_humanoid(&folder).unwrap();
        for path in stale {
            assert_eq!(
                fs::read(folder.join(path)).unwrap(),
                b"an earlier build's",
                "{path}: a project's file is kept"
            );
        }
        assert_eq!(
            refresh_default_humanoid(&folder).unwrap(),
            DEFAULT_CHARACTER_PREFAB_PATH
        );
        assert_eq!(
            fs::read(folder.join(DEFAULT_HUMANOID_ANIMATIONS_PATH)).unwrap(),
            DEFAULT_HUMANOID_ANIMATIONS
        );
        assert_eq!(
            fs::read(folder.join(DEFAULT_HUMANOID_MODEL_PATH)).unwrap(),
            DEFAULT_HUMANOID_MODEL
        );
        for (path, text) in default_humanoid_animation_files() {
            assert_eq!(
                fs::read_to_string(folder.join(&path)).unwrap(),
                text,
                "{path}"
            );
        }
        let prefab = fs::read_to_string(folder.join(DEFAULT_CHARACTER_PREFAB_PATH)).unwrap();
        assert!(prefab.contains("HumanoidRootPart"), "the engine's prefab");
        // Nothing is written when nothing differs.
        let file = folder.join(DEFAULT_HUMANOID_ANIMATIONS_PATH);
        let written = fs::metadata(&file).unwrap().modified().unwrap();
        refresh_default_humanoid(&folder).unwrap();
        assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), written);
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn its_feet_are_at_the_position_and_it_faces_minus_z() {
        let mut world = World::new();
        let model = default_character(&mut world, Vector3::new(4.0, 10.0, -2.0));
        let property = |name: &str, what: &str| {
            let id = world.find_first_child(model, name).unwrap();
            world
                .get(id)
                .unwrap()
                .property(what)
                .unwrap()
                .as_vector3()
                .unwrap()
        };
        let (root, root_size) = (
            property("HumanoidRootPart", "Position"),
            property("HumanoidRootPart", "Size"),
        );
        assert!((root.y - root_size.y * 0.5 - 10.0).abs() < 1e-4);
        assert!((root_size.x - 0.6 * STUDS_PER_METRE).abs() < 1e-4);
        let (body, body_size) = (property("Body", "Position"), property("Body", "Size"));
        assert!(
            (body.y - body_size.y * 0.5 - 10.0).abs() < 1e-4,
            "feet on the ground"
        );
        assert!((body_size.y - 1.75 * STUDS_PER_METRE).abs() < 1e-3);
        assert_eq!(
            property("Body", "Orientation"),
            Vector3::new(0.0, 180.0, 0.0)
        );
    }
}
