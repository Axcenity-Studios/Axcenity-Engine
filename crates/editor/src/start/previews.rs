//! Pictures for the blank templates (Baseplate and Empty), drawn with the
//! painter from theme tokens so they follow the theme and accent: small
//! isometric scenes of what the new world starts with, where the genre
//! templates have screenshots.

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Stroke, Vec2, pos2, vec2};

use axcenity_project::Template;

use super::icon_glow;
use crate::theme::{self, Palette};

/// The baseplate's grey, as the template's world has it (0.35, 0.37, 0.4).
const PLATE: Color32 = Color32::from_rgb(89, 94, 102);

/// An isometric view: `center` is the world's origin on screen and `scale`
/// how many points one unit is.
#[derive(Clone, Copy)]
struct Iso {
    center: Pos2,
    scale: f32,
}

impl Iso {
    /// Where the point `(x, y)` on the ground, `z` up, lands on screen.
    fn at(self, x: f32, y: f32, z: f32) -> Pos2 {
        self.center
            + vec2(
                (x - y) * 0.866 * self.scale,
                ((x + y) * 0.5 - z) * self.scale,
            )
    }
}

/// `template`'s picture in `rect`. `hover` (0 to 1) lifts the scene and
/// warms its edges toward the accent; `float` is the hover's parallax.
pub(super) fn blank_preview(
    ui: &egui::Ui,
    template: Template,
    rect: Rect,
    radius: CornerRadius,
    hover: f32,
    float: Vec2,
    palette: &Palette,
) {
    let painter = ui.painter().with_clip_rect(rect);
    // A darker well than the card, so the scene reads as a picture.
    let well = theme::mix(palette.surface, palette.app_bg, 0.55);
    painter.rect(
        rect,
        radius,
        theme::mix(well, palette.control_hover, 0.4 * hover),
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );
    // Sized so the whole scene sits inside the well with a margin all
    // round, and centred on its own middle (not the origin's), so nothing
    // touches the well's edges.
    let (half_width, top, bottom) = extent(template);
    let room = rect.shrink2(vec2(rect.width() * 0.12, rect.height() * 0.14));
    let scale = (room.width() / (2.0 * half_width)).min(room.height() / (bottom - top));
    let iso = Iso {
        center: rect.center() - vec2(0.0, (top + bottom) / 2.0 * scale) + float
            - vec2(0.0, 2.0 * hover),
        scale,
    };
    icon_glow(ui, iso.center, scale * 1.6, hover, palette);
    match template {
        Template::Baseplate => baseplate(&painter, iso, hover, palette),
        _ => empty(&painter, iso, hover, palette),
    }
}

/// How far `template`'s scene reaches from the origin on screen, in units:
/// half its width, and its top and bottom (down is positive).
fn extent(template: Template) -> (f32, f32, f32) {
    match template {
        // The diamond's side corners, its back corner, and its front corner
        // with the slab's thickness under it.
        Template::Baseplate => (1.732, -1.0, 1.0 + BASEPLATE_THICK),
        // The grid's corners (it fades out before them, so a little less)
        // and the dashed box's top.
        _ => (EMPTY_REACH * 1.732 * 0.85, -1.5, EMPTY_REACH * 0.85),
    }
}

/// How thick the baseplate's slab is, in units.
const BASEPLATE_THICK: f32 = 0.16;
/// How far the empty world's grid runs from the origin, in units.
const EMPTY_REACH: f32 = 1.2;

/// A grey slab with a stud grid on top and the character standing on it.
fn baseplate(painter: &egui::Painter, iso: Iso, hover: f32, palette: &Palette) {
    const THICK: f32 = BASEPLATE_THICK;
    let edge = Stroke::new(
        1.0_f32,
        theme::mix(palette.border_strong, palette.accent, 0.35 + 0.45 * hover),
    );
    let face = |corners: [(f32, f32, f32); 4], fill: Color32| {
        let points = corners.map(|(x, y, z)| iso.at(x, y, z)).to_vec();
        painter.add(egui::Shape::convex_polygon(points, fill, edge));
    };
    // The two sides that face the viewer, then the top.
    face(
        [
            (1.0, -1.0, 0.0),
            (1.0, 1.0, 0.0),
            (1.0, 1.0, -THICK),
            (1.0, -1.0, -THICK),
        ],
        theme::mix(PLATE, Color32::BLACK, 0.35),
    );
    face(
        [
            (-1.0, 1.0, 0.0),
            (1.0, 1.0, 0.0),
            (1.0, 1.0, -THICK),
            (-1.0, 1.0, -THICK),
        ],
        theme::mix(PLATE, Color32::BLACK, 0.5),
    );
    face(
        [
            (-1.0, -1.0, 0.0),
            (1.0, -1.0, 0.0),
            (1.0, 1.0, 0.0),
            (-1.0, 1.0, 0.0),
        ],
        theme::mix(PLATE, Color32::WHITE, 0.08),
    );
    // The grid the parts snap to.
    let grid = Stroke::new(1.0_f32, Color32::WHITE.gamma_multiply(0.10));
    for step in 1..8 {
        let t = -1.0 + step as f32 * 0.25;
        painter.line_segment([iso.at(t, -1.0, 0.0), iso.at(t, 1.0, 0.0)], grid);
        painter.line_segment([iso.at(-1.0, t, 0.0), iso.at(1.0, t, 0.0)], grid);
    }
    figure(painter, iso, palette);
}

/// Nothing yet: the ground's grid fading out from the origin, a dashed box
/// where the first part could go, and a glowing point at the origin.
fn empty(painter: &egui::Painter, iso: Iso, hover: f32, palette: &Palette) {
    const REACH: f32 = EMPTY_REACH;
    const STEPS: i32 = 8;
    let line = palette.text_weak;
    for step in -STEPS..=STEPS {
        let t = step as f32 / STEPS as f32 * REACH;
        // Fainter toward the edge, so the grid has no hard border.
        let fade = 1.0 - (t.abs() / REACH).powi(2);
        let stroke = Stroke::new(1.0_f32, line.gamma_multiply(0.28 * fade));
        painter.line_segment([iso.at(t, -REACH, 0.0), iso.at(t, REACH, 0.0)], stroke);
        painter.line_segment([iso.at(-REACH, t, 0.0), iso.at(REACH, t, 0.0)], stroke);
    }
    let dashed = Stroke::new(
        1.2_f32,
        theme::mix(palette.text_secondary, palette.accent, 0.3 + 0.6 * hover),
    );
    let h = 0.5;
    let corner = |x: f32, y: f32, z: f32| iso.at(x * h, y * h, z * 2.0 * h);
    let edges = [
        // The bottom, the top, then the uprights.
        ((-1.0, -1.0, 0.0), (1.0, -1.0, 0.0)),
        ((1.0, -1.0, 0.0), (1.0, 1.0, 0.0)),
        ((1.0, 1.0, 0.0), (-1.0, 1.0, 0.0)),
        ((-1.0, 1.0, 0.0), (-1.0, -1.0, 0.0)),
        ((-1.0, -1.0, 1.0), (1.0, -1.0, 1.0)),
        ((1.0, -1.0, 1.0), (1.0, 1.0, 1.0)),
        ((1.0, 1.0, 1.0), (-1.0, 1.0, 1.0)),
        ((-1.0, 1.0, 1.0), (-1.0, -1.0, 1.0)),
        ((-1.0, -1.0, 0.0), (-1.0, -1.0, 1.0)),
        ((1.0, -1.0, 0.0), (1.0, -1.0, 1.0)),
        ((1.0, 1.0, 0.0), (1.0, 1.0, 1.0)),
        ((-1.0, 1.0, 0.0), (-1.0, 1.0, 1.0)),
    ];
    for ((ax, ay, az), (bx, by, bz)) in edges {
        let points = [corner(ax, ay, az), corner(bx, by, bz)];
        painter.extend(egui::Shape::dashed_line(&points, dashed, 3.0, 3.0));
    }
    let origin = iso.at(0.0, 0.0, 0.0);
    crate::widgets::soft_pool(
        painter,
        origin,
        iso.scale * 0.35,
        palette.accent.gamma_multiply(0.45),
    );
    painter.circle_filled(origin, 2.0, palette.accent);
}

/// The default character, as a simple pawn in the accent colour, standing
/// at the origin with its shadow.
fn figure(painter: &egui::Painter, iso: Iso, palette: &Palette) {
    let foot = iso.at(0.0, 0.0, 0.0);
    let unit = iso.scale;
    painter.add(egui::Shape::ellipse_filled(
        foot,
        vec2(unit * 0.16, unit * 0.08),
        Color32::BLACK.gamma_multiply(0.3),
    ));
    let body = Rect::from_min_max(
        pos2(foot.x - unit * 0.09, foot.y - unit * 0.42),
        pos2(foot.x + unit * 0.09, foot.y),
    );
    painter.rect_filled(
        body,
        CornerRadius::same((unit * 0.06) as u8),
        palette.accent,
    );
    painter.circle_filled(
        pos2(foot.x, body.top() - unit * 0.1),
        unit * 0.085,
        palette.accent,
    );
}
