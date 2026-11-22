//! Settings > This project > Branding (Brand Kit, cohesion pass CW8.1):
//! the game's icon and banner, with a live preview of how they look in a
//! game window, taskbar and loading screen, drag-and-drop from the OS file
//! explorer, a computer file picker, and a project asset picker. Saved in
//! the manifest (`project::branding`); the player and Build Game use them.

use std::collections::HashMap;
use std::path::Path;

use axcenity_project::branding::{self, Branding, Picture};

use super::*;

/// The previews shown, by picture, each with the path it was made for, plus
/// the last drawn drop-zone rects so OS file drops land in the right slot.
#[derive(Default)]
pub(crate) struct Previews {
    shown: HashMap<&'static str, Preview>,
    pub(crate) icon_drop_rect: Option<egui::Rect>,
    pub(crate) banner_drop_rect: Option<egui::Rect>,
    pub(crate) section_rect: Option<egui::Rect>,
}

struct Preview {
    path: String,
    /// The file's time when it was read, so a changed file is read again.
    modified: Option<std::time::SystemTime>,
    texture: Option<egui::TextureHandle>,
    /// Image dimensions and format read from its header.
    info: Option<branding::ImageInfo>,
    /// What's wrong with the picture, or `None` when it's good.
    problem: Option<String>,
}

impl Previews {
    /// What's wrong with the picture last previewed, if it was.
    #[cfg(test)]
    pub(crate) fn problem(&self, picture: Picture) -> Option<&str> {
        self.shown.get(picture.name())?.problem.as_deref()
    }

    /// The preview of `picture` at `path` in `folder`, made the first time.
    fn get(
        &mut self,
        ctx: &egui::Context,
        folder: &Path,
        picture: Picture,
        path: &str,
    ) -> &Preview {
        let full = folder.join(path);
        let modified = std::fs::metadata(&full)
            .and_then(|metadata| metadata.modified())
            .ok();
        let stale = self.shown.get(picture.name()).is_none_or(|preview| {
            let missing_in_ctx = preview
                .texture
                .as_ref()
                .is_some_and(|tex| ctx.tex_manager().read().meta(tex.id()).is_none());
            preview.path != path || preview.modified != modified || missing_in_ctx
        });
        if stale {
            let problem = branding::check_picture(folder, picture, path);
            let info = std::fs::read(&full)
                .ok()
                .and_then(|bytes| branding::image_info(&bytes).ok());
            let texture = image::open(&full).ok().map(|image| {
                let image = image.thumbnail(512, 512).to_rgba8();
                let size = [image.width() as usize, image.height() as usize];
                ctx.load_texture(
                    format!("branding {}", picture.name()),
                    egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                    egui::TextureOptions::LINEAR,
                )
            });
            self.shown.insert(
                picture.name(),
                Preview {
                    path: path.to_owned(),
                    modified,
                    texture,
                    info,
                    problem,
                },
            );
        }
        &self.shown[picture.name()]
    }

    fn texture_of(&self, picture: Picture, path: Option<&str>) -> Option<egui::TextureId> {
        let path = path?;
        let preview = self.shown.get(picture.name())?;
        (preview.path == path)
            .then(|| preview.texture.as_ref().map(|t| t.id()))
            .flatten()
    }
}

/// The project's PNG and JPEG files, by path.
fn image_files(folder: &Path) -> Vec<String> {
    crate::asset_browser::scan(folder)
        .into_iter()
        .filter(|entry| is_image_path(&entry.path))
        .map(|entry| entry.path)
        .collect()
}

fn is_image_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    [".png", ".jpg", ".jpeg"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

/// Ensures `source` lives inside `project_folder` (copying external files into
/// `assets/branding/<filename>`) and returns its normalized project-relative path.
pub(crate) fn import_branding_file(project_folder: &Path, source: &Path) -> Result<String, String> {
    let file_name = source
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Invalid file name.".to_owned())?;
    if !is_image_path(file_name) {
        return Err("Only PNG and JPEG images are supported for branding.".to_owned());
    }
    let canon_project =
        std::fs::canonicalize(project_folder).unwrap_or_else(|_| project_folder.to_path_buf());
    let canon_source = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    if let Ok(rel) = canon_source.strip_prefix(&canon_project) {
        return Ok(rel.to_string_lossy().replace('\\', "/"));
    }
    let bytes = std::fs::read(source).map_err(|err| format!("Couldn't read image: {err}"))?;
    branding::image_info(&bytes).map_err(|why| format!("Image is {why}."))?;
    let branding_dir = project_folder.join("assets").join("branding");
    std::fs::create_dir_all(&branding_dir)
        .map_err(|err| format!("Couldn't create assets/branding: {err}"))?;
    let dest = branding_dir.join(file_name);
    std::fs::write(&dest, bytes)
        .map_err(|err| format!("Couldn't copy image into project: {err}"))?;
    Ok(format!("assets/branding/{file_name}"))
}

/// Guesses whether an image dropped on the general Brand Kit area is better
/// suited as an Icon (square PNG) or Banner (wide image).
fn guess_picture_slot(path: &Path) -> Picture {
    if let Ok(bytes) = std::fs::read(path)
        && let Ok(info) = branding::image_info(&bytes)
    {
        if info.width > info.height.saturating_mul(3) / 2 {
            return Picture::Banner;
        }
        return Picture::Icon;
    }
    Picture::Icon
}

impl EditorApp {
    /// Checks whether files dropped onto the OS window landed on the Brand Kit
    /// in Settings ▸ This project. Returns `true` if handled so generic texture
    /// import does not also import them.
    pub(in crate::app) fn try_drop_branding(
        &mut self,
        ctx: &egui::Context,
        files: &[PathBuf],
    ) -> bool {
        if !self.settings.open || self.settings.page != settings::SettingsPage::Project {
            return false;
        }
        let Some(project) = &self.project else {
            return false;
        };
        if self.playing() {
            return false;
        }
        let pointer = ctx.input(|input| {
            input
                .pointer
                .interact_pos()
                .or_else(|| input.pointer.hover_pos())
                .or_else(|| input.pointer.latest_pos())
        });
        let target_slot = match pointer {
            Some(pos)
                if self
                    .branding_previews
                    .icon_drop_rect
                    .is_some_and(|r| r.contains(pos)) =>
            {
                Some(Some(Picture::Icon))
            }
            Some(pos)
                if self
                    .branding_previews
                    .banner_drop_rect
                    .is_some_and(|r| r.contains(pos)) =>
            {
                Some(Some(Picture::Banner))
            }
            Some(pos)
                if self
                    .branding_previews
                    .section_rect
                    .is_some_and(|r| r.contains(pos)) =>
            {
                Some(None)
            }
            _ => None,
        };
        let Some(explicit_slot) = target_slot else {
            return false;
        };
        let image_paths: Vec<&PathBuf> = files
            .iter()
            .filter(|path| path.to_str().is_some_and(is_image_path))
            .collect();
        if image_paths.is_empty() {
            return false;
        }
        let folder = project.folder.clone();
        let mut branding = project.branding.clone();
        let mut updated = Vec::new();
        for path in image_paths {
            let slot = explicit_slot.unwrap_or_else(|| guess_picture_slot(path));
            match import_branding_file(&folder, path) {
                Ok(rel) => {
                    match slot {
                        Picture::Icon => branding.icon = Some(rel),
                        Picture::Banner => branding.banner = Some(rel),
                    }
                    updated.push(slot.name());
                }
                Err(err) => self.output.error(format!("Branding drop failed: {err}")),
            }
        }
        if !updated.is_empty() {
            self.set_branding(branding);
            self.toasts
                .success(format!("Updated branding {}", updated.join(" & ")));
        }
        true
    }

    /// The Branding (Brand Kit) section of Settings > This project, under its heading.
    pub(super) fn branding_settings(&mut self, ui: &mut egui::Ui) {
        let Some(project) = &self.project else {
            return;
        };
        let folder = project.folder.clone();
        let project_name = project.name.clone();
        let mut branding = project.branding.clone();
        let palette = theme::palette(ui.ctx());
        let editing = !self.playing();

        // Warm up preview textures before drawing the combined live showcase.
        if let Some(icon_path) = branding.icon.clone() {
            self.branding_previews
                .get(ui.ctx(), &folder, Picture::Icon, &icon_path);
        }
        if let Some(banner_path) = branding.banner.clone() {
            self.branding_previews
                .get(ui.ctx(), &folder, Picture::Banner, &banner_path);
        }

        let dragging_files = ui.ctx().input(|input| !input.raw.hovered_files.is_empty());
        let pointer = ui.ctx().input(|input| input.pointer.hover_pos());

        let section_response = ui.vertical(|ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Your game's icon (window, taskbar and executable) and banner (menus and \
                         loading screens). Drag and drop images from your file explorer, browse \
                         your computer, or pick from this project. Scripts and ImageLabels reach \
                         them as branding://icon and branding://banner.",
                    )
                    .size(theme::type_scale::SMALL)
                    .color(palette.text_secondary),
                )
                .wrap(),
            );
            ui.add_space(theme::space::S);

            // Live Brand Kit Preview Showcase.
            let icon_tex = self
                .branding_previews
                .texture_of(Picture::Icon, branding.icon.as_deref());
            let banner_tex = self
                .branding_previews
                .texture_of(Picture::Banner, branding.banner.as_deref());
            let fallback_mark = self.brand.as_ref().map(|b| b.mark().id());
            draw_brand_preview(
                ui,
                &palette,
                &project_name,
                icon_tex,
                banner_tex,
                fallback_mark,
            );
            ui.add_space(theme::space::M);

            let mut changed = false;
            let mut icon_rect = None;
            let mut banner_rect = None;
            for picture in [Picture::Icon, Picture::Banner] {
                let slot = match picture {
                    Picture::Icon => &mut branding.icon,
                    Picture::Banner => &mut branding.banner,
                };
                let (title, icon_glyph) = match picture {
                    Picture::Icon => ("App Icon", icons::APP_WINDOW),
                    Picture::Banner => ("Banner", icons::IMAGE),
                };

                let prev_rect = match picture {
                    Picture::Icon => self.branding_previews.icon_drop_rect,
                    Picture::Banner => self.branding_previews.banner_drop_rect,
                };
                let drop_hovered = dragging_files
                    && pointer.is_some_and(|p| prev_rect.is_some_and(|r| r.contains(p)));
                let border_color = if drop_hovered {
                    palette.accent
                } else if dragging_files {
                    palette.border_strong
                } else {
                    palette.border
                };
                let fill_color = if drop_hovered {
                    palette.accent_soft
                } else {
                    palette.surface
                };

                let card = egui::Frame::new()
                    .fill(fill_color)
                    .stroke(egui::Stroke::new(
                        if drop_hovered { 2.0_f32 } else { 1.0_f32 },
                        border_color,
                    ))
                    .corner_radius(theme::radius::CARD)
                    .inner_margin(theme::space::margin(theme::space::M, theme::space::S))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        // Header row: title + status badge.
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(theme::with_icon(icon_glyph, title))
                                    .family(theme::medium_family())
                                    .color(palette.text_strong),
                            );
                            if let Some(path) = slot.clone() {
                                let preview =
                                    self.branding_previews
                                        .get(ui.ctx(), &folder, picture, &path);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let (badge_text, badge_color) =
                                            match (&preview.problem, preview.info) {
                                                (None, Some(info)) => {
                                                    let fmt = if info.png { "PNG" } else { "JPEG" };
                                                    (
                                                        format!(
                                                            "{}×{} {fmt} · Good",
                                                            info.width, info.height
                                                        ),
                                                        palette.success,
                                                    )
                                                }
                                                (None, None) => {
                                                    ("Ready".to_owned(), palette.success)
                                                }
                                                (Some(_), Some(info)) => {
                                                    let fmt = if info.png { "PNG" } else { "JPEG" };
                                                    (
                                                        format!(
                                                            "{}×{} {fmt} · Check size",
                                                            info.width, info.height
                                                        ),
                                                        palette.warning,
                                                    )
                                                }
                                                (Some(_), None) => {
                                                    ("Invalid".to_owned(), palette.warning)
                                                }
                                            };
                                        ui.label(
                                            egui::RichText::new(badge_text)
                                                .size(theme::type_scale::CAPTION)
                                                .color(badge_color),
                                        );
                                    },
                                );
                            } else {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new("Using Axcenity default")
                                                .size(theme::type_scale::CAPTION)
                                                .color(palette.text_weak),
                                        );
                                    },
                                );
                            }
                        });
                        ui.add_space(theme::space::XS);

                        // Picker row: Choose from project + Browse computer + Clear.
                        ui.add_enabled_ui(editing, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                egui::ComboBox::from_id_salt(("branding", picture.name()))
                                    .selected_text(
                                        slot.as_deref().unwrap_or("Choose from project…"),
                                    )
                                    .width(210.0)
                                    .show_ui(ui, |ui| {
                                        let files = image_files(&folder);
                                        if files.is_empty() {
                                            ui.label(
                                                egui::RichText::new(
                                                    "No PNG or JPEG images in project yet.",
                                                )
                                                .size(theme::type_scale::SMALL)
                                                .color(palette.text_weak),
                                            );
                                        }
                                        for path in files {
                                            let picked = slot.as_deref() == Some(path.as_str());
                                            if ui.selectable_label(picked, &path).clicked()
                                                && !picked
                                            {
                                                *slot = Some(path);
                                                changed = true;
                                            }
                                        }
                                    });
                                if ui
                                    .add(widgets::button(
                                        widgets::Role::Secondary,
                                        Some(icons::FOLDER_OPEN),
                                        "Browse…",
                                        &palette,
                                    ))
                                    .on_hover_text(
                                        "Pick a PNG or JPEG from your computer (copied into \
                                         assets/branding/).",
                                    )
                                    .clicked()
                                    && let Some(file) = rfd::FileDialog::new()
                                        .set_title(format!("Choose {title}"))
                                        .add_filter("Images", &["png", "jpg", "jpeg"])
                                        .pick_file()
                                {
                                    match import_branding_file(&folder, &file) {
                                        Ok(rel) => {
                                            *slot = Some(rel);
                                            changed = true;
                                        }
                                        Err(err) => {
                                            self.output
                                                .error(format!("Branding import failed: {err}"));
                                        }
                                    }
                                }
                                if slot.is_some()
                                    && widgets::icon_button(ui, icons::X, "Clear", false).clicked()
                                {
                                    *slot = None;
                                    changed = true;
                                }
                            });
                        });

                        ui.add_space(theme::space::XS);
                        let drop_hint = if drop_hovered {
                            format!("Drop image here to set as {title}")
                        } else {
                            format!(
                                "Drop a file here from your file explorer, or use {}.",
                                picture.wanted()
                            )
                        };
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(drop_hint)
                                    .size(theme::type_scale::CAPTION)
                                    .color(if drop_hovered {
                                        palette.accent_text
                                    } else {
                                        palette.text_weak
                                    }),
                            )
                            .wrap(),
                        );

                        if let Some(path) = slot.clone() {
                            let preview =
                                self.branding_previews
                                    .get(ui.ctx(), &folder, picture, &path);
                            if let Some(problem) = &preview.problem {
                                ui.add_space(theme::space::XXS);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(format!(
                                            "{} {problem} Built games use Axcenity's until it's \
                                             fixed.",
                                            icons::WARNING
                                        ))
                                        .size(theme::type_scale::SMALL)
                                        .color(palette.warning),
                                    )
                                    .wrap(),
                                );
                            }
                        }
                    });

                match picture {
                    Picture::Icon => icon_rect = Some(card.response.rect),
                    Picture::Banner => banner_rect = Some(card.response.rect),
                }
                ui.add_space(theme::space::S);
            }

            if changed {
                self.set_branding(branding);
            }
            (icon_rect, banner_rect)
        });

        self.branding_previews.section_rect = Some(section_response.response.rect);
        self.branding_previews.icon_drop_rect = section_response.inner.0;
        self.branding_previews.banner_drop_rect = section_response.inner.1;
    }

    /// Keeps `branding` in the manifest and in use.
    pub(crate) fn set_branding(&mut self, branding: Branding) {
        let Some(project) = &mut self.project else {
            return;
        };
        match branding::set_branding(&project.folder, &branding) {
            Ok(()) => {
                self.assets
                    .set_branding(branding.icon.as_deref(), branding.banner.as_deref());
                project.branding = branding;
            }
            Err(error) => self
                .output
                .error(format!("Couldn't save the project's branding: {error}")),
        }
    }
}

/// Paints a combined live preview of the game's Brand Kit: loading/menu banner
/// with overlaid icon & progress bar, plus window title bar and desktop tile.
fn draw_brand_preview(
    ui: &mut egui::Ui,
    palette: &theme::Palette,
    project_name: &str,
    icon_tex: Option<egui::TextureId>,
    banner_tex: Option<egui::TextureId>,
    fallback_mark: Option<egui::TextureId>,
) {
    let shown_icon = icon_tex.or(fallback_mark);
    egui::Frame::new()
        .fill(palette.inset)
        .stroke(egui::Stroke::new(1.0_f32, palette.border))
        .corner_radius(theme::radius::CARD)
        .inner_margin(theme::space::margin(theme::space::M, theme::space::S))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(widgets::micro("Live preview", palette.text_weak));
            ui.add_space(theme::space::XS);

            // 1. Mock Loading Screen / Menu Banner card.
            let width = ui.available_width();
            let banner_height = (width * 0.30).clamp(112.0, 150.0);
            let (banner_rect, _) =
                ui.allocate_exact_size(egui::vec2(width, banner_height), egui::Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(banner_rect, theme::radius::CONTROL, palette.app_bg);
            if let Some(tex) = banner_tex {
                painter.image(
                    tex,
                    banner_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                // Subtle gradient-like placeholder bands when no banner is set yet.
                painter.rect_filled(
                    banner_rect,
                    theme::radius::CONTROL,
                    theme::mix(palette.app_bg, palette.accent, 0.14),
                );
                painter.text(
                    banner_rect.center() - egui::vec2(0.0, 16.0),
                    egui::Align2::CENTER_CENTER,
                    "Banner preview (drop a wide PNG/JPEG below)",
                    egui::FontId::proportional(theme::type_scale::SMALL),
                    palette.text_weak,
                );
            }
            // Dark scrim at bottom of banner for legibility.
            let scrim = egui::Rect::from_min_max(
                egui::pos2(banner_rect.left(), banner_rect.bottom() - 52.0),
                banner_rect.max,
            );
            painter.rect_filled(
                scrim,
                egui::CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: theme::radius::CONTROL,
                    se: theme::radius::CONTROL,
                },
                egui::Color32::from_black_alpha(165),
            );
            // Icon + title + loading bar inside scrim.
            let icon_box = egui::Rect::from_min_size(
                egui::pos2(scrim.left() + 10.0, scrim.top() + 8.0),
                egui::vec2(36.0, 36.0),
            );
            painter.rect_filled(icon_box, 6, palette.surface);
            if let Some(tex) = shown_icon {
                let inset = if icon_tex.is_some() { 0.0 } else { 6.0 };
                painter.image(
                    tex,
                    icon_box.shrink(inset),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            painter.text(
                egui::pos2(icon_box.right() + 10.0, icon_box.top() + 10.0),
                egui::Align2::LEFT_CENTER,
                project_name,
                egui::FontId::new(theme::type_scale::BODY, theme::heading_family()),
                egui::Color32::WHITE,
            );
            let bar_bg = egui::Rect::from_min_size(
                egui::pos2(icon_box.right() + 10.0, icon_box.bottom() - 10.0),
                egui::vec2((scrim.width() - 72.0).max(80.0), 5.0),
            );
            painter.rect_filled(bar_bg, 2, egui::Color32::from_white_alpha(45));
            painter.rect_filled(
                bar_bg.with_max_x(bar_bg.left() + bar_bg.width() * 0.62),
                2,
                palette.accent,
            );
            painter.rect_stroke(
                banner_rect,
                theme::radius::CONTROL,
                egui::Stroke::new(1.0_f32, palette.border),
                egui::StrokeKind::Inside,
            );

            ui.add_space(theme::space::S);

            // 2. Window Title Bar + Desktop / Taskbar Icon previews side by side.
            ui.horizontal(|ui| {
                // Mini Window Title Bar preview.
                let titlebar_w = (ui.available_width() * 0.62).max(160.0);
                let (tb_rect, _) =
                    ui.allocate_exact_size(egui::vec2(titlebar_w, 34.0), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(tb_rect, theme::radius::CONTROL, palette.surface);
                p.rect_stroke(
                    tb_rect,
                    theme::radius::CONTROL,
                    egui::Stroke::new(1.0_f32, palette.border),
                    egui::StrokeKind::Inside,
                );
                let small_icon = egui::Rect::from_min_size(
                    egui::pos2(tb_rect.left() + 8.0, tb_rect.center().y - 9.0),
                    egui::vec2(18.0, 18.0),
                );
                if let Some(tex) = shown_icon {
                    p.image(
                        tex,
                        small_icon,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        if icon_tex.is_some() {
                            egui::Color32::WHITE
                        } else {
                            palette.text
                        },
                    );
                }
                p.text(
                    egui::pos2(small_icon.right() + 7.0, tb_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    project_name,
                    egui::FontId::proportional(theme::type_scale::SMALL),
                    palette.text_strong,
                );
                for (i, color) in [palette.text_muted, palette.text_muted, palette.error]
                    .into_iter()
                    .enumerate()
                {
                    p.circle_filled(
                        egui::pos2(tb_rect.right() - 36.0 + i as f32 * 12.0, tb_rect.center().y),
                        3.2,
                        color,
                    );
                }

                // Taskbar / Desktop Icon tile preview.
                let tile_w = ui.available_width().max(80.0);
                let (tile_rect, _) =
                    ui.allocate_exact_size(egui::vec2(tile_w, 34.0), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(tile_rect, theme::radius::CONTROL, palette.surface);
                p.rect_stroke(
                    tile_rect,
                    theme::radius::CONTROL,
                    egui::Stroke::new(1.0_f32, palette.border),
                    egui::StrokeKind::Inside,
                );
                let app_icon = egui::Rect::from_min_size(
                    egui::pos2(tile_rect.left() + 7.0, tile_rect.center().y - 11.0),
                    egui::vec2(22.0, 22.0),
                );
                if let Some(tex) = shown_icon {
                    p.image(
                        tex,
                        app_icon,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        if icon_tex.is_some() {
                            egui::Color32::WHITE
                        } else {
                            palette.text
                        },
                    );
                }
                p.text(
                    egui::pos2(app_icon.right() + 7.0, tile_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "App icon",
                    egui::FontId::proportional(theme::type_scale::CAPTION),
                    palette.text_secondary,
                );
            });
        });
}
