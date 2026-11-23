//! The block editor: snap-together blocks on a canvas, in Axcenity's own
//! look (see "The look" below).
//!
//! On the left are the categories (Events, Control, Motion, ...) and a
//! palette of their blocks, with a search box and each category's Luau
//! blocks folded under "Lua blocks"; on the right, the script's canvas. Drag a block
//! from the palette onto the canvas, or next to a stack, where it snaps
//! into place. Dragging a block takes the blocks under it along, as in
//! Scratch, and dropping blocks on the palette deletes them. Reporters (the
//! round and pointed blocks) drop into slots: a pointed one (true or false)
//! fits any slot, a round one only round slots. Right-click a block to
//! duplicate or delete it, and a value slot to change what kind of value
//! it holds. "View as Lua" shows the Lua the blocks become.
//!
//! Blocks are painted here, piece by piece. egui only fills convex shapes,
//! so a block is filled as convex pieces (rectangles, the rounded tab
//! under it) and then outlined as one path.
//!
//! The look (usability brief §5): a category's colour is one deep hue its
//! white labels read on (`Category::hue`, the kit's `theme::Hue`), with a darker shade of it
//! as the block's edge and the shadow it casts, so stacks read as raised
//! pieces. Labels are Inter Medium. Blocks join with a rounded tab and
//! notch; an event block (a hat) has a flat top with a lighter band and
//! its category's icon, and the category list shows the same icons on
//! rounded swatches. Typed values sit in light rounded fields with dark
//! text, menus in darker ones; values that report are rounded rectangles
//! and true-or-false ones pointed. Sizes keep to a 4-point grid.
//!
//! Where to change behavior:
//! - `Category`, `sections` and `reporter_parts` — which pieces a block has
//! - `BlockPainter` — convex fills and the outline path
//! - `palette`, `show`, `show_canvas` and `finish_drag` — dragging, snapping and the toolbar

use std::collections::BTreeSet;

use axcenity_blocks::{
    Block, BlockKind, BlockScript, BlockShape, Comparison, Expression, LogicOp, MathFunction,
    MathOp, ObjectRef, TableField, VariableDef, clone_identifiers, translate, valid_identifier,
};
use axcenity_core::{ClassName, KeyCode};
use eframe::egui::{
    self, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2, pos2,
    vec2,
};

use crate::lua_syntax;
use crate::theme::{self, icons};

mod canvas;
mod graph;
mod menus;
mod painter;
mod palette;

use canvas::*;
use menus::*;
use painter::*;
use palette::*;

pub use graph::show_graph;
#[cfg(test)]
pub(crate) use graph::view_of;

/// An object a block can name: its path below the Workspace, such as
/// `Pad.Chime`, and its class.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectInfo {
    pub path: String,
    pub class: ClassName,
    /// For an Animator, the names its controller has, and for anything
    /// with attributes, their names, to offer in its blocks' menus.
    pub names: Vec<(NameKind, String)>,
}

/// What a name typed into an animation block names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameKind {
    Bool,
    Number,
    Trigger,
    State,
    Event,
    /// A state machine layer (Phase 78).
    Layer,
    /// A child's name, suggested from what's in the object.
    Child,
    /// A BindableEvent's path in ReplicatedStorage, suggested from the
    /// ReplicatedStorage entry of the objects (see [`STORAGE_PATH`]).
    Bindable,
    /// A data table's name, suggested from the same entry.
    DataTable,
    /// A column of a data table, suggested from the same entry.
    Column,
    /// An attribute's name, suggested from the attributes the object has
    /// in the editor.
    Attribute,
    /// One of an object's events, such as `Touched`, suggested from its
    /// class.
    Signal,
}

/// The objects' entry for ReplicatedStorage: the empty path, which no
/// Workspace object has. Its `names` are the BindableEvents in it, and the
/// project's data tables and their columns. It's a service, so object
/// menus leave it out.
pub const STORAGE_PATH: &str = "";

impl NameKind {
    /// What the menu offers for a block with nothing to suggest.
    fn example(self) -> &'static str {
        match self {
            NameKind::Bool => "IsGrounded",
            NameKind::Number => "Speed",
            NameKind::Trigger => "Jump",
            NameKind::State => "Idle",
            NameKind::Event => "Footstep",
            NameKind::Layer => "UpperBody",
            NameKind::Child => "Door",
            NameKind::Bindable => "Scored",
            NameKind::DataTable => "weapons",
            NameKind::Column => "Damage",
            NameKind::Attribute => "Health",
            NameKind::Signal => "Touched",
        }
    }

    fn what(self) -> &'static str {
        match self {
            NameKind::Bool => "Bool parameter",
            NameKind::Number => "Number parameter",
            NameKind::Trigger => "Trigger",
            NameKind::State => "state",
            NameKind::Event => "animation event",
            NameKind::Layer => "layer",
            NameKind::Child => "child",
            NameKind::Bindable => "BindableEvent",
            NameKind::DataTable => "data table",
            NameKind::Column => "column",
            NameKind::Attribute => "attribute",
            NameKind::Signal => "event",
        }
    }
}

/// What the block editor needs besides the script.
pub struct BlockContext<'a> {
    /// Tells this script's canvas (its scroll and zoom) apart from others'.
    pub key: &'a str,
    /// True for a client block script (a LocalScript).
    pub client: bool,
    /// The block a runtime error came from, outlined in red.
    pub highlighted: Option<u64>,
    /// What the blocks' object menus offer.
    pub objects: &'a [ObjectInfo],
}

// Sizes at zoom 1, in points, on a 4-point grid.
const FONT_SIZE: f32 = 13.0;
const ROW_HEIGHT: f32 = 40.0;
const PAD: f32 = 12.0;
const GAP: f32 = 8.0;
const MIN_WIDTH: f32 = 96.0;
const ARM: f32 = 16.0;
const MOUTH_MIN: f32 = 24.0;
const FOOT: f32 = 20.0;
const CORNER: f32 = 8.0;
const INNER_CORNER: f32 = 4.0;
/// A hat's top band, above its row.
const CAP: f32 = 8.0;
/// A hat's top corners, rounder than a block's.
const HAT_CORNER: f32 = 14.0;
const TAB_LEFT: f32 = 16.0;
const TAB_WIDTH: f32 = 28.0;
const TAB_DEPTH: f32 = 6.0;
/// How far below a block its shadow shows.
const DROP: f32 = 2.0;
const INPUT_HEIGHT: f32 = 24.0;
/// The corners of typed fields, menus and reporters.
const FIELD_CORNER: f32 = 6.0;
const REPORTER_MIN_HEIGHT: f32 = 32.0;
/// How near a dragged stack has to be to a place to snap into it.
const SNAP: f32 = 34.0;
const STACK_GAP: f32 = 32.0;
const CATEGORY_WIDTH: f32 = 74.0;
const PALETTE_WIDTH: f32 = 360.0;
const PALETTE_MIN_WIDTH: f32 = 180.0;
const PALETTE_MAX_WIDTH: f32 = 760.0;
const PALETTE_SPLITTER_WIDTH: f32 = 8.0;
const TOOLBAR_HEIGHT: f32 = 40.0;

/// Typed values: dark ink on a light field, 4.5:1 or more.
const INPUT_TEXT: Color32 = Color32::from_rgb(34, 38, 52);
const INPUT_FIELD: Color32 = Color32::from_rgb(250, 250, 252);
const ERROR_OUTLINE: Color32 = Color32::from_rgb(255, 72, 72);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Category {
    Events,
    Control,
    Motion,
    Looks,
    Sound,
    Operators,
    Variables,
    Sensing,
    /// An Animator's parameters and states.
    Animation,
    /// Properties moving smoothly (`TweenService`).
    Tweens,
    /// Data tables, save slots, checkpoints and debug print.
    DataAndSaves,
    /// Levels, data layers, streaming, cloning and destroying instances.
    WorldAndLevels,
    /// Hosting, joining, network ownership and chat.
    MultiplayerAndChat,
    /// Grey blocks of Lua that have no blocks, from a script switched from
    /// Code. Not in the palette.
    Lua,
}

impl Category {
    const ALL: [Category; 13] = [
        Category::Events,
        Category::Control,
        Category::Motion,
        Category::Looks,
        Category::Sound,
        Category::Operators,
        Category::Variables,
        Category::Sensing,
        Category::Animation,
        Category::Tweens,
        Category::DataAndSaves,
        Category::WorldAndLevels,
        Category::MultiplayerAndChat,
    ];

    fn name(self) -> &'static str {
        match self {
            Category::Events => "Events",
            Category::Control => "Control",
            Category::Motion => "Motion",
            Category::Looks => "Looks",
            Category::Sound => "Sound",
            Category::Operators => "Operators",
            Category::Variables => "Variables",
            Category::Sensing => "Sensing",
            Category::Animation => "Animation",
            Category::Tweens => "Tweens",
            Category::DataAndSaves => "Data & Saves",
            Category::WorldAndLevels => "World & Levels",
            Category::MultiplayerAndChat => "Multiplayer & Chat",
            Category::Lua => "Lua",
        }
    }

    /// Short label for the narrow category rail on the left.
    fn short_name(self) -> &'static str {
        match self {
            Category::DataAndSaves => "Data",
            Category::WorldAndLevels => "World",
            Category::MultiplayerAndChat => "Network",
            other => other.name(),
        }
    }

    /// The category's colour, darkened with Increase contrast until its
    /// white labels reach 4.5:1.
    fn fill(self, palette: &theme::Palette) -> Color32 {
        self.hue().fill(palette)
    }

    /// Axcenity's category colours, shared with the rest of the editor as
    /// the kit's [`theme::Hue`]s: warm for what starts and steers a script,
    /// cool for what it moves and shows, green for values.
    fn hue(self) -> theme::Hue {
        match self {
            Category::Events => theme::Hue::Amber,
            Category::Control => theme::Hue::Orange,
            Category::Variables => theme::Hue::Red,
            Category::Sound => theme::Hue::Magenta,
            Category::DataAndSaves => theme::Hue::Purple,
            Category::Looks => theme::Hue::Violet,
            Category::Tweens => theme::Hue::Indigo,
            Category::Motion => theme::Hue::Blue,
            Category::Sensing => theme::Hue::Cyan,
            Category::Animation => theme::Hue::Teal,
            Category::Operators => theme::Hue::Green,
            Category::WorldAndLevels => theme::Hue::Olive,
            Category::MultiplayerAndChat => theme::Hue::Bronze,
            Category::Lua => theme::Hue::Slate,
        }
    }

    /// The category's colour before Increase contrast.
    #[cfg(test)]
    fn base_fill(self) -> Color32 {
        self.hue().base()
    }

    /// The darker shade under a block (its edge and the shadow it casts),
    /// and the background of menus on the block.
    fn edge(self, palette: &theme::Palette) -> Color32 {
        self.hue().edge(palette)
    }

    /// What the category's blocks are for, drawn white on its swatch in
    /// the category list and at the start of its hat blocks.
    fn icon(self) -> &'static str {
        match self {
            Category::Events => icons::LIGHTNING,
            Category::Control => icons::FLOW_ARROW,
            Category::Motion => icons::ARROWS_OUT_CARDINAL,
            Category::Looks => icons::EYE,
            Category::Sound => icons::SPEAKER_HIGH,
            Category::Operators => icons::PLUS_MINUS,
            Category::Variables => icons::BRACKETS_CURLY,
            Category::Sensing => icons::HAND_POINTING,
            Category::Animation => icons::PERSON_SIMPLE_RUN,
            Category::Tweens => icons::WAVE_SINE,
            Category::DataAndSaves => icons::DATABASE,
            Category::WorldAndLevels => icons::GLOBE,
            Category::MultiplayerAndChat => icons::USERS,
            Category::Lua => icons::CODE,
        }
    }
}

fn block_category(kind: &BlockKind) -> Category {
    match kind {
        BlockKind::WhenStarted { .. }
        | BlockKind::WhenTouched { .. }
        | BlockKind::WhenTouchEnds { .. }
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
        | BlockKind::Disconnect { .. }
        | BlockKind::WhenClicked { .. }
        | BlockKind::WhenMouseEnters { .. }
        | BlockKind::WhenMouseLeaves { .. }
        | BlockKind::WhenValueChanged { .. }
        | BlockKind::EveryFrame { .. }
        | BlockKind::BeforePhysics { .. }
        | BlockKind::BeforeRender { .. }
        | BlockKind::WhenEventFired { .. }
        | BlockKind::FireEvent { .. }
        | BlockKind::WhenSoundEnded { .. }
        | BlockKind::WhenCutsceneFinished { .. }
        | BlockKind::WhenConnectionFails { .. }
        | BlockKind::WhenPromptTriggered { .. }
        | BlockKind::WhenPartClicked { .. }
        | BlockKind::WhenCommandRun { .. }
        | BlockKind::WhenPropertyChanged { .. }
        | BlockKind::WhenAttributeChanged { .. }
        | BlockKind::Unattached { .. }
        | BlockKind::RunScript { .. }
        | BlockKind::OnSignal { .. }
        | BlockKind::Connect { .. } => Category::Events,
        BlockKind::Wait { .. }
        | BlockKind::WaitUntil { .. }
        | BlockKind::Repeat { .. }
        | BlockKind::Forever { .. }
        | BlockKind::RepeatUntil { .. }
        | BlockKind::If { .. }
        | BlockKind::IfElse { .. }
        | BlockKind::StopScript
        | BlockKind::DestroyScript
        | BlockKind::ForRange { .. }
        | BlockKind::ForEachInList { .. }
        | BlockKind::ForEachTagged { .. }
        | BlockKind::Sequence { .. }
        | BlockKind::CooldownGate { .. }
        | BlockKind::SpawnAsync { .. }
        | BlockKind::ForEachChild { .. }
        | BlockKind::ForEachPartInRadius { .. }
        | BlockKind::ForEachPartTouching { .. }
        | BlockKind::WaitForChild { .. }
        | BlockKind::WaitForEvent { .. }
        | BlockKind::DestroyAfter { .. }
        | BlockKind::DefineFunction { .. }
        | BlockKind::While { .. }
        | BlockKind::RepeatLoop { .. }
        | BlockKind::NumericFor { .. }
        | BlockKind::GenericFor { .. }
        | BlockKind::DoBlock { .. }
        | BlockKind::Break
        | BlockKind::Continue
        | BlockKind::Return { .. } => Category::Control,
        BlockKind::MoveTo { .. }
        | BlockKind::MoveBy { .. }
        | BlockKind::LookAt { .. }
        | BlockKind::MoveForward { .. }
        | BlockKind::TurnTo { .. }
        | BlockKind::TurnBy { .. }
        | BlockKind::SlideTo { .. }
        | BlockKind::RotateTo { .. }
        | BlockKind::SetGravity { .. }
        | BlockKind::SetAngularVelocity { .. }
        | BlockKind::LaunchPart { .. }
        | BlockKind::SetAnchored { .. }
        | BlockKind::PushWithForce { .. }
        | BlockKind::SetPartVelocity { .. }
        | BlockKind::SetFriction { .. }
        | BlockKind::SetDestination { .. }
        | BlockKind::FollowTarget { .. }
        | BlockKind::StopAgent { .. }
        | BlockKind::SetVelocity { .. }
        | BlockKind::WhenAgentSenses { .. }
        | BlockKind::WhenAgentLosesPlayer { .. }
        | BlockKind::WhenAgentArrives { .. }
        | BlockKind::Patrol { .. }
        | BlockKind::PlaceOnSpline { .. }
        | BlockKind::ControlSpline { .. }
        | BlockKind::SetSplineSpeed { .. }
        | BlockKind::WhenSplineReached { .. }
        | BlockKind::WhenSeatOccupied { .. }
        | BlockKind::WhenSeatVacated { .. }
        | BlockKind::SitInSeat { .. }
        | BlockKind::LeaveSeat { .. }
        | BlockKind::SetCameraType { .. }
        | BlockKind::PointCameraAt { .. }
        | BlockKind::ShakeCamera { .. } => Category::Motion,
        BlockKind::SetColor { .. }
        | BlockKind::SetText { .. }
        | BlockKind::Show { .. }
        | BlockKind::Hide { .. }
        | BlockKind::ToggleVisibility { .. }
        | BlockKind::SetTransparency { .. }
        | BlockKind::SetMaterial { .. }
        | BlockKind::EmitParticles { .. }
        | BlockKind::SetEffectEnabled { .. }
        | BlockKind::SetLightingProperty { .. }
        | BlockKind::SetPostProcessing { .. }
        | BlockKind::TweenProperty { .. }
        | BlockKind::SetImage { .. }
        | BlockKind::ShowPage { .. }
        | BlockKind::ScrollTo { .. }
        | BlockKind::SetValue { .. }
        | BlockKind::SetProperty { .. }
        | BlockKind::SetToneMapping { .. }
        | BlockKind::SetClockTime { .. }
        | BlockKind::SetAtmosphere { .. }
        | BlockKind::SetLanguage { .. }
        | BlockKind::SetGraphicsQuality { .. }
        | BlockKind::SetGraphicsOption { .. }
        | BlockKind::SetGraphicsNumber { .. }
        | BlockKind::SpawnDecal { .. } => Category::Looks,
        BlockKind::PlaySound { .. }
        | BlockKind::PlaySoundAtPosition { .. }
        | BlockKind::StopSound { .. }
        | BlockKind::PauseSound { .. }
        | BlockKind::ResumeSound { .. }
        | BlockKind::SetVolume { .. }
        | BlockKind::FadeVolume { .. }
        | BlockKind::SetSoundPitch { .. }
        | BlockKind::SetSoundLooping { .. }
        | BlockKind::SetSoundTime { .. } => Category::Sound,
        BlockKind::WhenStateChanged { .. }
        | BlockKind::WhenAnimationEvent { .. }
        | BlockKind::WhenAnimationFinished { .. }
        | BlockKind::SetAnimatorBool { .. }
        | BlockKind::SetAnimatorNumber { .. }
        | BlockKind::SetAnimatorLayerWeight { .. }
        | BlockKind::SetAnimatorTrigger { .. }
        | BlockKind::ResetAnimatorTrigger { .. }
        | BlockKind::PlayAnimatorState { .. }
        | BlockKind::PlayAnimation { .. }
        | BlockKind::StopAnimation { .. }
        | BlockKind::SetAnimatorSpeed { .. }
        | BlockKind::SetIkTarget { .. }
        | BlockKind::SetRagdollEnabled { .. }
        | BlockKind::PlayCutscene { .. }
        | BlockKind::PauseCutscene { .. }
        | BlockKind::ResumeCutscene { .. }
        | BlockKind::StopCutscene { .. } => Category::Animation,
        BlockKind::Tween { .. }
        | BlockKind::SlideIn { .. }
        | BlockKind::SlideOut { .. }
        | BlockKind::FadeIn { .. }
        | BlockKind::FadeOut { .. }
        | BlockKind::TweenPivot { .. }
        | BlockKind::PulseScale { .. }
        | BlockKind::CancelTween { .. } => Category::Tweens,
        BlockKind::SetVariable { .. }
        | BlockKind::ChangeVariable { .. }
        | BlockKind::SetAttribute { .. }
        | BlockKind::ListCreate { .. }
        | BlockKind::ListAdd { .. }
        | BlockKind::ListInsert { .. }
        | BlockKind::ListRemoveIndex { .. }
        | BlockKind::ListRemoveValue { .. }
        | BlockKind::ListClear { .. }
        | BlockKind::Local { .. }
        | BlockKind::Assign { .. }
        | BlockKind::CompoundAssign { .. } => Category::Variables,
        BlockKind::Print { .. }
        | BlockKind::SaveSlot { .. }
        | BlockKind::LoadSlot { .. }
        | BlockKind::DeleteSlot { .. }
        | BlockKind::ForEachRow { .. }
        | BlockKind::SetCheckpoint { .. }
        | BlockKind::RestoreCheckpoint => Category::DataAndSaves,
        BlockKind::Destroy { .. }
        | BlockKind::CloneInto { .. }
        | BlockKind::LoadLevel { .. }
        | BlockKind::AddLevel { .. }
        | BlockKind::RemoveLevel { .. }
        | BlockKind::RequestStreamAround { .. }
        | BlockKind::WhenStreamedIn { .. }
        | BlockKind::WhenStreamedOut { .. }
        | BlockKind::WhenLayerChanged { .. }
        | BlockKind::SetLayerActive { .. } => Category::WorldAndLevels,
        BlockKind::HostGame { .. }
        | BlockKind::JoinGame { .. }
        | BlockKind::LeaveGame
        | BlockKind::SetNetworkOwner { .. }
        | BlockKind::SetNetworkOwnershipAuto { .. }
        | BlockKind::KickPlayer { .. }
        | BlockKind::WhenChatMessageReceived { .. }
        | BlockKind::SendChatMessage { .. }
        | BlockKind::ShowSystemMessage { .. } => Category::MultiplayerAndChat,
        BlockKind::Code { .. }
        | BlockKind::TopCode { .. }
        | BlockKind::Comment { .. }
        | BlockKind::CallFunction { .. }
        | BlockKind::CallMethod { .. } => Category::Lua,
    }
}

fn reporter_category(expression: &Expression) -> Category {
    match expression {
        Expression::Variable { .. }
        | Expression::BlockOutput { .. }
        | Expression::Attribute { .. }
        | Expression::ListItem { .. }
        | Expression::ListLength { .. }
        | Expression::ListContains { .. } => Category::Variables,
        Expression::Property { property, .. } if property == "CurrentState" => Category::Animation,
        Expression::Property { .. }
        | Expression::ObjectPath { .. }
        | Expression::TouchingPart
        | Expression::TriggeringPlayer
        | Expression::CommandArgs
        | Expression::DeltaTime
        | Expression::PartUnderMouse
        | Expression::MouseHitPosition
        | Expression::CameraPosition
        | Expression::CameraLookDirection
        | Expression::NearestPlayerTo { .. }
        | Expression::PlayerOfPart { .. }
        | Expression::PlayerIsAlive { .. }
        | Expression::ParentOf { .. }
        | Expression::ChildOf { .. }
        | Expression::HasTag { .. }
        | Expression::GameTime
        | Expression::KeyDown { .. }
        | Expression::Raycast { .. }
        | Expression::RaycastIgnoring { .. }
        | Expression::IsA { .. }
        | Expression::IsStreamedIn { .. }
        | Expression::SplinePosition { .. }
        | Expression::SplineClosestDistance { .. } => Category::Sensing,
        Expression::AnimatorBool { .. }
        | Expression::AnimatorNumber { .. }
        | Expression::AnimatorLayerWeight { .. }
        | Expression::AnimatorNewState
        | Expression::AnimatorPreviousState
        | Expression::AnimationEventName => Category::Animation,
        Expression::SoundProperty { .. } => Category::Sound,
        Expression::DataRow { .. }
        | Expression::DataValue { .. }
        | Expression::SlotExists { .. }
        | Expression::DataTableRowCount { .. } => Category::DataAndSaves,
        Expression::LayerActive { .. }
        | Expression::ChangedLayerName
        | Expression::ChangedLayerActive => Category::WorldAndLevels,
        Expression::ConnectionFailureReason
        | Expression::ChatMessageText
        | Expression::ChatMessageSender
        | Expression::IsHost
        | Expression::IsConnected => Category::MultiplayerAndChat,
        Expression::Code { .. }
        | Expression::Call { .. }
        | Expression::MethodCall { .. }
        | Expression::Field { .. }
        | Expression::Index { .. }
        | Expression::Table { .. }
        | Expression::Cast { .. } => Category::Lua,
        _ => Category::Operators,
    }
}

/// Semantic vector icon for each block, shown in both the Blocks and Graph
/// palettes and node/block headers.
pub(crate) fn block_icon(kind: &BlockKind) -> &'static str {
    match kind {
        BlockKind::WhenStarted { .. } | BlockKind::RunScript { .. } => icons::PLAY,
        BlockKind::WhenTouched { .. } | BlockKind::WhenTouchEnds { .. } => icons::HAND_POINTING,
        BlockKind::WhenKeyPressed { .. } | BlockKind::WhenKeyReleased { .. } => icons::KEYBOARD,
        BlockKind::WhenPlayerJoins { .. }
        | BlockKind::WhenPlayerLeaves { .. }
        | BlockKind::WhenCharacterSpawns { .. }
        | BlockKind::WhenPlayerDies { .. } => icons::USER,
        BlockKind::WhenChildAdded { .. } | BlockKind::WhenChildRemoved { .. } => icons::FLOW_ARROW,
        BlockKind::WhenTagAdded { .. }
        | BlockKind::WhenTagRemoved { .. }
        | BlockKind::ForEachTagged { .. } => icons::TAG,
        BlockKind::WhenRemoteFires { .. } => icons::BROADCAST,
        BlockKind::Disconnect { .. } => icons::LINK_BREAK,
        BlockKind::WhenClicked { .. } | BlockKind::WhenPartClicked { .. } => icons::CURSOR_CLICK,
        BlockKind::WhenMouseEnters { .. } | BlockKind::WhenMouseLeaves { .. } => icons::CURSOR,
        BlockKind::WhenValueChanged { .. }
        | BlockKind::WhenPropertyChanged { .. }
        | BlockKind::WhenAttributeChanged { .. } => icons::SLIDERS_HORIZONTAL,
        BlockKind::EveryFrame { .. }
        | BlockKind::BeforePhysics { .. }
        | BlockKind::BeforeRender { .. } => icons::TIMER,
        BlockKind::WhenEventFired { .. } | BlockKind::FireEvent { .. } => icons::BROADCAST,
        BlockKind::WhenSoundEnded { .. } => icons::SPEAKER_HIGH,
        BlockKind::WhenCutsceneFinished { .. } => icons::FILM_STRIP,
        BlockKind::WhenConnectionFails { .. } => icons::CLOUD_WARNING,
        BlockKind::WhenPromptTriggered { .. } => icons::LIGHTNING,
        BlockKind::WhenCommandRun { .. } => icons::TERMINAL_WINDOW,
        BlockKind::OnSignal { .. } | BlockKind::Connect { .. } => icons::LINK,
        BlockKind::Unattached { .. } => icons::DOTS_THREE,
        BlockKind::Wait { .. }
        | BlockKind::WaitUntil { .. }
        | BlockKind::WaitForChild { .. }
        | BlockKind::WaitForEvent { .. } => icons::CLOCK,
        BlockKind::Repeat { .. }
        | BlockKind::Forever { .. }
        | BlockKind::RepeatUntil { .. }
        | BlockKind::While { .. }
        | BlockKind::RepeatLoop { .. }
        | BlockKind::NumericFor { .. }
        | BlockKind::GenericFor { .. }
        | BlockKind::ForEachChild { .. }
        | BlockKind::ForEachPartInRadius { .. }
        | BlockKind::ForEachPartTouching { .. }
        | BlockKind::ForRange { .. }
        | BlockKind::ForEachInList { .. } => icons::REPEAT,
        BlockKind::If { .. } | BlockKind::IfElse { .. } | BlockKind::Sequence { .. } => {
            icons::GIT_BRANCH
        }
        BlockKind::CooldownGate { .. } => icons::SHIELD,
        BlockKind::SpawnAsync { .. } => icons::LIGHTNING,
        BlockKind::StopScript
        | BlockKind::DestroyScript
        | BlockKind::Break
        | BlockKind::Continue
        | BlockKind::Return { .. } => icons::STOP,
        BlockKind::DestroyAfter { .. } | BlockKind::Destroy { .. } => icons::TRASH,
        BlockKind::DoBlock { .. } | BlockKind::DefineFunction { .. } => icons::FLOW_ARROW,
        BlockKind::MoveTo { .. }
        | BlockKind::MoveBy { .. }
        | BlockKind::MoveForward { .. }
        | BlockKind::SlideTo { .. }
        | BlockKind::PlaceOnSpline { .. } => icons::ARROWS_OUT_CARDINAL,
        BlockKind::TurnTo { .. }
        | BlockKind::TurnBy { .. }
        | BlockKind::RotateTo { .. }
        | BlockKind::SetAngularVelocity { .. }
        | BlockKind::LookAt { .. } => icons::ARROWS_CLOCKWISE,
        BlockKind::SetAnchored { .. } | BlockKind::SetGravity { .. } => icons::ANCHOR,
        BlockKind::PushWithForce { .. }
        | BlockKind::SetPartVelocity { .. }
        | BlockKind::LaunchPart { .. }
        | BlockKind::SetFriction { .. } => icons::ARROW_RIGHT,
        BlockKind::SetDestination { .. }
        | BlockKind::FollowTarget { .. }
        | BlockKind::SetVelocity { .. }
        | BlockKind::Patrol { .. } => icons::PERSON_SIMPLE_RUN,
        BlockKind::StopAgent { .. } => icons::STOP,
        BlockKind::ControlSpline { .. } | BlockKind::SetSplineSpeed { .. } => icons::PLAY_CIRCLE,
        BlockKind::WhenSeatOccupied { .. }
        | BlockKind::WhenSeatVacated { .. }
        | BlockKind::SitInSeat { .. }
        | BlockKind::LeaveSeat { .. } => icons::USER,
        BlockKind::WhenAgentSenses { .. } | BlockKind::WhenAgentLosesPlayer { .. } => icons::EYES,
        BlockKind::WhenAgentArrives { .. } | BlockKind::WhenSplineReached { .. } => {
            icons::FLAG_CHECKERED
        }
        BlockKind::SetCameraType { .. }
        | BlockKind::PointCameraAt { .. }
        | BlockKind::ShakeCamera { .. } => icons::CAMERA,
        BlockKind::SetColor { .. }
        | BlockKind::SetTransparency { .. }
        | BlockKind::SetMaterial { .. } => icons::PALETTE,
        BlockKind::SetText { .. } | BlockKind::SetLanguage { .. } => icons::PENCIL_SIMPLE,
        BlockKind::Show { .. } | BlockKind::ToggleVisibility { .. } => icons::EYE,
        BlockKind::Hide { .. } => icons::EYE_SLASH,
        BlockKind::ShowPage { .. }
        | BlockKind::ScrollTo { .. }
        | BlockKind::SetValue { .. }
        | BlockKind::SetProperty { .. } => icons::SLIDERS,
        BlockKind::SetImage { .. } => icons::IMAGE,
        BlockKind::EmitParticles { .. } | BlockKind::SetEffectEnabled { .. } => icons::SPARKLE,
        BlockKind::TweenProperty { .. } => icons::WAVE_SINE,
        BlockKind::SetAttribute { .. } => icons::TAG,
        BlockKind::SetToneMapping { .. }
        | BlockKind::SetClockTime { .. }
        | BlockKind::SetLightingProperty { .. }
        | BlockKind::SetPostProcessing { .. } => icons::SUN,
        BlockKind::SetAtmosphere { .. } => icons::CLOUD_SUN,
        BlockKind::SetGraphicsQuality { .. }
        | BlockKind::SetGraphicsOption { .. }
        | BlockKind::SetGraphicsNumber { .. } => icons::SLIDERS_HORIZONTAL,
        BlockKind::SpawnDecal { .. } => icons::SPARKLE,
        BlockKind::PlaySound { .. } | BlockKind::PlaySoundAtPosition { .. } => icons::SPEAKER_HIGH,
        BlockKind::StopSound { .. } => icons::SPEAKER_SLASH,
        BlockKind::PauseSound { .. } => icons::PAUSE,
        BlockKind::ResumeSound { .. } => icons::PLAY_CIRCLE,
        BlockKind::SetVolume { .. }
        | BlockKind::FadeVolume { .. }
        | BlockKind::SetSoundPitch { .. }
        | BlockKind::SetSoundLooping { .. }
        | BlockKind::SetSoundTime { .. } => icons::WAVEFORM,
        BlockKind::SetAnimatorTrigger { .. }
        | BlockKind::ResetAnimatorTrigger { .. }
        | BlockKind::SetAnimatorBool { .. }
        | BlockKind::SetAnimatorNumber { .. }
        | BlockKind::SetAnimatorLayerWeight { .. }
        | BlockKind::PlayAnimatorState { .. }
        | BlockKind::PlayAnimation { .. }
        | BlockKind::StopAnimation { .. }
        | BlockKind::SetAnimatorSpeed { .. }
        | BlockKind::SetIkTarget { .. }
        | BlockKind::SetRagdollEnabled { .. }
        | BlockKind::WhenStateChanged { .. }
        | BlockKind::WhenAnimationEvent { .. }
        | BlockKind::WhenAnimationFinished { .. } => icons::PERSON_SIMPLE_RUN,
        BlockKind::PlayCutscene { .. }
        | BlockKind::PauseCutscene { .. }
        | BlockKind::ResumeCutscene { .. }
        | BlockKind::StopCutscene { .. } => icons::FILM_STRIP,
        BlockKind::Tween { .. }
        | BlockKind::SlideIn { .. }
        | BlockKind::SlideOut { .. }
        | BlockKind::FadeIn { .. }
        | BlockKind::FadeOut { .. }
        | BlockKind::TweenPivot { .. }
        | BlockKind::PulseScale { .. }
        | BlockKind::CancelTween { .. } => icons::WAVE_SINE,
        BlockKind::SetVariable { .. }
        | BlockKind::ChangeVariable { .. }
        | BlockKind::Local { .. }
        | BlockKind::Assign { .. }
        | BlockKind::CompoundAssign { .. } => icons::BRACKETS_CURLY,
        BlockKind::ListCreate { .. }
        | BlockKind::ListAdd { .. }
        | BlockKind::ListInsert { .. }
        | BlockKind::ListRemoveIndex { .. }
        | BlockKind::ListRemoveValue { .. }
        | BlockKind::ListClear { .. } => icons::LIST,
        BlockKind::Print { .. } => icons::TERMINAL_WINDOW,
        BlockKind::CloneInto { .. } => icons::COPY,
        BlockKind::SaveSlot { .. } => icons::FLOPPY_DISK,
        BlockKind::LoadSlot { .. } => icons::FOLDER_OPEN,
        BlockKind::DeleteSlot { .. } => icons::TRASH,
        BlockKind::ForEachRow { .. } => icons::TABLE,
        BlockKind::SetCheckpoint { .. } | BlockKind::RestoreCheckpoint => icons::FLAG,
        BlockKind::LoadLevel { .. }
        | BlockKind::AddLevel { .. }
        | BlockKind::RemoveLevel { .. } => icons::MAP_TRIFOLD,
        BlockKind::WhenLayerChanged { .. } | BlockKind::SetLayerActive { .. } => icons::STACK,
        BlockKind::RequestStreamAround { .. }
        | BlockKind::WhenStreamedIn { .. }
        | BlockKind::WhenStreamedOut { .. } => icons::GLOBE,
        BlockKind::HostGame { .. }
        | BlockKind::JoinGame { .. }
        | BlockKind::LeaveGame
        | BlockKind::SetNetworkOwner { .. }
        | BlockKind::SetNetworkOwnershipAuto { .. } => icons::USERS,
        BlockKind::KickPlayer { .. } => icons::USER_MINUS,
        BlockKind::WhenChatMessageReceived { .. }
        | BlockKind::SendChatMessage { .. }
        | BlockKind::ShowSystemMessage { .. } => icons::CHAT_CIRCLE_TEXT,
        BlockKind::Code { .. }
        | BlockKind::TopCode { .. }
        | BlockKind::Comment { .. }
        | BlockKind::CallFunction { .. }
        | BlockKind::CallMethod { .. } => icons::CODE,
    }
}

/// Semantic vector icon for each reporter/value node.
pub(crate) fn reporter_icon(expression: &Expression) -> &'static str {
    match expression {
        Expression::Variable { .. } | Expression::Name { .. } | Expression::BlockOutput { .. } => {
            icons::BRACKETS_CURLY
        }
        Expression::ListItem { .. }
        | Expression::ListLength { .. }
        | Expression::ListContains { .. } => icons::LIST,
        Expression::ObjectPath { .. } => icons::CUBE,
        Expression::Property { .. } => icons::SLIDERS,
        Expression::Attribute { .. } => icons::TAG,
        Expression::TouchingPart => icons::HAND_POINTING,
        Expression::TriggeringPlayer
        | Expression::NearestPlayerTo { .. }
        | Expression::PlayerOfPart { .. }
        | Expression::PlayerIsAlive { .. } => icons::USER,
        Expression::CommandArgs => icons::TERMINAL_WINDOW,
        Expression::DeltaTime | Expression::GameTime => icons::CLOCK,
        Expression::PartUnderMouse | Expression::MouseHitPosition => icons::CURSOR_CLICK,
        Expression::CameraPosition | Expression::CameraLookDirection => icons::CAMERA,
        Expression::ParentOf { .. } | Expression::ChildOf { .. } => icons::TREE_STRUCTURE,
        Expression::HasTag { .. } => icons::TAG,
        Expression::KeyDown { .. } => icons::KEYBOARD,
        Expression::Raycast { .. } | Expression::RaycastIgnoring { .. } => icons::CROSSHAIR,
        Expression::IsA { .. } => icons::INFO,
        Expression::IsStreamedIn { .. } => icons::GLOBE,
        Expression::SplinePosition { .. } | Expression::SplineClosestDistance { .. } => {
            icons::RULER
        }
        Expression::AnimatorBool { .. }
        | Expression::AnimatorNumber { .. }
        | Expression::AnimatorLayerWeight { .. }
        | Expression::AnimatorNewState
        | Expression::AnimatorPreviousState
        | Expression::AnimationEventName => icons::PERSON_SIMPLE_RUN,
        Expression::DataRow { .. }
        | Expression::DataValue { .. }
        | Expression::DataTableRowCount { .. } => icons::TABLE,
        Expression::SlotExists { .. } => icons::FLOPPY_DISK,
        Expression::SoundProperty { .. } => icons::WAVEFORM,
        Expression::LayerActive { .. }
        | Expression::ChangedLayerName
        | Expression::ChangedLayerActive => icons::STACK,
        Expression::ConnectionFailureReason => icons::CLOUD_WARNING,
        Expression::ChatMessageText | Expression::ChatMessageSender => icons::CHAT_CIRCLE_TEXT,
        Expression::IsHost | Expression::IsConnected => icons::USERS,
        Expression::Random { .. } | Expression::RandomNumber { .. } => icons::SHUFFLE,
        Expression::Lerp { .. } => icons::WAVE_SINE,
        Expression::MakeColorRgb { .. } | Expression::MakeColorHsv { .. } => icons::PALETTE,
        Expression::MakeVector3 { .. }
        | Expression::VectorAxis { .. }
        | Expression::Distance { .. }
        | Expression::Direction { .. }
        | Expression::Magnitude { .. } => icons::COMPASS,
        Expression::Code { .. }
        | Expression::Call { .. }
        | Expression::MethodCall { .. }
        | Expression::Field { .. }
        | Expression::Index { .. }
        | Expression::Table { .. }
        | Expression::Cast { .. } => icons::CODE,
        _ => icons::PLUS_MINUS,
    }
}

/// Which objects an object menu offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    Parts,
    Sounds,
    Buttons,
    Texts,
    PageLayouts,
    ScrollingFrames,
    /// Sliders, Toggles, Dropdowns and ProgressBars: what has a `Value`.
    Widgets,
    Gui,
    Animators,
    Animations,
    IkControls,
    Ragdolls,
    Sequences,
    /// NavigationAgents.
    Agents,
    Splines,
    SplineFollowers,
    Checkpoints,
    Seats,
    Prompts,
    ClickDetectors,
    /// ThirdPersonControllers and FirstPersonControllers.
    Controllers,
    Any,
    /// Anything, or the Workspace itself (an empty path): where children are.
    Places,
}

impl Filter {
    fn matches(self, class: ClassName) -> bool {
        match self {
            Filter::Parts => class.is_base_part(),
            Filter::Sounds => class == ClassName::Sound,
            Filter::Buttons => class == ClassName::TextButton,
            Filter::Texts => {
                matches!(
                    class,
                    ClassName::TextLabel | ClassName::TextButton | ClassName::TextBox
                )
            }
            Filter::PageLayouts => class == ClassName::PageLayout,
            Filter::ScrollingFrames => class == ClassName::ScrollingFrame,
            Filter::Widgets => class.is_value_widget(),
            Filter::Animators => class == ClassName::Animator,
            Filter::Animations => class == ClassName::Animation,
            Filter::IkControls => class == ClassName::IKControl,
            Filter::Ragdolls => class == ClassName::Ragdoll,
            Filter::Sequences => class == ClassName::Sequence,
            Filter::Agents => class == ClassName::NavigationAgent,
            Filter::Splines => class == ClassName::Spline,
            Filter::SplineFollowers => class == ClassName::SplineFollower,
            Filter::Checkpoints => class == ClassName::Checkpoint,
            Filter::Seats => class.is_seat(),
            Filter::Prompts => class == ClassName::ProximityPrompt,
            Filter::ClickDetectors => class == ClassName::ClickDetector,
            Filter::Controllers => class.is_character_controller(),
            Filter::Gui => matches!(
                class,
                ClassName::ScreenGui
                    | ClassName::Frame
                    | ClassName::TextLabel
                    | ClassName::TextButton
                    | ClassName::ImageLabel
                    | ClassName::TextBox
                    | ClassName::PageLayout
                    | ClassName::Table
                    | ClassName::ScrollingFrame
                    | ClassName::Slider
                    | ClassName::Toggle
                    | ClassName::Dropdown
                    | ClassName::ProgressBar
            ),
            // Services (the ReplicatedStorage entry) aren't in the Workspace.
            Filter::Any | Filter::Places => !class.is_service(),
        }
    }

    /// A name to use when the Workspace has no object of this kind yet.
    fn placeholder(self) -> &'static str {
        match self {
            Filter::Parts | Filter::Any => "Part",
            Filter::Places => "",
            Filter::Sounds => "Sound",
            Filter::Buttons => "TextButton",
            Filter::Texts => "TextLabel",
            Filter::PageLayouts => "PageLayout",
            Filter::ScrollingFrames => "ScrollingFrame",
            Filter::Widgets => "Slider",
            Filter::Gui => "Frame",
            Filter::Animators => "Model.Animator",
            Filter::Animations => "Model.Animation",
            Filter::IkControls => "Model.IKControl",
            Filter::Ragdolls => "Model.Ragdoll",
            Filter::Sequences => "Sequence",
            Filter::Agents => "Model.NavigationAgent",
            Filter::Splines => "Spline",
            Filter::SplineFollowers => "Part.SplineFollower",
            Filter::Checkpoints => "Checkpoint",
            Filter::Seats => "Seat",
            Filter::Prompts => "Part.ProximityPrompt",
            Filter::ClickDetectors => "Part.ClickDetector",
            Filter::Controllers => "Character.ThirdPersonController",
        }
    }
}

/// What a value slot expects, which decides its look, the value it's
/// reset to, and which reporters fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotKind {
    Number,
    Text,
    /// Any value: right-click to pick which kind.
    Any,
    /// A pointed slot (if, until): only true-or-false reporters fit.
    Condition,
    /// True or false, picked from a menu.
    Truth,
    Vector,
    Color,
    /// A scene object picked from a dropdown, or wired/dropped dynamically.
    Object(Filter),
}

impl SlotKind {
    fn accepts(self, shape: BlockShape) -> bool {
        let slot = if self == SlotKind::Condition {
            BlockShape::Boolean
        } else {
            BlockShape::Value
        };
        slot.accepts(shape)
    }

    /// What's left in the slot when its reporter is dragged out.
    fn empty(self) -> Expression {
        match self {
            SlotKind::Number => Expression::Number { value: 0.0 },
            SlotKind::Text | SlotKind::Any => Expression::Text {
                value: String::new(),
            },
            SlotKind::Condition => Expression::Boolean { value: false },
            SlotKind::Truth => Expression::Boolean { value: true },
            SlotKind::Vector => Expression::Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            SlotKind::Color => Expression::Color3 {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
            SlotKind::Object(_) => Expression::ObjectPath {
                path: String::new(),
            },
        }
    }
}

/// One thing in a block's row.
enum Part<'a> {
    Label(String),
    Icon(&'static str),
    Slot(&'a mut Expression, SlotKind),
    Property(&'a mut String, Option<ClassName>),
    Variable(&'a mut String),
    Key(&'a mut String),
    /// A name in an Animator's controller (a parameter, state or event),
    /// with the Animator's path, for the menu's suggestions.
    Name(&'a mut String, NameKind, String),
    Math(&'a mut MathOp),
    Compare(&'a mut Comparison),
    Logic(&'a mut LogicOp),
    Function(&'a mut MathFunction),
    /// One of a fixed list of names, such as an easing style.
    Choice(&'a mut String, &'static [&'static str]),
    /// Lua kept word for word, in a grey block. Switch to Code to change it.
    Code(String),
    /// A name or some Lua typed in: a method, parameters, a type.
    Text(&'a mut String),
    /// A function in a value slot: its parameters. Its blocks are a mouth,
    /// so it's not a slot anything can be dropped into, but it counts as one.
    Params(&'a mut String),
}

/// A row of a block, or the mouth of a C-shaped block (if, repeat, ...)
/// with the blocks inside it.
enum Section<'a> {
    Row(Vec<Part<'a>>),
    Mouth(&'a mut Vec<Block>),
}

/// The most lines of Lua a grey block shows.
const CODE_LINES: usize = 14;

/// The Lua a grey block shows: long code is cut short.
fn shown_code(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    if lines.len() <= CODE_LINES {
        return code.to_owned();
    }
    let mut shown = lines[..CODE_LINES - 1].join(
        "
",
    );
    shown.push_str(&format!(
        "
-- … {} more lines (switch to Code to see them)",
        lines.len() - (CODE_LINES - 1)
    ));
    shown
}

fn label(text: &str) -> Part<'static> {
    Part::Label(text.to_owned())
}

fn object(target: &mut ObjectRef, filter: Filter) -> Part<'_> {
    Part::Slot(target.as_expr_mut(), SlotKind::Object(filter))
}

fn class_of(objects: &[ObjectInfo], target: &ObjectRef) -> Option<ClassName> {
    let path = target.as_path()?;
    objects
        .iter()
        .find(|object| object.path == path)
        .map(|object| object.class)
}

/// A value in a Luau block: a slot, or a function's parameters and a mouth
/// for its blocks.
fn push_value<'a>(
    value: &'a mut Expression,
    parts: &mut Vec<Part<'a>>,
    mouths: &mut Vec<Section<'a>>,
) {
    match value {
        Expression::Closure { params, body, .. } => {
            parts.push(label("function"));
            parts.push(Part::Params(params));
            mouths.push(Section::Mouth(body));
        }
        value => parts.push(Part::Slot(value, SlotKind::Any)),
    }
}

/// Values in a row, with commas between them.
fn push_values<'a>(
    values: &'a mut [Expression],
    parts: &mut Vec<Part<'a>>,
    mouths: &mut Vec<Section<'a>>,
) {
    for (index, value) in values.iter_mut().enumerate() {
        if index > 0 {
            parts.push(label(","));
        }
        push_value(value, parts, mouths);
    }
}

/// A row, then the mouths of the functions in it.
fn row_and_mouths<'a>(parts: Vec<Part<'a>>, mouths: Vec<Section<'a>>) -> Vec<Section<'a>> {
    let mut sections = vec![Section::Row(parts)];
    sections.extend(mouths);
    sections
}

/// What a Luau block shows, top to bottom.
fn luau_sections(kind: &mut BlockKind) -> Vec<Section<'_>> {
    use Part::{Slot, Text};
    use Section::{Mouth, Row};
    let mut parts = Vec::new();
    let mut mouths = Vec::new();
    match kind {
        BlockKind::RunScript { .. } => vec![Row(vec![label("when script runs")])],
        BlockKind::OnSignal {
            signal,
            method,
            params,
            ..
        } => vec![Row(vec![
            label("when"),
            Slot(signal, SlotKind::Any),
            Part::Choice(method, &axcenity_blocks::SIGNAL_METHODS),
            label("gives"),
            Text(params),
        ])],
        BlockKind::Connect {
            signal,
            method,
            params,
            body,
        } => vec![
            Row(vec![
                label("when"),
                Slot(signal, SlotKind::Any),
                Part::Choice(method, &axcenity_blocks::SIGNAL_METHODS),
                label("gives"),
                Text(params),
            ]),
            Mouth(body),
        ],
        BlockKind::Comment { text } if text.contains('\n') => {
            vec![Row(vec![Part::Code(shown_code(text))])]
        }
        BlockKind::Comment { text } => vec![Row(vec![Text(text)])],
        BlockKind::Local { names, values } => {
            parts.push(label("local"));
            parts.push(Text(names));
            if !values.is_empty() {
                parts.push(label("="));
                push_values(values, &mut parts, &mut mouths);
            }
            row_and_mouths(parts, mouths)
        }
        BlockKind::Assign { targets, values } => {
            for target in targets {
                parts.push(Slot(target, SlotKind::Any));
            }
            parts.push(label("="));
            push_values(values, &mut parts, &mut mouths);
            row_and_mouths(parts, mouths)
        }
        BlockKind::CompoundAssign { target, op, value } => vec![Row(vec![
            Slot(target, SlotKind::Any),
            Part::Choice(op, &axcenity_blocks::LUA_COMPOUND_OPS),
            label("="),
            Slot(value, SlotKind::Any),
        ])],
        BlockKind::CallFunction { function, args } => {
            parts.push(Slot(function, SlotKind::Any));
            parts.push(label("("));
            push_values(args, &mut parts, &mut mouths);
            parts.push(label(")"));
            row_and_mouths(parts, mouths)
        }
        BlockKind::CallMethod {
            object,
            method,
            args,
        } => {
            parts.push(Slot(object, SlotKind::Any));
            parts.push(label(":"));
            parts.push(Text(method));
            parts.push(label("("));
            push_values(args, &mut parts, &mut mouths);
            parts.push(label(")"));
            row_and_mouths(parts, mouths)
        }
        BlockKind::DefineFunction {
            name,
            local,
            params,
            generics,
            returns,
            attributes,
            body,
        } => {
            let mut row = Vec::new();
            if !attributes.is_empty() {
                row.push(label(attributes));
            }
            row.push(label(if *local { "local function" } else { "function" }));
            row.push(Text(name));
            if !generics.is_empty() {
                row.push(label(generics));
            }
            row.push(label("("));
            row.push(Text(params));
            row.push(label(")"));
            if !returns.is_empty() {
                row.push(label(returns));
            }
            vec![Row(row), Mouth(body)]
        }
        BlockKind::While { condition, body } => vec![
            Row(vec![
                label("while"),
                Slot(condition, SlotKind::Condition),
                label("do"),
            ]),
            Mouth(body),
        ],
        BlockKind::RepeatLoop { body, condition } => vec![
            Row(vec![label("repeat")]),
            Mouth(body),
            Row(vec![label("until"), Slot(condition, SlotKind::Condition)]),
        ],
        BlockKind::NumericFor {
            variable,
            start,
            stop,
            step,
            body,
        } => {
            let mut row = vec![
                label("for"),
                Text(variable),
                label("="),
                Slot(start, SlotKind::Any),
                label(","),
                Slot(stop, SlotKind::Any),
            ];
            if let Some(step) = step {
                row.push(label(","));
                row.push(Slot(step, SlotKind::Any));
            }
            row.push(label("do"));
            vec![Row(row), Mouth(body)]
        }
        BlockKind::GenericFor {
            names,
            values,
            body,
        } => {
            parts.push(label("for"));
            parts.push(Text(names));
            parts.push(label("in"));
            push_values(values, &mut parts, &mut mouths);
            parts.push(label("do"));
            let mut sections = vec![Row(parts), Mouth(body)];
            sections.extend(mouths);
            sections
        }
        BlockKind::DoBlock { body } => vec![Row(vec![label("do")]), Mouth(body)],
        BlockKind::Break => vec![Row(vec![label("break")])],
        BlockKind::Continue => vec![Row(vec![label("continue")])],
        BlockKind::Return { values } => {
            parts.push(label("return"));
            push_values(values, &mut parts, &mut mouths);
            row_and_mouths(parts, mouths)
        }
        _ => Vec::new(),
    }
}

/// What a block shows, top to bottom. An event block's own body isn't
/// here: it's the stack under it.
fn sections<'a>(kind: &'a mut BlockKind, objects: &[ObjectInfo]) -> Vec<Section<'a>> {
    use Part::{Key, Property, Slot, Variable};
    use Section::{Mouth, Row};
    match kind {
        BlockKind::WhenStarted { .. } => {
            vec![Row(vec![label("when script starts")])]
        }
        BlockKind::WhenTouched {
            part,
            filter,
            filter_value,
            cooldown,
            ..
        } => {
            if filter.is_empty() {
                *filter = "Any Part".to_owned();
            }
            if filter == "Tagged / Named..." && filter_value.is_none() {
                *filter_value = Some(Expression::Text {
                    value: String::new(),
                });
            } else if filter != "Tagged / Named..." && filter_value.is_some() {
                *filter_value = None;
            }
            let mut row = vec![
                label("when"),
                object(part, Filter::Parts),
                label("is touched"),
                Part::Choice(filter, &axcenity_blocks::TOUCH_FILTERS),
            ];
            if let Some(fv) = filter_value {
                row.push(Slot(fv, SlotKind::Text));
            }
            if let Some(cd) = cooldown {
                row.push(label("cooldown"));
                row.push(Slot(cd, SlotKind::Number));
                row.push(label("s"));
            }
            vec![Row(row)]
        }
        BlockKind::WhenTouchEnds {
            part,
            filter,
            filter_value,
            ..
        } => {
            if filter.is_empty() {
                *filter = "Any Part".to_owned();
            }
            if filter == "Tagged / Named..." && filter_value.is_none() {
                *filter_value = Some(Expression::Text {
                    value: String::new(),
                });
            } else if filter != "Tagged / Named..." && filter_value.is_some() {
                *filter_value = None;
            }
            let mut row = vec![
                label("when"),
                object(part, Filter::Parts),
                label("touch ends"),
                Part::Choice(filter, &axcenity_blocks::TOUCH_FILTERS),
            ];
            if let Some(fv) = filter_value {
                row.push(Slot(fv, SlotKind::Text));
            }
            vec![Row(row)]
        }
        BlockKind::WhenKeyPressed { key, .. } => {
            vec![Row(vec![label("when"), Key(key), label("key pressed")])]
        }
        BlockKind::WhenKeyReleased { key, .. } => {
            vec![Row(vec![label("when"), Key(key), label("key released")])]
        }
        BlockKind::WhenPlayerJoins { .. } => vec![Row(vec![label("when player joins")])],
        BlockKind::WhenPlayerLeaves { .. } => vec![Row(vec![label("when player leaves")])],
        BlockKind::WhenCharacterSpawns { .. } => vec![Row(vec![label("when character spawns")])],
        BlockKind::WhenPlayerDies { .. } => vec![Row(vec![label("when player dies")])],
        BlockKind::WhenChildAdded { target, .. } => vec![Row(vec![
            label("when child added to"),
            object(target, Filter::Places),
        ])],
        BlockKind::WhenChildRemoved { target, .. } => vec![Row(vec![
            label("when child removed from"),
            object(target, Filter::Places),
        ])],
        BlockKind::WhenTagAdded { tag, .. } => vec![Row(vec![
            label("when tag"),
            Slot(tag, SlotKind::Text),
            label("added"),
        ])],
        BlockKind::WhenTagRemoved { tag, .. } => vec![Row(vec![
            label("when tag"),
            Slot(tag, SlotKind::Text),
            label("removed"),
        ])],
        BlockKind::WhenRemoteFires { remote, side, .. } => {
            if side.is_empty() {
                *side = "Server (OnServerEvent)".to_owned();
            }
            vec![Row(vec![
                label("when remote"),
                object(remote, Filter::Any),
                label("fires on"),
                Part::Choice(side, &axcenity_blocks::REMOTE_SIDES),
            ])]
        }
        BlockKind::Disconnect { connection } => vec![Row(vec![
            label("disconnect"),
            Slot(connection, SlotKind::Any),
        ])],
        BlockKind::WhenClicked { button, .. } => vec![Row(vec![
            label("when"),
            object(button, Filter::Buttons),
            label("is clicked"),
        ])],
        BlockKind::WhenMouseEnters { element, .. } => vec![Row(vec![
            label("when mouse enters"),
            object(element, Filter::Gui),
        ])],
        BlockKind::WhenMouseLeaves { element, .. } => vec![Row(vec![
            label("when mouse leaves"),
            object(element, Filter::Gui),
        ])],
        BlockKind::WhenValueChanged { widget, .. } => vec![Row(vec![
            label("when"),
            object(widget, Filter::Widgets),
            label("value changes"),
        ])],
        BlockKind::EveryFrame { .. } => vec![Row(vec![label("every frame")])],
        BlockKind::BeforePhysics { .. } => vec![Row(vec![label("every frame, before physics")])],
        BlockKind::BeforeRender { .. } => vec![Row(vec![label("every frame, before drawing")])],
        BlockKind::WhenEventFired { event, .. } => vec![Row(vec![
            label("when event"),
            Part::Name(event, NameKind::Bindable, STORAGE_PATH.to_owned()),
            label("fires"),
        ])],
        BlockKind::FireEvent { event, data } => {
            let mut row = vec![
                label("fire event"),
                Part::Name(event, NameKind::Bindable, STORAGE_PATH.to_owned()),
            ];
            if let Some(payload) = data {
                row.push(label("with"));
                row.push(Slot(payload, SlotKind::Any));
            }
            vec![Row(row)]
        }
        BlockKind::WhenSoundEnded { sound, .. } => vec![Row(vec![
            label("when"),
            object(sound, Filter::Sounds),
            label("ends"),
        ])],
        BlockKind::WhenCutsceneFinished { sequence, .. } => vec![Row(vec![
            label("when cutscene"),
            object(sequence, Filter::Sequences),
            label("finishes"),
        ])],
        BlockKind::WhenConnectionFails { .. } => vec![Row(vec![label("when connection fails")])],
        BlockKind::WhenChatMessageReceived { .. } => {
            vec![Row(vec![label("when chat message received")])]
        }
        BlockKind::WhenStateChanged { animator, .. } => vec![Row(vec![
            label("when"),
            object(animator, Filter::Animators),
            label("changes state"),
        ])],
        BlockKind::WhenAnimationEvent {
            animator, event, ..
        } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("when"),
                object(animator, Filter::Animators),
                label("reaches event"),
                Part::Name(event, NameKind::Event, path),
            ])]
        }
        BlockKind::WhenAnimationFinished {
            animator,
            animation,
            ..
        } => vec![Row(vec![
            label("when animation"),
            object(animation, Filter::Animations),
            label("finishes on"),
            object(animator, Filter::Animators),
        ])],
        BlockKind::SetAnimatorBool {
            animator,
            parameter,
            value,
        } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("set"),
                Part::Name(parameter, NameKind::Bool, path),
                label("of"),
                object(animator, Filter::Animators),
                label("to"),
                Slot(value, SlotKind::Truth),
            ])]
        }
        BlockKind::SetAnimatorNumber {
            animator,
            parameter,
            value,
        } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("set"),
                Part::Name(parameter, NameKind::Number, path),
                label("of"),
                object(animator, Filter::Animators),
                label("to"),
                Slot(value, SlotKind::Number),
            ])]
        }
        BlockKind::SetAnimatorLayerWeight {
            animator,
            layer,
            weight,
            seconds,
        } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("set layer"),
                Part::Name(layer, NameKind::Layer, path),
                label("of"),
                object(animator, Filter::Animators),
                label("to"),
                Slot(weight, SlotKind::Number),
                label("over"),
                Slot(seconds, SlotKind::Number),
                label("seconds"),
            ])]
        }
        BlockKind::SetAnimatorTrigger {
            animator,
            parameter,
        } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("set trigger"),
                Part::Name(parameter, NameKind::Trigger, path),
                label("of"),
                object(animator, Filter::Animators),
            ])]
        }
        BlockKind::ResetAnimatorTrigger {
            animator,
            parameter,
        } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("reset trigger"),
                Part::Name(parameter, NameKind::Trigger, path),
                label("of"),
                object(animator, Filter::Animators),
            ])]
        }
        BlockKind::PlayAnimatorState { animator, state } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("play state"),
                Part::Name(state, NameKind::State, path),
                label("of"),
                object(animator, Filter::Animators),
            ])]
        }
        BlockKind::PlayAnimation {
            animator,
            animation,
            fade_time,
            speed,
            weight,
            wait_until_done,
        } => {
            let mut row = vec![
                label("play animation"),
                object(animation, Filter::Animations),
                label("on"),
                object(animator, Filter::Animators),
            ];
            if let Some(f) = fade_time {
                row.push(label("fade"));
                row.push(Slot(f, SlotKind::Number));
            }
            if let Some(s) = speed {
                row.push(label("speed"));
                row.push(Slot(s, SlotKind::Number));
            }
            if let Some(w) = weight {
                row.push(label("weight"));
                row.push(Slot(w, SlotKind::Number));
            }
            if *wait_until_done {
                row.push(label("and wait"));
            }
            vec![Row(row)]
        }
        BlockKind::StopAnimation {
            animator,
            animation,
            fade_time,
        } => {
            let mut row = vec![label("stop")];
            if let Some(anim) = animation {
                row.push(object(anim, Filter::Animations));
            } else {
                row.push(label("all animations"));
            }
            row.push(label("on"));
            row.push(object(animator, Filter::Animators));
            if let Some(f) = fade_time {
                row.push(label("fade"));
                row.push(Slot(f, SlotKind::Number));
            }
            vec![Row(row)]
        }
        BlockKind::SetAnimatorSpeed { animator, speed } => vec![Row(vec![
            label("set speed of"),
            object(animator, Filter::Animators),
            label("to"),
            Slot(speed, SlotKind::Number),
        ])],
        BlockKind::SetIkTarget {
            ik_control,
            target,
            weight,
        } => {
            let mut row = vec![
                label("set IK"),
                object(ik_control, Filter::IkControls),
                label("target to"),
                object(target, Filter::Any),
            ];
            if let Some(w) = weight {
                row.push(label("weight"));
                row.push(Slot(w, SlotKind::Number));
            }
            vec![Row(row)]
        }
        BlockKind::SetRagdollEnabled { ragdoll, enabled } => vec![Row(vec![
            label("set ragdoll"),
            object(ragdoll, Filter::Ragdolls),
            label("enabled"),
            Slot(enabled, SlotKind::Truth),
        ])],
        BlockKind::Tween {
            target,
            property,
            value,
            seconds,
            style,
            direction,
            repeat_count,
            reverses,
            wait_until_done,
            completed,
        } => {
            let class = class_of(objects, target);
            let mut row2 = vec![
                label("over"),
                Slot(seconds, SlotKind::Number),
                label("seconds"),
                Part::Choice(style, &axcenity_blocks::EASING_STYLES),
                Part::Choice(direction, &axcenity_blocks::EASING_DIRECTIONS),
            ];
            if let Some(rep) = repeat_count {
                row2.push(label("repeat"));
                row2.push(Slot(rep, SlotKind::Number));
            }
            if *reverses {
                row2.push(label("reverses"));
            }
            if *wait_until_done {
                row2.push(label("and wait"));
            }
            let mut out = vec![
                Row(vec![
                    label("tween"),
                    Property(property, class),
                    label("of"),
                    object(target, Filter::Any),
                    label("to"),
                    Slot(value, SlotKind::Any),
                ]),
                Row(row2),
            ];
            if !completed.is_empty() {
                out.push(Row(vec![label("when done")]));
                out.push(Mouth(completed));
            }
            out
        }
        BlockKind::SlideIn {
            target,
            from,
            seconds,
            wait_until_done,
            completed,
        } => {
            let mut row = vec![
                label("slide"),
                object(target, Filter::Gui),
                label("in from"),
                Part::Choice(from, &axcenity_blocks::SLIDE_EDGES),
                label("over"),
                Slot(seconds, SlotKind::Number),
                label("seconds"),
            ];
            if *wait_until_done {
                row.push(label("and wait"));
            }
            if completed.is_empty() {
                vec![Row(row)]
            } else {
                vec![Row(row), Row(vec![label("when done")]), Mouth(completed)]
            }
        }
        BlockKind::SlideOut {
            target,
            to,
            seconds,
            wait_until_done,
            completed,
        } => {
            let mut row = vec![
                label("slide"),
                object(target, Filter::Gui),
                label("out to"),
                Part::Choice(to, &axcenity_blocks::SLIDE_EDGES),
                label("over"),
                Slot(seconds, SlotKind::Number),
                label("seconds"),
            ];
            if *wait_until_done {
                row.push(label("and wait"));
            }
            if completed.is_empty() {
                vec![Row(row)]
            } else {
                vec![Row(row), Row(vec![label("when done")]), Mouth(completed)]
            }
        }
        BlockKind::FadeIn {
            target,
            seconds,
            wait_until_done,
            completed,
        } => {
            let mut row = vec![
                label("fade"),
                object(target, Filter::Any),
                label("in over"),
                Slot(seconds, SlotKind::Number),
                label("seconds"),
            ];
            if *wait_until_done {
                row.push(label("and wait"));
            }
            if completed.is_empty() {
                vec![Row(row)]
            } else {
                vec![Row(row), Row(vec![label("when done")]), Mouth(completed)]
            }
        }
        BlockKind::FadeOut {
            target,
            seconds,
            wait_until_done,
            completed,
        } => {
            let mut row = vec![
                label("fade"),
                object(target, Filter::Any),
                label("out over"),
                Slot(seconds, SlotKind::Number),
                label("seconds"),
            ];
            if *wait_until_done {
                row.push(label("and wait"));
            }
            if completed.is_empty() {
                vec![Row(row)]
            } else {
                vec![Row(row), Row(vec![label("when done")]), Mouth(completed)]
            }
        }
        BlockKind::TweenPivot {
            target,
            position,
            duration,
            easing,
            direction,
            wait_until_done,
            completed,
        } => {
            let mut row = vec![
                label("tween pivot of"),
                object(target, Filter::Any),
                label("to"),
                Slot(position, SlotKind::Vector),
                label("over"),
                Slot(duration, SlotKind::Number),
                label("s"),
                Part::Choice(easing, &axcenity_blocks::EASING_STYLES),
                Part::Choice(direction, &axcenity_blocks::EASING_DIRECTIONS),
            ];
            if *wait_until_done {
                row.push(label("and wait"));
            }
            if completed.is_empty() {
                vec![Row(row)]
            } else {
                vec![Row(row), Row(vec![label("when done")]), Mouth(completed)]
            }
        }
        BlockKind::PulseScale {
            target,
            scale,
            duration,
            wait_until_done,
        } => {
            let mut row = vec![
                label("pulse scale of"),
                object(target, Filter::Any),
                label("by"),
                Slot(scale, SlotKind::Number),
                label("over"),
                Slot(duration, SlotKind::Number),
                label("s"),
            ];
            if *wait_until_done {
                row.push(label("and wait"));
            }
            vec![Row(row)]
        }
        BlockKind::CancelTween { tween } => {
            vec![Row(vec![label("cancel tween"), object(tween, Filter::Any)])]
        }
        BlockKind::Unattached { .. } => Vec::new(),
        BlockKind::Wait { seconds } => vec![Row(vec![
            label("wait"),
            Slot(seconds, SlotKind::Number),
            label("seconds"),
        ])],
        BlockKind::WaitUntil { condition } => vec![Row(vec![
            label("wait until"),
            Slot(condition, SlotKind::Condition),
        ])],
        BlockKind::Repeat { count, body } => vec![
            Row(vec![
                label("repeat"),
                Slot(count, SlotKind::Number),
                label("times"),
            ]),
            Mouth(body),
        ],
        BlockKind::Forever { body } => vec![Row(vec![label("forever")]), Mouth(body)],
        BlockKind::ForEachChild {
            target,
            variable,
            body,
        } => vec![
            Row(vec![
                label("for each"),
                Variable(variable),
                label("in"),
                object(target, Filter::Places),
            ]),
            Mouth(body),
        ],
        BlockKind::ForEachPartInRadius {
            center,
            radius,
            variable,
            ignore,
            body,
            ..
        } => {
            let mut row = vec![
                label("for each part"),
                Variable(variable),
                label("within"),
                Slot(radius, SlotKind::Number),
                label("studs of"),
                Slot(center, SlotKind::Vector),
            ];
            if let Some(ign) = ignore {
                row.push(label("ignoring"));
                row.push(Slot(ign, SlotKind::Any));
            }
            vec![Row(row), Mouth(body)]
        }
        BlockKind::ForEachPartTouching {
            target,
            variable,
            ignore,
            body,
            ..
        } => {
            let mut row = vec![
                label("for each part"),
                Variable(variable),
                label("touching"),
                object(target, Filter::Parts),
            ];
            if let Some(ign) = ignore {
                row.push(label("ignoring"));
                row.push(Slot(ign, SlotKind::Any));
            }
            vec![Row(row), Mouth(body)]
        }
        BlockKind::DestroyAfter { target, seconds } => vec![Row(vec![
            label("destroy"),
            object(target, Filter::Any),
            label("after"),
            Slot(seconds, SlotKind::Number),
            label("seconds"),
        ])],
        BlockKind::WaitForEvent { target, event } => {
            let path = target.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("wait until"),
                object(target, Filter::Any),
                Part::Name(event, NameKind::Signal, path),
                label("fires"),
            ])]
        }
        BlockKind::WaitForChild {
            target,
            name,
            timeout,
        } => {
            let path = target.as_path().unwrap_or("").to_owned();
            let mut row = vec![
                label("wait for child"),
                Part::Name(name, NameKind::Child, path),
                label("in"),
                object(target, Filter::Places),
            ];
            if let Some(secs) = timeout {
                row.push(label("timeout"));
                row.push(Slot(secs, SlotKind::Number));
                row.push(label("s"));
            }
            vec![Row(row)]
        }
        BlockKind::ForRange {
            start,
            end,
            step,
            body,
        } => vec![
            Row(vec![
                label("for i from"),
                Slot(start, SlotKind::Number),
                label("to"),
                Slot(end, SlotKind::Number),
                label("step"),
                Slot(step, SlotKind::Number),
            ]),
            Mouth(body),
        ],
        BlockKind::ForEachInList { list, body } => vec![
            Row(vec![label("for each item in"), Slot(list, SlotKind::Any)]),
            Mouth(body),
        ],
        BlockKind::ForEachTagged { tag, body } => vec![
            Row(vec![label("for each tagged"), Slot(tag, SlotKind::Text)]),
            Mouth(body),
        ],
        BlockKind::Sequence { branches } => {
            let mut out = vec![Row(vec![label("sequence")])];
            for (index, branch) in branches.iter_mut().enumerate() {
                if index > 0 {
                    out.push(Row(vec![Part::Label(format!("then {index}"))]));
                }
                out.push(Mouth(branch));
            }
            out
        }
        BlockKind::CooldownGate { cooldown, body } => vec![
            Row(vec![
                label("do once / cooldown"),
                Slot(cooldown, SlotKind::Number),
                label("s"),
            ]),
            Mouth(body),
        ],
        BlockKind::SpawnAsync { delay, body } => vec![
            Row(vec![
                label("spawn async after"),
                Slot(delay, SlotKind::Number),
                label("s"),
            ]),
            Mouth(body),
        ],
        BlockKind::RepeatUntil { condition, body } => vec![
            Row(vec![
                label("repeat until"),
                Slot(condition, SlotKind::Condition),
            ]),
            Mouth(body),
        ],
        BlockKind::If { condition, body } => vec![
            Row(vec![
                label("if"),
                Slot(condition, SlotKind::Condition),
                label("then"),
            ]),
            Mouth(body),
        ],
        BlockKind::IfElse {
            condition,
            then_body,
            else_body,
        } => vec![
            Row(vec![
                label("if"),
                Slot(condition, SlotKind::Condition),
                label("then"),
            ]),
            Mouth(then_body),
            Row(vec![label("else")]),
            Mouth(else_body),
        ],
        BlockKind::StopScript => vec![Row(vec![label("stop this script")])],
        BlockKind::DestroyScript => vec![Row(vec![label("destroy / stop script")])],
        BlockKind::MoveTo { target, position } => vec![Row(vec![
            label("move"),
            object(target, Filter::Parts),
            label("to"),
            Slot(position, SlotKind::Vector),
        ])],
        BlockKind::MoveBy { target, offset } => vec![Row(vec![
            label("move"),
            object(target, Filter::Parts),
            label("by"),
            Slot(offset, SlotKind::Vector),
        ])],
        BlockKind::LookAt { target, position } => vec![Row(vec![
            label("turn"),
            object(target, Filter::Parts),
            label("to look at"),
            Slot(position, SlotKind::Vector),
        ])],
        BlockKind::MoveForward { target, distance } => vec![Row(vec![
            label("move"),
            object(target, Filter::Parts),
            label("forward by"),
            Slot(distance, SlotKind::Number),
        ])],
        BlockKind::TurnTo {
            target,
            orientation,
        } => vec![Row(vec![
            label("turn"),
            object(target, Filter::Parts),
            label("to"),
            Slot(orientation, SlotKind::Vector),
            label("degrees"),
        ])],
        BlockKind::TurnBy {
            target,
            rotation,
            space,
        } => vec![Row(vec![
            label("turn"),
            object(target, Filter::Parts),
            label("by"),
            Slot(rotation, SlotKind::Vector),
            label("degrees in"),
            Part::Choice(space, &axcenity_blocks::SPACE_MODES),
            label("space"),
        ])],
        BlockKind::SlideTo {
            target,
            position,
            duration,
            easing,
            wait_until_done,
            completed,
        } => {
            let mut row = vec![
                label("slide"),
                object(target, Filter::Parts),
                label("to"),
                Slot(position, SlotKind::Vector),
                label("over"),
                Slot(duration, SlotKind::Number),
                label("s"),
                Part::Choice(easing, &axcenity_blocks::EASING_STYLES),
            ];
            if *wait_until_done {
                row.push(label("(wait)"));
            }
            vec![Row(row), Mouth(completed)]
        }
        BlockKind::RotateTo {
            target,
            rotation,
            duration,
            easing,
            wait_until_done,
            completed,
        } => {
            let mut row = vec![
                label("rotate"),
                object(target, Filter::Parts),
                label("to"),
                Slot(rotation, SlotKind::Vector),
                label("over"),
                Slot(duration, SlotKind::Number),
                label("s"),
                Part::Choice(easing, &axcenity_blocks::EASING_STYLES),
            ];
            if *wait_until_done {
                row.push(label("(wait)"));
            }
            vec![Row(row), Mouth(completed)]
        }
        BlockKind::SetGravity { target, multiplier } => vec![Row(vec![
            label("set gravity of"),
            object(target, Filter::Parts),
            label("to"),
            Slot(multiplier, SlotKind::Number),
            label("×"),
        ])],
        BlockKind::SetAngularVelocity { target, velocity } => vec![Row(vec![
            label("set angular velocity of"),
            object(target, Filter::Parts),
            label("to"),
            Slot(velocity, SlotKind::Vector),
        ])],
        BlockKind::LaunchPart {
            target,
            direction,
            speed,
        } => vec![Row(vec![
            label("launch"),
            object(target, Filter::Parts),
            label("along"),
            Slot(direction, SlotKind::Vector),
            label("at speed"),
            Slot(speed, SlotKind::Number),
        ])],
        BlockKind::WhenAgentSenses { agent, sense, .. } => vec![Row(vec![
            label("when"),
            object(agent, Filter::Agents),
            label("senses the player by"),
            Part::Choice(sense, &axcenity_blocks::SENSES),
        ])],
        BlockKind::WhenAgentLosesPlayer { agent, sense, .. } => vec![Row(vec![
            label("when"),
            object(agent, Filter::Agents),
            label("loses the player by"),
            Part::Choice(sense, &axcenity_blocks::SENSES),
        ])],
        BlockKind::WhenAgentArrives { agent, .. } => vec![Row(vec![
            label("when"),
            object(agent, Filter::Agents),
            label("arrives"),
        ])],
        BlockKind::WhenSplineReached {
            follower, which, ..
        } => vec![Row(vec![
            label("when"),
            object(follower, Filter::SplineFollowers),
            label("reaches the"),
            Part::Choice(which, &axcenity_blocks::SPLINE_ENDS),
        ])],
        BlockKind::ControlSpline { follower, action } => vec![Row(vec![
            Part::Choice(action, &axcenity_blocks::SPLINE_ACTIONS),
            label("spline"),
            object(follower, Filter::SplineFollowers),
        ])],
        BlockKind::SetSplineSpeed { follower, speed } => vec![Row(vec![
            label("set spline speed of"),
            object(follower, Filter::SplineFollowers),
            label("to"),
            Slot(speed, SlotKind::Number),
        ])],
        BlockKind::Patrol { agent, spline } => vec![Row(vec![
            label("patrol"),
            object(agent, Filter::Agents),
            label("along"),
            object(spline, Filter::Splines),
        ])],
        BlockKind::PlaceOnSpline {
            target,
            spline,
            distance,
        } => vec![Row(vec![
            label("put"),
            object(target, Filter::Any),
            label("on"),
            object(spline, Filter::Splines),
            label("at"),
            Slot(distance, SlotKind::Number),
            label("studs"),
        ])],
        BlockKind::WhenPromptTriggered {
            prompt, cooldown, ..
        } => {
            let mut row = vec![
                label("when prompt"),
                object(prompt, Filter::Prompts),
                label("is triggered"),
            ];
            if let Some(cd) = cooldown {
                row.push(label("cooldown"));
                row.push(Slot(cd, SlotKind::Number));
                row.push(label("s"));
            }
            vec![Row(row)]
        }
        BlockKind::WhenPartClicked {
            detector, cooldown, ..
        } => {
            let mut row = vec![
                label("when"),
                object(detector, Filter::ClickDetectors),
                label("is clicked"),
            ];
            if let Some(cd) = cooldown {
                row.push(label("cooldown"));
                row.push(Slot(cd, SlotKind::Number));
                row.push(label("s"));
            }
            vec![Row(row)]
        }
        BlockKind::WhenCommandRun {
            command,
            help,
            admin_only,
            ..
        } => {
            let mut row = vec![
                label("when console command"),
                Slot(command, SlotKind::Text),
                label("is run, help"),
                Slot(help, SlotKind::Text),
            ];
            if let Some(admin) = admin_only {
                row.push(label("admin only"));
                row.push(Slot(admin, SlotKind::Truth));
            }
            vec![Row(row)]
        }
        BlockKind::WhenPropertyChanged {
            target, property, ..
        } => {
            let class = class_of(objects, target);
            vec![Row(vec![
                label("when"),
                Property(property, class),
                label("of"),
                object(target, Filter::Any),
                label("changes"),
            ])]
        }
        BlockKind::WhenAttributeChanged {
            target, attribute, ..
        } => {
            let path = target.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("when attribute"),
                Part::Name(attribute, NameKind::Attribute, path),
                label("of"),
                object(target, Filter::Any),
                label("changes"),
            ])]
        }
        BlockKind::WhenSeatOccupied { seat, .. } => vec![Row(vec![
            label("when someone sits on"),
            object(seat, Filter::Seats),
        ])],
        BlockKind::WhenSeatVacated { seat, .. } => vec![Row(vec![
            label("when someone leaves"),
            object(seat, Filter::Seats),
        ])],
        BlockKind::SitInSeat { seat, controller } => vec![Row(vec![
            label("sit in"),
            object(seat, Filter::Seats),
            label("controller"),
            object(controller, Filter::Controllers),
        ])],
        BlockKind::LeaveSeat { controller } => vec![Row(vec![
            label("make"),
            object(controller, Filter::Controllers),
            label("leave the seat"),
        ])],
        BlockKind::SetDestination { agent, position } => vec![Row(vec![
            label("walk"),
            object(agent, Filter::Agents),
            label("to"),
            Slot(position, SlotKind::Vector),
        ])],
        BlockKind::FollowTarget { agent, target } => vec![Row(vec![
            label("have"),
            object(agent, Filter::Agents),
            label("follow"),
            object(target, Filter::Any),
        ])],
        BlockKind::StopAgent { agent } => vec![Row(vec![
            label("stop agent"),
            object(agent, Filter::Agents),
        ])],
        BlockKind::SetVelocity { agent, velocity } => vec![Row(vec![
            label("set agent velocity of"),
            object(agent, Filter::Agents),
            label("to"),
            Slot(velocity, SlotKind::Vector),
        ])],
        BlockKind::SetAnchored { target, anchored } => vec![Row(vec![
            label("set"),
            object(target, Filter::Parts),
            label("anchored to"),
            Slot(anchored, SlotKind::Truth),
        ])],
        BlockKind::PushWithForce { target, force } => vec![Row(vec![
            label("push"),
            object(target, Filter::Parts),
            label("with force"),
            Slot(force, SlotKind::Vector),
        ])],
        BlockKind::SetPartVelocity { target, velocity } => vec![Row(vec![
            label("set velocity of"),
            object(target, Filter::Parts),
            label("to"),
            Slot(velocity, SlotKind::Vector),
        ])],
        BlockKind::SetFriction { target, friction } => vec![Row(vec![
            label("set friction of"),
            object(target, Filter::Parts),
            label("to"),
            Slot(friction, SlotKind::Number),
        ])],
        BlockKind::SetColor { target, color } => vec![Row(vec![
            label("set color of"),
            object(target, Filter::Parts),
            label("to"),
            Slot(color, SlotKind::Color),
        ])],
        BlockKind::SetText { target, text } => vec![Row(vec![
            label("set text of"),
            object(target, Filter::Texts),
            label("to"),
            Slot(text, SlotKind::Text),
        ])],
        BlockKind::Show { target } => vec![Row(vec![label("show"), object(target, Filter::Gui)])],
        BlockKind::Hide { target } => vec![Row(vec![label("hide"), object(target, Filter::Gui)])],
        BlockKind::ShowPage { layout, page } => vec![Row(vec![
            label("show page"),
            Slot(page, SlotKind::Text),
            label("of"),
            object(layout, Filter::PageLayouts),
        ])],
        BlockKind::ScrollTo { target, x, y } => vec![Row(vec![
            label("scroll"),
            object(target, Filter::ScrollingFrames),
            label("to x"),
            Slot(x, SlotKind::Number),
            label("y"),
            Slot(y, SlotKind::Number),
        ])],
        BlockKind::SetValue { target, value } => {
            // A Slider's and a ProgressBar's value is a number, a Toggle's
            // true or false, and a Dropdown's an option's text.
            let kind = match class_of(objects, target) {
                Some(ClassName::Slider | ClassName::ProgressBar) => SlotKind::Number,
                Some(ClassName::Toggle) => SlotKind::Truth,
                Some(ClassName::Dropdown) => SlotKind::Text,
                _ => SlotKind::Any,
            };
            vec![Row(vec![
                label("set value of"),
                object(target, Filter::Widgets),
                label("to"),
                Slot(value, kind),
            ])]
        }
        BlockKind::SetProperty {
            target,
            property,
            value,
        } => {
            let class = class_of(objects, target);
            vec![Row(vec![
                label("set"),
                Property(property, class),
                label("of"),
                object(target, Filter::Any),
                label("to"),
                Slot(value, SlotKind::Any),
            ])]
        }
        BlockKind::SetAttribute {
            target,
            attribute,
            value,
        } => {
            let path = target.as_path().unwrap_or("").to_owned();
            vec![Row(vec![
                label("set attribute"),
                Part::Name(attribute, NameKind::Attribute, path),
                label("of"),
                object(target, Filter::Any),
                label("to"),
                Slot(value, SlotKind::Any),
            ])]
        }
        BlockKind::PlaySound {
            target,
            pitch,
            wait_until_done,
        } => {
            let mut row = vec![label("play sound"), object(target, Filter::Sounds)];
            if let Some(p) = pitch {
                row.push(label("pitch"));
                row.push(Slot(p, SlotKind::Number));
            }
            if *wait_until_done {
                row.push(label("and wait"));
            }
            vec![Row(row)]
        }
        BlockKind::PlaySoundAtPosition {
            target,
            position,
            volume,
            pitch,
        } => {
            let mut row = vec![
                label("play sound"),
                object(target, Filter::Sounds),
                label("at"),
                Slot(position, SlotKind::Vector),
            ];
            if let Some(v) = volume {
                row.push(label("vol"));
                row.push(Slot(v, SlotKind::Number));
            }
            if let Some(p) = pitch {
                row.push(label("pitch"));
                row.push(Slot(p, SlotKind::Number));
            }
            vec![Row(row)]
        }
        BlockKind::PlayCutscene { sequence } => {
            vec![Row(vec![
                label("play cutscene"),
                object(sequence, Filter::Sequences),
            ])]
        }
        BlockKind::PauseCutscene { sequence } => {
            vec![Row(vec![
                label("pause cutscene"),
                object(sequence, Filter::Sequences),
            ])]
        }
        BlockKind::ResumeCutscene { sequence } => {
            vec![Row(vec![
                label("resume cutscene"),
                object(sequence, Filter::Sequences),
            ])]
        }
        BlockKind::StopCutscene { sequence } => {
            vec![Row(vec![
                label("stop cutscene"),
                object(sequence, Filter::Sequences),
            ])]
        }
        BlockKind::StopSound { target } => {
            vec![Row(vec![
                label("stop sound"),
                object(target, Filter::Sounds),
            ])]
        }
        BlockKind::PauseSound { target } => {
            vec![Row(vec![
                label("pause sound"),
                object(target, Filter::Sounds),
            ])]
        }
        BlockKind::ResumeSound { target } => {
            vec![Row(vec![
                label("resume sound"),
                object(target, Filter::Sounds),
            ])]
        }
        BlockKind::SetVolume { target, volume } => vec![Row(vec![
            label("set volume of"),
            object(target, Filter::Sounds),
            label("to"),
            Slot(volume, SlotKind::Number),
        ])],
        BlockKind::FadeVolume {
            target,
            volume,
            duration,
            wait_until_done,
        } => {
            let mut row = vec![
                label("fade volume of"),
                object(target, Filter::Sounds),
                label("to"),
                Slot(volume, SlotKind::Number),
                label("over"),
                Slot(duration, SlotKind::Number),
                label("s"),
            ];
            if *wait_until_done {
                row.push(label("and wait"));
            }
            vec![Row(row)]
        }
        BlockKind::SetSoundPitch { target, pitch } => vec![Row(vec![
            label("set pitch of"),
            object(target, Filter::Sounds),
            label("to"),
            Slot(pitch, SlotKind::Number),
        ])],
        BlockKind::SetSoundLooping { target, looped } => vec![Row(vec![
            label("set looping of"),
            object(target, Filter::Sounds),
            label("to"),
            Slot(looped, SlotKind::Truth),
        ])],
        BlockKind::SetSoundTime { target, time } => vec![Row(vec![
            label("seek"),
            object(target, Filter::Sounds),
            label("to"),
            Slot(time, SlotKind::Number),
            label("s"),
        ])],
        BlockKind::SetVariable { name, value } => vec![Row(vec![
            label("set"),
            Variable(name),
            label("to"),
            Slot(value, SlotKind::Any),
        ])],
        BlockKind::ChangeVariable { name, by } => vec![Row(vec![
            label("change"),
            Variable(name),
            label("by"),
            Slot(by, SlotKind::Number),
        ])],
        BlockKind::ListCreate { name, items } => {
            let mut row = vec![label("create list"), Variable(name)];
            if !items.is_empty() {
                row.push(label("with"));
                for (index, item) in items.iter_mut().enumerate() {
                    if index > 0 {
                        row.push(label(","));
                    }
                    row.push(Slot(item, SlotKind::Any));
                }
            }
            vec![Row(row)]
        }
        BlockKind::ListAdd { list, value } => vec![Row(vec![
            label("add"),
            Slot(value, SlotKind::Any),
            label("to list"),
            Slot(list, SlotKind::Any),
        ])],
        BlockKind::ListInsert { list, index, value } => vec![Row(vec![
            label("insert"),
            Slot(value, SlotKind::Any),
            label("at"),
            Slot(index, SlotKind::Number),
            label("of list"),
            Slot(list, SlotKind::Any),
        ])],
        BlockKind::ListRemoveIndex { list, index } => vec![Row(vec![
            label("remove item"),
            Slot(index, SlotKind::Number),
            label("of list"),
            Slot(list, SlotKind::Any),
        ])],
        BlockKind::ListRemoveValue { list, value } => vec![Row(vec![
            label("remove"),
            Slot(value, SlotKind::Any),
            label("from list"),
            Slot(list, SlotKind::Any),
        ])],
        BlockKind::ListClear { list } => {
            vec![Row(vec![label("clear list"), Slot(list, SlotKind::Any)])]
        }
        BlockKind::Print { value } => vec![Row(vec![label("print"), Slot(value, SlotKind::Any)])],
        BlockKind::Destroy { target } => {
            vec![Row(vec![label("destroy"), object(target, Filter::Any)])]
        }
        BlockKind::CloneInto {
            target,
            parent,
            count,
            position,
            offset_per_clone,
            per_clone_positions,
        } => {
            let mut rows = vec![Row(vec![
                label("clone"),
                object(target, Filter::Any),
                label("into"),
                object(parent, Filter::Places),
            ])];
            let mut row2 = Vec::new();
            if let Some(count) = count {
                row2.push(label("count"));
                row2.push(Slot(count, SlotKind::Number));
            }
            if let Some(position) = position {
                row2.push(label("at"));
                row2.push(Slot(position, SlotKind::Vector));
            }
            if !row2.is_empty() {
                rows.push(Row(row2));
            }
            let mut row3 = Vec::new();
            if let Some(offset) = offset_per_clone {
                row3.push(label("offset"));
                row3.push(Slot(offset, SlotKind::Vector));
            }
            if let Some(positions) = per_clone_positions {
                row3.push(label("positions"));
                row3.push(Slot(positions, SlotKind::Any));
            }
            if !row3.is_empty() {
                rows.push(Row(row3));
            }
            rows
        }
        BlockKind::SaveSlot { slot } => vec![Row(vec![
            label("save game to slot"),
            Slot(slot, SlotKind::Number),
        ])],
        BlockKind::LoadSlot { slot } => vec![Row(vec![
            label("load game from slot"),
            Slot(slot, SlotKind::Number),
        ])],
        BlockKind::DeleteSlot { slot } => vec![Row(vec![
            label("delete save slot"),
            Slot(slot, SlotKind::Number),
        ])],
        BlockKind::ForEachRow { table, body } => vec![
            Row(vec![
                label("for each row in"),
                Part::Name(table, NameKind::DataTable, STORAGE_PATH.to_owned()),
            ]),
            Mouth(body),
        ],
        BlockKind::SetCheckpoint { checkpoint } => vec![Row(vec![
            label("set checkpoint to"),
            object(checkpoint, Filter::Checkpoints),
        ])],
        BlockKind::RestoreCheckpoint => vec![Row(vec![label("go back to checkpoint")])],
        BlockKind::LoadLevel { level, spawn } => vec![Row(vec![
            label("load level"),
            Slot(level, SlotKind::Text),
            label("at"),
            Slot(spawn, SlotKind::Text),
        ])],
        BlockKind::AddLevel { level } => {
            vec![Row(vec![label("add level"), Slot(level, SlotKind::Text)])]
        }
        BlockKind::RemoveLevel { level } => vec![Row(vec![
            label("remove level"),
            Slot(level, SlotKind::Text),
        ])],
        BlockKind::HostGame { port } => vec![Row(vec![
            label("host game on port"),
            Slot(port, SlotKind::Number),
        ])],
        BlockKind::JoinGame { address } => vec![Row(vec![
            label("join game at"),
            Slot(address, SlotKind::Text),
        ])],
        BlockKind::LeaveGame => vec![Row(vec![label("leave game")])],
        BlockKind::SetNetworkOwner { target, owner } => vec![Row(vec![
            label("let"),
            Slot(owner, SlotKind::Any),
            label("simulate"),
            object(target, Filter::Parts),
        ])],
        BlockKind::SetNetworkOwnershipAuto { target } => vec![Row(vec![
            label("let the nearest player simulate"),
            object(target, Filter::Parts),
        ])],
        BlockKind::SendChatMessage { text } => vec![Row(vec![
            label("send chat message"),
            Slot(text, SlotKind::Text),
        ])],
        BlockKind::ShowSystemMessage { text } => vec![Row(vec![
            label("show system message"),
            Slot(text, SlotKind::Text),
        ])],
        BlockKind::WhenLayerChanged { data_layer, .. } => vec![Row(vec![
            label("when layer"),
            Part::Text(data_layer),
            label("changes"),
        ])],
        BlockKind::SetLayerActive { data_layer, active } => vec![Row(vec![
            label("set layer"),
            Part::Text(data_layer),
            label("active"),
            Slot(active, SlotKind::Truth),
        ])],
        BlockKind::RequestStreamAround { player, position } => vec![Row(vec![
            label("load for"),
            Slot(player, SlotKind::Any),
            label("around"),
            Slot(position, SlotKind::Vector),
        ])],
        BlockKind::WhenStreamedIn { target, .. } => vec![Row(vec![
            label("when"),
            object(target, Filter::Parts),
            label("streams in"),
        ])],
        BlockKind::WhenStreamedOut { target, .. } => vec![Row(vec![
            label("when"),
            object(target, Filter::Parts),
            label("streams out"),
        ])],
        BlockKind::KickPlayer { player, reason } => vec![Row(vec![
            label("kick"),
            Slot(player, SlotKind::Any),
            label("because"),
            Slot(reason, SlotKind::Text),
        ])],
        BlockKind::SetLanguage { language } => vec![Row(vec![
            label("set language to"),
            Slot(language, SlotKind::Text),
        ])],
        BlockKind::SetCameraType { camera_type } => vec![Row(vec![
            label("set camera to"),
            Part::Choice(camera_type, &axcenity_blocks::CAMERA_TYPES),
        ])],
        BlockKind::PointCameraAt { position } => vec![Row(vec![
            label("point camera at"),
            Slot(position, SlotKind::Vector),
        ])],
        BlockKind::ShakeCamera { intensity, seconds } => vec![Row(vec![
            label("shake camera by"),
            Slot(intensity, SlotKind::Number),
            label("for"),
            Slot(seconds, SlotKind::Number),
            label("seconds"),
        ])],
        BlockKind::SpawnDecal {
            texture,
            position,
            facing,
            size,
            seconds,
        } => vec![
            Row(vec![
                label("spawn decal"),
                Slot(texture, SlotKind::Text),
                label("at"),
                Slot(position, SlotKind::Vector),
                label("facing"),
                Slot(facing, SlotKind::Vector),
            ]),
            Row(vec![
                label("size"),
                Slot(size, SlotKind::Number),
                label("fading after"),
                Slot(seconds, SlotKind::Number),
                label("seconds"),
            ]),
        ],
        BlockKind::SetToneMapping { curve } => vec![Row(vec![
            label("set tone mapping to"),
            Part::Choice(curve, &axcenity_blocks::TONE_MAPPINGS),
        ])],
        BlockKind::SetClockTime { time, duration } => {
            let mut row = vec![
                label("set time of day to"),
                Part::Slot(time, SlotKind::Number),
                label("hours"),
            ];
            if let Some(dur) = duration {
                row.push(label("over"));
                row.push(Part::Slot(dur, SlotKind::Number));
                row.push(label("s"));
            }
            vec![Row(row)]
        }
        BlockKind::SetAtmosphere {
            property,
            value,
            duration,
        } => {
            let mut row = vec![
                label("set atmosphere"),
                Part::Choice(property, &axcenity_blocks::ATMOSPHERE_NUMBERS),
                label("to"),
                Part::Slot(value, SlotKind::Number),
            ];
            if let Some(dur) = duration {
                row.push(label("over"));
                row.push(Part::Slot(dur, SlotKind::Number));
                row.push(label("s"));
            }
            vec![Row(row)]
        }
        BlockKind::ToggleVisibility { target } => vec![Row(vec![
            label("toggle visibility of"),
            object(target, Filter::Any),
        ])],
        BlockKind::SetTransparency {
            target,
            transparency,
        } => vec![Row(vec![
            label("set transparency of"),
            object(target, Filter::Any),
            label("to"),
            Part::Slot(transparency, SlotKind::Number),
        ])],
        BlockKind::SetMaterial { target, material } => vec![Row(vec![
            label("set material of"),
            object(target, Filter::Parts),
            label("to"),
            Part::Choice(material, &axcenity_blocks::MATERIALS),
        ])],
        BlockKind::EmitParticles { target, count } => vec![Row(vec![
            label("emit particles from"),
            object(target, Filter::Any),
            label("count"),
            Part::Slot(count, SlotKind::Number),
        ])],
        BlockKind::SetEffectEnabled { target, enabled } => vec![Row(vec![
            label("set effect"),
            object(target, Filter::Any),
            label("enabled"),
            Part::Slot(enabled, SlotKind::Truth),
        ])],
        BlockKind::SetLightingProperty { property, value } => vec![Row(vec![
            label("set lighting"),
            Part::Choice(property, &axcenity_blocks::LIGHTING_PROPERTIES),
            label("to"),
            Part::Slot(value, SlotKind::Any),
        ])],
        BlockKind::SetPostProcessing {
            effect,
            property,
            value,
        } => {
            let props = axcenity_blocks::post_processing_properties(effect);
            if !props.contains(&property.as_str())
                && let Some(first) = props.first()
            {
                *property = (*first).to_owned();
            }
            vec![Row(vec![
                label("set"),
                Part::Choice(effect, &axcenity_blocks::POST_PROCESSING_EFFECTS),
                Part::Choice(property, props),
                label("to"),
                Part::Slot(value, SlotKind::Any),
            ])]
        }
        BlockKind::TweenProperty {
            target,
            property,
            value,
            duration,
            easing,
            wait_until_done,
            completed,
        } => {
            let mut row = vec![
                label("tween"),
                Part::Property(property, class_of(objects, target)),
                label("of"),
                object(target, Filter::Any),
                label("to"),
                Part::Slot(value, SlotKind::Any),
                label("over"),
                Part::Slot(duration, SlotKind::Number),
                label("s"),
                Part::Choice(easing, &axcenity_blocks::EASING_STYLES),
            ];
            if *wait_until_done {
                row.push(label("and wait"));
            }
            if completed.is_empty() {
                vec![Row(row)]
            } else {
                vec![
                    Row(row),
                    Row(vec![label("when done")]),
                    Section::Mouth(completed),
                ]
            }
        }
        BlockKind::SetImage { target, image } => vec![Row(vec![
            label("set image of"),
            object(target, Filter::Any),
            label("to"),
            Part::Slot(image, SlotKind::Text),
        ])],
        BlockKind::SetGraphicsQuality { quality } => vec![Row(vec![
            label("set graphics quality to"),
            Part::Choice(quality, &axcenity_blocks::GRAPHICS_QUALITIES),
        ])],
        BlockKind::SetGraphicsOption {
            setting,
            choice,
            value,
        } => {
            if axcenity_blocks::GRAPHICS_NUMBERS.contains(&setting.as_str()) {
                let v = value.get_or_insert_with(|| Expression::Number { value: 1.0 });
                vec![Row(vec![
                    label("set graphics"),
                    Part::Choice(setting, &axcenity_blocks::ALL_GRAPHICS_SETTINGS),
                    label("to"),
                    Part::Slot(v, SlotKind::Number),
                ])]
            } else {
                *value = None;
                let choices = axcenity_blocks::graphics_choices(setting);
                if !choices.contains(&choice.as_str())
                    && let Some(first) = choices.first()
                {
                    *choice = (*first).to_owned();
                }
                vec![Row(vec![
                    label("set graphics"),
                    Part::Choice(setting, &axcenity_blocks::ALL_GRAPHICS_SETTINGS),
                    label("to"),
                    Part::Choice(choice, choices),
                ])]
            }
        }
        BlockKind::SetGraphicsNumber { setting, value } => vec![Row(vec![
            label("set graphics"),
            Part::Choice(setting, &axcenity_blocks::GRAPHICS_NUMBERS),
            label("to"),
            Part::Slot(value, SlotKind::Number),
        ])],
        BlockKind::Code { code } | BlockKind::TopCode { code } => {
            vec![Row(vec![label("Lua"), Part::Code(shown_code(code))])]
        }
        BlockKind::RunScript { .. }
        | BlockKind::OnSignal { .. }
        | BlockKind::Connect { .. }
        | BlockKind::Comment { .. }
        | BlockKind::Local { .. }
        | BlockKind::Assign { .. }
        | BlockKind::CompoundAssign { .. }
        | BlockKind::CallFunction { .. }
        | BlockKind::CallMethod { .. }
        | BlockKind::DefineFunction { .. }
        | BlockKind::While { .. }
        | BlockKind::RepeatLoop { .. }
        | BlockKind::NumericFor { .. }
        | BlockKind::GenericFor { .. }
        | BlockKind::DoBlock { .. }
        | BlockKind::Break
        | BlockKind::Continue
        | BlockKind::Return { .. } => luau_sections(kind),
    }
}

/// Arguments in a row, with commas between them.
fn push_slots<'a>(values: &'a mut [Expression], parts: &mut Vec<Part<'a>>) {
    for (index, value) in values.iter_mut().enumerate() {
        if index > 0 {
            parts.push(label(","));
        }
        parts.push(Part::Slot(value, SlotKind::Any));
    }
}

/// What a reporter shows. Its slots are in the order of
/// [`Expression::children_mut`].
fn reporter_parts<'a>(expression: &'a mut Expression, objects: &[ObjectInfo]) -> Vec<Part<'a>> {
    use Part::{Key, Property, Slot};
    match expression {
        Expression::Variable { name } => vec![Part::Label(name.clone())],
        Expression::BlockOutput { name, .. } => vec![Part::Label(name.clone())],
        Expression::ListItem { list, index } => vec![
            label("item"),
            Slot(index, SlotKind::Number),
            label("of"),
            Slot(list, SlotKind::Any),
        ],
        Expression::ListLength { list } => vec![label("length of"), Slot(list, SlotKind::Any)],
        Expression::ListContains { list, value } => vec![
            Slot(list, SlotKind::Any),
            label("contains"),
            Slot(value, SlotKind::Any),
        ],
        Expression::Math { op, left, right } => vec![
            Slot(left, SlotKind::Any),
            Part::Math(op),
            Slot(right, SlotKind::Any),
        ],
        Expression::Function { function, value } => vec![
            Part::Function(function),
            label("of"),
            Slot(value, SlotKind::Number),
        ],
        Expression::Random { min, max } => vec![
            label("random integer"),
            Slot(min, SlotKind::Number),
            label("to"),
            Slot(max, SlotKind::Number),
        ],
        Expression::RandomNumber { min, max } => vec![
            label("random decimal"),
            Slot(min, SlotKind::Number),
            label("to"),
            Slot(max, SlotKind::Number),
        ],
        Expression::Lerp { from, to, alpha } => vec![
            label("lerp"),
            Slot(from, SlotKind::Any),
            label("to"),
            Slot(to, SlotKind::Any),
            label("by"),
            Slot(alpha, SlotKind::Number),
        ],
        Expression::MakeVector3 { x, y, z } => vec![
            label("vector ("),
            Slot(x, SlotKind::Number),
            label(","),
            Slot(y, SlotKind::Number),
            label(","),
            Slot(z, SlotKind::Number),
            label(")"),
        ],
        Expression::VectorAxis { axis, vector } => vec![
            Part::Choice(axis, &axcenity_blocks::VECTOR_AXES),
            label("of"),
            Slot(vector, SlotKind::Vector),
        ],
        Expression::Distance { from, to } => vec![
            label("distance"),
            Slot(from, SlotKind::Vector),
            label("to"),
            Slot(to, SlotKind::Vector),
        ],
        Expression::Direction { from, to } => vec![
            label("direction"),
            Slot(from, SlotKind::Vector),
            label("to"),
            Slot(to, SlotKind::Vector),
        ],
        Expression::Magnitude { vector } => {
            vec![label("magnitude of"), Slot(vector, SlotKind::Vector)]
        }
        Expression::Clamp { value, min, max } => vec![
            label("clamp"),
            Slot(value, SlotKind::Number),
            label("between"),
            Slot(min, SlotKind::Number),
            label("and"),
            Slot(max, SlotKind::Number),
        ],
        Expression::MinMax { op, left, right } => vec![
            Part::Choice(op, &axcenity_blocks::MIN_MAX_OPS),
            label("of"),
            Slot(left, SlotKind::Number),
            label("and"),
            Slot(right, SlotKind::Number),
        ],
        Expression::StringLength { text } => {
            vec![label("length of text"), Slot(text, SlotKind::Text)]
        }
        Expression::StringContains { text, substring } => vec![
            Slot(text, SlotKind::Text),
            label("contains"),
            Slot(substring, SlotKind::Text),
        ],
        Expression::FormatNumber { value, decimals } => vec![
            label("format"),
            Slot(value, SlotKind::Number),
            label("to"),
            Slot(decimals, SlotKind::Number),
            label("decimals"),
        ],
        Expression::MakeColorRgb { r, g, b } => vec![
            label("rgb ("),
            Slot(r, SlotKind::Number),
            label(","),
            Slot(g, SlotKind::Number),
            label(","),
            Slot(b, SlotKind::Number),
            label(")"),
        ],
        Expression::MakeColorHsv { h, s, v } => vec![
            label("hsv ("),
            Slot(h, SlotKind::Number),
            label(","),
            Slot(s, SlotKind::Number),
            label(","),
            Slot(v, SlotKind::Number),
            label(")"),
        ],
        Expression::AnimatorNewState => vec![label("new state")],
        Expression::AnimatorPreviousState => vec![label("previous state")],
        Expression::AnimationEventName => vec![label("event name")],
        Expression::ConnectionFailureReason => vec![label("failure reason")],
        Expression::ChangedLayerName => vec![label("layer name")],
        Expression::ChangedLayerActive => vec![label("layer is active")],
        Expression::ChatMessageText => vec![label("chat message")],
        Expression::ChatMessageSender => vec![label("chat sender")],
        Expression::IsHost => vec![label("is host")],
        Expression::IsConnected => vec![label("is connected")],
        Expression::SlotExists { slot } => {
            vec![
                label("save slot"),
                Slot(slot, SlotKind::Number),
                label("exists"),
            ]
        }
        Expression::DataTableRowCount { table } => vec![
            label("number of rows in"),
            Part::Name(table, NameKind::DataTable, STORAGE_PATH.to_owned()),
        ],
        Expression::Join { left, right } => vec![
            label("join"),
            Slot(left, SlotKind::Text),
            Slot(right, SlotKind::Text),
        ],
        Expression::Compare { op, left, right } => vec![
            Slot(left, SlotKind::Any),
            Part::Compare(op),
            Slot(right, SlotKind::Any),
        ],
        Expression::Logic { op, left, right } => vec![
            Slot(left, SlotKind::Condition),
            Part::Logic(op),
            Slot(right, SlotKind::Condition),
        ],
        Expression::Not { value } => vec![label("not"), Slot(value, SlotKind::Condition)],
        Expression::Property { target, property } => {
            let class = class_of(objects, target);
            vec![
                Property(property, class),
                label("of"),
                object(target, Filter::Any),
            ]
        }
        Expression::Attribute { target, attribute } => {
            let path = target.as_path().unwrap_or("").to_owned();
            vec![
                label("attribute"),
                Part::Name(attribute, NameKind::Attribute, path),
                label("of"),
                object(target, Filter::Any),
            ]
        }
        Expression::SoundProperty { target, property } => vec![
            Part::Choice(property, &axcenity_blocks::SOUND_PROPERTIES),
            label("of sound"),
            object(target, Filter::Sounds),
        ],
        Expression::Raycast {
            origin,
            direction,
            output,
            ignore,
        } => {
            if output.is_empty() {
                *output = "Result".to_owned();
            }
            let mut parts = vec![
                label("raycast"),
                Part::Choice(output, &axcenity_blocks::RAYCAST_OUTPUTS),
                label("from"),
                Slot(origin, SlotKind::Vector),
                label("along"),
                Slot(direction, SlotKind::Vector),
            ];
            if let Some(ig) = ignore {
                parts.push(label("ignoring"));
                parts.push(object(ig, Filter::Any));
            }
            parts
        }
        Expression::RaycastIgnoring {
            origin,
            direction,
            ignore,
        } => vec![
            label("raycast from"),
            Slot(origin, SlotKind::Vector),
            label("along"),
            Slot(direction, SlotKind::Vector),
            label("ignoring"),
            object(ignore, Filter::Any),
        ],
        Expression::TouchingPart => vec![label("touching part")],
        Expression::TriggeringPlayer => vec![label("triggering player")],
        Expression::CommandArgs => vec![label("command args")],
        Expression::DeltaTime => vec![label("delta time")],
        Expression::PartUnderMouse => vec![label("part under mouse")],
        Expression::MouseHitPosition => vec![label("mouse hit position")],
        Expression::CameraPosition => vec![label("camera position")],
        Expression::CameraLookDirection => vec![label("camera look direction")],
        Expression::NearestPlayerTo { position } => {
            vec![label("nearest player to"), Slot(position, SlotKind::Vector)]
        }
        Expression::PlayerOfPart { part } => {
            vec![label("player of"), object(part, Filter::Parts)]
        }
        Expression::PlayerIsAlive { player } => {
            vec![object(player, Filter::Any), label("is alive")]
        }
        Expression::ParentOf { target } => {
            vec![label("parent of"), object(target, Filter::Any)]
        }
        Expression::ChildOf { name, target } => vec![
            label("child"),
            Slot(name, SlotKind::Text),
            label("of"),
            object(target, Filter::Any),
        ],
        Expression::HasTag { target, tag } => vec![
            object(target, Filter::Any),
            label("has tag"),
            Slot(tag, SlotKind::Text),
        ],
        Expression::GameTime => vec![label("game time")],
        Expression::KeyDown { key } => vec![label("key"), Key(key), label("pressed?")],
        Expression::LayerActive { data_layer } => {
            vec![label("layer"), Part::Text(data_layer), label("is active")]
        }
        Expression::IsStreamedIn { value } => {
            vec![Slot(value, SlotKind::Any), label("is streamed in")]
        }
        Expression::IsA { value, class } => vec![
            Slot(value, SlotKind::Any),
            label("is a"),
            Part::Choice(class, &axcenity_blocks::IS_A_CLASSES),
        ],
        Expression::AnimatorBool {
            animator,
            parameter,
        } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![
                Part::Name(parameter, NameKind::Bool, path),
                label("of"),
                object(animator, Filter::Animators),
            ]
        }
        Expression::AnimatorLayerWeight { animator, layer } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![
                label("weight of layer"),
                Part::Name(layer, NameKind::Layer, path),
                label("of"),
                object(animator, Filter::Animators),
            ]
        }
        Expression::AnimatorNumber {
            animator,
            parameter,
        } => {
            let path = animator.as_path().unwrap_or("").to_owned();
            vec![
                Part::Name(parameter, NameKind::Number, path),
                label("of"),
                object(animator, Filter::Animators),
            ]
        }
        Expression::SplinePosition { spline, t } => vec![
            label("point"),
            Slot(t, SlotKind::Number),
            label("of the way along"),
            object(spline, Filter::Splines),
        ],
        Expression::SplineClosestDistance { spline, position } => vec![
            label("distance along"),
            object(spline, Filter::Splines),
            label("nearest"),
            Slot(position, SlotKind::Vector),
        ],
        Expression::DataRow { table, key } => vec![
            label("row"),
            Slot(key, SlotKind::Text),
            label("of table"),
            Part::Name(table, NameKind::DataTable, STORAGE_PATH.to_owned()),
        ],
        Expression::DataValue { column, row } => vec![
            label("value of column"),
            Part::Name(column, NameKind::Column, STORAGE_PATH.to_owned()),
            label("in row"),
            Slot(row, SlotKind::Any),
        ],
        Expression::Code { code } => vec![Part::Code(shown_code(code))],
        Expression::Field { object, name } => {
            vec![Slot(object, SlotKind::Any), label("."), Part::Text(name)]
        }
        Expression::Index { object, key } => vec![
            Slot(object, SlotKind::Any),
            label("["),
            Slot(key, SlotKind::Any),
            label("]"),
        ],
        Expression::Call { function, args } => {
            let mut parts = vec![Slot(function, SlotKind::Any), label("(")];
            push_slots(args, &mut parts);
            parts.push(label(")"));
            parts
        }
        Expression::MethodCall {
            object,
            method,
            args,
        } => {
            let mut parts = vec![
                Slot(object, SlotKind::Any),
                label(":"),
                Part::Text(method),
                label("("),
            ];
            push_slots(args, &mut parts);
            parts.push(label(")"));
            parts
        }
        Expression::Table { fields, .. } => {
            let mut parts = vec![label("{")];
            for (index, field) in fields.iter_mut().enumerate() {
                if index > 0 {
                    parts.push(label(","));
                }
                match field {
                    TableField::Named { name, value } => {
                        parts.push(Part::Text(name));
                        parts.push(label("="));
                        parts.push(Slot(value, SlotKind::Any));
                    }
                    TableField::Keyed { key, value } => {
                        parts.push(label("["));
                        parts.push(Slot(key, SlotKind::Any));
                        parts.push(label("] ="));
                        parts.push(Slot(value, SlotKind::Any));
                    }
                    TableField::Item { value } => parts.push(Slot(value, SlotKind::Any)),
                }
            }
            parts.push(label("}"));
            parts
        }
        Expression::Binary { op, left, right } => vec![
            Slot(left, SlotKind::Any),
            Part::Choice(op, &axcenity_blocks::LUA_BINARY_OPS),
            Slot(right, SlotKind::Any),
        ],
        Expression::Unary { op, value } => vec![
            Part::Choice(op, &axcenity_blocks::LUA_UNARY_OPS),
            Slot(value, SlotKind::Any),
        ],
        Expression::Paren { value } => {
            vec![label("("), Slot(value, SlotKind::Any), label(")")]
        }
        Expression::Interpolated {
            parts: texts,
            values,
        } => {
            let mut parts = Vec::new();
            let mut values = values.iter_mut();
            for text in texts {
                parts.push(Part::Text(text));
                if let Some(value) = values.next() {
                    parts.push(Slot(value, SlotKind::Any));
                }
            }
            parts
        }
        Expression::IfValue { parts: values } => {
            let count = values.len();
            let mut parts = Vec::new();
            for (index, value) in values.iter_mut().enumerate() {
                parts.push(label(if index + 1 == count {
                    "else"
                } else if index == 0 {
                    "if"
                } else if index % 2 == 0 {
                    "elseif"
                } else {
                    "then"
                }));
                parts.push(Slot(value, SlotKind::Any));
            }
            parts
        }
        Expression::Cast { value, annotation } => vec![
            Slot(value, SlotKind::Any),
            label("::"),
            Part::Text(annotation),
        ],
        // Values typed in aren't reporters, and are drawn as inputs.
        Expression::Number { .. }
        | Expression::Text { .. }
        | Expression::Boolean { .. }
        | Expression::Vector3 { .. }
        | Expression::Color3 { .. }
        | Expression::ObjectPath { .. }
        | Expression::Nil
        | Expression::Vararg
        | Expression::Name { .. }
        | Expression::Closure { .. } => Vec::new(),
    }
}

/// Where a value slot is: the block it's in, then which slot at each level
/// down through the reporters in it.
#[derive(Clone, Debug, PartialEq)]
struct SlotAddress {
    block: u64,
    path: Vec<usize>,
}

impl SlotAddress {
    fn child(&self, index: usize) -> SlotAddress {
        let mut path = self.path.clone();
        path.push(index);
        SlotAddress {
            block: self.block,
            path,
        }
    }

    fn resolve<'a>(&self, script: &'a mut BlockScript) -> Option<&'a mut Expression> {
        let block = script.find_mut(self.block)?;
        let (first, rest) = self.path.split_first()?;
        let mut slot = block.kind.expressions_mut().into_iter().nth(*first)?;
        for &index in rest {
            slot = slot.children_mut().into_iter().nth(index)?;
        }
        Some(slot)
    }
}

/// Where dragged stacks can snap, at the point their top-left corner goes.
type StackSpots = Vec<(Pos2, StackSpot)>;
/// Value slots a dragged reporter can drop into.
type SlotSpots = Vec<(Rect, SlotAddress, SlotKind)>;

/// Somewhere a dragged stack can snap to.
#[derive(Clone, Debug, PartialEq)]
enum StackSpot {
    /// Under this block.
    After { block: u64, last: bool },
    /// At the top of this mouth of a C-shaped block, or under an event.
    Mouth {
        block: u64,
        mouth: usize,
        empty: bool,
    },
    /// Above the loose stack at this index of the script's top level.
    Above(usize),
}

/// What's being dragged.
#[derive(Clone, Debug)]
enum Dragged {
    Stack(Vec<Block>),
    Reporter(Expression),
}

#[derive(Clone, Debug)]
struct Drag {
    item: Dragged,
    /// From the dragged item's top-left corner to the pointer.
    grab: Vec2,
    /// Its size as last drawn, for snapping it above a stack.
    size: Vec2,
}

/// What the editor remembers between frames for one script.
#[derive(Clone, Debug)]
struct State {
    pan: Vec2,
    zoom: f32,
    category: Category,
    /// A category clicked, for the palette to scroll to.
    scroll_to: Option<Category>,
    show_lua: bool,
    drag: Option<Drag>,
    /// The name being typed for a new variable, while the box is open.
    new_variable: Option<String>,
    /// Initial value typed for a new variable (parsed as number, bool, or text).
    new_variable_initial: String,
    /// Whether a new variable is Global / Shared (`_G`) instead of This Script.
    new_variable_global: bool,
    /// Focus the new-variable text box on the frame it opens.
    focus_new_variable: bool,
    /// Variables made with "Make a Variable" that no block uses yet.
    variables: Vec<String>,
    /// Typed in the palette's search box: only matching blocks are shown.
    search: String,
    /// Categories whose "Lua blocks" are opened in the palette.
    luau_open: Vec<Category>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            category: Category::Events,
            scroll_to: None,
            show_lua: false,
            drag: None,
            new_variable: None,
            new_variable_initial: "0".to_owned(),
            new_variable_global: false,
            focus_new_variable: false,
            variables: Vec::new(),
            search: String::new(),
            luau_open: Vec::new(),
        }
    }
}

/// Something done to a block from its right-click menu.
enum Action {
    Duplicate(u64),
    Delete(u64),
    /// One more of a call's arguments, or a local's values.
    AddInput(u64),
    RemoveInput(u64),
}

/// Every variable the script uses, and those just made, sorted.
fn known_variables(script: &BlockScript, made: &[String]) -> Vec<String> {
    let mut names: BTreeSet<String> = made.iter().cloned().collect();
    for def in &script.variables {
        names.insert(def.name.clone());
    }
    fn visit(expression: &Expression, names: &mut BTreeSet<String>) {
        if let Expression::Variable { name } = expression {
            names.insert(name.clone());
        }
        for child in expression.children() {
            visit(child, names);
        }
    }
    script.for_each_block(&mut |block| {
        if let BlockKind::SetVariable { name, .. }
        | BlockKind::ChangeVariable { name, .. }
        | BlockKind::ListCreate { name, .. } = &block.kind
        {
            names.insert(name.clone());
        }
        for expression in block.kind.expressions() {
            visit(expression, &mut names);
        }
    });
    names.into_iter().collect()
}

/// Draws the block editor for `script`. Returns true if the script changed.
pub fn show(ui: &mut egui::Ui, script: &mut BlockScript, context: BlockContext) -> bool {
    let before = script.clone();
    let state_id = egui::Id::new(("block editor", context.key));
    let mut state: State = ui
        .data_mut(|data| data.get_temp(state_id))
        .unwrap_or_default();
    let variables = known_variables(script, &state.variables);

    let full = ui.available_rect_before_wrap();
    let toolbar = Rect::from_min_size(full.min, vec2(full.width(), TOOLBAR_HEIGHT));
    let body = Rect::from_min_max(pos2(full.left(), toolbar.bottom()), full.max);
    let categories = Rect::from_min_size(body.min, vec2(CATEGORY_WIDTH, body.height()));
    let lua_width = if state.show_lua {
        (body.width() * 0.3).clamp(260.0, 460.0)
    } else {
        0.0
    };
    let palette_width_id = egui::Id::new("block_editor_palette_width");
    let max_palette = PALETTE_MAX_WIDTH
        .min((body.width() - CATEGORY_WIDTH - lua_width - 140.0).max(PALETTE_MIN_WIDTH));
    let mut palette_width = ui
        .data_mut(|data| data.get_persisted::<f32>(palette_width_id))
        .unwrap_or(PALETTE_WIDTH)
        .clamp(PALETTE_MIN_WIDTH, max_palette);
    // Dedicated non-overlapping strip between the palette and the canvas so
    // neither the palette scrollbar nor the canvas steals hover/drag events.
    let splitter_rect = Rect::from_min_max(
        pos2(
            categories.right() + palette_width - PALETTE_SPLITTER_WIDTH,
            body.top(),
        ),
        pos2(categories.right() + palette_width, body.bottom()),
    );
    let splitter = ui
        .interact(
            splitter_rect,
            egui::Id::new(("block palette splitter", context.key)),
            Sense::drag(),
        )
        .on_hover_cursor(CursorIcon::ResizeHorizontal)
        .on_hover_text("Drag to resize the blocks sidebar");
    if splitter.dragged() {
        palette_width =
            (palette_width + splitter.drag_delta().x).clamp(PALETTE_MIN_WIDTH, max_palette);
        ui.data_mut(|data| data.insert_persisted(palette_width_id, palette_width));
    }
    let splitter_rect = Rect::from_min_max(
        pos2(
            categories.right() + palette_width - PALETTE_SPLITTER_WIDTH,
            body.top(),
        ),
        pos2(categories.right() + palette_width, body.bottom()),
    );
    let palette_rect = Rect::from_min_max(
        pos2(categories.right(), body.top()),
        pos2(splitter_rect.left(), body.bottom()),
    );
    let canvas = Rect::from_min_max(
        pos2(splitter_rect.right(), body.top()),
        pos2(body.right() - lua_width, body.bottom()),
    );
    let lua_rect = Rect::from_min_max(pos2(canvas.right(), body.top()), body.max);

    show_toolbar(ui, toolbar, &mut state, &context, script);
    show_categories(ui, categories, &mut state);
    show_palette(ui, palette_rect, script, &mut state, &context, &variables);
    let (stack_spots, slot_spots) =
        show_canvas(ui, canvas, script, &mut state, &context, &variables);
    // Paint the resize handle between palette and canvas so it is always visible.
    {
        let painter = ui.painter();
        let active = splitter.hovered() || splitter.dragged();
        let bg = if active {
            theme::palette(ui.ctx()).accent.gamma_multiply(0.18)
        } else {
            ui.visuals().faint_bg_color
        };
        painter.rect_filled(splitter_rect, 0, bg);
        let line_color = if active {
            theme::palette(ui.ctx()).accent
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke.color
        };
        painter.vline(
            splitter_rect.right() - 0.5,
            splitter_rect.y_range(),
            Stroke::new(if active { 2.0 } else { 1.0 }, line_color),
        );
        let grip = Rect::from_center_size(splitter_rect.center(), vec2(3.0, 36.0));
        painter.rect_filled(
            grip,
            1.5,
            if active {
                theme::palette(ui.ctx()).accent
            } else {
                ui.visuals().weak_text_color().gamma_multiply(0.55)
            },
        );
    }
    if state.show_lua {
        show_lua(ui, lua_rect, script);
    }
    finish_drag(
        ui,
        &mut state,
        script,
        &context,
        &variables,
        canvas,
        palette_rect.union(categories).union(splitter_rect),
        &stack_spots,
        &slot_spots,
    );
    ui.advance_cursor_after_rect(full);
    ui.data_mut(|data| data.insert_temp(state_id, state));
    *script != before
}

fn show_toolbar(
    ui: &mut egui::Ui,
    rect: Rect,
    state: &mut State,
    context: &BlockContext,
    script: &mut BlockScript,
) {
    ui.painter().rect_filled(rect, 0, ui.visuals().panel_fill);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
    let mut bar = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(10.0, 6.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let palette = theme::palette(ui.ctx());
    let (icon, side, color, tip) = if context.client {
        (
            icons::USER,
            "Client blocks",
            palette.success,
            "Runs on each player's side, like a LocalScript: it can use the keyboard and UI clicks.",
        )
    } else {
        (
            icons::CLOUD,
            "Server blocks",
            palette.accent,
            "Runs on the server, like a Script. Keyboard and UI click blocks need a client block script.",
        )
    };
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0_f32, color))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(&mut bar, |ui| {
            ui.label(
                egui::RichText::new(theme::with_icon(icon, side))
                    .color(color)
                    .strong(),
            );
        })
        .response
        .on_hover_text(tip);
    // The buttons go on the right first; the hint gets whatever room is left
    // and shortens itself (full text on hover) instead of running under them.
    let narrow = rect.width() < TOOLBAR_WORDS_FROM;
    let worded = |icon: &str, text: &str| {
        if narrow {
            icon.to_owned()
        } else {
            theme::with_icon(icon, text)
        }
    };
    bar.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let lua = egui::Button::new(worded(icons::CODE, "View as Lua")).selected(state.show_lua);
        if ui
            .add(lua)
            .on_hover_text(
                "View as Lua: shows the Lua these blocks become. It runs exactly like a typed script.",
            )
            .clicked()
        {
            state.show_lua = !state.show_lua;
        }
        ui.separator();
        if ui
            .button(icons::MAGNIFYING_GLASS_PLUS)
            .on_hover_text("Zoom in")
            .clicked()
        {
            state.zoom = (state.zoom * 1.2).min(2.0);
        }
        if ui
            .button(format!("{:.0}%", state.zoom * 100.0))
            .on_hover_text("Back to 100%")
            .clicked()
        {
            state.zoom = 1.0;
        }
        if ui
            .button(icons::MAGNIFYING_GLASS_MINUS)
            .on_hover_text("Zoom out")
            .clicked()
        {
            state.zoom = (state.zoom / 1.2).max(0.5);
        }
        if ui
            .button(worded(icons::BROOM, "Clean up"))
            .on_hover_text("Clean up: lines the scripts up down the left side")
            .clicked()
        {
            for block in &mut script.blocks {
                block.position = None;
            }
            state.pan = Vec2::ZERO;
        }
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add(egui::Label::new(egui::RichText::new(TOOLBAR_HINT).weak()).truncate())
                    .on_hover_text(TOOLBAR_HINT);
            },
        );
    });
}

/// What the Blocks toolbar says between the side chip and its buttons.
pub(crate) const TOOLBAR_HINT: &str =
    "Drag blocks onto the canvas. Drop them back on the palette to delete.";

/// Below this width the toolbar's buttons show only their icons.
const TOOLBAR_WORDS_FROM: f32 = 560.0;

fn show_categories(ui: &mut egui::Ui, rect: Rect, state: &mut State) {
    let visuals = ui.visuals().clone();
    ui.painter().rect_filled(rect, 0, visuals.panel_fill);
    ui.painter().vline(
        rect.right() - 0.5,
        rect.y_range(),
        visuals.widgets.noninteractive.bg_stroke,
    );
    let mut rail = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    rail.set_clip_rect(rect);
    egui::ScrollArea::vertical()
        .id_salt("block category rail")
        .auto_shrink(false)
        .show(&mut rail, |ui| {
            ui.add_space(6.0);
            for category in Category::ALL {
                let (item, response) =
                    ui.allocate_exact_size(vec2(rect.width(), 52.0), Sense::click());
                let response = response
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_text(category.name());
                if category == state.category && state.search.is_empty() {
                    ui.painter().rect_filled(
                        item.shrink(3.0),
                        6,
                        visuals.selection.bg_fill.gamma_multiply(0.4),
                    );
                } else if response.hovered() {
                    ui.painter().rect_filled(
                        item.shrink(3.0),
                        6,
                        visuals.widgets.hovered.weak_bg_fill,
                    );
                }
                let palette = theme::palette(ui.ctx());
                let swatch = Rect::from_center_size(
                    pos2(item.center().x, item.top() + 18.0),
                    vec2(26.0, 26.0),
                );
                ui.painter().rect_filled(
                    swatch.translate(vec2(0.0, DROP)),
                    8,
                    category.edge(&palette),
                );
                ui.painter().rect_filled(swatch, 8, category.fill(&palette));
                ui.painter().text(
                    swatch.center(),
                    egui::Align2::CENTER_CENTER,
                    category.icon(),
                    FontId::proportional(15.0),
                    Color32::WHITE,
                );
                ui.painter().text(
                    pos2(item.center().x, item.top() + 41.0),
                    egui::Align2::CENTER_CENTER,
                    category.short_name(),
                    FontId::new(10.5, theme::medium_family()),
                    visuals.text_color(),
                );
                if response.clicked() {
                    state.category = category;
                    state.scroll_to = Some(category);
                    state.search.clear();
                }
            }
            ui.add_space(6.0);
        });
}

#[cfg(test)]
mod tests;
