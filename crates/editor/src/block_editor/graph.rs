//! The graph view of a block script: Unreal Blueprint-style nodes and wires.
//!
//! It shows the same blocks as the block canvas, so a script switches
//! between Code, Blocks and Graph without losing anything. Each block is a
//! node. White exec wires say what runs next: an event's wire leads to the
//! first block of its stack, a block's Then to the block after it, and a
//! loop's Loop Body or a Branch's True and False to the blocks in its
//! mouths. Reporters are value nodes, wired into the inputs they feed;
//! values typed in stay on the pin. Nodes not wired to an event don't run,
//! like loose blocks.
//!
//! It draws on the shared graph canvas (`node_graph::canvas`, Phase 91):
//! right-click the canvas, press Space or drop a wire on it for the
//! searchable node menu. Drag from a pin to wire it, Alt-click a pin to
//! break its wires. Left-drag the background to select a box of nodes,
//! right-drag (or the wheel) to pan, Ctrl + wheel to zoom and F to frame
//! the selection. Delete deletes the selected nodes, Ctrl+D duplicates
//! them, and Ctrl+C, X and V copy, cut and paste them as Lua.
//!
//! Where to change behavior:
//! - `edit.rs` — what wiring, breaking and deleting does to the blocks
//! - `nodes.rs` — node titles, pin names and colours
//! - `layout.rs` — where nodes go before they're dragged
//! - `menu.rs` — the node menu
//! - this file — measuring, drawing and the pointer

use std::collections::{BTreeMap, HashMap, HashSet};

use axcenity_blocks::{Graph, LooseValue};

use super::*;
use crate::node_graph::canvas::{
    self as graph_canvas, BoxSelect, Canvas, MenuChoice, PinAction, draw_exec_pin, draw_value_pin,
    draw_wire,
};

mod edit;
mod layout;
mod menu;
mod nodes;

use edit::*;
use layout::*;
use menu::*;
use nodes::*;

// Sizes at zoom 1, in graph units (points).
pub(super) const HEADER: f32 = 28.0;
const ROW: f32 = 26.0;
const PIN: f32 = graph_canvas::PIN_RADIUS;
/// From a node's edge to the text beside a pin.
const PIN_SPACE: f32 = 22.0;
const MIDDLE_GAP: f32 = 24.0;
const MIN_NODE_WIDTH: f32 = 130.0;
const NAME_GAP: f32 = 6.0;
/// How near a pin a wire has to be let go to join it.
const PIN_REACH: f32 = 14.0;
const MIN_ZOOM: f32 = 0.3;
const MAX_ZOOM: f32 = 2.0;

/// Which mouth or follow-on an exec output is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Out {
    /// What runs in mouth `n` (an event's stack is its mouth 0).
    Body(usize),
    /// The block after this one.
    Then,
}

/// Where a value is: its owner (a block, or a loose value node's id) and
/// the slots down to it. A loose value node itself has an empty path.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct ValueAt {
    pub(super) owner: u64,
    pub(super) path: Vec<usize>,
}

impl ValueAt {
    pub(super) fn loose(id: u64) -> Self {
        Self {
            owner: id,
            path: Vec::new(),
        }
    }

    pub(super) fn child(&self, index: usize) -> Self {
        let mut path = self.path.clone();
        path.push(index);
        Self {
            owner: self.owner,
            path,
        }
    }

    pub(super) fn parent(&self) -> Option<Self> {
        let mut path = self.path.clone();
        path.pop()?;
        Some(Self {
            owner: self.owner,
            path,
        })
    }

    /// How [`Graph::nodes`] names it: `"12.0.1"`.
    pub(super) fn key(&self) -> String {
        let mut key = self.owner.to_string();
        for index in &self.path {
            key.push_str(&format!(".{index}"));
        }
        key
    }

    /// Whether this is `other` or a slot inside it.
    pub(super) fn within(&self, other: &ValueAt) -> bool {
        self.owner == other.owner && self.path.starts_with(&other.path)
    }
}

/// A node: a block, or a value node (a reporter).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum NodeRef {
    Block(u64),
    Value(ValueAt),
}

impl NodeRef {
    pub(super) fn key(&self) -> String {
        match self {
            NodeRef::Block(id) => id.to_string(),
            NodeRef::Value(at) => at.key(),
        }
    }

    /// What this node's input slots are numbered under.
    fn slots(&self) -> ValueAt {
        match self {
            NodeRef::Block(id) => ValueAt::loose(*id),
            NodeRef::Value(at) => at.clone(),
        }
    }
}

/// A pin on a node.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Pin {
    ExecIn(u64),
    ExecOut(u64, Out),
    /// A value slot.
    Input(ValueAt),
    /// A value node's output.
    Output(ValueAt),
    /// A block node's right-side data output pin (`block_id`, `output_index`).
    BlockOutput(u64, usize),
}

impl Pin {
    fn exec(&self) -> bool {
        matches!(self, Pin::ExecIn(_) | Pin::ExecOut(..))
    }
}

/// What the graph view remembers between frames for one script.
#[derive(Clone, Debug)]
struct GraphState {
    /// Pan, zoom, the selection box and notices; `state.pan` and
    /// `state.zoom` reach it through `Deref`.
    canvas: Canvas,
    show_lua: bool,
    selected: HashSet<NodeRef>,
    /// A wire being dragged out of this pin.
    wire: Option<Pin>,
    menu: Option<Menu>,
    /// Fit the selected nodes (or all) into view on the next frame.
    frame_next: bool,
}

impl std::ops::Deref for GraphState {
    type Target = Canvas;

    fn deref(&self) -> &Canvas {
        &self.canvas
    }
}

impl std::ops::DerefMut for GraphState {
    fn deref_mut(&mut self) -> &mut Canvas {
        &mut self.canvas
    }
}

impl Default for GraphState {
    fn default() -> Self {
        Self {
            canvas: Canvas::new(vec2(20.0, 20.0)),
            show_lua: false,
            selected: HashSet::new(),
            wire: None,
            menu: None,
            frame_next: false,
        }
    }
}

/// A node's measurements at zoom 1.
#[derive(Clone, Debug)]
struct Geometry {
    size: Vec2,
    title: String,
    icon: &'static str,
    category: Category,
    is_event: bool,
    layout: Layout,
    /// Each row's height.
    rows: Vec<f32>,
    /// Where each pin is, from the node's top-left corner.
    pins: Vec<(Pin, Vec2)>,
}

/// A node's script piece, borrowed for measuring or drawing.
enum Body<'a> {
    Block(&'a mut BlockKind),
    Value(&'a mut Expression),
}

/// Calls `visit` for every node, with whether it runs (is wired to an
/// event): events and the blocks after them, top-level Lua, and the value
/// nodes wired into those.
fn visit_nodes(script: &mut BlockScript, visit: &mut dyn FnMut(NodeRef, Body, bool)) {
    let mut block_ids = HashSet::new();
    script.for_each_block(&mut |b| {
        block_ids.insert(b.id);
    });
    fn values(
        at: ValueAt,
        value: &mut Expression,
        runs: bool,
        block_ids: &HashSet<u64>,
        visit: &mut dyn FnMut(NodeRef, Body, bool),
    ) {
        if let Expression::BlockOutput { block, .. } = value
            && block_ids.contains(block)
        {
            return;
        }
        visit(NodeRef::Value(at.clone()), Body::Value(value), runs);
        for (index, child) in value.children_mut().into_iter().enumerate() {
            if !child.is_literal() {
                values(at.child(index), child, runs, block_ids, visit);
            }
        }
    }
    fn visit_block(
        block: &mut Block,
        runs: bool,
        block_ids: &HashSet<u64>,
        visit: &mut dyn FnMut(NodeRef, Body, bool),
    ) {
        let id = block.id;
        visit(NodeRef::Block(id), Body::Block(&mut block.kind), runs);
        for (slot, value) in block.kind.expressions_mut().into_iter().enumerate() {
            if !value.is_literal() {
                values(
                    ValueAt {
                        owner: id,
                        path: vec![slot],
                    },
                    value,
                    runs,
                    block_ids,
                    visit,
                );
            }
        }
        for body in block.kind.bodies_mut() {
            for inner in body {
                visit_block(inner, runs, block_ids, visit);
            }
        }
    }
    for top in &mut script.blocks {
        match &mut top.kind {
            BlockKind::Unattached { body } => {
                for inner in body {
                    visit_block(inner, false, &block_ids, visit);
                }
            }
            _ => visit_block(top, true, &block_ids, visit),
        }
    }
    for loose in &mut script.graph.loose {
        values(
            ValueAt::loose(loose.id),
            &mut loose.value,
            false,
            &block_ids,
            visit,
        );
    }
}

/// Which exec and block-output pins have wires: each output's target, and the
/// blocks with a wire coming in.
#[derive(Default)]
struct Wiring {
    targets: HashMap<(u64, Out), u64>,
    fed: HashSet<u64>,
    block_outputs_fed: HashSet<(u64, usize)>,
}

fn wiring(script: &BlockScript) -> Wiring {
    fn scan_expr(expr: &Expression, wiring: &mut Wiring) {
        if let Expression::BlockOutput { block, output, .. } = expr {
            wiring.block_outputs_fed.insert((*block, *output));
        }
        for child in expr.children() {
            scan_expr(child, wiring);
        }
    }
    fn stack(blocks: &[Block], wiring: &mut Wiring, loose: bool) {
        for (index, block) in blocks.iter().enumerate() {
            if index > 0 || !loose {
                wiring.fed.insert(block.id);
            }
            if let Some(next) = blocks.get(index + 1) {
                wiring.targets.insert((block.id, Out::Then), next.id);
            }
            for expr in block.kind.expressions() {
                scan_expr(expr, wiring);
            }
            mouths(block, wiring);
        }
    }
    fn mouths(block: &Block, wiring: &mut Wiring) {
        for expr in block.kind.expressions() {
            scan_expr(expr, wiring);
        }
        for (mouth, body) in block.kind.bodies().into_iter().enumerate() {
            if let Some(first) = body.first() {
                wiring
                    .targets
                    .insert((block.id, Out::Body(mouth)), first.id);
            }
            stack(body, wiring, false);
        }
    }
    let mut wiring = Wiring::default();
    for top in &script.blocks {
        match &top.kind {
            BlockKind::Unattached { body } => stack(body, &mut wiring, true),
            _ => mouths(top, &mut wiring),
        }
    }
    for loose in &script.graph.loose {
        scan_expr(&loose.value, &mut wiring);
    }
    wiring
}

/// The title, colour, icon, event flag and rows of a node.
fn describe(
    node: &NodeRef,
    body: &Body,
    objects: &[ObjectInfo],
    clone_ids: &HashMap<u64, (String, String)>,
) -> (String, Category, &'static str, bool, Layout) {
    match body {
        Body::Block(kind) => {
            let block_id = match node {
                NodeRef::Block(id) => *id,
                NodeRef::Value(_) => 0,
            };
            let mut probe = (*kind).clone();
            let layout = block_layout(kind, block_id, &block_parts(&mut probe, objects), clone_ids);
            (
                block_title(kind),
                block_category(kind),
                block_icon(kind),
                kind.is_event(),
                layout,
            )
        }
        Body::Value(expression) => {
            let mut probe = (*expression).clone();
            let layout = value_layout(expression, &reporter_parts(&mut probe, objects));
            (
                value_names(expression).0,
                reporter_category(expression),
                reporter_icon(expression),
                false,
                layout,
            )
        }
    }
}

/// The parts of a node's piece, to measure and draw its menus and inputs.
fn parts_of<'a>(body: &'a mut Body, objects: &[ObjectInfo]) -> Vec<Part<'a>> {
    match body {
        Body::Block(kind) => block_parts(kind, objects),
        Body::Value(expression) => reporter_parts(expression, objects),
    }
}

/// How big an item is, at the painter's zoom.
fn item_size(painter: &BlockPainter, ui: &egui::Ui, item: &Item, parts: &mut [Part]) -> Vec2 {
    let s = |value: f32| painter.s(value);
    match item {
        Item::ExecIn | Item::Output(_) => vec2(s(PIN_SPACE), s(ROW)),
        Item::ExecOut(_, name) => vec2(painter.text_width(ui, name) + s(PIN_SPACE + 4.0), s(ROW)),
        Item::BlockOutput { name, .. } => {
            vec2(painter.text_width(ui, name) + s(PIN_SPACE + 4.0), s(ROW))
        }
        Item::Input { part, name, .. } => {
            let mut width = s(PIN_SPACE) + painter.text_width(ui, name);
            let mut height = s(ROW);
            if let Some(Part::Slot(value, kind)) = parts.get(*part)
                && value.is_literal()
            {
                let literal = painter.literal_size(ui, value, *kind);
                width += s(NAME_GAP) + literal.x;
                height = height.max(literal.y + s(6.0));
            }
            vec2(width + s(4.0), height)
        }
        Item::Field { part, name } => {
            let field = parts
                .get_mut(*part)
                .map_or(Vec2::ZERO, |part| painter.part_size(ui, part));
            vec2(
                s(10.0) + painter.text_width(ui, name) + s(NAME_GAP) + field.x + s(4.0),
                s(ROW).max(field.y + s(6.0)),
            )
        }
        Item::Code { part } => {
            let code = parts
                .get_mut(*part)
                .map_or(Vec2::ZERO, |part| painter.part_size(ui, part));
            code + vec2(s(20.0), s(8.0))
        }
    }
}

/// Measures a node at zoom 1.
#[allow(clippy::too_many_arguments)]
fn measure(
    painter: &BlockPainter,
    ui: &egui::Ui,
    node: &NodeRef,
    title: String,
    category: Category,
    icon: &'static str,
    is_event: bool,
    layout: Layout,
    parts: &mut [Part],
) -> Geometry {
    let header_label = theme::with_icon(icon, &title);
    let mut width = painter.text_width(ui, &header_label) + 2.0 * 12.0;
    let mut rows = Vec::new();
    for row in 0..layout.rows() {
        let left = layout
            .left
            .get(row)
            .map_or(Vec2::ZERO, |item| item_size(painter, ui, item, parts));
        let right = layout
            .right
            .get(row)
            .map_or(Vec2::ZERO, |item| item_size(painter, ui, item, parts));
        width = width.max(left.x + MIDDLE_GAP + right.x);
        rows.push(left.y.max(right.y).max(ROW));
    }
    let width = width.max(MIN_NODE_WIDTH);
    let mut pins = Vec::new();
    let mut y = HEADER + 2.0;
    let slots = node.slots();
    for (row, height) in rows.iter().enumerate() {
        let middle = y + height * 0.5;
        match layout.left.get(row) {
            Some(Item::ExecIn) => {
                if let NodeRef::Block(id) = node {
                    pins.push((Pin::ExecIn(*id), vec2(PIN + 6.0, middle)));
                }
            }
            Some(Item::Input { slot, .. }) => {
                pins.push((Pin::Input(slots.child(*slot)), vec2(PIN + 6.0, middle)));
            }
            _ => {}
        }
        match layout.right.get(row) {
            Some(Item::ExecOut(out, _)) => {
                if let NodeRef::Block(id) = node {
                    pins.push((Pin::ExecOut(*id, *out), vec2(width - PIN - 6.0, middle)));
                }
            }
            Some(Item::Output(_)) => {
                if let NodeRef::Value(at) = node {
                    pins.push((Pin::Output(at.clone()), vec2(width - PIN - 6.0, middle)));
                }
            }
            Some(Item::BlockOutput { index, .. }) => {
                if let NodeRef::Block(id) = node {
                    pins.push((
                        Pin::BlockOutput(*id, *index),
                        vec2(width - PIN - 6.0, middle),
                    ));
                }
            }
            _ => {}
        }
        y += height;
    }
    Geometry {
        size: vec2(width, y + 8.0),
        title,
        icon,
        category,
        is_event,
        layout,
        rows,
        pins,
    }
}

/// Every node's measurements at zoom 1.
fn measure_all(
    ui: &egui::Ui,
    script: &mut BlockScript,
    context: &BlockContext,
    variables: &[String],
) -> HashMap<NodeRef, Geometry> {
    let clone_ids = clone_identifiers(script);
    let painter = BlockPainter::new(
        ui.painter().clone(),
        1.0,
        false,
        theme::canvas(ui.ctx()).node_body,
        egui::Id::new("graph measure"),
        context,
        variables,
    );
    let mut geometry = HashMap::new();
    visit_nodes(script, &mut |node, mut body, _| {
        let (title, category, icon, is_event, layout) =
            describe(&node, &body, context.objects, &clone_ids);
        let mut parts = parts_of(&mut body, context.objects);
        let measured = measure(
            &painter, ui, &node, title, category, icon, is_event, layout, &mut parts,
        );
        geometry.insert(node, measured);
    });
    geometry
}

/// Something done with the pointer or keys this frame, applied once
/// everything is drawn.
enum Event {
    WireFrom(Pin),
    BreakPin(Pin),
    Select(NodeRef, bool),
    MoveStart(NodeRef),
    Move(Vec2),
    Duplicate(NodeRef),
    /// One more input (true) or one fewer on a call's or a local's node.
    Resize(NodeRef, bool),
    /// Toggle optional Position input on a `CloneInto` block.
    ToggleClonePosition(u64),
    /// Toggle collapsible Advanced settings (Offset / Per-Clone Positions) on `CloneInto`.
    ToggleCloneAdvanced(u64),
    /// Toggle an optional input/flag on an Event or Control Flow block.
    ToggleOptionalPin(u64, &'static str),
    ToggleRaycastIgnore(NodeRef),
    /// Add or remove a `Then N` branch on a `Sequence` block.
    ResizeSequence(u64, bool),
    Delete(NodeRef),
}

/// Where the graph view with `key` is looking (pan and zoom), for tests.
#[cfg(test)]
pub(crate) fn view_of(ctx: &egui::Context, key: &str) -> (Vec2, f32) {
    let state: GraphState = ctx
        .data_mut(|data| data.get_temp(egui::Id::new(("graph editor", key))))
        .unwrap_or_default();
    (state.pan, state.zoom)
}

/// Draws the graph view of `script`. Returns true if the script changed.
pub fn show_graph(ui: &mut egui::Ui, script: &mut BlockScript, context: BlockContext) -> bool {
    let before = script.clone();
    let state_id = egui::Id::new(("graph editor", context.key));
    let mut state: GraphState = ui
        .data_mut(|data| data.get_temp(state_id))
        .unwrap_or_default();
    let variables = known_variables(script, &[]);

    let full = ui.available_rect_before_wrap();
    let toolbar = Rect::from_min_size(full.min, vec2(full.width(), TOOLBAR_HEIGHT));
    let body = Rect::from_min_max(pos2(full.left(), toolbar.bottom()), full.max);
    let lua_width = if state.show_lua {
        (body.width() * 0.3).clamp(260.0, 460.0)
    } else {
        0.0
    };
    let canvas = Rect::from_min_max(body.min, pos2(body.right() - lua_width, body.bottom()));
    let lua_rect = Rect::from_min_max(pos2(canvas.right(), body.top()), body.max);

    let add_clicked = show_graph_toolbar(ui, toolbar, &mut state, &context, script);
    show_graph_canvas(
        ui,
        canvas,
        script,
        &mut state,
        &context,
        &variables,
        add_clicked,
    );
    if state.show_lua {
        show_lua(ui, lua_rect, script);
    }
    ui.advance_cursor_after_rect(full);
    ui.data_mut(|data| data.insert_temp(state_id, state));
    *script != before
}

/// The bar over the graph. Returns true when Add Node was clicked.
fn show_graph_toolbar(
    ui: &mut egui::Ui,
    rect: Rect,
    state: &mut GraphState,
    context: &BlockContext,
    script: &mut BlockScript,
) -> bool {
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
    let (icon, side, color) = if context.client {
        (icons::USER, "Client graph", palette.success)
    } else {
        (icons::CLOUD, "Server graph", palette.accent)
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
        });
    let mut add = false;
    if bar
        .button(theme::with_icon(icons::PLUS, "Add node"))
        .on_hover_text("Or right-click the graph, or drop a wire on it")
        .clicked()
    {
        add = true;
    }
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
            .on_hover_text("Shows the Lua this graph becomes. It runs exactly like a typed script.")
            .clicked()
        {
            state.show_lua = !state.show_lua;
        }
        ui.separator();
        if graph_canvas::zoom_buttons(ui, &mut state.canvas, MIN_ZOOM..=MAX_ZOOM) {
            state.selected.clear();
            state.frame_next = true;
        }
        if ui
            .button(worded(icons::BROOM, "Clean up"))
            .on_hover_text("Lays every node out again, left to right")
            .clicked()
        {
            script.graph.nodes.clear();
            state.canvas.pan = GraphState::default().pan;
        }
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add(egui::Label::new(egui::RichText::new(GRAPH_TOOLBAR_HINT).weak()).truncate())
                    .on_hover_text(GRAPH_TOOLBAR_HINT);
            },
        );
    });
    add
}

/// What the Graph toolbar says between Add node and its buttons.
const GRAPH_TOOLBAR_HINT: &str = "Drag from a pin to wire it. Right-drag to pan.";

/// Stores every node's place, so nothing moves when an edit rewires them.
fn freeze(graph: &mut Graph, places: &HashMap<NodeRef, Pos2>) {
    graph.nodes = places
        .iter()
        .map(|(node, at)| (node.key(), [at.x.round(), at.y.round()]))
        .collect::<BTreeMap<_, _>>();
}

/// The canvas with the nodes and wires.
fn show_graph_canvas(
    ui: &mut egui::Ui,
    rect: Rect,
    script: &mut BlockScript,
    state: &mut GraphState,
    context: &BlockContext,
    variables: &[String],
    add_clicked: bool,
) {
    let colors = theme::canvas(ui.ctx());
    let mut canvas = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    canvas.set_clip_rect(rect);
    let painter = canvas.painter_at(rect);

    let geometry = measure_all(&canvas, script, context, variables);
    let sizes: HashMap<NodeRef, Vec2> = geometry
        .iter()
        .map(|(node, geometry)| (node.clone(), geometry.size))
        .collect();
    let places = layout(script, &sizes);
    let node_area = |node: &NodeRef| Some(Rect::from_min_size(*places.get(node)?, sizes[node]));
    if std::mem::take(&mut state.frame_next) {
        let area = sizes
            .keys()
            .filter(|node| state.selected.is_empty() || state.selected.contains(*node))
            .filter_map(node_area)
            .reduce(|a, b| a.union(b));
        state.canvas.fit(rect, area, MIN_ZOOM, 1.0);
    }

    // The shared canvas: the background and grid, pan and zoom.
    let frame = state.canvas.begin(
        &mut canvas,
        rect,
        egui::Id::new(("graph canvas", context.key)),
        MIN_ZOOM..=MAX_ZOOM,
        state.menu.is_none(),
    );
    let (z, hovered, pointer) = (frame.zoom, frame.hovered, frame.pointer);
    let to_screen = |at: Pos2| frame.to_screen(at);
    let to_graph = |at: Pos2| frame.to_graph(at);

    let wired = wiring(script);
    let pin_at = |pin: &Pin| -> Option<Pos2> {
        geometry.iter().find_map(|(node, geometry)| {
            let place = places.get(node)?;
            geometry
                .pins
                .iter()
                .find(|(candidate, _)| candidate == pin)
                .map(|(_, offset)| to_screen(*place + *offset))
        })
    };
    let pins: Vec<(Pin, Pos2)> = geometry
        .iter()
        .filter_map(|(node, geometry)| Some((places.get(node)?, geometry)))
        .flat_map(|(place, geometry)| {
            geometry
                .pins
                .iter()
                .map(move |(pin, offset)| (pin.clone(), to_screen(*place + *offset)))
        })
        .collect();

    if script.blocks.is_empty() && script.graph.loose.is_empty() {
        graph_canvas::empty_hint(
            &canvas,
            rect,
            "Right-click to add a node, such as \"When Game Starts\".",
        );
    }

    // Wires, under the nodes. Exec wires are white, or dark on a light
    // canvas.
    let exec_wire = colors.wire;
    for ((from, out), to) in &wired.targets {
        if let (Some(start), Some(end)) = (
            pin_at(&Pin::ExecOut(*from, *out)),
            pin_at(&Pin::ExecIn(*to)),
        ) {
            draw_wire(&painter, start, end, exec_wire, 2.5 * z);
        }
    }
    for (pin, end) in &pins {
        if let Pin::Input(at) = pin
            && let Some(value) = value(script, at)
            && !value.is_literal()
        {
            if let Expression::BlockOutput { block, output, .. } = value {
                if let Some(start) = pin_at(&Pin::BlockOutput(*block, *output)) {
                    let color = slot_kind(script, at).map_or(exec_wire, pin_color);
                    draw_wire(&painter, start, *end, color, 2.0 * z);
                }
            } else if let Some(start) = pin_at(&Pin::Output(at.clone())) {
                draw_wire(&painter, start, *end, pin_color(value_type(value)), 2.0 * z);
            }
        }
    }

    // The nodes.
    let mut drawer = BlockPainter::new(
        painter.clone(),
        z,
        state.menu.is_none(),
        colors.node_body,
        egui::Id::new(("graph", context.key)),
        context,
        variables,
    );
    let mut events = Vec::new();
    let selected = state.selected.clone();
    let wire_held = state.wire.is_some();
    visit_nodes(script, &mut |node, mut body, runs| {
        let (Some(geometry), Some(place)) = (geometry.get(&node), places.get(&node)) else {
            return;
        };
        let top_left = to_screen(*place);
        let node_rect = Rect::from_min_size(top_left, geometry.size * z);
        if !node_rect.intersects(rect) {
            return;
        }
        let highlighted = matches!(node, NodeRef::Block(id) if context.highlighted == Some(id));
        draw_node(
            &mut canvas,
            &mut drawer,
            NodeDraw {
                node: &node,
                body: &mut body,
                geometry,
                rect: node_rect,
                runs,
                selected: selected.contains(&node),
                highlighted,
                wired: &wired,
                wire_held,
                objects: context.objects,
            },
            &mut events,
        );
    });

    // The wire being dragged, from its pin to the pointer.
    if let (Some(from), Some(pointer)) = (&state.wire, pointer)
        && let Some(start) = pin_at(from)
    {
        let color = match from {
            Pin::Input(at) | Pin::Output(at) => slot_kind(script, at)
                .or_else(|| value(script, at).map(value_type))
                .map_or(exec_wire, pin_color),
            _ => exec_wire,
        };
        let (a, b) = if matches!(from, Pin::ExecIn(_) | Pin::Input(_)) {
            (pointer, start)
        } else {
            (start, pointer)
        };
        draw_wire(&painter, a, b, color, 2.0 * z);
    }

    // The box being dragged out.
    match state.canvas.box_select(&canvas, &frame, true) {
        BoxSelect::Started { add: false } => state.selected.clear(),
        BoxSelect::Done(area) => {
            for (node, geometry) in &geometry {
                if let Some(place) = places.get(node) {
                    let node_rect = Rect::from_min_size(to_screen(*place), geometry.size * z);
                    if area.intersects(node_rect) {
                        state.selected.insert(node.clone());
                    }
                }
            }
        }
        _ => {}
    }
    if frame.back.clicked() {
        state.selected.clear();
        state.menu = None;
    }

    let mut edited = false;
    let mut moved = Vec2::ZERO;
    for event in events {
        match event {
            Event::WireFrom(pin) => state.wire = Some(pin),
            Event::BreakPin(pin) => {
                freeze(&mut script.graph, &places);
                break_pin(script, &pin);
                edited = true;
            }
            Event::Select(node, add) => {
                if !add {
                    state.selected.clear();
                }
                if add && state.selected.contains(&node) {
                    state.selected.remove(&node);
                } else {
                    state.selected.insert(node);
                }
            }
            Event::MoveStart(node) => {
                if !state.selected.contains(&node) {
                    state.selected.clear();
                    state.selected.insert(node);
                }
                freeze(&mut script.graph, &places);
            }
            Event::Move(delta) => moved += delta,
            Event::Resize(node, grow) => {
                freeze(&mut script.graph, &places);
                resize_inputs(script, &node, grow);
                edited = true;
            }
            Event::ToggleClonePosition(block_id) => {
                freeze(&mut script.graph, &places);
                if let Some(block) = script.find_mut(block_id)
                    && let BlockKind::CloneInto { position, .. } = &mut block.kind
                {
                    if position.is_some() {
                        *position = None;
                    } else {
                        *position = Some(Expression::Vector3 {
                            x: 0.0,
                            y: 5.0,
                            z: 0.0,
                        });
                    }
                    edited = true;
                }
            }
            Event::ToggleCloneAdvanced(block_id) => {
                freeze(&mut script.graph, &places);
                if let Some(block) = script.find_mut(block_id)
                    && let BlockKind::CloneInto {
                        offset_per_clone,
                        per_clone_positions,
                        ..
                    } = &mut block.kind
                {
                    if offset_per_clone.is_some() || per_clone_positions.is_some() {
                        *offset_per_clone = None;
                        *per_clone_positions = None;
                    } else {
                        *offset_per_clone = Some(Expression::Vector3 {
                            x: 5.0,
                            y: 0.0,
                            z: 0.0,
                        });
                        *per_clone_positions = Some(Expression::Nil);
                    }
                    edited = true;
                }
            }
            Event::ToggleRaycastIgnore(node) => {
                freeze(&mut script.graph, &places);
                if let NodeRef::Value(at) = &node
                    && let Some(Expression::Raycast { ignore, .. }) = value_mut(script, at)
                {
                    *ignore = if ignore.is_some() {
                        None
                    } else {
                        Some(ObjectRef::default())
                    };
                    edited = true;
                }
            }
            Event::ToggleOptionalPin(block_id, which_pin) => {
                freeze(&mut script.graph, &places);
                if let Some(block) = script.find_mut(block_id) {
                    match (&mut block.kind, which_pin) {
                        (
                            BlockKind::WhenTouched { cooldown, .. }
                            | BlockKind::WhenPromptTriggered { cooldown, .. }
                            | BlockKind::WhenPartClicked { cooldown, .. },
                            "cooldown",
                        ) => {
                            *cooldown = if cooldown.is_some() {
                                None
                            } else {
                                Some(Expression::Number { value: 0.5 })
                            };
                            edited = true;
                        }
                        (BlockKind::WhenCommandRun { admin_only, .. }, "admin_only") => {
                            *admin_only = if admin_only.is_some() {
                                None
                            } else {
                                Some(Expression::Boolean { value: true })
                            };
                            edited = true;
                        }
                        (BlockKind::FireEvent { data, .. }, "data") => {
                            *data = if data.is_some() {
                                None
                            } else {
                                Some(Expression::Text {
                                    value: String::new(),
                                })
                            };
                            edited = true;
                        }
                        (BlockKind::WaitForChild { timeout, .. }, "timeout") => {
                            *timeout = if timeout.is_some() {
                                None
                            } else {
                                Some(Expression::Number { value: 5.0 })
                            };
                            edited = true;
                        }
                        (
                            BlockKind::ForEachPartInRadius { players_only, .. }
                            | BlockKind::ForEachPartTouching { players_only, .. },
                            "players_only",
                        ) => {
                            *players_only = !*players_only;
                            edited = true;
                        }
                        (
                            BlockKind::SlideTo {
                                wait_until_done, ..
                            }
                            | BlockKind::RotateTo {
                                wait_until_done, ..
                            }
                            | BlockKind::TweenProperty {
                                wait_until_done, ..
                            }
                            | BlockKind::PlaySound {
                                wait_until_done, ..
                            }
                            | BlockKind::FadeVolume {
                                wait_until_done, ..
                            }
                            | BlockKind::PlayAnimation {
                                wait_until_done, ..
                            }
                            | BlockKind::Tween {
                                wait_until_done, ..
                            }
                            | BlockKind::SlideIn {
                                wait_until_done, ..
                            }
                            | BlockKind::SlideOut {
                                wait_until_done, ..
                            }
                            | BlockKind::FadeIn {
                                wait_until_done, ..
                            }
                            | BlockKind::FadeOut {
                                wait_until_done, ..
                            }
                            | BlockKind::TweenPivot {
                                wait_until_done, ..
                            }
                            | BlockKind::PulseScale {
                                wait_until_done, ..
                            },
                            "wait_until_done",
                        ) => {
                            *wait_until_done = !*wait_until_done;
                            edited = true;
                        }
                        (
                            BlockKind::PlayAnimation { fade_time, .. }
                            | BlockKind::StopAnimation { fade_time, .. },
                            "fade_time",
                        ) => {
                            *fade_time = if fade_time.is_some() {
                                None
                            } else {
                                Some(Expression::Number { value: 0.2 })
                            };
                            edited = true;
                        }
                        (BlockKind::PlayAnimation { speed, .. }, "speed") => {
                            *speed = if speed.is_some() {
                                None
                            } else {
                                Some(Expression::Number { value: 1.0 })
                            };
                            edited = true;
                        }
                        (
                            BlockKind::PlayAnimation { weight, .. }
                            | BlockKind::SetIkTarget { weight, .. },
                            "weight",
                        ) => {
                            *weight = if weight.is_some() {
                                None
                            } else {
                                Some(Expression::Number { value: 1.0 })
                            };
                            edited = true;
                        }
                        (BlockKind::StopAnimation { animation, .. }, "animation") => {
                            *animation = if animation.is_some() {
                                None
                            } else {
                                Some(ObjectRef::path("Model.Animation"))
                            };
                            edited = true;
                        }
                        (BlockKind::Tween { repeat_count, .. }, "repeat_count") => {
                            *repeat_count = if repeat_count.is_some() {
                                None
                            } else {
                                Some(Expression::Number { value: -1.0 })
                            };
                            edited = true;
                        }
                        (BlockKind::Tween { reverses, .. }, "reverses") => {
                            *reverses = !*reverses;
                            edited = true;
                        }
                        (
                            BlockKind::SetClockTime { duration, .. }
                            | BlockKind::SetAtmosphere { duration, .. },
                            "duration",
                        ) => {
                            *duration = if duration.is_some() {
                                None
                            } else {
                                Some(Expression::Number { value: 2.0 })
                            };
                            edited = true;
                        }
                        (
                            BlockKind::PlaySound { pitch, .. }
                            | BlockKind::PlaySoundAtPosition { pitch, .. },
                            "pitch",
                        ) => {
                            *pitch = if pitch.is_some() {
                                None
                            } else {
                                Some(Expression::Number { value: 1.0 })
                            };
                            edited = true;
                        }
                        (BlockKind::PlaySoundAtPosition { volume, .. }, "volume") => {
                            *volume = if volume.is_some() {
                                None
                            } else {
                                Some(Expression::Number { value: 1.0 })
                            };
                            edited = true;
                        }
                        (
                            BlockKind::ForEachPartInRadius { ignore, .. }
                            | BlockKind::ForEachPartTouching { ignore, .. },
                            "ignore",
                        ) => {
                            *ignore = if ignore.is_some() {
                                None
                            } else {
                                Some(Expression::Nil)
                            };
                            edited = true;
                        }
                        _ => {}
                    }
                }
            }
            Event::ResizeSequence(block_id, grow) => {
                freeze(&mut script.graph, &places);
                if let Some(block) = script.find_mut(block_id)
                    && let BlockKind::Sequence { branches } = &mut block.kind
                {
                    if grow && branches.len() < 12 {
                        branches.push(Vec::new());
                        edited = true;
                    } else if !grow && branches.len() > 2 {
                        branches.pop();
                        edited = true;
                    }
                }
            }
            Event::Duplicate(node) => {
                freeze(&mut script.graph, &places);
                duplicate_at(script, &node, &places);
                edited = true;
            }
            Event::Delete(node) => {
                freeze(&mut script.graph, &places);
                let mut nodes: Vec<NodeRef> = state.selected.drain().collect();
                if !nodes.contains(&node) {
                    nodes = vec![node];
                }
                delete_nodes(script, &nodes);
                edited = true;
            }
        }
    }
    if moved != Vec2::ZERO {
        for node in &state.selected {
            let key = node.key();
            if let Some(at) = script.graph.nodes.get_mut(&key) {
                at[0] += moved.x / z;
                at[1] += moved.y / z;
            }
        }
    }

    // Letting go of a wire: on a pin, it joins; on the canvas, it opens
    // the node menu to make a node for it.
    if state.wire.is_some() && canvas.input(|i| i.pointer.any_released()) {
        let from = state.wire.take().expect("checked above");
        let target = pointer.and_then(|pointer| {
            pins.iter()
                .filter(|(pin, at)| {
                    *pin != from
                        && pin.exec() == from.exec()
                        && at.distance(pointer) <= PIN_REACH * z.max(0.6)
                })
                .min_by(|a, b| a.1.distance(pointer).total_cmp(&b.1.distance(pointer)))
                .map(|(pin, _)| pin.clone())
        });
        match target {
            Some(to) => {
                freeze(&mut script.graph, &places);
                match join(script, &from, &to) {
                    Ok(()) => edited = true,
                    Err(why) => state.canvas.say(canvas.ctx(), why),
                }
            }
            None => {
                if let Some(pointer) = pointer
                    && rect.contains(pointer)
                {
                    state.menu = Some(Menu::new(pointer, to_graph(pointer), Some(from)));
                }
            }
        }
    }

    // The keys every graph shares.
    if hovered && state.menu.is_none() {
        let keys = graph_canvas::keys(canvas.ctx());
        let picked: Vec<NodeRef> = state.selected.iter().cloned().collect();
        if (keys.copy || keys.cut)
            && let Some(text) = copy_nodes(script, &state.selected, &places)
        {
            canvas.ctx().copy_text(text);
        }
        if (keys.delete || keys.cut) && !picked.is_empty() {
            freeze(&mut script.graph, &places);
            state.selected.clear();
            delete_nodes(script, &picked);
            edited = true;
        }
        if keys.duplicate && !picked.is_empty() {
            freeze(&mut script.graph, &places);
            state.selected.clear();
            for node in picked {
                if let Some(copy) = duplicate_at(script, &node, &places) {
                    state.selected.insert(copy);
                }
            }
            edited = true;
        }
        if let Some(text) = keys.paste {
            freeze(&mut script.graph, &places);
            let pasted = paste_nodes(script, &text, frame.pointer_in_graph());
            if !pasted.is_empty() {
                state.selected = pasted.into_iter().collect();
                edited = true;
            }
        }
        if keys.select_all {
            state.selected = sizes.keys().cloned().collect();
        }
        if keys.escape {
            state.selected.clear();
        }
        if keys.frame {
            state.frame_next = true;
        }
        if keys.search {
            let at = frame.to_screen(frame.pointer_in_graph());
            state.menu = Some(Menu::new(at, to_graph(at), None));
        }
    }

    // The node menu.
    if frame.back.secondary_clicked()
        && let Some(pointer) = frame.back.interact_pointer_pos()
    {
        state.menu = Some(Menu::new(pointer, to_graph(pointer), None));
    }
    if add_clicked {
        state.menu = Some(Menu::new(rect.center(), to_graph(rect.center()), None));
    }
    if let Some(mut menu) = state.menu.take() {
        let items = menu_items(script, menu.from.as_ref(), context, variables);
        match show_menu(ui, &mut menu, &items, context.key) {
            MenuChoice::Open => state.menu = Some(menu),
            MenuChoice::Close => {}
            MenuChoice::Pick(index) => {
                freeze(&mut script.graph, &places);
                if let Some(node) = add_node(script, &items[index].item) {
                    let at = menu.menu.graph;
                    script
                        .graph
                        .nodes
                        .insert(node.key(), [at.x.round(), at.y.round()]);
                    if let Some(from) = &menu.from
                        && let Err(why) = join_new(script, from, &node)
                    {
                        state.canvas.say(canvas.ctx(), why);
                    }
                    state.selected = HashSet::from([node]);
                }
                edited = true;
            }
        }
    }

    // The minimap, once some nodes are out of view, and why the last wire
    // didn't join.
    let areas: Vec<Rect> = sizes.keys().filter_map(node_area).collect();
    state.canvas.minimap(&mut canvas, &frame, &areas);
    state.canvas.show_notice(&canvas, &frame);

    if edited {
        tidy(script);
        // Nodes that are gone don't keep their places.
        let keys: HashSet<String> = {
            let mut keys = HashSet::new();
            visit_nodes(script, &mut |node, _, _| {
                keys.insert(node.key());
            });
            keys
        };
        script.graph.nodes.retain(|key, _| keys.contains(key));
        state.selected.retain(|node| keys.contains(&node.key()));
    }
}

/// Joins a wire dragged from `from` and let go on `to`, in either order.
fn join(script: &mut BlockScript, from: &Pin, to: &Pin) -> Result<(), Refusal> {
    match (from, to) {
        (Pin::ExecOut(block, out), Pin::ExecIn(target))
        | (Pin::ExecIn(target), Pin::ExecOut(block, out)) => {
            connect_exec(script, *block, *out, *target)
        }
        (Pin::Output(value), Pin::Input(slot)) | (Pin::Input(slot), Pin::Output(value)) => {
            connect_value(script, value, slot)
        }
        (Pin::BlockOutput(block, output), Pin::Input(slot))
        | (Pin::Input(slot), Pin::BlockOutput(block, output)) => {
            connect_block_output(script, *block, *output, slot)
        }
        _ => Err("Wire an output to an input."),
    }
}

/// Wires a node just made from the menu to the pin its wire came from.
fn join_new(script: &mut BlockScript, from: &Pin, node: &NodeRef) -> Result<(), Refusal> {
    match (from, node) {
        (Pin::ExecOut(..), NodeRef::Block(id)) => join(script, from, &Pin::ExecIn(*id)),
        (Pin::ExecIn(_), NodeRef::Block(id)) => {
            let kind = &find_block(script, *id).ok_or("That node is gone.")?.kind;
            let (out, _) = *exec_outs(kind).first().ok_or("It has no exec output.")?;
            join(script, from, &Pin::ExecOut(*id, out))
        }
        (Pin::Input(_), NodeRef::Value(at)) => join(script, from, &Pin::Output(at.clone())),
        (Pin::Output(at), node) => {
            let shape = value(script, at).ok_or("That node is gone.")?.shape();
            let slots = match node {
                NodeRef::Block(id) => {
                    block_slots(&find_block(script, *id).ok_or("That node is gone.")?.kind)
                }
                NodeRef::Value(at) => value_slots(value(script, at).ok_or("That node is gone.")?),
            };
            let slot = slots
                .iter()
                .position(|kind| kind.accepts(shape))
                .ok_or("It has no input this value fits.")?;
            join(script, from, &Pin::Input(node.slots().child(slot)))
        }
        (Pin::BlockOutput(..), node) => {
            let slots = match node {
                NodeRef::Block(id) => {
                    block_slots(&find_block(script, *id).ok_or("That node is gone.")?.kind)
                }
                NodeRef::Value(at) => value_slots(value(script, at).ok_or("That node is gone.")?),
            };
            let slot = slots
                .iter()
                .position(|kind| kind.accepts(BlockShape::Value))
                .ok_or("It has no input this value fits.")?;
            join(script, from, &Pin::Input(node.slots().child(slot)))
        }
        _ => Err("That node can't be wired there."),
    }
}

/// Breaks every wire on `pin`.
fn break_pin(script: &mut BlockScript, pin: &Pin) {
    match pin {
        Pin::ExecIn(id) => break_exec_in(script, *id),
        Pin::ExecOut(id, out) => break_exec_out(script, *id, *out),
        Pin::Input(at) | Pin::Output(at) => break_value(script, at),
        Pin::BlockOutput(block, output) => break_block_output(script, *block, *output),
    }
}

/// Duplicates `node` a little below and right of it.
fn duplicate_at(
    script: &mut BlockScript,
    node: &NodeRef,
    places: &HashMap<NodeRef, Pos2>,
) -> Option<NodeRef> {
    let copy = duplicate_node(script, node)?;
    if let Some(place) = places.get(node) {
        script
            .graph
            .nodes
            .insert(copy.key(), [place.x + 30.0, place.y + 30.0]);
    }
    Some(copy)
}

/// What `draw_node` needs about one node.
struct NodeDraw<'a, 'b> {
    node: &'a NodeRef,
    body: &'a mut Body<'b>,
    geometry: &'a Geometry,
    rect: Rect,
    runs: bool,
    selected: bool,
    highlighted: bool,
    wired: &'a Wiring,
    wire_held: bool,
    objects: &'a [ObjectInfo],
}

fn draw_node(
    ui: &mut egui::Ui,
    drawer: &mut BlockPainter,
    node: NodeDraw,
    events: &mut Vec<Event>,
) {
    let z = drawer.z;
    let NodeDraw {
        node: which,
        mut body,
        geometry,
        rect,
        runs,
        selected,
        highlighted,
        wired,
        wire_held,
        objects,
    } = node;
    let id = egui::Id::new(("graph node", which.key())).with(drawer.id);
    let grows = match &mut body {
        Body::Block(kind) => kind.input_list_mut().is_some(),
        Body::Value(value) => value.input_list_mut().is_some(),
    };

    let is_clone = matches!(body, Body::Block(BlockKind::CloneInto { .. }));
    let clone_has_pos = matches!(
        body,
        Body::Block(BlockKind::CloneInto {
            position: Some(_),
            ..
        })
    );
    let clone_has_adv = matches!(
        body,
        Body::Block(BlockKind::CloneInto {
            offset_per_clone: Some(_),
            ..
        }) | Body::Block(BlockKind::CloneInto {
            per_clone_positions: Some(_),
            ..
        })
    );

    // The node itself: click to select, drag to move.
    if drawer.interactive {
        let response = ui
            .interact(rect, id, Sense::click_and_drag())
            .on_hover_cursor(CursorIcon::Grab);
        let response = if runs {
            response.on_hover_ui(|ui| {
                let info = match &body {
                    Body::Block(kind) => axcenity_blocks::catalogue::block_info(kind),
                    Body::Value(value) => axcenity_blocks::catalogue::expression_info(value),
                };
                crate::node_graph::node_card(ui, &info, "");
            })
        } else {
            response.on_hover_text("Not wired to an event, so it doesn't run.")
        };
        if response.clicked() {
            let add = ui.input(|i| i.modifiers.shift || i.modifiers.command);
            events.push(Event::Select(which.clone(), add));
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            events.push(Event::MoveStart(which.clone()));
        }
        if response.dragged_by(egui::PointerButton::Primary) {
            events.push(Event::Move(response.drag_delta()));
        }
        let optional_toggles: Vec<(&'static str, &'static str)> = match body {
            Body::Block(
                BlockKind::WhenTouched { cooldown, .. }
                | BlockKind::WhenPromptTriggered { cooldown, .. }
                | BlockKind::WhenPartClicked { cooldown, .. },
            ) => vec![(
                "cooldown",
                if cooldown.is_some() {
                    "Hide Cooldown (s)"
                } else {
                    "Show Cooldown (s)"
                },
            )],
            Body::Block(BlockKind::WhenCommandRun { admin_only, .. }) => vec![(
                "admin_only",
                if admin_only.is_some() {
                    "Hide Admin Only"
                } else {
                    "Show Admin Only"
                },
            )],
            Body::Block(BlockKind::FireEvent { data, .. }) => vec![(
                "data",
                if data.is_some() {
                    "Hide Payload / Data"
                } else {
                    "Show Payload / Data"
                },
            )],
            Body::Block(BlockKind::WaitForChild { timeout, .. }) => vec![(
                "timeout",
                if timeout.is_some() {
                    "Hide Timeout (s)"
                } else {
                    "Show Timeout (s)"
                },
            )],
            Body::Block(
                BlockKind::SlideTo {
                    wait_until_done, ..
                }
                | BlockKind::RotateTo {
                    wait_until_done, ..
                }
                | BlockKind::TweenProperty {
                    wait_until_done, ..
                }
                | BlockKind::FadeVolume {
                    wait_until_done, ..
                }
                | BlockKind::SlideIn {
                    wait_until_done, ..
                }
                | BlockKind::SlideOut {
                    wait_until_done, ..
                }
                | BlockKind::FadeIn {
                    wait_until_done, ..
                }
                | BlockKind::FadeOut {
                    wait_until_done, ..
                }
                | BlockKind::TweenPivot {
                    wait_until_done, ..
                }
                | BlockKind::PulseScale {
                    wait_until_done, ..
                },
            ) => vec![(
                "wait_until_done",
                if *wait_until_done {
                    "Don't Wait Until Done"
                } else {
                    "Wait Until Done"
                },
            )],
            Body::Block(BlockKind::Tween {
                repeat_count,
                reverses,
                wait_until_done,
                ..
            }) => vec![
                (
                    "repeat_count",
                    if repeat_count.is_some() {
                        "Hide Repeat Count"
                    } else {
                        "Show Repeat Count"
                    },
                ),
                (
                    "reverses",
                    if *reverses {
                        "Disable Reverses"
                    } else {
                        "Enable Reverses"
                    },
                ),
                (
                    "wait_until_done",
                    if *wait_until_done {
                        "Don't Wait Until Done"
                    } else {
                        "Wait Until Done"
                    },
                ),
            ],
            Body::Block(BlockKind::PlayAnimation {
                fade_time,
                speed,
                weight,
                wait_until_done,
                ..
            }) => vec![
                (
                    "fade_time",
                    if fade_time.is_some() {
                        "Hide Fade Time"
                    } else {
                        "Show Fade Time"
                    },
                ),
                (
                    "speed",
                    if speed.is_some() {
                        "Hide Speed"
                    } else {
                        "Show Speed"
                    },
                ),
                (
                    "weight",
                    if weight.is_some() {
                        "Hide Weight"
                    } else {
                        "Show Weight"
                    },
                ),
                (
                    "wait_until_done",
                    if *wait_until_done {
                        "Don't Wait Until Done"
                    } else {
                        "Wait Until Done"
                    },
                ),
            ],
            Body::Block(BlockKind::StopAnimation {
                animation,
                fade_time,
                ..
            }) => vec![
                (
                    "animation",
                    if animation.is_some() {
                        "Stop All Animations"
                    } else {
                        "Stop Specific Animation"
                    },
                ),
                (
                    "fade_time",
                    if fade_time.is_some() {
                        "Hide Fade Time"
                    } else {
                        "Show Fade Time"
                    },
                ),
            ],
            Body::Block(BlockKind::SetIkTarget { weight, .. }) => vec![(
                "weight",
                if weight.is_some() {
                    "Hide Weight"
                } else {
                    "Show Weight"
                },
            )],
            Body::Block(
                BlockKind::SetClockTime { duration, .. }
                | BlockKind::SetAtmosphere { duration, .. },
            ) => vec![(
                "duration",
                if duration.is_some() {
                    "Hide Duration (s)"
                } else {
                    "Show Duration (s)"
                },
            )],
            Body::Block(BlockKind::PlaySound {
                pitch,
                wait_until_done,
                ..
            }) => vec![
                (
                    "pitch",
                    if pitch.is_some() {
                        "Hide Pitch"
                    } else {
                        "Show Pitch"
                    },
                ),
                (
                    "wait_until_done",
                    if *wait_until_done {
                        "Don't Wait Until Done"
                    } else {
                        "Wait Until Done"
                    },
                ),
            ],
            Body::Block(BlockKind::PlaySoundAtPosition { volume, pitch, .. }) => vec![
                (
                    "volume",
                    if volume.is_some() {
                        "Hide Volume"
                    } else {
                        "Show Volume"
                    },
                ),
                (
                    "pitch",
                    if pitch.is_some() {
                        "Hide Pitch"
                    } else {
                        "Show Pitch"
                    },
                ),
            ],
            Body::Block(
                BlockKind::ForEachPartInRadius {
                    players_only,
                    ignore,
                    ..
                }
                | BlockKind::ForEachPartTouching {
                    players_only,
                    ignore,
                    ..
                },
            ) => vec![
                (
                    "players_only",
                    if *players_only {
                        "Filter: Any Part (currently Players Only)"
                    } else {
                        "Filter: Players Only"
                    },
                ),
                (
                    "ignore",
                    if ignore.is_some() {
                        "Hide Ignore Input"
                    } else {
                        "Show Ignore Input"
                    },
                ),
            ],
            _ => Vec::new(),
        };
        let raycast_has_ignore = match body {
            Body::Value(Expression::Raycast { ignore, .. }) => Some(ignore.is_some()),
            _ => None,
        };
        let sequence_branches = match body {
            Body::Block(BlockKind::Sequence { branches }) => Some(branches.len()),
            _ => None,
        };
        response.context_menu(|ui| {
            if is_clone && let NodeRef::Block(block_id) = which {
                let pos_label = if clone_has_pos {
                    "Hide Position Input"
                } else {
                    "Show Position Input"
                };
                if ui.button(pos_label).clicked() {
                    events.push(Event::ToggleClonePosition(*block_id));
                    ui.close();
                }
                let adv_label = if clone_has_adv {
                    "Hide Advanced Clone Settings"
                } else {
                    "Show Advanced Clone Settings"
                };
                if ui.button(adv_label).clicked() {
                    events.push(Event::ToggleCloneAdvanced(*block_id));
                    ui.close();
                }
                ui.separator();
            }
            if !optional_toggles.is_empty()
                && let NodeRef::Block(block_id) = which
            {
                for (key, label) in &optional_toggles {
                    if ui.button(*label).clicked() {
                        events.push(Event::ToggleOptionalPin(*block_id, key));
                        ui.close();
                    }
                }
                ui.separator();
            }
            if let Some(has_ignore) = raycast_has_ignore {
                let label = if has_ignore {
                    "Hide Ignore Input"
                } else {
                    "Show Ignore Input"
                };
                if ui.button(label).clicked() {
                    events.push(Event::ToggleRaycastIgnore(which.clone()));
                    ui.close();
                }
                ui.separator();
            }
            if let Some(branch_count) = sequence_branches
                && let NodeRef::Block(block_id) = which
            {
                if branch_count < 12 && ui.button("Add Sequence Branch").clicked() {
                    events.push(Event::ResizeSequence(*block_id, true));
                    ui.close();
                }
                if branch_count > 2 && ui.button("Remove Last Branch").clicked() {
                    events.push(Event::ResizeSequence(*block_id, false));
                    ui.close();
                }
                ui.separator();
            }
            if grows {
                if ui.button("Add Input").clicked() {
                    events.push(Event::Resize(which.clone(), true));
                    ui.close();
                }
                if ui.button("Remove Last Input").clicked() {
                    events.push(Event::Resize(which.clone(), false));
                    ui.close();
                }
            }
            if ui
                .button(theme::with_icon(icons::COPY, "Duplicate"))
                .clicked()
            {
                events.push(Event::Duplicate(which.clone()));
                ui.close();
            }
            if ui
                .button(theme::with_icon(icons::TRASH, "Delete"))
                .clicked()
            {
                events.push(Event::Delete(which.clone()));
                ui.close();
            }
        });
    }

    let painter = drawer.painter.clone();
    let colors = theme::canvas(ui.ctx());
    let corner = drawer.s(6.0);
    painter.rect_filled(
        rect.translate(vec2(0.0, drawer.s(3.0))),
        radius(corner),
        Color32::from_black_alpha(70),
    );
    painter.rect_filled(rect, radius(corner), colors.node_body);
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), drawer.s(HEADER)));
    let header_category = if geometry.is_event {
        Category::Events
    } else {
        geometry.category
    };
    let mut fill = header_category.fill(&theme::palette(ui.ctx()));
    if !runs {
        fill = fill.lerp_to_gamma(Color32::from_gray(90), 0.6);
    }
    let r = radius(corner);
    painter.rect_filled(
        header,
        CornerRadius {
            nw: r,
            ne: r,
            sw: 0,
            se: 0,
        },
        colors.header(fill),
    );
    painter.text(
        pos2(header.left() + drawer.s(10.0), header.center().y),
        egui::Align2::LEFT_CENTER,
        theme::with_icon(geometry.icon, &geometry.title),
        FontId::proportional(13.0 * z),
        colors.header_text,
    );
    let outline = if highlighted {
        Stroke::new(2.5 * z.max(0.8), colors.error)
    } else if selected {
        Stroke::new(2.0 * z.max(0.8), colors.selected)
    } else {
        Stroke::new(1.0_f32, colors.node_edge)
    };
    painter.rect_stroke(rect, radius(corner), outline, egui::StrokeKind::Outside);

    // Rows: inputs and menus on the left, outputs on the right.
    let mut parts = parts_of(body, objects);
    let mut y = rect.top() + drawer.s(HEADER + 2.0);
    for (row, height) in geometry.rows.iter().enumerate() {
        let middle = y + drawer.s(*height) * 0.5;
        if let Some(item) = geometry.layout.left.get(row) {
            let pin_at = pos2(rect.left() + drawer.s(PIN + 6.0), middle);
            let text_x = rect.left() + drawer.s(PIN_SPACE);
            match item {
                Item::ExecIn => {
                    if let NodeRef::Block(block) = which {
                        let pin = Pin::ExecIn(*block);
                        draw_exec_pin(&painter, pin_at, z, wired.fed.contains(block), &colors);
                        pin_response(ui, drawer, &pin, pin_at, wire_held, events);
                    }
                }
                Item::Input {
                    slot,
                    part,
                    kind,
                    name,
                } => {
                    let pin = Pin::Input(which.slots().child(*slot));
                    let mut connected = false;
                    if let Some(Part::Slot(value, _)) = parts.get(*part) {
                        connected = !value.is_literal();
                    }
                    draw_value_pin(&painter, pin_at, z, pin_color(*kind), connected, &colors);
                    pin_response(ui, drawer, &pin, pin_at, wire_held, events);
                    painter.text(
                        pos2(text_x, middle),
                        egui::Align2::LEFT_CENTER,
                        *name,
                        drawer.font(),
                        colors.node_text,
                    );
                    if let Some(Part::Slot(value, kind)) = parts.get_mut(*part)
                        && value.is_literal()
                    {
                        let size = drawer.literal_size(ui, value, *kind);
                        let left = text_x + drawer.text_width(ui, name) + drawer.s(NAME_GAP);
                        let input = Rect::from_min_size(pos2(left, middle - size.y * 0.5), size);
                        drawer.literal(ui, value, *kind, input, id.with(("slot", *slot)));
                    }
                }
                Item::Field { part, name } => {
                    let label_x = rect.left() + drawer.s(10.0);
                    painter.text(
                        pos2(label_x, middle),
                        egui::Align2::LEFT_CENTER,
                        *name,
                        drawer.font(),
                        colors.node_text,
                    );
                    if let Some(field) = parts.get_mut(*part) {
                        let size = drawer.part_size(ui, field);
                        let left = label_x + drawer.text_width(ui, name) + drawer.s(NAME_GAP);
                        let field_rect =
                            Rect::from_min_size(pos2(left, middle - size.y * 0.5), size);
                        drawer.draw_field(ui, field, field_rect, id.with(("part", *part)));
                    }
                }
                Item::Code { part } => {
                    if let Some(field) = parts.get_mut(*part) {
                        let size = drawer.part_size(ui, field);
                        let field_rect = Rect::from_min_size(
                            pos2(rect.left() + drawer.s(10.0), middle - size.y * 0.5),
                            size,
                        );
                        drawer.draw_field(ui, field, field_rect, id.with(("part", *part)));
                    }
                }
                Item::ExecOut(..) | Item::Output(_) | Item::BlockOutput { .. } => {}
            }
        }
        if let Some(item) = geometry.layout.right.get(row) {
            let pin_at = pos2(rect.right() - drawer.s(PIN + 6.0), middle);
            let text_right = rect.right() - drawer.s(PIN_SPACE);
            match item {
                Item::ExecOut(out, name) => {
                    if let NodeRef::Block(block) = which {
                        let pin = Pin::ExecOut(*block, *out);
                        draw_exec_pin(
                            &painter,
                            pin_at,
                            z,
                            wired.targets.contains_key(&(*block, *out)),
                            &colors,
                        );
                        pin_response(ui, drawer, &pin, pin_at, wire_held, events);
                        painter.text(
                            pos2(text_right, middle),
                            egui::Align2::RIGHT_CENTER,
                            *name,
                            drawer.font(),
                            colors.node_text,
                        );
                    }
                }
                Item::Output(kind) => {
                    if let NodeRef::Value(at) = which {
                        let pin = Pin::Output(at.clone());
                        draw_value_pin(
                            &painter,
                            pin_at,
                            z,
                            pin_color(*kind),
                            !at.path.is_empty(),
                            &colors,
                        );
                        pin_response(ui, drawer, &pin, pin_at, wire_held, events);
                    }
                }
                Item::BlockOutput { index, kind, name } => {
                    if let NodeRef::Block(block) = which {
                        let pin = Pin::BlockOutput(*block, *index);
                        draw_value_pin(
                            &painter,
                            pin_at,
                            z,
                            pin_color(*kind),
                            wired.block_outputs_fed.contains(&(*block, *index)),
                            &colors,
                        );
                        pin_response(ui, drawer, &pin, pin_at, wire_held, events);
                        painter.text(
                            pos2(text_right, middle),
                            egui::Align2::RIGHT_CENTER,
                            name.as_str(),
                            drawer.font(),
                            colors.node_text,
                        );
                    }
                }
                _ => {}
            }
        }
        y += drawer.s(*height);
    }
}

/// A pin's pointer handling (`canvas::pin_interact`), as events.
fn pin_response(
    ui: &mut egui::Ui,
    drawer: &BlockPainter,
    pin: &Pin,
    at: Pos2,
    wire_held: bool,
    events: &mut Vec<Event>,
) {
    if !drawer.interactive {
        return;
    }
    let id = egui::Id::new(("graph pin", pin.clone())).with(drawer.id);
    match graph_canvas::pin_interact(ui, &drawer.painter, id, at, drawer.s(1.0), wire_held) {
        PinAction::WireFrom => events.push(Event::WireFrom(pin.clone())),
        PinAction::Break => events.push(Event::BreakPin(pin.clone())),
        PinAction::None => {}
    }
}

#[cfg(test)]
mod tests;
