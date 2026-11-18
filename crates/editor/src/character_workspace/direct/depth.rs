//! What the overlay can see: a small depth buffer of the character as it's
//! drawn this frame (its body and the parts it wears), rasterized on the
//! CPU from the posed corners. The overlay is drawn over the preview with
//! egui, which has no depth test, so a handle or a tint on skin behind
//! other skin (the far jaw angle behind the cheek, the far eye behind the
//! nose, a waist behind an arm) would show through. Each point the overlay
//! draws is tested against this first.

use axcenity_render::glam::{Vec2, Vec3};
use axcenity_scene::CameraView;
use eframe::egui::Rect;

/// Points of the screen per texel of the buffer (a quarter of the texels
/// of the screen's points: plenty for a handle or a tint's corner).
const TEXEL: f32 = 2.0;
/// The nearest a corner may be to the camera (world units) and still be
/// rasterized; a triangle with a corner nearer is skipped.
const NEAR: f32 = 0.01;

/// The nearest depth (distance along the camera's forward axis, in world
/// units) of each texel of the view.
pub(super) struct Depth {
    rect: Rect,
    width: usize,
    height: usize,
    depth: Vec<f32>,
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
            view,
            eye,
            forward: (look_at - eye).normalize_or_zero(),
            texel_per_depth: 2.0 * tan * TEXEL / rect.height().max(1.0),
        }
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
    /// buffer: each texel whose middle a triangle covers keeps the nearest
    /// depth. Both sides of a triangle count. `slack` (world units) puts
    /// them that much farther than they are: what's worn hides only what's
    /// well behind it, not the skin just under it.
    pub(super) fn add(&mut self, corners: &[Vec3], indices: &[u32], slack: f32) {
        let located: Vec<Option<(Vec2, f32)>> =
            corners.iter().map(|point| self.locate(*point)).collect();
        for triangle in indices.as_chunks::<3>().0 {
            let [Some(Some(a)), Some(Some(b)), Some(Some(c))] =
                triangle.map(|corner| located.get(corner as usize).copied())
            else {
                continue;
            };
            self.fill(a, b, c, slack);
        }
    }

    fn fill(
        &mut self,
        (a, da): (Vec2, f32),
        (b, db): (Vec2, f32),
        (c, dc): (Vec2, f32),
        slack: f32,
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
        for y in y0..=y1 as usize {
            for x in x0..=x1 as usize {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                // Each corner's share of `p` (barycentric), by the areas
                // of the triangles `p` makes with the other two.
                let wa = (c - b).perp_dot(p - b) / area;
                let wb = (a - c).perp_dot(p - c) / area;
                let wc = 1.0 - wa - wb;
                if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                    continue;
                }
                let depth = 1.0 / (wa * inverse[0] + wb * inverse[1] + wc * inverse[2]) + slack;
                let slot = &mut self.depth[y * self.width + x];
                if depth < *slot {
                    *slot = depth;
                }
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
        depth.add(&corners, &[0, 1, 2, 0, 2, 3], 0.0);
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
    fn what_s_worn_hides_only_what_s_well_behind_it() {
        let (mut depth, _) = view();
        let corners = [
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];
        depth.add(&corners, &[0, 1, 2, 0, 2, 3], 0.1);
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
