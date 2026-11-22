use super::*;

/// What the page header's buttons asked for.
pub(super) enum HeaderAction {
    New,
    Open,
    Join,
}

/// The Projects page's title and a line about the engine, with Join
/// Session, Open Project and New Project at the right.
pub(super) fn page_header(ui: &mut egui::Ui, first_time: bool) -> Option<HeaderAction> {
    let title = if first_time {
        "Welcome to Axcenity"
    } else {
        "Projects"
    };
    // Joining is about people, not files: the quieter button, furthest
    // from New, named for what it joins.
    let buttons = [
        HeaderButton {
            role: Role::Ghost,
            icon: icons::SIGN_IN,
            label: "Join Session…",
            tip: "Join a teammate's live Collaborate session with the invite link \
                  they sent you, and build the same project together",
        },
        HeaderButton {
            role: Role::Secondary,
            icon: icons::FOLDER_OPEN,
            label: "Open Project…",
            tip: "Open a project folder",
        },
        HeaderButton {
            role: Role::Primary,
            icon: icons::PLUS,
            label: "New Project",
            tip: "Start a project from a template",
        },
    ];
    let subtitle = "Build worlds from parts, script them in Lua or blocks, and press Play.";
    match page_title(ui, title, subtitle, &buttons, |_| {}) {
        Some(0) => Some(HeaderAction::Join),
        Some(1) => Some(HeaderAction::Open),
        Some(_) => Some(HeaderAction::New),
        None => None,
    }
}

/// One of a page title's buttons.
pub(super) struct HeaderButton<'a> {
    pub role: Role,
    pub icon: &'a str,
    pub label: &'a str,
    pub tip: &'a str,
}

/// Below this many points for the title beside its buttons, the buttons go
/// under it.
const TITLE_ROOM: f32 = 300.0;
/// A page title's button height.
const HEADER_BUTTON_HEIGHT: f32 = 32.0;

/// Every start page's header, laid out the same way: the title, a line
/// under it and anything `below` adds (quiet links), with the page's main
/// `buttons` (left to right, the primary last) at the right of the title row.
/// In a narrow window the buttons go under the title. Returns the index of
/// the button clicked.
pub(super) fn page_title(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    buttons: &[HeaderButton],
    below: impl FnOnce(&mut egui::Ui),
) -> Option<usize> {
    let palette = theme::palette(ui.ctx());
    let gap = 8.0;
    let buttons_width = buttons
        .iter()
        .map(|button| button_width(ui, button.icon, button.label))
        .sum::<f32>()
        + gap * buttons.len().saturating_sub(1) as f32;
    let room = ui.available_width() - buttons_width - 24.0;
    // Returns the title's height, to centre the buttons on.
    let heading = |ui: &mut egui::Ui| {
        let title = ui.label(
            RichText::new(title)
                .family(theme::display_family())
                .size(type_scale::DISPLAY)
                .extra_letter_spacing(theme::DISPLAY_TRACKING)
                .color(palette.text_strong),
        );
        ui.add(egui::Label::new(RichText::new(subtitle).color(palette.text_secondary)).wrap());
        below(ui);
        title.rect.height()
    };
    let mut clicked = None;
    // Drawn in either direction: right to left from the page's edge, so the
    // row ends exactly there whatever the estimate above, or left to right
    // under the title.
    let mut row = |ui: &mut egui::Ui, from_right: bool| {
        ui.spacing_mut().item_spacing.x = gap;
        let order: Vec<usize> = if from_right {
            (0..buttons.len()).rev().collect()
        } else {
            (0..buttons.len()).collect()
        };
        for index in order {
            let button = &buttons[index];
            let widget = widgets::button(button.role, Some(button.icon), button.label, &palette)
                .min_size(vec2(0.0, HEADER_BUTTON_HEIGHT));
            if ui.add(widget).on_hover_text(button.tip).clicked() {
                clicked = Some(index);
            }
        }
    };
    if buttons.is_empty() {
        ui.vertical(heading);
    } else if room < TITLE_ROOM {
        ui.vertical(|ui| {
            heading(ui);
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.y = gap;
                row(ui, false);
            });
        });
    } else {
        ui.horizontal(|ui| {
            let title_height = ui
                .vertical(|ui| {
                    ui.set_max_width(room);
                    heading(ui)
                })
                .inner;
            // Centred on the title's own line, so a long subtitle or links
            // under it don't pull the buttons down.
            let right = ui.max_rect().right();
            let top = ui.min_rect().top() + (title_height - HEADER_BUTTON_HEIGHT).max(0.0) / 2.0;
            let rect = Rect::from_min_max(
                pos2(right - buttons_width - 24.0, top),
                pos2(right, top + HEADER_BUTTON_HEIGHT),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                |ui| row(ui, true),
            );
        });
    }
    clicked
}

/// How wide `widgets::button` makes a button with `icon` and `label`.
pub(super) fn button_width(ui: &egui::Ui, icon: &str, label: &str) -> f32 {
    let text = RichText::new(theme::with_icon(icon, label)).family(theme::medium_family());
    let galley = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    galley.size().x + 2.0 * ui.spacing().button_padding.x
}

/// A card's surface with a spotlight under the pointer, for cards that
/// don't lean ([`tilt`]).
pub(super) fn card_frame(
    ui: &egui::Ui,
    rect: Rect,
    response: &egui::Response,
    hover: f32,
    palette: &Palette,
) {
    card_surface(ui, rect, response, hover, palette);
    widgets::spotlight(ui, rect, response.id, hover);
}

/// A card's surface: a thin outline that strengthens on hover (`hover`
/// from 0 to 1) toward the accent, a faint fill, and the focus ring.
pub(super) fn card_surface(
    ui: &egui::Ui,
    rect: Rect,
    response: &egui::Response,
    hover: f32,
    palette: &Palette,
) {
    ui.painter().rect(
        rect,
        CARD_RADIUS,
        theme::mix(palette.panel, palette.surface, 0.6 + 0.4 * hover),
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
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            CARD_RADIUS + 2,
            Stroke::new(1.5_f32, palette.accent),
            egui::StrokeKind::Inside,
        );
    }
}

/// A soft accent glow behind an icon at `center`, `radius` wide, by the
/// hover's fade.
pub(super) fn icon_glow(
    ui: &egui::Ui,
    center: egui::Pos2,
    radius: f32,
    hover: f32,
    palette: &Palette,
) {
    if hover <= 0.0 {
        return;
    }
    let alpha = if palette.dark { 0.30 } else { 0.20 };
    widgets::soft_pool(
        ui.painter(),
        center,
        radius,
        palette.accent.gamma_multiply(alpha * hover),
    );
}

/// A project's picture: its thumbnail, or a quiet placeholder with its
/// initials on a faint grid. `float` is the hover's parallax: the picture
/// pans inside its frame, zoomed in by as much so no edge shows, and the
/// initials drift.
pub(super) fn thumbnail_image(
    ui: &egui::Ui,
    rect: Rect,
    radius: CornerRadius,
    name: &str,
    thumbnail: Option<&egui::TextureHandle>,
    float: egui::Vec2,
) {
    let palette = theme::palette(ui.ctx());
    match thumbnail {
        Some(texture) => {
            let room = (float.abs() / rect.size()).max_elem().min(0.2);
            let half = 0.5 - room;
            let pan = -float / rect.size();
            let center = pos2(
                0.5 + pan.x.clamp(-room, room),
                0.5 + pan.y.clamp(-room, room),
            );
            egui::Image::new(texture)
                .corner_radius(radius)
                .uv(Rect::from_center_size(center, vec2(2.0 * half, 2.0 * half)))
                .paint_at(ui, rect);
        }
        None => {
            let painter = ui.painter().with_clip_rect(rect);
            painter.rect_filled(rect, radius, palette.surface);
            let step = (rect.height() / 6.0).max(8.0);
            let line = Stroke::new(1.0_f32, palette.border.gamma_multiply(0.7));
            let mut x = rect.left() + step;
            while x < rect.right() {
                painter.vline(x, rect.y_range(), line);
                x += step;
            }
            let mut y = rect.top() + step;
            while y < rect.bottom() {
                painter.hline(rect.x_range(), y, line);
                y += step;
            }
            let initials: String = name
                .split_whitespace()
                .filter_map(|word| word.chars().next())
                .take(2)
                .flat_map(char::to_uppercase)
                .collect();
            painter.text(
                rect.center() + float * 1.5,
                egui::Align2::CENTER_CENTER,
                if initials.is_empty() {
                    "?".to_owned()
                } else {
                    initials
                },
                FontId::new(rect.height() * 0.26, theme::heading_family()),
                palette.text_weak,
            );
        }
    }
}

/// A project's options, for its ⋯ button and its right-click menu.
pub(super) fn project_menu(ui: &mut egui::Ui, action: &mut Option<CardAction>) {
    let palette = theme::palette(ui.ctx());
    let mut item = |ui: &mut egui::Ui, icon: &str, label: &str, picked: CardAction| {
        if ui.button(theme::with_icon(icon, label)).clicked() {
            *action = Some(picked);
        }
    };
    item(ui, icons::FOLDER_OPEN, "Open", CardAction::Open);
    item(ui, icons::GEAR, "Settings…", CardAction::Settings);
    item(
        ui,
        icons::CLOCK_COUNTER_CLOCKWISE,
        "Version History…",
        CardAction::History,
    );
    item(
        ui,
        icons::ARROW_SQUARE_OUT,
        "Show in File Manager",
        CardAction::Reveal,
    );
    if ui
        .button(theme::with_icon(icons::X_CIRCLE, "Remove from List"))
        .on_hover_text("Stops tracking it here. The files are kept.")
        .clicked()
    {
        *action = Some(CardAction::Forget);
    }
    ui.separator();
    let delete = RichText::new(theme::with_icon(icons::TRASH, "Delete…")).color(palette.error);
    if ui.button(delete).clicked() {
        *action = Some(CardAction::Delete);
    }
}

/// The ⋯ button of a card or row, shown while `visible` (the card is
/// hovered) or while its menu is open.
pub(super) fn card_menu(
    ui: &mut egui::Ui,
    rect: Rect,
    id: egui::Id,
    visible: bool,
) -> Option<CardAction> {
    let response = ui.interact(rect, id, Sense::click());
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    let shown = theme::ease(
        ui.ctx(),
        id.with("shown"),
        visible || open || response.has_focus(),
    );
    if shown > 0.0 {
        let palette = theme::palette(ui.ctx());
        let fill = if response.hovered() || open {
            palette.control_active
        } else {
            palette.elevated
        };
        ui.painter().rect(
            rect,
            6,
            fill.gamma_multiply(shown),
            Stroke::new(1.0_f32, palette.border.gamma_multiply(shown)),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icons::DOTS_THREE,
            FontId::proportional(16.0),
            palette.text_strong.gamma_multiply(shown),
        );
    }
    let response = response.on_hover_text("More");
    let mut action = None;
    egui::Popup::menu(&response).show(|ui| project_menu(ui, &mut action));
    action
}

/// One project as a card: its picture, name and when it was edited. Clicking
/// it opens it; ⋯ (shown on hover) and right-click have its options.
pub(super) fn project_card(
    ui: &mut egui::Ui,
    link: &ProjectLink,
    thumbnail: Option<&egui::TextureHandle>,
    edited: Option<String>,
    width: f32,
    section: &str,
) -> Option<CardAction> {
    let picture_height = (width * 9.0 / 16.0).round();
    let size = vec2(width, picture_height + 54.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &link.name));
    let palette = theme::palette(ui.ctx());
    let hovered = ui.rect_contains_pointer(rect);
    let lean = tilt::lean(ui, rect, response.id, hovered);
    let hover = lean.hover;
    let picture = Rect::from_min_size(rect.min, vec2(width, picture_height));
    let mut action = tilt::leaning(ui, picture, CARD_RADIUS, lean, |ui| {
        thumbnail_image(
            ui,
            picture,
            CornerRadius::same(CARD_RADIUS),
            &link.name,
            thumbnail,
            lean.drift(4.0),
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
        if response.has_focus() {
            ui.painter().rect_stroke(
                picture.expand(2.0),
                CARD_RADIUS + 2,
                Stroke::new(1.5_f32, palette.accent),
                egui::StrokeKind::Inside,
            );
        }
        let menu = Rect::from_min_size(
            pos2(picture.right() - 36.0, picture.top() + 8.0),
            vec2(28.0, 24.0),
        );
        card_menu(
            ui,
            menu,
            egui::Id::new(("card menu", section, &link.folder)),
            hovered,
        )
    });

    // The name and details sit under the picture, lifting with it.
    let rise = if lean.motion { -2.0 * hover } else { 0.0 };
    let text_left = rect.left() + 2.0;
    let text_width = width - 4.0;
    let name = widgets::elided(
        ui,
        link.name.clone(),
        FontId::new(type_scale::BODY + 0.5, theme::medium_family()),
        palette.text_strong,
        text_width,
        1,
    );
    let name_top = picture.bottom() + 10.0 + rise;
    ui.painter()
        .galley(pos2(text_left, name_top), name, palette.text_strong);
    let detail = match &edited {
        Some(edited) => format!("Edited {edited}"),
        None => link.folder.display().to_string(),
    };
    let detail = match link
        .description
        .lines()
        .next()
        .filter(|line| !line.is_empty())
    {
        Some(description) => format!("{detail}  ·  {description}"),
        None => detail,
    };
    let detail = widgets::elided(
        ui,
        detail,
        FontId::proportional(type_scale::SMALL),
        palette.text_weak,
        text_width,
        1,
    );
    ui.painter()
        .galley(pos2(text_left, name_top + 20.0), detail, palette.text_weak);

    let response = response
        .on_hover_text(format!("Open {}", link.folder.display()))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    response.context_menu(|ui| project_menu(ui, &mut action));
    if action.is_none() && response.clicked() {
        action = Some(CardAction::Open);
    }
    action
}

/// One project as a row of the list view.
pub(super) fn project_row(
    ui: &mut egui::Ui,
    link: &ProjectLink,
    thumbnail: Option<&egui::TextureHandle>,
    edited: Option<String>,
) -> Option<CardAction> {
    let size = vec2(ui.available_width(), 52.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &link.name));
    let palette = theme::palette(ui.ctx());
    let hovered = ui.rect_contains_pointer(rect);
    widgets::paint_hover(ui, rect, &response, false, theme::radius::CARD);
    if hovered && !response.hovered() {
        // Over the ⋯ button: keep the row lit.
        ui.painter().rect_filled(rect, 8, palette.control_hover);
    }
    let picture = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(36.0 * 16.0 / 9.0, 36.0));
    thumbnail_image(
        ui,
        picture,
        CornerRadius::same(5),
        &link.name,
        thumbnail,
        egui::Vec2::ZERO,
    );
    ui.painter().rect_stroke(
        picture,
        5,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );
    let left = picture.right() + 14.0;
    let edited_width = 150.0;
    let text_width = (rect.right() - edited_width - 50.0 - left).max(40.0);
    let name = widgets::elided(
        ui,
        link.name.clone(),
        FontId::new(type_scale::BODY, theme::medium_family()),
        palette.text_strong,
        text_width,
        1,
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 9.0), name, palette.text_strong);
    let path = widgets::elided(
        ui,
        link.folder.display().to_string(),
        FontId::proportional(type_scale::SMALL),
        palette.text_weak,
        text_width,
        1,
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 28.0), path, palette.text_weak);
    if let Some(edited) = edited {
        ui.painter().text(
            rect.right_center() - vec2(50.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            format!("Edited {edited}"),
            FontId::proportional(type_scale::SMALL),
            palette.text_weak,
        );
    }
    let menu = Rect::from_center_size(rect.right_center() - vec2(22.0, 0.0), vec2(28.0, 24.0));
    let mut action = card_menu(ui, menu, egui::Id::new(("row menu", &link.folder)), hovered);
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    response.context_menu(|ui| project_menu(ui, &mut action));
    if action.is_none() && response.clicked() {
        action = Some(CardAction::Open);
    }
    action
}

/// A soft panel: a gentle fill and a faint shadow, no outline, for notes
/// and rows that sit on the start screen's scene.
pub(super) fn soft_panel(palette: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(super::backdrop::soft_fill(palette))
        .shadow(super::backdrop::soft_shadow(palette))
        .corner_radius(CARD_RADIUS)
}

/// A quiet note in place of the project cards.
pub(super) fn empty_state(ui: &mut egui::Ui, icon: &str, title: &str, text: &str) {
    empty_state_with(ui, icon, title, text, &[]);
}

/// [`empty_state`] with buttons under the note, so the way out is right
/// there: `actions` are (role, icon, label). Returns the index of the one
/// clicked.
pub(super) fn empty_state_with(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    text: &str,
    actions: &[(Role, &str, &str)],
) -> Option<usize> {
    let palette = theme::palette(ui.ctx());
    let mut clicked = None;
    ui.add_space(8.0);
    soft_panel(&palette)
        .inner_margin(egui::Margin::symmetric(16, 32))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(icon).size(24.0).color(palette.text_weak));
                ui.add_space(4.0);
                ui.label(
                    RichText::new(title)
                        .family(theme::medium_family())
                        .color(palette.text_strong),
                );
                ui.label(RichText::new(text).color(palette.text_secondary));
                if actions.is_empty() {
                    return;
                }
                ui.add_space(14.0);
                // Centred as a group: measure the buttons, then lay them out
                // from where the row starts.
                let buttons: Vec<_> = actions
                    .iter()
                    .map(|(role, icon, label)| {
                        widgets::button(*role, Some(icon), label, &palette)
                            .min_size(vec2(0.0, 30.0))
                    })
                    .collect();
                let gap = 8.0;
                let widths: f32 = actions
                    .iter()
                    .map(|(_, icon, label)| button_width(ui, icon, label))
                    .sum::<f32>()
                    + gap * (actions.len() - 1) as f32;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.add_space(((ui.available_width() - widths) / 2.0).max(0.0));
                    for (index, button) in buttons.into_iter().enumerate() {
                        if ui.add(button).clicked() {
                            clicked = Some(index);
                        }
                    }
                });
            });
        });
    ui.add_space(8.0);
    clicked
}
