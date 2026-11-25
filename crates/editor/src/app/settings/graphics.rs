//! The Graphics page: this computer's preset, its overrides, and what's in
//! use once the GPU's limits are applied.

use super::*;

/// This computer's graphics settings: a preset, its overrides, and what's
/// in use once the GPU's limits are applied, with why anything was lowered.
/// True when Restart now was clicked.
///
/// `drawn` is what the viewport's last frame held on the graphics card: a
/// world that needs more than the memory budgets is said so under the
/// quality, where it's seen without scrolling
/// (`RenderStats::memory_warnings`).
pub(super) fn graphics_page(
    ui: &mut egui::Ui,
    settings: &mut GraphicsSettings,
    capabilities: &Capabilities,
    drawn: Option<&axcenity_render::RenderStats>,
) -> bool {
    page_title(
        ui,
        "Graphics",
        "This computer's quality, for the viewport and Play. Not saved in projects.",
    );
    let palette = theme::palette(ui.ctx());
    section(ui, "Quality");
    widgets::segmented(
        ui,
        &mut settings.quality,
        &Quality::ALL.map(|quality| (quality, None, quality.label())),
    );
    let preset = GraphicsSettings::preset(settings.quality);
    // What the world in the viewport needs past the memory budgets.
    let memory = drawn
        .map(|stats| stats.memory_warnings(&settings.effective(capabilities)))
        .unwrap_or_default();
    if !memory.is_empty() {
        ui.add_space(space::S);
        for warning in &memory {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(theme::with_icon(icons::WARNING, warning))
                        .size(type_scale::SMALL)
                        .color(palette.warning),
                )
                .wrap(),
            );
        }
    }
    ui.add_space(space::L);
    section(ui, "Overrides");
    egui::Grid::new("graphics overrides")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            found_label(ui, "Shadow quality");
            let label = |quality: Option<ShadowQuality>| match quality {
                Some(quality) => quality.label().to_owned(),
                None => format!("Preset ({})", preset.shadow_quality().label()),
            };
            egui::ComboBox::from_id_salt("shadow quality")
                .selected_text(label(settings.shadow_quality))
                .show_ui(ui, |ui| {
                    for choice in std::iter::once(None).chain(ShadowQuality::ALL.map(Some)) {
                        ui.selectable_value(&mut settings.shadow_quality, choice, label(choice));
                    }
                });
            ui.end_row();

            found_label(ui, "Shadow distance");
            ui.horizontal(|ui| {
                let mut limited = settings.shadow_distance.is_some();
                if ui
                    .checkbox(&mut limited, "")
                    .on_hover_text("Off, the preset's distance is used.")
                    .changed()
                {
                    settings.shadow_distance = limited.then(|| preset.shadow_distance());
                }
                match &mut settings.shadow_distance {
                    Some(distance) => {
                        ui.add(
                            egui::DragValue::new(distance)
                                .range(0.0..=5000.0)
                                .speed(1.0)
                                .suffix(" studs"),
                        );
                    }
                    None => {
                        ui.label(
                            egui::RichText::new(format!(
                                "Preset ({} studs)",
                                preset.shadow_distance()
                            ))
                            .color(palette.text_secondary),
                        );
                    }
                }
            });
            ui.end_row();

            found_label(ui, "Post-processing");
            let label = |on: Option<bool>| match on {
                Some(true) => "On".to_owned(),
                Some(false) => "Off".to_owned(),
                None => format!(
                    "Preset ({})",
                    if preset.post_processing() {
                        "On"
                    } else {
                        "Off"
                    }
                ),
            };
            egui::ComboBox::from_id_salt("post-processing")
                .selected_text(label(settings.post_processing))
                .show_ui(ui, |ui| {
                    for choice in [None, Some(true), Some(false)] {
                        ui.selectable_value(&mut settings.post_processing, choice, label(choice));
                    }
                });
            ui.end_row();

            found_label(ui, "Render scale");
            ui.horizontal(|ui| {
                let mut custom = settings.render_scale.is_some();
                if ui
                    .checkbox(&mut custom, "")
                    .on_hover_text("Off, the preset's scale is used.")
                    .changed()
                {
                    settings.render_scale = custom.then(|| preset.render_scale());
                }
                match &mut settings.render_scale {
                    Some(scale) => {
                        let mut percent = (*scale * 100.0).round();
                        let range = RENDER_SCALE_RANGE;
                        if ui
                            .add(
                                egui::DragValue::new(&mut percent)
                                    .range(range.start() * 100.0..=range.end() * 100.0)
                                    .speed(1.0)
                                    .suffix("%"),
                            )
                            .changed()
                        {
                            *scale = percent / 100.0;
                        }
                    }
                    None => {
                        ui.label(
                            egui::RichText::new(format!(
                                "Preset ({}%)",
                                (preset.render_scale() * 100.0).round()
                            ))
                            .color(palette.text_secondary),
                        );
                    }
                }
            });
            ui.end_row();

            found_label(ui, "Texture quality");
            let label = |quality: Option<TextureQuality>| match quality {
                Some(quality) => quality.label().to_owned(),
                None => format!("Preset ({})", preset.texture_quality().label()),
            };
            egui::ComboBox::from_id_salt("texture quality")
                .selected_text(label(settings.texture_quality))
                .show_ui(ui, |ui| {
                    for choice in std::iter::once(None).chain(TextureQuality::ALL.map(Some)) {
                        ui.selectable_value(&mut settings.texture_quality, choice, label(choice));
                    }
                });
            ui.end_row();

            found_label(ui, "Texture memory");
            ui.horizontal(|ui| {
                let mut custom = settings.texture_budget.is_some();
                if ui
                    .checkbox(&mut custom, "")
                    .on_hover_text(
                        "Off, the budget follows this graphics card's memory, \
                         or is the preset's where that isn't known.",
                    )
                    .changed()
                {
                    settings.texture_budget =
                        custom.then(|| preset.texture_budget_on(capabilities));
                }
                match &mut settings.texture_budget {
                    Some(budget) => {
                        ui.add(
                            egui::DragValue::new(budget)
                                .range(TEXTURE_BUDGET_RANGE)
                                .speed(4.0)
                                .suffix(" MB"),
                        );
                    }
                    None => {
                        let (text, why) = automatic_texture_budget(&preset, capabilities);
                        ui.label(egui::RichText::new(text).color(palette.text_secondary))
                            .on_hover_text(why);
                    }
                }
            });
            ui.end_row();

            found_label(ui, "Anti-aliasing");
            let label = |mode: Option<AntiAliasing>| match mode {
                Some(mode) => mode.label().to_owned(),
                None => format!("Preset ({})", preset.anti_aliasing().label()),
            };
            egui::ComboBox::from_id_salt("anti-aliasing")
                .selected_text(label(settings.anti_aliasing))
                .show_ui(ui, |ui| {
                    for choice in std::iter::once(None).chain(AntiAliasing::ALL.map(Some)) {
                        ui.selectable_value(&mut settings.anti_aliasing, choice, label(choice));
                    }
                });
            ui.end_row();

            found_label(ui, "Ambient occlusion");
            let label = |mode: Option<AmbientOcclusion>| match mode {
                Some(mode) => mode.label().to_owned(),
                None => format!("Preset ({})", preset.ambient_occlusion().label()),
            };
            egui::ComboBox::from_id_salt("ambient occlusion")
                .selected_text(label(settings.ambient_occlusion))
                .show_ui(ui, |ui| {
                    for choice in std::iter::once(None).chain(AmbientOcclusion::ALL.map(Some)) {
                        ui.selectable_value(&mut settings.ambient_occlusion, choice, label(choice));
                    }
                });
            ui.end_row();

            found_label(ui, "Reflection quality");
            let label = |quality: Option<ReflectionQuality>| match quality {
                Some(quality) => quality.label().to_owned(),
                None => format!("Preset ({})", preset.reflection_quality().label()),
            };
            egui::ComboBox::from_id_salt("reflection quality")
                .selected_text(label(settings.reflection_quality))
                .show_ui(ui, |ui| {
                    for choice in std::iter::once(None).chain(ReflectionQuality::ALL.map(Some)) {
                        ui.selectable_value(
                            &mut settings.reflection_quality,
                            choice,
                            label(choice),
                        );
                    }
                });
            ui.end_row();

            found_label(ui, "Cloud quality");
            let label = |quality: Option<CloudQuality>| match quality {
                Some(quality) => quality.label().to_owned(),
                None => format!("Preset ({})", preset.cloud_quality().label()),
            };
            egui::ComboBox::from_id_salt("cloud quality")
                .selected_text(label(settings.cloud_quality))
                .show_ui(ui, |ui| {
                    for choice in std::iter::once(None).chain(CloudQuality::ALL.map(Some)) {
                        ui.selectable_value(&mut settings.cloud_quality, choice, label(choice));
                    }
                });
            ui.end_row();

            found_label(ui, "Contact shadows");
            let on_off = |on: bool| if on { "On" } else { "Off" };
            let label = |on: Option<bool>| match on {
                Some(on) => on_off(on).to_owned(),
                None => format!("Preset ({})", on_off(preset.contact_shadows())),
            };
            egui::ComboBox::from_id_salt("contact shadows")
                .selected_text(label(settings.contact_shadows))
                .show_ui(ui, |ui| {
                    for choice in [None, Some(true), Some(false)] {
                        ui.selectable_value(&mut settings.contact_shadows, choice, label(choice));
                    }
                });
            ui.end_row();

            found_label(ui, "Real-time GI");
            let label = |on: Option<bool>| match on {
                Some(on) => on_off(on).to_owned(),
                None => format!("Preset ({})", on_off(preset.realtime_gi())),
            };
            egui::ComboBox::from_id_salt("realtime gi")
                .selected_text(label(settings.realtime_gi))
                .show_ui(ui, |ui| {
                    for choice in [None, Some(true), Some(false)] {
                        ui.selectable_value(&mut settings.realtime_gi, choice, label(choice));
                    }
                })
                .response
                .on_hover_text(
                    "Light probe volumes traced again while the world's light changes, \
                     instead of keeping their bakes. Off at every preset. On, the preset sets \
                     how much it traces: Low one probe a frame at 8² a face, Medium two at \
                     8², High two at 16² (a bake's size) and Ultra four at 16².",
                );
            ui.end_row();

            found_label(ui, "Volumetric fog");
            let label = |on: Option<bool>| match on {
                Some(on) => on_off(on).to_owned(),
                None => format!("Preset ({})", on_off(preset.volumetric_fog())),
            };
            egui::ComboBox::from_id_salt("volumetric fog")
                .selected_text(label(settings.volumetric_fog))
                .show_ui(ui, |ui| {
                    for choice in [None, Some(true), Some(false)] {
                        ui.selectable_value(&mut settings.volumetric_fog, choice, label(choice));
                    }
                });
            ui.end_row();

            found_label(ui, "Depth of field");
            let label = |quality: Option<DepthOfFieldQuality>| match quality {
                Some(quality) => quality.label().to_owned(),
                None => format!("Preset ({})", preset.depth_of_field().label()),
            };
            egui::ComboBox::from_id_salt("depth of field")
                .selected_text(label(settings.depth_of_field))
                .show_ui(ui, |ui| {
                    for choice in std::iter::once(None).chain(DepthOfFieldQuality::ALL.map(Some)) {
                        ui.selectable_value(&mut settings.depth_of_field, choice, label(choice));
                    }
                });
            ui.end_row();

            found_label(ui, "Motion blur");
            let label = |quality: Option<MotionBlurQuality>| match quality {
                Some(quality) => quality.label().to_owned(),
                None => format!("Preset ({})", preset.motion_blur().label()),
            };
            egui::ComboBox::from_id_salt("motion blur")
                .selected_text(label(settings.motion_blur))
                .show_ui(ui, |ui| {
                    for choice in std::iter::once(None).chain(MotionBlurQuality::ALL.map(Some)) {
                        ui.selectable_value(&mut settings.motion_blur, choice, label(choice));
                    }
                });
            ui.end_row();

            let switches = [
                ("Vignette", preset.vignette(), &mut settings.vignette),
                ("Film grain", preset.film_grain(), &mut settings.film_grain),
                (
                    "Chromatic aberration",
                    preset.chromatic_aberration(),
                    &mut settings.chromatic_aberration,
                ),
                ("Sharpen", preset.sharpen(), &mut settings.sharpen),
                ("Sun rays", preset.sun_rays(), &mut settings.sun_rays),
            ];
            for (name, preset_on, value) in switches {
                found_label(ui, name);
                let label = |on: Option<bool>| match on {
                    Some(on) => on_off(on).to_owned(),
                    None => format!("Preset ({})", on_off(preset_on)),
                };
                egui::ComboBox::from_id_salt(name)
                    .selected_text(label(*value))
                    .show_ui(ui, |ui| {
                        for choice in [None, Some(true), Some(false)] {
                            ui.selectable_value(value, choice, label(choice));
                        }
                    });
                ui.end_row();
            }
        });
    ui.add_space(space::L);
    let restart = ray_tracing_section(ui, settings, capabilities, &preset);
    ui.add_space(space::L);
    in_use_section(ui, settings, capabilities, &preset);
    restart
}

/// Structured, scannable summary of what the renderer is actually using after
/// preset defaults, custom overrides and GPU capability limits are applied.
fn in_use_section(
    ui: &mut egui::Ui,
    settings: &GraphicsSettings,
    capabilities: &Capabilities,
    preset: &GraphicsSettings,
) {
    let palette = theme::palette(ui.ctx());
    section(ui, "In use");
    let effective = settings.effective(capabilities);

    let badge = |ui: &mut egui::Ui, text: &str, fg: egui::Color32, bg: egui::Color32| {
        egui::Frame::new()
            .fill(bg)
            .stroke(egui::Stroke::new(1.0_f32, palette.border))
            .corner_radius(theme::radius::CONTROL)
            .inner_margin(egui::Margin::symmetric(7, 2))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(text)
                        .size(type_scale::CAPTION)
                        .family(theme::medium_family())
                        .color(fg),
                );
            });
    };

    egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0_f32, palette.border))
        .corner_radius(theme::radius::CARD)
        .inner_margin(space::margin(space::M, space::S))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Top at-a-glance status pills.
            let rt_summary = ray_tracing_in_use(&effective);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                badge(
                    ui,
                    &format!("Preset: {}", settings.quality.label()),
                    palette.accent_text,
                    palette.accent_soft,
                );
                badge(
                    ui,
                    &format!("Scale: {}%", (effective.render_scale * 100.0).round()),
                    palette.text_strong,
                    palette.control,
                );
                badge(
                    ui,
                    &format!("AA: {}", effective.anti_aliasing.label()),
                    palette.text_strong,
                    palette.control,
                );
                let rt_any = effective.ray_traced_shadows != RayTracedShadows::Off
                    || effective.ray_traced_ambient_occlusion != RayTracedAmbientOcclusion::Off
                    || effective.ray_traced_global_illumination != RayTracedGlobalIllumination::Off
                    || effective.ray_traced_reflections != RayTracedReflections::Off;
                let rt_badge = ui.horizontal(|ui| {
                    if effective.ray_queries && rt_any {
                        badge(
                            ui,
                            "Hardware RT active",
                            palette.success,
                            theme::mix(palette.surface, palette.success, 0.14),
                        );
                    } else if effective.ray_traced_gi_in_software {
                        badge(
                            ui,
                            "Software GI active",
                            palette.accent_text,
                            palette.accent_soft,
                        );
                    } else if effective.ray_queries {
                        badge(
                            ui,
                            "Hardware RT ready",
                            palette.text_secondary,
                            palette.control,
                        );
                    } else {
                        badge(ui, "Raster only", palette.text_weak, palette.control);
                    }
                });
                rt_badge.response.on_hover_text(rt_summary);
            });

            ui.add_space(space::S);
            ui.separator();
            ui.add_space(space::XS);

            let row = |ui: &mut egui::Ui,
                       label: &str,
                       status: &str,
                       detail: &str,
                       active: bool,
                       overridden: bool| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(label)
                            .size(type_scale::SMALL)
                            .color(palette.text_secondary),
                    );
                    if overridden {
                        ui.label(
                            egui::RichText::new("•")
                                .size(type_scale::SMALL)
                                .color(palette.accent),
                        )
                        .on_hover_text("Overridden from the selected quality preset.");
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(
                        egui::RichText::new(status)
                            .size(type_scale::SMALL)
                            .family(theme::medium_family())
                            .color(if active {
                                palette.text_strong
                            } else {
                                palette.text_weak
                            }),
                    );
                    if !detail.is_empty() {
                        ui.label(
                            egui::RichText::new(detail)
                                .size(type_scale::SMALL)
                                .color(palette.text_weak),
                        );
                    }
                });
                ui.end_row();
            };

            let group_header = |ui: &mut egui::Ui, title: &str| {
                ui.label(widgets::micro(title, palette.text_weak));
                ui.label("");
                ui.end_row();
            };

            egui::Grid::new("graphics in use summary")
                .num_columns(2)
                .spacing([14.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    group_header(ui, "Core & resolution");
                    row(
                        ui,
                        "Render scale",
                        &format!("{}%", (effective.render_scale * 100.0).round()),
                        "Scene resolution (UI always 100%)",
                        true,
                        settings.render_scale.is_some(),
                    );
                    row(
                        ui,
                        "Textures",
                        effective.texture_quality.label(),
                        &format!(
                            "{} MB budget, {} MB for meshes",
                            effective.texture_budget >> 20,
                            effective.mesh_budget >> 20
                        ),
                        true,
                        settings.texture_quality.is_some() || settings.texture_budget.is_some(),
                    );
                    row(
                        ui,
                        "Anti-aliasing",
                        effective.anti_aliasing.label(),
                        match effective.anti_aliasing {
                            AntiAliasing::Off => "No smoothing",
                            AntiAliasing::Fxaa => "Fast post-process pass",
                            AntiAliasing::Taa => "Temporal smoothing (recommended for RT)",
                            AntiAliasing::Msaa2 => "2× hardware multisampling",
                            AntiAliasing::Msaa4 => "4× hardware multisampling",
                        },
                        effective.anti_aliasing != AntiAliasing::Off,
                        settings.anti_aliasing.is_some(),
                    );

                    group_header(ui, "Lighting & shadows");
                    match effective.shadow_quality.map() {
                        Some(map) => row(
                            ui,
                            "Shadow maps",
                            effective.shadow_quality.label(),
                            &format!(
                                "{}² · {} cascades · {} studs",
                                map.size, map.cascades, effective.shadow_distance
                            ),
                            true,
                            settings.shadow_quality.is_some() || settings.shadow_distance.is_some(),
                        ),
                        None => row(
                            ui,
                            "Shadow maps",
                            "Off",
                            "No directional shadow maps",
                            false,
                            settings.shadow_quality.is_some(),
                        ),
                    }
                    row(
                        ui,
                        "Contact shadows",
                        if effective.contact_shadows {
                            "On"
                        } else {
                            "Off"
                        },
                        if effective.contact_shadows {
                            "Fine screen-space crevices"
                        } else {
                            ""
                        },
                        effective.contact_shadows,
                        settings.contact_shadows.is_some(),
                    );
                    match effective.ambient_occlusion {
                        AmbientOcclusion::Off => row(
                            ui,
                            "Ambient occlusion",
                            "Off",
                            "",
                            false,
                            settings.ambient_occlusion.is_some(),
                        ),
                        mode => row(
                            ui,
                            "Ambient occlusion",
                            mode.label(),
                            &format!("SSAO · {} samples/px", mode.samples()),
                            true,
                            settings.ambient_occlusion.is_some(),
                        ),
                    }
                    match effective.reflection_quality.map() {
                        Some(map) => row(
                            ui,
                            "Reflections",
                            effective.reflection_quality.label(),
                            &format!("{}² sky cubemap · {} samples", map.size, map.samples),
                            true,
                            settings.reflection_quality.is_some(),
                        ),
                        None => row(
                            ui,
                            "Reflections",
                            "Low",
                            "Sky gradient fallback",
                            true,
                            settings.reflection_quality.is_some(),
                        ),
                    }
                    if effective.ray_traced_global_illumination != RayTracedGlobalIllumination::Off
                    {
                        let gi = effective.ray_traced_global_illumination;
                        row(
                            ui,
                            "Global illumination",
                            &format!(
                                "RT {} ({})",
                                gi.label(),
                                if effective.ray_traced_gi_in_software {
                                    "Software"
                                } else {
                                    "Hardware"
                                }
                            ),
                            &format!(
                                "{} probes/frame · {} rays/probe",
                                gi.probes_per_frame(),
                                gi.rays()
                            ),
                            true,
                            settings.ray_traced_global_illumination.is_some(),
                        );
                    } else if effective.realtime_gi {
                        let tier = effective.realtime_gi_tier;
                        row(
                            ui,
                            "Global illumination",
                            "Real-time probes",
                            &format!(
                                "{} probes/frame · {}² per face",
                                tier.probes_per_frame, tier.face_size
                            ),
                            true,
                            settings.realtime_gi.is_some(),
                        );
                    } else {
                        row(
                            ui,
                            "Global illumination",
                            "Baked",
                            "Light probe volumes keep their bakes",
                            false,
                            settings.realtime_gi.is_some(),
                        );
                    }

                    group_header(ui, "Ray tracing");
                    let reach = effective.ray_tracing_distance.round();
                    row(
                        ui,
                        "Hardware RT",
                        if effective.ray_queries {
                            "Active"
                        } else {
                            "Off"
                        },
                        &if effective.ray_queries {
                            format!("Up to {reach} studs reach")
                        } else {
                            "Enable Hardware ray tracing & restart".to_owned()
                        },
                        effective.ray_queries,
                        settings.ray_tracing.is_some() || settings.ray_tracing_distance.is_some(),
                    );
                    if effective.ray_queries {
                        let mut active_rt = Vec::new();
                        if effective.ray_traced_shadows != RayTracedShadows::Off {
                            active_rt.push(format!(
                                "Shadows {} ({} rpp)",
                                effective.ray_traced_shadows.label(),
                                effective.ray_traced_shadows.rays()
                            ));
                        }
                        if effective.ray_traced_ambient_occlusion != RayTracedAmbientOcclusion::Off
                        {
                            active_rt.push(format!(
                                "AO {} ({} rpp)",
                                effective.ray_traced_ambient_occlusion.label(),
                                effective.ray_traced_ambient_occlusion.rays()
                            ));
                        }
                        if effective.ray_traced_reflections != RayTracedReflections::Off {
                            let refl = effective.ray_traced_reflections;
                            active_rt.push(format!(
                                "Reflections {} (< {} rough{})",
                                refl.label(),
                                refl.roughness_cutoff(),
                                if refl.downscale() > 1 { ", ½ res" } else { "" }
                            ));
                        }
                        if effective.ray_traced_global_illumination
                            != RayTracedGlobalIllumination::Off
                        {
                            active_rt.push(format!(
                                "GI {}",
                                effective.ray_traced_global_illumination.label()
                            ));
                        }
                        row(
                            ui,
                            "RT effects",
                            if active_rt.is_empty() { "None" } else { "On" },
                            &if active_rt.is_empty() {
                                "Turn on individual effects above".to_owned()
                            } else {
                                active_rt.join(" · ")
                            },
                            !active_rt.is_empty(),
                            false,
                        );
                    }

                    group_header(ui, "Atmosphere & post-processing");
                    match effective.cloud_quality.steps() {
                        (0, _) => row(
                            ui,
                            "Clouds",
                            "Low",
                            "Flat cloud sheet",
                            true,
                            settings.cloud_quality.is_some(),
                        ),
                        (steps, _) => row(
                            ui,
                            "Clouds",
                            effective.cloud_quality.label(),
                            &format!("Volumetric · {steps} march steps"),
                            true,
                            settings.cloud_quality.is_some(),
                        ),
                    }
                    row(
                        ui,
                        "Volumetric fog",
                        if effective.volumetric_fog {
                            "On"
                        } else {
                            "Off"
                        },
                        if effective.volumetric_fog {
                            "Sun shafts and shadowed lamp fog"
                        } else {
                            "Unshadowed atmosphere fog"
                        },
                        effective.volumetric_fog,
                        settings.volumetric_fog.is_some(),
                    );
                    row(
                        ui,
                        "Post-processing",
                        if effective.post_processing {
                            "On"
                        } else {
                            "Off"
                        },
                        if effective.post_processing {
                            "Bloom & tone mapping"
                        } else {
                            ""
                        },
                        effective.post_processing,
                        settings.post_processing.is_some(),
                    );
                    match effective.depth_of_field {
                        DepthOfFieldQuality::Off => row(
                            ui,
                            "Depth of field",
                            "Off",
                            "",
                            false,
                            settings.depth_of_field.is_some(),
                        ),
                        quality => row(
                            ui,
                            "Depth of field",
                            quality.label(),
                            &format!("{} samples/px", quality.samples()),
                            true,
                            settings.depth_of_field.is_some(),
                        ),
                    }
                    match effective.motion_blur {
                        MotionBlurQuality::Off => row(
                            ui,
                            "Motion blur",
                            "Off",
                            "",
                            false,
                            settings.motion_blur.is_some(),
                        ),
                        quality => row(
                            ui,
                            "Motion blur",
                            quality.label(),
                            &format!("{} samples/px", quality.samples()),
                            true,
                            settings.motion_blur.is_some(),
                        ),
                    }
                    let lens: Vec<&str> = [
                        (effective.vignette, "Vignette"),
                        (effective.film_grain, "Film grain"),
                        (effective.chromatic_aberration, "Chromatic aberration"),
                        (effective.sharpen, "Sharpening"),
                        (effective.sun_rays, "Sun rays"),
                    ]
                    .into_iter()
                    .filter_map(|(on, name)| on.then_some(name))
                    .collect();
                    let lens_overridden = settings.vignette.is_some()
                        || settings.film_grain.is_some()
                        || settings.chromatic_aberration.is_some()
                        || settings.sharpen.is_some()
                        || settings.sun_rays.is_some();
                    row(
                        ui,
                        "Lens effects",
                        if lens.is_empty() { "None" } else { "On" },
                        &lens.join(", "),
                        !lens.is_empty(),
                        lens_overridden,
                    );
                    let _ = preset;
                });

            if !effective.notes.is_empty() {
                ui.add_space(space::S);
                ui.separator();
                ui.add_space(space::XS);
                for note in &effective.notes {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(theme::with_icon(icons::WARNING, note))
                                .size(type_scale::SMALL)
                                .color(palette.warning),
                        )
                        .wrap(),
                    );
                }
            }
        });

    ui.add_space(space::XS);
    ui.add(
        egui::Label::new(
            egui::RichText::new(
                "Accent dot (•) marks custom overrides. A world's Lighting can ask for less \
                 (ShadowsEnabled, ShadowDistance, PostProcessingEnabled), never more.",
            )
            .size(type_scale::CAPTION)
            .color(palette.text_weak),
        )
        .wrap(),
    );
}

/// Hardware ray tracing: whether this GPU can, and how far rays reach. Each
/// ray traced effect is its own switch here, off unless turned on
/// (`docs/rendering/ray-tracing.md`).
/// The Ray tracing section. True when Restart now was clicked.
fn ray_tracing_section(
    ui: &mut egui::Ui,
    settings: &mut GraphicsSettings,
    capabilities: &Capabilities,
    preset: &GraphicsSettings,
) -> bool {
    let palette = theme::palette(ui.ctx());
    section(ui, "Ray tracing");

    let can_rt = axcenity_render::adapters::adapter_can_ray_trace();
    let restart_needed = needs_restart(capabilities.ray_queries, can_rt, settings.ray_tracing());
    let mut restart = false;

    // 1. Hardware acceleration & reach card.
    egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0_f32, palette.border))
        .corner_radius(theme::radius::CARD)
        .inner_margin(space::margin(space::M, space::S))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (pill_text, pill_color) =
                    match (capabilities.ray_queries, can_rt, settings.ray_tracing()) {
                        (true, _, true) => ("Hardware RT active", palette.success),
                        (true, _, false) | (false, true, true) => {
                            ("Restart required", palette.warning)
                        }
                        (false, true, false) => ("Hardware RT available", palette.text_secondary),
                        (false, false, _) => ("Software GI only", palette.text_weak),
                    };
                ui.label(
                    egui::RichText::new(theme::with_icon(icons::SPARKLE, "Hardware acceleration"))
                        .family(theme::medium_family())
                        .color(palette.text_strong),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::Frame::new()
                        .fill(theme::mix(palette.surface, pill_color, 0.16))
                        .corner_radius(theme::radius::CONTROL)
                        .inner_margin(egui::Margin::symmetric(7, 2))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(pill_text)
                                    .size(type_scale::CAPTION)
                                    .family(theme::medium_family())
                                    .color(pill_color),
                            );
                        });
                });
            });
            ui.add_space(space::XXS);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(ray_tracing_status(
                        capabilities.ray_queries,
                        can_rt,
                        settings.ray_tracing(),
                    ))
                    .size(type_scale::SMALL)
                    .color(palette.text_secondary),
                )
                .wrap(),
            );
            ui.add_space(space::S);

            egui::Grid::new("ray tracing hardware settings")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    found_label(ui, "Hardware ray tracing");
                    let mut on = settings.ray_tracing();
                    if ui
                        .checkbox(&mut on, "Enable on startup (requires restart)")
                        .on_hover_text(
                            "Opens the GPU with hardware ray tracing the next time the editor \
                             starts, unlocking ray traced shadows, ambient occlusion, reflections, \
                             hardware GI and View Mode > Ray Hits.",
                        )
                        .changed()
                    {
                        settings.ray_tracing = on.then_some(true);
                    }
                    ui.end_row();

                    found_label(ui, "Ray tracing distance");
                    ui.horizontal(|ui| {
                        let mut custom = settings.ray_tracing_distance.is_some();
                        if ui
                            .checkbox(&mut custom, "")
                            .on_hover_text(
                                "Off, the preset's distance is used. Farther traces more of the \
                                 world and costs more to keep up to date.",
                            )
                            .changed()
                        {
                            settings.ray_tracing_distance =
                                custom.then(|| preset.ray_tracing_distance());
                        }
                        match &mut settings.ray_tracing_distance {
                            Some(distance) => {
                                ui.add(
                                    egui::DragValue::new(distance)
                                        .range(RAY_TRACING_DISTANCE_RANGE)
                                        .speed(5.0)
                                        .suffix(" studs"),
                                );
                            }
                            None => {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Preset ({} studs)",
                                        preset.ray_tracing_distance()
                                    ))
                                    .color(palette.text_secondary),
                                );
                            }
                        }
                    });
                    ui.end_row();
                });

            if restart_needed {
                ui.add_space(space::S);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(
                            if settings.ray_tracing()
                                && axcenity_render::adapters::ray_tracing_override() == Some(false)
                            {
                                "AXCENITY_RAY_TRACING=0 keeps ray tracing off. Restart now starts \
                                 it without that."
                            } else if settings.ray_tracing() {
                                "Restart the editor to start hardware ray tracing."
                            } else {
                                "Restart the editor to turn hardware ray tracing off."
                            },
                        )
                        .size(type_scale::SMALL)
                        .color(palette.warning),
                    );
                    restart = ui
                        .add(widgets::button(
                            widgets::Role::Primary,
                            Some(icons::ARROW_CLOCKWISE),
                            "Restart now",
                            &palette,
                        ))
                        .on_hover_text(
                            "Saves these settings and starts the editor again with this project, \
                             carrying unsaved world and script edits.",
                        )
                        .clicked();
                });
            }
        });

    ui.add_space(space::S);

    // 2. Individual ray traced effects card.
    let any_effect_on = settings.ray_traced_shadows().ne(&RayTracedShadows::Off)
        || settings
            .ray_traced_ambient_occlusion()
            .ne(&RayTracedAmbientOcclusion::Off)
        || settings
            .ray_traced_global_illumination()
            .ne(&RayTracedGlobalIllumination::Off)
        || settings
            .ray_traced_reflections()
            .ne(&RayTracedReflections::Off);

    egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0_f32, palette.border))
        .corner_radius(theme::radius::CARD)
        .inner_margin(space::margin(space::M, space::S))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Effects")
                        .family(theme::medium_family())
                        .color(palette.text_strong),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(any_effect_on, egui::Button::new("Turn all off"))
                        .on_hover_text(
                            "Turns every ray traced effect off without changing Hardware ray \
                             tracing.",
                        )
                        .clicked()
                    {
                        *settings = settings.without_ray_traced_effects();
                    }
                });
            });
            ui.add_space(space::XS);

            egui::Grid::new("ray tracing effects")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    found_label(ui, "Ray traced shadows");
                    let label = |shadows: Option<RayTracedShadows>| match shadows {
                        Some(shadows) => shadows.label().to_owned(),
                        None => format!("Preset ({})", preset.ray_traced_shadows().label()),
                    };
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(capabilities.ray_queries, |ui| {
                            egui::ComboBox::from_id_salt("ray traced shadows")
                                .selected_text(label(settings.ray_traced_shadows))
                                .show_ui(ui, |ui| {
                                    for choice in
                                        std::iter::once(None).chain(RayTracedShadows::ALL.map(Some))
                                    {
                                        ui.selectable_value(
                                            &mut settings.ray_traced_shadows,
                                            choice,
                                            label(choice),
                                        );
                                    }
                                })
                                .response
                                .on_hover_text(
                                    "The sun's and lamps' shadows traced as rays: exact up close \
                                     with soft edges by blocker distance. Needs Hardware ray \
                                     tracing on.",
                                );
                        });
                        if !capabilities.ray_queries {
                            ui.label(
                                egui::RichText::new("Needs Hardware RT")
                                    .size(type_scale::CAPTION)
                                    .color(palette.text_weak),
                            );
                        }
                    });
                    ui.end_row();

                    found_label(ui, "Ray traced ambient occlusion");
                    let label = |rays: Option<RayTracedAmbientOcclusion>| match rays {
                        Some(rays) => rays.label().to_owned(),
                        None => {
                            format!("Preset ({})", preset.ray_traced_ambient_occlusion().label())
                        }
                    };
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(capabilities.ray_queries, |ui| {
                            egui::ComboBox::from_id_salt("ray traced ambient occlusion")
                                .selected_text(label(settings.ray_traced_ambient_occlusion))
                                .show_ui(ui, |ui| {
                                    for choice in std::iter::once(None)
                                        .chain(RayTracedAmbientOcclusion::ALL.map(Some))
                                    {
                                        ui.selectable_value(
                                            &mut settings.ray_traced_ambient_occlusion,
                                            choice,
                                            label(choice),
                                        );
                                    }
                                })
                                .response
                                .on_hover_text(
                                    "Corners and creases darkened by short rays, including \
                                     off-screen occluders. Low/Medium/High trace 2/4/8 rays/px. \
                                     Needs Hardware ray tracing on.",
                                );
                        });
                        if !capabilities.ray_queries {
                            ui.label(
                                egui::RichText::new("Needs Hardware RT")
                                    .size(type_scale::CAPTION)
                                    .color(palette.text_weak),
                            );
                        }
                    });
                    ui.end_row();

                    found_label(ui, "Ray traced reflections");
                    let label = |rays: Option<RayTracedReflections>| match rays {
                        Some(rays) => rays.label().to_owned(),
                        None => format!("Preset ({})", preset.ray_traced_reflections().label()),
                    };
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(capabilities.ray_queries, |ui| {
                            egui::ComboBox::from_id_salt("ray traced reflections")
                                .selected_text(label(settings.ray_traced_reflections))
                                .show_ui(ui, |ui| {
                                    for choice in std::iter::once(None)
                                        .chain(RayTracedReflections::ALL.map(Some))
                                    {
                                        ui.selectable_value(
                                            &mut settings.ray_traced_reflections,
                                            choice,
                                            label(choice),
                                        );
                                    }
                                })
                                .response
                                .on_hover_text(
                                    "Glossy surfaces mirror off-screen geometry with a ray per \
                                     pixel. Low traces roughness < 0.15 at half size; Medium < \
                                     0.3 and High < 0.5 at full size. Needs Hardware ray tracing \
                                     on.",
                                );
                        });
                        if !capabilities.ray_queries {
                            ui.label(
                                egui::RichText::new("Needs Hardware RT")
                                    .size(type_scale::CAPTION)
                                    .color(palette.text_weak),
                            );
                        }
                    });
                    ui.end_row();

                    found_label(ui, "Ray traced GI");
                    let label = |gi: Option<RayTracedGlobalIllumination>| match gi {
                        Some(gi) => gi.label().to_owned(),
                        None => format!(
                            "Preset ({})",
                            preset.ray_traced_global_illumination().label()
                        ),
                    };
                    ui.horizontal(|ui| {
                        // Without ray queries its rays are traced in software (RL3).
                        ui.add_enabled_ui(
                            capabilities.ray_queries || capabilities.compute_shaders,
                            |ui| {
                                egui::ComboBox::from_id_salt("ray traced global illumination")
                                    .selected_text(label(settings.ray_traced_global_illumination))
                                    .show_ui(ui, |ui| {
                                        for choice in std::iter::once(None)
                                            .chain(RayTracedGlobalIllumination::ALL.map(Some))
                                        {
                                            ui.selectable_value(
                                                &mut settings.ray_traced_global_illumination,
                                                choice,
                                                label(choice),
                                            );
                                        }
                                    })
                                    .response
                                    .on_hover_text(
                                        "Dynamic bounced light via ray-traced light probes around \
                                         the camera and in LightProbeVolumes. Uses hardware ray \
                                         tracing when active, or software BVH tracing on any GPU \
                                         with compute shaders.",
                                    );
                            },
                        );
                        let gi_mode = if capabilities.ray_queries {
                            "Hardware RT"
                        } else if capabilities.compute_shaders {
                            "Software fallback"
                        } else {
                            "Needs compute shaders"
                        };
                        ui.label(
                            egui::RichText::new(gi_mode)
                                .size(type_scale::CAPTION)
                                .color(palette.text_weak),
                        );
                    });
                    ui.end_row();
                });
        });
    restart
}

/// Whether the GPU must be opened again for Hardware ray tracing to be as
/// set: `wanted` but not `opened` on a GPU that `can`, or opened but no
/// longer wanted.
pub(super) fn needs_restart(opened: bool, can: bool, wanted: bool) -> bool {
    if wanted { !opened && can } else { opened }
}

/// What the Ray tracing section says about this GPU: whether the device
/// was `opened` with ray tracing, whether its adapter `can` ray trace, and
/// whether it's set to be opened with it (`wanted`) at the next start.
pub(super) fn ray_tracing_status(opened: bool, can: bool, wanted: bool) -> &'static str {
    match (opened, can, wanted) {
        (true, _, true) => {
            "Hardware ray tracing is on. Each ray traced effect is off until turned on; \
             View Mode > Ray Hits shows what rays hit."
        }
        (true, _, false) => "Hardware ray tracing turns off when the editor restarts.",
        (false, true, true) => "Hardware ray tracing turns on when the editor restarts.",
        (false, true, false) => {
            "This GPU can ray trace. Turn on Hardware ray tracing and restart the editor to \
             use it; until then the GPU is opened without it."
        }
        (false, false, _) => {
            "This GPU can't ray trace (it needs DirectX 12 ray tracing tier 1.1, Vulkan ray \
             queries or Metal on macOS 15). Everything is drawn without it."
        }
    }
}

/// The "In use" line for ray tracing: off, or each ray traced effect on.
pub(super) fn ray_tracing_in_use(effective: &EffectiveGraphics) -> String {
    if !effective.ray_queries {
        let off = "Ray tracing: off (Settings > Graphics > Hardware ray tracing) or not available";
        // GI's rays are then traced in software (RL3).
        if effective.ray_traced_gi_in_software {
            return format!(
                "{off}; GI {} traced in software ({} probes a frame, {} rays each)",
                effective.ray_traced_global_illumination.label(),
                effective.ray_traced_global_illumination.probes_per_frame(),
                effective.ray_traced_global_illumination.rays()
            );
        }
        return off.to_owned();
    }
    let mut effects = Vec::new();
    if effective.ray_traced_shadows != RayTracedShadows::Off {
        effects.push(format!(
            "shadows {} ({} rays a pixel where soft)",
            effective.ray_traced_shadows.label(),
            effective.ray_traced_shadows.rays()
        ));
    }
    if effective.ray_traced_ambient_occlusion != RayTracedAmbientOcclusion::Off {
        effects.push(format!(
            "ambient occlusion {} ({} rays a pixel)",
            effective.ray_traced_ambient_occlusion.label(),
            effective.ray_traced_ambient_occlusion.rays()
        ));
    }
    if effective.ray_traced_global_illumination != RayTracedGlobalIllumination::Off {
        effects.push(format!(
            "GI {} ({} probes a frame, {} rays each)",
            effective.ray_traced_global_illumination.label(),
            effective.ray_traced_global_illumination.probes_per_frame(),
            effective.ray_traced_global_illumination.rays()
        ));
    }
    if effective.ray_traced_reflections != RayTracedReflections::Off {
        effects.push(format!(
            "reflections {} (smoother than {} roughness{})",
            effective.ray_traced_reflections.label(),
            effective.ray_traced_reflections.roughness_cutoff(),
            if effective.ray_traced_reflections.downscale() > 1 {
                ", half size"
            } else {
                ""
            }
        ));
    }
    let reach = effective.ray_tracing_distance.round();
    if effects.is_empty() {
        format!("Ray tracing: available, up to {reach} studs (no ray traced effects on)")
    } else {
        format!("Ray tracing up to {reach} studs: {}", effects.join(", "))
    }
}

/// What the Texture memory row says while no budget is set, and why: the
/// budget the card's memory gives where that is more than the preset's,
/// otherwise the preset's.
pub(super) fn automatic_texture_budget(
    preset: &GraphicsSettings,
    capabilities: &Capabilities,
) -> (String, String) {
    let own = preset.texture_budget();
    let automatic = preset.texture_budget_on(capabilities);
    match capabilities.video_memory {
        Some(memory) if automatic > own => (
            format!("Automatic ({automatic} MB)"),
            format!(
                "A quarter of this graphics card's {} GB. The preset alone gives {own} MB.",
                axcenity_render::whole_gib(memory)
            ),
        ),
        Some(memory) => (
            format!("Preset ({own} MB)"),
            format!(
                "A quarter of this graphics card's {} GB is no more than the preset's budget.",
                axcenity_render::whole_gib(memory)
            ),
        ),
        None => (
            format!("Preset ({own} MB)"),
            "This graphics card doesn't say how much memory it has, so the preset's budget \
             is used. Set one by hand if the card has room."
                .to_owned(),
        ),
    }
}
