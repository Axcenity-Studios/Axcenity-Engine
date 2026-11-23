//! Environment Weather Inspector & Manual Interior/Exterior Surface Mask Paint
//! Tool (Section 4).
//!
//! Shown inside the Properties panel whenever `Lighting`, `Weather`, or the
//! empty Place Settings view is open:
//! - Quick enable/disable toggle for the world's `Weather`
//! - One-click precipitation swap (`None`, `Rain`, `Snow`) and intensity/snow
//!   thickness sliders
//! - Built-in weather presets (`Clear`, `Drizzle`, `Rain`, `Storm`, `Light Snow`,
//!   `Heavy Snow`, `Blizzard`, `Wet After Rain`) plus custom saved profiles
//!   stored in `.axcenity/weather_profiles.json`
//! - Manual Interior / Exterior surface mask paint brush (`Interior`,
//!   `Exterior`, `Erase`) with viewport overlay & raycast painting.

use std::path::{Path, PathBuf};

use axcenity_core::{ClassName, EnumItem, InstanceId, Value, Vector3, World};
use axcenity_render::DEFAULT_SNOW_THICKNESS;
use axcenity_render::glam::{Vec2, Vec3};
use axcenity_scene::{
    CameraView, SurfaceMaskKind, format_surface_mask, paint_surface_mask, parse_surface_mask,
    pick_hit,
};
use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::theme::{self, icons};

/// A named weather configuration that can be applied in one click or saved as
/// a custom project profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeatherProfile {
    pub name: String,
    pub enabled: bool,
    /// `"None"`, `"Rain"`, or `"Snow"`.
    pub precipitation: String,
    pub intensity: f64,
    pub wetness: f64,
    pub snow_cover: f64,
    pub snow_thickness: f64,
    pub transition_time: f64,
    pub wind: [f32; 3],
}

/// Built-in one-click weather presets.
pub fn built_in_profiles() -> Vec<WeatherProfile> {
    vec![
        WeatherProfile {
            name: "Clear".to_owned(),
            enabled: false,
            precipitation: "None".to_owned(),
            intensity: 0.0,
            wetness: 0.0,
            snow_cover: 0.0,
            snow_thickness: DEFAULT_SNOW_THICKNESS as f64,
            transition_time: 6.0,
            wind: [0.0, 0.0, 0.0],
        },
        WeatherProfile {
            name: "Drizzle".to_owned(),
            enabled: true,
            precipitation: "Rain".to_owned(),
            intensity: 0.25,
            wetness: 0.35,
            snow_cover: 0.0,
            snow_thickness: DEFAULT_SNOW_THICKNESS as f64,
            transition_time: 8.0,
            wind: [3.0, 0.0, 1.0],
        },
        WeatherProfile {
            name: "Rain".to_owned(),
            enabled: true,
            precipitation: "Rain".to_owned(),
            intensity: 0.6,
            wetness: 0.7,
            snow_cover: 0.0,
            snow_thickness: DEFAULT_SNOW_THICKNESS as f64,
            transition_time: 8.0,
            wind: [8.0, 0.0, 3.0],
        },
        WeatherProfile {
            name: "Storm".to_owned(),
            enabled: true,
            precipitation: "Rain".to_owned(),
            intensity: 1.0,
            wetness: 1.0,
            snow_cover: 0.0,
            snow_thickness: DEFAULT_SNOW_THICKNESS as f64,
            transition_time: 5.0,
            wind: [24.0, 0.0, 10.0],
        },
        WeatherProfile {
            name: "Light Snow".to_owned(),
            enabled: true,
            precipitation: "Snow".to_owned(),
            intensity: 0.3,
            wetness: 0.0,
            snow_cover: 0.45,
            snow_thickness: 0.35,
            transition_time: 10.0,
            wind: [3.0, 0.0, 1.0],
        },
        WeatherProfile {
            name: "Heavy Snow".to_owned(),
            enabled: true,
            precipitation: "Snow".to_owned(),
            intensity: 0.8,
            wetness: 0.0,
            snow_cover: 0.9,
            snow_thickness: 0.75,
            transition_time: 10.0,
            wind: [10.0, 0.0, 4.0],
        },
        WeatherProfile {
            name: "Blizzard".to_owned(),
            enabled: true,
            precipitation: "Snow".to_owned(),
            intensity: 1.0,
            wetness: 0.0,
            snow_cover: 1.0,
            snow_thickness: 1.2,
            transition_time: 6.0,
            wind: [28.0, 0.0, 12.0],
        },
        WeatherProfile {
            name: "Wet After Rain".to_owned(),
            enabled: true,
            precipitation: "None".to_owned(),
            intensity: 0.0,
            wetness: 0.8,
            snow_cover: 0.0,
            snow_thickness: DEFAULT_SNOW_THICKNESS as f64,
            transition_time: 8.0,
            wind: [2.0, 0.0, 0.0],
        },
    ]
}

/// Active mode of the manual Interior / Exterior surface mask paint brush.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MaskBrushMode {
    #[default]
    Off,
    Interior,
    Exterior,
    Erase,
}

impl MaskBrushMode {
    pub fn is_active(self) -> bool {
        self != Self::Off
    }

    pub fn mask_kind(self) -> Option<SurfaceMaskKind> {
        match self {
            Self::Interior => Some(SurfaceMaskKind::Interior),
            Self::Exterior => Some(SurfaceMaskKind::Exterior),
            Self::Off | Self::Erase => None,
        }
    }
}

/// State of the manual Interior/Exterior paint tool.
#[derive(Clone, Debug)]
pub struct MaskBrushState {
    pub mode: MaskBrushMode,
    pub radius: f32,
    pub show_overlay: bool,
}

impl Default for MaskBrushState {
    fn default() -> Self {
        Self {
            mode: MaskBrushMode::Off,
            radius: 3.0,
            show_overlay: true,
        }
    }
}

impl MaskBrushState {
    pub fn is_active(&self) -> bool {
        self.mode.is_active()
    }
}

/// Editor-side UI state for the Environment Weather Inspector and custom
/// profiles.
#[derive(Clone, Debug, Default)]
pub struct WeatherInspectorState {
    pub brush: MaskBrushState,
    pub custom_profiles: Vec<WeatherProfile>,
    pub new_profile_name: String,
    loaded_project: Option<PathBuf>,
}

impl WeatherInspectorState {
    /// Lazily loads `.axcenity/weather_profiles.json` when the project changes.
    pub fn sync_project(&mut self, project_root: Option<&Path>) {
        let target = project_root.map(Path::to_path_buf);
        if self.loaded_project == target {
            return;
        }
        self.loaded_project = target;
        self.custom_profiles = project_root.map(load_custom_profiles).unwrap_or_default();
    }

    pub fn save_custom_profile(&mut self, project_root: Option<&Path>, profile: WeatherProfile) {
        if let Some(existing) = self
            .custom_profiles
            .iter_mut()
            .find(|p| p.name.eq_ignore_ascii_case(&profile.name))
        {
            *existing = profile;
        } else {
            self.custom_profiles.push(profile);
        }
        if let Some(root) = project_root {
            let _ = persist_custom_profiles(root, &self.custom_profiles);
        }
    }

    pub fn delete_custom_profile(&mut self, project_root: Option<&Path>, index: usize) {
        if index < self.custom_profiles.len() {
            self.custom_profiles.remove(index);
            if let Some(root) = project_root {
                let _ = persist_custom_profiles(root, &self.custom_profiles);
            }
        }
    }
}

fn profiles_path(project_root: &Path) -> PathBuf {
    project_root.join(".axcenity").join("weather_profiles.json")
}

fn load_custom_profiles(project_root: &Path) -> Vec<WeatherProfile> {
    let path = profiles_path(project_root);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn persist_custom_profiles(
    project_root: &Path,
    profiles: &[WeatherProfile],
) -> std::io::Result<()> {
    let path = profiles_path(project_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(profiles).unwrap_or_else(|_| "[]".to_owned());
    std::fs::write(path, json)
}

/// An edit requested by the Weather Inspector UI.
#[derive(Clone, Debug)]
pub enum WeatherInspectorAction {
    ToggleEnabled(bool),
    ApplyProfile(WeatherProfile),
    SetPrecipitation(&'static str),
    SetBool(&'static str, bool),
    SetNumber(&'static str, f64),
    ClearSurfaceMask,
    SelectWeather,
}

/// Finds Lighting's first `Weather` instance, if any.
pub fn find_weather(world: &World) -> Option<InstanceId> {
    let lighting = world.get(world.lighting())?;
    lighting.children().iter().copied().find(|&id| {
        world
            .get(id)
            .is_some_and(|c| c.class() == ClassName::Weather)
    })
}

/// Captures the world's current weather and wind settings as a `WeatherProfile`.
pub fn current_profile(world: &World, name: String) -> WeatherProfile {
    let weather = find_weather(world).and_then(|id| world.get(id));
    let enabled = weather
        .and_then(|w| w.property("Enabled").and_then(Value::as_bool))
        .unwrap_or(false);
    let precipitation = weather
        .and_then(|w| match w.property("Precipitation") {
            Some(Value::EnumItem(item)) => Some(item.item.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "None".to_owned());
    let num = |prop: &str, default: f64| {
        weather
            .and_then(|w| w.property(prop).and_then(Value::as_number))
            .unwrap_or(default)
    };
    let wind = world
        .get(world.workspace())
        .and_then(|ws| match ws.property("GlobalWind") {
            Some(Value::Vector3(v)) => Some([v.x, v.y, v.z]),
            _ => None,
        })
        .unwrap_or([0.0; 3]);
    WeatherProfile {
        name,
        enabled,
        precipitation,
        intensity: num("Intensity", 0.5),
        wetness: num("Wetness", 0.0),
        snow_cover: num("SnowCover", 0.0),
        snow_thickness: num("SnowThickness", DEFAULT_SNOW_THICKNESS as f64),
        transition_time: num("TransitionTime", 10.0),
        wind,
    }
}

/// Draws the Environment Weather quick controls, presets, custom profiles, and
/// manual Interior/Exterior surface mask paint tools.
pub fn show(
    ui: &mut egui::Ui,
    world: &World,
    state: &mut WeatherInspectorState,
    project_root: Option<&Path>,
    show_select_button: bool,
) -> Option<WeatherInspectorAction> {
    state.sync_project(project_root);
    let palette = theme::palette(ui.ctx());
    let mut action = None;

    let weather_id = find_weather(world);
    let weather = weather_id.and_then(|id| world.get(id));
    let enabled = weather
        .and_then(|w| w.property("Enabled").and_then(Value::as_bool))
        .unwrap_or(false);
    let precip = weather
        .and_then(|w| match w.property("Precipitation") {
            Some(Value::EnumItem(item)) => Some(item.item.as_str()),
            _ => None,
        })
        .unwrap_or("None");
    let intensity = weather
        .and_then(|w| w.property("Intensity").and_then(Value::as_number))
        .unwrap_or(0.5);
    let snow_thickness = weather
        .and_then(|w| w.property("SnowThickness").and_then(Value::as_number))
        .unwrap_or(DEFAULT_SNOW_THICKNESS as f64);
    let occlusion = weather
        .and_then(|w| w.property("OcclusionEnabled").and_then(Value::as_bool))
        .unwrap_or(true);
    let footprints = weather
        .and_then(|w| w.property("FootprintsEnabled").and_then(Value::as_bool))
        .unwrap_or(true);
    let mask_count = weather
        .and_then(|w| match w.property("SurfaceMask") {
            Some(Value::String(s)) => Some(parse_surface_mask(s).len()),
            _ => None,
        })
        .unwrap_or(0);

    ui.add_space(theme::space::M);
    egui::CollapsingHeader::new(
        egui::RichText::new(theme::with_icon(icons::CLOUD_RAIN, "Weather & Surface Detection"))
            .family(theme::heading_family())
            .color(palette.text_strong),
    )
    .id_salt("environment_weather_inspector")
    .default_open(true)
    .show(ui, |ui| {
        // Enable toggle + quick jump to Weather instance.
        ui.horizontal(|ui| {
            let mut on = enabled;
            if ui
                .checkbox(&mut on, "Weather Enabled")
                .on_hover_text("Quickly enable or disable precipitation and weathered surfaces")
                .changed()
            {
                action = Some(WeatherInspectorAction::ToggleEnabled(on));
            }
            if show_select_button && weather_id.is_some() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("Inspect")
                        .on_hover_text("Select Lighting/Weather to inspect all properties")
                        .clicked()
                    {
                        action = Some(WeatherInspectorAction::SelectWeather);
                    }
                });
            }
        });

        // Quick precipitation type swap.
        ui.add_space(theme::space::XS);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Particles:").color(palette.text_weak));
            for (label, value) in [("None", "None"), ("Rain", "Rain"), ("Snow", "Snow")] {
                let active = enabled && precip == value;
                if ui.selectable_label(active, label).clicked() {
                    action = Some(WeatherInspectorAction::SetPrecipitation(value));
                }
            }
        });

        // Quick Intensity and Snow Thickness sliders.
        if weather.is_some() {
            let mut int_val = intensity;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Intensity").color(palette.text_weak));
                if ui
                    .add(
                        egui::Slider::new(&mut int_val, 0.0..=1.0)
                            .clamping(egui::SliderClamping::Always),
                    )
                    .changed()
                {
                    action = Some(WeatherInspectorAction::SetNumber("Intensity", int_val));
                }
            });
            let mut thick_val = snow_thickness;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Snow Depth").color(palette.text_weak));
                if ui
                    .add(
                        egui::Slider::new(&mut thick_val, 0.0..=2.0)
                            .clamping(egui::SliderClamping::Always)
                            .suffix(" st"),
                    )
                    .changed()
                {
                    action = Some(WeatherInspectorAction::SetNumber("SnowThickness", thick_val));
                }
            });
        }

        // Quick presets grid.
        ui.add_space(theme::space::XS);
        ui.label(
            egui::RichText::new("Presets")
                .size(theme::type_scale::SMALL)
                .color(palette.text_weak),
        );
        ui.horizontal_wrapped(|ui| {
            for preset in built_in_profiles() {
                if ui
                    .small_button(&preset.name)
                    .on_hover_text(format!(
                        "Apply {} ({} {:.0}%)",
                        preset.name,
                        preset.precipitation,
                        preset.intensity * 100.0
                    ))
                    .clicked()
                {
                    action = Some(WeatherInspectorAction::ApplyProfile(preset));
                }
            }
        });

        // Surface occlusion & footprints toggles + manual paint brush.
        ui.add_space(theme::space::S);
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            let mut occ = occlusion;
            if ui
                .checkbox(&mut occ, "Roof Occlusion")
                .on_hover_text(
                    "Auto-detect exposed exterior surfaces via sky raycasts and stop rain/snow under roofs",
                )
                .changed()
            {
                action = Some(WeatherInspectorAction::SetBool("OcclusionEnabled", occ));
            }
            let mut fp = footprints;
            if ui
                .checkbox(&mut fp, "Snow Footprints")
                .on_hover_text("Stamp dynamic trails and footprints into snow as characters move")
                .changed()
            {
                action = Some(WeatherInspectorAction::SetBool("FootprintsEnabled", fp));
            }
        });

        ui.add_space(theme::space::XS);
        ui.label(
            egui::RichText::new("Surface Mask Paint Tool")
                .size(theme::type_scale::SMALL)
                .color(palette.text_weak),
        );
        ui.horizontal_wrapped(|ui| {
            for (mode, label, tip) in [
                (
                    MaskBrushMode::Interior,
                    "Paint Interior",
                    "Click/drag in Scene View to force sheltered dry/snow-free interior cells",
                ),
                (
                    MaskBrushMode::Exterior,
                    "Paint Exterior",
                    "Click/drag in Scene View to force exposed exterior cells",
                ),
                (
                    MaskBrushMode::Erase,
                    "Erase (Auto)",
                    "Restore painted cells back to automatic sky-visibility detection",
                ),
            ] {
                let selected = state.brush.mode == mode;
                if ui.selectable_label(selected, label).on_hover_text(tip).clicked() {
                    state.brush.mode = if selected { MaskBrushMode::Off } else { mode };
                }
            }
        });
        if state.brush.is_active() || mask_count > 0 {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Brush").color(palette.text_weak));
                ui.add(
                    egui::Slider::new(&mut state.brush.radius, 1.0..=16.0)
                        .suffix(" st")
                        .max_decimals(1),
                );
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.brush.show_overlay, "Show Mask");
                if mask_count > 0 {
                    ui.label(
                        egui::RichText::new(format!("{mask_count} cells"))
                            .size(theme::type_scale::SMALL)
                            .color(palette.text_weak),
                    );
                    if ui
                        .small_button("Clear")
                        .on_hover_text("Clear all manually painted interior/exterior cells")
                        .clicked()
                    {
                        action = Some(WeatherInspectorAction::ClearSurfaceMask);
                    }
                }
            });
        }

        // Custom Weather Profiles.
        ui.add_space(theme::space::S);
        ui.separator();
        ui.label(
            egui::RichText::new("Custom Profiles")
                .size(theme::type_scale::SMALL)
                .color(palette.text_weak),
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.new_profile_name)
                    .hint_text("Profile name…")
                    .desired_width((ui.available_width() - 62.0).max(80.0)),
            );
            let can_save = !state.new_profile_name.trim().is_empty();
            if ui
                .add_enabled(can_save, egui::Button::new("Save"))
                .on_hover_text("Save current weather and wind settings as a reusable profile")
                .clicked()
            {
                let name = state.new_profile_name.trim().to_owned();
                let profile = current_profile(world, name);
                state.save_custom_profile(project_root, profile);
                state.new_profile_name.clear();
            }
        });
        let mut delete_index = None;
        for (idx, profile) in state.custom_profiles.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui
                    .small_button(&profile.name)
                    .on_hover_text("Apply this custom weather profile")
                    .clicked()
                {
                    action = Some(WeatherInspectorAction::ApplyProfile(profile.clone()));
                }
                if ui
                    .small_button("×")
                    .on_hover_text("Delete custom profile")
                    .clicked()
                {
                    delete_index = Some(idx);
                }
            });
        }
        if let Some(idx) = delete_index {
            state.delete_custom_profile(project_root, idx);
        }
    });

    action
}

/// Applies a [`WeatherInspectorAction`] to `world`, creating `Lighting/Weather`
/// automatically if needed. Returns the `Weather` instance id if touched.
pub fn apply_action(world: &mut World, action: WeatherInspectorAction) -> Option<InstanceId> {
    let ensure_weather = |world: &mut World| -> Option<InstanceId> {
        if let Some(id) = find_weather(world) {
            return Some(id);
        }
        let id = world.create(ClassName::Weather).ok()?;
        world.set_parent(id, Some(world.lighting())).ok()?;
        Some(id)
    };
    let set_enum = |world: &mut World, id: InstanceId, item: &str| {
        let _ = world.set_property(
            id,
            "Precipitation",
            Value::EnumItem(EnumItem {
                enum_name: "Precipitation".to_owned(),
                item: item.to_owned(),
            }),
        );
    };

    match action {
        WeatherInspectorAction::ToggleEnabled(on) => {
            let id = ensure_weather(world)?;
            let _ = world.set_property(id, "Enabled", Value::Bool(on));
            Some(id)
        }
        WeatherInspectorAction::ApplyProfile(profile) => {
            let id = ensure_weather(world)?;
            let _ = world.set_property(id, "Enabled", Value::Bool(profile.enabled));
            set_enum(world, id, &profile.precipitation);
            let _ = world.set_property(id, "Intensity", Value::Number(profile.intensity));
            let _ = world.set_property(id, "Wetness", Value::Number(profile.wetness));
            let _ = world.set_property(id, "SnowCover", Value::Number(profile.snow_cover));
            let _ = world.set_property(id, "SnowThickness", Value::Number(profile.snow_thickness));
            let _ =
                world.set_property(id, "TransitionTime", Value::Number(profile.transition_time));
            let ws = world.workspace();
            let _ = world.set_property(
                ws,
                "GlobalWind",
                Value::Vector3(Vector3::new(
                    profile.wind[0],
                    profile.wind[1],
                    profile.wind[2],
                )),
            );
            Some(id)
        }
        WeatherInspectorAction::SetPrecipitation(item) => {
            let id = ensure_weather(world)?;
            let _ = world.set_property(id, "Enabled", Value::Bool(true));
            set_enum(world, id, item);
            Some(id)
        }
        WeatherInspectorAction::SetBool(prop, value) => {
            let id = ensure_weather(world)?;
            let _ = world.set_property(id, prop, Value::Bool(value));
            Some(id)
        }
        WeatherInspectorAction::SetNumber(prop, value) => {
            let id = ensure_weather(world)?;
            let _ = world.set_property(id, prop, Value::Number(value));
            Some(id)
        }
        WeatherInspectorAction::ClearSurfaceMask => {
            let id = find_weather(world)?;
            let _ = world.set_property(id, "SurfaceMask", Value::String(String::new()));
            Some(id)
        }
        WeatherInspectorAction::SelectWeather => find_weather(world),
    }
}

/// Paints the 3D surface-mask brush ring and existing manual interior/exterior
/// cells onto `rect`, and returns a new `SurfaceMask` string if the user
/// clicked or dragged to paint this frame.
pub fn viewport_paint_and_overlay(
    ctx: &egui::Context,
    rect: egui::Rect,
    camera: axcenity_render::Camera,
    world: &World,
    brush: &MaskBrushState,
) -> Option<String> {
    if !brush.is_active() && !brush.show_overlay {
        return None;
    }
    let width = rect.width().max(1.0);
    let height = rect.height().max(1.0);
    let view = CameraView::new(camera, width / height);
    let screen = axcenity_editor_kit::screen::Screen::new(camera, rect);
    let project = |p: Vec3| -> Option<egui::Pos2> { screen.point(p) };

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("weather_mask_brush_overlay"),
    ));
    let painter = painter.with_clip_rect(rect);

    // Draw existing painted Interior (blue) / Exterior (amber) cells around camera.
    let weather_raw = find_weather(world)
        .and_then(|id| world.get(id))
        .and_then(|w| match w.property("SurfaceMask") {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");
    let mut mask = parse_surface_mask(weather_raw);
    if brush.show_overlay && !mask.is_empty() {
        for (&(cx, cz), &kind) in &mask {
            let center = Vec3::new(cx as f32 + 0.5, 0.15, cz as f32 + 0.5);
            if (center - camera.position).length_squared() > 64.0 * 64.0 {
                continue;
            }
            if let Some(screen_pos) = project(center) {
                let color = match kind {
                    SurfaceMaskKind::Interior => {
                        egui::Color32::from_rgba_unmultiplied(80, 160, 255, 190)
                    }
                    SurfaceMaskKind::Exterior => {
                        egui::Color32::from_rgba_unmultiplied(245, 180, 60, 190)
                    }
                };
                painter.circle_filled(screen_pos, 3.0, color);
            }
        }
    }

    if !brush.is_active() {
        return None;
    }

    let (pointer_pos, primary_down) = ctx.input(|input| {
        (
            input.pointer.hover_pos(),
            input.pointer.primary_down() && !input.modifiers.alt,
        )
    });
    let pointer = pointer_pos.filter(|pos| rect.contains(*pos))?;
    let uv = Vec2::new(
        (pointer.x - rect.left()) / width,
        (pointer.y - rect.top()) / height,
    );
    let ray = view.ray(uv);
    let hit_point = if let Some(hit) = pick_hit(world, ray, None) {
        ray.at(hit.distance)
    } else if ray.direction.y.abs() > 1e-4 {
        let t = -ray.origin.y / ray.direction.y;
        if t > 0.0 && t < 500.0 {
            ray.at(t)
        } else {
            return None;
        }
    } else {
        return None;
    };

    // Draw 3D brush circle on the surface.
    let ring_color = match brush.mode {
        MaskBrushMode::Interior => egui::Color32::from_rgb(80, 170, 255),
        MaskBrushMode::Exterior => egui::Color32::from_rgb(245, 185, 65),
        MaskBrushMode::Erase => egui::Color32::from_rgb(235, 100, 100),
        MaskBrushMode::Off => egui::Color32::TRANSPARENT,
    };
    const SEGMENTS: usize = 32;
    let mut ring = Vec::with_capacity(SEGMENTS + 1);
    for i in 0..=SEGMENTS {
        let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
        let pt = Vec3::new(
            hit_point.x + angle.cos() * brush.radius,
            hit_point.y + 0.1,
            hit_point.z + angle.sin() * brush.radius,
        );
        if let Some(sp) = project(pt) {
            ring.push(sp);
        }
    }
    if ring.len() >= 2 {
        painter.add(egui::Shape::line(ring, egui::Stroke::new(2.0, ring_color)));
    }

    if primary_down
        && paint_surface_mask(
            &mut mask,
            hit_point.x,
            hit_point.z,
            brush.radius,
            brush.mode.mask_kind(),
        )
    {
        return Some(format_surface_mask(&mask));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_profiles_apply_to_world_and_round_trip() {
        let mut world = World::new();
        let blizzard = built_in_profiles()
            .into_iter()
            .find(|p| p.name == "Blizzard")
            .unwrap();
        let id = apply_action(
            &mut world,
            WeatherInspectorAction::ApplyProfile(blizzard.clone()),
        )
        .expect("creates Lighting/Weather");
        assert_eq!(find_weather(&world), Some(id));
        let read_back = current_profile(&world, "Blizzard".to_owned());
        assert_eq!(read_back, blizzard);

        apply_action(&mut world, WeatherInspectorAction::ToggleEnabled(false));
        assert!(!current_profile(&world, "Off".to_owned()).enabled);
    }

    #[test]
    fn custom_profiles_persist_to_disk() {
        let temp = std::env::temp_dir().join(format!(
            "axcenity_weather_profiles_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        let mut state = WeatherInspectorState::default();
        state.sync_project(Some(temp.as_path()));
        let profile = WeatherProfile {
            name: "Arctic Gale".to_owned(),
            enabled: true,
            precipitation: "Snow".to_owned(),
            intensity: 0.9,
            wetness: 0.0,
            snow_cover: 1.0,
            snow_thickness: 1.5,
            transition_time: 4.0,
            wind: [35.0, 0.0, 5.0],
        };
        state.save_custom_profile(Some(temp.as_path()), profile.clone());

        let mut reloaded = WeatherInspectorState::default();
        reloaded.sync_project(Some(temp.as_path()));
        assert_eq!(reloaded.custom_profiles, vec![profile]);

        reloaded.delete_custom_profile(Some(temp.as_path()), 0);
        let mut after_delete = WeatherInspectorState::default();
        after_delete.sync_project(Some(temp.as_path()));
        assert!(after_delete.custom_profiles.is_empty());
        let _ = std::fs::remove_dir_all(&temp);
    }
}
