//! The custom tab editor: the tab's icon and name, a live preview of the
//! tab drawn by the ribbon's own code, and its groups (add, take out,
//! reorder by dragging or with the arrows).

use eframe::egui;

use super::icon_picker::{icon_glyph, icon_label, icon_picker};
use super::{
    ActiveTab, BuiltInTab, CustomTab, RibbonGroup, RibbonTabs, default_icon, tab_button, unique,
};
use crate::theme::{self, icons, type_scale};
use crate::widgets;

/// A custom tab being made (`index` `None`) or changed.
pub(super) struct TabEditor {
    pub(super) index: Option<usize>,
    pub(super) name: String,
    /// Its icon's name ([`CustomTab::icon`]).
    pub(super) icon: String,
    pub(super) groups: Vec<RibbonGroup>,
    /// Whether the icon picker is open, and what's typed in its search.
    pub(super) picking_icon: bool,
    pub(super) icon_query: String,
}

impl TabEditor {
    /// Applies a change to the list of groups.
    fn reorder(&mut self, change: Reorder) {
        match change {
            Reorder::Remove(index) if index < self.groups.len() => {
                self.groups.remove(index);
            }
            Reorder::Move(from, to) => move_group(&mut self.groups, from, to),
            Reorder::Add(group) if !self.groups.contains(&group) => self.groups.push(group),
            _ => {}
        }
    }
}

impl RibbonTabs {
    /// Opens the window that makes a custom tab (`None`) or changes one.
    pub fn edit(&mut self, index: Option<usize>) {
        let (name, icon, groups) = match index.and_then(|index| self.custom.get(index)) {
            Some(tab) => (tab.name.clone(), tab.icon.clone(), tab.groups.clone()),
            None => (
                format!("My Tab {}", self.custom.len() + 1),
                default_icon(),
                Vec::new(),
            ),
        };
        self.editing = Some(TabEditor {
            index,
            name,
            icon,
            groups,
            picking_icon: false,
            icon_query: String::new(),
        });
    }

    /// Whether the custom tab window is open.
    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    /// The tab editor: the tab's icon and name, a preview of the tab as the
    /// ribbon will draw it, the groups in it (drag or use the arrows to
    /// reorder), and the groups to add. `preview` draws a row of groups the
    /// way the ribbon does (`EditorApp::ribbon_row`); a cover over it keeps
    /// its buttons from doing anything.
    pub fn editor_window(
        &mut self,
        ctx: &egui::Context,
        preview: &mut dyn FnMut(&mut egui::Ui, &[RibbonGroup]),
    ) {
        let Self {
            custom, editing, ..
        } = self;
        let Some(editor) = editing else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        let title = if editor.index.is_some() {
            "Edit Ribbon Tab"
        } else {
            "New Ribbon Tab"
        };
        // The tab row as it will look once this is saved.
        let mut row: Vec<(String, bool)> = BuiltInTab::ALL
            .iter()
            .map(|tab| (theme::with_icon(tab.icon(), tab.label()), false))
            .collect();
        let this = theme::with_icon(icon_glyph(&editor.icon), editor.name.trim());
        let mut placed = false;
        for (index, tab) in custom.iter().enumerate() {
            if editor.index == Some(index) {
                row.push((this.clone(), true));
                placed = true;
            } else {
                row.push((theme::with_icon(tab.glyph(), &tab.name), false));
            }
        }
        if !placed {
            row.push((this, true));
        }
        widgets::tool_window(icons::PENCIL_SIMPLE, title)
            .id("ribbon tab editor")
            .fit(EDITOR_WIDTH)
            .show(ctx, &mut open, |ui| {
                let palette = theme::palette(ui.ctx());
                ui.set_width(EDITOR_WIDTH - 16.0);
                name_row(ui, editor, &palette);
                if editor.picking_icon {
                    ui.add_space(theme::space::S);
                    icon_picker(ui, editor, &palette);
                }
                ui.add_space(theme::space::M);
                widgets::section_label(ui, "Preview");
                preview_ui(ui, &row, &editor.groups, preview, &palette);
                ui.add_space(theme::space::M);
                let mut change = None;
                // The lists give up room before the buttons go off the screen.
                let height = (ui.available_height() - 72.0).clamp(112.0, LIST_HEIGHT);
                ui.columns(2, |columns| {
                    widgets::section_label(&mut columns[0], "In this tab");
                    groups_list(
                        &mut columns[0],
                        &editor.groups,
                        &mut change,
                        height,
                        &palette,
                    );
                    widgets::section_label(&mut columns[1], "Add a group");
                    add_list(
                        &mut columns[1],
                        &editor.groups,
                        &mut change,
                        height,
                        &palette,
                    );
                });
                if let Some(change) = change {
                    editor.reorder(change);
                }
                ui.add_space(theme::space::M);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let ready = !editor.name.trim().is_empty() && !editor.groups.is_empty();
                    save = ui
                        .add_enabled(
                            ready,
                            widgets::button(widgets::Role::Primary, None, "Save", &palette),
                        )
                        .on_disabled_hover_text("Give the tab a name and at least one group.")
                        .clicked();
                    cancel = ui
                        .add(widgets::button(
                            widgets::Role::Secondary,
                            None,
                            "Cancel",
                            &palette,
                        ))
                        .clicked();
                });
            });
        if save {
            let editor = self.editing.take().expect("checked above");
            match editor.index.filter(|&index| index < self.custom.len()) {
                Some(index) => {
                    self.custom[index] = CustomTab {
                        name: editor.name.trim().to_owned(),
                        groups: unique(&editor.groups),
                        icon: editor.icon,
                    };
                    self.active = ActiveTab::Custom(index);
                }
                None => self.add_custom(&editor.name, &editor.icon, &editor.groups),
            }
        } else if cancel || !open {
            self.editing = None;
        }
    }
}

/// How wide the tab editor is: room for a preview of most tabs.
const EDITOR_WIDTH: f32 = 720.0;
/// How tall the lists of groups get before they scroll.
const LIST_HEIGHT: f32 = 230.0;

/// The tab's icon (a button that opens the picker) and its name.
fn name_row(ui: &mut egui::Ui, editor: &mut TabEditor, palette: &theme::Palette) {
    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                true,
                editor.picking_icon,
                "Tab icon",
            )
        });
        let hover = theme::ease(ui.ctx(), response.id.with("hover"), response.hovered());
        let (fill, stroke, ink) = if editor.picking_icon {
            (palette.accent_soft, palette.accent, palette.accent)
        } else {
            (
                theme::mix(palette.control, palette.control_hover, hover),
                palette.border,
                palette.text_strong,
            )
        };
        ui.painter().rect(
            rect,
            theme::radius::CARD,
            fill,
            egui::Stroke::new(1.0_f32, stroke),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center() - egui::vec2(0.0, 1.0),
            egui::Align2::CENTER_CENTER,
            icon_glyph(&editor.icon),
            egui::FontId::proportional(22.0),
            ink,
        );
        ui.painter().text(
            rect.right_bottom() - egui::vec2(5.0, 3.0),
            egui::Align2::RIGHT_BOTTOM,
            icons::CARET_DOWN,
            egui::FontId::proportional(9.0),
            palette.text_weak,
        );
        if response
            .on_hover_text("Pick the tab's icon")
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            editor.picking_icon = !editor.picking_icon;
            editor.icon_query.clear();
        }
        ui.add_space(theme::space::S);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = theme::space::XS;
            ui.label(widgets::micro("Name", palette.text_weak));
            ui.add(egui::TextEdit::singleline(&mut editor.name).desired_width(300.0));
        });
        ui.add_space(theme::space::S);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = theme::space::XS;
            ui.label(widgets::micro("Icon", palette.text_weak));
            ui.add_space(theme::space::XS);
            ui.label(egui::RichText::new(icon_label(&editor.icon)).color(palette.text_secondary));
        });
    });
}

/// The tab row and the ribbon under it as they'll look once the tab is
/// saved, drawn by the ribbon's own code (`preview`) and covered so none of
/// it can be clicked, hovered or reached with Tab.
fn preview_ui(
    ui: &mut egui::Ui,
    row: &[(String, bool)],
    groups: &[RibbonGroup],
    preview: &mut dyn FnMut(&mut egui::Ui, &[RibbonGroup]),
    palette: &theme::Palette,
) {
    let id = ui.id().with("ribbon tab preview");
    let outer = egui::Frame::new()
        .fill(palette.app_bg)
        .stroke(egui::Stroke::new(1.0_f32, palette.border))
        .inner_margin(egui::Margin {
            left: 1,
            right: 1,
            top: 6,
            bottom: 1,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            let active = preview_tab_row(ui, row, palette);
            let band = egui::Frame::new()
                .fill(palette.panel)
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let density = theme::density(ui.ctx());
                    let tallest: f32 = ui
                        .data(|data| data.get_temp(crate::ribbon::tallest_id(ui.ctx())))
                        .unwrap_or(0.0);
                    let height = tallest.max(crate::ribbon::button_height(density));
                    ui.set_min_height(height);
                    if groups.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space((height / 2.0 - 9.0).max(0.0));
                            ui.label(
                                egui::RichText::new("Add groups below to fill the tab.")
                                    .color(palette.text_weak),
                            );
                        });
                    } else {
                        crate::ribbon::separately(ui, id, |ui| preview(ui, groups));
                    }
                })
                .response
                .rect;
            let line = egui::Stroke::new(1.0_f32, palette.border);
            ui.painter().hline(band.x_range(), band.top() + 0.5, line);
            if let Some(tab) = active {
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(tab.left() + 1.0, band.top() - 0.5),
                        egui::pos2(tab.right() - 1.0, band.top() + 1.0),
                    ),
                    0,
                    palette.panel,
                );
            }
        })
        .response
        .rect;
    #[cfg(test)]
    ui.data_mut(|data| data.insert_temp(egui::Id::new("tab preview rect"), outer));
    let cover_id = id.with("cover");
    let cover = ui.interact(outer, cover_id, egui::Sense::click_and_drag());
    cover.on_hover_text("How the tab will look in the ribbon. Its buttons work once it's saved.");
    // Tab could still reach a button under the cover; send it on.
    if let Some(focused) = ui.memory(|memory| memory.focused())
        && focused != cover_id
        && ui
            .ctx()
            .read_response(focused)
            .is_some_and(|response| outer.contains_rect(response.rect))
    {
        ui.memory_mut(|memory| memory.surrender_focus(focused));
    }
}

/// The preview's row of tabs, centred like the real one (or, when it's
/// wider than the preview, slid so the tab being edited shows). Returns
/// where that tab is.
fn preview_tab_row(
    ui: &mut egui::Ui,
    row: &[(String, bool)],
    palette: &theme::Palette,
) -> Option<egui::Rect> {
    const GAP: f32 = 2.0;
    const DIVIDER: f32 = 8.0;
    let font = egui::FontId::new(type_scale::BODY, theme::medium_family());
    let widths: Vec<f32> = row
        .iter()
        .map(|(label, _)| {
            ui.painter()
                .layout_no_wrap(label.clone(), font.clone(), egui::Color32::PLACEHOLDER)
                .size()
                .x
                + 16.0
        })
        .collect();
    let divider_at = BuiltInTab::ALL.len();
    let mut lefts = Vec::with_capacity(widths.len());
    let mut x = 0.0;
    for (index, width) in widths.iter().enumerate() {
        if index == divider_at {
            x += DIVIDER + GAP;
        }
        lefts.push(x);
        x += width + GAP;
    }
    let total = (x - GAP).max(0.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::hover());
    let room = rect.width();
    let offset = if total <= room {
        (room - total) / 2.0
    } else {
        let active = row.iter().position(|(_, active)| *active).unwrap_or(0);
        let middle = lefts[active] + widths[active] / 2.0;
        (room / 2.0 - middle).clamp(room - total - 8.0, 8.0)
    };
    let background = ui.painter().add(egui::Shape::Noop);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_size(
                egui::pos2(rect.left() + offset, rect.top()),
                egui::vec2(total.max(room), 26.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child.spacing_mut().item_spacing.x = GAP;
    let mut active = None;
    for (index, (label, is_active)) in row.iter().enumerate() {
        if index == divider_at {
            let (divider, _) =
                child.allocate_exact_size(egui::vec2(DIVIDER, 26.0), egui::Sense::hover());
            child.painter().vline(
                divider.center().x,
                divider.top() + 6.0..=divider.bottom() - 6.0,
                egui::Stroke::new(1.0_f32, palette.border),
            );
        }
        let response = tab_button(&mut child, label, *is_active);
        if *is_active {
            active = Some(response.rect);
        }
    }
    if let Some(tab) = active {
        let top_corners = egui::CornerRadius {
            nw: 6,
            ne: 6,
            sw: 0,
            se: 0,
        };
        ui.painter().set(
            background,
            egui::epaint::RectShape::new(
                tab,
                top_corners,
                palette.panel,
                egui::Stroke::new(1.0_f32, palette.border),
                egui::StrokeKind::Inside,
            ),
        );
        child.painter().rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(tab.left() + 8.0, tab.bottom() - 2.0),
                egui::pos2(tab.right() - 8.0, tab.bottom()),
            ),
            1,
            palette.accent,
        );
    }
    active
}

/// A small dot of a group's category colour.
fn hue_dot(ui: &mut egui::Ui, group: RibbonGroup, palette: &theme::Palette) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 3.5, group.hue().ink(palette));
}

/// The groups in the tab, in order: drag one by its handle, or use the
/// arrows, to move it; × takes it out.
fn groups_list(
    ui: &mut egui::Ui,
    groups: &[RibbonGroup],
    change: &mut Option<Reorder>,
    height: f32,
    palette: &theme::Palette,
) {
    if groups.is_empty() {
        ui.label(
            egui::RichText::new("Nothing yet. Add groups from the list at the right.")
                .color(palette.text_weak),
        );
        return;
    }
    let count = groups.len();
    egui::ScrollArea::vertical()
        .id_salt("tab groups")
        .max_height(height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (index, &group) in groups.iter().enumerate() {
                let row = ui
                    .horizontal(|ui| {
                        ui.set_min_height(28.0);
                        let handle = ui.id().with(("tab group", index));
                        ui.dnd_drag_source(handle, index, |ui| {
                            ui.label(
                                egui::RichText::new(icons::DOTS_SIX_VERTICAL)
                                    .color(palette.text_weak),
                            )
                        })
                        .response
                        .on_hover_text("Drag to move it")
                        .on_hover_cursor(egui::CursorIcon::Grab);
                        hue_dot(ui, group, palette);
                        ui.label(group.label()).on_hover_text(group.description());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            if widgets::icon_button(ui, icons::X, "Take it out of this tab", false)
                                .clicked()
                            {
                                *change = Some(Reorder::Remove(index));
                            }
                            let later = ui
                                .add_enabled_ui(index + 1 < count, |ui| {
                                    widgets::icon_button(
                                        ui,
                                        icons::ARROW_DOWN,
                                        "Later: further right in the ribbon",
                                        false,
                                    )
                                })
                                .inner;
                            if later.clicked() {
                                *change = Some(Reorder::Move(index, index + 1));
                            }
                            let sooner = ui
                                .add_enabled_ui(index > 0, |ui| {
                                    widgets::icon_button(
                                        ui,
                                        icons::ARROW_UP,
                                        "Sooner: further left in the ribbon",
                                        false,
                                    )
                                })
                                .inner;
                            if sooner.clicked() {
                                *change = Some(Reorder::Move(index, index - 1));
                            }
                        });
                    })
                    .response;
                if let Some(from) = row.dnd_hover_payload::<usize>()
                    && *from != index
                {
                    // Where it'll land: above this row when coming from
                    // below, under it when coming from above.
                    let y = if *from > index {
                        row.rect.top()
                    } else {
                        row.rect.bottom()
                    };
                    ui.painter().hline(
                        row.rect.x_range(),
                        y,
                        egui::Stroke::new(2.0_f32, palette.accent),
                    );
                }
                if let Some(from) = row.dnd_release_payload::<usize>() {
                    *change = Some(Reorder::Move(*from, index));
                }
            }
        });
}

/// The groups not in the tab yet; clicking one adds it at the end.
fn add_list(
    ui: &mut egui::Ui,
    groups: &[RibbonGroup],
    change: &mut Option<Reorder>,
    height: f32,
    palette: &theme::Palette,
) {
    let left: Vec<RibbonGroup> = RibbonGroup::ALL
        .into_iter()
        .filter(|group| !groups.contains(group))
        .collect();
    if left.is_empty() {
        ui.label(egui::RichText::new("Every group is in this tab.").color(palette.text_weak));
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("groups to add")
        .max_height(height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for group in left {
                if add_row(ui, group, palette).clicked() {
                    *change = Some(Reorder::Add(group));
                }
            }
        });
}

/// One group to add: +, its colour, its name and what it has.
fn add_row(ui: &mut egui::Ui, group: RibbonGroup, palette: &theme::Palette) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Add {}", group.label()),
        )
    });
    let hover = theme::ease(ui.ctx(), response.id.with("hover"), response.hovered());
    if hover > 0.0 {
        ui.painter().rect_filled(
            rect,
            theme::radius::CONTROL,
            palette.control_hover.gamma_multiply(hover),
        );
    }
    let middle = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, middle),
        egui::Align2::CENTER_CENTER,
        icons::PLUS,
        egui::FontId::proportional(theme::size::ICON_SMALL),
        theme::mix(palette.text_weak, palette.accent, hover),
    );
    ui.painter().circle_filled(
        egui::pos2(rect.left() + 28.0, middle),
        3.5,
        group.hue().ink(palette),
    );
    let name = ui.painter().text(
        egui::pos2(rect.left() + 38.0, middle),
        egui::Align2::LEFT_CENTER,
        group.label(),
        egui::FontId::proportional(type_scale::BODY),
        palette.text_strong,
    );
    let room = rect.right() - name.right() - 16.0;
    if room > 40.0 {
        let about = widgets::elided(
            ui,
            group.description(),
            egui::FontId::proportional(type_scale::CAPTION),
            palette.text_weak,
            room,
            1,
        );
        let size = about.size();
        ui.painter().galley(
            egui::pos2(name.right() + 10.0, middle - size.y / 2.0),
            about,
            palette.text_weak,
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.on_hover_text(group.description())
}

/// A change to the groups in the tab being edited.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Reorder {
    Remove(usize),
    /// Takes the group at the first place and puts it at the second.
    Move(usize, usize),
    Add(RibbonGroup),
}

/// Takes the group at `from` out and puts it back at `to` (a place in the
/// list it ends up in), as dragging it there does.
fn move_group(groups: &mut Vec<RibbonGroup>, from: usize, to: usize) {
    if from >= groups.len() || from == to {
        return;
    }
    let group = groups.remove(from);
    groups.insert(to.min(groups.len()), group);
}

#[cfg(test)]
impl RibbonTabs {
    /// Opens the editor on a new tab called `name`, with `icon` and
    /// `groups`, and its icon picker open (searching `query`) or not.
    pub fn edit_for_test(
        &mut self,
        name: &str,
        icon: &str,
        groups: &[RibbonGroup],
        picking: Option<&str>,
    ) {
        self.edit(None);
        let editor = self.editing.as_mut().expect("just opened");
        editor.name = name.to_owned();
        editor.icon = icon.to_owned();
        editor.groups = groups.to_vec();
        editor.picking_icon = picking.is_some();
        editor.icon_query = picking.unwrap_or_default().to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::super::DEFAULT_ICON;
    use super::*;

    fn editor(groups: &[RibbonGroup]) -> TabEditor {
        TabEditor {
            index: None,
            name: "Mine".to_owned(),
            icon: DEFAULT_ICON.to_owned(),
            groups: groups.to_vec(),
            picking_icon: false,
            icon_query: String::new(),
        }
    }

    #[test]
    fn groups_move_where_they_are_dropped_and_come_and_go() {
        use RibbonGroup::*;
        let mut tab = editor(&[Game, Tools, Parts, Lights]);
        // Dragged down onto the third row.
        tab.reorder(Reorder::Move(0, 2));
        assert_eq!(tab.groups, [Tools, Parts, Game, Lights]);
        // Dragged up onto the first.
        tab.reorder(Reorder::Move(3, 0));
        assert_eq!(tab.groups, [Lights, Tools, Parts, Game]);
        // The arrows are moves by one.
        tab.reorder(Reorder::Move(1, 2));
        assert_eq!(tab.groups, [Lights, Parts, Tools, Game]);
        // Out of range does nothing.
        tab.reorder(Reorder::Move(9, 0));
        tab.reorder(Reorder::Remove(9));
        assert_eq!(tab.groups, [Lights, Parts, Tools, Game]);
        tab.reorder(Reorder::Remove(0));
        tab.reorder(Reorder::Add(Ui));
        tab.reorder(Reorder::Add(Ui));
        assert_eq!(tab.groups, [Parts, Tools, Game, Ui]);
    }

    /// Draws the editor once, with `preview` noting the groups it's asked
    /// to draw.
    fn draw(tabs: &mut RibbonTabs, ctx: &egui::Context, drawn: &mut Vec<Vec<RibbonGroup>>) {
        let _ = theme::run(ctx, Default::default(), |ui| {
            let ctx = ui.ctx().clone();
            egui::CentralPanel::default().show(ui, |_| {});
            tabs.editor_window(&ctx, &mut |ui, groups| {
                drawn.push(groups.to_vec());
                ui.label("groups");
            });
        });
    }

    #[test]
    fn the_preview_follows_the_edits_and_saving_keeps_the_icon() {
        let ctx = theme::test_context();
        let mut tabs = RibbonTabs::default();
        tabs.edit(None);
        let mut drawn = Vec::new();
        draw(&mut tabs, &ctx, &mut drawn);
        assert!(drawn.is_empty(), "nothing to preview without groups");
        let editor = tabs.editing.as_mut().unwrap();
        editor.reorder(Reorder::Add(RibbonGroup::Parts));
        editor.reorder(Reorder::Add(RibbonGroup::Tools));
        draw(&mut tabs, &ctx, &mut drawn);
        assert_eq!(
            drawn.last(),
            Some(&vec![RibbonGroup::Parts, RibbonGroup::Tools])
        );
        let editor = tabs.editing.as_mut().unwrap();
        editor.reorder(Reorder::Move(1, 0));
        // The icon picker draws too.
        editor.picking_icon = true;
        editor.icon_query = "rocket".to_owned();
        draw(&mut tabs, &ctx, &mut drawn);
        assert_eq!(
            drawn.last(),
            Some(&vec![RibbonGroup::Tools, RibbonGroup::Parts])
        );
        let editor = tabs.editing.take().unwrap();
        assert_eq!(editor.groups, [RibbonGroup::Tools, RibbonGroup::Parts]);
        tabs.add_custom(&editor.name, "ROCKET", &editor.groups);
        assert_eq!(tabs.custom[0].glyph(), icons::ROCKET);
        // Editing it again starts from its icon.
        tabs.edit(Some(0));
        assert_eq!(tabs.editing.as_ref().unwrap().icon, "ROCKET");
    }
}
