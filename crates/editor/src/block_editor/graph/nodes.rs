//! What each block looks like as a node: its title, its pins and the names
//! on them. The pieces come from the block's own `sections` (or a
//! reporter's `reporter_parts`), so a node always has the same inputs and
//! menus as its block; this file only names them.

use super::*;

/// The exec outputs a block's node has, with their names, top to bottom.
/// Events have one (what runs when it happens); C-shaped blocks one per
/// mouth and then Completed; blocks that never finish have no Then.
pub(super) fn exec_outs(kind: &BlockKind) -> Vec<(Out, &'static str)> {
    let mut outs = own_exec_outs(kind);
    // A function in one of its values is one more place blocks run from,
    // before Completed.
    let own = outs
        .iter()
        .filter(|(out, _)| matches!(out, Out::Body(_)))
        .count();
    let at = if matches!(outs.last(), Some((Out::Then, _))) {
        outs.len() - 1
    } else {
        outs.len()
    };
    for mouth in (own..kind.bodies().len()).rev() {
        outs.insert(at, (Out::Body(mouth), "Function"));
    }
    outs
}

const SEQUENCE_LABELS: [&str; 12] = [
    "Then 0", "Then 1", "Then 2", "Then 3", "Then 4", "Then 5", "Then 6", "Then 7", "Then 8",
    "Then 9", "Then 10", "Then 11",
];

fn own_exec_outs(kind: &BlockKind) -> Vec<(Out, &'static str)> {
    match kind {
        BlockKind::Unattached { .. }
        | BlockKind::TopCode { .. }
        | BlockKind::StopScript
        | BlockKind::DestroyScript
        | BlockKind::Break
        | BlockKind::Continue
        | BlockKind::Return { .. } => Vec::new(),
        kind if kind.is_event() => vec![(Out::Body(0), "")],
        BlockKind::Repeat { .. }
        | BlockKind::RepeatUntil { .. }
        | BlockKind::ForRange { .. }
        | BlockKind::ForEachInList { .. }
        | BlockKind::ForEachTagged { .. }
        | BlockKind::ForEachRow { .. }
        | BlockKind::ForEachChild { .. }
        | BlockKind::ForEachPartInRadius { .. }
        | BlockKind::ForEachPartTouching { .. }
        | BlockKind::While { .. }
        | BlockKind::RepeatLoop { .. }
        | BlockKind::NumericFor { .. }
        | BlockKind::GenericFor { .. } => {
            vec![(Out::Body(0), "Loop Body"), (Out::Then, "Completed")]
        }
        BlockKind::Sequence { branches } => (0..branches.len())
            .map(|i| {
                (
                    Out::Body(i),
                    SEQUENCE_LABELS.get(i).copied().unwrap_or("Then"),
                )
            })
            .collect(),
        BlockKind::CooldownGate { .. } => {
            vec![(Out::Body(0), "Allowed"), (Out::Then, "Completed")]
        }
        BlockKind::SpawnAsync { .. } => {
            vec![(Out::Body(0), "Async Body"), (Out::Then, "Completed")]
        }
        BlockKind::SlideTo { .. }
        | BlockKind::RotateTo { .. }
        | BlockKind::TweenProperty { .. }
        | BlockKind::Tween { .. }
        | BlockKind::SlideIn { .. }
        | BlockKind::SlideOut { .. }
        | BlockKind::FadeIn { .. }
        | BlockKind::FadeOut { .. }
        | BlockKind::TweenPivot { .. } => {
            vec![(Out::Then, ""), (Out::Body(0), "Completed")]
        }
        BlockKind::Connect { .. } => {
            vec![(Out::Body(0), "On Event"), (Out::Then, "Completed")]
        }
        BlockKind::DefineFunction { .. } | BlockKind::DoBlock { .. } => {
            vec![(Out::Body(0), "Body"), (Out::Then, "Completed")]
        }
        BlockKind::Forever { .. } => vec![(Out::Body(0), "Loop Body")],
        // An if is a Branch whose False isn't wired yet.
        BlockKind::If { .. } | BlockKind::IfElse { .. } => vec![
            (Out::Body(0), "True"),
            (Out::Body(1), "False"),
            (Out::Then, "Completed"),
        ],
        _ => vec![(Out::Then, "")],
    }
}

/// Whether a block's node has an exec input: every block in a stack.
pub(super) fn has_exec_in(kind: &BlockKind) -> bool {
    !kind.is_event()
}

/// A block's node title, and the names of its menus and inputs in the
/// order its parts show them (labels and icons aside).
pub(super) fn block_names(kind: &BlockKind) -> (&'static str, &'static [&'static str]) {
    match kind {
        BlockKind::WhenStarted { .. } => ("When Script Starts", &[]),
        BlockKind::WhenTouched { .. } => ("When Touched", &["Part", "Filter"]),
        BlockKind::WhenTouchEnds { .. } => ("When Touch Ends", &["Part", "Filter"]),
        BlockKind::WhenStreamedIn { .. } => ("When Streamed In", &["Object"]),
        BlockKind::WhenStreamedOut { .. } => ("When Streamed Out", &["Object"]),
        BlockKind::WhenKeyPressed { .. } => ("When Key Pressed", &["Key"]),
        BlockKind::WhenKeyReleased { .. } => ("When Key Released", &["Key"]),
        BlockKind::WhenPlayerJoins { .. } => ("When Player Joins", &[]),
        BlockKind::WhenPlayerLeaves { .. } => ("When Player Leaves", &[]),
        BlockKind::WhenCharacterSpawns { .. } => ("When Character Spawns", &[]),
        BlockKind::WhenPlayerDies { .. } => ("When Player Dies", &[]),
        BlockKind::WhenChildAdded { .. } => ("When Child Added", &["Object"]),
        BlockKind::WhenChildRemoved { .. } => ("When Child Removed", &["Object"]),
        BlockKind::WhenTagAdded { .. } => ("When Tag Added", &["Tag"]),
        BlockKind::WhenTagRemoved { .. } => ("When Tag Removed", &["Tag"]),
        BlockKind::WhenRemoteFires { .. } => ("When Remote Fires", &["Remote", "Side"]),
        BlockKind::Disconnect { .. } => ("Disconnect", &["Connection"]),
        BlockKind::WhenClicked { .. } => ("When Button Clicked", &["Button"]),
        BlockKind::WhenMouseEnters { .. } => ("When Mouse Enters", &["Element"]),
        BlockKind::WhenMouseLeaves { .. } => ("When Mouse Leaves", &["Element"]),
        BlockKind::WhenValueChanged { .. } => ("When Value Changes", &["Widget"]),
        BlockKind::EveryFrame { .. } => ("Every Frame", &[]),
        BlockKind::BeforePhysics { .. } => ("Before Physics", &[]),
        BlockKind::BeforeRender { .. } => ("Before Drawing", &[]),
        BlockKind::WhenEventFired { .. } => ("When Event Fires", &["Event"]),
        BlockKind::FireEvent { .. } => ("Fire Event", &["Event"]),
        BlockKind::DestroyAfter { .. } => ("Destroy After", &["Object", "Seconds"]),
        BlockKind::WhenSoundEnded { .. } => ("When Sound Ends", &["Sound"]),
        BlockKind::WhenCutsceneFinished { .. } => ("When Cutscene Finishes", &["Sequence"]),
        BlockKind::WhenStateChanged { .. } => ("When State Changes", &["Animator"]),
        BlockKind::WhenAnimationEvent { .. } => ("When Animation Event", &["Animator", "Event"]),
        BlockKind::WhenAnimationFinished { .. } => {
            ("When Animation Finishes", &["Animation", "Animator"])
        }
        BlockKind::WhenAgentSenses { .. } => ("When Agent Senses Player", &["Agent", "Sense"]),
        BlockKind::WhenAgentLosesPlayer { .. } => ("When Agent Loses Player", &["Agent", "Sense"]),
        BlockKind::WhenAgentArrives { .. } => ("When Agent Arrives", &["Agent"]),
        BlockKind::WhenSplineReached { .. } => ("When Spline End Reached", &["Follower", "End"]),
        BlockKind::WhenSeatOccupied { .. } => ("When Someone Sits", &["Seat"]),
        BlockKind::WhenSeatVacated { .. } => ("When Someone Leaves Seat", &["Seat"]),
        BlockKind::WhenLayerChanged { .. } => ("When Layer Changes", &["Layer"]),
        BlockKind::WhenPropertyChanged { .. } => ("When Property Changes", &["Property", "Object"]),
        BlockKind::WhenAttributeChanged { .. } => {
            ("When Attribute Changes", &["Attribute", "Object"])
        }
        BlockKind::WhenPromptTriggered { .. } => ("When Prompt Triggered", &["Prompt"]),
        BlockKind::WhenPartClicked { .. } => ("When Part Clicked", &["Click Detector"]),
        BlockKind::WhenCommandRun { .. } => ("When Command Run", &["Command", "Help"]),
        BlockKind::Unattached { .. } => ("Loose Blocks", &[]),
        BlockKind::TopCode { .. } => ("Lua (runs first)", &["Code"]),
        BlockKind::Wait { .. } => ("Wait", &["Seconds"]),
        BlockKind::WaitUntil { .. } => ("Wait Until", &["Condition"]),
        BlockKind::Repeat { .. } => ("Repeat", &["Times"]),
        BlockKind::Forever { .. } => ("Forever", &[]),
        BlockKind::RepeatUntil { .. } => ("Repeat Until", &["Condition"]),
        BlockKind::If { .. } | BlockKind::IfElse { .. } => ("Branch", &["Condition"]),
        BlockKind::StopScript => ("Stop This Script", &[]),
        BlockKind::DestroyScript => ("Destroy / Stop Script", &[]),
        BlockKind::ForRange { .. } => ("For Loop (Range)", &["Start", "End", "Step"]),
        BlockKind::ForEachInList { .. } => ("For Each In List", &["List"]),
        BlockKind::ForEachTagged { .. } => ("For Each Tagged", &["Tag"]),
        BlockKind::Sequence { .. } => ("Sequence", &[]),
        BlockKind::CooldownGate { .. } => ("Do Once / Cooldown Gate", &["Cooldown (s)"]),
        BlockKind::SpawnAsync { .. } => ("Spawn / Delay (Async)", &["Delay (s)"]),
        BlockKind::ForEachChild { .. } => ("For Each Child", &["Variable", "Object"]),
        BlockKind::ForEachPartInRadius { .. } => {
            ("For Each Part In Radius", &["Variable", "Radius", "Center"])
        }
        BlockKind::ForEachPartTouching { .. } => ("For Each Part Touching", &["Variable", "Part"]),
        BlockKind::WaitForChild { .. } => ("Wait For Child", &["Name", "Object"]),
        BlockKind::WaitForEvent { .. } => ("Wait For Event", &["Object", "Event"]),
        BlockKind::CloneInto { .. } => ("Clone", &["Object", "Into"]),
        BlockKind::MoveTo { .. } => ("Move To", &["Part", "Position"]),
        BlockKind::MoveBy { .. } => ("Move By", &["Part", "Offset"]),
        BlockKind::LookAt { .. } => ("Look At", &["Part", "Position"]),
        BlockKind::SetCameraType { .. } => ("Set Camera Type", &["Camera Type"]),
        BlockKind::PointCameraAt { .. } => ("Point Camera At", &["Position"]),
        BlockKind::ShakeCamera { .. } => ("Shake Camera", &["Intensity", "Seconds"]),
        BlockKind::SpawnDecal { .. } => (
            "Spawn Decal",
            &["Texture", "Position", "Facing", "Size", "Seconds"],
        ),
        BlockKind::MoveForward { .. } => ("Move Forward", &["Part", "Distance"]),
        BlockKind::TurnTo { .. } => ("Turn To", &["Part", "Orientation"]),
        BlockKind::TurnBy { .. } => ("Turn By", &["Part", "Rotation", "Space"]),
        BlockKind::SlideTo { .. } => ("Slide To", &["Object", "Position", "Duration", "Easing"]),
        BlockKind::RotateTo { .. } => ("Rotate To", &["Object", "Rotation", "Duration", "Easing"]),
        BlockKind::SetGravity { .. } => ("Set Gravity", &["Part", "Multiplier"]),
        BlockKind::SetAngularVelocity { .. } => ("Set Angular Velocity", &["Part", "Velocity"]),
        BlockKind::LaunchPart { .. } => ("Launch Part", &["Part", "Direction", "Speed"]),
        BlockKind::SetAnchored { .. } => ("Set Anchored", &["Part", "Anchored"]),
        BlockKind::PushWithForce { .. } => ("Push", &["Part", "Force"]),
        BlockKind::SetPartVelocity { .. } => ("Set Part Velocity", &["Part", "Velocity"]),
        BlockKind::SetFriction { .. } => ("Set Friction", &["Part", "Friction"]),
        BlockKind::SetDestination { .. } => ("Walk To", &["Agent", "Destination"]),
        BlockKind::FollowTarget { .. } => ("Follow Target", &["Agent", "Target"]),
        BlockKind::StopAgent { .. } => ("Stop Agent", &["Agent"]),
        BlockKind::SetVelocity { .. } => ("Set Agent Velocity", &["Agent", "Velocity"]),
        BlockKind::SitInSeat { .. } => ("Sit In Seat", &["Seat", "Controller"]),
        BlockKind::LeaveSeat { .. } => ("Leave Seat", &["Controller"]),
        BlockKind::ControlSpline { .. } => ("Control Spline", &["Action", "Follower"]),
        BlockKind::SetSplineSpeed { .. } => ("Set Spline Speed", &["Follower", "Speed"]),
        BlockKind::SetColor { .. } => ("Set Color", &["Object", "Color"]),
        BlockKind::SetText { .. } => ("Set Text", &["Object", "Text"]),
        BlockKind::Show { .. } => ("Show", &["Object"]),
        BlockKind::Hide { .. } => ("Hide", &["Object"]),
        BlockKind::ToggleVisibility { .. } => ("Toggle Visibility", &["Object"]),
        BlockKind::SetTransparency { .. } => ("Set Transparency", &["Object", "Transparency"]),
        BlockKind::SetMaterial { .. } => ("Set Material", &["Part", "Material"]),
        BlockKind::EmitParticles { .. } => ("Emit Particles", &["Emitter", "Count"]),
        BlockKind::SetEffectEnabled { .. } => ("Set Effect Enabled", &["Effect", "Enabled"]),
        BlockKind::SetLightingProperty { .. } => ("Set Lighting Property", &["Property", "Value"]),
        BlockKind::SetPostProcessing { .. } => {
            ("Set Post-Processing", &["Effect", "Property", "Value"])
        }
        BlockKind::TweenProperty { .. } => (
            "Tween Property",
            &["Property", "Object", "Value", "Duration", "Easing"],
        ),
        BlockKind::SetImage { .. } => ("Set Image", &["Object", "Image"]),
        BlockKind::ShowPage { .. } => ("Show Page", &["Page", "Layout"]),
        BlockKind::ScrollTo { .. } => ("Scroll To", &["Frame", "X", "Y"]),
        BlockKind::SetValue { .. } => ("Set Value", &["Widget", "Value"]),
        BlockKind::SetProperty { .. } => ("Set Property", &["Property", "Object", "Value"]),
        BlockKind::SetAttribute { .. } => ("Set Attribute", &["Attribute", "Object", "Value"]),
        BlockKind::PlaySound { .. } => ("Play Sound", &["Sound"]),
        BlockKind::PlaySoundAtPosition { .. } => ("Play Sound At Position", &["Sound", "Position"]),
        BlockKind::StopSound { .. } => ("Stop Sound", &["Sound"]),
        BlockKind::PauseSound { .. } => ("Pause Sound", &["Sound"]),
        BlockKind::ResumeSound { .. } => ("Resume Sound", &["Sound"]),
        BlockKind::SetVolume { .. } => ("Set Volume", &["Sound", "Volume"]),
        BlockKind::FadeVolume { .. } => ("Fade Volume", &["Sound", "Volume", "Duration"]),
        BlockKind::SetSoundPitch { .. } => ("Set Sound Pitch", &["Sound", "Pitch"]),
        BlockKind::SetSoundLooping { .. } => ("Set Sound Looping", &["Sound", "Looped"]),
        BlockKind::SetSoundTime { .. } => ("Set Sound Time", &["Sound", "Time (s)"]),
        BlockKind::SetAnimatorBool { .. } => ("Set Bool", &["Parameter", "Animator", "Value"]),
        BlockKind::SetAnimatorNumber { .. } => ("Set Number", &["Parameter", "Animator", "Value"]),
        BlockKind::SetAnimatorLayerWeight { .. } => (
            "Set Layer Weight",
            &["Layer", "Animator", "Weight", "Seconds"],
        ),
        BlockKind::SetAnimatorTrigger { .. } => ("Set Trigger", &["Trigger", "Animator"]),
        BlockKind::ResetAnimatorTrigger { .. } => ("Reset Trigger", &["Trigger", "Animator"]),
        BlockKind::PlayAnimatorState { .. } => ("Play State", &["State", "Animator"]),
        BlockKind::PlayAnimation { .. } => ("Play Animation", &["Animation", "Animator"]),
        BlockKind::StopAnimation { .. } => ("Stop Animation", &["Animator"]),
        BlockKind::SetAnimatorSpeed { .. } => ("Set Animator Speed", &["Animator", "Speed"]),
        BlockKind::SetIkTarget { .. } => ("Set IK Target", &["IK Control", "Target"]),
        BlockKind::SetRagdollEnabled { .. } => ("Set Ragdoll Enabled", &["Ragdoll", "Enabled"]),
        BlockKind::PlayCutscene { .. } => ("Play Cutscene", &["Sequence"]),
        BlockKind::PauseCutscene { .. } => ("Pause Cutscene", &["Sequence"]),
        BlockKind::ResumeCutscene { .. } => ("Resume Cutscene", &["Sequence"]),
        BlockKind::StopCutscene { .. } => ("Stop Cutscene", &["Sequence"]),
        BlockKind::Patrol { .. } => ("Patrol", &["Agent", "Spline"]),
        BlockKind::PlaceOnSpline { .. } => ("Put On Spline", &["Object", "Spline", "Distance"]),
        BlockKind::Tween { .. } => (
            "Tween",
            &[
                "Property",
                "Object",
                "Value",
                "Seconds",
                "Style",
                "Direction",
            ],
        ),
        BlockKind::SlideIn { .. } => ("Slide In", &["Object", "From", "Seconds"]),
        BlockKind::SlideOut { .. } => ("Slide Out", &["Object", "To", "Seconds"]),
        BlockKind::FadeIn { .. } => ("Fade In", &["Object", "Seconds"]),
        BlockKind::FadeOut { .. } => ("Fade Out", &["Object", "Seconds"]),
        BlockKind::TweenPivot { .. } => (
            "Tween Model / Pivot",
            &["Object", "Position", "Duration", "Easing", "Direction"],
        ),
        BlockKind::PulseScale { .. } => ("Pulse / Pop Scale", &["Object", "Scale", "Duration"]),
        BlockKind::CancelTween { .. } => ("Cancel Tween", &["Tween"]),
        BlockKind::SetVariable { .. } => ("Set Variable", &["Variable", "Value"]),
        BlockKind::ChangeVariable { .. } => ("Change Variable", &["Variable", "By"]),
        BlockKind::ListCreate { .. } => ("Create List", &[]),
        BlockKind::ListAdd { .. } => ("Add to List", &["Value", "List"]),
        BlockKind::ListInsert { .. } => ("Insert in List", &["Value", "Index", "List"]),
        BlockKind::ListRemoveIndex { .. } => ("Remove Index", &["Index", "List"]),
        BlockKind::ListRemoveValue { .. } => ("Remove Value", &["Value", "List"]),
        BlockKind::ListClear { .. } => ("Clear List", &["List"]),
        BlockKind::Print { .. } => ("Print", &["Value"]),
        BlockKind::Destroy { .. } => ("Destroy", &["Object"]),
        BlockKind::SaveSlot { .. } => ("Save Slot", &["Slot"]),
        BlockKind::LoadSlot { .. } => ("Load Slot", &["Slot"]),
        BlockKind::DeleteSlot { .. } => ("Delete Slot", &["Slot"]),
        BlockKind::ForEachRow { .. } => ("For Each Row", &["Table"]),
        BlockKind::SetCheckpoint { .. } => ("Set Checkpoint", &["Checkpoint"]),
        BlockKind::RestoreCheckpoint => ("Restore Checkpoint", &[]),
        BlockKind::SetToneMapping { .. } => ("Set Tone Mapping", &["Tone Mapping"]),
        BlockKind::SetClockTime { .. } => ("Set Time of Day", &["Hours"]),
        BlockKind::SetAtmosphere { .. } => ("Set Atmosphere", &["Property", "Value"]),
        BlockKind::SetLanguage { .. } => ("Set Language", &["Language"]),
        BlockKind::LoadLevel { .. } => ("Load Level", &["Level", "Spawn"]),
        BlockKind::AddLevel { .. } => ("Add Level", &["Level"]),
        BlockKind::RemoveLevel { .. } => ("Remove Level", &["Level"]),
        BlockKind::HostGame { .. } => ("Host Game", &["Port"]),
        BlockKind::JoinGame { .. } => ("Join Game", &["Address"]),
        BlockKind::LeaveGame => ("Leave Game", &[]),
        BlockKind::SetNetworkOwner { .. } => ("Set Network Owner", &["Owner", "Part"]),
        BlockKind::SetNetworkOwnershipAuto { .. } => ("Network Ownership Auto", &["Part"]),
        BlockKind::KickPlayer { .. } => ("Kick Player", &["Player", "Reason"]),
        BlockKind::RequestStreamAround { .. } => ("Load Around Player", &["Player", "Position"]),
        BlockKind::SendChatMessage { .. } => ("Send Chat Message", &["Text"]),
        BlockKind::ShowSystemMessage { .. } => ("Show System Message", &["Text"]),
        BlockKind::SetLayerActive { .. } => ("Set Layer Active", &["Layer", "Active"]),
        BlockKind::WhenConnectionFails { .. } => ("When Connection Fails", &[]),
        BlockKind::WhenChatMessageReceived { .. } => ("When Chat Message Received", &[]),
        BlockKind::SetGraphicsQuality { .. } => ("Set Graphics Quality", &["Quality"]),
        BlockKind::SetGraphicsOption { .. } => ("Set Graphics Option", &["Setting", "Choice"]),
        BlockKind::SetGraphicsNumber { .. } => ("Set Graphics Number", &["Setting", "Value"]),
        BlockKind::Code { .. } => ("Lua", &["Code"]),
        // Their inputs vary: see `block_pin_names`.
        BlockKind::RunScript { .. } => ("Script Runs", &[]),
        BlockKind::OnSignal { .. } => ("On Signal", &["Signal", "Method", "Parameters"]),
        BlockKind::Connect { .. } => ("Connect", &["Signal", "Method", "Parameters"]),
        BlockKind::Comment { .. } => ("Comment", &["Text"]),
        BlockKind::Local { .. } => ("Local Variable", &[]),
        BlockKind::Assign { .. } => ("Set", &[]),
        BlockKind::CompoundAssign { .. } => ("Change", &["Target", "Operator", "Value"]),
        BlockKind::CallFunction { .. } => ("Call", &[]),
        BlockKind::CallMethod { .. } => ("Call Method", &[]),
        BlockKind::DefineFunction { .. } => ("Function", &["Name", "Parameters"]),
        BlockKind::While { .. } => ("While", &["Condition"]),
        BlockKind::RepeatLoop { .. } => ("Repeat Until", &["Condition"]),
        BlockKind::NumericFor { .. } => ("For", &[]),
        BlockKind::GenericFor { .. } => ("For Each", &[]),
        BlockKind::DoBlock { .. } => ("Do", &[]),
        BlockKind::Break => ("Break Loop", &[]),
        BlockKind::Continue => ("Continue", &[]),
        BlockKind::Return { .. } => ("Return", &[]),
    }
}

const ARGUMENTS: [&str; 12] = [
    "Arg 1", "Arg 2", "Arg 3", "Arg 4", "Arg 5", "Arg 6", "Arg 7", "Arg 8", "Arg 9", "Arg 10",
    "Arg 11", "Arg 12",
];

/// `count` names for arguments.
fn arguments(count: usize) -> impl Iterator<Item = &'static str> {
    (0..count).map(|index| ARGUMENTS.get(index).copied().unwrap_or("Arg"))
}

/// The names of a block's menus and inputs, in the order its parts show them.
pub(super) fn block_pin_names(kind: &BlockKind) -> Vec<&'static str> {
    match kind {
        BlockKind::WhenTouched {
            filter_value,
            cooldown,
            ..
        } => {
            let mut names = vec!["Part", "Filter"];
            if filter_value.is_some() {
                names.push("Tag / Name");
            }
            if cooldown.is_some() {
                names.push("Cooldown (s)");
            }
            names
        }
        BlockKind::WhenTouchEnds { filter_value, .. } => {
            let mut names = vec!["Part", "Filter"];
            if filter_value.is_some() {
                names.push("Tag / Name");
            }
            names
        }
        BlockKind::WhenPromptTriggered { cooldown, .. } => {
            let mut names = vec!["Prompt"];
            if cooldown.is_some() {
                names.push("Cooldown (s)");
            }
            names
        }
        BlockKind::WhenPartClicked { cooldown, .. } => {
            let mut names = vec!["Click Detector"];
            if cooldown.is_some() {
                names.push("Cooldown (s)");
            }
            names
        }
        BlockKind::WhenCommandRun { admin_only, .. } => {
            let mut names = vec!["Command", "Help"];
            if admin_only.is_some() {
                names.push("Admin Only");
            }
            names
        }
        BlockKind::FireEvent { data, .. } => {
            let mut names = vec!["Event"];
            if data.is_some() {
                names.push("Data");
            }
            names
        }
        BlockKind::WaitForChild { timeout, .. } => {
            let mut names = vec!["Name", "Object"];
            if timeout.is_some() {
                names.push("Timeout (s)");
            }
            names
        }
        BlockKind::ForEachPartInRadius { ignore, .. } => {
            let mut names = vec!["Variable", "Radius", "Center"];
            if ignore.is_some() {
                names.push("Ignore");
            }
            names
        }
        BlockKind::ForEachPartTouching { ignore, .. } => {
            let mut names = vec!["Variable", "Part"];
            if ignore.is_some() {
                names.push("Ignore");
            }
            names
        }
        BlockKind::SetClockTime { duration, .. } => {
            let mut names = vec!["Hours"];
            if duration.is_some() {
                names.push("Duration (s)");
            }
            names
        }
        BlockKind::SetAtmosphere { duration, .. } => {
            let mut names = vec!["Property", "Value"];
            if duration.is_some() {
                names.push("Duration (s)");
            }
            names
        }
        BlockKind::SetGraphicsOption { setting, .. } => {
            if axcenity_blocks::GRAPHICS_NUMBERS.contains(&setting.as_str()) {
                vec!["Setting", "Value"]
            } else {
                vec!["Setting", "Choice"]
            }
        }
        BlockKind::PlaySound { pitch, .. } => {
            let mut names = vec!["Sound"];
            if pitch.is_some() {
                names.push("Pitch");
            }
            names
        }
        BlockKind::PlaySoundAtPosition { volume, pitch, .. } => {
            let mut names = vec!["Sound", "Position"];
            if volume.is_some() {
                names.push("Volume");
            }
            if pitch.is_some() {
                names.push("Pitch");
            }
            names
        }
        BlockKind::PlayAnimation {
            fade_time,
            speed,
            weight,
            ..
        } => {
            let mut names = vec!["Animation", "Animator"];
            if fade_time.is_some() {
                names.push("Fade (s)");
            }
            if speed.is_some() {
                names.push("Speed");
            }
            if weight.is_some() {
                names.push("Weight");
            }
            names
        }
        BlockKind::StopAnimation {
            animation,
            fade_time,
            ..
        } => {
            let mut names = Vec::new();
            if animation.is_some() {
                names.push("Animation");
            }
            names.push("Animator");
            if fade_time.is_some() {
                names.push("Fade (s)");
            }
            names
        }
        BlockKind::SetIkTarget { weight, .. } => {
            let mut names = vec!["IK Control", "Target"];
            if weight.is_some() {
                names.push("Weight");
            }
            names
        }
        BlockKind::Tween { repeat_count, .. } => {
            let mut names = vec![
                "Property",
                "Object",
                "Value",
                "Seconds",
                "Style",
                "Direction",
            ];
            if repeat_count.is_some() {
                names.push("Repeat");
            }
            names
        }
        BlockKind::Local { names, values } => {
            let label = if names.contains(',') { "Names" } else { "Name" };
            std::iter::once(label)
                .chain(values.iter().map(|_| "Value"))
                .collect()
        }
        BlockKind::ListCreate { items, .. } => std::iter::once("List")
            .chain(items.iter().map(|_| "Item"))
            .collect(),
        BlockKind::CloneInto {
            count,
            position,
            offset_per_clone,
            per_clone_positions,
            ..
        } => {
            let mut names = vec!["Object", "Into"];
            if count.is_some() {
                names.push("Count");
            }
            if position.is_some() {
                names.push("Position");
            }
            if offset_per_clone.is_some() {
                names.push("Offset");
            }
            if per_clone_positions.is_some() {
                names.push("Positions");
            }
            names
        }
        BlockKind::Assign { targets, values } => targets
            .iter()
            .map(|_| "Target")
            .chain(values.iter().map(|_| "Value"))
            .collect(),
        BlockKind::CallFunction { args, .. } => std::iter::once("Function")
            .chain(arguments(args.len()))
            .collect(),
        BlockKind::CallMethod { args, .. } => ["Object", "Method"]
            .into_iter()
            .chain(arguments(args.len()))
            .collect(),
        BlockKind::NumericFor { step, .. } => {
            let mut names = vec!["Variable", "Start", "Stop"];
            if step.is_some() {
                names.push("Step");
            }
            names
        }
        BlockKind::GenericFor { values, .. } => std::iter::once("Names")
            .chain(values.iter().map(|_| "Values"))
            .collect(),
        BlockKind::Return { values } => values.iter().map(|_| "Value").collect(),
        kind => block_names(kind).1.to_vec(),
    }
}

/// A block's node title: a method call is named after its method.
pub(super) fn block_title(kind: &BlockKind) -> String {
    match kind {
        BlockKind::CallMethod { method, .. } => method.clone(),
        BlockKind::CallFunction {
            function: Expression::Name { name },
            ..
        } => name.clone(),
        BlockKind::Local { names, .. } => {
            let trimmed = names.trim();
            if trimmed.is_empty() {
                "Local Variable".to_owned()
            } else {
                format!("Local: {trimmed}")
            }
        }
        BlockKind::DefineFunction { name, .. } => format!("Function {name}"),
        kind => block_names(kind).0.to_owned(),
    }
}

/// A value node's title, and the names of its menus and inputs.
pub(super) fn value_names(expression: &Expression) -> (String, &'static [&'static str]) {
    let (title, names): (&str, &'static [&'static str]) = match expression {
        // A variable's node is named after it, as in Unreal's getters.
        Expression::Variable { name } => return (format!("Get {name}"), &[]),
        Expression::BlockOutput { name, .. } => return (name.clone(), &[]),
        Expression::ObjectPath { path } => {
            let label = if path.is_empty() {
                "Workspace"
            } else {
                path.as_str()
            };
            return (label.to_owned(), &[]);
        }
        Expression::ListItem { .. } => ("Item of List", &["Index", "List"]),
        Expression::ListLength { .. } => ("Length of List", &["List"]),
        Expression::ListContains { .. } => ("List Contains", &["List", "Value"]),
        Expression::Math { .. } => ("Math", &["A", "Operator", "B"]),
        Expression::Function { .. } => ("Math Function", &["Function", "Value"]),
        Expression::Random { .. } => ("Random Integer", &["Min", "Max"]),
        Expression::RandomNumber { .. } => ("Random Decimal", &["Min", "Max"]),
        Expression::Lerp { .. } => ("Lerp", &["From", "To", "Alpha"]),
        Expression::MakeVector3 { .. } => ("Vector (X, Y, Z)", &["X", "Y", "Z"]),
        Expression::VectorAxis { .. } => ("Vector Axis", &["Axis", "Vector"]),
        Expression::Distance { .. } => ("Distance Between", &["From", "To"]),
        Expression::Direction { .. } => ("Direction A to B", &["From", "To"]),
        Expression::Magnitude { .. } => ("Magnitude", &["Vector"]),
        Expression::Clamp { .. } => ("Clamp", &["Value", "Min", "Max"]),
        Expression::MinMax { .. } => ("Min / Max", &["Op", "A", "B"]),
        Expression::StringLength { .. } => ("Length of Text", &["Text"]),
        Expression::StringContains { .. } => ("Text Contains", &["Text", "Substring"]),
        Expression::FormatNumber { .. } => ("Format Number", &["Value", "Decimals"]),
        Expression::MakeColorRgb { .. } => ("RGB Color", &["R", "G", "B"]),
        Expression::MakeColorHsv { .. } => ("HSV Color", &["H", "S", "V"]),
        Expression::AnimatorNewState => ("New State", &[]),
        Expression::AnimatorPreviousState => ("Previous State", &[]),
        Expression::AnimationEventName => ("Event Name", &[]),
        Expression::ConnectionFailureReason => ("Failure Reason", &[]),
        Expression::ChangedLayerName => ("Layer Name", &[]),
        Expression::ChangedLayerActive => ("Layer Is Active", &[]),
        Expression::ChatMessageText => ("Chat Message", &[]),
        Expression::ChatMessageSender => ("Chat Sender", &[]),
        Expression::IsHost => ("Is Host", &[]),
        Expression::IsConnected => ("Is Connected", &[]),
        Expression::SlotExists { .. } => ("Slot Exists", &["Slot"]),
        Expression::DataTableRowCount { .. } => ("Row Count", &["Table"]),
        Expression::Join { .. } => ("Join Text", &["A", "B"]),
        Expression::Compare { .. } => ("Compare", &["A", "Operator", "B"]),
        Expression::Logic { .. } => ("And / Or", &["A", "Operator", "B"]),
        Expression::Not { .. } => ("Not", &["Value"]),
        Expression::Property { .. } => ("Get Property", &["Property", "Object"]),
        Expression::Attribute { .. } => ("Get Attribute", &["Attribute", "Object"]),
        Expression::SoundProperty { .. } => ("Sound Property", &["Property", "Sound"]),
        Expression::Raycast { ignore, .. } => {
            if ignore.is_some() {
                ("Raycast", &["Output", "Origin", "Direction", "Ignore"])
            } else {
                ("Raycast", &["Output", "Origin", "Direction"])
            }
        }
        Expression::RaycastIgnoring { .. } => {
            ("Raycast Ignoring", &["Origin", "Direction", "Ignore"])
        }
        Expression::TouchingPart => ("Touching Part", &[]),
        Expression::TriggeringPlayer => ("Triggering Player", &[]),
        Expression::CommandArgs => ("Command Args", &[]),
        Expression::DeltaTime => ("Delta Time", &[]),
        Expression::PartUnderMouse => ("Part Under Mouse", &[]),
        Expression::MouseHitPosition => ("Mouse Hit Position", &[]),
        Expression::CameraPosition => ("Camera Position", &[]),
        Expression::CameraLookDirection => ("Camera Look Direction", &[]),
        Expression::NearestPlayerTo { .. } => ("Nearest Player To", &["Position"]),
        Expression::PlayerOfPart { .. } => ("Player of Part", &["Part"]),
        Expression::PlayerIsAlive { .. } => ("Player Is Alive", &["Player"]),
        Expression::ParentOf { .. } => ("Parent Of", &["Object"]),
        Expression::ChildOf { .. } => ("Find Child", &["Name", "Object"]),
        Expression::HasTag { .. } => ("Has Tag", &["Object", "Tag"]),
        Expression::GameTime => ("Game Time", &[]),
        Expression::KeyDown { .. } => ("Is Key Down", &["Key"]),
        Expression::LayerActive { .. } => ("Is Layer Active", &["Layer"]),
        Expression::IsA { .. } => ("Is A", &["Object", "Class"]),
        Expression::IsStreamedIn { .. } => ("Is Streamed In", &["Object"]),
        Expression::AnimatorBool { .. } => ("Get Bool", &["Parameter", "Animator"]),
        Expression::AnimatorNumber { .. } => ("Get Number", &["Parameter", "Animator"]),
        Expression::AnimatorLayerWeight { .. } => ("Layer Weight", &["Layer", "Animator"]),
        Expression::DataRow { .. } => ("Get Row", &["Key", "Table"]),
        Expression::SplinePosition { .. } => ("Point Along Spline", &["T", "Spline"]),
        Expression::SplineClosestDistance { .. } => {
            ("Distance Along Spline", &["Spline", "Position"])
        }
        Expression::DataValue { .. } => ("Get Column", &["Column", "Row"]),
        Expression::Code { .. } => ("Lua", &["Code"]),
        Expression::Number { .. } => ("Number", &[]),
        Expression::Text { .. } => ("Text", &[]),
        Expression::Boolean { .. } => ("True or False", &[]),
        Expression::Vector3 { .. } => ("Vector3", &[]),
        Expression::Color3 { .. } => ("Color", &[]),
        Expression::Nil => ("Nil", &[]),
        Expression::Vararg => ("Arguments", &[]),
        Expression::Name { name } => return (format!("Get {name}"), &[]),
        Expression::Closure { .. } => ("Function", &[]),
        Expression::Field { name, .. } => return (format!("Get {name}"), &["Object", "Name"]),
        Expression::Index { .. } => ("Look Up", &["Table", "Key"]),
        Expression::Call {
            function: boxed, ..
        } => match &**boxed {
            Expression::Name { name } => return (name.clone(), &[]),
            _ => ("Call", &[]),
        },
        Expression::MethodCall { method, .. } => return (method.clone(), &[]),
        Expression::Table { .. } => ("Make Table", &[]),
        Expression::Binary { op, .. } => return (format!("A {op} B"), &["A", "Operator", "B"]),
        Expression::Unary { op, .. } => return (format!("{op} Value"), &["Operator", "Value"]),
        Expression::Paren { .. } => ("First Value", &["Value"]),
        Expression::Interpolated { .. } => ("Format Text", &[]),
        Expression::IfValue { .. } => ("If Value", &[]),
        Expression::Cast { .. } => ("Cast", &["Value", "Type"]),
    };
    (title.to_owned(), names)
}

/// The names of a value node's menus and inputs.
pub(super) fn value_pin_names(expression: &Expression) -> Vec<&'static str> {
    match expression {
        Expression::Call { args, .. } => std::iter::once("Function")
            .chain(arguments(args.len()))
            .collect(),
        Expression::MethodCall { args, .. } => ["Object", "Method"]
            .into_iter()
            .chain(arguments(args.len()))
            .collect(),
        Expression::Table { fields, .. } => fields
            .iter()
            .flat_map(|field| match field {
                TableField::Named { .. } => vec!["Name", "Value"],
                TableField::Keyed { .. } => vec!["Key", "Value"],
                TableField::Item { .. } => vec!["Item"],
            })
            .collect(),
        Expression::Interpolated { parts, values } => {
            let mut names = Vec::new();
            for index in 0..parts.len() {
                names.push("Text");
                if index < values.len() {
                    names.push("Value");
                }
            }
            names
        }
        Expression::IfValue { parts } => {
            let count = parts.len();
            (0..count)
                .map(|index| {
                    if index + 1 == count {
                        "Else"
                    } else if index % 2 == 0 {
                        "Condition"
                    } else {
                        "Then"
                    }
                })
                .collect()
        }
        expression => value_names(expression).1.to_vec(),
    }
}

/// The kind of value a value node gives, for its output pin's colour.
pub(super) fn value_type(expression: &Expression) -> SlotKind {
    match expression {
        _ if expression.shape() == BlockShape::Boolean
            && !expression.is_untyped()
            && !matches!(expression, Expression::Code { .. }) =>
        {
            SlotKind::Truth
        }
        Expression::Number { .. }
        | Expression::Math { .. }
        | Expression::Function { .. }
        | Expression::Random { .. }
        | Expression::RandomNumber { .. }
        | Expression::VectorAxis { .. }
        | Expression::Distance { .. }
        | Expression::Magnitude { .. }
        | Expression::Clamp { .. }
        | Expression::MinMax { .. }
        | Expression::StringLength { .. }
        | Expression::ListLength { .. }
        | Expression::DeltaTime
        | Expression::GameTime
        | Expression::AnimatorNumber { .. }
        | Expression::AnimatorLayerWeight { .. }
        | Expression::DataTableRowCount { .. } => SlotKind::Number,
        Expression::ObjectPath { .. }
        | Expression::ParentOf { .. }
        | Expression::ChildOf { .. }
        | Expression::ChatMessageSender => SlotKind::Object(Filter::Any),
        Expression::Text { .. }
        | Expression::Join { .. }
        | Expression::FormatNumber { .. }
        | Expression::AnimatorNewState
        | Expression::AnimatorPreviousState
        | Expression::AnimationEventName
        | Expression::ConnectionFailureReason
        | Expression::ChangedLayerName
        | Expression::ChatMessageText
        | Expression::Interpolated { .. } => SlotKind::Text,
        Expression::Vector3 { .. }
        | Expression::MakeVector3 { .. }
        | Expression::Direction { .. }
        | Expression::MouseHitPosition
        | Expression::CameraPosition
        | Expression::CameraLookDirection => SlotKind::Vector,
        Expression::PlayerIsAlive { .. } | Expression::HasTag { .. } => SlotKind::Truth,
        Expression::Color3 { .. }
        | Expression::MakeColorRgb { .. }
        | Expression::MakeColorHsv { .. } => SlotKind::Color,
        _ => SlotKind::Any,
    }
}

/// Unreal-like pin colours: green numbers, pink text, red true or false,
/// gold vectors, blue colours, grey for anything.
pub(super) fn pin_color(kind: SlotKind) -> Color32 {
    match kind {
        SlotKind::Number => Color32::from_rgb(140, 220, 70),
        SlotKind::Text => Color32::from_rgb(240, 90, 210),
        SlotKind::Condition | SlotKind::Truth => Color32::from_rgb(220, 50, 50),
        SlotKind::Vector => Color32::from_rgb(245, 195, 40),
        SlotKind::Color => Color32::from_rgb(70, 140, 255),
        SlotKind::Object(_) => Color32::from_rgb(65, 195, 220),
        SlotKind::Any => Color32::from_rgb(170, 170, 178),
    }
}

/// A right-side data output pin on a block node (e.g. `Cloned Object` on
/// `Clone`, or `Value` on `Local`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BlockValueOut {
    pub(super) label: String,
    pub(super) kind: SlotKind,
    pub(super) lua_name: String,
}

/// The right-side data output pins a block node exposes.
pub(super) fn block_value_outs(
    kind: &BlockKind,
    block_id: u64,
    clone_ids: &HashMap<u64, (String, String)>,
) -> Vec<BlockValueOut> {
    match kind {
        BlockKind::Local { names, .. } => names
            .split(',')
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|raw| {
                // Strip optional Luau type annotation (`x: number` -> `x`).
                let ident = raw.split(':').next().unwrap_or(raw).trim();
                BlockValueOut {
                    label: ident.to_owned(),
                    kind: SlotKind::Any,
                    lua_name: ident.to_owned(),
                }
            })
            .collect(),
        BlockKind::CloneInto {
            target,
            count,
            offset_per_clone,
            per_clone_positions,
            ..
        } => {
            let (single, array) = clone_ids.get(&block_id).cloned().unwrap_or_else(|| {
                let base = axcenity_blocks::clone_base_name(target);
                (format!("{base}_cloned"), format!("{base}_clones"))
            });
            let mut outs = vec![BlockValueOut {
                label: single.clone(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: single,
            }];
            let is_batch = per_clone_positions.is_some()
                || offset_per_clone.is_some()
                || matches!(count, Some(expr) if !matches!(expr, Expression::Number { value } if (*value - 1.0).abs() < f64::EPSILON));
            if is_batch {
                outs.push(BlockValueOut {
                    label: array.clone(),
                    kind: SlotKind::Any,
                    lua_name: array,
                });
            }
            outs
        }
        BlockKind::ListCreate { name, .. } if !name.trim().is_empty() => vec![BlockValueOut {
            label: name.clone(),
            kind: SlotKind::Any,
            lua_name: name.clone(),
        }],
        BlockKind::SpawnDecal { .. } => vec![BlockValueOut {
            label: "Decal".to_owned(),
            kind: SlotKind::Object(Filter::Any),
            lua_name: "decal".to_owned(),
        }],
        BlockKind::TweenProperty { .. }
        | BlockKind::Tween { .. }
        | BlockKind::SlideIn { .. }
        | BlockKind::SlideOut { .. }
        | BlockKind::FadeIn { .. }
        | BlockKind::FadeOut { .. }
        | BlockKind::TweenPivot { .. }
        | BlockKind::PulseScale { .. } => vec![BlockValueOut {
            label: "Tween".to_owned(),
            kind: SlotKind::Object(Filter::Any),
            lua_name: "tween".to_owned(),
        }],
        BlockKind::PlayAnimation { .. } => vec![BlockValueOut {
            label: "Track".to_owned(),
            kind: SlotKind::Object(Filter::Any),
            lua_name: "track".to_owned(),
        }],
        BlockKind::WhenStateChanged { .. } => vec![
            BlockValueOut {
                label: "New State".to_owned(),
                kind: SlotKind::Text,
                lua_name: "state".to_owned(),
            },
            BlockValueOut {
                label: "Previous State".to_owned(),
                kind: SlotKind::Text,
                lua_name: "previous".to_owned(),
            },
        ],
        BlockKind::WhenAnimationEvent { .. } => vec![BlockValueOut {
            label: "Event Name".to_owned(),
            kind: SlotKind::Text,
            lua_name: "name".to_owned(),
        }],
        // Events (§3.1) output pins.
        BlockKind::WhenTouched { .. } | BlockKind::WhenTouchEnds { .. } => vec![
            BlockValueOut {
                label: "Other Part".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: "otherPart".to_owned(),
            },
            BlockValueOut {
                label: "Player".to_owned(),
                kind: SlotKind::Any,
                lua_name: "player".to_owned(),
            },
        ],
        BlockKind::WhenSeatOccupied { .. } | BlockKind::WhenSeatVacated { .. } => vec![
            BlockValueOut {
                label: "Occupant".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: "occupant".to_owned(),
            },
            BlockValueOut {
                label: "Player".to_owned(),
                kind: SlotKind::Any,
                lua_name: "player".to_owned(),
            },
        ],
        BlockKind::WhenAgentSenses { .. } | BlockKind::WhenAgentLosesPlayer { .. } => vec![
            BlockValueOut {
                label: "Player".to_owned(),
                kind: SlotKind::Any,
                lua_name: "player".to_owned(),
            },
            BlockValueOut {
                label: "Character".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: "character".to_owned(),
            },
        ],
        BlockKind::WhenSplineReached { .. } => vec![BlockValueOut {
            label: "Which".to_owned(),
            kind: SlotKind::Text,
            lua_name: "which".to_owned(),
        }],
        BlockKind::WhenPromptTriggered { .. }
        | BlockKind::WhenPartClicked { .. }
        | BlockKind::WhenPlayerJoins { .. }
        | BlockKind::WhenPlayerLeaves { .. } => vec![BlockValueOut {
            label: "Player".to_owned(),
            kind: SlotKind::Any,
            lua_name: "player".to_owned(),
        }],
        BlockKind::WhenCharacterSpawns { .. } => vec![
            BlockValueOut {
                label: "Character".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: "character".to_owned(),
            },
            BlockValueOut {
                label: "Player".to_owned(),
                kind: SlotKind::Any,
                lua_name: "player".to_owned(),
            },
        ],
        BlockKind::WhenPlayerDies { .. } => vec![
            BlockValueOut {
                label: "Player".to_owned(),
                kind: SlotKind::Any,
                lua_name: "player".to_owned(),
            },
            BlockValueOut {
                label: "Character".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: "character".to_owned(),
            },
        ],
        BlockKind::WhenKeyPressed { .. } | BlockKind::WhenKeyReleased { .. } => vec![
            BlockValueOut {
                label: "KeyCode".to_owned(),
                kind: SlotKind::Any,
                lua_name: "keyCode".to_owned(),
            },
            BlockValueOut {
                label: "GameProcessed".to_owned(),
                kind: SlotKind::Truth,
                lua_name: "gameProcessed".to_owned(),
            },
        ],
        BlockKind::WhenChildAdded { .. } | BlockKind::WhenChildRemoved { .. } => {
            vec![BlockValueOut {
                label: "Child".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: "child".to_owned(),
            }]
        }
        BlockKind::WhenTagAdded { .. } | BlockKind::WhenTagRemoved { .. } => vec![BlockValueOut {
            label: "Instance".to_owned(),
            kind: SlotKind::Object(Filter::Any),
            lua_name: "instance".to_owned(),
        }],
        BlockKind::WhenRemoteFires { side, .. } => {
            if side.starts_with("Client") {
                vec![BlockValueOut {
                    label: "Data".to_owned(),
                    kind: SlotKind::Any,
                    lua_name: "data".to_owned(),
                }]
            } else {
                vec![
                    BlockValueOut {
                        label: "Player".to_owned(),
                        kind: SlotKind::Any,
                        lua_name: "player".to_owned(),
                    },
                    BlockValueOut {
                        label: "Data".to_owned(),
                        kind: SlotKind::Any,
                        lua_name: "data".to_owned(),
                    },
                ]
            }
        }
        BlockKind::WhenValueChanged { .. }
        | BlockKind::WhenPropertyChanged { .. }
        | BlockKind::WhenAttributeChanged { .. } => vec![BlockValueOut {
            label: "New Value".to_owned(),
            kind: SlotKind::Any,
            lua_name: "newValue".to_owned(),
        }],
        BlockKind::EveryFrame { .. } | BlockKind::BeforeRender { .. } => vec![BlockValueOut {
            label: "Delta Time".to_owned(),
            kind: SlotKind::Number,
            lua_name: "deltaTime".to_owned(),
        }],
        BlockKind::BeforePhysics { .. } => vec![
            BlockValueOut {
                label: "Time".to_owned(),
                kind: SlotKind::Number,
                lua_name: "time".to_owned(),
            },
            BlockValueOut {
                label: "Delta Time".to_owned(),
                kind: SlotKind::Number,
                lua_name: "deltaTime".to_owned(),
            },
        ],
        BlockKind::WhenCommandRun { .. } => vec![
            BlockValueOut {
                label: "Player".to_owned(),
                kind: SlotKind::Any,
                lua_name: "player".to_owned(),
            },
            BlockValueOut {
                label: "Args".to_owned(),
                kind: SlotKind::Any,
                lua_name: "args".to_owned(),
            },
        ],
        BlockKind::WhenEventFired { .. } => vec![BlockValueOut {
            label: "Data".to_owned(),
            kind: SlotKind::Any,
            lua_name: "data".to_owned(),
        }],
        BlockKind::OnSignal { params, .. } | BlockKind::Connect { params, .. } => {
            let mut outs: Vec<BlockValueOut> = params
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(|raw| {
                    let ident = raw.split(':').next().unwrap_or(raw).trim();
                    BlockValueOut {
                        label: ident.to_owned(),
                        kind: SlotKind::Any,
                        lua_name: ident.to_owned(),
                    }
                })
                .collect();
            outs.push(BlockValueOut {
                label: "Connection".to_owned(),
                kind: SlotKind::Any,
                lua_name: format!("conn_{block_id}"),
            });
            outs
        }
        // Control Flow (§3.2) output pins.
        BlockKind::Wait { .. } => vec![BlockValueOut {
            label: "Actual Elapsed".to_owned(),
            kind: SlotKind::Number,
            lua_name: format!("elapsed_{block_id}"),
        }],
        BlockKind::WaitForChild { .. } => vec![BlockValueOut {
            label: "Child".to_owned(),
            kind: SlotKind::Object(Filter::Any),
            lua_name: format!("child_{block_id}"),
        }],
        BlockKind::WaitForEvent { event, .. } => {
            let label = match event.as_str() {
                "Touched" | "TouchEnded" => "Other Part",
                "ChildAdded" | "ChildRemoved" => "Child",
                "Triggered" | "MouseClick" => "Player",
                _ => "Result",
            };
            vec![BlockValueOut {
                label: label.to_owned(),
                kind: SlotKind::Any,
                lua_name: format!("event_{block_id}"),
            }]
        }
        BlockKind::Repeat { .. } | BlockKind::ForRange { .. } => vec![BlockValueOut {
            label: "Index".to_owned(),
            kind: SlotKind::Number,
            lua_name: format!("i_{block_id}"),
        }],
        BlockKind::ForEachChild { variable, .. } => vec![
            BlockValueOut {
                label: "Child".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: variable.clone(),
            },
            BlockValueOut {
                label: "Index".to_owned(),
                kind: SlotKind::Number,
                lua_name: format!("i_{block_id}"),
            },
        ],
        BlockKind::ForEachPartInRadius { variable, .. }
        | BlockKind::ForEachPartTouching { variable, .. } => vec![
            BlockValueOut {
                label: "Part".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: variable.clone(),
            },
            BlockValueOut {
                label: "Index".to_owned(),
                kind: SlotKind::Number,
                lua_name: format!("i_{block_id}"),
            },
        ],
        BlockKind::ForEachInList { .. } => vec![
            BlockValueOut {
                label: "Item".to_owned(),
                kind: SlotKind::Any,
                lua_name: format!("item_{block_id}"),
            },
            BlockValueOut {
                label: "Index".to_owned(),
                kind: SlotKind::Number,
                lua_name: format!("i_{block_id}"),
            },
        ],
        BlockKind::ForEachTagged { .. } => vec![
            BlockValueOut {
                label: "Instance".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: format!("tagged_{block_id}"),
            },
            BlockValueOut {
                label: "Index".to_owned(),
                kind: SlotKind::Number,
                lua_name: format!("i_{block_id}"),
            },
        ],
        BlockKind::ForEachRow { .. } => vec![
            BlockValueOut {
                label: "Row".to_owned(),
                kind: SlotKind::Any,
                lua_name: format!("row_{block_id}"),
            },
            BlockValueOut {
                label: "Index".to_owned(),
                kind: SlotKind::Number,
                lua_name: format!("i_{block_id}"),
            },
        ],
        BlockKind::WhenConnectionFails { .. } => vec![BlockValueOut {
            label: "Reason".to_owned(),
            kind: SlotKind::Text,
            lua_name: "reason".to_owned(),
        }],
        BlockKind::WhenLayerChanged { .. } => vec![
            BlockValueOut {
                label: "Layer Name".to_owned(),
                kind: SlotKind::Text,
                lua_name: "layerName".to_owned(),
            },
            BlockValueOut {
                label: "Is Active".to_owned(),
                kind: SlotKind::Truth,
                lua_name: "active".to_owned(),
            },
        ],
        BlockKind::WhenChatMessageReceived { .. } => vec![
            BlockValueOut {
                label: "Message".to_owned(),
                kind: SlotKind::Text,
                lua_name: "message.Text".to_owned(),
            },
            BlockValueOut {
                label: "Sender".to_owned(),
                kind: SlotKind::Object(Filter::Any),
                lua_name: "message.Player".to_owned(),
            },
        ],
        _ => Vec::new(),
    }
}

/// One row item on a node's left (inputs) or right (outputs) side.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Item {
    ExecIn,
    ExecOut(Out, &'static str),
    /// Value slot `slot` (in slot order), fed by a wire or typed in.
    Input {
        slot: usize,
        part: usize,
        kind: SlotKind,
        name: &'static str,
    },
    /// A menu on the node (an object, a key, an operator), as part `part`.
    Field {
        part: usize,
        name: &'static str,
    },
    /// Grey Lua, shown across the node.
    Code {
        part: usize,
    },
    Output(SlotKind),
    /// A right-side data output pin on a block node.
    BlockOutput {
        index: usize,
        kind: SlotKind,
        name: String,
    },
}

/// A node's rows: what's on the left and on the right of each.
#[derive(Clone, Debug, Default)]
pub(super) struct Layout {
    pub(super) left: Vec<Item>,
    pub(super) right: Vec<Item>,
}

impl Layout {
    pub(super) fn rows(&self) -> usize {
        self.left.len().max(self.right.len())
    }
}

/// The items for `parts` named by `names`, after the exec input if any.
fn items(parts: &[Part], names: &[&'static str], left: &mut Vec<Item>) {
    let mut named = names.iter().copied();
    let mut slot = 0;
    for (part, item) in parts.iter().enumerate() {
        match item {
            Part::Label(_) | Part::Icon(_) => {}
            Part::Slot(_, kind) => {
                left.push(Item::Input {
                    slot,
                    part,
                    kind: *kind,
                    name: named.next().unwrap_or(""),
                });
                slot += 1;
            }
            Part::Code(_) => {
                named.next();
                left.push(Item::Code { part });
            }
            // Its blocks are an exec output, so it isn't wired, but it
            // counts as the slot it's in.
            Part::Params(_) => {
                left.push(Item::Field {
                    part,
                    name: named.next().unwrap_or(""),
                });
                slot += 1;
            }
            _ => left.push(Item::Field {
                part,
                name: named.next().unwrap_or(""),
            }),
        }
    }
}

/// A block's rows, from its parts (`sections` with the mouths left out).
pub(super) fn block_layout(
    kind: &BlockKind,
    block_id: u64,
    parts: &[Part],
    clone_ids: &HashMap<u64, (String, String)>,
) -> Layout {
    let mut layout = Layout::default();
    if has_exec_in(kind) {
        layout.left.push(Item::ExecIn);
    }
    items(parts, &block_pin_names(kind), &mut layout.left);
    layout.right = exec_outs(kind)
        .into_iter()
        .map(|(out, name)| Item::ExecOut(out, name))
        .collect();
    for (index, out) in block_value_outs(kind, block_id, clone_ids)
        .into_iter()
        .enumerate()
    {
        layout.right.push(Item::BlockOutput {
            index,
            kind: out.kind,
            name: out.label,
        });
    }
    layout
}

/// A value node's rows, from its `reporter_parts`.
pub(super) fn value_layout(expression: &Expression, parts: &[Part]) -> Layout {
    let mut layout = Layout::default();
    items(parts, &value_pin_names(expression), &mut layout.left);
    layout.right.push(Item::Output(value_type(expression)));
    layout
}

/// A block's parts on its rows, with its mouths left out (they're pins).
pub(super) fn block_parts<'a>(kind: &'a mut BlockKind, objects: &[ObjectInfo]) -> Vec<Part<'a>> {
    sections(kind, objects)
        .into_iter()
        .flat_map(|section| match section {
            Section::Row(parts) => parts,
            Section::Mouth(_) => Vec::new(),
        })
        .collect()
}
