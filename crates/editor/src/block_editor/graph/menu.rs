//! The node menu: every palette block and reporter by category, with a
//! search box. Opened from a wire, it offers only the nodes that wire fits,
//! and the new node is wired to it.

use super::*;
use crate::node_graph::canvas::{MenuEntry, SearchMenu};

/// The open node menu.
#[derive(Clone, Debug)]
pub(super) struct Menu {
    /// Where it opened, on screen and in the graph (where the node goes).
    pub(super) menu: SearchMenu,
    /// The pin a wire was dropped from, to wire the new node to.
    pub(super) from: Option<Pin>,
}

impl Menu {
    pub(super) fn new(screen: Pos2, graph: Pos2, from: Option<Pin>) -> Self {
        Self {
            menu: SearchMenu::new(screen, graph),
            from,
        }
    }
}

pub(super) struct MenuItem {
    pub(super) category: Category,
    pub(super) is_event: bool,
    pub(super) display_title: String,
    pub(super) item: PaletteItem,
    /// What the search box finds it by: its title, and the words and Lua
    /// the Blocks palette's search uses.
    words: String,
}

/// Whether a node made from `item` can be wired to `from`.
fn fits(script: &BlockScript, item: &PaletteItem, from: Option<&Pin>) -> bool {
    match (from, item) {
        (_, PaletteItem::NewVariable) => false,
        (None, _) => true,
        (Some(Pin::ExecOut(..)), PaletteItem::Block(kind)) => !kind.is_event(),
        (Some(Pin::ExecIn(_)), PaletteItem::Block(kind)) => !exec_outs(kind).is_empty(),
        (Some(Pin::Input(at)), PaletteItem::Reporter(expression)) => {
            slot_kind(script, at).is_some_and(|kind| kind.accepts(expression.shape()))
        }
        (Some(Pin::Output(at)), item) => {
            let Some(shape) = value(script, at).map(Expression::shape) else {
                return false;
            };
            let slots = match item {
                PaletteItem::Block(kind) => block_slots(kind),
                PaletteItem::Reporter(expression) => value_slots(expression),
                PaletteItem::NewVariable => Vec::new(),
            };
            slots.iter().any(|kind| kind.accepts(shape))
        }
        (Some(Pin::BlockOutput(..)), item) => {
            let slots = match item {
                PaletteItem::Block(kind) => block_slots(kind),
                PaletteItem::Reporter(expression) => value_slots(expression),
                PaletteItem::NewVariable => Vec::new(),
            };
            slots.iter().any(|kind| kind.accepts(BlockShape::Value))
        }
        _ => false,
    }
}

/// What the menu offers, in palette order.
pub(super) fn menu_items(
    script: &BlockScript,
    from: Option<&Pin>,
    context: &BlockContext,
    variables: &[String],
) -> Vec<MenuItem> {
    let mut items = Vec::new();
    for category in Category::ALL {
        for item in palette(category, context.client, context.objects, variables) {
            if !fits(script, &item, from) {
                continue;
            }
            let (title, icon, is_event) = match &item {
                PaletteItem::Block(kind) => (
                    block_names(kind).0.to_owned(),
                    block_icon(kind),
                    kind.is_event(),
                ),
                PaletteItem::Reporter(expression) => {
                    (value_names(expression).0, reporter_icon(expression), false)
                }
                PaletteItem::NewVariable => continue,
            };
            let display_title = theme::with_icon(icon, &title);
            let words = format!(
                "{} {}",
                title.to_lowercase(),
                search_text(&item, category, context.objects)
            );
            items.push(MenuItem {
                category,
                is_event,
                display_title,
                item,
                words,
            });
        }
    }
    items
}

/// Draws the menu (the shared `canvas::SearchMenu`). Escape or a click
/// outside closes it; Enter picks the first match.
pub(super) fn show_menu(
    ui: &mut egui::Ui,
    menu: &mut Menu,
    items: &[MenuItem],
    key: &str,
) -> MenuChoice {
    let palette = theme::palette(ui.ctx());
    let entries: Vec<MenuEntry> = items
        .iter()
        .map(|item| {
            let color_category = if item.is_event {
                Category::Events
            } else {
                item.category
            };
            MenuEntry {
                group: item.category.name(),
                color: color_category.fill(&palette),
                title: &item.display_title,
                words: &item.words,
            }
        })
        .collect();
    let title = if menu.from.is_some() {
        "Nodes for this wire"
    } else {
        "Add a node"
    };
    menu.menu.show(
        ui.ctx(),
        egui::Id::new(("graph menu", key)),
        title,
        &entries,
        Some(&|ui: &mut egui::Ui, index: usize| {
            let item = &items[index].item;
            lua_hint(ui, "Click to add it.", item_info(item), item_lua(item));
        }),
    )
}
