//! Scratch-style script data and its one-way translation to Lua: what a
//! game needs at runtime, so the player can load block scripts and UI
//! Event Graphs without the block editor's tools (those, such as
//! `from_lua`, are in `axcenity-blocks`, which re-exports this crate).
//!
//! A [`BlockScript`] is a canvas of stacks. Each stack starts with an event
//! block (a "hat", such as "when game starts") whose body runs when the
//! event happens, or is an [`BlockKind::Unattached`] stack of loose blocks
//! that doesn't run, like blocks left lying around in Scratch. Statement
//! blocks hold values in slots: plain values typed in (numbers, text, ...)
//! or reporter blocks ([`Expression`]) that work a value out.
//!
//! Every block translates to the documented Lua API (see `docs/scripting/blocks.md`),
//! so a block script runs exactly like a typed one. The Graph view shows
//! the same blocks as nodes and wires, laid out by [`Graph`].

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod behavior_tree;
pub mod catalogue;
pub mod dialogue;
pub mod node_graph;
mod translate;
pub mod ui_graph;

pub use translate::*;

static NEXT_BLOCK_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockShape {
    /// Starts a stack ("when ..."): only at the top level.
    Event,
    /// One step in a stack.
    Statement,
    /// A round reporter: a number, text or other value.
    Value,
    /// A pointed reporter: true or false.
    Boolean,
}

impl BlockShape {
    /// Whether a block of shape `child` fits where a block of shape `self`
    /// goes. As in Scratch, a true-or-false reporter also fits a value slot,
    /// but only true-or-false reporters fit a condition.
    pub fn accepts(self, child: BlockShape) -> bool {
        matches!(
            (self, child),
            (BlockShape::Event, BlockShape::Event)
                | (BlockShape::Statement, BlockShape::Statement)
                | (BlockShape::Value, BlockShape::Value | BlockShape::Boolean)
                | (BlockShape::Boolean, BlockShape::Boolean)
        )
    }
}

/// Which side of the game a block script runs on: blocks that read the
/// keyboard or UI clicks only work on the player's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Server,
    Client,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockScript {
    pub blocks: Vec<Block>,
    /// How the script looks as a node graph. Written only once it's been
    /// shown as one, so other block scripts don't change.
    #[serde(default, skip_serializing_if = "Graph::is_empty")]
    pub graph: Graph,
    /// Declared variables with their optional initial value, scope and live
    /// debug watcher flag.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<VariableDef>,
    /// Indent the Lua with tabs, as the typed script it came from was.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tabs: bool,
}

fn default_variable_initial() -> Expression {
    Expression::Number { value: 0.0 }
}

/// Metadata for a script variable declared in the Variables palette.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableDef {
    pub name: String,
    #[serde(default = "default_variable_initial")]
    pub initial: Expression,
    /// When false, declared as a script-local `local <name>`. When true,
    /// stored in `_G.<name>` so other scripts can share it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub global: bool,
    /// When true, the editor/Play HUD watches this variable's live value.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub watched: bool,
}

impl VariableDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            initial: default_variable_initial(),
            global: false,
            watched: false,
        }
    }
}

/// A reference to a scene Instance: either a static dotted path picked from
/// the scene dropdown (serializes as a plain JSON string for backwards
/// compatibility) or a dynamic [`Expression`] wired in from another node or
/// dropped as a reporter pill.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectRef(pub Box<Expression>);

impl ObjectRef {
    pub fn path(path: impl Into<String>) -> Self {
        Self(Box::new(Expression::ObjectPath { path: path.into() }))
    }

    pub fn expr(expression: Expression) -> Self {
        Self(Box::new(expression))
    }

    pub fn as_path(&self) -> Option<&str> {
        match self.0.as_ref() {
            Expression::ObjectPath { path } => Some(path.as_str()),
            _ => None,
        }
    }

    pub fn as_expr(&self) -> &Expression {
        &self.0
    }

    pub fn as_expr_mut(&mut self) -> &mut Expression {
        &mut self.0
    }

    pub fn is_empty_path(&self) -> bool {
        matches!(self.0.as_ref(), Expression::ObjectPath { path } if path.is_empty())
    }
}

impl Default for ObjectRef {
    fn default() -> Self {
        Self::path(String::new())
    }
}

impl From<String> for ObjectRef {
    fn from(path: String) -> Self {
        Self::path(path)
    }
}

impl From<&str> for ObjectRef {
    fn from(path: &str) -> Self {
        Self::path(path)
    }
}

impl Serialize for ObjectRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0.as_ref() {
            Expression::ObjectPath { path } => serializer.serialize_str(path),
            other => other.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ObjectRef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Path(String),
            Expr(Expression),
        }
        match Repr::deserialize(deserializer)? {
            Repr::Path(path) => Ok(Self::path(path)),
            Repr::Expr(expr) => Ok(Self::expr(expr)),
        }
    }
}

/// The node-graph view of a block script (see `docs/scripting/graphs.md`).
/// The blocks are the script; this is only where the nodes are, which view
/// is shown, and value nodes wired to nothing, so it never changes the Lua.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    /// The script is shown as a node graph rather than as blocks.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shown: bool,
    /// Where nodes have been placed, in graph units, by node key: a block's
    /// id (`"12"`), or a value node's owner id and slots (`"12.0.1"`).
    /// Nodes without one are laid out after the node they're wired to.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub nodes: BTreeMap<String, [f32; 2]>,
    /// Value nodes wired to nothing. They don't run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loose: Vec<LooseValue>,
}

impl Graph {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A value node wired to nothing, with its own id (from the blocks' ids).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LooseValue {
    pub id: u64,
    pub value: Expression,
}

impl LooseValue {
    pub fn new(value: Expression) -> Self {
        Self {
            id: NEXT_BLOCK_ID.fetch_add(1, Ordering::Relaxed),
            value,
        }
    }
}

impl BlockScript {
    /// The block with `id`, wherever it is.
    pub fn find_mut(&mut self, id: u64) -> Option<&mut Block> {
        fn find(blocks: &mut [Block], id: u64) -> Option<&mut Block> {
            for block in blocks {
                if block.id == id {
                    return Some(block);
                }
                for body in block.kind.bodies_mut() {
                    if let Some(found) = find(body, id) {
                        return Some(found);
                    }
                }
            }
            None
        }
        find(&mut self.blocks, id)
    }

    /// The stack holding the block with `id`, and where in it the block is.
    pub fn stack_of_mut(&mut self, id: u64) -> Option<(&mut Vec<Block>, usize)> {
        fn find(blocks: &mut Vec<Block>, id: u64) -> Option<(&mut Vec<Block>, usize)> {
            if let Some(index) = blocks.iter().position(|block| block.id == id) {
                return Some((blocks, index));
            }
            for block in blocks.iter_mut() {
                for body in block.kind.bodies_mut() {
                    if let Some(found) = find(body, id) {
                        return Some(found);
                    }
                }
            }
            None
        }
        find(&mut self.blocks, id)
    }

    /// Calls `visit` for every block in the script, stacks and bodies alike,
    /// parents before their bodies.
    pub fn for_each_block(&self, visit: &mut impl FnMut(&Block)) {
        fn walk(blocks: &[Block], visit: &mut impl FnMut(&Block)) {
            for block in blocks {
                visit(block);
                for body in block.kind.bodies() {
                    walk(body, visit);
                }
            }
        }
        walk(&self.blocks, visit);
    }
}

/// Makes sure blocks made from now on get IDs that `script` doesn't use.
pub fn reserve_block_ids(script: &BlockScript) {
    let mut maximum = 0;
    script.for_each_block(&mut |block| maximum = maximum.max(block.id));
    for loose in &script.graph.loose {
        maximum = maximum.max(loose.id);
    }
    NEXT_BLOCK_ID.fetch_max(maximum.saturating_add(1), Ordering::Relaxed);
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub id: u64,
    pub kind: BlockKind,
    /// Where a stack's top is on the editor's canvas, for top-level blocks.
    /// `None` lays it out below the stack before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f32; 2]>,
    /// A blank line goes before this block's Lua, as in the typed script.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub spaced: bool,
    /// Written joined to what's before it: an if as the `elseif` of the else
    /// it's alone in, or a comment at the end of the last line.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub chained: bool,
}

impl Block {
    pub fn new(kind: BlockKind) -> Self {
        Self {
            id: NEXT_BLOCK_ID.fetch_add(1, Ordering::Relaxed),
            kind,
            position: None,
            spaced: false,
            chained: false,
        }
    }

    /// A copy with new IDs, for duplicating blocks.
    pub fn duplicate(&self) -> Self {
        let mut copy = self.clone();
        copy.renumber();
        copy
    }

    fn renumber(&mut self) {
        self.id = NEXT_BLOCK_ID.fetch_add(1, Ordering::Relaxed);
        for body in self.kind.bodies_mut() {
            for block in body {
                block.renumber();
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockKind {
    // Events: the top of a stack.
    WhenStarted {
        body: Vec<Block>,
    },
    WhenTouched {
        part: ObjectRef,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        filter: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter_value: Option<Expression>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cooldown: Option<Expression>,
        body: Vec<Block>,
    },
    /// When a part stops touching `part` (`TouchEnded`).
    WhenTouchEnds {
        part: ObjectRef,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        filter: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter_value: Option<Expression>,
        body: Vec<Block>,
    },
    /// World streaming brought `target` (or a Model's contents) back
    /// (Phase 80).
    WhenStreamedIn {
        target: ObjectRef,
        body: Vec<Block>,
    },
    /// World streaming removed `target` (or a Model's contents) from this
    /// device (`target.StreamedOut`).
    WhenStreamedOut {
        target: ObjectRef,
        body: Vec<Block>,
    },
    /// Client only.
    WhenKeyPressed {
        key: String,
        body: Vec<Block>,
    },
    /// When a key is released (`UserInputService.InputEnded`). Client only.
    WhenKeyReleased {
        key: String,
        body: Vec<Block>,
    },
    /// When a player joins the game (`Players.PlayerAdded`).
    WhenPlayerJoins {
        body: Vec<Block>,
    },
    /// When a player leaves the game (`Players.PlayerRemoving`).
    WhenPlayerLeaves {
        body: Vec<Block>,
    },
    /// When any player's character spawns (`player.CharacterAdded`).
    WhenCharacterSpawns {
        body: Vec<Block>,
    },
    /// When any player's character dies (`Humanoid.Died` / `Health.Died`).
    WhenPlayerDies {
        body: Vec<Block>,
    },
    /// When a child is added to `target` (the Workspace when empty).
    WhenChildAdded {
        target: ObjectRef,
        body: Vec<Block>,
    },
    /// When a child is removed from `target` (the Workspace when empty).
    WhenChildRemoved {
        target: ObjectRef,
        body: Vec<Block>,
    },
    /// When `tag` is added to any Instance (`CollectionService:GetInstanceAddedSignal`).
    WhenTagAdded {
        tag: Expression,
        body: Vec<Block>,
    },
    /// When `tag` is removed from any Instance (`CollectionService:GetInstanceRemovedSignal`).
    WhenTagRemoved {
        tag: Expression,
        body: Vec<Block>,
    },
    /// When a RemoteEvent fires (`OnServerEvent` or `OnClientEvent`).
    WhenRemoteFires {
        remote: ObjectRef,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        side: String,
        body: Vec<Block>,
    },
    /// When a TextButton is clicked or tapped. Client only.
    WhenClicked {
        button: ObjectRef,
        body: Vec<Block>,
    },
    /// When the pointer comes onto a UI element (`MouseEnter`). Client only.
    WhenMouseEnters {
        element: ObjectRef,
        body: Vec<Block>,
    },
    /// When the pointer goes off a UI element (`MouseLeave`). Client only.
    WhenMouseLeaves {
        element: ObjectRef,
        body: Vec<Block>,
    },
    /// When a Slider, Toggle, Dropdown or ProgressBar's `Value` changes
    /// (`ValueChanged`): dragged, clicked, picked or set by a script. The
    /// body reads the new value as the widget's `Value` property.
    WhenValueChanged {
        widget: ObjectRef,
        body: Vec<Block>,
    },
    /// Runs every frame (`RunService.Heartbeat`).
    EveryFrame {
        body: Vec<Block>,
    },
    /// Runs every frame before physics (`RunService.Stepped`).
    BeforePhysics {
        body: Vec<Block>,
    },
    /// Runs every frame just before it's drawn (`RunService.RenderStepped`).
    /// Client only.
    BeforeRender {
        body: Vec<Block>,
    },
    /// When a BindableEvent fires (`Event`): `event` is its path in
    /// ReplicatedStorage.
    WhenEventFired {
        event: String,
        body: Vec<Block>,
    },
    WhenSoundEnded {
        sound: ObjectRef,
        body: Vec<Block>,
    },
    /// When a Sequence reaches its end, or is skipped there (`Completed`).
    WhenCutsceneFinished {
        sequence: ObjectRef,
        body: Vec<Block>,
    },
    /// When hosting or joining fails, a join is refused or the connection
    /// is lost (`NetworkService.ConnectionFailed`, Phase 63).
    WhenConnectionFails {
        body: Vec<Block>,
    },
    /// When a chat message arrives on this device
    /// (`TextChatService.MessageReceived`, Phase 67).
    WhenChatMessageReceived {
        body: Vec<Block>,
    },
    /// When an Animator moves to another state (`StateChanged`).
    WhenStateChanged {
        animator: ObjectRef,
        body: Vec<Block>,
    },
    /// When an Animator's clip passes the event named `event`
    /// (`AnimationEvent`). Empty or `"Any"` fires for every event.
    WhenAnimationEvent {
        animator: ObjectRef,
        event: String,
        body: Vec<Block>,
    },
    /// When a loaded Animation finishes playing on an Animator (`track.Stopped`).
    WhenAnimationFinished {
        animator: ObjectRef,
        animation: ObjectRef,
        body: Vec<Block>,
    },
    /// When a NavigationAgent starts seeing or hearing the player character
    /// (`PlayerSensed`), `sense` being one of [`SENSES`].
    WhenAgentSenses {
        agent: ObjectRef,
        sense: String,
        body: Vec<Block>,
    },
    /// When a NavigationAgent stops seeing or hearing the player character
    /// (`PlayerLost`), `sense` being one of [`SENSES`].
    WhenAgentLosesPlayer {
        agent: ObjectRef,
        sense: String,
        body: Vec<Block>,
    },
    /// When a NavigationAgent reaches its `Destination` (`DestinationReached`).
    WhenAgentArrives {
        agent: ObjectRef,
        body: Vec<Block>,
    },
    /// When a SplineFollower gets to an end of its spline (`Reached`),
    /// `which` being one of [`SPLINE_ENDS`] (Phase 73).
    WhenSplineReached {
        follower: ObjectRef,
        which: String,
        body: Vec<Block>,
    },
    /// When a character sits on a Seat: its `Occupant` changes to a
    /// controller (not to nil).
    WhenSeatOccupied {
        seat: ObjectRef,
        body: Vec<Block>,
    },
    /// When a character leaves a Seat: its `Occupant` changes from a
    /// controller to nil.
    WhenSeatVacated {
        seat: ObjectRef,
        body: Vec<Block>,
    },
    /// When the data layer `layer` is switched on or off
    /// (`DataLayerService.LayerChanged`, roadmap R16.4), on both sides.
    WhenLayerChanged {
        data_layer: String,
        body: Vec<Block>,
    },
    /// When one property of an object changes
    /// (`GetPropertyChangedSignal`), whoever changed it.
    WhenPropertyChanged {
        target: ObjectRef,
        property: String,
        body: Vec<Block>,
    },
    /// When one attribute of an object changes, is added or removed
    /// (`GetAttributeChangedSignal`).
    WhenAttributeChanged {
        target: ObjectRef,
        attribute: String,
        body: Vec<Block>,
    },
    /// When a ProximityPrompt is triggered: its action pressed, or held
    /// for its `HoldDuration` (Phase 44).
    WhenPromptTriggered {
        prompt: ObjectRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cooldown: Option<Expression>,
        body: Vec<Block>,
    },
    /// When a ClickDetector's part is clicked (`MouseClick`).
    WhenPartClicked {
        detector: ObjectRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cooldown: Option<Expression>,
        body: Vec<Block>,
    },
    /// When `command` is typed in the Developer Console (Phase 71):
    /// `ConsoleService:RegisterCommand(command, help, function() ... end)`.
    /// A LocalScript's runs on that device, a Script's on the server.
    WhenCommandRun {
        command: Expression,
        help: Expression,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        admin_only: Option<Expression>,
        body: Vec<Block>,
    },
    /// Disconnects a signal connection (`connection:Disconnect()`).
    Disconnect {
        connection: Expression,
    },
    /// Loose blocks on the canvas, under no event. They don't run.
    Unattached {
        body: Vec<Block>,
    },
    /// Lua at the top of the script that has no blocks, kept word for
    /// word from a script switched from Code to Blocks. It runs where it is.
    TopCode {
        code: String,
    },

    // Control.
    Wait {
        seconds: Expression,
    },
    WaitUntil {
        condition: Expression,
    },
    /// Waits until an object's event next fires (`signal:Wait()`), such as
    /// a part's `Touched`.
    WaitForEvent {
        target: ObjectRef,
        event: String,
    },
    Repeat {
        count: Expression,
        body: Vec<Block>,
    },
    Forever {
        body: Vec<Block>,
    },
    RepeatUntil {
        condition: Expression,
        body: Vec<Block>,
    },
    If {
        condition: Expression,
        body: Vec<Block>,
    },
    IfElse {
        condition: Expression,
        then_body: Vec<Block>,
        else_body: Vec<Block>,
    },
    /// Ends the event's handler early.
    StopScript,
    /// Disconnects all script-tracked connections and destroys this script (`script:Destroy()`).
    DestroyScript,
    /// Numeric range loop (`for i = start, end, step do ... end`).
    ForRange {
        start: Expression,
        end: Expression,
        step: Expression,
        body: Vec<Block>,
    },
    /// Iterates over every element of a list (`for i, item in ipairs(list) do ... end`).
    ForEachInList {
        list: Expression,
        body: Vec<Block>,
    },
    /// Iterates over every Instance with `tag` (`CollectionService:GetTagged(tag)`).
    ForEachTagged {
        tag: Expression,
        body: Vec<Block>,
    },
    /// Runs multiple execution branches in sequence (`Then 0`, `Then 1`, ...).
    Sequence {
        branches: Vec<Vec<Block>>,
    },
    /// Rate-limiting execution gate (`Cooldown (s) = 0` runs once; `> 0` debounces via `os.clock()`).
    CooldownGate {
        cooldown: Expression,
        body: Vec<Block>,
    },
    /// Runs `body` asynchronously without blocking (`task.spawn` or `task.delay`).
    SpawnAsync {
        delay: Expression,
        body: Vec<Block>,
    },
    /// Runs `body` once for each child of `target` (the Workspace when
    /// empty), with the child in the variable `variable`.
    ForEachChild {
        target: ObjectRef,
        variable: String,
        body: Vec<Block>,
    },
    /// Runs `body` once for each part whose bounding box is within `radius`
    /// studs of `center` (`workspace:GetPartBoundsInRadius`), with the part
    /// in the variable `variable`.
    ForEachPartInRadius {
        center: Expression,
        radius: Expression,
        variable: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        players_only: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ignore: Option<Expression>,
        body: Vec<Block>,
    },
    /// Runs `body` once for each part overlapping `target`, such as the
    /// parts inside a trigger zone (`workspace:GetPartsInPart`), with the
    /// part in the variable `variable`.
    ForEachPartTouching {
        target: ObjectRef,
        variable: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        players_only: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ignore: Option<Expression>,
        body: Vec<Block>,
    },
    /// Waits until `target` (the Workspace when empty) has a child `name`,
    /// with an optional `timeout` in seconds.
    WaitForChild {
        target: ObjectRef,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<Expression>,
    },
    /// Destroys `target` after `seconds`, without waiting
    /// (`Debris:AddItem`).
    DestroyAfter {
        target: ObjectRef,
        seconds: Expression,
    },

    // Motion.
    MoveTo {
        target: ObjectRef,
        position: Expression,
    },
    MoveBy {
        target: ObjectRef,
        offset: Expression,
    },
    /// Turns `target` to face `position`: `CFrame.lookAt`.
    LookAt {
        target: ObjectRef,
        position: Expression,
    },
    /// Moves `target` along the way it faces (its `CFrame.LookVector`).
    MoveForward {
        target: ObjectRef,
        distance: Expression,
    },
    TurnTo {
        target: ObjectRef,
        orientation: Expression,
    },
    TurnBy {
        target: ObjectRef,
        rotation: Expression,
        #[serde(default = "default_space")]
        space: String,
    },
    /// Smoothly slides/glides `target` to `position` over `duration` seconds
    /// using `TweenService` (`CFrame`).
    SlideTo {
        target: ObjectRef,
        position: Expression,
        duration: Expression,
        #[serde(default = "default_easing_style")]
        easing: String,
        #[serde(default)]
        wait_until_done: bool,
        #[serde(default)]
        completed: Vec<Block>,
    },
    /// Smoothly rotates `target` to `rotation` (degrees) over `duration`
    /// seconds using `TweenService` (`CFrame`).
    RotateTo {
        target: ObjectRef,
        rotation: Expression,
        duration: Expression,
        #[serde(default = "default_easing_style")]
        easing: String,
        #[serde(default)]
        wait_until_done: bool,
        #[serde(default)]
        completed: Vec<Block>,
    },
    /// Sets a gravity multiplier on `target` via a `VectorForce` (`0` = zero
    /// gravity / anti-gravity, `1` = normal gravity).
    SetGravity {
        target: ObjectRef,
        multiplier: Expression,
    },
    /// Sets `target`'s `AssemblyAngularVelocity` in radians per second.
    SetAngularVelocity {
        target: ObjectRef,
        velocity: Expression,
    },
    /// Launches or bounces `target` along `direction` at `speed` studs/sec.
    LaunchPart {
        target: ObjectRef,
        direction: Expression,
        speed: Expression,
    },
    SetAnchored {
        target: ObjectRef,
        anchored: Expression,
    },
    /// Pushes `target` with an impulse: `ApplyImpulse`, which changes its
    /// velocity by `force` over its mass.
    PushWithForce {
        target: ObjectRef,
        force: Expression,
    },
    /// Throws `target`: sets its `AssemblyLinearVelocity`.
    SetPartVelocity {
        target: ObjectRef,
        velocity: Expression,
    },
    /// Sets `target`'s friction, keeping its other physical properties:
    /// its `CustomPhysicalProperties` from its `CurrentPhysicalProperties`.
    SetFriction {
        target: ObjectRef,
        friction: Expression,
    },

    // Looks.
    SetColor {
        target: ObjectRef,
        color: Expression,
    },
    SetText {
        target: ObjectRef,
        text: Expression,
    },
    Show {
        target: ObjectRef,
    },
    Hide {
        target: ObjectRef,
    },
    /// Sets a PageLayout's current page to a Frame's name.
    ShowPage {
        layout: ObjectRef,
        page: Expression,
    },
    /// Scrolls a ScrollingFrame: sets its `CanvasPosition` to `x`, `y` pixels.
    ScrollTo {
        target: ObjectRef,
        x: Expression,
        y: Expression,
    },
    /// Sets a Slider, Toggle, Dropdown or ProgressBar's `Value`, which
    /// moves it and fires its `ValueChanged`.
    SetValue {
        target: ObjectRef,
        value: Expression,
    },
    SetProperty {
        target: ObjectRef,
        property: String,
        value: Expression,
    },
    /// Sets one of an object's attributes (`SetAttribute`); nil removes it.
    SetAttribute {
        target: ObjectRef,
        attribute: String,
        value: Expression,
    },

    // Navigation & Seats & Splines.
    /// Walks a NavigationAgent along the navigation mesh to `position`: sets
    /// its `Destination`.
    SetDestination {
        agent: ObjectRef,
        position: Expression,
    },
    /// Directs a NavigationAgent to chase/follow `target`'s current position.
    FollowTarget {
        agent: ObjectRef,
        target: ObjectRef,
    },
    /// Stops a NavigationAgent immediately (`Velocity = Vector3.new(0, 0, 0)`).
    StopAgent {
        agent: ObjectRef,
    },
    /// Sits a character controller in a `Seat`: `seat:Sit(controller)`.
    SitInSeat {
        seat: ObjectRef,
        controller: ObjectRef,
    },
    /// Stands a character controller's character up from its seat: sets
    /// its `Sit` to false.
    LeaveSeat {
        controller: ObjectRef,
    },
    /// Moves a NavigationAgent straight at `velocity` studs a second, with
    /// no path: sets its `Velocity`. Zero stops it where it is.
    SetVelocity {
        agent: ObjectRef,
        velocity: Expression,
    },
    /// Starts, stops, or reverses a `SplineFollower` (`action` in [`SPLINE_ACTIONS`]).
    ControlSpline {
        follower: ObjectRef,
        action: String,
    },
    /// Sets a `SplineFollower`'s `Speed` in studs a second.
    SetSplineSpeed {
        follower: ObjectRef,
        speed: Expression,
    },

    // Saving: `SaveService`.
    /// Writes save slot `slot`: player data, world state, the active
    /// checkpoint and settings.
    SaveSlot {
        slot: Expression,
    },
    /// Puts the game back as save slot `slot` was saved.
    LoadSlot {
        slot: Expression,
    },
    /// Deletes save slot `slot` (`SaveService:DeleteSlot`).
    DeleteSlot {
        slot: Expression,
    },
    /// Iterates every row in a data table (`DataService:GetTable(t):GetRows()`).
    ForEachRow {
        table: String,
        body: Vec<Block>,
    },
    /// Makes a Checkpoint the active one, capturing the game there.
    SetCheckpoint {
        checkpoint: ObjectRef,
    },
    /// Returns to the active checkpoint: what was captured there, with the
    /// player character standing on it.
    RestoreCheckpoint,

    // Levels: `LevelService` (Phase 48).
    /// Puts the level `level` in place of the Workspace, with characters
    /// at the Instance called `spawn` there (empty text: a SpawnLocation).
    LoadLevel {
        level: Expression,
        spawn: Expression,
    },
    /// Copies the level `level` into a Model beside the current level.
    AddLevel {
        level: Expression,
    },
    /// Takes the added level `level` out again.
    RemoveLevel {
        level: Expression,
    },

    // Multiplayer: `NetworkService` (Phase 63).
    /// Hosts the game on UDP port `port`, for other players to join.
    HostGame {
        port: Expression,
    },
    /// Joins the game at `address` (`"192.168.1.20:47800"`).
    JoinGame {
        address: Expression,
    },
    /// Leaves the game joined, or stops hosting.
    LeaveGame,
    /// Gives `target`'s assembly to `owner` (a Player, or nil for the
    /// server) to simulate: `SetNetworkOwner` (Phase 66).
    SetNetworkOwner {
        target: ObjectRef,
        owner: Expression,
    },
    /// Lets the engine choose who simulates `target`'s assembly: the
    /// nearest player: `SetNetworkOwnershipAuto` (Phase 66).
    SetNetworkOwnershipAuto {
        target: ObjectRef,
    },
    /// Takes `player` out of the game, telling them `reason`:
    /// `Player:Kick` (Phase 67).
    KickPlayer {
        player: Expression,
        reason: Expression,
    },
    /// Loads what's around `position` for `player` and waits for it:
    /// `Player:RequestStreamAroundAsync` (Phase 80).
    RequestStreamAround {
        player: Expression,
        position: Expression,
    },
    /// Sends `text` to the chat as this device's player:
    /// `TextChatService:SendMessage` (LocalScripts, Phase 67).
    SendChatMessage {
        text: Expression,
    },
    /// Shows `text` in the chat as the game's:
    /// `TextChatService:DisplaySystemMessage` (Phase 67).
    ShowSystemMessage {
        text: Expression,
    },
    /// Switches the data layer `layer` on or off:
    /// `DataLayerService:SetLayerActive` (Scripts only, roadmap R16.4).
    SetLayerActive {
        data_layer: String,
        active: Expression,
    },

    // Camera: `workspace.CurrentCamera` (LocalScripts only).
    /// Sets `CameraType` to one of [`CAMERA_TYPES`]: Scriptable leaves the
    /// camera to scripts, Custom gives it back to the controller.
    SetCameraType {
        camera_type: String,
    },
    /// Turns the camera to look at `position`, from where it is.
    PointCameraAt {
        position: Expression,
    },
    /// `Camera:Shake(intensity, seconds)`.
    ShakeCamera {
        intensity: Expression,
        seconds: Expression,
    },
    /// Places a decal that fades out after `seconds`
    /// (`workspace:SpawnDecal`): the picture `texture` laid on what's at
    /// `position`, facing `facing` (a surface's normal), `size` studs.
    SpawnDecal {
        texture: Expression,
        position: Expression,
        facing: Expression,
        size: Expression,
        seconds: Expression,
    },

    // Localization: `LocalizationService`.
    /// Sets the language bound UI text shows (`LocalizationService.Language`).
    SetLanguage {
        language: Expression,
    },

    // Lighting: the world's look.
    /// Sets `Lighting.ToneMapping` to one of [`TONE_MAPPINGS`].
    SetToneMapping {
        curve: String,
    },
    /// Sets `Lighting.ClockTime`: the time of day, in hours.
    /// When `duration` is set, tweens `ClockTime` smoothly over that many seconds.
    SetClockTime {
        time: Expression,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<Expression>,
    },
    /// Sets one of [`ATMOSPHERE_NUMBERS`] on Lighting's Atmosphere (the
    /// child named Atmosphere). When `duration` is set, tweens it smoothly.
    SetAtmosphere {
        property: String,
        value: Expression,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<Expression>,
    },
    /// Toggles visibility of a GUI element (`Visible`) or Part (`Transparency`).
    ToggleVisibility {
        target: ObjectRef,
    },
    /// Sets transparency (`Transparency`, `TextTransparency`, `ImageTransparency`,
    /// or `BackgroundTransparency` depending on class).
    SetTransparency {
        target: ObjectRef,
        transparency: Expression,
    },
    /// Sets a BasePart's `Material` to one of [`MATERIALS`].
    SetMaterial {
        target: ObjectRef,
        material: String,
    },
    /// Emits `count` particles immediately from a `ParticleEmitter`.
    EmitParticles {
        target: ObjectRef,
        count: Expression,
    },
    /// Sets `.Enabled` on a `ParticleEmitter`, Light, or post-processing effect.
    SetEffectEnabled {
        target: ObjectRef,
        enabled: Expression,
    },
    /// Sets one of [`LIGHTING_PROPERTIES`] on `Lighting`.
    SetLightingProperty {
        property: String,
        value: Expression,
    },
    /// Sets a property on a post-processing effect inside `Lighting`.
    SetPostProcessing {
        effect: String,
        property: String,
        value: Expression,
    },
    /// Tweens any property of an object over `duration` seconds with `easing`.
    TweenProperty {
        target: ObjectRef,
        property: String,
        value: Expression,
        duration: Expression,
        #[serde(default = "default_easing_style")]
        easing: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        completed: Vec<Block>,
    },
    /// Sets `.Image` (on `ImageLabel`/`ImageButton`) or `.Texture` (on `Decal`/`ProjectedDecal`/`ParticleEmitter`).
    SetImage {
        target: ObjectRef,
        image: Expression,
    },

    // Graphics: the player's computer's settings (LocalScripts only).
    /// Sets `UserGameSettings.GraphicsQuality` to one of
    /// [`GRAPHICS_QUALITIES`].
    SetGraphicsQuality {
        quality: String,
    },
    /// Sets one of [`GRAPHICS_OPTIONS`] or [`GRAPHICS_NUMBERS`] in
    /// `UserGameSettings`. For choice/switch settings uses `choice` ([`PRESET`]
    /// clears the override); for numeric settings uses `value`.
    SetGraphicsOption {
        setting: String,
        choice: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<Expression>,
    },
    /// Sets one of [`GRAPHICS_NUMBERS`] in `UserGameSettings`.
    /// Kept for backwards-compatible deserialization; unified into
    /// `SetGraphicsOption` in the palette.
    SetGraphicsNumber {
        setting: String,
        value: Expression,
    },

    // Sound.
    PlaySound {
        target: ObjectRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pitch: Option<Expression>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
    },
    PlaySoundAtPosition {
        target: ObjectRef,
        position: Expression,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        volume: Option<Expression>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pitch: Option<Expression>,
    },
    StopSound {
        target: ObjectRef,
    },
    PauseSound {
        target: ObjectRef,
    },
    ResumeSound {
        target: ObjectRef,
    },
    SetVolume {
        target: ObjectRef,
        volume: Expression,
    },
    FadeVolume {
        target: ObjectRef,
        volume: Expression,
        duration: Expression,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
    },
    SetSoundPitch {
        target: ObjectRef,
        pitch: Expression,
    },
    SetSoundLooping {
        target: ObjectRef,
        looped: Expression,
    },
    SetSoundTime {
        target: ObjectRef,
        time: Expression,
    },

    // Animation: an Animator's parameters and states.
    SetAnimatorBool {
        animator: ObjectRef,
        parameter: String,
        value: Expression,
    },
    SetAnimatorNumber {
        animator: ObjectRef,
        parameter: String,
        value: Expression,
    },
    SetAnimatorTrigger {
        animator: ObjectRef,
        parameter: String,
    },
    ResetAnimatorTrigger {
        animator: ObjectRef,
        parameter: String,
    },
    PlayAnimatorState {
        animator: ObjectRef,
        state: String,
    },
    /// How much a state machine layer counts, faded over `seconds`
    /// (`animator:SetLayerWeight(layer, weight, seconds)`, Phase 78).
    SetAnimatorLayerWeight {
        animator: ObjectRef,
        layer: String,
        weight: Expression,
        seconds: Expression,
    },
    /// Loads an Animation on an Animator and plays the track
    /// (`animator:LoadAnimation(animation):Play(fadeTime, weight, speed)`).
    PlayAnimation {
        animator: ObjectRef,
        animation: ObjectRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fade_time: Option<Expression>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        speed: Option<Expression>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        weight: Option<Expression>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
    },
    /// Stops one Animation (or all playing tracks when `animation` is `None`)
    /// on an Animator, optionally fading out over `fade_time` seconds.
    StopAnimation {
        animator: ObjectRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        animation: Option<ObjectRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fade_time: Option<Expression>,
    },
    /// Sets an Animator's playback speed (`animator.Speed = speed`).
    SetAnimatorSpeed {
        animator: ObjectRef,
        speed: Expression,
    },
    /// Sets an IKControl's target instance (`ikControl.Target = target`) and
    /// optional blend weight (`ikControl.Weight = weight`).
    SetIkTarget {
        ik_control: ObjectRef,
        target: ObjectRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        weight: Option<Expression>,
    },
    /// Enables or disables a Ragdoll (`ragdoll.Enabled = enabled`).
    SetRagdollEnabled {
        ragdoll: ObjectRef,
        enabled: Expression,
    },
    /// Plays a Sequence (`sequence:Play()`, Phase 51).
    PlayCutscene {
        sequence: ObjectRef,
    },
    /// Pauses a playing Sequence (`sequence:Pause()`).
    PauseCutscene {
        sequence: ObjectRef,
    },
    /// Resumes a paused Sequence (`sequence:Play()`).
    ResumeCutscene {
        sequence: ObjectRef,
    },
    /// Stops a Sequence and resets it (`sequence:Stop()`).
    StopCutscene {
        sequence: ObjectRef,
    },
    /// Puts an object `distance` studs along a Spline, facing along it
    /// (`target:PivotTo(spline:GetCFrameAt(distance))`, Phase 73).
    PlaceOnSpline {
        target: ObjectRef,
        spline: ObjectRef,
        distance: Expression,
    },
    /// Has a NavigationAgent walk a Spline's points in order
    /// (`agent:Patrol(spline)`).
    Patrol {
        agent: ObjectRef,
        spline: ObjectRef,
    },

    // Tweens: a property moving smoothly (`TweenService`).
    /// Moves one property of an object to `value` over `seconds`, eased by
    /// `style` (one of [`EASING_STYLES`]) and `direction` (one of
    /// [`EASING_DIRECTIONS`]).
    Tween {
        target: ObjectRef,
        property: String,
        value: Expression,
        seconds: Expression,
        style: String,
        direction: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repeat_count: Option<Expression>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        reverses: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        completed: Vec<Block>,
    },
    /// Slides a UI element in to its cached rest position (`_RestPos`) from one
    /// parent's width or height past an edge, `from` being one of [`SLIDE_EDGES`].
    SlideIn {
        target: ObjectRef,
        from: String,
        seconds: Expression,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        completed: Vec<Block>,
    },
    /// Slides a UI element out to one parent's width or height past an edge,
    /// `to` being one of [`SLIDE_EDGES`], then hides it (`Visible = false`).
    SlideOut {
        target: ObjectRef,
        to: String,
        seconds: Expression,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        completed: Vec<Block>,
    },
    /// Fades a UI element or BasePart in (sets `Visible = true` for UI).
    FadeIn {
        target: ObjectRef,
        seconds: Expression,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        completed: Vec<Block>,
    },
    /// Fades a UI element or BasePart out (sets `Visible = false` after UI fade).
    FadeOut {
        target: ObjectRef,
        seconds: Expression,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        completed: Vec<Block>,
    },
    /// Smoothly moves a Model or BasePart's pivot to `position` (Vector3 or
    /// CFrame) over `duration` seconds using a `CFrameValue` proxy.
    TweenPivot {
        target: ObjectRef,
        position: Expression,
        duration: Expression,
        easing: String,
        direction: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        completed: Vec<Block>,
    },
    /// Briefly scales a BasePart or GuiObject up by `scale` and reverses back
    /// over `duration` seconds.
    PulseScale {
        target: ObjectRef,
        scale: Expression,
        duration: Expression,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        wait_until_done: bool,
    },
    /// Cancels a running Tween (`tween:Cancel()`).
    CancelTween {
        tween: ObjectRef,
    },

    // Variables & Lists.
    SetVariable {
        name: String,
        value: Expression,
    },
    ChangeVariable {
        name: String,
        by: Expression,
    },
    /// Creates or resets a list variable (`name = { ... }`).
    ListCreate {
        name: String,
        #[serde(default)]
        items: Vec<Expression>,
    },
    /// Appends `value` to `list` (`table.insert(list, value)`).
    ListAdd {
        list: Expression,
        value: Expression,
    },
    /// Inserts `value` at 1-based `index` in `list` (`table.insert(list, index, value)`).
    ListInsert {
        list: Expression,
        index: Expression,
        value: Expression,
    },
    /// Removes item at 1-based `index` from `list` (`table.remove(list, index)`).
    ListRemoveIndex {
        list: Expression,
        index: Expression,
    },
    /// Removes the first occurrence of `value` from `list`.
    ListRemoveValue {
        list: Expression,
        value: Expression,
    },
    /// Removes all items from `list` (`table.clear(list)`).
    ListClear {
        list: Expression,
    },

    // Other.
    Print {
        value: Expression,
    },
    Destroy {
        target: ObjectRef,
    },
    /// Copies `target` and puts the copy in `parent` (the Workspace when empty).
    /// Supports optional batch `count`, `position`, `offset_per_clone` and
    /// `per_clone_positions`, binding `<Object>_cloned` (and `<Object>_clones`
    /// when batching).
    CloneInto {
        target: ObjectRef,
        parent: ObjectRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count: Option<Expression>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        position: Option<Expression>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offset_per_clone: Option<Expression>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        per_clone_positions: Option<Expression>,
    },
    /// Fires a BindableEvent (`event:Fire(data?)`), by its path in ReplicatedStorage.
    FireEvent {
        event: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<Expression>,
    },

    // Luau itself, which any typed script is made of (see
    // `docs/scripting/blocks.md`). Names, parameters and types are text, as
    // written, so `a: number, ...` is one parameter list.
    /// The statements at the script's top level, in order: not an event,
    /// and written without anything around them.
    RunScript {
        body: Vec<Block>,
    },
    /// `signal:Connect(function(params) ... end)` at the top level.
    OnSignal {
        signal: Expression,
        method: String,
        params: String,
        body: Vec<Block>,
    },
    /// `signal:Connect(function(params) ... end)` among other blocks.
    Connect {
        signal: Expression,
        method: String,
        params: String,
        body: Vec<Block>,
    },
    /// One or more `--` comments, as written.
    Comment {
        text: String,
    },
    /// `local names = values`.
    Local {
        names: String,
        values: Vec<Expression>,
    },
    /// `targets = values`.
    Assign {
        targets: Vec<Expression>,
        values: Vec<Expression>,
    },
    /// `target += value`; `op` is the operator without the `=`.
    CompoundAssign {
        target: Expression,
        op: String,
        value: Expression,
    },
    /// `function(args)` as a statement.
    CallFunction {
        function: Expression,
        args: Vec<Expression>,
    },
    /// `object:method(args)` as a statement.
    CallMethod {
        object: Expression,
        method: String,
        args: Vec<Expression>,
    },
    /// `local function name(params) ... end`, or `function name(params) ...
    /// end` with a name such as `a.b:c`.
    DefineFunction {
        name: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        local: bool,
        params: String,
        /// `<T>`, as written.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        generics: String,
        /// `: number`, as written.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        returns: String,
        /// `@native`, as written.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        attributes: String,
        body: Vec<Block>,
    },
    /// `while condition do ... end`.
    While {
        condition: Expression,
        body: Vec<Block>,
    },
    /// `repeat ... until condition`.
    RepeatLoop {
        body: Vec<Block>,
        condition: Expression,
    },
    /// `for variable = start, stop, step do ... end`.
    NumericFor {
        variable: String,
        start: Expression,
        stop: Expression,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<Expression>,
        body: Vec<Block>,
    },
    /// `for names in values do ... end`.
    GenericFor {
        names: String,
        values: Vec<Expression>,
        body: Vec<Block>,
    },
    /// `do ... end`.
    DoBlock {
        body: Vec<Block>,
    },
    Break,
    Continue,
    /// `return values`.
    Return {
        values: Vec<Expression>,
    },

    /// Lua that has no blocks, kept word for word from a script switched
    /// from Code to Blocks.
    Code {
        code: String,
    },
}

/// The binary operators Luau has, which [`Expression::Binary`] can be.
pub const LUA_BINARY_OPS: [&str; 16] = [
    "+", "-", "*", "/", "//", "%", "^", "..", "==", "~=", "<", "<=", ">", ">=", "and", "or",
];
/// Luau's unary operators.
pub const LUA_UNARY_OPS: [&str; 3] = ["not", "-", "#"];
/// The operators a compound assignment can use.
pub const LUA_COMPOUND_OPS: [&str; 8] = ["+", "-", "*", "/", "//", "%", "^", ".."];
/// The ways to listen to a signal.
pub const SIGNAL_METHODS: [&str; 3] = ["Connect", "Once", "ConnectParallel"];

/// `Enum.EasingStyle`'s items, which the tween block offers.
pub const EASING_STYLES: [&str; 7] = [
    "Linear", "Sine", "Quad", "Cubic", "Back", "Bounce", "Elastic",
];
/// `Enum.EasingDirection`'s items.
pub const EASING_DIRECTIONS: [&str; 3] = ["In", "Out", "InOut"];
/// The edges "slide in" can come from.
pub const SLIDE_EDGES: [&str; 4] = ["Left", "Right", "Top", "Bottom"];
/// `Enum.CameraType`'s items, which the camera type block offers.
pub const CAMERA_TYPES: [&str; 5] = ["Scriptable", "Custom", "Follow", "Track", "Fixed"];
/// `Enum.ToneMapping`'s items, which the tone mapping block offers.
pub const TONE_MAPPINGS: [&str; 3] = ["Classic", "Neutral", "Filmic"];
/// The Atmosphere numbers the atmosphere block sets.
pub const ATMOSPHERE_NUMBERS: [&str; 4] = ["Density", "Offset", "Glare", "Haze"];
/// `Enum.GraphicsQuality`'s items, which the graphics quality block offers.
pub const GRAPHICS_QUALITIES: [&str; 5] = ["Automatic", "Low", "Medium", "High", "Ultra"];
/// The `UserGameSettings` overrides the graphics option block sets.
pub const GRAPHICS_OPTIONS: [&str; 21] = [
    "ShadowQuality",
    "AntiAliasing",
    "TextureQuality",
    "AmbientOcclusion",
    "ReflectionQuality",
    "CloudQuality",
    "DepthOfField",
    "MotionBlur",
    "RayTracedShadows",
    "RayTracedAmbientOcclusion",
    "RayTracedGlobalIllumination",
    "RayTracedReflections",
    "PostProcessing",
    "ContactShadows",
    "RealtimeGI",
    "VolumetricFog",
    "Vignette",
    "FilmGrain",
    "ChromaticAberration",
    "Sharpen",
    "SunRays",
];
/// The [`GRAPHICS_OPTIONS`] that are on or off (booleans in Lua), not enums.
pub const GRAPHICS_SWITCHES: [&str; 9] = [
    "PostProcessing",
    "ContactShadows",
    "RealtimeGI",
    "VolumetricFog",
    "Vignette",
    "FilmGrain",
    "ChromaticAberration",
    "Sharpen",
    "SunRays",
];
/// The `UserGameSettings` overrides the graphics number block sets.
pub const GRAPHICS_NUMBERS: [&str; 2] = ["ShadowDistance", "RenderScale"];
/// Every graphics setting offered by the unified `SetGraphicsOption` block
/// (choice/switch settings plus numeric settings).
pub const ALL_GRAPHICS_SETTINGS: [&str; 23] = [
    "ShadowQuality",
    "AntiAliasing",
    "TextureQuality",
    "AmbientOcclusion",
    "ReflectionQuality",
    "CloudQuality",
    "DepthOfField",
    "MotionBlur",
    "RayTracedShadows",
    "RayTracedAmbientOcclusion",
    "RayTracedGlobalIllumination",
    "RayTracedReflections",
    "PostProcessing",
    "ContactShadows",
    "RealtimeGI",
    "VolumetricFog",
    "Vignette",
    "FilmGrain",
    "ChromaticAberration",
    "Sharpen",
    "SunRays",
    "ShadowDistance",
    "RenderScale",
];
/// `Enum.Material`'s items, offered by `SetMaterial`.
pub const MATERIALS: [&str; 16] = [
    "Plastic",
    "SmoothPlastic",
    "Wood",
    "WoodPlanks",
    "Marble",
    "Granite",
    "Concrete",
    "Slate",
    "Brick",
    "Fabric",
    "Sand",
    "Metal",
    "Glass",
    "Neon",
    "Ice",
    "Rubber",
];
/// Properties of `Lighting` offered by `SetLightingProperty`.
pub const LIGHTING_PROPERTIES: [&str; 17] = [
    "Exposure",
    "EnvironmentIntensity",
    "FogStart",
    "FogEnd",
    "FogColor",
    "BloomIntensity",
    "BloomThreshold",
    "BloomEnabled",
    "ShadowsEnabled",
    "ShadowDistance",
    "ShadowSoftness",
    "AmbientOcclusionIntensity",
    "GeographicLatitude",
    "SkyTopColor",
    "SkyHorizonColor",
    "GroundColor",
    "PostProcessingEnabled",
];
/// Post-processing effect classes inside `Lighting` offered by `SetPostProcessing`.
pub const POST_PROCESSING_EFFECTS: [&str; 9] = [
    "BloomEffect",
    "ColorCorrectionEffect",
    "DepthOfFieldEffect",
    "VignetteEffect",
    "SunRaysEffect",
    "FilmGrainEffect",
    "ChromaticAberrationEffect",
    "SharpenEffect",
    "MotionBlurEffect",
];
/// Properties available on each post-processing effect class.
pub fn post_processing_properties(effect: &str) -> &'static [&'static str] {
    match effect {
        "BloomEffect" => &["Enabled", "Intensity", "Threshold"],
        "ColorCorrectionEffect" => &[
            "Enabled",
            "Brightness",
            "Contrast",
            "Saturation",
            "TintColor",
        ],
        "DepthOfFieldEffect" => &[
            "Enabled",
            "FocusDistance",
            "InFocusRadius",
            "NearIntensity",
            "FarIntensity",
        ],
        "VignetteEffect" => &["Enabled", "Intensity", "Size", "Color"],
        "SunRaysEffect" => &["Enabled", "Intensity", "Spread"],
        "FilmGrainEffect" => &["Enabled", "Intensity", "Size"],
        "ChromaticAberrationEffect" | "SharpenEffect" | "MotionBlurEffect" => {
            &["Enabled", "Intensity"]
        }
        _ => &[],
    }
}
/// Properties readable by `Expression::SoundProperty`.
pub const SOUND_PROPERTIES: [&str; 6] = [
    "Playing",
    "TimePosition",
    "TimeLength",
    "Volume",
    "PlaybackSpeed",
    "Looped",
];
/// Axes selectable in `Expression::VectorAxis`.
pub const VECTOR_AXES: [&str; 3] = ["X", "Y", "Z"];
/// Operations selectable in `Expression::MinMax`.
pub const MIN_MAX_OPS: [&str; 2] = ["min", "max"];
/// The graphics option block's choice that removes the override (nil).
pub const PRESET: &str = "Preset";

/// What the graphics option block can set `setting` (one of
/// [`GRAPHICS_OPTIONS`]) to: its enum's items (On and Off for the
/// [`GRAPHICS_SWITCHES`]), then [`PRESET`]. Empty for any other name.
pub fn graphics_choices(setting: &str) -> &'static [&'static str] {
    match setting {
        "ShadowQuality" => &["Off", "Low", "Medium", "High", "Ultra", PRESET],
        "AntiAliasing" => &["Off", "FXAA", "MSAA2", "MSAA4", "TAA", PRESET],
        "TextureQuality" => &["Quarter", "Half", "Full", PRESET],
        "AmbientOcclusion" | "DepthOfField" | "MotionBlur" => &["Off", "Low", "High", PRESET],
        "ReflectionQuality" | "CloudQuality" => &["Low", "Medium", "High", "Ultra", PRESET],
        "RayTracedShadows"
        | "RayTracedAmbientOcclusion"
        | "RayTracedGlobalIllumination"
        | "RayTracedReflections" => &["Off", "Low", "Medium", "High", PRESET],
        "PostProcessing"
        | "ContactShadows"
        | "RealtimeGI"
        | "VolumetricFog"
        | "Vignette"
        | "FilmGrain"
        | "ChromaticAberration"
        | "Sharpen"
        | "SunRays" => &["On", "Off", PRESET],
        _ => &[],
    }
}

/// The classes the "is a" block offers, abstract ones first.
pub const IS_A_CLASSES: [&str; 16] = [
    "BasePart",
    "GuiObject",
    "Part",
    "MeshPart",
    "Model",
    "Folder",
    "Frame",
    "TextLabel",
    "TextButton",
    "ImageLabel",
    "Sound",
    "Light",
    "ParticleEmitter",
    "Script",
    "LocalScript",
    "ModuleScript",
];

/// The senses a NavigationAgent's `PlayerSensed` / `PlayerLost` names.
pub const SENSES: [&str; 3] = ["Sight", "Hearing", "Any"];

/// The ends a SplineFollower's `Reached` names.
pub const SPLINE_ENDS: [&str; 3] = ["End", "Start", "Any"];

/// Coordinate space choices for `Turn By`.
pub const SPACE_MODES: [&str; 2] = ["Local", "World"];

/// Actions for `Start / Stop / Reverse Spline`.
pub const SPLINE_ACTIONS: [&str; 3] = ["Start", "Stop", "Reverse"];

/// Output field choices for the unified `Raycast` reporter.
pub const RAYCAST_OUTPUTS: [&str; 5] = [
    "Hit Part",
    "Hit Position",
    "Hit Normal",
    "Distance",
    "Result",
];

fn default_space() -> String {
    "Local".to_owned()
}

fn default_easing_style() -> String {
    "Quad".to_owned()
}

/// Filter modes for `When Touched` and `When Touch Ends`.
pub const TOUCH_FILTERS: [&str; 3] = ["Any Part", "Players Only", "Tagged / Named..."];

/// Listening side for `When Remote Fires`.
pub const REMOTE_SIDES: [&str; 2] = ["Server (OnServerEvent)", "Client (OnClientEvent)"];

impl BlockKind {
    pub fn shape(&self) -> BlockShape {
        if self.is_event() {
            BlockShape::Event
        } else {
            BlockShape::Statement
        }
    }

    /// Event blocks, and unattached stacks: what can be at the top level.
    pub fn is_event(&self) -> bool {
        matches!(
            self,
            BlockKind::WhenStarted { .. }
                | BlockKind::WhenTouched { .. }
                | BlockKind::WhenTouchEnds { .. }
                | BlockKind::WhenStreamedIn { .. }
                | BlockKind::WhenStreamedOut { .. }
                | BlockKind::WhenKeyPressed { .. }
                | BlockKind::WhenKeyReleased { .. }
                | BlockKind::WhenPlayerJoins { .. }
                | BlockKind::WhenPlayerLeaves { .. }
                | BlockKind::WhenCharacterSpawns { .. }
                | BlockKind::WhenPlayerDies { .. }
                | BlockKind::WhenChildAdded { .. }
                | BlockKind::WhenChildRemoved { .. }
                | BlockKind::WhenTagAdded { .. }
                | BlockKind::WhenTagRemoved { .. }
                | BlockKind::WhenRemoteFires { .. }
                | BlockKind::WhenClicked { .. }
                | BlockKind::WhenMouseEnters { .. }
                | BlockKind::WhenMouseLeaves { .. }
                | BlockKind::WhenValueChanged { .. }
                | BlockKind::EveryFrame { .. }
                | BlockKind::BeforePhysics { .. }
                | BlockKind::BeforeRender { .. }
                | BlockKind::WhenEventFired { .. }
                | BlockKind::WhenSoundEnded { .. }
                | BlockKind::WhenCutsceneFinished { .. }
                | BlockKind::WhenConnectionFails { .. }
                | BlockKind::WhenChatMessageReceived { .. }
                | BlockKind::WhenStateChanged { .. }
                | BlockKind::WhenAnimationEvent { .. }
                | BlockKind::WhenAnimationFinished { .. }
                | BlockKind::WhenAgentSenses { .. }
                | BlockKind::WhenAgentLosesPlayer { .. }
                | BlockKind::WhenAgentArrives { .. }
                | BlockKind::WhenSplineReached { .. }
                | BlockKind::WhenSeatOccupied { .. }
                | BlockKind::WhenSeatVacated { .. }
                | BlockKind::WhenLayerChanged { .. }
                | BlockKind::WhenPropertyChanged { .. }
                | BlockKind::WhenAttributeChanged { .. }
                | BlockKind::WhenPromptTriggered { .. }
                | BlockKind::WhenPartClicked { .. }
                | BlockKind::WhenCommandRun { .. }
                | BlockKind::Unattached { .. }
                | BlockKind::TopCode { .. }
                | BlockKind::RunScript { .. }
                | BlockKind::OnSignal { .. }
        )
    }

    /// Whether this block only works in client block scripts (LocalScripts).
    pub fn client_only(&self) -> bool {
        matches!(
            self,
            BlockKind::WhenKeyPressed { .. }
                | BlockKind::WhenKeyReleased { .. }
                | BlockKind::WhenClicked { .. }
                | BlockKind::WhenMouseEnters { .. }
                | BlockKind::WhenMouseLeaves { .. }
                | BlockKind::BeforeRender { .. }
                | BlockKind::SetCameraType { .. }
                | BlockKind::PointCameraAt { .. }
                | BlockKind::ShakeCamera { .. }
        )
    }

    /// The stacks of blocks inside this one, in order (an if/else's two),
    /// then the functions written in its value slots: those directly in a
    /// slot first, as they're the ones drawn as mouths on a block.
    pub fn bodies(&self) -> Vec<&Vec<Block>> {
        let mut bodies = match self {
            BlockKind::WhenStarted { body }
            | BlockKind::WhenTouched { body, .. }
            | BlockKind::WhenTouchEnds { body, .. }
            | BlockKind::WhenStreamedIn { body, .. }
            | BlockKind::WhenStreamedOut { body, .. }
            | BlockKind::WhenKeyPressed { body, .. }
            | BlockKind::WhenKeyReleased { body, .. }
            | BlockKind::WhenPlayerJoins { body }
            | BlockKind::WhenPlayerLeaves { body }
            | BlockKind::WhenCharacterSpawns { body }
            | BlockKind::WhenPlayerDies { body }
            | BlockKind::WhenChildAdded { body, .. }
            | BlockKind::WhenChildRemoved { body, .. }
            | BlockKind::WhenTagAdded { body, .. }
            | BlockKind::WhenTagRemoved { body, .. }
            | BlockKind::WhenRemoteFires { body, .. }
            | BlockKind::WhenClicked { body, .. }
            | BlockKind::WhenMouseEnters { body, .. }
            | BlockKind::WhenMouseLeaves { body, .. }
            | BlockKind::WhenValueChanged { body, .. }
            | BlockKind::EveryFrame { body }
            | BlockKind::BeforePhysics { body }
            | BlockKind::BeforeRender { body }
            | BlockKind::WhenEventFired { body, .. }
            | BlockKind::WhenSoundEnded { body, .. }
            | BlockKind::WhenCutsceneFinished { body, .. }
            | BlockKind::WhenConnectionFails { body }
            | BlockKind::WhenChatMessageReceived { body }
            | BlockKind::WhenStateChanged { body, .. }
            | BlockKind::WhenAnimationEvent { body, .. }
            | BlockKind::WhenAnimationFinished { body, .. }
            | BlockKind::WhenAgentSenses { body, .. }
            | BlockKind::WhenAgentLosesPlayer { body, .. }
            | BlockKind::WhenAgentArrives { body, .. }
            | BlockKind::WhenSplineReached { body, .. }
            | BlockKind::WhenSeatOccupied { body, .. }
            | BlockKind::WhenSeatVacated { body, .. }
            | BlockKind::WhenLayerChanged { body, .. }
            | BlockKind::WhenPropertyChanged { body, .. }
            | BlockKind::WhenAttributeChanged { body, .. }
            | BlockKind::WhenPromptTriggered { body, .. }
            | BlockKind::WhenPartClicked { body, .. }
            | BlockKind::WhenCommandRun { body, .. }
            | BlockKind::SlideTo {
                completed: body, ..
            }
            | BlockKind::RotateTo {
                completed: body, ..
            }
            | BlockKind::TweenProperty {
                completed: body, ..
            }
            | BlockKind::Tween {
                completed: body, ..
            }
            | BlockKind::SlideIn {
                completed: body, ..
            }
            | BlockKind::SlideOut {
                completed: body, ..
            }
            | BlockKind::FadeIn {
                completed: body, ..
            }
            | BlockKind::FadeOut {
                completed: body, ..
            }
            | BlockKind::TweenPivot {
                completed: body, ..
            }
            | BlockKind::Unattached { body }
            | BlockKind::Repeat { body, .. }
            | BlockKind::Forever { body }
            | BlockKind::RepeatUntil { body, .. }
            | BlockKind::ForRange { body, .. }
            | BlockKind::ForEachInList { body, .. }
            | BlockKind::ForEachTagged { body, .. }
            | BlockKind::CooldownGate { body, .. }
            | BlockKind::SpawnAsync { body, .. }
            | BlockKind::ForEachChild { body, .. }
            | BlockKind::ForEachRow { body, .. }
            | BlockKind::ForEachPartInRadius { body, .. }
            | BlockKind::ForEachPartTouching { body, .. }
            | BlockKind::If { body, .. }
            | BlockKind::RunScript { body }
            | BlockKind::OnSignal { body, .. }
            | BlockKind::Connect { body, .. }
            | BlockKind::DefineFunction { body, .. }
            | BlockKind::While { body, .. }
            | BlockKind::RepeatLoop { body, .. }
            | BlockKind::NumericFor { body, .. }
            | BlockKind::GenericFor { body, .. }
            | BlockKind::DoBlock { body } => vec![body],
            BlockKind::IfElse {
                then_body,
                else_body,
                ..
            } => vec![then_body, else_body],
            BlockKind::Sequence { branches } => branches.iter().collect(),
            _ => Vec::new(),
        };
        let slots = self.expressions();
        bodies.extend(slots.iter().filter_map(|slot| match slot {
            Expression::Closure { body, .. } => Some(body),
            _ => None,
        }));
        for slot in slots {
            if !matches!(slot, Expression::Closure { .. }) {
                slot.closure_bodies(&mut bodies);
            }
        }
        bodies
    }

    /// Like [`BlockKind::bodies`], for changing them.
    pub fn bodies_mut(&mut self) -> Vec<&mut Vec<Block>> {
        // The slots of the kinds that also have stacks, which are separate
        // fields of the variant, so both can be borrowed at once.
        let (mut bodies, slots): (Vec<&mut Vec<Block>>, Vec<&mut Expression>) = match self {
            BlockKind::Repeat { count: e, body }
            | BlockKind::RepeatUntil { condition: e, body }
            | BlockKind::ForEachInList { list: e, body }
            | BlockKind::ForEachTagged { tag: e, body }
            | BlockKind::CooldownGate { cooldown: e, body }
            | BlockKind::SpawnAsync { delay: e, body }
            | BlockKind::WhenTagAdded { tag: e, body }
            | BlockKind::WhenTagRemoved { tag: e, body }
            | BlockKind::If { condition: e, body }
            | BlockKind::OnSignal {
                signal: e, body, ..
            }
            | BlockKind::Connect {
                signal: e, body, ..
            }
            | BlockKind::While { condition: e, body }
            | BlockKind::RepeatLoop { condition: e, body } => (vec![body], vec![e]),
            BlockKind::SlideTo {
                target,
                position,
                duration,
                completed,
                ..
            } => (
                vec![completed],
                vec![target.as_expr_mut(), position, duration],
            ),
            BlockKind::RotateTo {
                target,
                rotation,
                duration,
                completed,
                ..
            } => (
                vec![completed],
                vec![target.as_expr_mut(), rotation, duration],
            ),
            BlockKind::TweenProperty {
                target,
                value,
                duration,
                completed,
                ..
            } => (vec![completed], vec![target.as_expr_mut(), value, duration]),
            BlockKind::Tween {
                target,
                value,
                seconds,
                repeat_count,
                completed,
                ..
            } => {
                let mut slots = vec![target.as_expr_mut(), value, seconds];
                slots.extend(repeat_count.as_mut());
                (vec![completed], slots)
            }
            BlockKind::SlideIn {
                target,
                seconds,
                completed,
                ..
            }
            | BlockKind::SlideOut {
                target,
                seconds,
                completed,
                ..
            }
            | BlockKind::FadeIn {
                target,
                seconds,
                completed,
                ..
            }
            | BlockKind::FadeOut {
                target,
                seconds,
                completed,
                ..
            } => (vec![completed], vec![target.as_expr_mut(), seconds]),
            BlockKind::TweenPivot {
                target,
                position,
                duration,
                completed,
                ..
            } => (
                vec![completed],
                vec![target.as_expr_mut(), position, duration],
            ),
            BlockKind::ForRange {
                start,
                end,
                step,
                body,
            } => (vec![body], vec![start, end, step]),
            BlockKind::IfElse {
                condition,
                then_body,
                else_body,
            } => (vec![then_body, else_body], vec![condition]),
            BlockKind::Sequence { branches } => (branches.iter_mut().collect(), Vec::new()),
            BlockKind::ForEachPartInRadius {
                center,
                radius,
                ignore,
                body,
                ..
            } => {
                let mut slots = vec![radius, center];
                slots.extend(ignore.as_mut());
                (vec![body], slots)
            }
            BlockKind::ForEachPartTouching {
                target,
                ignore,
                body,
                ..
            } => {
                let mut slots = vec![target.as_expr_mut()];
                slots.extend(ignore.as_mut());
                (vec![body], slots)
            }
            BlockKind::WhenTouched {
                part,
                filter_value,
                cooldown,
                body,
                ..
            } => {
                let mut slots = vec![part.as_expr_mut()];
                slots.extend(filter_value.as_mut());
                slots.extend(cooldown.as_mut());
                (vec![body], slots)
            }
            BlockKind::WhenTouchEnds {
                part,
                filter_value,
                body,
                ..
            } => {
                let mut slots = vec![part.as_expr_mut()];
                slots.extend(filter_value.as_mut());
                (vec![body], slots)
            }
            BlockKind::WhenPromptTriggered {
                prompt,
                cooldown,
                body,
            } => {
                let mut slots = vec![prompt.as_expr_mut()];
                slots.extend(cooldown.as_mut());
                (vec![body], slots)
            }
            BlockKind::WhenPartClicked {
                detector,
                cooldown,
                body,
            } => {
                let mut slots = vec![detector.as_expr_mut()];
                slots.extend(cooldown.as_mut());
                (vec![body], slots)
            }
            BlockKind::WhenCommandRun {
                command,
                help,
                admin_only,
                body,
            } => {
                let mut slots = vec![command, help];
                slots.extend(admin_only.as_mut());
                (vec![body], slots)
            }
            BlockKind::NumericFor {
                start,
                stop,
                step,
                body,
                ..
            } => {
                let mut slots = vec![start, stop];
                slots.extend(step.as_mut());
                (vec![body], slots)
            }
            BlockKind::GenericFor { values, body, .. } => (vec![body], values.iter_mut().collect()),
            BlockKind::WhenStarted { body }
            | BlockKind::WhenStreamedIn { body, .. }
            | BlockKind::WhenStreamedOut { body, .. }
            | BlockKind::WhenKeyPressed { body, .. }
            | BlockKind::WhenKeyReleased { body, .. }
            | BlockKind::WhenPlayerJoins { body }
            | BlockKind::WhenPlayerLeaves { body }
            | BlockKind::WhenCharacterSpawns { body }
            | BlockKind::WhenPlayerDies { body }
            | BlockKind::WhenChildAdded { body, .. }
            | BlockKind::WhenChildRemoved { body, .. }
            | BlockKind::WhenRemoteFires { body, .. }
            | BlockKind::WhenClicked { body, .. }
            | BlockKind::WhenMouseEnters { body, .. }
            | BlockKind::WhenMouseLeaves { body, .. }
            | BlockKind::WhenValueChanged { body, .. }
            | BlockKind::EveryFrame { body }
            | BlockKind::BeforePhysics { body }
            | BlockKind::BeforeRender { body }
            | BlockKind::WhenEventFired { body, .. }
            | BlockKind::WhenSoundEnded { body, .. }
            | BlockKind::WhenCutsceneFinished { body, .. }
            | BlockKind::WhenConnectionFails { body }
            | BlockKind::WhenChatMessageReceived { body }
            | BlockKind::WhenStateChanged { body, .. }
            | BlockKind::WhenAnimationEvent { body, .. }
            | BlockKind::WhenAnimationFinished { body, .. }
            | BlockKind::WhenAgentSenses { body, .. }
            | BlockKind::WhenAgentLosesPlayer { body, .. }
            | BlockKind::WhenAgentArrives { body, .. }
            | BlockKind::WhenSplineReached { body, .. }
            | BlockKind::WhenSeatOccupied { body, .. }
            | BlockKind::WhenSeatVacated { body, .. }
            | BlockKind::WhenLayerChanged { body, .. }
            | BlockKind::WhenPropertyChanged { body, .. }
            | BlockKind::WhenAttributeChanged { body, .. }
            | BlockKind::Unattached { body }
            | BlockKind::Forever { body }
            | BlockKind::ForEachChild { body, .. }
            | BlockKind::ForEachRow { body, .. }
            | BlockKind::RunScript { body }
            | BlockKind::DefineFunction { body, .. }
            | BlockKind::DoBlock { body } => (vec![body], Vec::new()),
            other => (Vec::new(), other.expressions_mut()),
        };
        let (direct, nested): (Vec<_>, Vec<_>) = slots
            .into_iter()
            .partition(|slot| matches!(slot, Expression::Closure { .. }));
        for slot in direct {
            if let Expression::Closure { body, .. } = slot {
                bodies.push(body);
            }
        }
        for slot in nested {
            slot.closure_bodies_mut(&mut bodies);
        }
        bodies
    }

    /// Like [`BlockKind::expressions`], for changing them.
    pub fn expressions_mut(&mut self) -> Vec<&mut Expression> {
        match self {
            BlockKind::WhenStreamedIn { target: o, .. }
            | BlockKind::WhenStreamedOut { target: o, .. }
            | BlockKind::WhenChildAdded { target: o, .. }
            | BlockKind::WhenChildRemoved { target: o, .. }
            | BlockKind::WhenRemoteFires { remote: o, .. }
            | BlockKind::WhenClicked { button: o, .. }
            | BlockKind::WhenMouseEnters { element: o, .. }
            | BlockKind::WhenMouseLeaves { element: o, .. }
            | BlockKind::WhenValueChanged { widget: o, .. }
            | BlockKind::WhenSoundEnded { sound: o, .. }
            | BlockKind::WhenCutsceneFinished { sequence: o, .. }
            | BlockKind::WhenStateChanged { animator: o, .. }
            | BlockKind::WhenAnimationEvent { animator: o, .. }
            | BlockKind::WhenAgentSenses { agent: o, .. }
            | BlockKind::WhenAgentLosesPlayer { agent: o, .. }
            | BlockKind::WhenAgentArrives { agent: o, .. }
            | BlockKind::WhenSplineReached { follower: o, .. }
            | BlockKind::WhenSeatOccupied { seat: o, .. }
            | BlockKind::WhenSeatVacated { seat: o, .. }
            | BlockKind::WhenPropertyChanged { target: o, .. }
            | BlockKind::WhenAttributeChanged { target: o, .. }
            | BlockKind::WaitForEvent { target: o, .. }
            | BlockKind::ForEachChild { target: o, .. }
            | BlockKind::Show { target: o }
            | BlockKind::Hide { target: o }
            | BlockKind::ToggleVisibility { target: o }
            | BlockKind::SetMaterial { target: o, .. }
            | BlockKind::StopAgent { agent: o }
            | BlockKind::LeaveSeat { controller: o }
            | BlockKind::ControlSpline { follower: o, .. }
            | BlockKind::SetCheckpoint { checkpoint: o }
            | BlockKind::SetNetworkOwnershipAuto { target: o }
            | BlockKind::StopSound { target: o }
            | BlockKind::PauseSound { target: o }
            | BlockKind::ResumeSound { target: o }
            | BlockKind::SetAnimatorTrigger { animator: o, .. }
            | BlockKind::ResetAnimatorTrigger { animator: o, .. }
            | BlockKind::PlayAnimatorState { animator: o, .. }
            | BlockKind::PlayCutscene { sequence: o }
            | BlockKind::PauseCutscene { sequence: o }
            | BlockKind::ResumeCutscene { sequence: o }
            | BlockKind::StopCutscene { sequence: o }
            | BlockKind::CancelTween { tween: o }
            | BlockKind::Destroy { target: o } => vec![o.as_expr_mut()],
            BlockKind::PlaySound {
                target: o,
                pitch: opt,
                ..
            } => {
                let mut slots = vec![o.as_expr_mut()];
                slots.extend(opt.as_mut());
                slots
            }
            BlockKind::PlaySoundAtPosition {
                target: o,
                position: p,
                volume,
                pitch,
            } => {
                let mut slots = vec![o.as_expr_mut(), p];
                slots.extend(volume.as_mut());
                slots.extend(pitch.as_mut());
                slots
            }
            BlockKind::WhenTouched {
                part,
                filter_value,
                cooldown,
                ..
            } => {
                let mut slots = vec![part.as_expr_mut()];
                slots.extend(filter_value.as_mut());
                slots.extend(cooldown.as_mut());
                slots
            }
            BlockKind::WhenTouchEnds {
                part, filter_value, ..
            } => {
                let mut slots = vec![part.as_expr_mut()];
                slots.extend(filter_value.as_mut());
                slots
            }
            BlockKind::WhenPromptTriggered {
                prompt: o,
                cooldown: opt,
                ..
            }
            | BlockKind::WhenPartClicked {
                detector: o,
                cooldown: opt,
                ..
            }
            | BlockKind::WaitForChild {
                target: o,
                timeout: opt,
                ..
            }
            | BlockKind::ForEachPartTouching {
                target: o,
                ignore: opt,
                ..
            } => {
                let mut slots = vec![o.as_expr_mut()];
                slots.extend(opt.as_mut());
                slots
            }
            BlockKind::FireEvent { data, .. } => data.iter_mut().collect(),
            BlockKind::Wait { seconds: e }
            | BlockKind::WaitUntil { condition: e }
            | BlockKind::Repeat { count: e, .. }
            | BlockKind::RepeatUntil { condition: e, .. }
            | BlockKind::ForEachInList { list: e, .. }
            | BlockKind::ForEachTagged { tag: e, .. }
            | BlockKind::CooldownGate { cooldown: e, .. }
            | BlockKind::SpawnAsync { delay: e, .. }
            | BlockKind::WhenTagAdded { tag: e, .. }
            | BlockKind::WhenTagRemoved { tag: e, .. }
            | BlockKind::Disconnect { connection: e }
            | BlockKind::If { condition: e, .. }
            | BlockKind::IfElse { condition: e, .. }
            | BlockKind::SaveSlot { slot: e }
            | BlockKind::LoadSlot { slot: e }
            | BlockKind::DeleteSlot { slot: e }
            | BlockKind::AddLevel { level: e }
            | BlockKind::RemoveLevel { level: e }
            | BlockKind::HostGame { port: e }
            | BlockKind::JoinGame { address: e }
            | BlockKind::SendChatMessage { text: e }
            | BlockKind::ShowSystemMessage { text: e }
            | BlockKind::SetLayerActive { active: e, .. }
            | BlockKind::SetLanguage { language: e }
            | BlockKind::SetVariable { value: e, .. }
            | BlockKind::ChangeVariable { by: e, .. }
            | BlockKind::ListClear { list: e }
            | BlockKind::SetGraphicsNumber { value: e, .. }
            | BlockKind::SetLightingProperty { value: e, .. }
            | BlockKind::SetPostProcessing { value: e, .. }
            | BlockKind::PointCameraAt { position: e }
            | BlockKind::Print { value: e } => vec![e],
            BlockKind::SetGraphicsOption { value, .. } => value.iter_mut().collect(),
            BlockKind::SetClockTime { time: e, duration }
            | BlockKind::SetAtmosphere {
                value: e, duration, ..
            } => {
                let mut slots = vec![e];
                slots.extend(duration.as_mut());
                slots
            }
            BlockKind::SlideTo {
                target: o,
                position: a,
                duration: b,
                ..
            }
            | BlockKind::RotateTo {
                target: o,
                rotation: a,
                duration: b,
                ..
            }
            | BlockKind::TweenProperty {
                target: o,
                value: a,
                duration: b,
                ..
            }
            | BlockKind::FadeVolume {
                target: o,
                volume: a,
                duration: b,
                ..
            }
            | BlockKind::LaunchPart {
                target: o,
                direction: a,
                speed: b,
            } => vec![o.as_expr_mut(), a, b],
            BlockKind::FollowTarget {
                agent: a,
                target: b,
            }
            | BlockKind::SitInSeat {
                seat: a,
                controller: b,
            } => vec![a.as_expr_mut(), b.as_expr_mut()],
            BlockKind::MoveTo {
                target: o,
                position: e,
            }
            | BlockKind::MoveBy {
                target: o,
                offset: e,
            }
            | BlockKind::LookAt {
                target: o,
                position: e,
            }
            | BlockKind::MoveForward {
                target: o,
                distance: e,
            }
            | BlockKind::TurnTo {
                target: o,
                orientation: e,
            }
            | BlockKind::TurnBy {
                target: o,
                rotation: e,
                ..
            }
            | BlockKind::SetGravity {
                target: o,
                multiplier: e,
            }
            | BlockKind::SetAngularVelocity {
                target: o,
                velocity: e,
            }
            | BlockKind::SetSplineSpeed {
                follower: o,
                speed: e,
            }
            | BlockKind::SetAnchored {
                target: o,
                anchored: e,
            }
            | BlockKind::PushWithForce {
                target: o,
                force: e,
            }
            | BlockKind::SetPartVelocity {
                target: o,
                velocity: e,
            }
            | BlockKind::SetFriction {
                target: o,
                friction: e,
            }
            | BlockKind::SetColor {
                target: o,
                color: e,
            }
            | BlockKind::SetTransparency {
                target: o,
                transparency: e,
            }
            | BlockKind::EmitParticles {
                target: o,
                count: e,
            }
            | BlockKind::SetEffectEnabled {
                target: o,
                enabled: e,
            }
            | BlockKind::SetImage {
                target: o,
                image: e,
            }
            | BlockKind::SetSoundPitch {
                target: o,
                pitch: e,
            }
            | BlockKind::SetSoundLooping {
                target: o,
                looped: e,
            }
            | BlockKind::SetSoundTime { target: o, time: e }
            | BlockKind::SetText { target: o, text: e }
            | BlockKind::SetProperty {
                target: o,
                value: e,
                ..
            }
            | BlockKind::SetAttribute {
                target: o,
                value: e,
                ..
            }
            | BlockKind::SetValue {
                target: o,
                value: e,
            }
            | BlockKind::SetDestination {
                agent: o,
                position: e,
            }
            | BlockKind::SetVelocity {
                agent: o,
                velocity: e,
            }
            | BlockKind::SetVolume {
                target: o,
                volume: e,
            }
            | BlockKind::SetAnimatorBool {
                animator: o,
                value: e,
                ..
            }
            | BlockKind::SetAnimatorNumber {
                animator: o,
                value: e,
                ..
            }
            | BlockKind::SetAnimatorSpeed {
                animator: o,
                speed: e,
            }
            | BlockKind::SetRagdollEnabled {
                ragdoll: o,
                enabled: e,
            }
            | BlockKind::SlideIn {
                target: o,
                seconds: e,
                ..
            }
            | BlockKind::SlideOut {
                target: o,
                seconds: e,
                ..
            }
            | BlockKind::FadeIn {
                target: o,
                seconds: e,
                ..
            }
            | BlockKind::FadeOut {
                target: o,
                seconds: e,
                ..
            }
            | BlockKind::DestroyAfter {
                target: o,
                seconds: e,
            } => vec![o.as_expr_mut(), e],
            BlockKind::ShowPage { layout: o, page: e }
            | BlockKind::SetNetworkOwner {
                target: o,
                owner: e,
            } => vec![e, o.as_expr_mut()],
            BlockKind::WhenAnimationFinished {
                animator,
                animation,
                ..
            } => vec![animation.as_expr_mut(), animator.as_expr_mut()],
            BlockKind::PlayAnimation {
                animator,
                animation,
                fade_time,
                speed,
                weight,
                ..
            } => {
                let mut slots = vec![animation.as_expr_mut(), animator.as_expr_mut()];
                slots.extend(fade_time.as_mut());
                slots.extend(speed.as_mut());
                slots.extend(weight.as_mut());
                slots
            }
            BlockKind::StopAnimation {
                animator,
                animation,
                fade_time,
            } => {
                let mut slots = Vec::new();
                if let Some(anim) = animation {
                    slots.push(anim.as_expr_mut());
                }
                slots.push(animator.as_expr_mut());
                slots.extend(fade_time.as_mut());
                slots
            }
            BlockKind::SetIkTarget {
                ik_control,
                target,
                weight,
            } => {
                let mut slots = vec![ik_control.as_expr_mut(), target.as_expr_mut()];
                slots.extend(weight.as_mut());
                slots
            }
            BlockKind::Patrol { agent, spline } => vec![agent.as_expr_mut(), spline.as_expr_mut()],
            BlockKind::PlaceOnSpline {
                target,
                spline,
                distance,
            } => vec![target.as_expr_mut(), spline.as_expr_mut(), distance],
            BlockKind::Tween {
                target,
                value,
                seconds,
                repeat_count,
                ..
            } => {
                let mut slots = vec![target.as_expr_mut(), value, seconds];
                slots.extend(repeat_count.as_mut());
                slots
            }
            BlockKind::TweenPivot {
                target,
                position,
                duration,
                ..
            } => vec![target.as_expr_mut(), position, duration],
            BlockKind::PulseScale {
                target,
                scale,
                duration,
                ..
            } => vec![target.as_expr_mut(), scale, duration],
            BlockKind::LoadLevel { level, spawn } => vec![level, spawn],
            BlockKind::WhenCommandRun {
                command,
                help,
                admin_only,
                ..
            } => {
                let mut slots = vec![command, help];
                slots.extend(admin_only.as_mut());
                slots
            }
            BlockKind::ForRange {
                start, end, step, ..
            } => vec![start, end, step],
            BlockKind::KickPlayer { player, reason } => vec![player, reason],
            BlockKind::RequestStreamAround { player, position } => vec![player, position],
            BlockKind::ScrollTo { target, x, y } => vec![target.as_expr_mut(), x, y],
            BlockKind::ShakeCamera { intensity, seconds } => vec![intensity, seconds],
            BlockKind::SetAnimatorLayerWeight {
                animator,
                weight,
                seconds,
                ..
            } => vec![animator.as_expr_mut(), weight, seconds],
            BlockKind::SpawnDecal {
                texture,
                position,
                facing,
                size,
                seconds,
            } => vec![texture, position, facing, size, seconds],
            BlockKind::CloneInto {
                target,
                parent,
                count,
                position,
                offset_per_clone,
                per_clone_positions,
            } => {
                let mut slots = vec![target.as_expr_mut(), parent.as_expr_mut()];
                slots.extend(count.as_mut());
                slots.extend(position.as_mut());
                slots.extend(offset_per_clone.as_mut());
                slots.extend(per_clone_positions.as_mut());
                slots
            }
            BlockKind::ListCreate { items, .. } => items.iter_mut().collect(),
            BlockKind::ListAdd { list, value } | BlockKind::ListRemoveValue { list, value } => {
                vec![value, list]
            }
            BlockKind::ListInsert { list, index, value } => vec![value, index, list],
            BlockKind::ListRemoveIndex { list, index } => vec![index, list],
            BlockKind::Local { values, .. } | BlockKind::Return { values } => {
                values.iter_mut().collect()
            }
            BlockKind::GenericFor { values, .. } => values.iter_mut().collect(),
            BlockKind::Assign { targets, values } => {
                targets.iter_mut().chain(values.iter_mut()).collect()
            }
            BlockKind::CompoundAssign { target, value, .. } => vec![target, value],
            BlockKind::CallFunction {
                function: first,
                args,
            }
            | BlockKind::CallMethod {
                object: first,
                args,
                ..
            } => std::iter::once(first).chain(args.iter_mut()).collect(),
            BlockKind::OnSignal { signal: e, .. }
            | BlockKind::Connect { signal: e, .. }
            | BlockKind::While { condition: e, .. }
            | BlockKind::RepeatLoop { condition: e, .. } => vec![e],
            BlockKind::NumericFor {
                start, stop, step, ..
            } => {
                let mut slots = vec![start, stop];
                slots.extend(step.as_mut());
                slots
            }
            BlockKind::ForEachPartInRadius {
                center,
                radius,
                ignore,
                ..
            } => {
                let mut slots = vec![radius, center];
                slots.extend(ignore.as_mut());
                slots
            }
            _ => Vec::new(),
        }
    }

    /// The world's look blocks, which need `Lighting`.
    pub fn sets_lighting(&self) -> bool {
        matches!(
            self,
            BlockKind::SetToneMapping { .. }
                | BlockKind::SetClockTime { .. }
                | BlockKind::SetAtmosphere { .. }
                | BlockKind::SetLightingProperty { .. }
                | BlockKind::SetPostProcessing { .. }
        )
    }

    /// Graphics settings blocks, which need `UserGameSettings`.
    pub fn sets_graphics(&self) -> bool {
        matches!(
            self,
            BlockKind::SetGraphicsQuality { .. }
                | BlockKind::SetGraphicsOption { .. }
                | BlockKind::SetGraphicsNumber { .. }
        )
    }

    /// Tween blocks, which need `TweenService`.
    pub fn tweens(&self) -> bool {
        matches!(
            self,
            BlockKind::Tween { .. }
                | BlockKind::SlideIn { .. }
                | BlockKind::SlideOut { .. }
                | BlockKind::FadeIn { .. }
                | BlockKind::FadeOut { .. }
                | BlockKind::TweenPivot { .. }
                | BlockKind::PulseScale { .. }
                | BlockKind::SlideTo { .. }
                | BlockKind::RotateTo { .. }
                | BlockKind::TweenProperty { .. }
                | BlockKind::FadeVolume { .. }
                | BlockKind::SetClockTime {
                    duration: Some(_),
                    ..
                }
                | BlockKind::SetAtmosphere {
                    duration: Some(_),
                    ..
                }
        )
    }

    /// Save and checkpoint blocks, which need `SaveService`.
    pub fn saves(&self) -> bool {
        matches!(
            self,
            BlockKind::SaveSlot { .. }
                | BlockKind::LoadSlot { .. }
                | BlockKind::DeleteSlot { .. }
                | BlockKind::SetCheckpoint { .. }
                | BlockKind::RestoreCheckpoint
        )
    }

    /// Level blocks, which need `LevelService`.
    pub fn levels(&self) -> bool {
        matches!(
            self,
            BlockKind::LoadLevel { .. }
                | BlockKind::AddLevel { .. }
                | BlockKind::RemoveLevel { .. }
        )
    }

    /// Multiplayer blocks, which need `NetworkService`.
    pub fn networks(&self) -> bool {
        matches!(
            self,
            BlockKind::HostGame { .. }
                | BlockKind::JoinGame { .. }
                | BlockKind::LeaveGame
                | BlockKind::WhenConnectionFails { .. }
        )
    }

    /// Blocks nothing can go after: they never finish ("forever") or end
    /// the script ("stop this script").
    pub fn is_cap(&self) -> bool {
        matches!(
            self,
            BlockKind::Forever { .. }
                | BlockKind::StopScript
                | BlockKind::DestroyScript
                | BlockKind::Break
                | BlockKind::Continue
                | BlockKind::Return { .. }
        )
    }

    /// The list of inputs this block can have more or fewer of, if it has
    /// one: a call's arguments, or a local's or a return's values.
    pub fn input_list_mut(&mut self) -> Option<&mut Vec<Expression>> {
        match self {
            BlockKind::CallFunction { args: list, .. }
            | BlockKind::CallMethod { args: list, .. }
            | BlockKind::Local { values: list, .. }
            | BlockKind::Assign { values: list, .. }
            | BlockKind::Return { values: list }
            | BlockKind::GenericFor { values: list, .. }
            | BlockKind::ListCreate { items: list, .. } => Some(list),
            _ => None,
        }
    }

    /// The value slots this block has, in the order they're shown.
    pub fn expressions(&self) -> Vec<&Expression> {
        match self {
            BlockKind::WhenStreamedIn { target: o, .. }
            | BlockKind::WhenStreamedOut { target: o, .. }
            | BlockKind::WhenChildAdded { target: o, .. }
            | BlockKind::WhenChildRemoved { target: o, .. }
            | BlockKind::WhenRemoteFires { remote: o, .. }
            | BlockKind::WhenClicked { button: o, .. }
            | BlockKind::WhenMouseEnters { element: o, .. }
            | BlockKind::WhenMouseLeaves { element: o, .. }
            | BlockKind::WhenValueChanged { widget: o, .. }
            | BlockKind::WhenSoundEnded { sound: o, .. }
            | BlockKind::WhenCutsceneFinished { sequence: o, .. }
            | BlockKind::WhenStateChanged { animator: o, .. }
            | BlockKind::WhenAnimationEvent { animator: o, .. }
            | BlockKind::WhenAgentSenses { agent: o, .. }
            | BlockKind::WhenAgentLosesPlayer { agent: o, .. }
            | BlockKind::WhenAgentArrives { agent: o, .. }
            | BlockKind::WhenSplineReached { follower: o, .. }
            | BlockKind::WhenSeatOccupied { seat: o, .. }
            | BlockKind::WhenSeatVacated { seat: o, .. }
            | BlockKind::WhenPropertyChanged { target: o, .. }
            | BlockKind::WhenAttributeChanged { target: o, .. }
            | BlockKind::WaitForEvent { target: o, .. }
            | BlockKind::ForEachChild { target: o, .. }
            | BlockKind::Show { target: o }
            | BlockKind::Hide { target: o }
            | BlockKind::ToggleVisibility { target: o }
            | BlockKind::SetMaterial { target: o, .. }
            | BlockKind::StopAgent { agent: o }
            | BlockKind::LeaveSeat { controller: o }
            | BlockKind::ControlSpline { follower: o, .. }
            | BlockKind::SetCheckpoint { checkpoint: o }
            | BlockKind::SetNetworkOwnershipAuto { target: o }
            | BlockKind::StopSound { target: o }
            | BlockKind::PauseSound { target: o }
            | BlockKind::ResumeSound { target: o }
            | BlockKind::SetAnimatorTrigger { animator: o, .. }
            | BlockKind::ResetAnimatorTrigger { animator: o, .. }
            | BlockKind::PlayAnimatorState { animator: o, .. }
            | BlockKind::PlayCutscene { sequence: o }
            | BlockKind::PauseCutscene { sequence: o }
            | BlockKind::ResumeCutscene { sequence: o }
            | BlockKind::StopCutscene { sequence: o }
            | BlockKind::CancelTween { tween: o }
            | BlockKind::Destroy { target: o } => vec![o.as_expr()],
            BlockKind::PlaySound {
                target: o,
                pitch: opt,
                ..
            } => {
                let mut slots = vec![o.as_expr()];
                slots.extend(opt.as_ref());
                slots
            }
            BlockKind::PlaySoundAtPosition {
                target: o,
                position: p,
                volume,
                pitch,
            } => {
                let mut slots = vec![o.as_expr(), p];
                slots.extend(volume.as_ref());
                slots.extend(pitch.as_ref());
                slots
            }
            BlockKind::WhenTouched {
                part,
                filter_value,
                cooldown,
                ..
            } => {
                let mut slots = vec![part.as_expr()];
                slots.extend(filter_value.as_ref());
                slots.extend(cooldown.as_ref());
                slots
            }
            BlockKind::WhenTouchEnds {
                part, filter_value, ..
            } => {
                let mut slots = vec![part.as_expr()];
                slots.extend(filter_value.as_ref());
                slots
            }
            BlockKind::WhenPromptTriggered {
                prompt: o,
                cooldown: opt,
                ..
            }
            | BlockKind::WhenPartClicked {
                detector: o,
                cooldown: opt,
                ..
            }
            | BlockKind::WaitForChild {
                target: o,
                timeout: opt,
                ..
            }
            | BlockKind::ForEachPartTouching {
                target: o,
                ignore: opt,
                ..
            } => {
                let mut slots = vec![o.as_expr()];
                slots.extend(opt.as_ref());
                slots
            }
            BlockKind::FireEvent { data, .. } => data.iter().collect(),
            BlockKind::Wait { seconds: e }
            | BlockKind::WaitUntil { condition: e }
            | BlockKind::Repeat { count: e, .. }
            | BlockKind::RepeatUntil { condition: e, .. }
            | BlockKind::ForEachInList { list: e, .. }
            | BlockKind::ForEachTagged { tag: e, .. }
            | BlockKind::CooldownGate { cooldown: e, .. }
            | BlockKind::SpawnAsync { delay: e, .. }
            | BlockKind::WhenTagAdded { tag: e, .. }
            | BlockKind::WhenTagRemoved { tag: e, .. }
            | BlockKind::Disconnect { connection: e }
            | BlockKind::If { condition: e, .. }
            | BlockKind::IfElse { condition: e, .. }
            | BlockKind::SaveSlot { slot: e }
            | BlockKind::LoadSlot { slot: e }
            | BlockKind::DeleteSlot { slot: e }
            | BlockKind::AddLevel { level: e }
            | BlockKind::RemoveLevel { level: e }
            | BlockKind::HostGame { port: e }
            | BlockKind::JoinGame { address: e }
            | BlockKind::SendChatMessage { text: e }
            | BlockKind::ShowSystemMessage { text: e }
            | BlockKind::SetLayerActive { active: e, .. }
            | BlockKind::SetLanguage { language: e }
            | BlockKind::SetVariable { value: e, .. }
            | BlockKind::ChangeVariable { by: e, .. }
            | BlockKind::ListClear { list: e }
            | BlockKind::SetGraphicsNumber { value: e, .. }
            | BlockKind::SetLightingProperty { value: e, .. }
            | BlockKind::SetPostProcessing { value: e, .. }
            | BlockKind::PointCameraAt { position: e }
            | BlockKind::Print { value: e } => vec![e],
            BlockKind::SetGraphicsOption { value, .. } => value.iter().collect(),
            BlockKind::SetClockTime { time: e, duration }
            | BlockKind::SetAtmosphere {
                value: e, duration, ..
            } => {
                let mut slots = vec![e];
                slots.extend(duration.as_ref());
                slots
            }
            BlockKind::SlideTo {
                target: o,
                position: a,
                duration: b,
                ..
            }
            | BlockKind::RotateTo {
                target: o,
                rotation: a,
                duration: b,
                ..
            }
            | BlockKind::TweenProperty {
                target: o,
                value: a,
                duration: b,
                ..
            }
            | BlockKind::FadeVolume {
                target: o,
                volume: a,
                duration: b,
                ..
            }
            | BlockKind::LaunchPart {
                target: o,
                direction: a,
                speed: b,
            } => vec![o.as_expr(), a, b],
            BlockKind::FollowTarget {
                agent: a,
                target: b,
            }
            | BlockKind::SitInSeat {
                seat: a,
                controller: b,
            } => vec![a.as_expr(), b.as_expr()],
            BlockKind::MoveTo {
                target: o,
                position: e,
            }
            | BlockKind::MoveBy {
                target: o,
                offset: e,
            }
            | BlockKind::LookAt {
                target: o,
                position: e,
            }
            | BlockKind::MoveForward {
                target: o,
                distance: e,
            }
            | BlockKind::TurnTo {
                target: o,
                orientation: e,
            }
            | BlockKind::TurnBy {
                target: o,
                rotation: e,
                ..
            }
            | BlockKind::SetGravity {
                target: o,
                multiplier: e,
            }
            | BlockKind::SetAngularVelocity {
                target: o,
                velocity: e,
            }
            | BlockKind::SetSplineSpeed {
                follower: o,
                speed: e,
            }
            | BlockKind::SetAnchored {
                target: o,
                anchored: e,
            }
            | BlockKind::PushWithForce {
                target: o,
                force: e,
            }
            | BlockKind::SetPartVelocity {
                target: o,
                velocity: e,
            }
            | BlockKind::SetFriction {
                target: o,
                friction: e,
            }
            | BlockKind::SetColor {
                target: o,
                color: e,
            }
            | BlockKind::SetTransparency {
                target: o,
                transparency: e,
            }
            | BlockKind::EmitParticles {
                target: o,
                count: e,
            }
            | BlockKind::SetEffectEnabled {
                target: o,
                enabled: e,
            }
            | BlockKind::SetImage {
                target: o,
                image: e,
            }
            | BlockKind::SetSoundPitch {
                target: o,
                pitch: e,
            }
            | BlockKind::SetSoundLooping {
                target: o,
                looped: e,
            }
            | BlockKind::SetSoundTime { target: o, time: e }
            | BlockKind::SetText { target: o, text: e }
            | BlockKind::SetProperty {
                target: o,
                value: e,
                ..
            }
            | BlockKind::SetAttribute {
                target: o,
                value: e,
                ..
            }
            | BlockKind::SetValue {
                target: o,
                value: e,
            }
            | BlockKind::SetDestination {
                agent: o,
                position: e,
            }
            | BlockKind::SetVelocity {
                agent: o,
                velocity: e,
            }
            | BlockKind::SetVolume {
                target: o,
                volume: e,
            }
            | BlockKind::SetAnimatorBool {
                animator: o,
                value: e,
                ..
            }
            | BlockKind::SetAnimatorNumber {
                animator: o,
                value: e,
                ..
            }
            | BlockKind::SetAnimatorSpeed {
                animator: o,
                speed: e,
            }
            | BlockKind::SetRagdollEnabled {
                ragdoll: o,
                enabled: e,
            }
            | BlockKind::SlideIn {
                target: o,
                seconds: e,
                ..
            }
            | BlockKind::SlideOut {
                target: o,
                seconds: e,
                ..
            }
            | BlockKind::FadeIn {
                target: o,
                seconds: e,
                ..
            }
            | BlockKind::FadeOut {
                target: o,
                seconds: e,
                ..
            }
            | BlockKind::DestroyAfter {
                target: o,
                seconds: e,
            } => vec![o.as_expr(), e],
            BlockKind::ShowPage { layout: o, page: e }
            | BlockKind::SetNetworkOwner {
                target: o,
                owner: e,
            } => vec![e, o.as_expr()],
            BlockKind::WhenAnimationFinished {
                animator,
                animation,
                ..
            } => vec![animation.as_expr(), animator.as_expr()],
            BlockKind::PlayAnimation {
                animator,
                animation,
                fade_time,
                speed,
                weight,
                ..
            } => {
                let mut slots = vec![animation.as_expr(), animator.as_expr()];
                slots.extend(fade_time.as_ref());
                slots.extend(speed.as_ref());
                slots.extend(weight.as_ref());
                slots
            }
            BlockKind::StopAnimation {
                animator,
                animation,
                fade_time,
            } => {
                let mut slots = Vec::new();
                if let Some(anim) = animation {
                    slots.push(anim.as_expr());
                }
                slots.push(animator.as_expr());
                slots.extend(fade_time.as_ref());
                slots
            }
            BlockKind::SetIkTarget {
                ik_control,
                target,
                weight,
            } => {
                let mut slots = vec![ik_control.as_expr(), target.as_expr()];
                slots.extend(weight.as_ref());
                slots
            }
            BlockKind::Patrol { agent, spline } => vec![agent.as_expr(), spline.as_expr()],
            BlockKind::PlaceOnSpline {
                target,
                spline,
                distance,
            } => vec![target.as_expr(), spline.as_expr(), distance],
            BlockKind::Tween {
                target,
                value,
                seconds,
                repeat_count,
                ..
            } => {
                let mut slots = vec![target.as_expr(), value, seconds];
                slots.extend(repeat_count.as_ref());
                slots
            }
            BlockKind::TweenPivot {
                target,
                position,
                duration,
                ..
            } => vec![target.as_expr(), position, duration],
            BlockKind::PulseScale {
                target,
                scale,
                duration,
                ..
            } => vec![target.as_expr(), scale, duration],
            BlockKind::LoadLevel { level, spawn } => vec![level, spawn],
            BlockKind::WhenCommandRun {
                command,
                help,
                admin_only,
                ..
            } => {
                let mut slots = vec![command, help];
                slots.extend(admin_only.as_ref());
                slots
            }
            BlockKind::ForRange {
                start, end, step, ..
            } => vec![start, end, step],
            BlockKind::RequestStreamAround { player, position } => vec![player, position],
            BlockKind::KickPlayer { player, reason } => vec![player, reason],
            BlockKind::ScrollTo { target, x, y } => vec![target.as_expr(), x, y],
            BlockKind::ShakeCamera { intensity, seconds } => vec![intensity, seconds],
            BlockKind::SetAnimatorLayerWeight {
                animator,
                weight,
                seconds,
                ..
            } => vec![animator.as_expr(), weight, seconds],
            BlockKind::SpawnDecal {
                texture,
                position,
                facing,
                size,
                seconds,
            } => vec![texture, position, facing, size, seconds],
            BlockKind::CloneInto {
                target,
                parent,
                count,
                position,
                offset_per_clone,
                per_clone_positions,
            } => {
                let mut slots = vec![target.as_expr(), parent.as_expr()];
                slots.extend(count.as_ref());
                slots.extend(position.as_ref());
                slots.extend(offset_per_clone.as_ref());
                slots.extend(per_clone_positions.as_ref());
                slots
            }
            BlockKind::ListCreate { items, .. } => items.iter().collect(),
            BlockKind::ListAdd { list, value } | BlockKind::ListRemoveValue { list, value } => {
                vec![value, list]
            }
            BlockKind::ListInsert { list, index, value } => vec![value, index, list],
            BlockKind::ListRemoveIndex { list, index } => vec![index, list],
            BlockKind::Local { values, .. } | BlockKind::Return { values } => {
                values.iter().collect()
            }
            BlockKind::GenericFor { values, .. } => values.iter().collect(),
            BlockKind::Assign { targets, values } => targets.iter().chain(values.iter()).collect(),
            BlockKind::CompoundAssign { target, value, .. } => vec![target, value],
            BlockKind::CallFunction {
                function: first,
                args,
            }
            | BlockKind::CallMethod {
                object: first,
                args,
                ..
            } => std::iter::once(first).chain(args.iter()).collect(),
            BlockKind::OnSignal { signal: e, .. }
            | BlockKind::Connect { signal: e, .. }
            | BlockKind::While { condition: e, .. }
            | BlockKind::RepeatLoop { condition: e, .. } => vec![e],
            BlockKind::NumericFor {
                start, stop, step, ..
            } => {
                let mut slots = vec![start, stop];
                slots.extend(step.as_ref());
                slots
            }
            BlockKind::ForEachPartInRadius {
                center,
                radius,
                ignore,
                ..
            } => {
                let mut slots = vec![radius, center];
                slots.extend(ignore.as_ref());
                slots
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expression {
    // Values typed into a slot.
    Number {
        value: f64,
    },
    Text {
        value: String,
    },
    Boolean {
        value: bool,
    },
    Vector3 {
        x: f64,
        y: f64,
        z: f64,
    },
    Color3 {
        r: f64,
        g: f64,
        b: f64,
    },
    /// A static dotted scene path picked from an instance dropdown (inside an
    /// [`ObjectRef`] slot). Treated as a literal so the dropdown renders inline
    /// when unwired and disappears when a reporter or wire is connected.
    ObjectPath {
        path: String,
    },
    /// Reference to a right-side output data pin of block `block` (output index
    /// `output`), bound to Luau identifier `name` (e.g. `Baseplate_cloned`,
    /// `otherPart`, or a `Local` variable).
    BlockOutput {
        block: u64,
        output: usize,
        name: String,
    },

    // Reporters.
    Variable {
        name: String,
    },
    /// 1-based item lookup in a list: `list[index]`.
    ListItem {
        list: Box<Expression>,
        index: Box<Expression>,
    },
    /// Number of items in a list: `#list`.
    ListLength {
        list: Box<Expression>,
    },
    /// Whether `list` contains `value`: `table.find(list, value) ~= nil`.
    ListContains {
        list: Box<Expression>,
        value: Box<Expression>,
    },
    Math {
        op: MathOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// `math.abs` and friends, and rounding.
    Function {
        function: MathFunction,
        value: Box<Expression>,
    },
    /// A whole number from `min` to `max`, both included.
    Random {
        min: Box<Expression>,
        max: Box<Expression>,
    },
    /// Any number from `min` up to `max`: `Random.new():NextNumber`.
    RandomNumber {
        min: Box<Expression>,
        max: Box<Expression>,
    },
    /// The Vector3 or Color3 `alpha` of the way from `from` to `to`.
    Lerp {
        from: Box<Expression>,
        to: Box<Expression>,
        alpha: Box<Expression>,
    },
    /// Two values as text, one after the other.
    Join {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Compare {
        op: Comparison,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Logic {
        op: LogicOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Not {
        value: Box<Expression>,
    },
    /// An object's property, such as a part's `Position`.
    Property {
        target: ObjectRef,
        property: String,
    },
    /// One of an object's attributes (`GetAttribute`), nil when it has none
    /// by that name.
    Attribute {
        target: ObjectRef,
        attribute: String,
    },
    /// `workspace:Raycast(origin, direction[, params])`, optionally extracting
    /// one field from the result (`output` in [`RAYCAST_OUTPUTS`]).
    Raycast {
        origin: Box<Expression>,
        direction: Box<Expression>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        output: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ignore: Option<ObjectRef>,
    },
    /// Legacy: `workspace:Raycast(origin, direction, params)`, passing by `ignore`
    /// and everything inside it, such as the shooter's own character.
    RaycastIgnoring {
        origin: Box<Expression>,
        direction: Box<Expression>,
        ignore: ObjectRef,
    },
    /// The part that touched, inside "when touched".
    TouchingPart,
    /// The triggering player inside a player-bearing event (e.g. When Touched,
    /// When Prompt Triggered, When Part Clicked, When Command Run, When Player Joins).
    TriggeringPlayer,
    /// The command arguments list inside `When Command Run`.
    CommandArgs,
    /// The frame delta time (`dt`) inside `Every Frame`, `Before Render`, or
    /// `Before Physics`.
    DeltaTime,
    /// The part under the mouse: `LocalPlayer:GetMouse().Target`, the
    /// camera's ray through the mouse cast into the world. Client only.
    PartUnderMouse,
    /// The 3D world position under the mouse: `LocalPlayer:GetMouse().Hit.Position`.
    MouseHitPosition,
    /// The current camera's world position: `workspace.CurrentCamera.CFrame.Position`.
    CameraPosition,
    /// The current camera's unit look direction: `workspace.CurrentCamera.CFrame.LookVector`.
    CameraLookDirection,
    /// The player whose character is nearest `position`, or nil.
    NearestPlayerTo {
        position: Box<Expression>,
    },
    /// The `Player` owning `part`'s character (`Players:GetPlayerFromCharacter`).
    PlayerOfPart {
        part: ObjectRef,
    },
    /// Whether `player`'s character has Health > 0.
    PlayerIsAlive {
        player: ObjectRef,
    },
    /// `target.Parent`.
    ParentOf {
        target: ObjectRef,
    },
    /// `target:FindFirstChild(name)`.
    ChildOf {
        name: Box<Expression>,
        target: ObjectRef,
    },
    /// `CollectionService:HasTag(target, tag)`.
    HasTag {
        target: ObjectRef,
        tag: Box<Expression>,
    },
    /// Elapsed game time in seconds (`workspace.DistributedGameTime`).
    GameTime,
    /// Whether a key is held down. Client only.
    KeyDown {
        key: String,
    },
    /// Whether a data layer is on: `DataLayerService:IsLayerActive`
    /// (roadmap R16.4).
    LayerActive {
        data_layer: String,
    },
    /// An Animator's Bool parameter (`animator:GetBool`).
    AnimatorBool {
        animator: ObjectRef,
        parameter: String,
    },
    /// An Animator's Number parameter (`animator:GetNumber`).
    AnimatorNumber {
        animator: ObjectRef,
        parameter: String,
    },
    /// How much an Animator's layer counts now
    /// (`animator:GetLayerWeight`, Phase 78).
    AnimatorLayerWeight {
        animator: ObjectRef,
        layer: String,
    },
    /// Whether `value` (an Instance) is of class `class` or inherits from
    /// it: `child:IsA("BasePart")`.
    IsA {
        value: Box<Expression>,
        class: String,
    },
    /// Whether `value` is on this device:
    /// `value.StreamingState == Enum.StreamingState.StreamedIn` (Phase 80).
    IsStreamedIn {
        value: Box<Expression>,
    },
    /// A row of a data table, by its key:
    /// `DataService:GetTable("weapons"):Get(key)`. Nil when there's none.
    DataRow {
        table: String,
        key: Box<Expression>,
    },
    /// The point `t` of the way along a Spline (`spline:GetPositionAt(t)`,
    /// Phase 73).
    SplinePosition {
        spline: ObjectRef,
        t: Box<Expression>,
    },
    /// How far along a Spline its point nearest `position` is
    /// (`spline:GetClosestDistance(position)`).
    SplineClosestDistance {
        spline: ObjectRef,
        position: Box<Expression>,
    },
    /// A row's value in one column: `row.Damage`.
    DataValue {
        column: String,
        row: Box<Expression>,
    },
    /// A property of a `Sound` instance (`Playing`, `TimePosition`, `TimeLength`,
    /// `Volume`, `PlaybackSpeed`, `Looped`).
    SoundProperty {
        target: ObjectRef,
        property: String,
    },
    /// Constructs a Vector3 from three expressions: `Vector3.new(x, y, z)`.
    MakeVector3 {
        x: Box<Expression>,
        y: Box<Expression>,
        z: Box<Expression>,
    },
    /// Extracts `"X"`, `"Y"`, or `"Z"` from a Vector3: `vector.X`.
    VectorAxis {
        axis: String,
        vector: Box<Expression>,
    },
    /// Distance between two Vector3 points: `(from - to).Magnitude`.
    Distance {
        from: Box<Expression>,
        to: Box<Expression>,
    },
    /// Unit direction vector from `from` toward `to`: `(to - from).Unit`.
    Direction {
        from: Box<Expression>,
        to: Box<Expression>,
    },
    /// Length of a Vector3: `vector.Magnitude`.
    Magnitude {
        vector: Box<Expression>,
    },
    /// Clamps `value` between `min` and `max`: `math.clamp(value, min, max)`.
    Clamp {
        value: Box<Expression>,
        min: Box<Expression>,
        max: Box<Expression>,
    },
    /// Minimum or maximum of two numbers: `math.min(a, b)` / `math.max(a, b)`.
    MinMax {
        op: String,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// Character count of a string: `#tostring(text)`.
    StringLength {
        text: Box<Expression>,
    },
    /// Whether `text` contains `substring` (plain match):
    /// `string.find(tostring(text), tostring(substring), 1, true) ~= nil`.
    StringContains {
        text: Box<Expression>,
        substring: Box<Expression>,
    },
    /// Formats a number to `decimals` decimal places:
    /// `string.format("%.2f", value)`.
    FormatNumber {
        value: Box<Expression>,
        decimals: Box<Expression>,
    },
    /// Constructs a Color3 from 0–255 RGB components: `Color3.fromRGB(r, g, b)`.
    MakeColorRgb {
        r: Box<Expression>,
        g: Box<Expression>,
        b: Box<Expression>,
    },
    /// Constructs a Color3 from 0–1 HSV components: `Color3.fromHSV(h, s, v)`.
    MakeColorHsv {
        h: Box<Expression>,
        s: Box<Expression>,
        v: Box<Expression>,
    },
    /// The new state name (`state`) inside `WhenStateChanged`.
    AnimatorNewState,
    /// The previous state name (`previous`) inside `WhenStateChanged`.
    AnimatorPreviousState,
    /// The animation event name (`name`) inside `WhenAnimationEvent`.
    AnimationEventName,
    /// The failure reason (`reason`) inside `WhenConnectionFails`.
    ConnectionFailureReason,
    /// The changed data layer name (`layerName`) inside `WhenLayerChanged`.
    ChangedLayerName,
    /// Whether the changed data layer is active (`active`) inside `WhenLayerChanged`.
    ChangedLayerActive,
    /// The chat message text (`message.Text`) inside `WhenChatMessageReceived`.
    ChatMessageText,
    /// The player who sent the chat message (`message.Player`) inside `WhenChatMessageReceived`.
    ChatMessageSender,
    /// Whether this device is hosting the multiplayer game (`NetworkService.IsHost`).
    IsHost,
    /// Whether this device is connected to a multiplayer game (`NetworkService.IsConnected`).
    IsConnected,
    /// Whether save slot `slot` exists (`SaveService:HasSlot(slot)`).
    SlotExists {
        slot: Box<Expression>,
    },
    /// Number of rows in a data table (`DataService:GetTable(t):Count()`).
    DataTableRowCount {
        table: String,
    },

    // Luau's own values (see `BlockKind::RunScript`).
    Nil,
    /// `...`.
    Vararg,
    /// A name, or a name and fields: `waiting`, `row.Binding.Text`.
    Name {
        name: String,
    },
    /// `object.name`, for an object that isn't a plain name.
    Field {
        object: Box<Expression>,
        name: String,
    },
    /// `object[key]`.
    Index {
        object: Box<Expression>,
        key: Box<Expression>,
    },
    /// `function(args)`.
    Call {
        function: Box<Expression>,
        args: Vec<Expression>,
    },
    /// `object:method(args)`.
    MethodCall {
        object: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },
    /// `function(params) ... end`. Its blocks are shown as a mouth of the
    /// block it's in: see [`BlockKind::bodies`].
    Closure {
        params: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        generics: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        returns: String,
        body: Vec<Block>,
    },
    /// `{ ... }`, on one line or one field to a line.
    Table {
        fields: Vec<TableField>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        multiline: bool,
    },
    /// An operator with no block of its own, or used on values the block
    /// wouldn't take: `..`, `//`, `a or 5`.
    Binary {
        op: String,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// `-value`, `#value` or `not value`.
    Unary {
        op: String,
        value: Box<Expression>,
    },
    /// `(value)`, kept where the brackets change what it means.
    Paren {
        value: Box<Expression>,
    },
    /// `` `{a} and {b}` ``: the text around the values, as written, which
    /// is one more piece than there are values.
    Interpolated {
        parts: Vec<String>,
        values: Vec<Expression>,
    },
    /// `if a then b elseif c then d else e`, as
    /// `[a, b, c, d, e]`.
    IfValue {
        parts: Vec<Expression>,
    },
    /// `value :: Type`, with the type as written.
    Cast {
        value: Box<Expression>,
        annotation: String,
    },

    /// Lua that has no block, kept word for word from a script switched
    /// from Code to Blocks. Pointed, so it fits any slot.
    Code {
        code: String,
    },
}

/// One field of an [`Expression::Table`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TableField {
    /// `name = value`.
    Named { name: String, value: Expression },
    /// `[key] = value`.
    Keyed { key: Expression, value: Expression },
    /// `value`.
    Item { value: Expression },
}

impl Expression {
    pub fn shape(&self) -> BlockShape {
        match self {
            Expression::Boolean { .. }
            | Expression::Compare { .. }
            | Expression::Logic { .. }
            | Expression::Not { .. }
            | Expression::KeyDown { .. }
            | Expression::LayerActive { .. }
            | Expression::AnimatorBool { .. }
            | Expression::IsA { .. }
            | Expression::IsStreamedIn { .. }
            | Expression::ListContains { .. }
            | Expression::StringContains { .. }
            | Expression::ChangedLayerActive
            | Expression::IsHost
            | Expression::IsConnected
            | Expression::SlotExists { .. }
            | Expression::Code { .. } => BlockShape::Boolean,
            Expression::SoundProperty { property, .. }
                if matches!(property.as_str(), "Playing" | "Looped") =>
            {
                BlockShape::Boolean
            }
            // Any value can be a condition in Luau, so what can't be told
            // apart from one fits a condition.
            expression if expression.is_untyped() => BlockShape::Boolean,
            _ => BlockShape::Value,
        }
    }

    /// A value that could be anything until the script runs.
    pub fn is_untyped(&self) -> bool {
        matches!(
            self,
            Expression::Nil
                | Expression::Vararg
                | Expression::Name { .. }
                | Expression::BlockOutput { .. }
                | Expression::Field { .. }
                | Expression::Index { .. }
                | Expression::Call { .. }
                | Expression::MethodCall { .. }
                | Expression::Binary { .. }
                | Expression::Unary { .. }
                | Expression::Paren { .. }
                | Expression::IfValue { .. }
                | Expression::Cast { .. }
        )
    }

    /// Adds the bodies of the functions inside this value.
    fn closure_bodies<'a>(&'a self, bodies: &mut Vec<&'a Vec<Block>>) {
        if let Expression::Closure { body, .. } = self {
            bodies.push(body);
        }
        for child in self.children() {
            child.closure_bodies(bodies);
        }
    }

    fn closure_bodies_mut<'a>(&'a mut self, bodies: &mut Vec<&'a mut Vec<Block>>) {
        if let Expression::Closure { body, .. } = self {
            bodies.push(body);
            return;
        }
        for child in self.children_mut() {
            child.closure_bodies_mut(bodies);
        }
    }

    /// The list of inputs this value can have more or fewer of, if it has
    /// one: a call's arguments.
    pub fn input_list_mut(&mut self) -> Option<&mut Vec<Expression>> {
        match self {
            Expression::Call { args, .. } | Expression::MethodCall { args, .. } => Some(args),
            _ => None,
        }
    }

    /// A value typed in, rather than a reporter block.
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            Expression::Number { .. }
                | Expression::Text { .. }
                | Expression::Boolean { .. }
                | Expression::Vector3 { .. }
                | Expression::Color3 { .. }
                | Expression::ObjectPath { .. }
                | Expression::Nil
                | Expression::Vararg
                | Expression::Name { .. }
                | Expression::Closure { .. }
        )
    }

    /// Whether this reporter only works in client block scripts.
    pub fn client_only(&self) -> bool {
        matches!(
            self,
            Expression::KeyDown { .. } | Expression::PartUnderMouse
        )
    }

    /// Like [`Expression::children`], for changing them.
    pub fn children_mut(&mut self) -> Vec<&mut Expression> {
        match self {
            Expression::Math { left, right, .. }
            | Expression::Join { left, right }
            | Expression::Compare { left, right, .. }
            | Expression::Logic { left, right, .. } => vec![left, right],
            Expression::Random { min, max } | Expression::RandomNumber { min, max } => {
                vec![min, max]
            }
            Expression::Lerp { from, to, alpha } => vec![from, to, alpha],
            Expression::MakeVector3 { x, y, z } => vec![x, y, z],
            Expression::MakeColorRgb { r, g, b } => vec![r, g, b],
            Expression::MakeColorHsv { h, s, v } => vec![h, s, v],
            Expression::Clamp { value, min, max } => vec![value, min, max],
            Expression::Distance { from, to } | Expression::Direction { from, to } => {
                vec![from, to]
            }
            Expression::MinMax { left, right, .. } => vec![left, right],
            Expression::StringContains { text, substring } => vec![text, substring],
            Expression::FormatNumber { value, decimals } => vec![value, decimals],
            Expression::VectorAxis { vector: v, .. }
            | Expression::Magnitude { vector: v }
            | Expression::StringLength { text: v } => vec![v],
            Expression::Raycast {
                origin,
                direction,
                ignore,
                ..
            } => {
                let mut slots = vec![origin.as_mut(), direction.as_mut()];
                if let Some(ig) = ignore {
                    slots.push(ig.as_expr_mut());
                }
                slots
            }
            Expression::RaycastIgnoring {
                origin,
                direction,
                ignore,
            } => vec![origin, direction, ignore.as_expr_mut()],
            Expression::Property { target: o, .. }
            | Expression::Attribute { target: o, .. }
            | Expression::SoundProperty { target: o, .. }
            | Expression::PlayerOfPart { part: o }
            | Expression::PlayerIsAlive { player: o }
            | Expression::ParentOf { target: o }
            | Expression::AnimatorBool { animator: o, .. }
            | Expression::AnimatorNumber { animator: o, .. }
            | Expression::AnimatorLayerWeight { animator: o, .. } => vec![o.as_expr_mut()],
            Expression::NearestPlayerTo { position } => vec![position],
            Expression::ChildOf { name, target } => vec![name, target.as_expr_mut()],
            Expression::HasTag { target, tag } => vec![target.as_expr_mut(), tag],
            Expression::SplinePosition { spline, t } => vec![t, spline.as_expr_mut()],
            Expression::SplineClosestDistance { spline, position } => {
                vec![spline.as_expr_mut(), position]
            }
            Expression::ListItem { list, index } => vec![index, list],
            Expression::ListLength { list } => vec![list],
            Expression::ListContains { list, value } => vec![list, value],
            Expression::Function { value, .. }
            | Expression::Not { value }
            | Expression::IsA { value, .. }
            | Expression::IsStreamedIn { value }
            | Expression::DataRow { key: value, .. }
            | Expression::DataValue { row: value, .. }
            | Expression::SlotExists { slot: value }
            | Expression::Field { object: value, .. }
            | Expression::Unary { value, .. }
            | Expression::Paren { value }
            | Expression::Cast { value, .. } => vec![value],
            Expression::Index { object, key } => vec![object, key],
            Expression::Binary { left, right, .. } => vec![left, right],
            Expression::Call {
                function: first,
                args,
            }
            | Expression::MethodCall {
                object: first,
                args,
                ..
            } => std::iter::once(first.as_mut())
                .chain(args.iter_mut())
                .collect(),
            Expression::Table { fields, .. } => {
                let mut slots = Vec::new();
                for field in fields {
                    match field {
                        TableField::Named { value, .. } | TableField::Item { value } => {
                            slots.push(value);
                        }
                        TableField::Keyed { key, value } => {
                            slots.push(key);
                            slots.push(value);
                        }
                    }
                }
                slots
            }
            Expression::Interpolated { values: list, .. } | Expression::IfValue { parts: list } => {
                list.iter_mut().collect()
            }
            _ => Vec::new(),
        }
    }

    /// The reporters directly inside this one.
    pub fn children(&self) -> Vec<&Expression> {
        match self {
            Expression::Math { left, right, .. }
            | Expression::Join { left, right }
            | Expression::Compare { left, right, .. }
            | Expression::Logic { left, right, .. } => vec![left, right],
            Expression::Random { min, max } | Expression::RandomNumber { min, max } => {
                vec![min, max]
            }
            Expression::Lerp { from, to, alpha } => vec![from, to, alpha],
            Expression::MakeVector3 { x, y, z } => vec![x, y, z],
            Expression::MakeColorRgb { r, g, b } => vec![r, g, b],
            Expression::MakeColorHsv { h, s, v } => vec![h, s, v],
            Expression::Clamp { value, min, max } => vec![value, min, max],
            Expression::Distance { from, to } | Expression::Direction { from, to } => {
                vec![from, to]
            }
            Expression::MinMax { left, right, .. } => vec![left, right],
            Expression::StringContains { text, substring } => vec![text, substring],
            Expression::FormatNumber { value, decimals } => vec![value, decimals],
            Expression::VectorAxis { vector: v, .. }
            | Expression::Magnitude { vector: v }
            | Expression::StringLength { text: v } => vec![v],
            Expression::Raycast {
                origin,
                direction,
                ignore,
                ..
            } => {
                let mut slots = vec![origin.as_ref(), direction.as_ref()];
                if let Some(ig) = ignore {
                    slots.push(ig.as_expr());
                }
                slots
            }
            Expression::RaycastIgnoring {
                origin,
                direction,
                ignore,
            } => vec![origin, direction, ignore.as_expr()],
            Expression::Property { target: o, .. }
            | Expression::Attribute { target: o, .. }
            | Expression::SoundProperty { target: o, .. }
            | Expression::PlayerOfPart { part: o }
            | Expression::PlayerIsAlive { player: o }
            | Expression::ParentOf { target: o }
            | Expression::AnimatorBool { animator: o, .. }
            | Expression::AnimatorNumber { animator: o, .. }
            | Expression::AnimatorLayerWeight { animator: o, .. } => vec![o.as_expr()],
            Expression::NearestPlayerTo { position } => vec![position],
            Expression::ChildOf { name, target } => vec![name, target.as_expr()],
            Expression::HasTag { target, tag } => vec![target.as_expr(), tag],
            Expression::SplinePosition { spline, t } => vec![t, spline.as_expr()],
            Expression::SplineClosestDistance { spline, position } => {
                vec![spline.as_expr(), position]
            }
            Expression::ListItem { list, index } => vec![index, list],
            Expression::ListLength { list } => vec![list],
            Expression::ListContains { list, value } => vec![list, value],
            Expression::Function { value, .. }
            | Expression::Not { value }
            | Expression::IsStreamedIn { value }
            | Expression::IsA { value, .. }
            | Expression::DataRow { key: value, .. }
            | Expression::DataValue { row: value, .. }
            | Expression::SlotExists { slot: value }
            | Expression::Field { object: value, .. }
            | Expression::Unary { value, .. }
            | Expression::Paren { value }
            | Expression::Cast { value, .. } => vec![value],
            Expression::Index { object, key } => vec![object, key],
            Expression::Binary { left, right, .. } => vec![left, right],
            Expression::Call {
                function: first,
                args,
            }
            | Expression::MethodCall {
                object: first,
                args,
                ..
            } => std::iter::once(first.as_ref()).chain(args.iter()).collect(),
            Expression::Table { fields, .. } => {
                let mut slots = Vec::new();
                for field in fields {
                    match field {
                        TableField::Named { value, .. } | TableField::Item { value } => {
                            slots.push(value);
                        }
                        TableField::Keyed { key, value } => {
                            slots.push(key);
                            slots.push(value);
                        }
                    }
                }
                slots
            }
            Expression::Interpolated { values: list, .. } | Expression::IfValue { parts: list } => {
                list.iter().collect()
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

impl Comparison {
    pub const ALL: [Comparison; 6] = [
        Comparison::Equal,
        Comparison::NotEqual,
        Comparison::Less,
        Comparison::LessOrEqual,
        Comparison::Greater,
        Comparison::GreaterOrEqual,
    ];

    pub fn lua(self) -> &'static str {
        match self {
            Self::Equal => "==",
            Self::NotEqual => "~=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
        }
    }

    /// The symbol a block shows.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "≠",
            Self::Less => "<",
            Self::LessOrEqual => "≤",
            Self::Greater => ">",
            Self::GreaterOrEqual => "≥",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MathOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
}

impl MathOp {
    pub const ALL: [MathOp; 6] = [
        MathOp::Add,
        MathOp::Subtract,
        MathOp::Multiply,
        MathOp::Divide,
        MathOp::Modulo,
        MathOp::Power,
    ];

    pub fn lua(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::Power => "^",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "−",
            Self::Multiply => "×",
            Self::Divide => "÷",
            Self::Modulo => "mod",
            Self::Power => "^",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicOp {
    And,
    Or,
}

impl LogicOp {
    pub fn lua(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MathFunction {
    Abs,
    Floor,
    Ceil,
    Round,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Log,
    Exp,
    Sign,
}

impl MathFunction {
    pub const ALL: [MathFunction; 14] = [
        MathFunction::Round,
        MathFunction::Floor,
        MathFunction::Ceil,
        MathFunction::Abs,
        MathFunction::Sqrt,
        MathFunction::Sin,
        MathFunction::Cos,
        MathFunction::Tan,
        MathFunction::Asin,
        MathFunction::Acos,
        MathFunction::Atan,
        MathFunction::Log,
        MathFunction::Exp,
        MathFunction::Sign,
    ];

    /// The name a block shows.
    pub fn name(self) -> &'static str {
        match self {
            Self::Abs => "abs",
            Self::Floor => "floor",
            Self::Ceil => "ceiling",
            Self::Round => "round",
            Self::Sqrt => "sqrt",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Log => "log",
            Self::Exp => "exp",
            Self::Sign => "sign",
        }
    }
}

#[cfg(test)]
mod ui_graph_tests;
