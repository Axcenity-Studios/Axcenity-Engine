use super::*;

/// A block or reporter in the palette.
#[derive(Clone, Debug)]
pub(super) enum PaletteItem {
    /// Boxed: a block is far bigger than a reporter.
    Block(Box<BlockKind>),
    Reporter(Expression),
    /// The "Make a Variable" button.
    NewVariable,
}

/// The palette's blocks for `category`. Object menus start on the first
/// object of their kind, so a new block usually works as it is.
pub(super) fn palette(
    category: Category,
    client: bool,
    objects: &[ObjectInfo],
    variables: &[String],
) -> Vec<PaletteItem> {
    let (mut items, luau) = palette_parts(category, client, objects, variables);
    items.extend(luau);
    items
}

/// [`palette`]'s blocks in two lists: the category's own, then the blocks
/// for Luau itself (`local`, `while`, a `:Connect` …), which the Blocks
/// palette folds away under "Lua blocks" until opened.
pub(super) fn palette_parts(
    category: Category,
    client: bool,
    objects: &[ObjectInfo],
    variables: &[String],
) -> (Vec<PaletteItem>, Vec<PaletteItem>) {
    let first = |filter: Filter| {
        ObjectRef::path(
            objects
                .iter()
                .find(|object| filter.matches(object.class))
                .map_or_else(
                    || filter.placeholder().to_owned(),
                    |object| object.path.clone(),
                ),
        )
    };
    let number = |value| Expression::Number { value };
    let name = |name: &str| Expression::Name {
        name: name.to_owned(),
    };
    let text = |value: &str| Expression::Text {
        value: value.to_owned(),
    };
    let vector = |x, y, z| Expression::Vector3 { x, y, z };
    let condition = || Expression::Boolean { value: false };
    let part = first(Filter::Parts);
    let sound = first(Filter::Sounds);
    // The first BindableEvent in ReplicatedStorage, for the event blocks.
    let bindable = objects
        .iter()
        .find(|object| object.path == STORAGE_PATH && object.class == ClassName::ReplicatedStorage)
        .and_then(|storage| {
            storage
                .names
                .iter()
                .find(|(kind, _)| *kind == NameKind::Bindable)
                .map(|(_, name)| name.clone())
        })
        .unwrap_or_else(|| NameKind::Bindable.example().to_owned());
    // The first data table (or its first column) the project has.
    let storage_name = |kind: NameKind| {
        objects
            .iter()
            .find(|object| object.path == STORAGE_PATH)
            .and_then(|storage| storage.names.iter().find(|(of, _)| *of == kind))
            .map_or_else(|| kind.example().to_owned(), |(_, name)| name.clone())
    };
    let variable = variables
        .first()
        .cloned()
        .unwrap_or_else(|| "score".to_owned());
    use PaletteItem::Reporter as R;
    let block = |kind| PaletteItem::Block(Box::new(kind));
    let mut items = match category {
        Category::Events => vec![
            block(BlockKind::WhenStarted { body: Vec::new() }),
            block(BlockKind::WhenTouched {
                part: part.clone(),
                filter: "Any Part".to_owned(),
                filter_value: None,
                cooldown: None,
                body: Vec::new(),
            }),
            block(BlockKind::WhenTouchEnds {
                part: part.clone(),
                filter: "Any Part".to_owned(),
                filter_value: None,
                body: Vec::new(),
            }),
            block(BlockKind::WhenPlayerJoins { body: Vec::new() }),
            block(BlockKind::WhenPlayerLeaves { body: Vec::new() }),
            block(BlockKind::WhenCharacterSpawns { body: Vec::new() }),
            block(BlockKind::WhenPlayerDies { body: Vec::new() }),
            block(BlockKind::WhenKeyPressed {
                key: "Space".to_owned(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenKeyReleased {
                key: "Space".to_owned(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenPromptTriggered {
                prompt: first(Filter::Prompts),
                cooldown: None,
                body: Vec::new(),
            }),
            block(BlockKind::WhenPartClicked {
                detector: first(Filter::ClickDetectors),
                cooldown: None,
                body: Vec::new(),
            }),
            block(BlockKind::WhenCommandRun {
                command: text("heal"),
                help: text("Heals the player"),
                admin_only: None,
                body: Vec::new(),
            }),
            block(BlockKind::WhenClicked {
                button: first(Filter::Buttons),
                body: Vec::new(),
            }),
            block(BlockKind::WhenMouseEnters {
                element: first(Filter::Gui),
                body: Vec::new(),
            }),
            block(BlockKind::WhenMouseLeaves {
                element: first(Filter::Gui),
                body: Vec::new(),
            }),
            block(BlockKind::WhenValueChanged {
                widget: first(Filter::Widgets),
                body: Vec::new(),
            }),
            block(BlockKind::WhenChildAdded {
                target: ObjectRef::default(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenChildRemoved {
                target: ObjectRef::default(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenTagAdded {
                tag: text("Hazard"),
                body: Vec::new(),
            }),
            block(BlockKind::WhenTagRemoved {
                tag: text("Hazard"),
                body: Vec::new(),
            }),
            block(BlockKind::WhenRemoteFires {
                remote: ObjectRef::default(),
                side: "Server (OnServerEvent)".to_owned(),
                body: Vec::new(),
            }),
            block(BlockKind::EveryFrame { body: Vec::new() }),
            block(BlockKind::BeforePhysics { body: Vec::new() }),
            block(BlockKind::BeforeRender { body: Vec::new() }),
            block(BlockKind::WhenEventFired {
                event: bindable.clone(),
                body: Vec::new(),
            }),
            block(BlockKind::FireEvent {
                event: bindable.clone(),
                data: None,
            }),
            block(BlockKind::Disconnect {
                connection: Expression::Name {
                    name: "connection".to_owned(),
                },
            }),
            block(BlockKind::WhenSoundEnded {
                sound: sound.clone(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenPropertyChanged {
                target: part.clone(),
                property: "Transparency".to_owned(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenAttributeChanged {
                target: part.clone(),
                attribute: NameKind::Attribute.example().to_owned(),
                body: Vec::new(),
            }),
        ],
        Category::Control => vec![
            block(BlockKind::Wait {
                seconds: number(1.0),
            }),
            block(BlockKind::WaitUntil {
                condition: condition(),
            }),
            block(BlockKind::Repeat {
                count: number(10.0),
                body: Vec::new(),
            }),
            block(BlockKind::RepeatUntil {
                condition: condition(),
                body: Vec::new(),
            }),
            block(BlockKind::Forever { body: Vec::new() }),
            block(BlockKind::ForRange {
                start: number(1.0),
                end: number(10.0),
                step: number(1.0),
                body: Vec::new(),
            }),
            block(BlockKind::ForEachInList {
                list: Expression::Variable {
                    name: variable.clone(),
                },
                body: Vec::new(),
            }),
            block(BlockKind::ForEachTagged {
                tag: text("Enemy"),
                body: Vec::new(),
            }),
            block(BlockKind::If {
                condition: condition(),
                body: Vec::new(),
            }),
            block(BlockKind::IfElse {
                condition: condition(),
                then_body: Vec::new(),
                else_body: Vec::new(),
            }),
            block(BlockKind::Sequence {
                branches: vec![Vec::new(), Vec::new()],
            }),
            block(BlockKind::CooldownGate {
                cooldown: number(1.0),
                body: Vec::new(),
            }),
            block(BlockKind::SpawnAsync {
                delay: number(0.0),
                body: Vec::new(),
            }),
            block(BlockKind::Break),
            block(BlockKind::DestroyScript),
            block(BlockKind::StopScript),
            block(BlockKind::ForEachChild {
                target: ObjectRef::default(),
                variable: "child".to_owned(),
                body: Vec::new(),
            }),
            block(BlockKind::ForEachPartInRadius {
                center: vector(0.0, 0.0, 0.0),
                radius: number(10.0),
                variable: "part".to_owned(),
                players_only: false,
                ignore: None,
                body: Vec::new(),
            }),
            block(BlockKind::ForEachPartTouching {
                target: part.clone(),
                variable: "part".to_owned(),
                players_only: false,
                ignore: None,
                body: Vec::new(),
            }),
            block(BlockKind::WaitForChild {
                target: ObjectRef::default(),
                name: NameKind::Child.example().to_owned(),
                timeout: None,
            }),
            block(BlockKind::WaitForEvent {
                target: part.clone(),
                event: NameKind::Signal.example().to_owned(),
            }),
            block(BlockKind::Destroy {
                target: part.clone(),
            }),
            block(BlockKind::DestroyAfter {
                target: part.clone(),
                seconds: number(2.0),
            }),
        ],
        Category::Motion => vec![
            block(BlockKind::MoveTo {
                target: part.clone(),
                position: vector(0.0, 5.0, 0.0),
            }),
            block(BlockKind::MoveBy {
                target: part.clone(),
                offset: vector(0.0, 1.0, 0.0),
            }),
            block(BlockKind::MoveForward {
                target: part.clone(),
                distance: number(5.0),
            }),
            block(BlockKind::LookAt {
                target: part.clone(),
                position: vector(0.0, 0.0, 0.0),
            }),
            block(BlockKind::TurnTo {
                target: part.clone(),
                orientation: vector(0.0, 0.0, 0.0),
            }),
            block(BlockKind::TurnBy {
                target: part.clone(),
                rotation: vector(0.0, 15.0, 0.0),
                space: axcenity_blocks::SPACE_MODES[0].to_owned(),
            }),
            block(BlockKind::SlideTo {
                target: part.clone(),
                position: vector(0.0, 10.0, 0.0),
                duration: number(1.0),
                easing: axcenity_blocks::EASING_STYLES[1].to_owned(),
                wait_until_done: false,
                completed: Vec::new(),
            }),
            block(BlockKind::RotateTo {
                target: part.clone(),
                rotation: vector(0.0, 90.0, 0.0),
                duration: number(1.0),
                easing: axcenity_blocks::EASING_STYLES[1].to_owned(),
                wait_until_done: false,
                completed: Vec::new(),
            }),
            block(BlockKind::SetAnchored {
                target: part.clone(),
                anchored: Expression::Boolean { value: true },
            }),
            block(BlockKind::SetGravity {
                target: part.clone(),
                multiplier: number(0.0),
            }),
            block(BlockKind::PushWithForce {
                target: part.clone(),
                force: vector(0.0, 50.0, 0.0),
            }),
            block(BlockKind::SetPartVelocity {
                target: part.clone(),
                velocity: vector(0.0, 20.0, 0.0),
            }),
            block(BlockKind::SetAngularVelocity {
                target: part.clone(),
                velocity: vector(0.0, 5.0, 0.0),
            }),
            block(BlockKind::LaunchPart {
                target: part.clone(),
                direction: vector(0.0, 1.0, 0.0),
                speed: number(50.0),
            }),
            block(BlockKind::SetFriction {
                target: part.clone(),
                friction: number(0.1),
            }),
            block(BlockKind::SetDestination {
                agent: first(Filter::Agents),
                position: vector(10.0, 0.0, 0.0),
            }),
            block(BlockKind::FollowTarget {
                agent: first(Filter::Agents),
                target: part.clone(),
            }),
            block(BlockKind::StopAgent {
                agent: first(Filter::Agents),
            }),
            block(BlockKind::SetVelocity {
                agent: first(Filter::Agents),
                velocity: vector(0.0, 0.0, 0.0),
            }),
            block(BlockKind::WhenAgentSenses {
                agent: first(Filter::Agents),
                sense: axcenity_blocks::SENSES[0].to_owned(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenAgentLosesPlayer {
                agent: first(Filter::Agents),
                sense: axcenity_blocks::SENSES[0].to_owned(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenAgentArrives {
                agent: first(Filter::Agents),
                body: Vec::new(),
            }),
            block(BlockKind::Patrol {
                agent: first(Filter::Agents),
                spline: first(Filter::Splines),
            }),
            block(BlockKind::PlaceOnSpline {
                target: part.clone(),
                spline: first(Filter::Splines),
                distance: number(0.0),
            }),
            block(BlockKind::ControlSpline {
                follower: first(Filter::SplineFollowers),
                action: axcenity_blocks::SPLINE_ACTIONS[0].to_owned(),
            }),
            block(BlockKind::SetSplineSpeed {
                follower: first(Filter::SplineFollowers),
                speed: number(10.0),
            }),
            block(BlockKind::WhenSplineReached {
                follower: first(Filter::SplineFollowers),
                which: axcenity_blocks::SPLINE_ENDS[0].to_owned(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenSeatOccupied {
                seat: first(Filter::Seats),
                body: Vec::new(),
            }),
            block(BlockKind::WhenSeatVacated {
                seat: first(Filter::Seats),
                body: Vec::new(),
            }),
            block(BlockKind::SitInSeat {
                seat: first(Filter::Seats),
                controller: first(Filter::Controllers),
            }),
            block(BlockKind::LeaveSeat {
                controller: first(Filter::Controllers),
            }),
            block(BlockKind::SetCameraType {
                camera_type: axcenity_blocks::CAMERA_TYPES[0].to_owned(),
            }),
            block(BlockKind::PointCameraAt {
                position: vector(0.0, 0.0, 0.0),
            }),
            block(BlockKind::ShakeCamera {
                intensity: number(1.0),
                seconds: number(0.5),
            }),
        ],
        Category::Looks => vec![
            block(BlockKind::SpawnDecal {
                texture: Expression::Text {
                    value: "assets/textures/hole.png".to_owned(),
                },
                position: vector(0.0, 0.0, 0.0),
                facing: vector(0.0, 1.0, 0.0),
                size: number(1.0),
                seconds: number(10.0),
            }),
            block(BlockKind::SetColor {
                target: part.clone(),
                color: Expression::Color3 {
                    r: 1.0,
                    g: 0.3,
                    b: 0.3,
                },
            }),
            block(BlockKind::SetText {
                target: first(Filter::Texts),
                text: text("Hello!"),
            }),
            block(BlockKind::Show {
                target: first(Filter::Gui),
            }),
            block(BlockKind::Hide {
                target: first(Filter::Gui),
            }),
            block(BlockKind::ToggleVisibility {
                target: first(Filter::Gui),
            }),
            block(BlockKind::SetTransparency {
                target: part.clone(),
                transparency: number(0.5),
            }),
            block(BlockKind::SetMaterial {
                target: part.clone(),
                material: axcenity_blocks::MATERIALS[0].to_owned(),
            }),
            block(BlockKind::EmitParticles {
                target: part.clone(),
                count: number(20.0),
            }),
            block(BlockKind::SetEffectEnabled {
                target: part.clone(),
                enabled: Expression::Boolean { value: true },
            }),
            block(BlockKind::SetImage {
                target: first(Filter::Gui),
                image: text("assets/textures/icon.png"),
            }),
            block(BlockKind::TweenProperty {
                target: part.clone(),
                property: "Transparency".to_owned(),
                value: number(1.0),
                duration: number(1.0),
                easing: axcenity_blocks::EASING_STYLES[1].to_owned(),
                wait_until_done: false,
                completed: Vec::new(),
            }),
            block(BlockKind::ShowPage {
                layout: first(Filter::PageLayouts),
                page: text("Home"),
            }),
            block(BlockKind::ScrollTo {
                target: first(Filter::ScrollingFrames),
                x: number(0.0),
                y: number(0.0),
            }),
            block(BlockKind::SetValue {
                target: first(Filter::Widgets),
                value: number(50.0),
            }),
            block(BlockKind::SetProperty {
                target: part.clone(),
                property: "Size".to_owned(),
                value: vector(4.0, 1.0, 2.0),
            }),
            block(BlockKind::SetLanguage {
                language: text("English"),
            }),
            block(BlockKind::SetToneMapping {
                curve: "Neutral".to_owned(),
            }),
            block(BlockKind::SetClockTime {
                time: number(18.0),
                duration: None,
            }),
            block(BlockKind::SetAtmosphere {
                property: "Haze".to_owned(),
                value: number(2.0),
                duration: None,
            }),
            block(BlockKind::SetLightingProperty {
                property: axcenity_blocks::LIGHTING_PROPERTIES[0].to_owned(),
                value: number(1.0),
            }),
            block(BlockKind::SetPostProcessing {
                effect: axcenity_blocks::POST_PROCESSING_EFFECTS[0].to_owned(),
                property: "Intensity".to_owned(),
                value: number(0.5),
            }),
            block(BlockKind::SetGraphicsQuality {
                quality: "High".to_owned(),
            }),
            block(BlockKind::SetGraphicsOption {
                setting: "ShadowQuality".to_owned(),
                choice: "High".to_owned(),
                value: None,
            }),
        ],
        Category::Sound => vec![
            block(BlockKind::PlaySound {
                target: sound.clone(),
                pitch: None,
                wait_until_done: false,
            }),
            block(BlockKind::PlaySoundAtPosition {
                target: sound.clone(),
                position: vector(0.0, 0.0, 0.0),
                volume: None,
                pitch: None,
            }),
            block(BlockKind::StopSound {
                target: sound.clone(),
            }),
            block(BlockKind::PauseSound {
                target: sound.clone(),
            }),
            block(BlockKind::ResumeSound {
                target: sound.clone(),
            }),
            block(BlockKind::SetVolume {
                target: sound.clone(),
                volume: number(0.5),
            }),
            block(BlockKind::FadeVolume {
                target: sound.clone(),
                volume: number(0.0),
                duration: number(1.0),
                wait_until_done: false,
            }),
            block(BlockKind::SetSoundPitch {
                target: sound.clone(),
                pitch: number(1.0),
            }),
            block(BlockKind::SetSoundLooping {
                target: sound.clone(),
                looped: Expression::Boolean { value: true },
            }),
            block(BlockKind::SetSoundTime {
                target: sound.clone(),
                time: number(0.0),
            }),
            R(Expression::SoundProperty {
                target: sound,
                property: axcenity_blocks::SOUND_PROPERTIES[0].to_owned(),
            }),
        ],
        Category::Operators => {
            let math = |op| {
                R(Expression::Math {
                    op,
                    left: Box::new(number(0.0)),
                    right: Box::new(number(0.0)),
                })
            };
            let compare = |op, right| {
                R(Expression::Compare {
                    op,
                    left: Box::new(text("")),
                    right: Box::new(number(right)),
                })
            };
            let logic = |op| {
                R(Expression::Logic {
                    op,
                    left: Box::new(condition()),
                    right: Box::new(condition()),
                })
            };
            vec![
                math(MathOp::Add),
                math(MathOp::Subtract),
                math(MathOp::Multiply),
                math(MathOp::Divide),
                R(Expression::Random {
                    min: Box::new(number(1.0)),
                    max: Box::new(number(10.0)),
                }),
                R(Expression::RandomNumber {
                    min: Box::new(number(0.0)),
                    max: Box::new(number(1.0)),
                }),
                R(Expression::Lerp {
                    from: Box::new(number(0.0)),
                    to: Box::new(number(10.0)),
                    alpha: Box::new(number(0.5)),
                }),
                R(Expression::Clamp {
                    value: Box::new(number(5.0)),
                    min: Box::new(number(0.0)),
                    max: Box::new(number(10.0)),
                }),
                R(Expression::MinMax {
                    op: "min".to_owned(),
                    left: Box::new(number(0.0)),
                    right: Box::new(number(10.0)),
                }),
                compare(Comparison::Greater, 50.0),
                compare(Comparison::Less, 50.0),
                compare(Comparison::Equal, 50.0),
                logic(LogicOp::And),
                logic(LogicOp::Or),
                R(Expression::Not {
                    value: Box::new(condition()),
                }),
                R(Expression::Join {
                    left: Box::new(text("apple ")),
                    right: Box::new(text("banana")),
                }),
                R(Expression::StringLength {
                    text: Box::new(text("hello")),
                }),
                R(Expression::StringContains {
                    text: Box::new(text("hello")),
                    substring: Box::new(text("ell")),
                }),
                R(Expression::FormatNumber {
                    value: Box::new(number(12.345)),
                    decimals: Box::new(number(2.0)),
                }),
                math(MathOp::Modulo),
                R(Expression::Function {
                    function: MathFunction::Round,
                    value: Box::new(number(0.0)),
                }),
                R(Expression::MakeVector3 {
                    x: Box::new(number(0.0)),
                    y: Box::new(number(0.0)),
                    z: Box::new(number(0.0)),
                }),
                R(Expression::VectorAxis {
                    axis: "Y".to_owned(),
                    vector: Box::new(vector(0.0, 0.0, 0.0)),
                }),
                R(Expression::Distance {
                    from: Box::new(vector(0.0, 0.0, 0.0)),
                    to: Box::new(vector(0.0, 10.0, 0.0)),
                }),
                R(Expression::Direction {
                    from: Box::new(vector(0.0, 0.0, 0.0)),
                    to: Box::new(vector(0.0, 10.0, 0.0)),
                }),
                R(Expression::Magnitude {
                    vector: Box::new(vector(0.0, 10.0, 0.0)),
                }),
                R(Expression::MakeColorRgb {
                    r: Box::new(number(255.0)),
                    g: Box::new(number(128.0)),
                    b: Box::new(number(0.0)),
                }),
                R(Expression::MakeColorHsv {
                    h: Box::new(number(0.5)),
                    s: Box::new(number(1.0)),
                    v: Box::new(number(1.0)),
                }),
            ]
        }
        Category::Variables => {
            let mut items = vec![PaletteItem::NewVariable];
            items.extend(
                variables
                    .iter()
                    .map(|name| R(Expression::Variable { name: name.clone() })),
            );
            items.push(block(BlockKind::SetVariable {
                name: variable.clone(),
                value: number(0.0),
            }));
            items.push(block(BlockKind::ChangeVariable {
                name: variable.clone(),
                by: number(1.0),
            }));
            items.push(block(BlockKind::SetAttribute {
                target: part.clone(),
                attribute: NameKind::Attribute.example().to_owned(),
                value: number(100.0),
            }));
            items.push(R(Expression::Attribute {
                target: part.clone(),
                attribute: NameKind::Attribute.example().to_owned(),
            }));
            let list_var = Expression::Variable {
                name: "items".to_owned(),
            };
            items.push(block(BlockKind::ListCreate {
                name: "items".to_owned(),
                items: Vec::new(),
            }));
            items.push(block(BlockKind::ListAdd {
                list: list_var.clone(),
                value: text("item"),
            }));
            items.push(block(BlockKind::ListInsert {
                list: list_var.clone(),
                index: number(1.0),
                value: text("item"),
            }));
            items.push(block(BlockKind::ListRemoveIndex {
                list: list_var.clone(),
                index: number(1.0),
            }));
            items.push(block(BlockKind::ListRemoveValue {
                list: list_var.clone(),
                value: text("item"),
            }));
            items.push(block(BlockKind::ListClear {
                list: list_var.clone(),
            }));
            items.push(R(Expression::ListItem {
                list: Box::new(list_var.clone()),
                index: Box::new(number(1.0)),
            }));
            items.push(R(Expression::ListLength {
                list: Box::new(list_var.clone()),
            }));
            items.push(R(Expression::ListContains {
                list: Box::new(list_var),
                value: Box::new(text("item")),
            }));
            items
        }
        Category::Sensing => vec![
            R(Expression::TouchingPart),
            R(Expression::TriggeringPlayer),
            R(Expression::CommandArgs),
            R(Expression::DeltaTime),
            R(Expression::GameTime),
            R(Expression::PartUnderMouse),
            R(Expression::MouseHitPosition),
            R(Expression::CameraPosition),
            R(Expression::CameraLookDirection),
            R(Expression::KeyDown {
                key: "Space".to_owned(),
            }),
            R(Expression::Property {
                target: part.clone(),
                property: "Position".to_owned(),
            }),
            R(Expression::Raycast {
                origin: Box::new(vector(0.0, 10.0, 0.0)),
                direction: Box::new(vector(0.0, -20.0, 0.0)),
                output: axcenity_blocks::RAYCAST_OUTPUTS[0].to_owned(),
                ignore: None,
            }),
            R(Expression::NearestPlayerTo {
                position: Box::new(vector(0.0, 0.0, 0.0)),
            }),
            R(Expression::PlayerOfPart { part: part.clone() }),
            R(Expression::PlayerIsAlive {
                player: ObjectRef(Box::new(Expression::TriggeringPlayer)),
            }),
            R(Expression::ParentOf {
                target: part.clone(),
            }),
            R(Expression::ChildOf {
                name: Box::new(text("Humanoid")),
                target: part.clone(),
            }),
            R(Expression::HasTag {
                target: part,
                tag: Box::new(text("Hazard")),
            }),
            R(Expression::SplinePosition {
                spline: first(Filter::Splines),
                t: Box::new(number(0.5)),
            }),
            R(Expression::SplineClosestDistance {
                spline: first(Filter::Splines),
                position: Box::new(vector(0.0, 0.0, 0.0)),
            }),
            R(Expression::IsA {
                value: Box::new(Expression::TouchingPart),
                class: "BasePart".to_owned(),
            }),
        ],
        Category::Animation => {
            let animator = first(Filter::Animators);
            let animator_path = animator.as_path().unwrap_or("");
            let name = |kind: NameKind| {
                objects
                    .iter()
                    .find(|object| object.path == animator_path)
                    .and_then(|object| object.names.iter().find(|(of, _)| *of == kind))
                    .map_or_else(|| kind.example().to_owned(), |(_, name)| name.clone())
            };
            vec![
                block(BlockKind::SetAnimatorTrigger {
                    animator: animator.clone(),
                    parameter: name(NameKind::Trigger),
                }),
                block(BlockKind::SetAnimatorBool {
                    animator: animator.clone(),
                    parameter: name(NameKind::Bool),
                    value: Expression::Boolean { value: true },
                }),
                block(BlockKind::SetAnimatorNumber {
                    animator: animator.clone(),
                    parameter: name(NameKind::Number),
                    value: number(1.0),
                }),
                block(BlockKind::ResetAnimatorTrigger {
                    animator: animator.clone(),
                    parameter: name(NameKind::Trigger),
                }),
                block(BlockKind::SetAnimatorLayerWeight {
                    animator: animator.clone(),
                    layer: name(NameKind::Layer),
                    weight: number(1.0),
                    seconds: number(0.25),
                }),
                block(BlockKind::PlayAnimatorState {
                    animator: animator.clone(),
                    state: name(NameKind::State),
                }),
                block(BlockKind::PlayAnimation {
                    animator: animator.clone(),
                    animation: first(Filter::Animations),
                    fade_time: None,
                    speed: None,
                    weight: None,
                    wait_until_done: false,
                }),
                block(BlockKind::StopAnimation {
                    animator: animator.clone(),
                    animation: None,
                    fade_time: None,
                }),
                block(BlockKind::SetAnimatorSpeed {
                    animator: animator.clone(),
                    speed: number(1.0),
                }),
                block(BlockKind::SetIkTarget {
                    ik_control: first(Filter::IkControls),
                    target: part.clone(),
                    weight: None,
                }),
                block(BlockKind::SetRagdollEnabled {
                    ragdoll: first(Filter::Ragdolls),
                    enabled: Expression::Boolean { value: true },
                }),
                block(BlockKind::PlayCutscene {
                    sequence: first(Filter::Sequences),
                }),
                block(BlockKind::PauseCutscene {
                    sequence: first(Filter::Sequences),
                }),
                block(BlockKind::ResumeCutscene {
                    sequence: first(Filter::Sequences),
                }),
                block(BlockKind::StopCutscene {
                    sequence: first(Filter::Sequences),
                }),
                block(BlockKind::WhenCutsceneFinished {
                    sequence: first(Filter::Sequences),
                    body: Vec::new(),
                }),
                block(BlockKind::WhenAnimationFinished {
                    animator: animator.clone(),
                    animation: first(Filter::Animations),
                    body: Vec::new(),
                }),
                block(BlockKind::WhenStateChanged {
                    animator: animator.clone(),
                    body: Vec::new(),
                }),
                block(BlockKind::WhenAnimationEvent {
                    animator: animator.clone(),
                    event: name(NameKind::Event),
                    body: Vec::new(),
                }),
                R(Expression::AnimatorNewState),
                R(Expression::AnimatorPreviousState),
                R(Expression::AnimationEventName),
                R(Expression::AnimatorBool {
                    animator: animator.clone(),
                    parameter: name(NameKind::Bool),
                }),
                R(Expression::AnimatorNumber {
                    animator: animator.clone(),
                    parameter: name(NameKind::Number),
                }),
                R(Expression::AnimatorLayerWeight {
                    animator: animator.clone(),
                    layer: name(NameKind::Layer),
                }),
                R(Expression::Property {
                    target: animator,
                    property: "CurrentState".to_owned(),
                }),
            ]
        }
        Category::Tweens => {
            let gui = first(Filter::Gui);
            vec![
                block(BlockKind::Tween {
                    target: part.clone(),
                    property: "Position".to_owned(),
                    value: vector(0.0, 10.0, 0.0),
                    seconds: number(1.0),
                    style: "Quad".to_owned(),
                    direction: "Out".to_owned(),
                    repeat_count: None,
                    reverses: false,
                    wait_until_done: false,
                    completed: Vec::new(),
                }),
                block(BlockKind::TweenPivot {
                    target: part.clone(),
                    position: vector(0.0, 10.0, 0.0),
                    duration: number(1.0),
                    easing: "Quad".to_owned(),
                    direction: "Out".to_owned(),
                    wait_until_done: false,
                    completed: Vec::new(),
                }),
                block(BlockKind::SlideIn {
                    target: gui.clone(),
                    from: "Left".to_owned(),
                    seconds: number(0.5),
                    wait_until_done: false,
                    completed: Vec::new(),
                }),
                block(BlockKind::SlideOut {
                    target: gui.clone(),
                    to: "Left".to_owned(),
                    seconds: number(0.5),
                    wait_until_done: false,
                    completed: Vec::new(),
                }),
                block(BlockKind::FadeIn {
                    target: gui.clone(),
                    seconds: number(0.5),
                    wait_until_done: false,
                    completed: Vec::new(),
                }),
                block(BlockKind::FadeOut {
                    target: gui,
                    seconds: number(0.5),
                    wait_until_done: false,
                    completed: Vec::new(),
                }),
                block(BlockKind::PulseScale {
                    target: part.clone(),
                    scale: number(1.2),
                    duration: number(0.3),
                    wait_until_done: false,
                }),
                block(BlockKind::CancelTween { tween: part }),
            ]
        }
        Category::DataAndSaves => vec![
            block(BlockKind::Print {
                value: text("Hello!"),
            }),
            block(BlockKind::SaveSlot { slot: number(1.0) }),
            block(BlockKind::LoadSlot { slot: number(1.0) }),
            block(BlockKind::DeleteSlot { slot: number(1.0) }),
            R(Expression::SlotExists {
                slot: Box::new(number(1.0)),
            }),
            block(BlockKind::SetCheckpoint {
                checkpoint: first(Filter::Checkpoints),
            }),
            block(BlockKind::RestoreCheckpoint),
            R(Expression::DataValue {
                column: storage_name(NameKind::Column),
                row: Box::new(Expression::DataRow {
                    table: storage_name(NameKind::DataTable),
                    key: Box::new(text("sword")),
                }),
            }),
            R(Expression::DataRow {
                table: storage_name(NameKind::DataTable),
                key: Box::new(text("sword")),
            }),
            R(Expression::DataTableRowCount {
                table: storage_name(NameKind::DataTable),
            }),
            block(BlockKind::ForEachRow {
                table: storage_name(NameKind::DataTable),
                body: Vec::new(),
            }),
        ],
        Category::WorldAndLevels => vec![
            block(BlockKind::LoadLevel {
                level: text("Castle"),
                spawn: text("Gate"),
            }),
            block(BlockKind::AddLevel {
                level: text("Dungeon"),
            }),
            block(BlockKind::RemoveLevel {
                level: text("Dungeon"),
            }),
            block(BlockKind::SetLayerActive {
                data_layer: "Night".to_owned(),
                active: Expression::Boolean { value: true },
            }),
            block(BlockKind::WhenLayerChanged {
                data_layer: "Any".to_owned(),
                body: Vec::new(),
            }),
            R(Expression::LayerActive {
                data_layer: "Night".to_owned(),
            }),
            R(Expression::ChangedLayerName),
            R(Expression::ChangedLayerActive),
            block(BlockKind::RequestStreamAround {
                player: Expression::Variable {
                    name: "player".to_owned(),
                },
                position: Expression::Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 500.0,
                },
            }),
            block(BlockKind::WhenStreamedIn {
                target: part.clone(),
                body: Vec::new(),
            }),
            block(BlockKind::WhenStreamedOut {
                target: part.clone(),
                body: Vec::new(),
            }),
            R(Expression::IsStreamedIn {
                value: Box::new(part.as_expr().clone()),
            }),
            R(Expression::ChildOf {
                name: Box::new(text("Humanoid")),
                target: part.clone(),
            }),
            block(BlockKind::CloneInto {
                target: first(Filter::Parts),
                parent: ObjectRef::default(),
                count: Some(number(1.0)),
                position: None,
                offset_per_clone: None,
                per_clone_positions: None,
            }),
            block(BlockKind::Destroy {
                target: first(Filter::Parts),
            }),
        ],
        Category::MultiplayerAndChat => vec![
            block(BlockKind::HostGame {
                port: number(47800.0),
            }),
            block(BlockKind::JoinGame {
                address: text("192.168.1.20:47800"),
            }),
            block(BlockKind::LeaveGame),
            R(Expression::IsHost),
            R(Expression::IsConnected),
            block(BlockKind::WhenConnectionFails { body: Vec::new() }),
            R(Expression::ConnectionFailureReason),
            block(BlockKind::WhenPlayerJoins { body: Vec::new() }),
            block(BlockKind::WhenPlayerLeaves { body: Vec::new() }),
            block(BlockKind::SetNetworkOwner {
                target: part.clone(),
                owner: Expression::Variable {
                    name: "player".to_owned(),
                },
            }),
            block(BlockKind::SetNetworkOwnershipAuto {
                target: part.clone(),
            }),
            block(BlockKind::KickPlayer {
                player: Expression::Variable {
                    name: "player".to_owned(),
                },
                reason: text("Cheating"),
            }),
            block(BlockKind::WhenChatMessageReceived { body: Vec::new() }),
            R(Expression::ChatMessageText),
            R(Expression::ChatMessageSender),
            block(BlockKind::SendChatMessage {
                text: text("Hello"),
            }),
            block(BlockKind::ShowSystemMessage {
                text: text("Round over"),
            }),
        ],
        // Grey blocks only come from code, so there are none of those.
        Category::Lua => vec![
            block(BlockKind::Comment {
                text: "-- A note".to_owned(),
            }),
            block(BlockKind::CallFunction {
                function: name("print"),
                args: vec![text("Hello")],
            }),
            block(BlockKind::CallMethod {
                object: name("workspace"),
                method: "GetChildren".to_owned(),
                args: Vec::new(),
            }),
            R(Expression::Call {
                function: Box::new(name("tostring")),
                args: vec![number(1.0)],
            }),
            R(Expression::MethodCall {
                object: Box::new(name("workspace")),
                method: "WaitForChild".to_owned(),
                args: vec![text("Part")],
            }),
            R(Expression::Index {
                object: Box::new(name("list")),
                key: Box::new(number(1.0)),
            }),
            R(Expression::Table {
                fields: vec![TableField::Named {
                    name: "name".to_owned(),
                    value: number(1.0),
                }],
                multiline: false,
            }),
            R(Expression::Binary {
                op: "..".to_owned(),
                left: Box::new(text("a")),
                right: Box::new(text("b")),
            }),
            R(Expression::Unary {
                op: "#".to_owned(),
                value: Box::new(name("list")),
            }),
        ],
    };
    // The blocks for Luau itself come after each category's own.
    let mut luau = Vec::new();
    match category {
        Category::Events => luau.extend([
            block(BlockKind::RunScript { body: Vec::new() }),
            block(BlockKind::OnSignal {
                signal: name("workspace.ChildAdded"),
                method: "Connect".to_owned(),
                params: "child".to_owned(),
                body: Vec::new(),
            }),
            block(BlockKind::Connect {
                signal: name("workspace.ChildAdded"),
                method: "Connect".to_owned(),
                params: "child".to_owned(),
                body: Vec::new(),
            }),
        ]),
        Category::Control => luau.extend([
            block(BlockKind::DefineFunction {
                name: "myFunction".to_owned(),
                local: true,
                params: "value".to_owned(),
                generics: String::new(),
                returns: String::new(),
                attributes: String::new(),
                body: Vec::new(),
            }),
            block(BlockKind::While {
                condition: condition(),
                body: Vec::new(),
            }),
            block(BlockKind::RepeatLoop {
                body: Vec::new(),
                condition: condition(),
            }),
            block(BlockKind::NumericFor {
                variable: "i".to_owned(),
                start: number(1.0),
                stop: number(10.0),
                step: None,
                body: Vec::new(),
            }),
            block(BlockKind::GenericFor {
                names: "key, value".to_owned(),
                values: vec![name("list")],
                body: Vec::new(),
            }),
            block(BlockKind::DoBlock { body: Vec::new() }),
            block(BlockKind::Break),
            block(BlockKind::Continue),
            block(BlockKind::Return {
                values: vec![name("value")],
            }),
        ]),
        Category::Variables => luau.extend([
            block(BlockKind::Local {
                names: "name".to_owned(),
                values: vec![number(0.0)],
            }),
            block(BlockKind::Assign {
                targets: vec![name("name")],
                values: vec![number(0.0)],
            }),
            block(BlockKind::CompoundAssign {
                target: name("name"),
                op: "+".to_owned(),
                value: number(1.0),
            }),
        ]),
        _ => {}
    }
    // Keyboard and button blocks only work on the player's side.
    if !client {
        for list in [&mut items, &mut luau] {
            list.retain(|item| match item {
                PaletteItem::Block(kind) => !kind.client_only(),
                PaletteItem::Reporter(expression) => !expression.client_only(),
                PaletteItem::NewVariable => true,
            });
        }
    }
    (items, luau)
}

/// The height of the search box above the palette's blocks.
pub(super) const SEARCH_HEIGHT: f32 = 40.0;

/// What a palette item is found by when searching: the words on it, its
/// category and the Lua it becomes, in lower case.
pub(super) fn search_text(
    item: &PaletteItem,
    category: Category,
    objects: &[ObjectInfo],
) -> String {
    let mut words = vec![category.name().to_owned()];
    fn add_labels(words: &mut Vec<String>, parts: &[Part]) {
        for part in parts {
            if let Part::Label(text) = part {
                words.push(text.clone());
            }
        }
    }
    match item {
        PaletteItem::Block(kind) => {
            words.extend(item_lua(item));
            let mut kind = (**kind).clone();
            for section in sections(&mut kind, objects) {
                if let Section::Row(parts) = &section {
                    add_labels(&mut words, parts);
                }
            }
        }
        PaletteItem::Reporter(expression) => {
            words.extend(item_lua(item));
            let mut expression = expression.clone();
            add_labels(&mut words, &reporter_parts(&mut expression, objects));
        }
        PaletteItem::NewVariable => words.push("make a variable".to_owned()),
    }
    words.join(" ").to_lowercase()
}

/// Whether every word typed in the search box is in `text` (from
/// [`search_text`]).
pub(super) fn matches_search(text: &str, search: &str) -> bool {
    search
        .split_whitespace()
        .all(|word| text.contains(&word.to_lowercase()))
}

/// The Lua a palette block becomes, on its own.
fn block_lua(kind: &BlockKind) -> Option<String> {
    axcenity_blocks::block_lua(&Block::new(kind.clone()), true).ok()
}

/// The Lua a palette item becomes, on its own.
pub(super) fn item_lua(item: &PaletteItem) -> Option<String> {
    match item {
        PaletteItem::Block(kind) => block_lua(kind),
        PaletteItem::Reporter(expression) => axcenity_blocks::expression_lua(expression, true).ok(),
        PaletteItem::NewVariable => None,
    }
}

/// What the catalogue says about a palette or node menu item.
pub(super) fn item_info(item: &PaletteItem) -> Option<axcenity_blocks::catalogue::NodeInfo> {
    match item {
        PaletteItem::Block(kind) => Some(axcenity_blocks::catalogue::block_info(kind)),
        PaletteItem::Reporter(expression) => {
            Some(axcenity_blocks::catalogue::expression_info(expression))
        }
        PaletteItem::NewVariable => None,
    }
}

/// A palette, node menu or node tooltip (cohesion pass CW4.2): the
/// catalogue's card (what it does, its inputs, what it gives, an example),
/// `how` to use it, then the Lua it becomes, so a beginner sees what it
/// does and learns the Lua too.
pub(super) fn lua_hint(
    ui: &mut egui::Ui,
    how: &str,
    info: Option<axcenity_blocks::catalogue::NodeInfo>,
    lua: Option<String>,
) {
    match &info {
        Some(info) => crate::node_graph::node_card(ui, info, how),
        None => {
            ui.set_max_width(crate::widgets::INFO_CARD_WIDTH);
            ui.label(how);
        }
    }
    if let Some(lua) = lua {
        ui.label(
            egui::RichText::new("It becomes this Lua:")
                .size(theme::type_scale::SMALL)
                .color(theme::palette(ui.ctx()).text_weak),
        );
        let job = lua_syntax::layout_job(&lua, FontId::monospace(12.0), ui.visuals());
        ui.add(egui::Label::new(job));
    }
}

/// Parses an initial value string from the variable-creation UI into a literal
/// [`Expression`] (number, boolean, or text).
fn parse_initial_value(raw: &str) -> Expression {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Expression::Number { value: 0.0 };
    }
    if let Ok(value) = trimmed.parse::<f64>()
        && value.is_finite()
    {
        return Expression::Number { value };
    }
    match trimmed {
        "true" => Expression::Boolean { value: true },
        "false" => Expression::Boolean { value: false },
        _ => Expression::Text {
            value: trimmed.to_owned(),
        },
    }
}

pub(super) fn show_palette(
    ui: &mut egui::Ui,
    rect: Rect,
    script: &mut BlockScript,
    state: &mut State,
    context: &BlockContext,
    variables: &[String],
) {
    let background = ui.visuals().faint_bg_color;
    ui.painter().rect_filled(rect, 0, background);
    // The search box, then the blocks under it.
    let search_rect = Rect::from_min_size(rect.min, vec2(rect.width(), SEARCH_HEIGHT));
    let mut search_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(search_rect.shrink2(vec2(12.0, 8.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let search_box = search_ui
        .add(
            egui::TextEdit::singleline(&mut state.search)
                .id(egui::Id::new(("block search", context.key)))
                .hint_text(theme::with_icon(icons::MAGNIFYING_GLASS, "Search blocks"))
                .desired_width(f32::INFINITY),
        )
        .on_hover_text(
            "Finds blocks by their words or the Lua they become, such as \"sound\" or \"Destroy\"",
        );
    if search_box.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.search.clear();
    }
    let list_rect = Rect::from_min_max(pos2(rect.left(), search_rect.bottom()), rect.max);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list_rect));
    child.set_clip_rect(list_rect);
    let scroll_to = state.scroll_to.take();
    let mut new_variable = state.new_variable.take();
    let mut made: Option<(String, String, bool)> = None;
    let mut started = None;
    let clone_pills: Vec<(u64, String)> = clone_identifiers(script)
        .into_iter()
        .map(|(id, (single, _))| (id, single))
        .collect();
    let search = state.search.trim().to_owned();
    egui::ScrollArea::both()
        .id_salt(("block palette", context.key, search.is_empty()))
        .auto_shrink(false)
        .show(&mut child, |ui| {
            ui.add_space(8.0);
            let mut found = false;
            for category in Category::ALL {
                let (mut items, mut luau) =
                    palette_parts(category, context.client, context.objects, variables);
                if category == Category::Variables {
                    for (block_id, single_name) in &clone_pills {
                        items.insert(
                            1,
                            PaletteItem::Reporter(Expression::BlockOutput {
                                block: *block_id,
                                output: 0,
                                name: single_name.clone(),
                            }),
                        );
                    }
                }
                if !search.is_empty() {
                    for list in [&mut items, &mut luau] {
                        list.retain(|item| {
                            matches_search(&search_text(item, category, context.objects), &search)
                        });
                    }
                    if items.is_empty() && luau.is_empty() {
                        continue;
                    }
                    found = true;
                }
                // The Luau blocks under their own heading, folded unless
                // opened or searched for.
                let mut rows: Vec<Option<PaletteItem>> = items.into_iter().map(Some).collect();
                if !luau.is_empty() {
                    rows.push(None);
                    if !search.is_empty() || state.luau_open.contains(&category) {
                        rows.extend(luau.into_iter().map(Some));
                    }
                }
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    // The category list's swatch, small.
                    let (swatch, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    ui.painter()
                        .rect_filled(swatch, 5, category.fill(&theme::palette(ui.ctx())));
                    ui.painter().text(
                        swatch.center(),
                        egui::Align2::CENTER_CENTER,
                        category.icon(),
                        FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                    let heading = ui.label(
                        egui::RichText::new(category.name())
                            .strong()
                            .family(theme::heading_family()),
                    );
                    if scroll_to == Some(category) {
                        let row_rect = Rect::from_min_max(
                            pos2(swatch.left() - 12.0, heading.rect.top()),
                            heading.rect.max,
                        );
                        ui.scroll_to_rect(row_rect, Some(egui::Align::TOP));
                    }
                });
                ui.add_space(6.0);
                for row in rows {
                    let Some(item) = row else {
                        if luau_heading(ui, category, state, !search.is_empty()) {
                            if let Some(at) =
                                state.luau_open.iter().position(|open| *open == category)
                            {
                                state.luau_open.remove(at);
                            } else {
                                state.luau_open.push(category);
                            }
                        }
                        ui.add_space(8.0);
                        continue;
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        match item {
                            PaletteItem::NewVariable => {
                                if let Some(name) = &mut new_variable {
                                    let resp = ui.vertical(|ui| {
                                        let edit = ui.add(
                                            egui::TextEdit::singleline(name)
                                                .hint_text("Variable name")
                                                .desired_width(150.0),
                                        );
                                        let just_opened =
                                            std::mem::take(&mut state.focus_new_variable);
                                        if just_opened {
                                            edit.request_focus();
                                        }
                                        let mut enter = (edit.lost_focus() || edit.has_focus())
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                        let init_edit = ui.horizontal(|ui| {
                                            ui.label("Initial:");
                                            ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut state.new_variable_initial,
                                                )
                                                .hint_text("0")
                                                .desired_width(90.0),
                                            )
                                        });
                                        if (init_edit.inner.lost_focus()
                                            || init_edit.inner.has_focus())
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        {
                                            enter = true;
                                        }
                                        let ok = ui
                                            .horizontal(|ui| {
                                                ui.checkbox(
                                                    &mut state.new_variable_global,
                                                    "Global / Shared",
                                                )
                                                .on_hover_text(
                                                    "Checked: shared across scripts (_G). \
                                                     Unchecked: local to this script.",
                                                );
                                                ui.button("OK")
                                            })
                                            .inner;
                                        (just_opened, enter, ok.clicked())
                                    });
                                    let (just_opened, enter, ok_clicked) = resp.inner;
                                    let modal_rect = resp.response.rect;
                                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                    let clicked_outside = !just_opened
                                        && ui.input(|i| {
                                            i.pointer.any_pressed()
                                                && i.pointer
                                                    .interact_pos()
                                                    .is_some_and(|pos| !modal_rect.contains(pos))
                                        });
                                    if enter || ok_clicked {
                                        made = Some((
                                            name.clone(),
                                            state.new_variable_initial.clone(),
                                            state.new_variable_global,
                                        ));
                                    } else if escape || clicked_outside {
                                        new_variable = None;
                                    }
                                } else if ui
                                    .button("Make a Variable")
                                    .on_hover_text(
                                        "Names a value the whole script can read and change, \
                                         such as a score. Its blocks appear below.",
                                    )
                                    .clicked()
                                {
                                    new_variable = Some(String::new());
                                    state.new_variable_initial = "0".to_owned();
                                    state.new_variable_global = false;
                                    state.focus_new_variable = true;
                                }
                            }
                            PaletteItem::Block(kind) => {
                                if let Some(drag) =
                                    palette_block(ui, *kind, context, variables, background)
                                {
                                    started = Some(drag);
                                }
                            }
                            PaletteItem::Reporter(expression) => {
                                let var_name = match &expression {
                                    Expression::Variable { name } => Some(name.clone()),
                                    _ => None,
                                };
                                if let Some(drag) =
                                    palette_reporter(ui, expression, context, variables)
                                {
                                    started = Some(drag);
                                }
                                if category == Category::Variables
                                    && let Some(var_name) = var_name
                                {
                                    let (mut global, mut watched) = script
                                        .variables
                                        .iter()
                                        .find(|d| d.name == var_name)
                                        .map_or((false, false), |d| (d.global, d.watched));
                                    let prev = (global, watched);
                                    let scope_label = if global { "Global" } else { "Local" };
                                    if ui
                                        .small_button(scope_label)
                                        .on_hover_text(
                                            "Toggle variable scope: Local (this script) vs Global (_G shared)",
                                        )
                                        .clicked()
                                    {
                                        global = !global;
                                    }
                                    ui.checkbox(&mut watched, "")
                                        .on_hover_text("Watch live value in Play HUD");
                                    if (global, watched) != prev {
                                        if let Some(def) = script
                                            .variables
                                            .iter_mut()
                                            .find(|d| d.name == var_name)
                                        {
                                            def.global = global;
                                            def.watched = watched;
                                        } else {
                                            let mut def = VariableDef::new(var_name);
                                            def.global = global;
                                            def.watched = watched;
                                            script.variables.push(def);
                                        }
                                    }
                                }
                            }
                        }
                        ui.add_space(16.0);
                    });
                    ui.add_space(8.0);
                }
                ui.add_space(14.0);
            }
            if !search.is_empty() && !found {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(12.0);
                    ui.weak(format!(
                        "No blocks match \"{search}\". Try another word, or a word from the Lua."
                    ));
                });
            }
        });
    if let Some((name, initial_str, global)) = made {
        new_variable = None;
        let name = name.trim().to_owned();
        if valid_identifier(&name) {
            if !variables.contains(&name) {
                state.variables.push(name.clone());
            }
            let initial = parse_initial_value(&initial_str);
            if let Some(def) = script.variables.iter_mut().find(|d| d.name == name) {
                def.initial = initial;
                def.global = global;
            } else {
                script.variables.push(VariableDef {
                    name,
                    initial,
                    global,
                    watched: false,
                });
            }
        }
    }
    state.new_variable = new_variable;
    if started.is_some() {
        state.drag = started;
    }
}

/// The heading over a category's Luau blocks, which opens and folds them.
/// Returns true if it was clicked.
fn luau_heading(ui: &mut egui::Ui, category: Category, state: &State, searching: bool) -> bool {
    let open = searching || state.luau_open.contains(&category);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        let arrow = if open {
            icons::CARET_DOWN
        } else {
            icons::CARET_RIGHT
        };
        let response = ui
            .add(
                egui::Button::new(
                    egui::RichText::new(format!("{arrow} Lua blocks"))
                        .size(theme::type_scale::SMALL),
                )
                .frame(false),
            )
            .on_hover_text(
                "Blocks for Lua itself: local, while, for, a function, a :Connect. \
                 They write exactly the Lua they say. The blocks above do the common \
                 things more simply; these are for scripts switched from Code, or \
                 when you know the Lua you want.",
            );
        // While searching they're shown anyway.
        response.clicked() && !searching
    })
    .inner
}

/// A block in the palette, which can be dragged out. Returns the drag if
/// one started.
pub(super) fn palette_block(
    ui: &mut egui::Ui,
    kind: BlockKind,
    context: &BlockContext,
    variables: &[String],
    background: Color32,
) -> Option<Drag> {
    let event = kind.is_event();
    let mut block = Block::new(kind.clone());
    // Measured with a painter that draws nothing, then drawn where it fits.
    let measure = ui.painter().with_clip_rect(Rect::NOTHING);
    let mut painter = BlockPainter::new(
        measure,
        1.0,
        false,
        background,
        egui::Id::NULL,
        context,
        variables,
    );
    let size = if event {
        let drawn = painter.hat(ui, &mut block, Pos2::ZERO);
        vec2(drawn.width, drawn.height + TAB_DEPTH)
    } else {
        let drawn = painter.statement(ui, &mut block, Pos2::ZERO, true);
        vec2(drawn.width, drawn.height + TAB_DEPTH)
    };
    let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
    let response = response
        .on_hover_cursor(CursorIcon::Grab)
        .on_hover_ui(|ui| {
            lua_hint(
                ui,
                "Drag it onto the canvas.",
                Some(axcenity_blocks::catalogue::block_info(&kind)),
                block_lua(&kind),
            )
        });
    let mut painter = BlockPainter::new(
        ui.painter().clone(),
        1.0,
        false,
        background,
        egui::Id::NULL,
        context,
        variables,
    );
    if event {
        painter.hat(ui, &mut block, rect.min);
    } else {
        painter.statement(ui, &mut block, rect.min, true);
    }
    painter.finish();
    if response.drag_started()
        && let Some(origin) = response.interact_pointer_pos()
    {
        return Some(Drag {
            item: Dragged::Stack(vec![Block::new(kind)]),
            grab: origin - rect.min,
            size,
        });
    }
    None
}

pub(super) fn palette_reporter(
    ui: &mut egui::Ui,
    mut expression: Expression,
    context: &BlockContext,
    variables: &[String],
) -> Option<Drag> {
    let background = ui.visuals().faint_bg_color;
    let painter = BlockPainter::new(
        ui.painter().clone(),
        1.0,
        false,
        background,
        egui::Id::NULL,
        context,
        variables,
    );
    let size = painter.reporter_size(ui, &mut expression);
    let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
    let response = response
        .on_hover_cursor(CursorIcon::Grab)
        .on_hover_ui(|ui| {
            lua_hint(
                ui,
                "Drag it into a slot of the same shape.",
                Some(axcenity_blocks::catalogue::expression_info(&expression)),
                axcenity_blocks::expression_lua(&expression, true).ok(),
            )
        });
    let mut painter = painter;
    painter.reporter(ui, &mut expression.clone(), rect.min, egui::Id::NULL, None);
    if response.drag_started()
        && let Some(origin) = response.interact_pointer_pos()
    {
        return Some(Drag {
            item: Dragged::Reporter(expression),
            grab: origin - rect.min,
            size,
        });
    }
    None
}
