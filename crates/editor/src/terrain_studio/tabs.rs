//! The Terrain Studio's three tabs: Sculpt, Materials and Terrain. Each holds
//! one row of tools and the sections that go with them. The tab shown is
//! always the held tool's, however it was picked (a key, the ribbon's Brush
//! menu, universal search); with no tool held it's the last one shown,
//! remembered across restarts in egui's memory like the sections' folds.
//! Switching tabs with a tool held picks that tab's last-used tool.

use super::*;

/// One of the Terrain Studio's tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StudioTab {
    /// The sculpting brushes.
    #[default]
    Sculpt,
    /// Painting, layers, biomes and scatter rules.
    Materials,
    /// Whole-terrain tools, water and the terrain's data.
    Terrain,
}

/// Where the tab shown is remembered in egui's memory (saved with the window).
const TAB_MEMORY: &str = "terrain studio tab";

/// From this panel width (in points), the tabs show their icons too.
const TAB_ICONS_WIDTH: f32 = 300.0;

impl StudioTab {
    pub const ALL: [StudioTab; 3] = [StudioTab::Sculpt, StudioTab::Materials, StudioTab::Terrain];

    /// Its tools, in palette order.
    pub fn tools(self) -> &'static [TerrainTool] {
        match self {
            StudioTab::Sculpt => &TerrainTool::SCULPTING,
            StudioTab::Materials => &TerrainTool::MATERIALS,
            StudioTab::Terrain => &TerrainTool::WHOLE,
        }
    }

    /// The tab `tool` is on.
    pub fn of(tool: TerrainTool) -> StudioTab {
        Self::ALL
            .into_iter()
            .find(|tab| tab.tools().contains(&tool))
            .unwrap_or_default()
    }

    pub fn label(self) -> &'static str {
        match self {
            StudioTab::Sculpt => "Sculpt",
            StudioTab::Materials => "Materials",
            StudioTab::Terrain => "Terrain",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            StudioTab::Sculpt => icons::SHOVEL,
            StudioTab::Materials => icons::PAINT_ROLLER,
            StudioTab::Terrain => icons::MAP_TRIFOLD,
        }
    }

    fn tip(self) -> &'static str {
        match self {
            StudioTab::Sculpt => "Brushes that shape the ground",
            StudioTab::Materials => "Paint, layers, biomes and scatter rules",
            StudioTab::Terrain => "Generate and erode the landscape, its water and its data",
        }
    }

    /// What to do while none of its tools is picked.
    pub(super) fn pick_hint(self) -> &'static str {
        match self {
            StudioTab::Sculpt => "Pick a brush to start shaping the terrain.",
            StudioTab::Materials => "Pick a tool to paint the ground.",
            StudioTab::Terrain => "Pick a tool to generate or erode the whole landscape.",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

impl TerrainStudio {
    /// The tab shown: the held tool's, or the last one shown.
    pub fn tab(&self) -> StudioTab {
        self.tool.map_or(self.tab, StudioTab::of)
    }

    /// Picks `tool` (or puts the tool down), showing its tab. Every route
    /// that picks a tool goes through here, so its tab remembers it.
    pub fn pick(&mut self, tool: Option<TerrainTool>) {
        self.tab = self.tab();
        if let Some(tool) = tool {
            self.tab = StudioTab::of(tool);
            self.last_tools[self.tab.index()] = Some(tool);
        }
        self.tool = tool;
    }

    /// Shows `tab`. With a tool held, picks the tab's last-used tool (its
    /// first, if none yet); with none held, none is picked.
    pub fn switch_tab(&mut self, tab: StudioTab) {
        if tab == self.tab() {
            return;
        }
        self.tab = tab;
        if self.tool.is_some() {
            let tool = self.last_tools[tab.index()].unwrap_or(tab.tools()[0]);
            self.pick(Some(tool));
        }
    }

    /// Before drawing: remembers the held tool for its tab (it may have
    /// been set directly), restores the tab from the last run once, and
    /// shows Materials when a layer, biome or scatter rule is newly
    /// selected while no tool is held.
    pub(super) fn settle_tab(&mut self, ctx: &egui::Context, cx: &StudioContext) {
        let memory = egui::Id::new(TAB_MEMORY);
        if !self.tab_restored {
            self.tab_restored = true;
            if let Some(index) = ctx.data_mut(|data| data.get_persisted::<usize>(memory)) {
                self.tab = StudioTab::ALL.get(index).copied().unwrap_or_default();
            }
        }
        self.pick(self.tool);
        if cx.selected != self.last_selected {
            self.last_selected = cx.selected;
            let material = cx
                .selected
                .and_then(|id| cx.world.get(id))
                .is_some_and(|i| {
                    matches!(
                        i.class(),
                        ClassName::TerrainLayer | ClassName::Biome | ClassName::ScatterRule
                    )
                });
            if material && self.tool.is_none() {
                self.tab = StudioTab::Materials;
            }
        }
        let index = self.tab().index();
        ctx.data_mut(|data| data.insert_persisted(memory, index));
    }

    /// The tab strip: Sculpt, Materials, Terrain. Narrow, without icons.
    pub(super) fn tab_strip(&mut self, ui: &mut egui::Ui) {
        let wide = ui.available_width() >= TAB_ICONS_WIDTH;
        let options = StudioTab::ALL.map(|tab| (tab, wide.then_some(tab.icon()), tab.label()));
        let tips = StudioTab::ALL.map(StudioTab::tip);
        let mut tab = self.tab();
        if widgets::segmented_tips(ui, &mut tab, &options, &tips).0 {
            self.switch_tab(tab);
        }
    }
}
