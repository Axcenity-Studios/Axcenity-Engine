//! The tabs above the ribbon, like Roblox Studio's Home, Model and Test:
//! each tab shows some of the ribbon's groups. Besides the built-in tabs,
//! people can make their own ("custom tabs") from any groups, in any
//! order, with any Phosphor icon. Custom tabs are saved between runs, like
//! the theme.
//!
//! This module only knows which groups a tab has; the app draws the groups
//! themselves (`EditorApp::ribbon_group`), in the ribbon and in the tab
//! editor's preview.

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::theme::{self, icons, type_scale};
use crate::widgets;

mod editor;
mod icon_picker;

use editor::TabEditor;
pub use icon_picker::{DEFAULT_ICON, icon_glyph};
#[cfg(test)]
pub use icon_picker::{SUGGESTED_ICONS, find_icons, icon_label};

/// One captioned group of ribbon buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RibbonGroup {
    /// Play and Stop.
    Game,
    /// How many clients Play runs and the network between them (Phases 64
    /// and 67), and which one the viewport shows.
    Clients,
    /// Select, Move, Scale, Rotate and Transform.
    Tools,
    /// Paint, Material, Anchor, Lighting, Emission and Angular.
    Build,
    /// Union, Negate, Intersect and Separate (Phase 50).
    Solid,
    /// The move and rotate steps, Surfaces, Collisions, Join Surfaces and
    /// Collision Groups.
    Snapping,
    /// Block, Ball, Cylinder and Wedge.
    Parts,
    /// Lights, probes, the sky and the looks that go in Lighting.
    Lights,
    /// Sounds, particle emitters, lens effects and Graphics Volumes.
    Effects,
    /// Characters, health, seats, prompts, spawns, tools, vehicles, agents,
    /// checkpoints and cameras.
    Gameplay,
    /// Weld parts together, and insert Attachments.
    Constraints,
    /// Import…, for models, images, sounds and prefabs.
    Import,
    /// ScreenGui, Frame, TextLabel and the other UI elements.
    Ui,
    /// Script, LocalScript and ModuleScript.
    Scripts,
    /// The Terrain Studio and its main tools.
    Terrain,
    /// Explorer and Properties, and a menu of every other panel.
    Panels,
    /// Scan the open project for missing and broken assets.
    Check,
    /// A menu of the viewport's overlays, the view cube and View Mode.
    Overlays,
    /// The toolbars the editor's Lua plugins made (Phase 54).
    Plugins,
}

impl RibbonGroup {
    /// The group's category colour, from the same hues as Block Coding's
    /// categories (cohesion pass CW2.2): play and checks green, transform
    /// tools blue, building warm, content cool, layout slate.
    pub fn hue(self) -> crate::theme::Hue {
        use crate::theme::Hue;
        match self {
            RibbonGroup::Game | RibbonGroup::Clients | RibbonGroup::Check => Hue::Green,
            RibbonGroup::Tools | RibbonGroup::Snapping => Hue::Blue,
            RibbonGroup::Build | RibbonGroup::Solid => Hue::Orange,
            RibbonGroup::Parts | RibbonGroup::Import => Hue::Indigo,
            RibbonGroup::Lights => Hue::Amber,
            RibbonGroup::Effects => Hue::Purple,
            RibbonGroup::Gameplay => Hue::Red,
            RibbonGroup::Constraints => Hue::Cyan,
            RibbonGroup::Ui => Hue::Magenta,
            RibbonGroup::Scripts => Hue::Violet,
            RibbonGroup::Terrain => Hue::Teal,
            RibbonGroup::Panels | RibbonGroup::Overlays | RibbonGroup::Plugins => Hue::Slate,
        }
    }

    pub const ALL: [RibbonGroup; 19] = [
        RibbonGroup::Game,
        RibbonGroup::Clients,
        RibbonGroup::Tools,
        RibbonGroup::Build,
        RibbonGroup::Solid,
        RibbonGroup::Snapping,
        RibbonGroup::Parts,
        RibbonGroup::Lights,
        RibbonGroup::Effects,
        RibbonGroup::Gameplay,
        RibbonGroup::Constraints,
        RibbonGroup::Import,
        RibbonGroup::Ui,
        RibbonGroup::Scripts,
        RibbonGroup::Terrain,
        RibbonGroup::Panels,
        RibbonGroup::Check,
        RibbonGroup::Overlays,
        RibbonGroup::Plugins,
    ];

    /// Whether its buttons act on what the Scene View shows (picking a
    /// tool, snapping, placing or shaping parts), so they wait while no
    /// Scene View is showing rather than act out of sight.
    pub fn needs_scene(self) -> bool {
        matches!(
            self,
            RibbonGroup::Tools
                | RibbonGroup::Build
                | RibbonGroup::Solid
                | RibbonGroup::Snapping
                | RibbonGroup::Parts
                | RibbonGroup::Constraints
        )
    }

    /// The caption under the group.
    pub fn label(self) -> &'static str {
        match self {
            RibbonGroup::Game => "Game",
            RibbonGroup::Clients => "Clients",
            RibbonGroup::Tools => "Tools",
            RibbonGroup::Build => "Build",
            RibbonGroup::Solid => "Solid Modeling",
            RibbonGroup::Snapping => "Snapping",
            RibbonGroup::Parts => "Parts",
            RibbonGroup::Lights => "Lights & Sky",
            RibbonGroup::Effects => "Effects",
            RibbonGroup::Gameplay => "Gameplay",
            RibbonGroup::Constraints => "Constraints",
            RibbonGroup::Import => "Models",
            RibbonGroup::Ui => "UI",
            RibbonGroup::Scripts => "Scripts",
            RibbonGroup::Terrain => "Terrain",
            RibbonGroup::Panels => "Panels",
            RibbonGroup::Check => "Check",
            RibbonGroup::Overlays => "View",
            RibbonGroup::Plugins => "Plugins",
        }
    }

    fn description(self) -> &'static str {
        match self {
            RibbonGroup::Game => "Play and Stop",
            RibbonGroup::Clients => "How many clients Play runs, and the network between them",
            RibbonGroup::Tools => "Select, Move, Scale, Rotate and Transform",
            RibbonGroup::Build => "Paint, materials, anchor, lights and the angular tool",
            RibbonGroup::Solid => "Union, negate, intersect and separate parts",
            RibbonGroup::Snapping => {
                "Move and rotate steps, snapping to surfaces, collisions and joining surfaces"
            }
            RibbonGroup::Parts => "Insert blocks, balls, cylinders and wedges",
            RibbonGroup::Lights => "Insert lights, probes, the sky, clouds, weather and outlines",
            RibbonGroup::Effects => {
                "Insert sounds, particle emitters, lens effects and Graphics Volumes"
            }
            RibbonGroup::Gameplay => {
                "Insert characters, health, seats, prompts, spawns, tools, vehicles and cameras"
            }
            RibbonGroup::Constraints => "Weld parts together and insert Attachments",
            RibbonGroup::Import => {
                "Import models, images, sounds and prefabs, or make and edit models in the Model Studio"
            }
            RibbonGroup::Ui => "Insert screens, frames, text, buttons and other UI",
            RibbonGroup::Scripts => "Insert scripts",
            RibbonGroup::Terrain => "Sculpt, paint and generate terrain",
            RibbonGroup::Panels => {
                "Show or hide the Explorer, Properties, Console and Assets panels"
            }
            RibbonGroup::Check => "Scan the project for missing and broken assets",
            RibbonGroup::Overlays => "Overlays, the view cube, light cones and view modes",
            RibbonGroup::Plugins => "Buttons the project's and your plugins add",
        }
    }
}

/// The tabs every editor has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltInTab {
    Home,
    Model,
    // Gameplay and Terrain were tabs of their own; a saved one opens World.
    #[serde(alias = "Gameplay", alias = "Terrain")]
    World,
    Ui,
    Script,
    // So was View; its overlays are on Test now.
    #[serde(alias = "View")]
    Test,
    Plugins,
}

impl BuiltInTab {
    pub const ALL: [BuiltInTab; 7] = [
        BuiltInTab::Home,
        BuiltInTab::Model,
        BuiltInTab::World,
        BuiltInTab::Ui,
        BuiltInTab::Script,
        BuiltInTab::Test,
        BuiltInTab::Plugins,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BuiltInTab::Home => "Home",
            BuiltInTab::Model => "Model",
            BuiltInTab::World => "World",
            BuiltInTab::Ui => "UI",
            BuiltInTab::Script => "Script",
            BuiltInTab::Test => "Test",
            BuiltInTab::Plugins => "Plugins",
        }
    }

    /// The icon in front of its name on the tab.
    pub fn icon(self) -> &'static str {
        match self {
            BuiltInTab::Home => icons::HOUSE,
            BuiltInTab::Model => icons::CUBE,
            BuiltInTab::World => icons::GLOBE_HEMISPHERE_WEST,
            BuiltInTab::Ui => icons::LAYOUT,
            BuiltInTab::Script => icons::CODE,
            BuiltInTab::Test => icons::FLASK,
            BuiltInTab::Plugins => icons::PUZZLE_PIECE,
        }
    }

    pub fn groups(self) -> &'static [RibbonGroup] {
        use RibbonGroup::*;
        match self {
            // Play, Run and Stop are pinned beside the tabs, on every tab
            // (the Game group is still there for custom tabs).
            BuiltInTab::Home => &[Tools, Snapping, Parts, Panels],
            // Building and joining parts; what lights and fills the world,
            // and what plays in it, are on World.
            BuiltInTab::Model => &[Build, Solid, Constraints, Import],
            // Each of these is a few menus, not a wall of buttons: the
            // Explorer's Insert menu has every class too.
            BuiltInTab::World => &[Lights, Effects, Gameplay, Terrain],
            BuiltInTab::Ui => &[Ui],
            BuiltInTab::Script => &[Scripts],
            BuiltInTab::Test => &[Clients, Check, Overlays],
            BuiltInTab::Plugins => &[Plugins],
        }
    }
}

/// A tab someone made: a name, an icon and the groups it shows, in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomTab {
    pub name: String,
    pub groups: Vec<RibbonGroup>,
    /// The name of its Phosphor icon (`"ROCKET"`), as `icons::ICONS` lists
    /// them: a name, not the glyph, so a font update can't change it.
    /// Tabs saved before tabs had icons get the star.
    #[serde(default = "default_icon")]
    pub icon: String,
}

impl CustomTab {
    /// The glyph in front of its name.
    pub fn glyph(&self) -> &'static str {
        icon_glyph(&self.icon)
    }
}

fn default_icon() -> String {
    DEFAULT_ICON.to_owned()
}

/// Which tab is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveTab {
    BuiltIn(BuiltInTab),
    /// An index into [`RibbonTabs::custom`].
    Custom(usize),
}

/// What's saved between runs.
#[derive(Serialize, Deserialize)]
struct Saved {
    active: ActiveTab,
    custom: Vec<CustomTab>,
}

/// The ribbon's tabs, the custom ones, and the window that makes and
/// changes custom tabs.
pub struct RibbonTabs {
    active: ActiveTab,
    pub custom: Vec<CustomTab>,
    editing: Option<TabEditor>,
}

impl Default for RibbonTabs {
    fn default() -> Self {
        Self {
            active: ActiveTab::BuiltIn(BuiltInTab::Home),
            custom: Vec::new(),
            editing: None,
        }
    }
}

impl RibbonTabs {
    pub fn set_active(&mut self, tab: ActiveTab) {
        if let ActiveTab::Custom(index) = tab
            && index >= self.custom.len()
        {
            return;
        }
        self.active = tab;
    }

    /// The groups the ribbon shows now, in order.
    pub fn groups(&self) -> Vec<RibbonGroup> {
        match self.active {
            ActiveTab::BuiltIn(tab) => tab.groups().to_vec(),
            ActiveTab::Custom(index) => self
                .custom
                .get(index)
                .map_or_else(Vec::new, |tab| tab.groups.clone()),
        }
    }

    /// Adds a custom tab with the icon called `icon` and shows it. Groups
    /// listed twice are kept once.
    pub fn add_custom(&mut self, name: &str, icon: &str, groups: &[RibbonGroup]) {
        self.custom.push(CustomTab {
            name: name.trim().to_owned(),
            groups: unique(groups),
            icon: icon.to_owned(),
        });
        self.active = ActiveTab::Custom(self.custom.len() - 1);
    }

    /// Removes a custom tab. If it was showing, Home shows instead.
    pub fn remove_custom(&mut self, index: usize) {
        if index >= self.custom.len() {
            return;
        }
        self.custom.remove(index);
        self.active = match self.active {
            ActiveTab::Custom(active) if active == index => ActiveTab::BuiltIn(BuiltInTab::Home),
            ActiveTab::Custom(active) if active > index => ActiveTab::Custom(active - 1),
            other => other,
        };
    }

    /// Draws the row of tabs, from the left or `centered`: the built-in
    /// ones, the custom ones (right-click to change or delete), and + to
    /// make a new one. The active tab is filled like the ribbon under it
    /// and underlined in the accent colour, and the line slides to a newly
    /// picked tab. Returns where the active tab is, so the ribbon can join it.
    pub fn tab_bar(&mut self, ui: &mut egui::Ui, centered: bool) -> Option<egui::Rect> {
        let mut active_rect = None;
        // A place in the paint order behind the tabs, filled in once the
        // active tab's place is known.
        let background = ui.painter().add(egui::Shape::Noop);
        // A row is only as wide as what's in it once it's drawn, so it's
        // centred by last frame's width, and drawn again if that changed.
        let width_id = ui.id().with("ribbon tab row width");
        let last_width: f32 = ui.ctx().data(|data| data.get_temp(width_id)).unwrap_or(0.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if centered {
                ui.add_space(((ui.available_width() - last_width) / 2.0).max(0.0));
            }
            let start = ui.cursor().left();
            for tab in BuiltInTab::ALL {
                let active = ActiveTab::BuiltIn(tab);
                let label = theme::with_icon(tab.icon(), tab.label());
                let response = tab_button(ui, &label, self.active == active);
                if self.active == active {
                    active_rect = Some(response.rect);
                }
                if response.clicked() {
                    self.active = active;
                }
            }
            if !self.custom.is_empty() {
                ui.add(egui::Separator::default().vertical().spacing(8.0));
            }
            let mut remove = None;
            for index in 0..self.custom.len() {
                let active = ActiveTab::Custom(index);
                let tab = &self.custom[index];
                let label = theme::with_icon(tab.glyph(), &tab.name);
                let response = tab_button(ui, &label, self.active == active)
                    .on_hover_text("Your tab. Right-click to change or delete it.");
                if self.active == active {
                    active_rect = Some(response.rect);
                }
                if response.clicked() {
                    self.active = active;
                }
                response.context_menu(|ui| {
                    if ui
                        .button(theme::menu_item(
                            ui.ctx(),
                            icons::PENCIL_SIMPLE,
                            "Edit Tab…",
                        ))
                        .clicked()
                    {
                        self.edit(Some(index));
                        ui.close();
                    }
                    if ui
                        .button(theme::menu_item(ui.ctx(), icons::SMILEY, "Change Icon…"))
                        .clicked()
                    {
                        self.edit(Some(index));
                        if let Some(editor) = &mut self.editing {
                            editor.picking_icon = true;
                        }
                        ui.close();
                    }
                    if ui
                        .button(theme::menu_item(ui.ctx(), icons::TRASH, "Delete Tab"))
                        .clicked()
                    {
                        remove = Some(index);
                        ui.close();
                    }
                });
            }
            if let Some(index) = remove {
                self.remove_custom(index);
            }
            ui.add_space(2.0);
            if widgets::icon_button(
                ui,
                icons::PLUS,
                "Make your own tab from any of the ribbon's groups.",
                false,
            )
            .clicked()
            {
                self.edit(None);
            }
            let width = ui.min_rect().right() - start;
            if (width - last_width).abs() > 0.5 {
                ui.ctx().data_mut(|data| data.insert_temp(width_id, width));
                ui.ctx().request_repaint();
            }
        });
        if let Some(rect) = active_rect {
            let palette = theme::palette(ui.ctx());
            let top_corners = egui::CornerRadius {
                nw: 6,
                ne: 6,
                sw: 0,
                se: 0,
            };
            ui.painter().set(
                background,
                egui::epaint::RectShape::new(
                    rect,
                    top_corners,
                    palette.panel,
                    egui::Stroke::new(1.0_f32, palette.border),
                    egui::StrokeKind::Inside,
                ),
            );
        }
        // The accent line under the active tab, sliding from where it was.
        if let Some(rect) = active_rect {
            let ctx = ui.ctx();
            let time = ctx.global_style().animation_time * 1.4;
            let id = egui::Id::new("ribbon tab underline");
            let left = ctx.animate_value_with_time(id.with("left"), rect.left() + 8.0, time);
            let right = ctx.animate_value_with_time(id.with("right"), rect.right() - 8.0, time);
            let palette = theme::palette(ctx);
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(left, rect.bottom() - 2.0),
                    egui::pos2(right, rect.bottom()),
                ),
                1,
                palette.accent,
            );
        }
        active_rect
    }

    /// The custom tabs and the tab showing, as text for eframe's storage.
    pub fn to_text(&self) -> String {
        serde_json::to_string(&Saved {
            active: self.active,
            custom: self.custom.clone(),
        })
        .unwrap_or_default()
    }

    /// Tabs saved by [`to_text`](Self::to_text). Damaged text gives the
    /// built-in tabs only.
    pub fn from_text(text: &str) -> Self {
        let Ok(saved) = serde_json::from_str::<Saved>(text) else {
            return Self::default();
        };
        let mut tabs = Self {
            custom: saved.custom,
            ..Self::default()
        };
        tabs.set_active(saved.active);
        tabs
    }
}

/// One ribbon tab: its name, brighter when hovered or active.
fn tab_button(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::new(type_scale::BODY, theme::medium_family()),
        egui::Color32::PLACEHOLDER,
    );
    let size = egui::vec2(galley.size().x + 16.0, 26.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, label));
    let hover = theme::ease(ui.ctx(), response.id.with("hover"), response.hovered());
    let color = if active {
        palette.text_strong
    } else {
        theme::mix(palette.text_weak, palette.text_strong, hover)
    };
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            5,
            egui::Stroke::new(1.0_f32, palette.accent),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().galley(
        rect.center() - galley.size() / 2.0 - egui::vec2(0.0, 1.0),
        galley,
        color,
    );
    response
}

/// `groups` with any repeats left out, in order.
fn unique(groups: &[RibbonGroup]) -> Vec<RibbonGroup> {
    let mut kept = Vec::new();
    for &group in groups {
        if !kept.contains(&group) {
            kept.push(group);
        }
    }
    kept
}

#[cfg(test)]
impl RibbonTabs {
    /// Shows `tab`, as clicking it does.
    pub fn set_active_for_tour(&mut self, tab: BuiltInTab) {
        self.active = ActiveTab::BuiltIn(tab);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_is_only_the_everyday_groups() {
        assert_eq!(
            BuiltInTab::Home.groups(),
            &[
                RibbonGroup::Tools,
                RibbonGroup::Snapping,
                RibbonGroup::Parts,
                RibbonGroup::Panels,
            ]
        );
    }

    #[test]
    fn tabs_saved_before_they_were_merged_open_the_tab_they_joined() {
        for (old, now) in [
            ("View", BuiltInTab::Test),
            ("Gameplay", BuiltInTab::World),
            ("Terrain", BuiltInTab::World),
        ] {
            let text = format!(
                r#"{{"active":{{"BuiltIn":"{old}"}},"custom":[{{"name":"Mine","groups":["Ui"]}}]}}"#
            );
            let tabs = RibbonTabs::from_text(&text);
            assert_eq!(tabs.active, ActiveTab::BuiltIn(now), "{old}");
            assert_eq!(tabs.custom.len(), 1, "custom tabs are kept");
        }
    }

    #[test]
    fn model_is_only_building_and_insertables_have_their_own_tabs() {
        assert_eq!(
            BuiltInTab::Model.groups(),
            &[
                RibbonGroup::Build,
                RibbonGroup::Solid,
                RibbonGroup::Constraints,
                RibbonGroup::Import,
            ]
        );
        assert_eq!(
            BuiltInTab::World.groups(),
            &[
                RibbonGroup::Lights,
                RibbonGroup::Effects,
                RibbonGroup::Gameplay,
                RibbonGroup::Terrain,
            ]
        );
        // The overlays joined Test when View went.
        assert!(BuiltInTab::Test.groups().contains(&RibbonGroup::Overlays));
    }

    #[test]
    fn clients_and_the_network_are_only_on_the_test_tab() {
        for tab in BuiltInTab::ALL {
            assert_eq!(
                tab.groups().contains(&RibbonGroup::Clients),
                tab == BuiltInTab::Test,
                "{tab:?}"
            );
        }
    }

    #[test]
    fn play_and_stop_are_pinned_beside_the_tabs_not_on_one() {
        for tab in BuiltInTab::ALL {
            assert!(!tab.groups().contains(&RibbonGroup::Game), "{tab:?}");
        }
    }

    #[test]
    fn every_group_is_on_some_built_in_tab() {
        // Game is pinned beside the tabs instead.
        for group in RibbonGroup::ALL
            .into_iter()
            .filter(|g| *g != RibbonGroup::Game)
        {
            assert!(
                BuiltInTab::ALL
                    .iter()
                    .any(|tab| tab.groups().contains(&group)),
                "{group:?}"
            );
        }
    }

    #[test]
    fn a_custom_tab_shows_its_groups_in_its_order() {
        let mut tabs = RibbonTabs::default();
        assert_eq!(tabs.groups(), BuiltInTab::Home.groups());
        tabs.add_custom(
            " Building ",
            DEFAULT_ICON,
            &[RibbonGroup::Parts, RibbonGroup::Tools, RibbonGroup::Parts],
        );
        assert_eq!(tabs.active, ActiveTab::Custom(0));
        assert_eq!(tabs.custom[0].name, "Building");
        assert_eq!(tabs.groups(), [RibbonGroup::Parts, RibbonGroup::Tools]);
    }

    #[test]
    fn deleting_the_showing_custom_tab_goes_back_home() {
        let mut tabs = RibbonTabs::default();
        tabs.add_custom("One", DEFAULT_ICON, &[RibbonGroup::Game]);
        tabs.add_custom("Two", DEFAULT_ICON, &[RibbonGroup::Ui]);
        tabs.remove_custom(0);
        assert_eq!(tabs.active, ActiveTab::Custom(0));
        assert_eq!(tabs.groups(), [RibbonGroup::Ui]);
        tabs.remove_custom(0);
        assert_eq!(tabs.active, ActiveTab::BuiltIn(BuiltInTab::Home));
    }

    #[test]
    fn custom_tabs_survive_saving_and_bad_text_is_ignored() {
        let mut tabs = RibbonTabs::default();
        tabs.add_custom(
            "Lights",
            DEFAULT_ICON,
            &[RibbonGroup::Effects, RibbonGroup::Game],
        );
        tabs.add_custom("Rockets", "ROCKET", &[RibbonGroup::Parts]);
        let reloaded = RibbonTabs::from_text(&tabs.to_text());
        assert_eq!(reloaded.custom, tabs.custom);
        assert_eq!(reloaded.custom[1].icon, "ROCKET");
        assert_eq!(reloaded.custom[1].glyph(), icons::ROCKET);
        assert_eq!(reloaded.active, ActiveTab::Custom(1));
        let broken = RibbonTabs::from_text("{");
        assert!(broken.custom.is_empty());
        assert_eq!(broken.active, ActiveTab::BuiltIn(BuiltInTab::Home));
        // Saved before tabs had icons: the star, as then.
        let old = RibbonTabs::from_text(
            r#"{"active":{"Custom":0},"custom":[{"name":"Mine","groups":["Ui"]}]}"#,
        );
        assert_eq!(old.custom[0].icon, DEFAULT_ICON);
        assert_eq!(old.custom[0].glyph(), icons::STAR);
        assert_eq!(old.active, ActiveTab::Custom(0));
        // An icon a later font doesn't have shows the star too.
        let unknown = RibbonTabs::from_text(
            r#"{"active":{"Custom":0},"custom":[{"name":"Mine","groups":["Ui"],"icon":"NO_SUCH_ICON"}]}"#,
        );
        assert_eq!(unknown.custom[0].glyph(), icons::STAR);
    }

    #[test]
    fn icons_are_found_by_name_and_read_as_words() {
        for name in SUGGESTED_ICONS {
            assert!(
                icons::ICONS.iter().any(|(icon, _)| icon == name),
                "{name} is a Phosphor icon"
            );
        }
        assert_eq!(icon_glyph("GAME_CONTROLLER"), icons::GAME_CONTROLLER);
        assert_eq!(icon_label("GAME_CONTROLLER"), "Game controller");
        assert_eq!(find_icons(""), SUGGESTED_ICONS);
        let found = find_icons("game contr");
        assert!(found.contains(&"GAME_CONTROLLER"), "{found:?}");
        assert!(found.iter().all(|name| name.contains("GAME")));
        assert!(find_icons("zzzz nothing").is_empty());
        // Every icon can be found, not only the suggestions.
        assert!(find_icons("acorn").contains(&"ACORN"));
    }
}
