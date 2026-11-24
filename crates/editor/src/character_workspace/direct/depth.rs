//! What the overlay can see: a small buffer of the character as it's drawn
//! this frame (its body and the parts it wears), rasterized on the CPU from
//! the posed corners. The overlay is drawn over the preview with egui,
//! which has no depth test, so a handle on skin behind other skin (the far
//! jaw angle behind the cheek, the far eye behind the nose) would show
//! through: each handle is tested against the buffer's depth first.
//!
//! The buffer can also keep **which surface** each texel shows
//! ([`Depth::recording`], [`Seen`]): the nearest triangle there and where
//! in it. The highlight ([`super::mask`]) reads a feature's weight from
//! that, so it lies on whatever the camera sees and nothing is compared
//! with a depth a second time.

use axcenity_render::glam::{Vec2, Vec3};
use axcenity_scene::CameraView;
use eframe::egui::Rect;

/// Points of the screen per texel of the buffer (a quarter of the texels
/// of the screen's points: plenty for a handle or a soft highlight).
pub(super) const TEXEL: f32 = 2.0;
/// The nearest a corner may be to the camera (world units) and still be
/// rasterized; a triangle with a corner nearer is skipped.
const NEAR: f32 = 0.01;

/// What a texel shows: the triangle nearest the camera there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Seen {
    /// The surface, as [`Depth::add`] was given it ([`Seen::NOTHING`]: the
    /// backdrop).
    pub surface: u32,
    /// The triangle's index in that surface's `indices`.
    pub triangle: u32,
    /// How much of the triangle's first and second corners makes the
    /// point (the third's is the rest), true to the perspective.
    pub shares: [f32; 2],
}

impl Seen {
    pub(super) const NOTHING: u32 = u32::MAX;
    const EMPTY: Self = Self {
        surface: Self::NOTHING,
        triangle: 0,
        shares: [0.0; 2],
    };
}

/// The nearest depth (distance along the camera's forward axis, in world
/// units) of each texel of the view, and what's seen there when asked to
/// keep it.
#[derive(Clone)]
pub(super) struct Depth {
    rect: Rect,
    width: usize,
    height: usize,
    depth: Vec<f32>,
    /// Each texel's surface (empty unless [`Self::recording`]).
    seen: Vec<Seen>,
    view: CameraView,
    eye: Vec3,
    forward: Vec3,
    /// World units across one texel, one unit in front of the camera.
    texel_per_depth: f32,
}

impl Depth {
    /// An empty buffer for `view` drawn into `rect`, the camera at `eye`
    /// looking at `look_at` with a vertical field of view of `fov_y`
    /// degrees.
    pub(super) fn new(view: CameraView, rect: Rect, eye: Vec3, look_at: Vec3, fov_y: f32) -> Self {
        let width = (rect.width() / TEXEL).ceil().max(1.0) as usize;
        let height = (rect.height() / TEXEL).ceil().max(1.0) as usize;
        let tan = (fov_y.to_radians() / 2.0).tan();
        Self {
            rect,
            width,
            height,
            depth: vec![f32::INFINITY; width * height],
            seen: Vec::new(),
            view,
            eye,
            forward: (look_at - eye).normalize_or_zero(),
            texel_per_depth: 2.0 * tan * TEXEL / rect.height().max(1.0),
        }
    }

    /// The buffer keeping what each texel shows too ([`Self::seen`]).
    pub(super) fn recording(mut self) -> Self {
        self.seen = vec![Seen::EMPTY; self.width * self.height];
        self
    }

    /// The depths alone, as they are now (what a handle is tested against
    /// before the parts worn over the skin are added).
    pub(super) fn depths(&self) -> Self {
        Self {
            depth: self.depth.clone(),
            seen: Vec::new(),
            ..*self
        }
    }

    /// The buffer's size in texels (across, down).
    pub(super) fn size(&self) -> [usize; 2] {
        [self.width, self.height]
    }

    /// The part of the screen the buffer covers.
    pub(super) fn rect(&self) -> Rect {
        self.rect
    }

    /// What each texel shows, row by row from the top (empty unless
    /// [`Self::recording`]).
    pub(super) fn seen(&self) -> &[Seen] {
        &self.seen
    }

    /// Where `point` falls in the buffer (texels, continuous: texel `i`'s
    /// middle is at `i + 0.5`), and its depth.
    fn locate(&self, point: Vec3) -> Option<(Vec2, f32)> {
        let depth = (point - self.eye).dot(self.forward);
        if depth < NEAR {
            return None;
        }
        let ndc = self.view.project_point(point)?;
        let x = (ndc.x + 1.0) * 0.5 * self.rect.width() / TEXEL;
        let y = (1.0 - ndc.y) * 0.5 * self.rect.height() / TEXEL;
        Some((Vec2::new(x, y), depth))
    }

    /// Draws the triangles `indices` of `corners` (world points) into the
    /// buffer as surface number `surface`: each texel whose middle a
    /// triangle covers keeps the nearest. Both sides of a triangle count.
    /// `slack` (world units) puts them that much farther than they are:
    /// a part that can't carry the highlight itself gives way to the skin
    /// just under it.
    pub(super) fn add(&mut self, surface: u32, corners: &[Vec3], indices: &[u32], slack: f32) {
        let located: Vec<Option<(Vec2, f32)>> =
            corners.iter().map(|point| self.locate(*point)).collect();
        for (index, triangle) in indices.as_chunks::<3>().0.iter().enumerate() {
            let [Some(Some(a)), Some(Some(b)), Some(Some(c))] =
                triangle.map(|corner| located.get(corner as usize).copied())
            else {
                continue;
            };
            self.fill(a, b, c, slack, (surface, index as u32));
        }
    }

    fn fill(
        &mut self,
        (a, da): (Vec2, f32),
        (b, db): (Vec2, f32),
        (c, dc): (Vec2, f32),
        slack: f32,
        (surface, triangle): (u32, u32),
    ) {
        let area = (b - a).perp_dot(c - a);
        if area.abs() < 1e-9 {
            return;
        }
        let low = a.min(b).min(c);
        let high = a.max(b).max(c);
        // The texels whose middles may be inside.
        let x0 = (low.x - 0.5).ceil().max(0.0) as usize;
        let y0 = (low.y - 0.5).ceil().max(0.0) as usize;
        let x1 = ((high.x - 0.5).floor() as isize).min(self.width as isize - 1);
        let y1 = ((high.y - 0.5).floor() as isize).min(self.height as isize - 1);
        if x1 < x0 as isize || y1 < y0 as isize {
            return;
        }
        // 1/depth is linear on the screen, so it's what's interpolated.
        let inverse = [1.0 / da, 1.0 / db, 1.0 / dc];
        // Each corner's share of a point (barycentric): the area of the
        // triangle the point makes with the other two, over the whole
        // one's. It changes evenly across the screen, so it's worked out
        // once for a row's first texel and stepped along the row.
        let (edge_a, edge_b) = ((c - b) / area, (a - c) / area);
        let (step_a, step_b) = (-edge_a.y, -edge_b.y);
        // A texel on a side shared by two triangles is in both (the
        // nearer keeps it), never in neither by a rounding.
        const ON_A_SIDE: f32 = -1e-5;
        for y in y0..=y1 as usize {
            let first = Vec2::new(x0 as f32 + 0.5, y as f32 + 0.5);
            let mut wa = edge_a.perp_dot(first - b);
            let mut wb = edge_b.perp_dot(first - c);
            let row = y * self.width;
            for at in row + x0..=row + x1 as usize {
                let wc = 1.0 - wa - wb;
                if wa >= ON_A_SIDE && wb >= ON_A_SIDE && wc >= ON_A_SIDE {
                    let (na, nb) = (wa * inverse[0], wb * inverse[1]);
                    let near = na + nb + wc * inverse[2];
                    let depth = 1.0 / near + slack;
                    if depth < self.depth[at] {
                        self.depth[at] = depth;
                        if let Some(seen) = self.seen.get_mut(at) {
                            *seen = Seen {
                                surface,
                                triangle,
                                shares: [na / near, nb / near],
                            };
                        }
                    }
                }
                wa += step_a;
                wb += step_b;
            }
        }
    }

    /// Whether `point` (a point of the skin, in the world) is behind what
    /// the buffer holds by more than `tolerance` (world units). It's
    /// compared with the farthest of the texels round it (`reach` of them
    /// each way: 1 is the four nearest), plus a texel's width at its
    /// depth, so skin at a slant isn't hidden by itself; where any of
    /// those is empty (the backdrop), it shows. A wider reach lets a point
    /// at the skin's own outline show (the side of the hips seen from the
    /// front is a little behind the skin just inside it).
    pub(super) fn hidden(&self, point: Vec3, tolerance: f32, reach: i32) -> bool {
        let Some((at, depth)) = self.locate(point) else {
            return true;
        };
        let (x, y) = ((at.x - 0.5).floor() as i32, (at.y - 0.5).floor() as i32);
        let mut farthest = 0.0_f32;
        for ty in y + 1 - reach..=y + reach {
            for tx in x + 1 - reach..=x + reach {
                if tx < 0 || ty < 0 || tx >= self.width as i32 || ty >= self.height as i32 {
                    return false;
                }
                farthest = farthest.max(self.depth[ty as usize * self.width + tx as usize]);
            }
        }
        depth > farthest + tolerance + self.texel_per_depth * depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axcenity_render::Camera;
    use eframe::egui;

    fn view() -> (Depth, Vec3) {
        let camera = Camera {
            position: Vec3::new(0.0, 0.0, 10.0),
            look_at: Vec3::ZERO,
            fov_y_degrees: 30.0,
        };
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 300.0));
        let view = CameraView::new(camera, 400.0 / 300.0);
        (
            Depth::new(view, rect, camera.position, camera.look_at, 30.0),
            camera.position,
        )
    }

    /// A square of side 2 at height `z`, facing the camera.
    fn square(depth: &mut Depth, z: f32) {
        let corners = [
            Vec3::new(-1.0, -1.0, z),
            Vec3::new(1.0, -1.0, z),
            Vec3::new(1.0, 1.0, z),
            Vec3::new(-1.0, 1.0, z),
        ];
        depth.add(0, &corners, &[0, 1, 2, 0, 2, 3], 0.0);
    }

    #[test]
    fn a_point_behind_a_surface_is_hidden_and_one_on_it_is_not() {
        let (mut depth, _) = view();
        square(&mut depth, 1.0);
        // On the square, and a hair behind it.
        assert!(!depth.hidden(Vec3::new(0.3, 0.2, 1.0), 0.003, 1));
        assert!(!depth.hidden(Vec3::new(0.3, 0.2, 0.999), 0.003, 1));
        // Well behind it, and behind it near its edge.
        assert!(depth.hidden(Vec3::new(0.3, 0.2, 0.5), 0.003, 1));
        assert!(depth.hidden(Vec3::new(-0.8, 0.8, 0.0), 0.003, 1));
        // Beside it, behind its plane: nothing in front.
        assert!(!depth.hidden(Vec3::new(2.5, 0.0, 0.0), 0.003, 1));
        // In front of it.
        assert!(!depth.hidden(Vec3::new(0.0, 0.0, 3.0), 0.003, 1));
    }

    #[test]
    fn the_nearer_of_two_surfaces_wins_whichever_is_drawn_first() {
        let (mut depth, _) = view();
        square(&mut depth, -2.0);
        square(&mut depth, 1.0);
        assert!(depth.hidden(Vec3::new(0.0, 0.0, -2.0), 0.003, 1));
        assert!(!depth.hidden(Vec3::new(0.0, 0.0, 1.0), 0.003, 1));
    }

    #[test]
    fn a_texel_shows_the_nearest_triangle_and_where_in_it() {
        let (depth, _) = view();
        let mut depth = depth.recording();
        // A wide far square, then a near one tilted toward the camera on
        // one side, drawn as surface 7.
        let far = [
            Vec3::new(-3.0, -1.0, -2.0),
            Vec3::new(3.0, -1.0, -2.0),
            Vec3::new(3.0, 1.0, -2.0),
            Vec3::new(-3.0, 1.0, -2.0),
        ];
        depth.add(0, &far, &[0, 1, 2, 0, 2, 3], 0.0);
        let corners = [
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 3.0),
            Vec3::new(1.0, 1.0, 3.0),
            Vec3::new(-1.0, 1.0, 0.0),
        ];
        let indices = [0, 1, 2, 0, 2, 3];
        depth.add(7, &corners, &indices, 0.0);
        let [width, height] = depth.size();
        assert_eq!([width, height], [200, 150]);
        let mut checked = 0;
        for (at, seen) in depth.seen().iter().enumerate() {
            if seen.surface != 7 {
                continue;
            }
            // The point of the triangle its shares name is seen through
            // the texel's middle (to a hundredth of a texel).
            let triangle = seen.triangle as usize * 3;
            let [a, b, c] = [0, 1, 2].map(|k| corners[indices[triangle + k] as usize]);
            let [sa, sb] = seen.shares;
            let point = a * sa + b * sb + c * (1.0 - sa - sb);
            let (on_screen, _) = depth.locate(point).unwrap();
            let middle = Vec2::new((at % width) as f32 + 0.5, (at / width) as f32 + 0.5);
            assert!(
                (on_screen - middle).length() < 0.01,
                "{on_screen} at {middle}"
            );
            checked += 1;
        }
        assert!(checked > 2000, "the near square covers {checked} texels");
        // Beside it the far square shows (surface 0), and beyond both
        // nothing does.
        assert_eq!(depth.seen()[75 * width + 40].surface, 0);
        assert_eq!(depth.seen()[0].surface, Seen::NOTHING);
        // The depths alone keep what hides what.
        let alone = depth.depths();
        assert!(alone.seen().is_empty());
        assert!(alone.hidden(Vec3::new(0.0, 0.0, -2.0), 0.003, 1));
    }

    #[test]
    fn what_s_worn_hides_only_what_s_well_behind_it() {
        let (mut depth, _) = view();
        let corners = [
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];
        depth.add(0, &corners, &[0, 1, 2, 0, 2, 3], 0.1);
        // Just under it, and well behind it.
        assert!(!depth.hidden(Vec3::new(0.0, 0.0, 0.95), 0.003, 1));
        assert!(depth.hidden(Vec3::new(0.0, 0.0, 0.5), 0.003, 1));
    }

    #[test]
    fn a_wider_reach_lets_a_point_near_an_outline_show() {
        let (mut depth, _) = view();
        square(&mut depth, 1.0);
        // Behind the square, a texel and a half inside its right edge as
        // seen (the edge, nearer, shows a little wider than its x).
        let point = Vec3::new(1.05, 0.0, 0.0);
        assert!(depth.hidden(point, 0.003, 1));
        assert!(!depth.hidden(point, 0.003, 3));
    }
}
