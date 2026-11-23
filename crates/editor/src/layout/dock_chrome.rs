//! What the editor draws over `egui_dock`'s tab bars once a dock is drawn:
//! an accent line along the top of each panel's active tab, and a chevron
//! at the right of a bottom panel's tab bar (the Console's, say) that
//! collapses it to its tabs, giving the panel above its room, and opens it
//! again.
//!
//! `egui_dock` has collapse buttons of its own, but they'd show on every
//! panel (the Scene and the Explorer too), where collapsing leaves an empty
//! column; only a panel stacked under another gives its room back.

use eframe::egui;
use egui_dock::{DockState, NodeIndex, NodePath, SurfaceIndex};

use super::{Pane, TAB_BAR_HEIGHT};
use crate::theme::{self, icons};

/// How thick the line over an active tab is, in points.
const ACTIVE_LINE: f32 = 2.0;

/// Each tab's place, from `TabViewer::on_tab_button` as the dock draws them.
pub type TabRects = Vec<(Pane, egui::Rect)>;

/// Whether the panel at `path` can collapse: it's the lower half of a split
/// stacked top and bottom, and the upper half isn't collapsed, so something
/// always keeps the room.
pub fn can_collapse(dock: &DockState<Pane>, path: NodePath) -> bool {
    let Some(parent) = path.node.parent() else {
        return false;
    };
    path.node.is_right()
        && dock[path].is_leaf()
        && dock[NodePath::new(path.surface, parent)].is_vertical()
        && !dock[NodePath::new(path.surface, parent.left())].is_collapsed()
}

/// Collapses the panel at `path` to its tabs, or opens it again, and
/// updates the counts `egui_dock` lays its splits out by.
pub fn set_collapsed(dock: &mut DockState<Pane>, path: NodePath, collapsed: bool) {
    if !dock[path].is_leaf() || dock[path].is_collapsed() == collapsed {
        return;
    }
    dock[path].set_collapsed(collapsed);
    recount(dock, path.surface);
}

/// Opens the panel at `path` if it's collapsed, such as when one of its
/// tabs is asked for from a menu.
pub fn expand(dock: &mut DockState<Pane>, path: NodePath) {
    set_collapsed(dock, path, false);
}

/// Works out each split's collapsed state from its children, from the
/// bottom of the tree up, as `egui_dock` does when its own buttons are used:
/// side by side a split is as collapsed as its more collapsed half, stacked
/// it adds them up.
fn recount(dock: &mut DockState<Pane>, surface: SurfaceIndex) {
    let Some(tree) = dock
        .get_surface_mut(surface)
        .and_then(|surface| surface.node_tree_mut())
    else {
        return;
    };
    for index in (0..tree.len()).rev() {
        let node = NodeIndex(index);
        if !tree[node].is_parent() {
            continue;
        }
        let (left, right) = (&tree[node.left()], &tree[node.right()]);
        let counts = (left.collapsed_leaf_count(), right.collapsed_leaf_count());
        let both = left.is_collapsed() && right.is_collapsed();
        let count = if tree[node].is_horizontal() {
            counts.0.max(counts.1)
        } else {
            counts.0 + counts.1
        };
        tree[node].set_collapsed(both);
        tree[node].set_collapsed_leaf_count(count);
    }
}

/// Draws over the tab bars `dock` was just drawn with: the accent line over
/// each panel's active tab (full strength on the focused panel, softer on
/// the others), and the collapse chevron on each panel that can collapse.
/// Returns true when one was collapsed or opened.
pub fn decorate(ui: &mut egui::Ui, dock: &mut DockState<Pane>, tabs: &TabRects) -> bool {
    let palette = theme::palette(ui.ctx());
    let focused = dock.focused_leaf();
    let mut toggle = None;
    for (path, leaf) in dock.iter_leaves() {
        let Some(&active) = leaf.tabs.get(leaf.active.0) else {
            continue;
        };
        let bar = leaf.rect;
        let collapsible = can_collapse(dock, path) && bar.is_positive();
        let chevron_size = TAB_BAR_HEIGHT - 8.0;
        // Reserve a solid slot at the right of the tab bar for the collapse
        // chevron so scrolled tabs and close buttons cannot collide with it.
        let chevron_slot_left = if collapsible {
            bar.right() - chevron_size - 8.0
        } else {
            bar.right()
        };
        if bar.is_positive()
            && let Some((_, rect)) = tabs.iter().find(|(pane, _)| *pane == active)
        {
            // Clip the accent line to the leaf's tab strip bounds so a
            // horizontally scrolled tab never bleeds into the viewport or
            // adjacent panels.
            let left = rect.left().max(bar.left());
            let right = rect.right().min(chevron_slot_left);
            if right > left {
                let color = if focused == Some(path) {
                    palette.accent
                } else {
                    palette.accent.gamma_multiply(0.7)
                };
                let line = egui::Rect::from_min_max(
                    egui::pos2(left, rect.top()),
                    egui::pos2(right, rect.top() + ACTIVE_LINE),
                );
                ui.painter().rect_filled(line, 0, color);
            }
        }
        if !collapsible {
            continue;
        }
        let collapsed = leaf.collapsed;
        let slot = egui::Rect::from_min_max(
            egui::pos2(chevron_slot_left, bar.top()),
            egui::pos2(bar.right(), bar.top() + TAB_BAR_HEIGHT - 1.0),
        );
        ui.painter().rect_filled(slot, 0, palette.app_bg);
        ui.painter().vline(
            slot.left() + 0.5,
            slot.y_range(),
            egui::Stroke::new(1.0_f32, palette.border),
        );
        let button = egui::Rect::from_center_size(
            egui::pos2(
                bar.right() - 4.0 - chevron_size / 2.0,
                bar.top() + TAB_BAR_HEIGHT / 2.0,
            ),
            egui::vec2(chevron_size, chevron_size),
        );
        let (icon, tip) = if collapsed {
            (icons::CARET_UP, "Open this panel again")
        } else {
            (
                icons::CARET_DOWN,
                "Collapse this panel to its tabs, giving the panel above its room",
            )
        };
        let clicked = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(button)
                    .id_salt(("dock collapse", path.surface.0, path.node.0))
                    .layout(egui::Layout::centered_and_justified(
                        egui::Direction::LeftToRight,
                    )),
                |ui| crate::widgets::icon_button(ui, icon, tip, false).clicked(),
            )
            .inner;
        if clicked {
            toggle = Some((path, !collapsed));
        }
    }
    match toggle {
        Some((path, collapsed)) => {
            set_collapsed(dock, path, collapsed);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default layout's Console leaf, under the Scene.
    fn console(dock: &DockState<Pane>) -> NodePath {
        let found = dock.find_tab(&Pane::Console).expect("the Console is open");
        NodePath::new(found.surface, found.node)
    }

    #[test]
    fn the_console_collapses_to_its_tabs_and_opens_again() {
        let mut dock = super::super::default_dock();
        let path = console(&dock);
        assert!(can_collapse(&dock, path));
        set_collapsed(&mut dock, path, true);
        assert!(dock[path].is_collapsed());
        let parent = NodePath::new(path.surface, path.node.parent().unwrap());
        assert_eq!(dock[parent].collapsed_leaf_count(), 1);
        // The Scene above it keeps the room, so it can't collapse too.
        let scene = dock.find_tab(&Pane::Scene).unwrap();
        assert!(!can_collapse(
            &dock,
            NodePath::new(scene.surface, scene.node)
        ));
        expand(&mut dock, path);
        assert!(!dock[path].is_collapsed());
        assert_eq!(dock[parent].collapsed_leaf_count(), 0);
    }

    #[test]
    fn only_panels_stacked_under_another_collapse() {
        let dock = super::super::default_dock();
        let explorer = dock.find_tab(&Pane::Explorer).unwrap();
        let properties = dock.find_tab(&Pane::Properties).unwrap();
        assert!(!can_collapse(
            &dock,
            NodePath::new(explorer.surface, explorer.node)
        ));
        assert!(can_collapse(
            &dock,
            NodePath::new(properties.surface, properties.node)
        ));
    }

    #[test]
    fn opening_a_tab_of_a_collapsed_panel_opens_the_panel() {
        let mut layout = super::super::Layout::default();
        let path = console(&layout.dock);
        set_collapsed(&mut layout.dock, path, true);
        layout.open(Pane::Console);
        assert!(!layout.dock[path].is_collapsed());
    }
}
