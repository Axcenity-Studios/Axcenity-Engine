//! The Appearance, Typography and Accessibility pages, and the small theme
//! preview they share.

use super::*;

use crate::theme::{CodeFont, CustomFont, UiFont};

pub(super) fn appearance_page(ui: &mut egui::Ui, appearance: &mut Appearance) {
    page_title(
        ui,
        "Appearance",
        "The theme, the accent colour and how big everything is.",
    );
    let palette = theme::palette(ui.ctx());
    section(ui, "Theme");
    ui.add(
        egui::Label::new(
            egui::RichText::new(
                "Pick a dark or light preset to stylize every panel, menu, toolbar and graph.",
            )
            .size(type_scale::SMALL)
            .color(palette.text_secondary),
        )
        .wrap(),
    );
    ui.add_space(space::XS);

    let avail = ui.available_width();
    let cols = if avail >= 420.0 { 3 } else { 2 };
    let gap = 10.0;
    let card_w = ((avail - gap * (cols - 1) as f32 - 2.0) / cols as f32)
        .floor()
        .clamp(112.0, 162.0);
    let mut picked_theme = None;

    // System mode row.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        if theme_preview(
            ui,
            ThemeChoice::System,
            appearance.accent,
            appearance.theme == ThemeChoice::System,
            card_w,
        )
        .clicked()
        {
            picked_theme = Some(ThemeChoice::System);
        }
        let desc_w = (avail - card_w - 14.0).max(120.0);
        ui.vertical(|ui| {
            ui.set_max_width(desc_w);
            ui.add_space(space::S);
            ui.label(
                egui::RichText::new("Match operating system")
                    .family(theme::medium_family())
                    .color(palette.text_strong),
            );
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Automatically switches between neutral Dark and Light when your OS \
                         appearance changes.",
                    )
                    .size(type_scale::SMALL)
                    .color(palette.text_secondary),
                )
                .wrap(),
            );
        });
    });

    let mut theme_grid = |ui: &mut egui::Ui, heading: &str, presets: &[ThemeChoice]| {
        ui.add_space(space::M);
        ui.label(widgets::micro(heading, palette.text_weak));
        ui.add_space(space::XS);
        for (row_idx, row) in presets.chunks(cols).enumerate() {
            if row_idx > 0 {
                ui.add_space(gap);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &choice in row {
                    let preview_accent = choice.suggested_accent().unwrap_or(appearance.accent);
                    let card_accent = if appearance.theme == choice {
                        appearance.accent
                    } else {
                        preview_accent
                    };
                    if theme_preview(ui, choice, card_accent, appearance.theme == choice, card_w)
                        .clicked()
                    {
                        picked_theme = Some(choice);
                    }
                }
            });
        }
    };
    theme_grid(ui, "Dark themes", &ThemeChoice::DARK_PRESETS);
    theme_grid(ui, "Light themes", &ThemeChoice::LIGHT_PRESETS);

    if let Some(choice) = picked_theme
        && appearance.theme != choice
    {
        appearance.theme = choice;
        if let Some(suggested) = choice.suggested_accent() {
            appearance.accent = suggested;
        }
    }

    ui.add_space(space::L);
    section(ui, "Accent");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
        for accent in Accent::ALL {
            if accent_swatch(ui, accent, appearance.accent == accent).clicked() {
                appearance.accent = accent;
            }
        }
        ui.add_space(space::XS);
        let palette = theme::palette(ui.ctx());
        ui.label(egui::RichText::new(appearance.accent.label()).color(palette.text_secondary));
    });
    ui.add_space(space::L);
    section(ui, "Interface scale");
    let options: Vec<(f32, Option<&str>, String)> = Appearance::UI_SCALES
        .iter()
        .map(|&scale| (scale, None, format!("{}%", (scale * 100.0).round())))
        .collect();
    let options: Vec<(f32, Option<&str>, &str)> = options
        .iter()
        .map(|(scale, icon, label)| (*scale, *icon, label.as_str()))
        .collect();
    widgets::segmented(ui, &mut appearance.ui_scale, &options);
    ui.add_space(space::L);
    section(ui, "Ribbon");
    widgets::segmented(
        ui,
        &mut appearance.ribbon,
        &RibbonAlign::ALL.map(|place| (place, None, place.label())),
    );
    let palette = theme::palette(ui.ctx());
    ui.label(
        egui::RichText::new(
            "Where the ribbon's buttons sit across the window: from the left edge, or in the \
             middle under the tabs.",
        )
        .color(palette.text_secondary),
    );
}

fn font_error_id() -> egui::Id {
    egui::Id::new("axcenity font import error")
}

/// Copies an imported font into `<storage_dir>/fonts/` (when available) so it
/// stays available even if the original file moves, and validates its header.
fn import_custom_font(source: &std::path::Path) -> Result<CustomFont, String> {
    let bytes = std::fs::read(source).map_err(|err| format!("Couldn't read font file: {err}"))?;
    theme::validate_font_bytes(&bytes).map_err(|err| err.to_owned())?;
    let name = source
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace(['-', '_'], " "))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Custom Font".to_owned());
    let file_name = source
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("custom.ttf");
    let final_path = if let Some(dir) = eframe::storage_dir(crate::APP_ID) {
        let fonts_dir = dir.join("fonts");
        if std::fs::create_dir_all(&fonts_dir).is_ok() {
            let dest = fonts_dir.join(file_name);
            if std::fs::write(&dest, &bytes).is_ok() {
                dest
            } else {
                source.to_path_buf()
            }
        } else {
            source.to_path_buf()
        }
    } else {
        source.to_path_buf()
    };
    Ok(CustomFont {
        name,
        path: final_path.to_string_lossy().into_owned(),
    })
}

pub(super) fn typography_page(ui: &mut egui::Ui, appearance: &mut Appearance) {
    page_title(
        ui,
        "Text & density",
        "How much room controls get, interface fonts, and the size of code.",
    );
    section(ui, "Density");
    widgets::segmented(
        ui,
        &mut appearance.density,
        &Density::ALL.map(|density| (density, None, density.label())),
    );
    let palette = theme::palette(ui.ctx());
    ui.label(
        egui::RichText::new(match appearance.density {
            Density::Comfortable => "Roomier rows and controls, easier to hit.",
            Density::Compact => "Tighter rows and controls, to see more at once.",
        })
        .color(palette.text_secondary),
    );
    ui.add_space(space::L);
    section(ui, "Code and Output");
    ui.add(
        egui::Label::new(
            egui::RichText::new(
                "Pick the typeface and size used in script editors, the Console log, and code \
                 snippets, or import your own .ttf / .otf font.",
            )
            .size(type_scale::SMALL)
            .color(palette.text_secondary),
        )
        .wrap(),
    );
    ui.add_space(space::XS);
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("code font selector")
            .selected_text(format!(
                "{} · {}",
                appearance.code_font.label(),
                appearance.code_font.kind()
            ))
            .width(240.0)
            .show_ui(ui, |ui| {
                ui.label(widgets::micro("Built-in", palette.text_weak));
                for font in CodeFont::BUILT_IN {
                    let label = format!("{}  ({})", font.label(), font.kind());
                    ui.selectable_value(&mut appearance.code_font, font, label);
                }
                if !appearance.custom_fonts.is_empty() {
                    ui.separator();
                    ui.label(widgets::micro("Imported", palette.text_weak));
                    for custom in &appearance.custom_fonts {
                        let choice = CodeFont::Custom {
                            name: custom.name.clone(),
                            path: custom.path.clone(),
                        };
                        let label = format!("{}  (Imported)", custom.name);
                        ui.selectable_value(&mut appearance.code_font, choice, label);
                    }
                }
            });

        if ui
            .add(widgets::button(
                widgets::Role::Secondary,
                Some(icons::UPLOAD_SIMPLE),
                "Import font…",
                &palette,
            ))
            .on_hover_text("Import a .ttf or .otf font file to use for code and the Console.")
            .clicked()
            && let Some(file) = rfd::FileDialog::new()
                .set_title("Import code font")
                .add_filter("Font files", &["ttf", "otf", "ttc"])
                .pick_file()
        {
            match import_custom_font(&file) {
                Ok(custom) => {
                    appearance
                        .custom_fonts
                        .retain(|existing| existing.path != custom.path);
                    appearance.code_font = CodeFont::Custom {
                        name: custom.name.clone(),
                        path: custom.path.clone(),
                    };
                    appearance.custom_fonts.push(custom);
                    ui.ctx()
                        .data_mut(|data| data.remove_temp::<String>(font_error_id()));
                }
                Err(error) => {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(font_error_id(), error));
                }
            }
        }

        if appearance.code_font != CodeFont::JetBrainsMono
            && ui
                .button("Reset")
                .on_hover_text("Switch back to JetBrains Mono (Default).")
                .clicked()
        {
            appearance.code_font = CodeFont::JetBrainsMono;
        }
    });
    ui.add_space(space::XS);
    ui.add(
        egui::Slider::new(&mut appearance.code_font_size, Appearance::CODE_FONT_SIZES)
            .step_by(0.5)
            .suffix(" pt"),
    );
    egui::Frame::new()
        .fill(palette.inset)
        .stroke(egui::Stroke::new(1.0_f32, palette.border))
        .corner_radius(theme::radius::CARD)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let font = egui::FontId::monospace(appearance.code_font_size);
            ui.label(
                egui::RichText::new("local part = workspace.Pad")
                    .font(font.clone())
                    .color(palette.text),
            );
            ui.label(
                egui::RichText::new("part.Touched:Connect(onTouched) -- 0O 1lI")
                    .font(font)
                    .color(palette.text_secondary),
            );
        });
    ui.add_space(space::L);
    section(ui, "Interface font");
    ui.add(
        egui::Label::new(
            egui::RichText::new(
                "Choose a built-in typeface for panels, menus and labels, or import your own .ttf \
                 or .otf font file. Icons always stay crisp with Phosphor.",
            )
            .size(type_scale::SMALL)
            .color(palette.text_secondary),
        )
        .wrap(),
    );
    ui.add_space(space::XS);
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("interface font selector")
            .selected_text(format!(
                "{} · {}",
                appearance.ui_font.label(),
                appearance.ui_font.kind()
            ))
            .width(240.0)
            .show_ui(ui, |ui| {
                ui.label(widgets::micro("Built-in", palette.text_weak));
                for font in UiFont::BUILT_IN {
                    let label = format!("{}  ({})", font.label(), font.kind());
                    ui.selectable_value(&mut appearance.ui_font, font, label);
                }
                if !appearance.custom_fonts.is_empty() {
                    ui.separator();
                    ui.label(widgets::micro("Imported", palette.text_weak));
                    for custom in &appearance.custom_fonts {
                        let choice = UiFont::Custom {
                            name: custom.name.clone(),
                            path: custom.path.clone(),
                        };
                        let label = format!("{}  (Imported)", custom.name);
                        ui.selectable_value(&mut appearance.ui_font, choice, label);
                    }
                }
            });

        if ui
            .add(widgets::button(
                widgets::Role::Secondary,
                Some(icons::UPLOAD_SIMPLE),
                "Import font…",
                &palette,
            ))
            .on_hover_text("Import a .ttf or .otf font file to use for the editor interface.")
            .clicked()
            && let Some(file) = rfd::FileDialog::new()
                .set_title("Import interface font")
                .add_filter("Font files", &["ttf", "otf", "ttc"])
                .pick_file()
        {
            match import_custom_font(&file) {
                Ok(custom) => {
                    appearance
                        .custom_fonts
                        .retain(|existing| existing.path != custom.path);
                    appearance.ui_font = UiFont::Custom {
                        name: custom.name.clone(),
                        path: custom.path.clone(),
                    };
                    appearance.custom_fonts.push(custom);
                    ui.ctx()
                        .data_mut(|data| data.remove_temp::<String>(font_error_id()));
                }
                Err(error) => {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(font_error_id(), error));
                }
            }
        }

        if appearance.ui_font != UiFont::Inter
            && ui
                .button("Reset")
                .on_hover_text("Switch back to Inter (Default).")
                .clicked()
        {
            appearance.ui_font = UiFont::Inter;
        }
    });

    if let Some(err) = ui
        .ctx()
        .data(|data| data.get_temp::<String>(font_error_id()))
    {
        ui.add_space(space::XXS);
        ui.colored_label(palette.error, theme::with_icon(icons::WARNING, &err));
    }

    if !appearance.custom_fonts.is_empty() {
        ui.add_space(space::XS);
        let mut remove_index = None;
        ui.horizontal_wrapped(|ui| {
            for (idx, custom) in appearance.custom_fonts.iter().enumerate() {
                let active = matches!(
                    &appearance.ui_font,
                    UiFont::Custom { path, .. } if path == &custom.path
                ) || matches!(
                    &appearance.code_font,
                    CodeFont::Custom { path, .. } if path == &custom.path
                );
                egui::Frame::new()
                    .fill(if active {
                        palette.accent_soft
                    } else {
                        palette.surface
                    })
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        if active {
                            palette.accent
                        } else {
                            palette.border
                        },
                    ))
                    .corner_radius(theme::radius::CONTROL)
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let pick = ui
                                .add(
                                    egui::Label::new(
                                        egui::RichText::new(&custom.name)
                                            .size(type_scale::SMALL)
                                            .color(if active {
                                                palette.accent_text
                                            } else {
                                                palette.text
                                            }),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            if pick.clicked() {
                                appearance.ui_font = UiFont::Custom {
                                    name: custom.name.clone(),
                                    path: custom.path.clone(),
                                };
                            }
                            if widgets::icon_button(ui, icons::X, "Remove imported font", false)
                                .clicked()
                            {
                                remove_index = Some(idx);
                            }
                        });
                    });
            }
        });
        if let Some(idx) = remove_index {
            let removed = appearance.custom_fonts.remove(idx);
            if matches!(
                &appearance.ui_font,
                UiFont::Custom { path, .. } if path == &removed.path
            ) {
                appearance.ui_font = UiFont::Inter;
            }
            if matches!(
                &appearance.code_font,
                CodeFont::Custom { path, .. } if path == &removed.path
            ) {
                appearance.code_font = CodeFont::JetBrainsMono;
            }
        }
    }

    ui.add_space(space::S);
    egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0_f32, palette.border))
        .corner_radius(theme::radius::CARD)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(format!("Preview — {}", appearance.ui_font.label()))
                    .family(theme::heading_family())
                    .size(type_scale::HEADING)
                    .color(palette.text_strong),
            );
            ui.label(
                egui::RichText::new(
                    "The quick brown fox jumps over 123 glowing parts in Workspace.",
                )
                .color(palette.text),
            );
            ui.label(
                egui::RichText::new("Secondary metadata · Position (0.00, 4.50, -12.00) · 60 FPS")
                    .size(type_scale::SMALL)
                    .color(palette.text_secondary),
            );
        });
}

pub(super) fn accessibility_page(ui: &mut egui::Ui, appearance: &mut Appearance) {
    page_title(
        ui,
        "Accessibility",
        "Motion, contrast, and other ways to make the editor easier to use.",
    );
    let palette = theme::palette(ui.ctx());
    section(ui, "Motion");
    let row = ui.checkbox(&mut appearance.reduce_motion, "Reduce motion");
    found(ui, &row, "Reduce motion");
    ui.label(
        egui::RichText::new(
            "Turns off hover fades, popup and window fades, smooth scrolling and the theme \
             crossfade. The viewport camera is never animated by the interface.",
        )
        .color(palette.text_secondary),
    );
    ui.add_space(space::L);
    section(ui, "Contrast");
    let row = ui.checkbox(&mut appearance.increase_contrast, "Increase contrast");
    found(ui, &row, "Increase contrast");
    ui.label(
        egui::RichText::new(
            "Stronger outlines around buttons and fields, stronger borders and faint text, a \
             thicker focus ring, and block colours dark enough for their white labels.",
        )
        .color(palette.text_secondary),
    );
    ui.add_space(space::L);
    section(ui, "Display");
    ui.horizontal(|ui| {
        found_label(ui, "Frame rate limit");
        egui::ComboBox::from_id_salt("frame rate limit")
            .selected_text(Appearance::steady_rate_name(appearance.steady_hz))
            .show_ui(ui, |ui| {
                for hz in Appearance::STEADY_RATES {
                    ui.selectable_value(
                        &mut appearance.steady_hz,
                        hz,
                        Appearance::steady_rate_name(hz),
                    );
                }
            });
    });
    ui.label(
        egui::RichText::new(
            "No limit shows each frame as soon as it's ready, up to your display's refresh, \
             and the viewport bar reads the real frame rate. With a limit, frames are shown at \
             that fixed rate while you work, so a FreeSync or G-Sync display holds one refresh \
             rate (OLED and VA monitors can flicker when it swings), and the viewport bar marks \
             the rate (held).",
        )
        .color(palette.text_secondary),
    );
    ui.add_space(space::S);
    let row = ui.checkbox(&mut appearance.hold_refresh, "Hold the refresh rate steady");
    found(ui, &row, "Hold the refresh rate steady");
    ui.label(
        egui::RichText::new(
            "Keeps drawing, at the frame rate limit if one is set, while the editor has focus \
             instead of stopping when it's idle, for displays that flicker as the rate drops. It uses \
             more of the graphics card.",
        )
        .color(palette.text_secondary),
    );
    ui.add_space(space::L);
    section(ui, "Readability");
    ui.label(
        egui::RichText::new(
            "Text and accent colours meet WCAG AA contrast in both themes. For larger text, \
             raise the interface scale in Appearance, or press Ctrl and + (Cmd and + on macOS).",
        )
        .color(palette.text_secondary),
    );
    ui.add_space(space::L);
    section(ui, "Keyboard");
    ui.label(
        egui::RichText::new(
            "Tab moves between controls and shows a focus ring in the accent colour; Space or \
             Enter presses the focused control.",
        )
        .color(palette.text_secondary),
    );
}

/// A small picture of the editor in `choice`'s colours, to pick a theme by.
/// System shows half dark, half light.
pub(super) fn theme_preview(
    ui: &mut egui::Ui,
    choice: ThemeChoice,
    accent: Accent,
    selected: bool,
    width: f32,
) -> egui::Response {
    let pic_h = (width * 0.46).round().clamp(56.0, 74.0);
    let size = egui::vec2(width, pic_h + 24.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            true,
            selected,
            choice.label(),
        )
    });
    let current = theme::palette(ui.ctx());
    let picture = egui::Rect::from_min_size(rect.min, egui::vec2(size.x, pic_h));
    match choice {
        ThemeChoice::System => {
            let half = picture.center().x;
            let left_clip = ui.clip_rect().intersect(picture.with_max_x(half));
            if left_clip.is_positive() {
                let mut left = ui.new_child(egui::UiBuilder::new().max_rect(picture));
                left.set_clip_rect(left_clip);
                mini_editor(
                    &left,
                    picture,
                    &Palette::for_theme(ThemeChoice::Dark, true, accent),
                );
            }
            let right_clip = ui.clip_rect().intersect(picture.with_min_x(half));
            if right_clip.is_positive() {
                let mut right = ui.new_child(egui::UiBuilder::new().max_rect(picture));
                right.set_clip_rect(right_clip);
                mini_editor(
                    &right,
                    picture,
                    &Palette::for_theme(ThemeChoice::Light, false, accent),
                );
            }
        }
        preset => {
            let dark = preset.is_dark().unwrap_or(true);
            mini_editor(ui, picture, &Palette::for_theme(preset, dark, accent));
        }
    }
    let t = theme::ease(
        ui.ctx(),
        response.id.with("ring"),
        selected || response.hovered(),
    );
    let ring = if selected {
        current.accent
    } else {
        theme::mix(current.border, current.border_strong, t)
    };
    ui.painter().rect_stroke(
        picture,
        6,
        egui::Stroke::new(if selected { 2.0_f32 } else { 1.0 }, ring),
        egui::StrokeKind::Inside,
    );
    let label_color = if selected {
        current.text_strong
    } else {
        current.text_secondary
    };
    let label_clip = ui.clip_rect().intersect(rect);
    if label_clip.is_positive() {
        ui.painter().with_clip_rect(label_clip).text(
            egui::pos2(rect.left() + 2.0, picture.bottom() + 13.0),
            egui::Align2::LEFT_CENTER,
            theme::with_icon(choice.icon(), choice.label()),
            egui::FontId::new(type_scale::SMALL, theme::medium_family()),
            label_color,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Paints a tiny editor: header, Explorer with a selected row, a viewport
/// and an accent button.
pub(super) fn mini_editor(ui: &egui::Ui, rect: egui::Rect, p: &Palette) {
    let painter = ui.painter();
    let sx = (rect.width() / 166.0).clamp(0.65, 1.15);
    let sy = (rect.height() / 76.0).clamp(0.65, 1.15);
    painter.rect_filled(rect, 6, p.app_bg);
    let header_h = (12.0 * sy).max(9.0);
    let header = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), header_h));
    painter.circle_filled(
        header.left_center() + egui::vec2(8.0 * sx, 0.0),
        (2.5 * sx).max(1.8),
        p.text_muted,
    );
    for (x, w) in [(16.0, 14.0), (34.0, 12.0), (50.0, 12.0)] {
        painter.rect_filled(
            egui::Rect::from_min_size(
                header.min + egui::vec2(x * sx, (header_h - 2.5) * 0.5),
                egui::vec2(w * sx, 2.5),
            ),
            1,
            p.text_weak,
        );
    }
    let pad = 4.0;
    let body = egui::Rect::from_min_max(
        header.left_bottom() + egui::vec2(pad, 0.0),
        rect.max - egui::vec2(pad, pad),
    );
    let explorer_w = (body.width() * 0.27).clamp(26.0, 44.0);
    let explorer = egui::Rect::from_min_size(body.min, egui::vec2(explorer_w, body.height()));
    painter.rect_filled(explorer, 4, p.panel);
    let row_step = ((explorer.height() - 10.0) / 5.0).max(7.0);
    for row in 0..5 {
        let y = explorer.top() + 5.0 + row as f32 * row_step;
        let indent = 4.0 + (row % 2) as f32 * (4.0 * sx);
        let line_w = (explorer.width() - indent - 5.0).max(8.0);
        let line = egui::Rect::from_min_size(
            egui::pos2(explorer.left() + indent, y),
            egui::vec2(line_w, 2.5),
        );
        if row == 2 {
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(explorer.left() + 2.0, y - 2.5),
                    egui::vec2((explorer.width() - 4.0).max(6.0), (row_step - 1.0).max(6.0)),
                ),
                2,
                p.accent_soft,
            );
            painter.rect_filled(line, 1, p.accent_text);
        } else {
            painter.rect_filled(line, 1, p.text_muted);
        }
    }
    let viewport =
        egui::Rect::from_min_max(egui::pos2(explorer.right() + 4.0, body.top()), body.max);
    painter.rect_filled(viewport, 4, theme::mix(p.app_bg, p.panel, 0.5));
    let ground_inset = (viewport.width() * 0.14).max(6.0);
    let ground = egui::Rect::from_min_max(
        egui::pos2(
            viewport.left() + ground_inset,
            viewport.center().y + 5.0 * sy,
        ),
        egui::pos2(
            viewport.right() - ground_inset,
            viewport.center().y + 12.0 * sy,
        ),
    );
    painter.rect_filled(ground, 2, p.border_strong);
    let cube = (13.0 * sy).clamp(9.0, 15.0);
    painter.rect_filled(
        egui::Rect::from_center_size(
            viewport.center() - egui::vec2(0.0, 2.0 * sy),
            egui::vec2(cube, cube),
        ),
        2,
        p.text_weak,
    );
    let btn_w = (20.0 * sx).clamp(14.0, 22.0);
    let btn_h = (7.5 * sy).clamp(5.5, 8.5);
    painter.rect_filled(
        egui::Rect::from_min_size(
            viewport.right_top() + egui::vec2(-(btn_w + 5.0), 5.0),
            egui::vec2(btn_w, btn_h),
        ),
        2,
        p.accent_solid,
    );
}

/// A round swatch of `accent`, ringed while it's picked.
pub(super) fn accent_swatch(ui: &mut egui::Ui, accent: Accent, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            true,
            selected,
            accent.label(),
        )
    });
    let current = theme::palette(ui.ctx());
    let color = Palette::new(current.dark, accent).accent;
    let center = rect.center();
    let grow = theme::ease(ui.ctx(), response.id.with("hover"), response.hovered());
    ui.painter().circle_filled(center, 9.0 + grow, color);
    if selected {
        ui.painter().circle_stroke(
            center,
            12.5,
            egui::Stroke::new(1.5_f32, current.text_strong),
        );
    }
    if response.has_focus() {
        ui.painter()
            .circle_stroke(center, 13.0, egui::Stroke::new(1.0_f32, current.accent));
    }
    response
        .on_hover_text(accent.label())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}
