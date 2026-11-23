use std::collections::HashMap;

use super::*;

mod luau;

pub use luau::{binary_priority, needs_brackets};

#[derive(Clone, Debug, PartialEq)]
pub struct Translation {
    pub source: String,
    /// The generated Lua line each block's code starts on (numbered from 1),
    /// with the block's ID, so a runtime error can point at its block.
    pub block_lines: Vec<(usize, u64)>,
}

impl Translation {
    /// The block whose code is on `line`: the last one starting at or
    /// before it, which is the innermost block around it.
    pub fn block_at(&self, line: usize) -> Option<u64> {
        self.block_lines
            .iter()
            .filter(|(start, _)| *start <= line)
            .max_by_key(|(start, _)| *start)
            .map(|(_, id)| *id)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum TranslateError {
    #[error("block ID {0} appears more than once")]
    DuplicateId(u64),
    #[error(
        "variable name `{0}` must start with a letter or underscore and contain only letters, digits or underscores"
    )]
    InvalidVariable(String),
    #[error("number blocks must contain finite numbers")]
    InvalidNumber,
    #[error("Instance target `{0}` must be a non-empty dotted path of child names")]
    InvalidTarget(String),
    #[error("`{0}` isn't a key name, such as Space, W or Up")]
    InvalidKey(String),
    #[error("property name `{0}` must contain only letters, digits or underscores")]
    InvalidProperty(String),
    #[error(
        "attribute name `{0}` must be 1 to 100 letters, digits or underscores, not starting with RBX"
    )]
    InvalidAttribute(String),
    #[error("only event blocks can be at the top level, and events cannot be nested")]
    InvalidEventPlacement,
    #[error("`{0}` isn't one of Luau's operators")]
    InvalidOperator(String),
    #[error("`{0}` isn't a name, or a name and fields such as `row.Binding.Text`")]
    InvalidName(String),
    #[error("an `if` value needs a condition and a value for each branch, then one more value")]
    InvalidIfValue,
    #[error("\"touching part\" only works inside a \"when touched\" block")]
    TouchingPartOutsideTouched,
    #[error("`{value}` isn't {what}; choose from {choices}")]
    InvalidChoice {
        what: &'static str,
        value: String,
        choices: String,
    },
}

/// Translates `script` to Lua. Unattached stacks are left out, as they
/// don't run.
pub fn translate(script: &BlockScript) -> Result<Translation, TranslateError> {
    let mut ids = HashSet::new();
    let global_vars: HashSet<String> = script
        .variables
        .iter()
        .filter(|def| def.global)
        .map(|def| def.name.clone())
        .collect();
    let var_defs: HashMap<&str, &VariableDef> = script
        .variables
        .iter()
        .map(|def| (def.name.as_str(), def))
        .collect();
    let (referenced_outputs, referenced_output_pins) = collect_referenced_outputs(&script.blocks);
    let clone_ids = clone_identifiers(script);
    let mut track_connections = false;
    let mut gate_ids = Vec::new();
    let mut cooldown_ids = Vec::new();
    script.for_each_block(&mut |block| match &block.kind {
        BlockKind::DestroyScript => track_connections = true,
        BlockKind::CooldownGate { .. } => gate_ids.push(block.id),
        BlockKind::WhenTouched {
            cooldown: Some(_), ..
        }
        | BlockKind::WhenPromptTriggered {
            cooldown: Some(_), ..
        }
        | BlockKind::WhenPartClicked {
            cooldown: Some(_), ..
        } => cooldown_ids.push(block.id),
        _ => {}
    });
    let mut writer = Writer {
        tabs: script.tabs,
        global_vars,
        referenced_outputs,
        referenced_output_pins,
        clone_ids,
        track_connections,
        ..Default::default()
    };
    // Everywhere at once, as a function's blocks are written separately.
    let mut every_id = HashSet::new();
    for block in &script.blocks {
        if !block.kind.is_event() {
            return Err(TranslateError::InvalidEventPlacement);
        }
        note_ids(block, &mut every_id)?;
    }

    // Variables are the whole script's, as in Scratch, so they're declared
    // once at the top, starting at their initial value (default 0).
    let mut variables = BTreeSet::new();
    let mut list_variables = HashSet::new();
    let mut services = BTreeSet::new();
    for def in &script.variables {
        variables.insert(def.name.clone());
    }
    script.for_each_block(&mut |block| {
        if let BlockKind::SetVariable { name, .. } | BlockKind::ChangeVariable { name, .. } =
            &block.kind
        {
            variables.insert(name.clone());
        }
        if let BlockKind::ListCreate { name, .. } = &block.kind {
            variables.insert(name.clone());
            list_variables.insert(name.clone());
        }
        match &block.kind {
            BlockKind::WhenKeyPressed { .. } | BlockKind::WhenKeyReleased { .. } => {
                services.insert("UserInputService");
            }
            BlockKind::WhenPlayerJoins { .. }
            | BlockKind::WhenPlayerLeaves { .. }
            | BlockKind::WhenCharacterSpawns { .. }
            | BlockKind::WhenPlayerDies { .. } => {
                services.insert("Players");
            }
            BlockKind::WhenSeatOccupied { .. } | BlockKind::WhenSeatVacated { .. } => {
                if writer.referenced_output_pins.contains(&(block.id, 1)) {
                    services.insert("Players");
                }
            }
            BlockKind::WhenAgentSenses { .. } | BlockKind::WhenAgentLosesPlayer { .. } => {
                if writer.referenced_outputs.contains(&block.id) {
                    services.insert("Players");
                }
            }
            BlockKind::WhenTouched { filter, .. } | BlockKind::WhenTouchEnds { filter, .. } => {
                if filter == "Players Only"
                    || writer.referenced_output_pins.contains(&(block.id, 1))
                {
                    services.insert("Players");
                }
                if filter == "Tagged / Named..." {
                    services.insert("CollectionService");
                }
            }
            BlockKind::ForEachPartInRadius { players_only, .. }
            | BlockKind::ForEachPartTouching { players_only, .. }
                if *players_only =>
            {
                services.insert("Players");
            }
            BlockKind::WhenTagAdded { .. }
            | BlockKind::WhenTagRemoved { .. }
            | BlockKind::ForEachTagged { .. } => {
                services.insert("CollectionService");
            }
            BlockKind::EveryFrame { .. }
            | BlockKind::BeforePhysics { .. }
            | BlockKind::BeforeRender { .. } => {
                services.insert("RunService");
            }
            BlockKind::DestroyAfter { .. } => {
                services.insert("Debris");
            }
            kind if kind.tweens() => {
                services.insert("TweenService");
            }
            kind if kind.saves() => {
                services.insert("SaveService");
            }
            kind if kind.levels() => {
                services.insert("LevelService");
            }
            kind if kind.networks() => {
                services.insert("NetworkService");
            }
            BlockKind::SendChatMessage { .. }
            | BlockKind::ShowSystemMessage { .. }
            | BlockKind::WhenChatMessageReceived { .. } => {
                services.insert("TextChatService");
            }
            BlockKind::ForEachRow { .. } => {
                services.insert("DataService");
            }
            BlockKind::WhenLayerChanged { .. } | BlockKind::SetLayerActive { .. } => {
                services.insert("DataLayerService");
            }
            BlockKind::WhenCommandRun { .. } => {
                services.insert("ConsoleService");
            }
            BlockKind::SetLanguage { .. } => {
                services.insert("LocalizationService");
            }
            kind if kind.sets_graphics() => {
                services.insert("UserGameSettings");
            }
            _ => {}
        }
        if block.kind.sets_lighting() {
            services.insert("Lighting");
        }
        for expression in block.kind.expressions() {
            visit_expressions(expression, &mut |expression| match expression {
                Expression::KeyDown { .. } => {
                    services.insert("UserInputService");
                }
                Expression::DataRow { .. } | Expression::DataTableRowCount { .. } => {
                    services.insert("DataService");
                }
                Expression::SlotExists { .. } => {
                    services.insert("SaveService");
                }
                Expression::IsHost | Expression::IsConnected => {
                    services.insert("NetworkService");
                }
                Expression::LayerActive { .. } => {
                    services.insert("DataLayerService");
                }
                Expression::MouseHitPosition
                | Expression::NearestPlayerTo { .. }
                | Expression::PlayerOfPart { .. } => {
                    services.insert("Players");
                }
                Expression::HasTag { .. } => {
                    services.insert("CollectionService");
                }
                _ => {}
            });
        }
    });
    read_variables(&script.blocks, &mut Vec::new(), &mut variables);
    if track_connections {
        variables.remove("_connections");
    }
    for service in &services {
        writer.line(format!("local {service} = game:GetService(\"{service}\")"));
    }
    if track_connections {
        writer.line("local _connections = {}");
    }
    let mut deferred_lists = Vec::new();
    for name in &variables {
        if !valid_identifier(name) {
            return Err(TranslateError::InvalidVariable(name.clone()));
        }
        let def = var_defs.get(name.as_str()).copied();
        let is_default_list = def.is_none() && list_variables.contains(name);
        if is_default_list {
            deferred_lists.push(name.clone());
            continue;
        }
        let initial = if let Some(def) = def {
            writer.expression(&def.initial)?
        } else {
            "0".to_owned()
        };
        if def.is_some_and(|d| d.global) {
            writer.line(format!(
                "if _G.{name} == nil then _G.{name} = {initial} end"
            ));
        } else {
            writer.line(format!("local {name} = {initial}"));
        }
    }
    for name in &deferred_lists {
        writer.line(format!("local {name} = {{}}"));
    }
    for id in &gate_ids {
        writer.line(format!("local _gate_{id} = -math.huge"));
    }
    for id in &cooldown_ids {
        writer.line(format!("local _cooldown_{id} = -math.huge"));
    }

    let typed = |block: &Block| {
        matches!(
            block.kind,
            BlockKind::RunScript { .. } | BlockKind::OnSignal { .. }
        )
    };
    let mut previous: Option<&Block> = None;
    for block in &script.blocks {
        if matches!(block.kind, BlockKind::Unattached { .. }) {
            continue;
        }
        // Stacks are apart, except where typed Lua wasn't.
        let apart = block.spaced || !(typed(block) || previous.is_some_and(typed));
        if apart && !writer.lines.is_empty() {
            writer.lines.push(String::new());
        }
        writer.write_event(block, &mut ids)?;
        previous = Some(block);
    }
    let source = writer.lines.join("\n") + if writer.lines.is_empty() { "" } else { "\n" };
    Ok(Translation {
        source,
        block_lines: writer.block_lines,
    })
}

/// Adds the variables `blocks` read, except a "for each child" loop's own
/// variable inside that loop (the loop declares it).
fn read_variables(blocks: &[Block], loops: &mut Vec<String>, variables: &mut BTreeSet<String>) {
    for block in blocks {
        for expression in block.kind.expressions() {
            visit_expressions(expression, &mut |expression| {
                if let Expression::Variable { name } = expression
                    && !loops.contains(name)
                {
                    variables.insert(name.clone());
                }
            });
        }
        let looping = match &block.kind {
            BlockKind::ForEachChild { variable, .. }
            | BlockKind::ForEachPartInRadius { variable, .. }
            | BlockKind::ForEachPartTouching { variable, .. } => Some(variable.clone()),
            _ => None,
        };
        let pushed = looping.is_some();
        loops.extend(looping);
        for body in block.kind.bodies() {
            read_variables(body, loops, variables);
        }
        if pushed {
            loops.pop();
        }
    }
}

pub(super) fn note_ids(block: &Block, ids: &mut HashSet<u64>) -> Result<(), TranslateError> {
    if !ids.insert(block.id) {
        return Err(TranslateError::DuplicateId(block.id));
    }
    for body in block.kind.bodies() {
        for child in body {
            note_ids(child, ids)?;
        }
    }
    Ok(())
}

pub(super) fn visit_expressions(expression: &Expression, visit: &mut impl FnMut(&Expression)) {
    visit(expression);
    for child in expression.children() {
        visit_expressions(child, visit);
    }
}

/// Base identifier for a cloned object (e.g. `"Workspace.Baseplate"` -> `"Baseplate"`).
pub fn clone_base_name(target: &ObjectRef) -> String {
    let raw = match target.as_expr() {
        Expression::ObjectPath { path } => path.rsplit('.').next().unwrap_or(""),
        Expression::Variable { name }
        | Expression::Name { name }
        | Expression::BlockOutput { name, .. } => name.as_str(),
        _ => "",
    };
    let mut sanitized: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() || sanitized.starts_with(|c: char| c.is_ascii_digit()) {
        sanitized.insert_str(0, "Object");
    }
    if !valid_identifier(&sanitized) {
        sanitized.push_str("_obj");
    }
    sanitized
}

/// Predictable `(single_name, array_name)` identifiers for every
/// [`BlockKind::CloneInto`] in `script`, incrementing `_2`, `_3`, ... when
/// multiple clones of the same base name appear in the same top-level scope.
pub fn clone_identifiers(script: &BlockScript) -> HashMap<u64, (String, String)> {
    fn walk(
        blocks: &[Block],
        counts: &mut HashMap<String, usize>,
        out: &mut HashMap<u64, (String, String)>,
    ) {
        for block in blocks {
            if let BlockKind::CloneInto { target, .. } = &block.kind {
                let base = clone_base_name(target);
                let count = counts.entry(base.clone()).or_insert(0);
                *count += 1;
                let suffix = if *count == 1 {
                    String::new()
                } else {
                    format!("_{count}")
                };
                out.insert(
                    block.id,
                    (
                        format!("{base}_cloned{suffix}"),
                        format!("{base}_clones{suffix}"),
                    ),
                );
            }
            for body in block.kind.bodies() {
                walk(body, counts, out);
            }
        }
    }
    let mut out = HashMap::new();
    for top in &script.blocks {
        let mut counts = HashMap::new();
        walk(std::slice::from_ref(top), &mut counts, &mut out);
    }
    out
}

/// All block IDs (and `(block_id, output_index)` pairs) whose right-side
/// output pins are referenced via [`Expression::BlockOutput`] or event context
/// reporters (`TriggeringPlayer`, `CommandArgs`, `DeltaTime`).
fn collect_referenced_outputs(blocks: &[Block]) -> (HashSet<u64>, HashSet<(u64, usize)>) {
    fn walk(
        blocks: &[Block],
        enclosing_event: Option<(u64, &BlockKind)>,
        ids: &mut HashSet<u64>,
        pins: &mut HashSet<(u64, usize)>,
    ) {
        for block in blocks {
            for expr in block.kind.expressions() {
                visit_expressions(expr, &mut |e| match e {
                    Expression::BlockOutput { block, output, .. } => {
                        ids.insert(*block);
                        pins.insert((*block, *output));
                    }
                    Expression::TriggeringPlayer => {
                        if let Some((ev_id, ev_kind)) = enclosing_event {
                            let pin = match ev_kind {
                                BlockKind::WhenTouched { .. }
                                | BlockKind::WhenTouchEnds { .. }
                                | BlockKind::WhenSeatOccupied { .. }
                                | BlockKind::WhenSeatVacated { .. }
                                | BlockKind::WhenCharacterSpawns { .. } => Some(1),
                                BlockKind::WhenPromptTriggered { .. }
                                | BlockKind::WhenPartClicked { .. }
                                | BlockKind::WhenCommandRun { .. }
                                | BlockKind::WhenPlayerJoins { .. }
                                | BlockKind::WhenPlayerLeaves { .. }
                                | BlockKind::WhenPlayerDies { .. }
                                | BlockKind::WhenAgentSenses { .. }
                                | BlockKind::WhenAgentLosesPlayer { .. }
                                | BlockKind::WhenRemoteFires { .. } => Some(0),
                                _ => None,
                            };
                            if let Some(p) = pin {
                                ids.insert(ev_id);
                                pins.insert((ev_id, p));
                            }
                        }
                    }
                    Expression::CommandArgs => {
                        if let Some((ev_id, BlockKind::WhenCommandRun { .. })) = enclosing_event {
                            ids.insert(ev_id);
                            pins.insert((ev_id, 1));
                        }
                    }
                    Expression::DeltaTime => {
                        if let Some((ev_id, ev_kind)) = enclosing_event {
                            let pin = match ev_kind {
                                BlockKind::EveryFrame { .. } | BlockKind::BeforeRender { .. } => {
                                    Some(0)
                                }
                                BlockKind::BeforePhysics { .. } => Some(1),
                                _ => None,
                            };
                            if let Some(p) = pin {
                                ids.insert(ev_id);
                                pins.insert((ev_id, p));
                            }
                        }
                    }
                    Expression::AnimatorNewState => {
                        if let Some((ev_id, BlockKind::WhenStateChanged { .. })) = enclosing_event {
                            ids.insert(ev_id);
                            pins.insert((ev_id, 0));
                        }
                    }
                    Expression::AnimatorPreviousState => {
                        if let Some((ev_id, BlockKind::WhenStateChanged { .. })) = enclosing_event {
                            ids.insert(ev_id);
                            pins.insert((ev_id, 1));
                        }
                    }
                    Expression::AnimationEventName => {
                        if let Some((ev_id, BlockKind::WhenAnimationEvent { .. })) = enclosing_event
                        {
                            ids.insert(ev_id);
                            pins.insert((ev_id, 0));
                        }
                    }
                    Expression::ConnectionFailureReason => {
                        if let Some((ev_id, BlockKind::WhenConnectionFails { .. })) =
                            enclosing_event
                        {
                            ids.insert(ev_id);
                            pins.insert((ev_id, 0));
                        }
                    }
                    Expression::ChangedLayerName => {
                        if let Some((ev_id, BlockKind::WhenLayerChanged { .. })) = enclosing_event {
                            ids.insert(ev_id);
                            pins.insert((ev_id, 0));
                        }
                    }
                    Expression::ChangedLayerActive => {
                        if let Some((ev_id, BlockKind::WhenLayerChanged { .. })) = enclosing_event {
                            ids.insert(ev_id);
                            pins.insert((ev_id, 1));
                        }
                    }
                    Expression::ChatMessageText => {
                        if let Some((ev_id, BlockKind::WhenChatMessageReceived { .. })) =
                            enclosing_event
                        {
                            ids.insert(ev_id);
                            pins.insert((ev_id, 0));
                        }
                    }
                    Expression::ChatMessageSender => {
                        if let Some((ev_id, BlockKind::WhenChatMessageReceived { .. })) =
                            enclosing_event
                        {
                            ids.insert(ev_id);
                            pins.insert((ev_id, 1));
                        }
                    }
                    _ => {}
                });
            }
            let next_event = if block.kind.is_event() {
                Some((block.id, &block.kind))
            } else {
                enclosing_event
            };
            for body in block.kind.bodies() {
                walk(body, next_event, ids, pins);
            }
        }
    }
    let mut ids = HashSet::new();
    let mut pins = HashSet::new();
    walk(blocks, None, &mut ids, &mut pins);
    (ids, pins)
}

/// Finds `BlockKind::Local` blocks nested inside control-flow bodies of `body`
/// whose output is referenced outside that nested body (so their `local`
/// declaration must be hoisted to the enclosing event/function scope).
fn find_hoisted_locals(body: &[Block]) -> Vec<(u64, String)> {
    fn collect_locals(blocks: &[Block], out: &mut Vec<(u64, String)>) {
        for block in blocks {
            if let BlockKind::Local { names, .. } = &block.kind {
                out.push((block.id, names.clone()));
            }
            for sub in block.kind.bodies() {
                collect_locals(sub, out);
            }
        }
    }
    fn collect_refs(blocks: &[Block], out: &mut HashSet<u64>) {
        for block in blocks {
            for expr in block.kind.expressions() {
                visit_expressions(expr, &mut |e| {
                    if let Expression::BlockOutput { block, .. } = e {
                        out.insert(*block);
                    }
                });
            }
            for sub in block.kind.bodies() {
                collect_refs(sub, out);
            }
        }
    }
    let mut hoisted = Vec::new();
    for (index, block) in body.iter().enumerate() {
        let mut nested_locals = Vec::new();
        for sub in block.kind.bodies() {
            collect_locals(sub, &mut nested_locals);
        }
        if nested_locals.is_empty() {
            continue;
        }
        let mut later_refs = HashSet::new();
        collect_refs(&body[index + 1..], &mut later_refs);
        for (id, names) in nested_locals {
            if later_refs.contains(&id) {
                hoisted.push((id, names));
            }
        }
    }
    hoisted
}

#[derive(Default)]
pub(super) struct Writer {
    pub(super) lines: Vec<String>,
    pub(super) block_lines: Vec<(usize, u64)>,
    pub(super) indent: usize,
    /// True inside "when touched", where `otherPart` exists.
    pub(super) in_touched: bool,
    /// Indent with tabs rather than four spaces.
    pub(super) tabs: bool,
    /// Variable names declared as Global / Shared (`_G.<name>`).
    pub(super) global_vars: HashSet<String>,
    /// Block IDs whose output pins are referenced by downstream expressions.
    pub(super) referenced_outputs: HashSet<u64>,
    /// Specific `(block_id, output_index)` output pins referenced downstream.
    pub(super) referenced_output_pins: HashSet<(u64, usize)>,
    /// Predictable `(single, array)` names per `CloneInto` block ID.
    pub(super) clone_ids: HashMap<u64, (String, String)>,
    /// `Local` block IDs whose `local` declaration was hoisted to the
    /// enclosing event/function scope.
    pub(super) hoisted_locals: HashSet<u64>,
    /// When true, top-level event connections are recorded in `_connections`
    /// so `DestroyScript` can disconnect them all before `script:Destroy()`.
    pub(super) track_connections: bool,
}

/// Starts a block's ID in the text of a function, marking the line as that
/// block's. A function is a value, so it's written before its line's place
/// in the script is known; [`Writer::line`] turns the marks into line numbers.
const LINE_MARK: char = '\u{1}';
const LINE_MARK_END: char = '\u{2}';

/// The Lua `block` is written as, on its own. For [`BlockKind::is_event`]
/// blocks that's the whole event. `touched` says it's inside "when touched".
pub fn block_lua(block: &Block, touched: bool) -> Result<String, TranslateError> {
    let mut writer = Writer {
        in_touched: touched,
        ..Default::default()
    };
    let mut ids = HashSet::new();
    if block.kind.is_event() {
        writer.write_event(block, &mut ids)?;
    } else {
        writer.write_body(std::slice::from_ref(block), &mut ids)?;
    }
    Ok(writer.lines.join("\n"))
}

/// The Lua `expression` is written as, on its own.
pub fn expression_lua(expression: &Expression, touched: bool) -> Result<String, TranslateError> {
    let writer = Writer {
        in_touched: touched,
        ..Default::default()
    };
    let text = writer.expression(expression)?;
    Ok(text
        .split('\n')
        .map(without_marks)
        .collect::<Vec<_>>()
        .join("\n"))
}

/// A line of a function's text without its block marks.
fn without_marks(line: &str) -> &str {
    let mut rest = line;
    while let Some(marked) = rest.strip_prefix(LINE_MARK) {
        rest = marked
            .split_once(LINE_MARK_END)
            .map_or(marked, |(_, after)| after);
    }
    rest
}

impl Writer {
    fn indent_unit(&self) -> &'static str {
        if self.tabs { "\t" } else { "    " }
    }

    /// Adds `text` at this indent, a line at a time. It can be several
    /// lines, if it has a function or a table of one field to a line in it.
    pub(super) fn line(&mut self, text: impl Into<String>) {
        let text = text.into();
        let indent = self.indent_unit().repeat(self.indent);
        for line in text.split('\n') {
            let bare = without_marks(line);
            let mut marks = &line[..line.len() - bare.len()];
            while let Some(marked) = marks.strip_prefix(LINE_MARK) {
                let Some((id, after)) = marked.split_once(LINE_MARK_END) else {
                    break;
                };
                if let Ok(id) = id.parse() {
                    self.block_lines.push((self.lines.len() + 1, id));
                }
                marks = after;
            }
            self.lines.push(format!("{indent}{bare}"));
        }
    }

    /// Notes that `id`'s code starts on the next line.
    pub(super) fn tag(&mut self, id: u64, ids: &mut HashSet<u64>) -> Result<(), TranslateError> {
        if !ids.insert(id) {
            return Err(TranslateError::DuplicateId(id));
        }
        self.block_lines.push((self.lines.len() + 1, id));
        Ok(())
    }

    fn open_connect(&mut self, line: String) -> &'static str {
        if self.track_connections && (line.contains(":Connect(") || line.contains(":Once(")) {
            self.line(format!("table.insert(_connections, {line}"));
            "end))"
        } else {
            self.line(line);
            "end)"
        }
    }

    fn write_cooldown_guard(
        &mut self,
        block_id: u64,
        cooldown: &Option<Expression>,
    ) -> Result<(), TranslateError> {
        if let Some(cd) = cooldown {
            let cd = self.expression(cd)?;
            self.line(format!(
                "if os.clock() - _cooldown_{block_id} < {cd} then return end"
            ));
            self.line(format!("_cooldown_{block_id} = os.clock()"));
        }
        Ok(())
    }

    fn write_touch_preamble(
        &mut self,
        block_id: u64,
        filter: &str,
        filter_value: &Option<Expression>,
        cooldown: &Option<Expression>,
    ) -> Result<(), TranslateError> {
        let needs_player =
            filter == "Players Only" || self.referenced_output_pins.contains(&(block_id, 1));
        if needs_player {
            self.line("local player = Players:GetPlayerFromCharacter(otherPart.Parent)");
        }
        if filter == "Players Only" {
            self.line("if not player then return end");
        } else if filter == "Tagged / Named..." {
            let val = match filter_value {
                Some(expr) => self.expression(expr)?,
                None => "\"\"".to_owned(),
            };
            self.line(format!(
                "if otherPart.Name ~= {val} and not CollectionService:HasTag(otherPart, {val}) then return end"
            ));
        }
        self.write_cooldown_guard(block_id, cooldown)?;
        Ok(())
    }

    pub(super) fn write_event(
        &mut self,
        block: &Block,
        ids: &mut HashSet<u64>,
    ) -> Result<(), TranslateError> {
        self.tag(block.id, ids)?;
        let (opening, body) = match &block.kind {
            BlockKind::WhenStarted { body } => ("task.spawn(function()".to_owned(), body),
            BlockKind::WhenTouched {
                part,
                filter,
                filter_value,
                cooldown,
                body,
            } => {
                self.in_touched = true;
                let close = self.open_connect(format!(
                    "{}.Touched:Connect(function(otherPart)",
                    self.target(part)?
                ));
                self.indent += 1;
                self.write_touch_preamble(block.id, filter, filter_value, cooldown)?;
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line(close);
                self.in_touched = false;
                return Ok(());
            }
            BlockKind::WhenTouchEnds {
                part,
                filter,
                filter_value,
                body,
            } => {
                self.in_touched = true;
                let close = self.open_connect(format!(
                    "{}.TouchEnded:Connect(function(otherPart)",
                    self.target(part)?
                ));
                self.indent += 1;
                self.write_touch_preamble(block.id, filter, filter_value, &None)?;
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line(close);
                self.in_touched = false;
                return Ok(());
            }
            BlockKind::WhenKeyPressed { key: name, body } => {
                let referenced = self.referenced_outputs.contains(&block.id);
                let params = if referenced {
                    "input, gameProcessed"
                } else {
                    "input"
                };
                let close = self.open_connect(format!(
                    "UserInputService.InputBegan:Connect(function({params})"
                ));
                self.indent += 1;
                if name == "Any" || name.is_empty() {
                    if referenced {
                        self.line("local keyCode = input.KeyCode");
                    }
                    self.write_body(body, ids)?;
                } else {
                    let key = key(name)?;
                    self.line(format!("if input.KeyCode == {key} then"));
                    self.indent += 1;
                    if referenced {
                        self.line("local keyCode = input.KeyCode");
                    }
                    self.write_body(body, ids)?;
                    self.indent -= 1;
                    self.line("end");
                }
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenKeyReleased { key: name, body } => {
                let referenced = self.referenced_outputs.contains(&block.id);
                let params = if referenced {
                    "input, gameProcessed"
                } else {
                    "input"
                };
                let close = self.open_connect(format!(
                    "UserInputService.InputEnded:Connect(function({params})"
                ));
                self.indent += 1;
                if name == "Any" || name.is_empty() {
                    if referenced {
                        self.line("local keyCode = input.KeyCode");
                    }
                    self.write_body(body, ids)?;
                } else {
                    let key = key(name)?;
                    self.line(format!("if input.KeyCode == {key} then"));
                    self.indent += 1;
                    if referenced {
                        self.line("local keyCode = input.KeyCode");
                    }
                    self.write_body(body, ids)?;
                    self.indent -= 1;
                    self.line("end");
                }
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenPlayerJoins { body } => (
                "Players.PlayerAdded:Connect(function(player)".to_owned(),
                body,
            ),
            BlockKind::WhenPlayerLeaves { body } => (
                "Players.PlayerRemoving:Connect(function(player)".to_owned(),
                body,
            ),
            BlockKind::WhenCharacterSpawns { body } => {
                let close =
                    self.open_connect("Players.PlayerAdded:Connect(function(player)".to_owned());
                self.indent += 1;
                let inner_close = self
                    .open_connect("player.CharacterAdded:Connect(function(character)".to_owned());
                self.nested(body, ids)?;
                self.line(inner_close);
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenPlayerDies { body } => {
                let close =
                    self.open_connect("Players.PlayerAdded:Connect(function(player)".to_owned());
                self.indent += 1;
                let char_close = self
                    .open_connect("player.CharacterAdded:Connect(function(character)".to_owned());
                self.indent += 1;
                self.line("local _hum = character:FindFirstChildOfClass(\"Humanoid\") or character:FindFirstChildOfClass(\"Health\")");
                self.line("if _hum then");
                self.indent += 1;
                let died_close = self.open_connect("_hum.Died:Connect(function()".to_owned());
                self.nested(body, ids)?;
                self.line(died_close);
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line(char_close);
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenChildAdded { target, body } => (
                format!(
                    "{}.ChildAdded:Connect(function(child)",
                    self.container(target)?
                ),
                body,
            ),
            BlockKind::WhenChildRemoved { target, body } => (
                format!(
                    "{}.ChildRemoved:Connect(function(child)",
                    self.container(target)?
                ),
                body,
            ),
            BlockKind::WhenTagAdded { tag, body } => (
                format!(
                    "CollectionService:GetInstanceAddedSignal({}):Connect(function(instance)",
                    self.expression(tag)?
                ),
                body,
            ),
            BlockKind::WhenTagRemoved { tag, body } => (
                format!(
                    "CollectionService:GetInstanceRemovedSignal({}):Connect(function(instance)",
                    self.expression(tag)?
                ),
                body,
            ),
            BlockKind::WhenRemoteFires { remote, side, body } => {
                let target = self.target(remote)?;
                let opening = if side.starts_with("Client") {
                    format!("{target}.OnClientEvent:Connect(function(data)")
                } else {
                    format!("{target}.OnServerEvent:Connect(function(player, data)")
                };
                (opening, body)
            }
            BlockKind::WhenClicked { button, body } => (
                format!("{}.Activated:Connect(function()", self.target(button)?),
                body,
            ),
            BlockKind::WhenMouseEnters { element, body } => (
                format!("{}.MouseEnter:Connect(function()", self.target(element)?),
                body,
            ),
            BlockKind::WhenMouseLeaves { element, body } => (
                format!("{}.MouseLeave:Connect(function()", self.target(element)?),
                body,
            ),
            BlockKind::WhenValueChanged { widget, body } => {
                let params = if self.referenced_outputs.contains(&block.id) {
                    "newValue"
                } else {
                    ""
                };
                (
                    format!(
                        "{}.ValueChanged:Connect(function({params})",
                        self.target(widget)?
                    ),
                    body,
                )
            }
            BlockKind::EveryFrame { body } => (
                "RunService.Heartbeat:Connect(function(deltaTime)".to_owned(),
                body,
            ),
            BlockKind::BeforePhysics { body } => (
                "RunService.Stepped:Connect(function(time, deltaTime)".to_owned(),
                body,
            ),
            BlockKind::BeforeRender { body } => (
                "RunService.RenderStepped:Connect(function(deltaTime)".to_owned(),
                body,
            ),
            BlockKind::WhenEventFired { event, body } => {
                let params = if self.referenced_outputs.contains(&block.id) {
                    "data"
                } else {
                    ""
                };
                (
                    format!("{}.Event:Connect(function({params})", stored(event)?),
                    body,
                )
            }
            BlockKind::WhenSoundEnded { sound, body } => (
                format!("{}.Ended:Connect(function()", self.target(sound)?),
                body,
            ),
            BlockKind::WhenCutsceneFinished { sequence, body } => (
                format!("{}.Completed:Connect(function()", self.target(sequence)?),
                body,
            ),
            BlockKind::WhenConnectionFails { body } => (
                "NetworkService.ConnectionFailed:Connect(function(reason)".to_owned(),
                body,
            ),
            BlockKind::WhenChatMessageReceived { body } => (
                "TextChatService.MessageReceived:Connect(function(message)".to_owned(),
                body,
            ),
            BlockKind::WhenStreamedIn {
                target: instance,
                body,
            } => (
                format!("{}.StreamedIn:Connect(function()", self.target(instance)?),
                body,
            ),
            BlockKind::WhenStreamedOut {
                target: instance,
                body,
            } => (
                format!("{}.StreamedOut:Connect(function()", self.target(instance)?),
                body,
            ),
            BlockKind::WhenStateChanged { animator, body } => (
                format!(
                    "{}.StateChanged:Connect(function(state, previous)",
                    self.target(animator)?
                ),
                body,
            ),
            BlockKind::WhenAnimationEvent {
                animator,
                event,
                body,
            } => {
                let close = self.open_connect(format!(
                    "{}.AnimationEvent:Connect(function(name)",
                    self.target(animator)?
                ));
                self.indent += 1;
                if event.is_empty() || event == "Any" {
                    self.write_body(body, ids)?;
                } else {
                    self.line(format!("if name == {} then", quote(event)));
                    self.nested(body, ids)?;
                    self.line("end");
                }
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenAnimationFinished {
                animator,
                animation,
                body,
            } => (
                format!(
                    "{}:LoadAnimation({}).Stopped:Connect(function()",
                    self.target(animator)?,
                    self.target(animation)?
                ),
                body,
            ),
            BlockKind::WhenSplineReached {
                follower,
                which,
                body,
            } => {
                let close = self.open_connect(format!(
                    "{}.Reached:Connect(function(which)",
                    self.target(follower)?
                ));
                self.indent += 1;
                if which == "Any" {
                    self.write_body(body, ids)?;
                } else {
                    self.line(format!("if which == {} then", quote(which)));
                    self.nested(body, ids)?;
                    self.line("end");
                }
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenAgentSenses { agent, sense, body } => {
                let close = self.open_connect(format!(
                    "{}.PlayerSensed:Connect(function(sense)",
                    self.target(agent)?
                ));
                self.indent += 1;
                let need_player = self.referenced_outputs.contains(&block.id);
                let need_char = self.referenced_output_pins.contains(&(block.id, 1));
                if sense == "Any" {
                    if need_player {
                        self.line("local player = Players.LocalPlayer or Players:GetPlayers()[1]");
                    }
                    if need_char {
                        self.line("local character = player and player.Character");
                    }
                    self.write_body(body, ids)?;
                } else {
                    self.line(format!("if sense == {} then", quote(sense)));
                    self.indent += 1;
                    if need_player {
                        self.line("local player = Players.LocalPlayer or Players:GetPlayers()[1]");
                    }
                    if need_char {
                        self.line("local character = player and player.Character");
                    }
                    self.write_body(body, ids)?;
                    self.indent -= 1;
                    self.line("end");
                }
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenAgentLosesPlayer { agent, sense, body } => {
                let close = self.open_connect(format!(
                    "{}.PlayerLost:Connect(function(sense)",
                    self.target(agent)?
                ));
                self.indent += 1;
                let need_player = self.referenced_outputs.contains(&block.id);
                let need_char = self.referenced_output_pins.contains(&(block.id, 1));
                if sense == "Any" {
                    if need_player {
                        self.line("local player = Players.LocalPlayer or Players:GetPlayers()[1]");
                    }
                    if need_char {
                        self.line("local character = player and player.Character");
                    }
                    self.write_body(body, ids)?;
                } else {
                    self.line(format!("if sense == {} then", quote(sense)));
                    self.indent += 1;
                    if need_player {
                        self.line("local player = Players.LocalPlayer or Players:GetPlayers()[1]");
                    }
                    if need_char {
                        self.line("local character = player and player.Character");
                    }
                    self.write_body(body, ids)?;
                    self.indent -= 1;
                    self.line("end");
                }
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenAgentArrives { agent, body } => {
                let close = self.open_connect(format!(
                    "{}.DestinationReached:Connect(function()",
                    self.target(agent)?
                ));
                self.nested(body, ids)?;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenPromptTriggered {
                prompt,
                cooldown,
                body,
            } => {
                let params = if self.referenced_outputs.contains(&block.id) {
                    "player"
                } else {
                    ""
                };
                let close = self.open_connect(format!(
                    "{}.Triggered:Connect(function({params})",
                    self.target(prompt)?
                ));
                self.indent += 1;
                self.write_cooldown_guard(block.id, cooldown)?;
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenPartClicked {
                detector,
                cooldown,
                body,
            } => {
                let params = if self.referenced_outputs.contains(&block.id) {
                    "player"
                } else {
                    ""
                };
                let close = self.open_connect(format!(
                    "{}.MouseClick:Connect(function({params})",
                    self.target(detector)?
                ));
                self.indent += 1;
                self.write_cooldown_guard(block.id, cooldown)?;
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenCommandRun {
                command,
                help,
                admin_only,
                body,
            } => {
                let command = self.expression(command)?;
                let help = self.expression(help)?;
                let referenced = self.referenced_outputs.contains(&block.id);
                let has_admin = admin_only.is_some();
                let params = if referenced || has_admin {
                    "player, ..."
                } else {
                    ""
                };
                self.line(format!(
                    "ConsoleService:RegisterCommand({command}, {help}, function({params})"
                ));
                self.indent += 1;
                if let Some(admin) = admin_only {
                    let admin = self.expression(admin)?;
                    self.line(format!(
                        "if ({admin}) and not (player and player:GetAttribute(\"IsAdmin\")) then return end"
                    ));
                }
                if self.referenced_output_pins.contains(&(block.id, 1)) {
                    self.line("local args = { ... }");
                }
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line("end)");
                return Ok(());
            }
            BlockKind::WhenPropertyChanged {
                target: t,
                property,
                body,
            } => {
                if !valid_identifier(property) {
                    return Err(TranslateError::InvalidProperty(property.clone()));
                }
                let target = self.target(t)?;
                let close = self.open_connect(format!(
                    "{target}:GetPropertyChangedSignal({}):Connect(function()",
                    quote(property)
                ));
                self.indent += 1;
                if self.referenced_outputs.contains(&block.id) {
                    self.line(format!("local newValue = {target}.{property}"));
                }
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenAttributeChanged {
                target: t,
                attribute,
                body,
            } => {
                let target = self.target(t)?;
                let attr = attribute_name(attribute)?;
                let close = self.open_connect(format!(
                    "{target}:GetAttributeChangedSignal({attr}):Connect(function()"
                ));
                self.indent += 1;
                if self.referenced_outputs.contains(&block.id) {
                    self.line(format!("local newValue = {target}:GetAttribute({attr})"));
                }
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenSeatOccupied { seat, body } => {
                let seat = self.target(seat)?;
                let close = self.open_connect(format!(
                    "{seat}:GetPropertyChangedSignal(\"Occupant\"):Connect(function()"
                ));
                self.indent += 1;
                self.line(format!("if {seat}.Occupant then"));
                self.indent += 1;
                let need_occupant = self.referenced_outputs.contains(&block.id);
                let need_player = self.referenced_output_pins.contains(&(block.id, 1));
                if need_occupant {
                    self.line(format!("local occupant = {seat}.Occupant"));
                }
                if need_player {
                    self.line("local player = Players:GetPlayerFromCharacter(occupant.Parent)");
                }
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::WhenSeatVacated { seat, body } => {
                let seat = self.target(seat)?;
                self.line("do");
                self.indent += 1;
                self.line(format!("local _prevOccupant = {seat}.Occupant"));
                let close = self.open_connect(format!(
                    "{seat}:GetPropertyChangedSignal(\"Occupant\"):Connect(function()"
                ));
                self.indent += 1;
                self.line(format!("local _curr = {seat}.Occupant"));
                self.line("if _prevOccupant and not _curr then");
                self.indent += 1;
                let need_occupant = self.referenced_outputs.contains(&block.id);
                let need_player = self.referenced_output_pins.contains(&(block.id, 1));
                if need_occupant {
                    self.line("local occupant = _prevOccupant");
                }
                if need_player {
                    self.line("local player = Players:GetPlayerFromCharacter(occupant.Parent)");
                }
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line("end");
                self.line("_prevOccupant = _curr");
                self.indent -= 1;
                self.line(close);
                self.indent -= 1;
                self.line("end");
                return Ok(());
            }
            BlockKind::WhenLayerChanged { data_layer, body } => {
                let close = self.open_connect(
                    "DataLayerService.LayerChanged:Connect(function(layerName, active)".to_owned(),
                );
                self.indent += 1;
                if data_layer.is_empty() || data_layer == "Any" {
                    self.write_body(body, ids)?;
                } else {
                    self.line(format!("if layerName == {} then", quote(data_layer)));
                    self.nested(body, ids)?;
                    self.line("end");
                }
                self.indent -= 1;
                self.line(close);
                return Ok(());
            }
            BlockKind::TopCode { code } => {
                self.code(code);
                return Ok(());
            }
            BlockKind::RunScript { body } => {
                self.write_body(body, ids)?;
                return Ok(());
            }
            BlockKind::OnSignal {
                signal,
                method,
                params,
                body,
            } => {
                self.write_connect(block.id, signal, method, params, body, ids)?;
                return Ok(());
            }
            _ => return Err(TranslateError::InvalidEventPlacement),
        };
        let close = self.open_connect(opening);
        self.indent += 1;
        self.write_body(body, ids)?;
        self.indent -= 1;
        self.line(close);
        self.in_touched = false;
        Ok(())
    }

    /// Lua kept from a typed script, one line at a time at this indent.
    pub(super) fn code(&mut self, code: &str) {
        for line in code.lines() {
            if line.trim().is_empty() {
                self.lines.push(String::new());
            } else {
                self.line(line);
            }
        }
    }

    pub(super) fn nested(
        &mut self,
        body: &[Block],
        ids: &mut HashSet<u64>,
    ) -> Result<(), TranslateError> {
        self.indent += 1;
        self.write_body(body, ids)?;
        self.indent -= 1;
        Ok(())
    }

    pub(super) fn write_body(
        &mut self,
        body: &[Block],
        ids: &mut HashSet<u64>,
    ) -> Result<(), TranslateError> {
        for (hoisted_id, names) in find_hoisted_locals(body) {
            if self.hoisted_locals.insert(hoisted_id) {
                self.line(format!("local {names}"));
            }
        }
        for (index, block) in body.iter().enumerate() {
            if block.kind.shape() != BlockShape::Statement {
                return Err(TranslateError::InvalidEventPlacement);
            }
            if let BlockKind::Comment { text } = &block.kind
                && block.chained
                && let Some(last) = self.lines.last_mut()
            {
                last.push(' ');
                last.push_str(text);
                continue;
            }
            if block.spaced && index > 0 {
                self.lines.push(String::new());
            }
            let first_line = self.lines.len();
            self.tag(block.id, ids)?;
            self.write_statement(block.id, &block.kind, ids)?;
            if index > 0 {
                self.separate_from_previous(first_line);
            }
        }
        Ok(())
    }

    pub(super) fn write_statement(
        &mut self,
        block_id: u64,
        kind: &BlockKind,
        ids: &mut HashSet<u64>,
    ) -> Result<(), TranslateError> {
        match kind {
            BlockKind::Wait { seconds } => {
                let seconds = self.expression(seconds)?;
                if self.referenced_outputs.contains(&block_id) {
                    self.line(format!("local elapsed_{block_id} = task.wait({seconds})"));
                } else {
                    self.line(format!("task.wait({seconds})"));
                }
            }
            BlockKind::WaitForEvent { target: t, event } => {
                if !valid_identifier(event) {
                    return Err(TranslateError::InvalidProperty(event.clone()));
                }
                let call = format!("{}.{event}:Wait()", self.target(t)?);
                if self.referenced_outputs.contains(&block_id) {
                    self.line(format!("local event_{block_id} = {call}"));
                } else {
                    self.line(call);
                }
            }
            BlockKind::WaitUntil { condition } => {
                let condition = self.expression(condition)?;
                self.line(format!("while not ({condition}) do"));
                self.indent += 1;
                self.line("task.wait()");
                self.indent -= 1;
                self.line("end");
            }
            // `_` is Lua's name for a counter nothing reads.
            BlockKind::Repeat { count, body } => {
                let count = self.expression(count)?;
                if self.referenced_outputs.contains(&block_id) {
                    self.line(format!("for i_{block_id} = 1, {count} do"));
                } else {
                    self.line(format!("for _ = 1, {count} do"));
                }
                self.nested(body, ids)?;
                self.line("end");
            }
            // Loops that may run for a long time wait a frame each time
            // round, as Scratch's do, so the game keeps running.
            BlockKind::Forever { body } => {
                self.line("while true do");
                self.nested(body, ids)?;
                self.indent += 1;
                self.line("task.wait()");
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::RepeatUntil { condition, body } => {
                let condition = self.expression(condition)?;
                self.line("repeat");
                self.nested(body, ids)?;
                self.indent += 1;
                self.line("task.wait()");
                self.indent -= 1;
                self.line(format!("until {condition}"));
            }
            BlockKind::ForRange {
                start,
                end,
                step,
                body,
            } => {
                let start = self.expression(start)?;
                let end = self.expression(end)?;
                let step = self.expression(step)?;
                let var = if self.referenced_outputs.contains(&block_id) {
                    format!("i_{block_id}")
                } else {
                    "i".to_owned()
                };
                self.line(format!("for {var} = {start}, {end}, {step} do"));
                self.nested(body, ids)?;
                self.line("end");
            }
            BlockKind::ForEachInList { list, body } => {
                let list = self.expression(list)?;
                let (idx, item) = if self.referenced_outputs.contains(&block_id) {
                    (format!("i_{block_id}"), format!("item_{block_id}"))
                } else {
                    ("_".to_owned(), "item".to_owned())
                };
                self.line(format!("for {idx}, {item} in ipairs({list}) do"));
                self.nested(body, ids)?;
                self.line("end");
            }
            BlockKind::ForEachTagged { tag, body } => {
                let tag = self.expression(tag)?;
                let (idx, inst) = if self.referenced_outputs.contains(&block_id) {
                    (format!("i_{block_id}"), format!("tagged_{block_id}"))
                } else {
                    ("_".to_owned(), "instance".to_owned())
                };
                self.line(format!(
                    "for {idx}, {inst} in CollectionService:GetTagged({tag}) do"
                ));
                self.nested(body, ids)?;
                self.line("end");
            }
            BlockKind::Sequence { branches } => {
                for branch in branches {
                    self.write_body(branch, ids)?;
                }
            }
            BlockKind::CooldownGate { cooldown, body } => {
                let is_do_once = matches!(cooldown, Expression::Number { value } if *value <= 0.0);
                if is_do_once {
                    self.line(format!("if _gate_{block_id} < 0 then"));
                    self.indent += 1;
                    self.line(format!("_gate_{block_id} = 1"));
                    self.write_body(body, ids)?;
                    self.indent -= 1;
                    self.line("end");
                } else {
                    let cd = self.expression(cooldown)?;
                    self.line(format!("if os.clock() - _gate_{block_id} >= {cd} then"));
                    self.indent += 1;
                    self.line(format!("_gate_{block_id} = os.clock()"));
                    self.write_body(body, ids)?;
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::SpawnAsync { delay, body } => {
                let is_immediate = matches!(delay, Expression::Number { value } if *value <= 0.0);
                if is_immediate {
                    self.line("task.spawn(function()");
                } else {
                    let delay = self.expression(delay)?;
                    self.line(format!("task.delay({delay}, function()"));
                }
                self.nested(body, ids)?;
                self.line("end)");
            }
            BlockKind::ForEachChild {
                target: t,
                variable,
                body,
            } => {
                if !valid_identifier(variable) {
                    return Err(TranslateError::InvalidVariable(variable.clone()));
                }
                let object = self.container(t)?;
                let idx = if self.referenced_output_pins.contains(&(block_id, 1)) {
                    format!("i_{block_id}")
                } else {
                    "_".to_owned()
                };
                self.line(format!(
                    "for {idx}, {variable} in {object}:GetChildren() do"
                ));
                self.nested(body, ids)?;
                self.line("end");
            }
            BlockKind::ForEachPartInRadius {
                center,
                radius,
                variable,
                players_only,
                ignore,
                body,
            } => {
                if !valid_identifier(variable) {
                    return Err(TranslateError::InvalidVariable(variable.clone()));
                }
                let idx = if self.referenced_output_pins.contains(&(block_id, 1)) {
                    format!("i_{block_id}")
                } else {
                    "_".to_owned()
                };
                self.line(format!(
                    "for {idx}, {variable} in workspace:GetPartBoundsInRadius({}, {}) do",
                    self.expression(center)?,
                    self.expression(radius)?
                ));
                self.indent += 1;
                if let Some(ign) = ignore {
                    let ign = self.expression(ign)?;
                    self.line(format!(
                        "if {variable} == {ign} or {variable}:IsDescendantOf({ign}) then continue end"
                    ));
                }
                if *players_only {
                    self.line(format!(
                        "if not Players:GetPlayerFromCharacter({variable}.Parent) then continue end"
                    ));
                }
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::ForEachPartTouching {
                target: t,
                variable,
                players_only,
                ignore,
                body,
            } => {
                if !valid_identifier(variable) {
                    return Err(TranslateError::InvalidVariable(variable.clone()));
                }
                let idx = if self.referenced_output_pins.contains(&(block_id, 1)) {
                    format!("i_{block_id}")
                } else {
                    "_".to_owned()
                };
                self.line(format!(
                    "for {idx}, {variable} in workspace:GetPartsInPart({}) do",
                    self.target(t)?
                ));
                self.indent += 1;
                if let Some(ign) = ignore {
                    let ign = self.expression(ign)?;
                    self.line(format!(
                        "if {variable} == {ign} or {variable}:IsDescendantOf({ign}) then continue end"
                    ));
                }
                if *players_only {
                    self.line(format!(
                        "if not Players:GetPlayerFromCharacter({variable}.Parent) then continue end"
                    ));
                }
                self.write_body(body, ids)?;
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::WaitForChild {
                target: t,
                name,
                timeout,
            } => {
                let call = match timeout {
                    Some(secs) => format!(
                        "{}:WaitForChild({}, {})",
                        self.container(t)?,
                        quote(name),
                        self.expression(secs)?
                    ),
                    None => format!("{}:WaitForChild({})", self.container(t)?, quote(name)),
                };
                if self.referenced_outputs.contains(&block_id) {
                    self.line(format!("local child_{block_id} = {call}"));
                } else {
                    self.line(call);
                }
            }
            BlockKind::DestroyAfter { target: t, seconds } => {
                let seconds = self.expression(seconds)?;
                self.line(format!("Debris:AddItem({}, {seconds})", self.target(t)?));
            }
            BlockKind::If { condition, body } => {
                let condition = self.expression(condition)?;
                self.line(format!("if {condition} then"));
                self.nested(body, ids)?;
                self.line("end");
            }
            BlockKind::IfElse {
                condition,
                then_body,
                else_body,
            } => {
                let condition = self.expression(condition)?;
                self.line(format!("if {condition} then"));
                self.nested(then_body, ids)?;
                let otherwise = self.write_elseifs(else_body, ids)?;
                if !otherwise.is_empty() {
                    self.line("else");
                    self.nested(otherwise, ids)?;
                }
                self.line("end");
            }
            // `do return end` can go anywhere; a bare `return` must be last.
            BlockKind::StopScript => self.line("do return end"),
            BlockKind::DestroyScript => {
                self.line("do");
                self.indent += 1;
                self.line("for _, _conn in _connections do");
                self.indent += 1;
                self.line("_conn:Disconnect()");
                self.indent -= 1;
                self.line("end");
                self.line("script:Destroy()");
                self.line("return");
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::Disconnect { connection } => {
                let conn = self.expression(connection)?;
                self.line(format!("{conn}:Disconnect()"));
            }
            BlockKind::MoveTo {
                target: t,
                position,
            } => {
                let (object, value) = (self.target(t)?, self.expression(position)?);
                self.line(format!(
                    "{object}:PivotTo(CFrame.new({value}) * {object}:GetPivot().Rotation)"
                ));
            }
            BlockKind::MoveBy { target: t, offset } => {
                let (object, value) = (self.target(t)?, self.expression(offset)?);
                self.line(format!("{object}:PivotTo({object}:GetPivot() + {value})"));
            }
            BlockKind::LookAt {
                target: t,
                position,
            } => {
                let (object, value) = (self.target(t)?, self.expression(position)?);
                self.line(format!(
                    "{object}.CFrame = CFrame.lookAt({object}.Position, {value})"
                ));
            }
            BlockKind::MoveForward {
                target: t,
                distance,
            } => {
                let (object, value) = (self.target(t)?, self.operand(distance, "*", true)?);
                self.line(format!(
                    "{object}.Position = {object}.Position + {object}.CFrame.LookVector * {value}"
                ));
            }
            BlockKind::TurnTo {
                target: t,
                orientation,
            } => {
                let value = self.expression(orientation)?;
                self.line(format!("{}.Orientation = {value}", self.target(t)?));
            }
            BlockKind::TurnBy {
                target: t,
                rotation,
                space,
            } => {
                let object = self.target(t)?;
                let world = space == "World";
                if let Expression::Vector3 { x, y, z } = rotation
                    && x.is_finite()
                    && y.is_finite()
                    && z.is_finite()
                {
                    let angles = format!(
                        "CFrame.Angles(math.rad({}), math.rad({}), math.rad({}))",
                        number(*x),
                        number(*y),
                        number(*z)
                    );
                    if world {
                        self.line(format!(
                            "{object}:PivotTo(CFrame.new({object}:GetPivot().Position) * {angles} * {object}:GetPivot().Rotation)"
                        ));
                    } else {
                        self.line(format!("{object}:PivotTo({object}:GetPivot() * {angles})"));
                    }
                } else {
                    let value = self.expression(rotation)?;
                    self.line("do");
                    self.indent += 1;
                    self.line(format!("local _rot = {value}"));
                    let angles =
                        "CFrame.Angles(math.rad(_rot.X), math.rad(_rot.Y), math.rad(_rot.Z))";
                    if world {
                        self.line(format!(
                            "{object}:PivotTo(CFrame.new({object}:GetPivot().Position) * {angles} * {object}:GetPivot().Rotation)"
                        ));
                    } else {
                        self.line(format!("{object}:PivotTo({object}:GetPivot() * {angles})"));
                    }
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::SlideTo {
                target: t,
                position,
                duration,
                easing,
                wait_until_done,
                completed,
            } => {
                let object = self.target(t)?;
                let pos = self.expression(position)?;
                let dur = self.expression(duration)?;
                let style = choice(easing, &EASING_STYLES, "an easing style")?;
                let goal =
                    format!("{{ CFrame = CFrame.new({pos}) * {object}:GetPivot().Rotation }}");
                let create = format!(
                    "TweenService:Create({object}, TweenInfo.new({dur}, Enum.EasingStyle.{style}), {goal})"
                );
                if !wait_until_done && completed.is_empty() {
                    self.line(format!("{create}:Play()"));
                } else {
                    self.line("do");
                    self.indent += 1;
                    self.line(format!("local _tween = {create}"));
                    if !completed.is_empty() && !wait_until_done {
                        let close =
                            self.open_connect("_tween.Completed:Connect(function()".to_owned());
                        self.nested(completed, ids)?;
                        self.line(close);
                    }
                    self.line("_tween:Play()");
                    if *wait_until_done {
                        self.line("_tween.Completed:Wait()");
                        if !completed.is_empty() {
                            self.write_body(completed, ids)?;
                        }
                    }
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::RotateTo {
                target: t,
                rotation,
                duration,
                easing,
                wait_until_done,
                completed,
            } => {
                let object = self.target(t)?;
                let dur = self.expression(duration)?;
                let style = choice(easing, &EASING_STYLES, "an easing style")?;
                let angles_inline = match rotation {
                    Expression::Vector3 { x, y, z }
                        if x.is_finite() && y.is_finite() && z.is_finite() =>
                    {
                        Some(format!(
                            "CFrame.Angles(math.rad({}), math.rad({}), math.rad({}))",
                            number(*x),
                            number(*y),
                            number(*z)
                        ))
                    }
                    _ => None,
                };
                if let Some(ref angles) = angles_inline
                    && !wait_until_done
                    && completed.is_empty()
                {
                    let goal = format!(
                        "{{ CFrame = CFrame.new({object}:GetPivot().Position) * {angles} }}"
                    );
                    self.line(format!(
                        "TweenService:Create({object}, TweenInfo.new({dur}, Enum.EasingStyle.{style}), {goal}):Play()"
                    ));
                } else {
                    self.line("do");
                    self.indent += 1;
                    let angles = if let Some(a) = angles_inline {
                        a
                    } else {
                        let rot = self.expression(rotation)?;
                        self.line(format!("local _rot = {rot}"));
                        "CFrame.Angles(math.rad(_rot.X), math.rad(_rot.Y), math.rad(_rot.Z))"
                            .to_owned()
                    };
                    let goal = format!(
                        "{{ CFrame = CFrame.new({object}:GetPivot().Position) * {angles} }}"
                    );
                    self.line(format!(
                        "local _tween = TweenService:Create({object}, TweenInfo.new({dur}, Enum.EasingStyle.{style}), {goal})"
                    ));
                    if !completed.is_empty() && !wait_until_done {
                        let close =
                            self.open_connect("_tween.Completed:Connect(function()".to_owned());
                        self.nested(completed, ids)?;
                        self.line(close);
                    }
                    self.line("_tween:Play()");
                    if *wait_until_done {
                        self.line("_tween.Completed:Wait()");
                        if !completed.is_empty() {
                            self.write_body(completed, ids)?;
                        }
                    }
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::SetGravity {
                target: t,
                multiplier,
            } => {
                let object = self.target(t)?;
                let mult = self.operand(multiplier, "-", true)?;
                self.line("do");
                self.indent += 1;
                self.line(format!(
                    "local _att = {object}:FindFirstChild(\"_GravityAttachment\") or Instance.new(\"Attachment\")"
                ));
                self.line("_att.Name = \"_GravityAttachment\"");
                self.line(format!("_att.Parent = {object}"));
                self.line(format!(
                    "local _vf = {object}:FindFirstChild(\"_GravityForce\") or Instance.new(\"VectorForce\")"
                ));
                self.line("_vf.Name = \"_GravityForce\"");
                self.line("_vf.Attachment0 = _att");
                self.line("_vf.RelativeTo = Enum.ActuatorRelativeTo.World");
                self.line("_vf.ApplyAtCenterOfMass = true");
                self.line(format!(
                    "_vf.Force = Vector3.new(0, {object}.AssemblyMass * workspace.Gravity * (1 - {mult}), 0)"
                ));
                self.line(format!("_vf.Parent = {object}"));
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::SetAngularVelocity {
                target: t,
                velocity,
            } => {
                let value = self.expression(velocity)?;
                self.line(format!(
                    "{}.AssemblyAngularVelocity = {value}",
                    self.target(t)?
                ));
            }
            BlockKind::LaunchPart {
                target: t,
                direction,
                speed,
            } => {
                let object = self.target(t)?;
                let dir = self.expression(direction)?;
                let spd = self.operand(speed, "*", true)?;
                self.line("do");
                self.indent += 1;
                self.line(format!("local _dir = {dir}"));
                self.line("if _dir.Magnitude > 0 then");
                self.indent += 1;
                self.line(format!(
                    "{object}.AssemblyLinearVelocity = _dir.Unit * {spd}"
                ));
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::SetAnchored {
                target: t,
                anchored,
            } => {
                let value = self.expression(anchored)?;
                self.line(format!("{}.Anchored = {value}", self.target(t)?));
            }
            BlockKind::PushWithForce { target: t, force } => {
                let value = self.expression(force)?;
                self.line(format!("{}:ApplyImpulse({value})", self.target(t)?));
            }
            BlockKind::SetPartVelocity {
                target: t,
                velocity,
            } => {
                let value = self.expression(velocity)?;
                self.line(format!(
                    "{}.AssemblyLinearVelocity = {value}",
                    self.target(t)?
                ));
            }
            // The friction first, so it can't see the `physical` local.
            BlockKind::SetFriction {
                target: t,
                friction,
            } => {
                let (object, friction) = (self.target(t)?, self.expression(friction)?);
                self.line("do");
                self.indent += 1;
                self.line(format!("local friction = {friction}"));
                self.line(format!(
                    "local physical = {object}.CurrentPhysicalProperties"
                ));
                self.line(format!(
                    "{object}.CustomPhysicalProperties = PhysicalProperties.new(physical.Density, \
                     friction, physical.Elasticity, physical.FrictionWeight, \
                     physical.ElasticityWeight)"
                ));
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::SetColor { target: t, color } => {
                let target = self.target(t)?;
                let value = self.expression(color)?;
                self.line("do");
                self.indent += 1;
                self.line(format!("local _t, _c = {target}, {value}"));
                self.line("if _t:IsA(\"TextLabel\") or _t:IsA(\"TextButton\") then");
                self.indent += 1;
                self.line("_t.TextColor3 = _c");
                self.indent -= 1;
                self.line("elseif _t:IsA(\"GuiObject\") then");
                self.indent += 1;
                self.line("_t.BackgroundColor3 = _c");
                self.indent -= 1;
                self.line("elseif _t:IsA(\"Decal\") or _t:IsA(\"ProjectedDecal\") then");
                self.indent += 1;
                self.line("_t.Color3 = _c");
                self.indent -= 1;
                self.line("else");
                self.indent += 1;
                self.line("_t.Color = _c");
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::SetText { target: t, text } => {
                let value = match text {
                    Expression::Text { .. } => self.expression(text)?,
                    other => format!("tostring({})", self.expression(other)?),
                };
                self.line(format!("{}.Text = {value}", self.target(t)?));
            }
            BlockKind::Show { target: t } => {
                self.line(format!("{}.Visible = true", self.target(t)?));
            }
            BlockKind::Hide { target: t } => {
                self.line(format!("{}.Visible = false", self.target(t)?));
            }
            BlockKind::ShowPage { layout, page } => {
                let value = match page {
                    Expression::Text { .. } => self.expression(page)?,
                    other => format!("tostring({})", self.expression(other)?),
                };
                self.line(format!("{}.CurrentPage = {value}", self.target(layout)?));
            }
            BlockKind::ScrollTo { target: t, x, y } => {
                let (x, y) = (self.expression(x)?, self.expression(y)?);
                self.line(format!(
                    "{}.CanvasPosition = Vector2.new({x}, {y})",
                    self.target(t)?
                ));
            }
            BlockKind::SetValue { target: t, value } => {
                let value = self.expression(value)?;
                self.line(format!("{}.Value = {value}", self.target(t)?));
            }
            BlockKind::SetDestination { agent, position } => {
                let value = self.expression(position)?;
                self.line(format!("{}.Destination = {value}", self.target(agent)?));
            }
            BlockKind::FollowTarget { agent, target: t } => {
                self.line(format!(
                    "{}.Destination = {}:GetPivot().Position",
                    self.target(agent)?,
                    self.target(t)?
                ));
            }
            BlockKind::StopAgent { agent } => {
                self.line(format!(
                    "{}.Velocity = Vector3.new(0, 0, 0)",
                    self.target(agent)?
                ));
            }
            BlockKind::SitInSeat { seat, controller } => {
                self.line(format!(
                    "{}:Sit({})",
                    self.target(seat)?,
                    self.target(controller)?
                ));
            }
            BlockKind::SetVelocity { agent, velocity } => {
                let value = self.expression(velocity)?;
                self.line(format!("{}.Velocity = {value}", self.target(agent)?));
            }
            BlockKind::ControlSpline { follower, action } => {
                let obj = self.target(follower)?;
                match choice(action, &SPLINE_ACTIONS, "a spline action")? {
                    "Start" => self.line(format!("{obj}.Enabled = true")),
                    "Stop" => self.line(format!("{obj}.Enabled = false")),
                    "Reverse" => self.line(format!("{obj}.Speed = -{obj}.Speed")),
                    _ => unreachable!(),
                }
            }
            BlockKind::SetSplineSpeed { follower, speed } => {
                let value = self.expression(speed)?;
                self.line(format!("{}.Speed = {value}", self.target(follower)?));
            }
            BlockKind::LeaveSeat { controller } => {
                self.line(format!("{}.Sit = false", self.target(controller)?));
            }
            BlockKind::SaveSlot { slot } => {
                let slot = self.expression(slot)?;
                self.line(format!("SaveService:SaveSlot({slot})"));
            }
            BlockKind::LoadSlot { slot } => {
                let slot = self.expression(slot)?;
                self.line(format!("SaveService:LoadSlot({slot})"));
            }
            BlockKind::DeleteSlot { slot } => {
                let slot = self.expression(slot)?;
                self.line(format!("SaveService:DeleteSlot({slot})"));
            }
            BlockKind::ForEachRow { table, body } => {
                let (idx, row) = if self.referenced_outputs.contains(&block_id) {
                    (format!("i_{block_id}"), format!("row_{block_id}"))
                } else {
                    ("_".to_owned(), "row".to_owned())
                };
                self.line(format!(
                    "for {idx}, {row} in DataService:GetTable({}):GetRows() do",
                    quote(table)
                ));
                self.nested(body, ids)?;
                self.line("end");
            }
            BlockKind::SetCheckpoint { checkpoint } => {
                self.line(format!(
                    "SaveService:SetCheckpoint({})",
                    self.target(checkpoint)?
                ));
            }
            BlockKind::RestoreCheckpoint => self.line("SaveService:RestoreCheckpoint()".to_owned()),
            BlockKind::LoadLevel { level, spawn } => {
                let level = self.expression(level)?;
                // No spawn name: the level's SpawnLocations.
                let line = match spawn {
                    Expression::Text { value } if value.is_empty() => {
                        format!("LevelService:LoadLevel({level})")
                    }
                    spawn => format!(
                        "LevelService:LoadLevel({level}, {})",
                        self.expression(spawn)?
                    ),
                };
                self.line(line);
            }
            BlockKind::AddLevel { level } => {
                let level = self.expression(level)?;
                self.line(format!("LevelService:AddLevel({level})"));
            }
            BlockKind::RemoveLevel { level } => {
                let level = self.expression(level)?;
                self.line(format!("LevelService:RemoveLevel({level})"));
            }
            BlockKind::HostGame { port } => {
                let port = self.expression(port)?;
                self.line(format!("NetworkService:Host({port})"));
            }
            BlockKind::JoinGame { address } => {
                let address = self.expression(address)?;
                self.line(format!("NetworkService:Join({address})"));
            }
            BlockKind::LeaveGame => self.line("NetworkService:Disconnect()".to_owned()),
            BlockKind::SetNetworkOwner { target: t, owner } => {
                let owner = self.expression(owner)?;
                self.line(format!("{}:SetNetworkOwner({owner})", self.target(t)?));
            }
            BlockKind::SetNetworkOwnershipAuto { target: t } => {
                self.line(format!("{}:SetNetworkOwnershipAuto()", self.target(t)?));
            }
            BlockKind::SendChatMessage { text } => {
                let text = self.expression(text)?;
                self.line(format!("TextChatService:SendMessage({text})"));
            }
            BlockKind::ShowSystemMessage { text } => {
                let text = self.expression(text)?;
                self.line(format!("TextChatService:DisplaySystemMessage({text})"));
            }
            BlockKind::SetLayerActive { data_layer, active } => {
                let active = self.expression(active)?;
                self.line(format!(
                    "DataLayerService:SetLayerActive({}, {active})",
                    quote(data_layer)
                ));
            }
            BlockKind::KickPlayer { player, reason } => {
                let player = self.expression(player)?;
                let reason = self.expression(reason)?;
                self.line(format!("{player}:Kick({reason})"));
            }
            BlockKind::RequestStreamAround { player, position } => {
                let player = self.expression(player)?;
                let position = self.expression(position)?;
                self.line(format!("{player}:RequestStreamAroundAsync({position})"));
            }
            BlockKind::SetLanguage { language } => {
                let language = self.expression(language)?;
                self.line(format!("LocalizationService.Language = {language}"));
            }
            BlockKind::SetCameraType { camera_type } => {
                let camera_type = choice(camera_type, &CAMERA_TYPES, "a camera type")?;
                self.line(format!(
                    "workspace.CurrentCamera.CameraType = Enum.CameraType.{camera_type}"
                ));
            }
            BlockKind::PointCameraAt { position } => {
                let value = self.expression(position)?;
                self.line(format!(
                    "workspace.CurrentCamera.CFrame = \
                     CFrame.lookAt(workspace.CurrentCamera.Position, {value})"
                ));
            }
            BlockKind::ShakeCamera { intensity, seconds } => {
                let (intensity, seconds) = (self.expression(intensity)?, self.expression(seconds)?);
                self.line(format!(
                    "workspace.CurrentCamera:Shake({intensity}, {seconds})"
                ));
            }
            BlockKind::SpawnDecal {
                texture,
                position,
                facing,
                size,
                seconds,
            } => {
                let texture = self.expression(texture)?;
                let position = self.expression(position)?;
                let facing = self.expression(facing)?;
                let (size, seconds) = (self.expression(size)?, self.expression(seconds)?);
                let referenced = self.referenced_outputs.contains(&block_id);
                if referenced {
                    self.line(format!("local _dpos = {position}"));
                    self.line(format!("local _dfacing = {facing}"));
                    self.line(
                        "local _dup = math.abs(_dfacing.Unit.Y) > 0.99 and Vector3.zAxis or Vector3.yAxis",
                    );
                    self.line(format!(
                        "local decal = workspace:SpawnDecal({texture}, CFrame.lookAt(_dpos, _dpos + _dfacing, _dup), {size}, {seconds})"
                    ));
                } else {
                    self.line("do");
                    self.indent += 1;
                    self.line(format!("local _dpos = {position}"));
                    self.line(format!("local _dfacing = {facing}"));
                    self.line(
                        "local _dup = math.abs(_dfacing.Unit.Y) > 0.99 and Vector3.zAxis or Vector3.yAxis",
                    );
                    self.line(format!(
                        "workspace:SpawnDecal({texture}, CFrame.lookAt(_dpos, _dpos + _dfacing, _dup), {size}, {seconds})"
                    ));
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::SetToneMapping { curve } => {
                let curve = choice(curve, &TONE_MAPPINGS, "a tone mapping")?;
                self.line(format!("Lighting.ToneMapping = Enum.ToneMapping.{curve}"));
            }
            BlockKind::SetClockTime { time, duration } => {
                let time = self.expression(time)?;
                if let Some(dur) = duration {
                    let dur = self.expression(dur)?;
                    self.line(format!(
                        "TweenService:Create(Lighting, TweenInfo.new({dur}), {{ ClockTime = {time} }}):Play()"
                    ));
                } else {
                    self.line(format!("Lighting.ClockTime = {time}"));
                }
            }
            BlockKind::SetAtmosphere {
                property,
                value,
                duration,
            } => {
                let property = choice(property, &ATMOSPHERE_NUMBERS, "an atmosphere number")?;
                let value = self.expression(value)?;
                if let Some(dur) = duration {
                    let dur = self.expression(dur)?;
                    self.line(format!(
                        "TweenService:Create(Lighting.Atmosphere, TweenInfo.new({dur}), {{ {property} = {value} }}):Play()"
                    ));
                } else {
                    self.line(format!("Lighting.Atmosphere.{property} = {value}"));
                }
            }
            BlockKind::ToggleVisibility { target: t } => {
                let target = self.target(t)?;
                self.line("do");
                self.indent += 1;
                self.line(format!("local _t = {target}"));
                self.line("if _t:IsA(\"GuiObject\") then");
                self.indent += 1;
                self.line("_t.Visible = not _t.Visible");
                self.indent -= 1;
                self.line("else");
                self.indent += 1;
                self.line("_t.Transparency = _t.Transparency < 0.5 and 1 or 0");
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::SetTransparency {
                target: t,
                transparency,
            } => {
                let target = self.target(t)?;
                let value = self.expression(transparency)?;
                self.line("do");
                self.indent += 1;
                self.line(format!("local _t, _v = {target}, {value}"));
                self.line("if _t:IsA(\"TextLabel\") or _t:IsA(\"TextButton\") then");
                self.indent += 1;
                self.line("_t.TextTransparency = _v");
                self.indent -= 1;
                self.line("elseif _t:IsA(\"ImageLabel\") or _t:IsA(\"ImageButton\") then");
                self.indent += 1;
                self.line("_t.ImageTransparency = _v");
                self.indent -= 1;
                self.line("elseif _t:IsA(\"GuiObject\") then");
                self.indent += 1;
                self.line("_t.BackgroundTransparency = _v");
                self.indent -= 1;
                self.line("else");
                self.indent += 1;
                self.line("_t.Transparency = _v");
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::SetMaterial {
                target: t,
                material,
            } => {
                let mat = choice(material, &MATERIALS, "a material")?;
                self.line(format!(
                    "{}.Material = Enum.Material.{mat}",
                    self.target(t)?
                ));
            }
            BlockKind::EmitParticles { target: t, count } => {
                let count = self.expression(count)?;
                self.line(format!("{}:Emit({count})", self.target(t)?));
            }
            BlockKind::SetEffectEnabled { target: t, enabled } => {
                let enabled = self.expression(enabled)?;
                self.line(format!("{}.Enabled = {enabled}", self.target(t)?));
            }
            BlockKind::SetLightingProperty { property, value } => {
                let prop = choice(property, &LIGHTING_PROPERTIES, "a lighting property")?;
                let value = self.expression(value)?;
                self.line(format!("Lighting.{prop} = {value}"));
            }
            BlockKind::SetPostProcessing {
                effect,
                property,
                value,
            } => {
                let fx = choice(effect, &POST_PROCESSING_EFFECTS, "a post-processing effect")?;
                let prop = choice(
                    property,
                    post_processing_properties(fx),
                    "a post-processing property",
                )?;
                let value = self.expression(value)?;
                self.line("do");
                self.indent += 1;
                self.line(format!(
                    "local _fx = Lighting:FindFirstChildOfClass({})",
                    quote(fx)
                ));
                self.line("if _fx then");
                self.indent += 1;
                self.line(format!("_fx.{prop} = {value}"));
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::TweenProperty {
                target: t,
                property,
                value,
                duration,
                easing,
                wait_until_done,
                completed,
            } => {
                if !valid_identifier(property) {
                    return Err(TranslateError::InvalidProperty(property.clone()));
                }
                let object = self.target(t)?;
                let val = self.expression(value)?;
                let dur = self.expression(duration)?;
                let style = choice(easing, &EASING_STYLES, "an easing style")?;
                let create = format!(
                    "TweenService:Create({object}, TweenInfo.new({dur}, Enum.EasingStyle.{style}), {{ {property} = {val} }})"
                );
                let referenced = self.referenced_outputs.contains(&block_id);
                if !referenced && !wait_until_done && completed.is_empty() {
                    self.line(format!("{create}:Play()"));
                } else if referenced {
                    self.line(format!("local tween = {create}"));
                    if !completed.is_empty() && !wait_until_done {
                        let close =
                            self.open_connect("tween.Completed:Connect(function()".to_owned());
                        self.nested(completed, ids)?;
                        self.line(close);
                    }
                    self.line("tween:Play()");
                    if *wait_until_done {
                        self.line("tween.Completed:Wait()");
                        if !completed.is_empty() {
                            self.write_body(completed, ids)?;
                        }
                    }
                } else {
                    self.line("do");
                    self.indent += 1;
                    self.line(format!("local _tween = {create}"));
                    if !completed.is_empty() && !wait_until_done {
                        let close =
                            self.open_connect("_tween.Completed:Connect(function()".to_owned());
                        self.nested(completed, ids)?;
                        self.line(close);
                    }
                    self.line("_tween:Play()");
                    if *wait_until_done {
                        self.line("_tween.Completed:Wait()");
                        if !completed.is_empty() {
                            self.write_body(completed, ids)?;
                        }
                    }
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::SetImage { target: t, image } => {
                let target = self.target(t)?;
                let img = self.expression(image)?;
                self.line("do");
                self.indent += 1;
                self.line(format!("local _t, _img = {target}, {img}"));
                self.line("if _t:IsA(\"ImageLabel\") or _t:IsA(\"ImageButton\") then");
                self.indent += 1;
                self.line("_t.Image = _img");
                self.indent -= 1;
                self.line("else");
                self.indent += 1;
                self.line("_t.Texture = _img");
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::SetGraphicsQuality { quality } => {
                let quality = choice(quality, &GRAPHICS_QUALITIES, "a graphics quality")?;
                self.line(format!(
                    "UserGameSettings.GraphicsQuality = Enum.GraphicsQuality.{quality}"
                ));
            }
            BlockKind::SetGraphicsOption {
                setting,
                choice: chosen,
                value,
            } => {
                if GRAPHICS_NUMBERS.contains(&setting.as_str()) {
                    let setting = choice(setting, &GRAPHICS_NUMBERS, "a graphics number")?;
                    let val = match value {
                        Some(expr) => self.expression(expr)?,
                        None => "1".to_owned(),
                    };
                    self.line(format!("UserGameSettings.{setting} = {val}"));
                } else {
                    let setting = choice(setting, &GRAPHICS_OPTIONS, "a graphics option")?;
                    let chosen = choice(chosen, graphics_choices(setting), "a choice for it")?;
                    let value = match (setting, chosen) {
                        (_, PRESET) => "nil".to_owned(),
                        (setting, on) if GRAPHICS_SWITCHES.contains(&setting) => {
                            (on == "On").to_string()
                        }
                        (setting, item) => format!("Enum.{setting}.{item}"),
                    };
                    self.line(format!("UserGameSettings.{setting} = {value}"));
                }
            }
            BlockKind::SetGraphicsNumber { setting, value } => {
                let setting = choice(setting, &GRAPHICS_NUMBERS, "a graphics number")?;
                let value = self.expression(value)?;
                self.line(format!("UserGameSettings.{setting} = {value}"));
            }
            BlockKind::SetProperty {
                target: t,
                property,
                value,
            } => {
                if !valid_identifier(property) {
                    return Err(TranslateError::InvalidProperty(property.clone()));
                }
                let value = self.expression(value)?;
                self.line(format!("{}.{property} = {value}", self.target(t)?));
            }
            BlockKind::SetAttribute {
                target: t,
                attribute,
                value,
            } => {
                let value = self.expression(value)?;
                self.line(format!(
                    "{}:SetAttribute({}, {value})",
                    self.target(t)?,
                    attribute_name(attribute)?
                ));
            }
            BlockKind::PlaySound {
                target: t,
                pitch,
                wait_until_done,
            } => {
                let target = self.target(t)?;
                if let Some(p) = pitch {
                    let p = self.expression(p)?;
                    self.line(format!("{target}.PlaybackSpeed = {p}"));
                }
                self.line(format!("{target}:Play()"));
                if *wait_until_done {
                    self.line(format!("{target}.Ended:Wait()"));
                }
            }
            BlockKind::PlaySoundAtPosition {
                target: t,
                position,
                volume,
                pitch,
            } => {
                let target = self.target(t)?;
                let pos = self.expression(position)?;
                let vol = volume.as_ref().map(|e| self.expression(e)).transpose()?;
                let spd = pitch.as_ref().map(|e| self.expression(e)).transpose()?;
                self.line("do");
                self.indent += 1;
                self.line(format!("local _s = {target}:Clone()"));
                self.line("_s.Spatial = true");
                self.line(format!("_s.Position = {pos}"));
                if let Some(v) = vol {
                    self.line(format!("_s.Volume = {v}"));
                }
                if let Some(p) = spd {
                    self.line(format!("_s.PlaybackSpeed = {p}"));
                }
                self.line("_s.Parent = workspace");
                self.line("_s:Play()");
                let close = if self.track_connections {
                    self.line("table.insert(_connections, _s.Ended:Once(function()");
                    "end))"
                } else {
                    self.line("_s.Ended:Once(function()");
                    "end)"
                };
                self.indent += 1;
                self.line("_s:Destroy()");
                self.indent -= 1;
                self.line(close);
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::PlayCutscene { sequence } => {
                self.line(format!("{}:Play()", self.target(sequence)?))
            }
            BlockKind::PauseCutscene { sequence } => {
                self.line(format!("{}:Pause()", self.target(sequence)?))
            }
            BlockKind::ResumeCutscene { sequence } => {
                self.line(format!("{}:Play()", self.target(sequence)?))
            }
            BlockKind::StopCutscene { sequence } => {
                self.line(format!("{}:Stop()", self.target(sequence)?))
            }
            BlockKind::PlaceOnSpline {
                target: t,
                spline,
                distance,
            } => {
                let distance = self.expression(distance)?;
                self.line(format!(
                    "{}:PivotTo({}:GetCFrameAt({distance}))",
                    self.target(t)?,
                    self.target(spline)?
                ));
            }
            BlockKind::Patrol { agent, spline } => {
                self.line(format!(
                    "{}:Patrol({})",
                    self.target(agent)?,
                    self.target(spline)?
                ));
            }
            BlockKind::StopSound { target: t } => self.line(format!("{}:Stop()", self.target(t)?)),
            BlockKind::PauseSound { target: t } => {
                self.line(format!("{}:Pause()", self.target(t)?))
            }
            BlockKind::ResumeSound { target: t } => {
                self.line(format!("{}:Resume()", self.target(t)?));
            }
            BlockKind::SetVolume { target: t, volume } => {
                let value = self.expression(volume)?;
                self.line(format!("{}.Volume = {value}", self.target(t)?));
            }
            BlockKind::FadeVolume {
                target: t,
                volume,
                duration,
                wait_until_done,
            } => {
                let target = self.target(t)?;
                let vol = self.expression(volume)?;
                let dur = self.expression(duration)?;
                let create = format!(
                    "TweenService:Create({target}, TweenInfo.new({dur}), {{ Volume = {vol} }})"
                );
                if *wait_until_done {
                    self.line("do");
                    self.indent += 1;
                    self.line(format!("local _tw = {create}"));
                    self.line("_tw:Play()");
                    self.line("_tw.Completed:Wait()");
                    self.indent -= 1;
                    self.line("end");
                } else {
                    self.line(format!("{create}:Play()"));
                }
            }
            BlockKind::SetSoundPitch { target: t, pitch } => {
                let value = self.expression(pitch)?;
                self.line(format!("{}.PlaybackSpeed = {value}", self.target(t)?));
            }
            BlockKind::SetSoundLooping { target: t, looped } => {
                let value = self.expression(looped)?;
                self.line(format!("{}.Looped = {value}", self.target(t)?));
            }
            BlockKind::SetSoundTime { target: t, time } => {
                let value = self.expression(time)?;
                self.line(format!("{}.TimePosition = {value}", self.target(t)?));
            }
            BlockKind::SetAnimatorBool {
                animator,
                parameter,
                value,
            } => {
                let value = self.expression(value)?;
                self.line(format!(
                    "{}:SetBool({}, {value})",
                    self.target(animator)?,
                    quote(parameter)
                ));
            }
            BlockKind::SetAnimatorNumber {
                animator,
                parameter,
                value,
            } => {
                let value = self.expression(value)?;
                self.line(format!(
                    "{}:SetNumber({}, {value})",
                    self.target(animator)?,
                    quote(parameter)
                ));
            }
            BlockKind::SetAnimatorLayerWeight {
                animator,
                layer,
                weight,
                seconds,
            } => {
                let weight = self.expression(weight)?;
                let seconds = self.expression(seconds)?;
                self.line(format!(
                    "{}:SetLayerWeight({}, {weight}, {seconds})",
                    self.target(animator)?,
                    quote(layer)
                ));
            }
            BlockKind::SetAnimatorTrigger {
                animator,
                parameter,
            } => self.line(format!(
                "{}:SetTrigger({})",
                self.target(animator)?,
                quote(parameter)
            )),
            BlockKind::ResetAnimatorTrigger {
                animator,
                parameter,
            } => self.line(format!(
                "{}:ResetTrigger({})",
                self.target(animator)?,
                quote(parameter)
            )),
            BlockKind::PlayAnimatorState { animator, state } => {
                self.line(format!("{}:Play({})", self.target(animator)?, quote(state)));
            }
            BlockKind::PlayAnimation {
                animator,
                animation,
                fade_time,
                speed,
                weight,
                wait_until_done,
            } => {
                let anim_target = self.target(animator)?;
                let anim_clip = self.target(animation)?;
                let out_ref = self.referenced_outputs.contains(&block_id);
                let play_args = match (fade_time, weight, speed) {
                    (None, None, None) => String::new(),
                    (Some(f), None, None) => self.expression(f)?,
                    (f, Some(w), None) => format!(
                        "{}, {}",
                        match f {
                            Some(e) => self.expression(e)?,
                            None => "0.1".to_owned(),
                        },
                        self.expression(w)?
                    ),
                    (f, w, Some(s)) => format!(
                        "{}, {}, {}",
                        match f {
                            Some(e) => self.expression(e)?,
                            None => "0.1".to_owned(),
                        },
                        match w {
                            Some(e) => self.expression(e)?,
                            None => "1".to_owned(),
                        },
                        self.expression(s)?
                    ),
                };
                if !out_ref && !*wait_until_done {
                    self.line(format!(
                        "{anim_target}:LoadAnimation({anim_clip}):Play({play_args})"
                    ));
                } else {
                    let var = if out_ref {
                        "track"
                    } else {
                        self.line("do");
                        self.indent += 1;
                        "_track"
                    };
                    self.line(format!(
                        "local {var} = {anim_target}:LoadAnimation({anim_clip})"
                    ));
                    self.line(format!("{var}:Play({play_args})"));
                    if *wait_until_done {
                        self.line(format!("{var}.Stopped:Wait()"));
                    }
                    if !out_ref {
                        self.indent -= 1;
                        self.line("end");
                    }
                }
            }
            BlockKind::StopAnimation {
                animator,
                animation,
                fade_time,
            } => {
                let anim_target = self.target(animator)?;
                let fade = match fade_time {
                    Some(e) => self.expression(e)?,
                    None => String::new(),
                };
                self.line(format!(
                    "for _, _track in ipairs({anim_target}:GetPlayingAnimationTracks()) do"
                ));
                self.indent += 1;
                if let Some(anim) = animation {
                    let clip = self.target(anim)?;
                    self.line(format!("if _track.Animation == {clip} then"));
                    self.indent += 1;
                    self.line(format!("_track:Stop({fade})"));
                    self.indent -= 1;
                    self.line("end");
                } else {
                    self.line(format!("_track:Stop({fade})"));
                }
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::SetAnimatorSpeed { animator, speed } => {
                let speed = self.expression(speed)?;
                self.line(format!("{}.Speed = {speed}", self.target(animator)?));
            }
            BlockKind::SetIkTarget {
                ik_control,
                target: t,
                weight,
            } => {
                let ik = self.target(ik_control)?;
                let target = self.target(t)?;
                self.line(format!("{ik}.Target = {target}"));
                if let Some(w) = weight {
                    let w = self.expression(w)?;
                    self.line(format!("{ik}.Weight = {w}"));
                }
            }
            BlockKind::SetRagdollEnabled { ragdoll, enabled } => {
                let enabled = self.expression(enabled)?;
                self.line(format!("{}.Enabled = {enabled}", self.target(ragdoll)?));
            }
            BlockKind::Tween {
                target: t,
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
                if !valid_identifier(property) {
                    return Err(TranslateError::InvalidProperty(property.clone()));
                }
                let style = choice(style, &EASING_STYLES, "an easing style")?;
                let direction = choice(direction, &EASING_DIRECTIONS, "an easing direction")?;
                let (object, value) = (self.target(t)?, self.expression(value)?);
                let seconds = self.expression(seconds)?;
                let info = if repeat_count.is_some() || *reverses {
                    let rep = match repeat_count {
                        Some(r) => self.expression(r)?,
                        None => "0".to_owned(),
                    };
                    format!(
                        "TweenInfo.new({seconds}, Enum.EasingStyle.{style}, \
                         Enum.EasingDirection.{direction}, {rep}, {reverses})"
                    )
                } else {
                    format!(
                        "TweenInfo.new({seconds}, Enum.EasingStyle.{style}, \
                         Enum.EasingDirection.{direction})"
                    )
                };
                let create =
                    format!("TweenService:Create({object}, {info}, {{{property} = {value}}})");
                let out_ref = self.referenced_outputs.contains(&block_id);
                if !out_ref && !*wait_until_done && completed.is_empty() {
                    self.line(format!("{create}:Play()"));
                } else {
                    let var = if out_ref {
                        "tween"
                    } else {
                        self.line("do");
                        self.indent += 1;
                        "_tw"
                    };
                    self.line(format!("local {var} = {create}"));
                    if !completed.is_empty() {
                        let close = self.open_connect(format!("{var}.Completed:Once(function()"));
                        self.nested(completed, ids)?;
                        self.line(close);
                    }
                    self.line(format!("{var}:Play()"));
                    if *wait_until_done {
                        self.line(format!("{var}.Completed:Wait()"));
                    }
                    if !out_ref {
                        self.indent -= 1;
                        self.line("end");
                    }
                }
            }
            // The info first, so `seconds` can't see the `goal` local.
            BlockKind::SlideIn {
                target: t,
                from,
                seconds,
                wait_until_done,
                completed,
            } => {
                let offset = match choice(from, &SLIDE_EDGES, "an edge")? {
                    "Left" => "goal - UDim2.fromScale(1, 0)",
                    "Right" => "goal + UDim2.fromScale(1, 0)",
                    "Top" => "goal - UDim2.fromScale(0, 1)",
                    _ => "goal + UDim2.fromScale(0, 1)",
                };
                let (object, seconds) = (self.target(t)?, self.expression(seconds)?);
                let out_ref = self.referenced_outputs.contains(&block_id);
                if !out_ref {
                    self.line("do");
                    self.indent += 1;
                }
                self.line(format!("local info = TweenInfo.new({seconds})"));
                self.line(format!("local goal = {object}:GetAttribute(\"_RestPos\")"));
                self.line("if not goal then");
                self.indent += 1;
                self.line(format!("goal = {object}.Position"));
                self.line(format!("{object}:SetAttribute(\"_RestPos\", goal)"));
                self.indent -= 1;
                self.line("end");
                self.line(format!("{object}.Visible = true"));
                self.line(format!("{object}.Position = {offset}"));
                let create = format!("TweenService:Create({object}, info, {{Position = goal}})");
                if !out_ref && !*wait_until_done && completed.is_empty() {
                    self.line(format!("{create}:Play()"));
                } else {
                    let var = if out_ref { "tween" } else { "_tw" };
                    self.line(format!("local {var} = {create}"));
                    if !completed.is_empty() {
                        let close = self.open_connect(format!("{var}.Completed:Once(function()"));
                        self.nested(completed, ids)?;
                        self.line(close);
                    }
                    self.line(format!("{var}:Play()"));
                    if *wait_until_done {
                        self.line(format!("{var}.Completed:Wait()"));
                    }
                }
                if !out_ref {
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::SlideOut {
                target: t,
                to,
                seconds,
                wait_until_done,
                completed,
            } => {
                let offset = match choice(to, &SLIDE_EDGES, "an edge")? {
                    "Left" => "rest - UDim2.fromScale(1, 0)",
                    "Right" => "rest + UDim2.fromScale(1, 0)",
                    "Top" => "rest - UDim2.fromScale(0, 1)",
                    _ => "rest + UDim2.fromScale(0, 1)",
                };
                let (object, seconds) = (self.target(t)?, self.expression(seconds)?);
                let out_ref = self.referenced_outputs.contains(&block_id);
                if !out_ref {
                    self.line("do");
                    self.indent += 1;
                }
                self.line(format!("local info = TweenInfo.new({seconds})"));
                self.line(format!("local rest = {object}:GetAttribute(\"_RestPos\")"));
                self.line("if not rest then");
                self.indent += 1;
                self.line(format!("rest = {object}.Position"));
                self.line(format!("{object}:SetAttribute(\"_RestPos\", rest)"));
                self.indent -= 1;
                self.line("end");
                let var = if out_ref { "tween" } else { "_tw" };
                self.line(format!(
                    "local {var} = TweenService:Create({object}, info, {{ Position = {offset} }})"
                ));
                let close = self.open_connect(format!("{var}.Completed:Once(function()"));
                self.indent += 1;
                self.line(format!("{object}.Visible = false"));
                if !completed.is_empty() {
                    self.write_body(completed, ids)?;
                }
                self.indent -= 1;
                self.line(close);
                self.line(format!("{var}:Play()"));
                if *wait_until_done {
                    self.line(format!("{var}.Completed:Wait()"));
                }
                if !out_ref {
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::FadeIn {
                target: t,
                seconds,
                wait_until_done,
                completed,
            }
            | BlockKind::FadeOut {
                target: t,
                seconds,
                wait_until_done,
                completed,
            } => {
                let is_out = matches!(kind, BlockKind::FadeOut { .. });
                let transparency = u8::from(is_out);
                let (object, seconds) = (self.target(t)?, self.expression(seconds)?);
                let out_ref = self.referenced_outputs.contains(&block_id);
                if !out_ref {
                    self.line("do");
                    self.indent += 1;
                }
                self.line(format!("local _t = {object}"));
                self.line(format!("local _info = TweenInfo.new({seconds})"));
                self.line("local _goals = {}");
                self.line("if _t:IsA(\"BasePart\") then");
                self.indent += 1;
                self.line(format!("_goals.Transparency = {transparency}"));
                self.indent -= 1;
                self.line("else");
                self.indent += 1;
                if !is_out {
                    self.line("_t.Visible = true");
                }
                self.line(format!("_goals.BackgroundTransparency = {transparency}"));
                self.line("if _t:IsA(\"TextLabel\") or _t:IsA(\"TextButton\") then");
                self.indent += 1;
                self.line(format!("_goals.TextTransparency = {transparency}"));
                self.indent -= 1;
                self.line("elseif _t:IsA(\"ImageLabel\") or _t:IsA(\"ImageButton\") then");
                self.indent += 1;
                self.line(format!("_goals.ImageTransparency = {transparency}"));
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line("end");
                let var = if out_ref { "tween" } else { "_tw" };
                let needs_local = out_ref || is_out || *wait_until_done || !completed.is_empty();
                if !needs_local {
                    self.line("TweenService:Create(_t, _info, _goals):Play()");
                } else {
                    self.line(format!(
                        "local {var} = TweenService:Create(_t, _info, _goals)"
                    ));
                    if is_out || !completed.is_empty() {
                        let close = self.open_connect(format!("{var}.Completed:Once(function()"));
                        self.indent += 1;
                        if is_out {
                            self.line("if not _t:IsA(\"BasePart\") then");
                            self.indent += 1;
                            self.line("_t.Visible = false");
                            self.indent -= 1;
                            self.line("end");
                        }
                        if !completed.is_empty() {
                            self.write_body(completed, ids)?;
                        }
                        self.indent -= 1;
                        self.line(close);
                    }
                    self.line(format!("{var}:Play()"));
                    if *wait_until_done {
                        self.line(format!("{var}.Completed:Wait()"));
                    }
                }
                if !out_ref {
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::TweenPivot {
                target: t,
                position,
                duration,
                easing,
                direction,
                wait_until_done,
                completed,
            } => {
                let style = choice(easing, &EASING_STYLES, "an easing style")?;
                let dir = choice(direction, &EASING_DIRECTIONS, "an easing direction")?;
                let (object, pos, dur) = (
                    self.target(t)?,
                    self.expression(position)?,
                    self.expression(duration)?,
                );
                let out_ref = self.referenced_outputs.contains(&block_id);
                if !out_ref {
                    self.line("do");
                    self.indent += 1;
                }
                self.line(format!("local _t = {object}"));
                self.line(format!(
                    "local _info = TweenInfo.new({dur}, Enum.EasingStyle.{style}, Enum.EasingDirection.{dir})"
                ));
                self.line("local _proxy = Instance.new(\"CFrameValue\")");
                self.line("_proxy.Value = _t:GetPivot()");
                self.line(format!("local _pos = {pos}"));
                self.line("local _goal = if typeof(_pos) == \"CFrame\" then _pos else CFrame.new(_pos) * _proxy.Value.Rotation");
                self.line("local _conn = _proxy.Changed:Connect(function(cf)");
                self.indent += 1;
                self.line("_t:PivotTo(cf)");
                self.indent -= 1;
                self.line("end)");
                if self.track_connections {
                    self.line("table.insert(_connections, _conn)");
                }
                let var = if out_ref { "tween" } else { "_tw" };
                self.line(format!(
                    "local {var} = TweenService:Create(_proxy, _info, {{ Value = _goal }})"
                ));
                let done_close = self.open_connect(format!("{var}.Completed:Once(function()"));
                self.indent += 1;
                self.line("_conn:Disconnect()");
                self.line("_proxy:Destroy()");
                if !completed.is_empty() {
                    self.write_body(completed, ids)?;
                }
                self.indent -= 1;
                self.line(done_close);
                self.line(format!("{var}:Play()"));
                if *wait_until_done {
                    self.line(format!("{var}.Completed:Wait()"));
                }
                if !out_ref {
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::PulseScale {
                target: t,
                scale,
                duration,
                wait_until_done,
            } => {
                let (object, scale, dur) = (
                    self.target(t)?,
                    self.expression(scale)?,
                    self.expression(duration)?,
                );
                let out_ref = self.referenced_outputs.contains(&block_id);
                if !out_ref {
                    self.line("do");
                    self.indent += 1;
                }
                self.line(format!("local _t = {object}"));
                self.line(format!("local _s = {scale}"));
                self.line(format!(
                    "local _info = TweenInfo.new(({dur}) / 2, Enum.EasingStyle.Quad, Enum.EasingDirection.Out, 0, true)"
                ));
                self.line("local _goal = if _t:IsA(\"BasePart\") then _t.Size * _s else UDim2.new(_t.Size.X.Scale * _s, _t.Size.X.Offset * _s, _t.Size.Y.Scale * _s, _t.Size.Y.Offset * _s)");
                let var = if out_ref { "tween" } else { "_tw" };
                self.line(format!(
                    "local {var} = TweenService:Create(_t, _info, {{ Size = _goal }})"
                ));
                self.line(format!("{var}:Play()"));
                if *wait_until_done {
                    self.line(format!("{var}.Completed:Wait()"));
                }
                if !out_ref {
                    self.indent -= 1;
                    self.line("end");
                }
            }
            BlockKind::CancelTween { tween } => {
                self.line(format!("{}:Cancel()", self.target(tween)?));
            }
            BlockKind::SetVariable { name, value } => {
                let value = self.expression(value)?;
                let var = self.var_ref(name);
                self.line(format!("{var} = {value}"));
            }
            BlockKind::ChangeVariable { name, by } => {
                let value = self.expression(by)?;
                let var = self.var_ref(name);
                self.line(format!("{var} = (tonumber({var}) or 0) + {value}"));
            }
            BlockKind::ListCreate { name, items } => {
                if !valid_identifier(name) {
                    return Err(TranslateError::InvalidVariable(name.clone()));
                }
                let var = self.var_ref(name);
                if items.is_empty() {
                    self.line(format!("{var} = {{}}"));
                } else {
                    let items = items
                        .iter()
                        .map(|item| self.expression(item))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ");
                    self.line(format!("{var} = {{ {items} }}"));
                }
            }
            BlockKind::ListAdd { list, value } => {
                let (list, value) = (self.expression(list)?, self.expression(value)?);
                self.line(format!("table.insert({list}, {value})"));
            }
            BlockKind::ListInsert { list, index, value } => {
                let (list, index, value) = (
                    self.expression(list)?,
                    self.expression(index)?,
                    self.expression(value)?,
                );
                self.line(format!("table.insert({list}, {index}, {value})"));
            }
            BlockKind::ListRemoveIndex { list, index } => {
                let (list, index) = (self.expression(list)?, self.expression(index)?);
                self.line(format!("table.remove({list}, {index})"));
            }
            BlockKind::ListRemoveValue { list, value } => {
                let (list, value) = (self.expression(list)?, self.expression(value)?);
                self.line("do");
                self.indent += 1;
                self.line(format!("local _i = table.find({list}, {value})"));
                self.line("if _i then");
                self.indent += 1;
                self.line(format!("table.remove({list}, _i)"));
                self.indent -= 1;
                self.line("end");
                self.indent -= 1;
                self.line("end");
            }
            BlockKind::ListClear { list } => {
                let list = self.expression(list)?;
                self.line(format!("table.clear({list})"));
            }
            BlockKind::Print { value } => {
                let value = self.expression(value)?;
                self.line(format!("print({value})"));
            }
            BlockKind::Destroy { target: t } => self.line(format!("{}:Destroy()", self.target(t)?)),
            BlockKind::FireEvent { event, data } => match data {
                Some(payload) => {
                    let payload = self.expression(payload)?;
                    self.line(format!("{}:Fire({payload})", stored(event)?));
                }
                None => self.line(format!("{}:Fire()", stored(event)?)),
            },
            BlockKind::CloneInto {
                target: t,
                parent,
                count,
                position,
                offset_per_clone,
                per_clone_positions,
            } => {
                let obj = self.target(t)?;
                let dest = self.container(parent)?;
                let referenced = self.referenced_outputs.contains(&block_id);
                let has_extras = count.is_some()
                    || position.is_some()
                    || offset_per_clone.is_some()
                    || per_clone_positions.is_some();
                if !referenced && !has_extras {
                    self.line(format!("{obj}:Clone().Parent = {dest}"));
                } else {
                    let (single_name, array_name) =
                        self.clone_ids.get(&block_id).cloned().unwrap_or_else(|| {
                            let base = clone_base_name(t);
                            (format!("{base}_cloned"), format!("{base}_clones"))
                        });
                    let is_batch = per_clone_positions.is_some()
                        || offset_per_clone.is_some()
                        || matches!(count, Some(expr) if !matches!(expr, Expression::Number { value } if (*value - 1.0).abs() < f64::EPSILON));
                    if !is_batch {
                        self.line(format!("local {single_name} = {obj}:Clone()"));
                        if let Some(pos) = position {
                            self.write_clone_pivot(&single_name, pos, None)?;
                        }
                        self.line(format!("{single_name}.Parent = {dest}"));
                    } else {
                        self.line(format!("local {array_name} = {{}}"));
                        self.line(format!("local {single_name} = nil"));
                        if let Some(positions) = per_clone_positions {
                            let pos_expr = self.expression(positions)?;
                            self.line(format!("local _positions = {pos_expr}"));
                            let count_expr = match count {
                                Some(c) => self.expression(c)?,
                                None => "#_positions".to_owned(),
                            };
                            self.line(format!("for _i = 1, {count_expr} do"));
                            self.indent += 1;
                            self.line(format!("local _clone = {obj}:Clone()"));
                            self.line("local _p = _positions[_i]");
                            self.line("if _p ~= nil then");
                            self.indent += 1;
                            self.line("_clone:PivotTo(typeof(_p) == \"CFrame\" and _p or CFrame.new(_p) * _clone:GetPivot().Rotation)");
                            self.indent -= 1;
                            self.line("end");
                            self.line(format!("_clone.Parent = {dest}"));
                            self.line(format!("table.insert({array_name}, _clone)"));
                            self.line(format!("{single_name} = _clone"));
                            self.indent -= 1;
                            self.line("end");
                        } else {
                            let count_expr = match count {
                                Some(c) => self.expression(c)?,
                                None => "1".to_owned(),
                            };
                            let offset_expr = match offset_per_clone {
                                Some(off) => Some(self.expression(off)?),
                                None => None,
                            };
                            self.line(format!("for _i = 1, {count_expr} do"));
                            self.indent += 1;
                            self.line(format!("local _clone = {obj}:Clone()"));
                            if let Some(pos) = position {
                                self.write_clone_pivot("_clone", pos, offset_expr.as_deref())?;
                            } else if let Some(off) = offset_expr.as_deref() {
                                self.line(format!(
                                    "_clone:PivotTo(_clone:GetPivot() + ({off}) * (_i - 1))"
                                ));
                            }
                            self.line(format!("_clone.Parent = {dest}"));
                            self.line(format!("table.insert({array_name}, _clone)"));
                            self.line(format!("{single_name} = _clone"));
                            self.indent -= 1;
                            self.line("end");
                        }
                    }
                }
            }
            BlockKind::Code { code } => self.code(code),
            BlockKind::Connect {
                signal,
                method,
                params,
                body,
            } => self.write_connect(block_id, signal, method, params, body, ids)?,
            BlockKind::Comment { .. }
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
            | BlockKind::Return { .. } => self.write_luau(block_id, kind, ids)?,
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
            | BlockKind::OnSignal { .. } => {
                return Err(TranslateError::InvalidEventPlacement);
            }
        }
        Ok(())
    }

    /// An expression as one side of `op`, bracketed only if Luau would
    /// read it differently without. `right` is the side.
    pub(super) fn operand(
        &self,
        expression: &Expression,
        op: &str,
        right: bool,
    ) -> Result<String, TranslateError> {
        let text = self.expression(expression)?;
        let parent = binary_priority(op).unwrap_or((luau::UNARY, luau::UNARY));
        Ok(match expression_priority(expression) {
            Some((child, unary)) if needs_brackets(child, unary, parent, right) => {
                format!("({text})")
            }
            _ => text,
        })
    }

    pub(super) fn target(&self, obj: &ObjectRef) -> Result<String, TranslateError> {
        match obj.as_expr() {
            Expression::ObjectPath { path } => target(path),
            other => self.receiver(other),
        }
    }

    pub(super) fn container(&self, obj: &ObjectRef) -> Result<String, TranslateError> {
        match obj.as_expr() {
            Expression::ObjectPath { path } => container(path),
            other => self.receiver(other),
        }
    }

    pub(super) fn var_ref(&self, name: &str) -> String {
        if self.global_vars.contains(name) {
            format!("_G.{name}")
        } else {
            name.to_owned()
        }
    }

    fn write_clone_pivot(
        &mut self,
        var: &str,
        pos: &Expression,
        offset: Option<&str>,
    ) -> Result<(), TranslateError> {
        let pos_expr = self.expression(pos)?;
        let offset_suffix = offset
            .map(|off| format!(" + ({off}) * (_i - 1)"))
            .unwrap_or_default();
        if matches!(pos, Expression::Vector3 { .. }) {
            self.line(format!(
                "{var}:PivotTo(CFrame.new({pos_expr}{offset_suffix}) * {var}:GetPivot().Rotation)"
            ));
        } else {
            self.line(format!("local _pos = {pos_expr}"));
            self.line(format!(
                "{var}:PivotTo((typeof(_pos) == \"CFrame\" and _pos or CFrame.new(_pos) * {var}:GetPivot().Rotation){offset_suffix})"
            ));
        }
        Ok(())
    }

    /// An expression before a `:` method call, which needs a name or a call
    /// there, so anything else is bracketed: `(a or b):IsA("Part")`.
    pub(super) fn receiver(&self, expression: &Expression) -> Result<String, TranslateError> {
        let text = self.expression(expression)?;
        Ok(match expression {
            Expression::Variable { .. }
            | Expression::ObjectPath { .. }
            | Expression::BlockOutput { .. }
            | Expression::ListItem { .. }
            | Expression::TouchingPart
            | Expression::TriggeringPlayer
            | Expression::CommandArgs
            | Expression::DeltaTime
            | Expression::AnimatorNewState
            | Expression::AnimatorPreviousState
            | Expression::AnimationEventName
            | Expression::ConnectionFailureReason
            | Expression::ChangedLayerName
            | Expression::ChangedLayerActive
            | Expression::ChatMessageText
            | Expression::ChatMessageSender
            | Expression::IsHost
            | Expression::IsConnected
            | Expression::SlotExists { .. }
            | Expression::DataTableRowCount { .. }
            | Expression::PartUnderMouse
            | Expression::MouseHitPosition
            | Expression::CameraPosition
            | Expression::CameraLookDirection
            | Expression::NearestPlayerTo { .. }
            | Expression::PlayerOfPart { .. }
            | Expression::ParentOf { .. }
            | Expression::ChildOf { .. }
            | Expression::GameTime
            | Expression::Property { .. }
            | Expression::Attribute { .. }
            | Expression::Raycast { .. }
            | Expression::RaycastIgnoring { .. }
            | Expression::Vector3 { .. }
            | Expression::Color3 { .. }
            | Expression::MakeVector3 { .. }
            | Expression::VectorAxis { .. }
            | Expression::Distance { .. }
            | Expression::Direction { .. }
            | Expression::Magnitude { .. }
            | Expression::MakeColorRgb { .. }
            | Expression::MakeColorHsv { .. }
            | Expression::Lerp { .. }
            | Expression::DataRow { .. }
            | Expression::SplinePosition { .. }
            | Expression::DataValue { .. }
            | Expression::Name { .. }
            | Expression::Field { .. }
            | Expression::Index { .. }
            | Expression::Call { .. }
            | Expression::MethodCall { .. }
            | Expression::Paren { .. } => text,
            _ => format!("({text})"),
        })
    }

    pub(super) fn expression(&self, expression: &Expression) -> Result<String, TranslateError> {
        let finite = |values: &[f64]| values.iter().all(|value| value.is_finite());
        Ok(match expression {
            Expression::Number { value } if value.is_finite() => number(*value),
            Expression::Text { value } => quote(value),
            Expression::Boolean { value } => value.to_string(),
            Expression::Vector3 { x, y, z } if finite(&[*x, *y, *z]) => {
                format!(
                    "Vector3.new({}, {}, {})",
                    number(*x),
                    number(*y),
                    number(*z)
                )
            }
            Expression::Color3 { r, g, b } if finite(&[*r, *g, *b]) => {
                format!("Color3.new({}, {}, {})", number(*r), number(*g), number(*b))
            }
            Expression::Number { .. } | Expression::Vector3 { .. } | Expression::Color3 { .. } => {
                return Err(TranslateError::InvalidNumber);
            }
            Expression::ObjectPath { path } => target(path)?,
            Expression::BlockOutput { name, .. } if valid_identifier(name) => name.clone(),
            Expression::BlockOutput { name, .. } => {
                return Err(TranslateError::InvalidVariable(name.clone()));
            }
            Expression::Variable { name } if valid_identifier(name) => self.var_ref(name),
            Expression::Variable { name } => {
                return Err(TranslateError::InvalidVariable(name.clone()));
            }
            Expression::ListItem { list, index } => {
                format!("{}[{}]", self.receiver(list)?, self.expression(index)?)
            }
            Expression::ListLength { list } => {
                format!("#{}", self.operand(list, "#", true)?)
            }
            Expression::ListContains { list, value } => format!(
                "table.find({}, {}) ~= nil",
                self.expression(list)?,
                self.expression(value)?
            ),
            Expression::Math { op, left, right } => format!(
                "{} {} {}",
                self.operand(left, op.lua(), false)?,
                op.lua(),
                self.operand(right, op.lua(), true)?
            ),
            Expression::Function { function, value } => {
                let value = self.expression(value)?;
                match function {
                    MathFunction::Abs => format!("math.abs({value})"),
                    MathFunction::Floor => format!("math.floor({value})"),
                    MathFunction::Ceil => format!("math.ceil({value})"),
                    MathFunction::Round => format!("math.floor({value} + 0.5)"),
                    MathFunction::Sqrt => format!("math.sqrt({value})"),
                    MathFunction::Sin => format!("math.sin({value})"),
                    MathFunction::Cos => format!("math.cos({value})"),
                    MathFunction::Tan => format!("math.tan({value})"),
                    MathFunction::Asin => format!("math.asin({value})"),
                    MathFunction::Acos => format!("math.acos({value})"),
                    MathFunction::Atan => format!("math.atan({value})"),
                    MathFunction::Log => format!("math.log({value})"),
                    MathFunction::Exp => format!("math.exp({value})"),
                    MathFunction::Sign => format!("math.sign({value})"),
                }
            }
            Expression::Random { min, max } => {
                let (min, max) = (self.expression(min)?, self.expression(max)?);
                format!("math.random(math.floor({min}), math.floor({max}))")
            }
            Expression::RandomNumber { min, max } => {
                let (min, max) = (self.expression(min)?, self.expression(max)?);
                format!("Random.new():NextNumber({min}, {max})")
            }
            Expression::Lerp { from, to, alpha } => {
                if is_scalar_expression(from) || is_scalar_expression(to) {
                    let a = self.expression(from)?;
                    let b = self.expression(to)?;
                    let t = self.expression(alpha)?;
                    format!("({a} + ({b} - {a}) * {t})")
                } else {
                    format!(
                        "{}:Lerp({}, {})",
                        self.receiver(from)?,
                        self.expression(to)?,
                        self.expression(alpha)?
                    )
                }
            }
            Expression::MakeVector3 { x, y, z } => format!(
                "Vector3.new({}, {}, {})",
                self.expression(x)?,
                self.expression(y)?,
                self.expression(z)?
            ),
            Expression::VectorAxis { axis, vector } => {
                let axis = choice(axis, &VECTOR_AXES, "a vector axis")?;
                format!("{}.{axis}", self.receiver(vector)?)
            }
            Expression::Distance { from, to } => format!(
                "({} - {}).Magnitude",
                self.operand(from, "-", false)?,
                self.operand(to, "-", true)?
            ),
            Expression::Direction { from, to } => format!(
                "({} - {}).Unit",
                self.operand(to, "-", false)?,
                self.operand(from, "-", true)?
            ),
            Expression::Magnitude { vector } => format!("{}.Magnitude", self.receiver(vector)?),
            Expression::Clamp { value, min, max } => format!(
                "math.clamp({}, {}, {})",
                self.expression(value)?,
                self.expression(min)?,
                self.expression(max)?
            ),
            Expression::MinMax { op, left, right } => {
                let op = choice(op, &MIN_MAX_OPS, "min or max")?;
                format!(
                    "math.{op}({}, {})",
                    self.expression(left)?,
                    self.expression(right)?
                )
            }
            Expression::StringLength { text } => {
                format!("#tostring({})", self.expression(text)?)
            }
            Expression::StringContains { text, substring } => format!(
                "string.find(tostring({}), tostring({}), 1, true) ~= nil",
                self.expression(text)?,
                self.expression(substring)?
            ),
            Expression::FormatNumber { value, decimals } => {
                let val = self.expression(value)?;
                if let Expression::Number { value: n } = decimals.as_ref()
                    && n.is_finite()
                    && *n >= 0.0
                    && n.fract() == 0.0
                {
                    format!("string.format(\"%.{}f\", {val})", *n as u32)
                } else {
                    format!(
                        "string.format(\"%.\" .. math.floor({}) .. \"f\", {val})",
                        self.expression(decimals)?
                    )
                }
            }
            Expression::MakeColorRgb { r, g, b } => format!(
                "Color3.fromRGB({}, {}, {})",
                self.expression(r)?,
                self.expression(g)?,
                self.expression(b)?
            ),
            Expression::MakeColorHsv { h, s, v } => format!(
                "Color3.fromHSV({}, {}, {})",
                self.expression(h)?,
                self.expression(s)?,
                self.expression(v)?
            ),
            Expression::Join { left, right } => {
                let (left, right) = (self.expression(left)?, self.expression(right)?);
                format!("tostring({left}) .. tostring({right})")
            }
            Expression::Compare { op, left, right } => format!(
                "{} {} {}",
                self.operand(left, op.lua(), false)?,
                op.lua(),
                self.operand(right, op.lua(), true)?
            ),
            Expression::Logic { op, left, right } => format!(
                "{} {} {}",
                self.operand(left, op.lua(), false)?,
                op.lua(),
                self.operand(right, op.lua(), true)?
            ),
            Expression::Not { value } => format!("not {}", self.operand(value, "not", true)?),
            Expression::Property {
                target: t,
                property,
            } => {
                if !valid_identifier(property) {
                    return Err(TranslateError::InvalidProperty(property.clone()));
                }
                format!("{}.{property}", self.target(t)?)
            }
            Expression::Attribute {
                target: t,
                attribute,
            } => format!(
                "{}:GetAttribute({})",
                self.target(t)?,
                attribute_name(attribute)?
            ),
            Expression::SoundProperty {
                target: t,
                property,
            } => {
                let prop = choice(property, &SOUND_PROPERTIES, "a sound property")?;
                format!("{}.{prop}", self.target(t)?)
            }
            Expression::Raycast {
                origin,
                direction,
                output,
                ignore,
            } => {
                let o = self.expression(origin)?;
                let d = self.expression(direction)?;
                let base = if let Some(ig) = ignore {
                    let ig_str = self.target(ig)?;
                    format!(
                        "workspace:Raycast({o}, {d}, RaycastParams.new({{FilterDescendantsInstances = {{{ig_str}}}}}))"
                    )
                } else {
                    format!("workspace:Raycast({o}, {d})")
                };
                match output.as_str() {
                    "" | "Result" => base,
                    "Hit Part" => {
                        format!("(function() local _r = {base} return _r and _r.Instance end)()")
                    }
                    "Hit Position" => {
                        format!("(function() local _r = {base} return _r and _r.Position end)()")
                    }
                    "Hit Normal" => {
                        format!("(function() local _r = {base} return _r and _r.Normal end)()")
                    }
                    "Distance" => {
                        format!("(function() local _r = {base} return _r and _r.Distance end)()")
                    }
                    other => {
                        return Err(TranslateError::InvalidChoice {
                            what: "a raycast output",
                            value: other.to_owned(),
                            choices: RAYCAST_OUTPUTS.join(", "),
                        });
                    }
                }
            }
            Expression::RaycastIgnoring {
                origin,
                direction,
                ignore,
            } => {
                format!(
                    "workspace:Raycast({}, {}, RaycastParams.new({{FilterDescendantsInstances = {{{}}}}}))",
                    self.expression(origin)?,
                    self.expression(direction)?,
                    self.target(ignore)?
                )
            }
            Expression::PartUnderMouse => "game.Players.LocalPlayer:GetMouse().Target".to_owned(),
            Expression::MouseHitPosition => {
                "Players.LocalPlayer:GetMouse().Hit.Position".to_owned()
            }
            Expression::CameraPosition => "workspace.CurrentCamera.CFrame.Position".to_owned(),
            Expression::CameraLookDirection => {
                "workspace.CurrentCamera.CFrame.LookVector".to_owned()
            }
            Expression::NearestPlayerTo { position } => {
                let pos = self.expression(position)?;
                format!(
                    "(function() local _best, _dist = nil, math.huge for _, _p in Players:GetPlayers() do local _c = _p.Character if _c then local _d = (_c:GetPivot().Position - ({pos})).Magnitude if _d < _dist then _best, _dist = _p, _d end end end return _best end)()"
                )
            }
            Expression::PlayerOfPart { part } => {
                format!(
                    "Players:GetPlayerFromCharacter({}.Parent)",
                    self.target(part)?
                )
            }
            Expression::PlayerIsAlive { player } => {
                format!(
                    "(function() local _c = {}.Character local _h = _c and (_c:FindFirstChildOfClass(\"Humanoid\") or _c:FindFirstChildOfClass(\"Health\")) return _h ~= nil and _h.Health > 0 end)()",
                    self.target(player)?
                )
            }
            Expression::ParentOf { target: t } => format!("{}.Parent", self.target(t)?),
            Expression::ChildOf { name, target: t } => {
                format!(
                    "{}:FindFirstChild({})",
                    self.target(t)?,
                    self.expression(name)?
                )
            }
            Expression::HasTag { target: t, tag } => {
                format!(
                    "CollectionService:HasTag({}, {})",
                    self.target(t)?,
                    self.expression(tag)?
                )
            }
            Expression::GameTime => "workspace.DistributedGameTime".to_owned(),
            Expression::TouchingPart if self.in_touched => "otherPart".to_owned(),
            Expression::TouchingPart => return Err(TranslateError::TouchingPartOutsideTouched),
            Expression::TriggeringPlayer => "player".to_owned(),
            Expression::CommandArgs => "args".to_owned(),
            Expression::DeltaTime => "deltaTime".to_owned(),
            Expression::AnimatorNewState => "state".to_owned(),
            Expression::AnimatorPreviousState => "previous".to_owned(),
            Expression::AnimationEventName => "name".to_owned(),
            Expression::ConnectionFailureReason => "reason".to_owned(),
            Expression::ChangedLayerName => "layerName".to_owned(),
            Expression::ChangedLayerActive => "active".to_owned(),
            Expression::ChatMessageText => "message.Text".to_owned(),
            Expression::ChatMessageSender => "message.Player".to_owned(),
            Expression::IsHost => "NetworkService.IsHost".to_owned(),
            Expression::IsConnected => "NetworkService.IsConnected".to_owned(),
            Expression::SlotExists { slot } => {
                format!("SaveService:HasSlot({})", self.expression(slot)?)
            }
            Expression::DataTableRowCount { table } => {
                format!("DataService:GetTable({}):Count()", quote(table))
            }
            Expression::KeyDown { key: name } => {
                format!("UserInputService:IsKeyDown({})", key(name)?)
            }
            Expression::LayerActive { data_layer } => {
                format!("DataLayerService:IsLayerActive({})", quote(data_layer))
            }
            Expression::AnimatorBool {
                animator,
                parameter,
            } => format!("{}:GetBool({})", self.target(animator)?, quote(parameter)),
            Expression::AnimatorNumber {
                animator,
                parameter,
            } => format!("{}:GetNumber({})", self.target(animator)?, quote(parameter)),
            Expression::AnimatorLayerWeight { animator, layer } => {
                format!(
                    "{}:GetLayerWeight({})",
                    self.target(animator)?,
                    quote(layer)
                )
            }
            Expression::IsStreamedIn { value } => format!(
                "{}.StreamingState == Enum.StreamingState.StreamedIn",
                self.receiver(value)?
            ),
            Expression::IsA { value, class } => {
                format!("{}:IsA({})", self.receiver(value)?, quote(class))
            }
            Expression::SplinePosition { spline, t } => format!(
                "{}:GetPositionAt({})",
                self.target(spline)?,
                self.expression(t)?
            ),
            Expression::SplineClosestDistance { spline, position } => format!(
                "{}:GetClosestDistance({})",
                self.target(spline)?,
                self.expression(position)?
            ),
            Expression::DataRow { table, key } => format!(
                "DataService:GetTable({}):Get({})",
                quote(table),
                self.expression(key)?
            ),
            // A column whose name isn't a Lua name (`Fire Rate`) is indexed.
            Expression::DataValue { column, row } if valid_identifier(column) => {
                format!("{}.{column}", self.receiver(row)?)
            }
            Expression::DataValue { column, row } => {
                format!("{}[{}]", self.receiver(row)?, quote(column))
            }
            // Bracketed already where it needs to be (see `from_lua`).
            Expression::Code { code } => code.clone(),
            luau => self.luau_value(luau)?,
        })
    }
}

/// How tightly `expression` holds together if it's an operator, and whether
/// it's written in front of its value (`not`, `-`, and negative numbers).
fn expression_priority(expression: &Expression) -> Option<((u8, u8), bool)> {
    let unary = Some(((luau::UNARY, luau::UNARY), true));
    match expression {
        Expression::Math { op, .. } => binary_priority(op.lua()).map(|p| (p, false)),
        Expression::Compare { op, .. } => binary_priority(op.lua()).map(|p| (p, false)),
        Expression::Logic { op, .. } => binary_priority(op.lua()).map(|p| (p, false)),
        Expression::Binary { op, .. } => binary_priority(op).map(|p| (p, false)),
        Expression::Join { .. } => binary_priority("..").map(|p| (p, false)),
        Expression::Not { .. } | Expression::Unary { .. } => unary,
        Expression::Number { value } if *value < 0.0 => unary,
        _ => None,
    }
}

/// A number as a block writes it, to tell it from one written another way.
pub fn number_text(value: f64) -> String {
    number(value)
}

/// A number as Lua writes it: whole numbers without a decimal point.
pub(super) fn number(value: f64) -> String {
    value.to_string()
}

/// `value` if it's one of `choices`, which are `what`.
fn choice<'a>(
    value: &str,
    choices: &[&'a str],
    what: &'static str,
) -> Result<&'a str, TranslateError> {
    choices
        .iter()
        .copied()
        .find(|choice| *choice == value)
        .ok_or_else(|| TranslateError::InvalidChoice {
            what,
            value: value.to_owned(),
            choices: choices.join(", "),
        })
}

/// `Enum.KeyCode.<name>`, for a key's name such as `Space`.
pub(super) fn key(name: &str) -> Result<String, TranslateError> {
    if !valid_identifier(name) {
        return Err(TranslateError::InvalidKey(name.to_owned()));
    }
    Ok(format!("Enum.KeyCode.{name}"))
}

/// The Lua for an object at a dotted path under the Workspace, such as
/// `workspace.Pad.Chime`. Names that aren't Lua names are bracketed:
/// `workspace["Red Ball"]`.
pub(super) fn target(path: &str) -> Result<String, TranslateError> {
    let parts = path.split('.').collect::<Vec<_>>();
    if parts.iter().any(|part| part.is_empty()) {
        return Err(TranslateError::InvalidTarget(path.to_owned()));
    }
    Ok(parts
        .into_iter()
        .fold("workspace".to_owned(), |prefix, part| {
            if valid_identifier(part) {
                format!("{prefix}.{part}")
            } else {
                format!("{prefix}[{}]", quote(part))
            }
        }))
}

/// An object in ReplicatedStorage, such as a BindableEvent, by its dotted
/// path there: `Events.Scored` is `game.ReplicatedStorage.Events.Scored`.
pub(super) fn stored(path: &str) -> Result<String, TranslateError> {
    Ok(target(path)?.replacen("workspace", "game.ReplicatedStorage", 1))
}

/// Like [`target`], with an empty path meaning the Workspace itself.
pub(super) fn container(path: &str) -> Result<String, TranslateError> {
    if path.is_empty() {
        Ok("workspace".to_owned())
    } else {
        target(path)
    }
}

pub(super) fn quote(text: &str) -> String {
    let mut escaped = String::from("\"");
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\0' => escaped.push_str("\\000"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\{:03}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

/// Whether `name` can be a Lua variable: letters, digits and underscores,
/// not starting with a digit, and not a Lua keyword.
/// `name` quoted, if it's an attribute name the engine takes (the rule of
/// `core::is_attribute_name`).
fn attribute_name(name: &str) -> Result<String, TranslateError> {
    let valid = !name.is_empty()
        && name.len() <= 100
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.starts_with("RBX");
    if valid {
        Ok(quote(name))
    } else {
        Err(TranslateError::InvalidAttribute(name.to_owned()))
    }
}

pub fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
        && !matches!(
            name,
            "and"
                | "break"
                | "do"
                | "else"
                | "elseif"
                | "end"
                | "false"
                | "for"
                | "function"
                | "goto"
                | "if"
                | "in"
                | "local"
                | "nil"
                | "not"
                | "or"
                | "repeat"
                | "return"
                | "then"
                | "true"
                | "until"
                | "while"
        )
}

/// Whether an expression is statically known to evaluate to a scalar `number`,
/// used by `Expression::Lerp` to emit scalar interpolation `a + (b - a) * t`
/// instead of `:Lerp(b, t)`.
fn is_scalar_expression(expr: &Expression) -> bool {
    matches!(
        expr,
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
            | Expression::SplineClosestDistance { .. }
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn quote_escapes_quote_marks() {
        assert_eq!(super::quote("sound\"path"), "\"sound\\\"path\"");
    }
}
