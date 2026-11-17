//! The Character Studio page's cards: a character or preset with its
//! picture and overview, and a pack with its characters' pictures.

use super::*;

/// What's on a character's or preset's card.
pub(super) struct CardView<'a> {
    pub id: egui::Id,
    pub name: &'a str,
    pub overview: &'a Overview,
    /// Its picture, once drawn.
    pub picture: Option<&'a egui::TextureHandle>,
    /// The picture is being drawn.
    pub pending: bool,
    /// The picture's tooltip.
    pub hover: String,
}

/// What a character's card asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CardButton {
    /// Edit (a saved character), or start from it (a preset). Clicking the
    /// picture does it too.
    Main,
    Delete,
}

/// What a pack's card asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PackButton {
    Edit,
    Export,
    AddToProject,
    Delete,
}

/// The picture: portrait, 4 by 5.
fn picture_height(width: f32) -> f32 {
    (width * 5.0 / 4.0).round()
}

/// A character's card, `width` wide: its picture (a placeholder with its
/// initials while it's drawn), name and overview, then Edit and Delete,
/// or for a `preset` "Start from this".
pub(super) fn character_card(
    ui: &mut egui::Ui,
    view: &CardView,
    width: f32,
    preset: bool,
) -> Option<CardButton> {
    let palette = theme::palette(ui.ctx());
    let mut picked = None;
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.spacing_mut().item_spacing.y = 2.0;
        let (picture, response) =
            ui.allocate_exact_size(vec2(width, picture_height(width)), Sense::click());
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, view.name));
        let hover = theme::ease(ui.ctx(), view.id.with("hover"), response.hovered());
        thumbnail_image(
            ui,
            picture,
            CornerRadius::same(CARD_RADIUS),
            view.name,
            view.picture,
            egui::Vec2::ZERO,
        );
        ui.painter().rect_stroke(
            picture,
            CARD_RADIUS,
            Stroke::new(
                1.0_f32,
                theme::mix(
                    palette.border,
                    theme::mix(palette.border_strong, palette.accent, 0.45),
                    hover,
                ),
            ),
            egui::StrokeKind::Inside,
        );
        widgets::spotlight(ui, picture, response.id, hover);
        if view.picture.is_none() && view.pending {
            let spinner =
                Rect::from_min_size(picture.right_top() + vec2(-26.0, 10.0), vec2(16.0, 16.0));
            ui.put(
                spinner,
                egui::Spinner::new().size(16.0).color(palette.text_weak),
            );
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                picture.expand(2.0),
                CARD_RADIUS + 2,
                Stroke::new(1.5_f32, palette.accent),
                egui::StrokeKind::Inside,
            );
        }
        let response = response
            .on_hover_text(&view.hover)
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            picked = Some(CardButton::Main);
        }

        ui.add_space(8.0);
        ui.add(
            egui::Label::new(
                RichText::new(view.name)
                    .family(theme::medium_family())
                    .color(palette.text_strong),
            )
            .truncate(),
        );
        for line in [&view.overview.body, &view.overview.look] {
            if !line.is_empty() {
                ui.add(
                    egui::Label::new(
                        RichText::new(line.as_str())
                            .size(type_scale::SMALL)
                            .color(palette.text_weak),
                    )
                    .truncate(),
                );
            }
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if preset {
                let start = widgets::button(
                    Role::Secondary,
                    Some(icons::PLUS),
                    "Start from this",
                    &palette,
                );
                if ui
                    .add(start)
                    .on_hover_text("A new character from this preset")
                    .clicked()
                {
                    picked = Some(CardButton::Main);
                }
            } else {
                let edit = widgets::button(
                    Role::Secondary,
                    Some(icons::PENCIL_SIMPLE),
                    "Edit",
                    &palette,
                );
                if ui
                    .add(edit)
                    .on_hover_text("Open it in Character Studio")
                    .clicked()
                {
                    picked = Some(CardButton::Main);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::icon_button(ui, icons::TRASH, "Delete…", false).clicked() {
                        picked = Some(CardButton::Delete);
                    }
                });
            }
        });
    });
    picked
}

/// A pack's card, `width` wide: its name, how many characters, their
/// pictures in a row, and Edit, Export, Add to Project and Delete.
/// `members` are its characters that are still saved: name and picture.
pub(super) fn pack_card(
    ui: &mut egui::Ui,
    pack: &Pack,
    members: &[(String, Option<egui::TextureHandle>)],
    width: f32,
) -> Option<PackButton> {
    let palette = theme::palette(ui.ctx());
    let mut picked = None;
    egui::Frame::new()
        .fill(theme::mix(palette.panel, palette.surface, 0.6))
        .stroke(Stroke::new(1.0_f32, palette.border))
        .corner_radius(CARD_RADIUS)
        .inner_margin(14.0)
        .show(ui, |ui| {
            let inner = width - 28.0;
            ui.set_width(inner);
            // The card's own column, whatever row it's in.
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(theme::with_icon(icons::PACKAGE, &pack.name))
                            .family(theme::medium_family())
                            .color(palette.text_strong),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::icon_button(ui, icons::TRASH, "Delete pack…", false).clicked()
                        {
                            picked = Some(PackButton::Delete);
                        }
                        let count = members.len();
                        ui.label(
                            RichText::new(format!(
                                "{count} character{}",
                                if count == 1 { "" } else { "s" }
                            ))
                            .size(type_scale::SMALL)
                            .color(palette.text_weak),
                        );
                    });
                });
                ui.add_space(8.0);
                // The characters' pictures, as many as fit, then "+n".
                let size = vec2(52.0, 65.0);
                let gap = 6.0;
                let fit = (((inner + gap) / (size.x + gap)).floor() as usize).max(1);
                let shown = if members.len() > fit {
                    fit - 1
                } else {
                    members.len()
                };
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (name, picture) in &members[..shown] {
                        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
                        small_picture(ui, rect, name, picture.as_ref());
                        response.on_hover_text(name.as_str());
                    }
                    if members.len() > shown {
                        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                        ui.painter().rect_filled(rect, 5, palette.surface);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("+{}", members.len() - shown),
                            FontId::proportional(type_scale::BODY),
                            palette.text_secondary,
                        );
                    }
                    if members.is_empty() {
                        ui.label(
                            RichText::new("Its characters were deleted.").color(palette.text_weak),
                        );
                    }
                });
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    let button = |ui: &mut egui::Ui, icon, label, tip| {
                        ui.add(widgets::button(
                            Role::Secondary,
                            Some(icon),
                            label,
                            &palette,
                        ))
                        .on_hover_text(tip)
                        .clicked()
                    };
                    if button(
                        ui,
                        icons::PENCIL_SIMPLE,
                        "Edit…",
                        "Rename it or change its characters",
                    ) {
                        picked = Some(PackButton::Edit);
                    }
                    if button(
                        ui,
                        icons::EXPORT,
                        "Export…",
                        "Save it as one file, to move to another computer",
                    ) {
                        picked = Some(PackButton::Export);
                    }
                    if button(
                        ui,
                        icons::FOLDER_PLUS,
                        "Add to Project…",
                        "Copy its characters into a game project",
                    ) {
                        picked = Some(PackButton::AddToProject);
                    }
                });
            });
        });
    picked
}

/// A small picture of a character (in a pack's row or the pack editor),
/// closer in than the card's.
pub(super) fn small_picture(
    ui: &egui::Ui,
    rect: Rect,
    name: &str,
    picture: Option<&egui::TextureHandle>,
) {
    let palette = theme::palette(ui.ctx());
    match picture {
        Some(texture) => egui::Image::new(texture)
            .corner_radius(5)
            .uv(Rect::from_min_max(pos2(0.125, 0.03), pos2(0.875, 0.78)))
            .paint_at(ui, rect),
        None => thumbnail_image(
            ui,
            rect,
            CornerRadius::same(5),
            name,
            None,
            egui::Vec2::ZERO,
        ),
    }
    ui.painter().rect_stroke(
        rect,
        5,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );
}
