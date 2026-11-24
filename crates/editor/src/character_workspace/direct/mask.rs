//! The highlight of a feature on the character: a soft mask over the
//! viewport, drawn as one textured quad.
//!
//! It's made from what the camera sees ([`Depth::seen`]: for each texel
//! the nearest triangle of the body or of a part worn over it), never from
//! the meshes' own triangles and edges:
//!
//! 1. **Cover.** A texel is the feature's when the weight of the surface
//!    seen there (the triangle's corners' weights, blended by where the
//!    texel is in it) is at least the edge weight: the same rule a click
//!    picks by, so what's lit is what's picked. The nearest surface was
//!    settled once, when the buffer was drawn; nothing is compared with a
//!    depth here, so two layers of cloth can't flicker against each other.
//! 2. **Soften.** The cover is blurred (a box of [`BLUR_RADIUS`] texels,
//!    [`BLUR_PASSES`] times each way, near enough a Gaussian), which
//!    rounds off the texels' steps and a coarse garment's triangles, and
//!    is then eased ([`FADE`]): an even tint inside that fades out over
//!    about 8 points at the edge. The edge is a little stronger where the
//!    blurred cover is about a half, a soft rim from the mask itself.
//! 3. **Draw.** The strength goes into one texture (white, by its alpha),
//!    allocated once for the viewport's size and updated in place only
//!    where it changed; it's drawn tinted with the theme's accent.

use std::borrow::Cow;

use eframe::egui::{self, Color32, Rect};

use super::depth::{Depth, Seen, TEXEL};
use super::worn::{self, Fine};

/// The box blur's reach each way, in texels.
const BLUR_RADIUS: usize = 1;
/// How many times the box is run each way (four of radius 1 spread a hard
/// edge like a Gaussian of 1.6 texels: 10% to 90% over about 8 points).
const BLUR_PASSES: usize = 4;
/// The blurred cover at which the tint starts and at which it's full.
const FADE: [f32; 2] = [0.06, 0.94];

/// A surface of the buffer: its triangles and, for a worn part, how each
/// is sampled ([`worn`]).
#[derive(Clone, Copy)]
pub(super) struct Surface<'a> {
    pub indices: &'a [u32],
    pub fine: &'a [Fine],
}

/// One feature's part of the highlight.
pub(super) struct Layer<'a> {
    /// For each surface of the buffer, by its number, its points' weights
    /// for the feature as they show: its corners', then for a worn part
    /// its triangles' lattices' (`None`: it carries none).
    pub weights: Vec<Option<Cow<'a, [f32]>>>,
    /// How strong the tint is inside (0 to 1).
    pub fill: f32,
    /// How much stronger it is along the edge.
    pub rim: f32,
}

/// A block of texels: from `x0`, `y0` up to but not `x1`, `y1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Region {
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
}

impl Region {
    fn texel(x: usize, y: usize) -> Self {
        Self {
            x0: x,
            y0: y,
            x1: x + 1,
            y1: y + 1,
        }
    }

    fn with(self, other: Self) -> Self {
        Self {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }

    /// Wider by `by` texels each way, inside a buffer of `width` by
    /// `height`.
    fn grown(self, by: usize, [width, height]: [usize; 2]) -> Self {
        Self {
            x0: self.x0.saturating_sub(by),
            y0: self.y0.saturating_sub(by),
            x1: (self.x1 + by).min(width),
            y1: (self.y1 + by).min(height),
        }
    }

    fn size(self) -> [usize; 2] {
        [self.x1 - self.x0, self.y1 - self.y0]
    }
}

fn union(a: Option<Region>, b: Option<Region>) -> Option<Region> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.with(b)),
        (a, b) => a.or(b),
    }
}

/// The highlight as last made, and its texture.
#[derive(Default)]
pub(super) struct Mask {
    texture: Option<egui::TextureHandle>,
    /// The egui context the texture is in.
    home: Option<egui::Context>,
    /// The buffer's size in texels and the part of the screen it covers.
    size: [usize; 2],
    rect: Option<Rect>,
    /// Each texel's strength as the texture holds it (its alpha).
    alpha: Vec<u8>,
    /// The texels that may be lit; `None` while nothing is.
    lit: Option<Region>,
    /// Whether it was drawn this frame.
    shown: bool,
    // Kept between frames so a rebuild allocates nothing.
    cover: Vec<f32>,
    scratch: Vec<f32>,
    strength: Vec<f32>,
    /// How many times it was made (tests: a still view makes it once).
    #[cfg(test)]
    pub(super) rebuilds: usize,
}

impl Mask {
    /// Makes the highlight of `layers` over what `depth` sees and puts it
    /// in the texture. `surfaces` holds each surface's triangles, by its
    /// number in the buffer; a texel is a layer's where its weight is at
    /// least `edge`.
    pub(super) fn rebuild(
        &mut self,
        ctx: &egui::Context,
        depth: &Depth,
        surfaces: &[Surface],
        layers: &[Layer],
        edge: f32,
    ) {
        let changed = self.compose(depth, surfaces, layers, edge);
        self.upload(ctx, changed);
    }

    /// The strengths of `layers` over what `depth` sees, into
    /// [`Self::alpha`]. The texels that changed are returned.
    fn compose(
        &mut self,
        depth: &Depth,
        surfaces: &[Surface],
        layers: &[Layer],
        edge: f32,
    ) -> Option<Region> {
        #[cfg(test)]
        {
            self.rebuilds += 1;
        }
        let size @ [width, height] = depth.size();
        let texels = width * height;
        let before = if self.size == size && self.alpha.len() == texels {
            self.lit.take()
        } else {
            // Another size: another texture.
            self.size = size;
            self.alpha = vec![0; texels];
            self.texture = None;
            self.lit = None;
            None
        };
        self.rect = Some(depth.rect());
        if let Some(region) = before {
            for y in region.y0..region.y1 {
                self.alpha[y * width + region.x0..y * width + region.x1].fill(0);
            }
        }
        self.strength.clear();
        self.strength.resize(texels, 0.0);
        let seen = depth.seen();
        let mut lit = None;
        for layer in layers {
            self.cover.clear();
            self.cover.resize(texels, 0.0);
            let mut own: Option<Region> = None;
            for (at, seen) in seen.iter().enumerate() {
                if weight(seen, surfaces, &layer.weights) >= edge {
                    self.cover[at] = 1.0;
                    own = union(own, Some(Region::texel(at % width, at / width)));
                }
            }
            let Some(own) = own else {
                continue;
            };
            // As far as the blur carries it.
            let region = own.grown(BLUR_RADIUS * BLUR_PASSES + 1, size);
            blur(&mut self.cover, &mut self.scratch, width, region);
            for y in region.y0..region.y1 {
                for x in region.x0..region.x1 {
                    let at = y * width + x;
                    let own = strength(self.cover[at], layer.fill, layer.rim);
                    // One layer over another, as two tints drawn in turn.
                    let under = self.strength[at];
                    self.strength[at] = under + own - under * own;
                }
            }
            lit = union(lit, Some(region));
        }
        if let Some(region) = lit {
            for y in region.y0..region.y1 {
                for x in region.x0..region.x1 {
                    let at = y * width + x;
                    self.alpha[at] = (self.strength[at].clamp(0.0, 1.0) * 255.0).round() as u8;
                }
            }
        }
        self.lit = lit;
        union(before, lit)
    }

    /// Puts the texels of `changed` in the texture (all of them into a new
    /// one when there's none of this size).
    fn upload(&mut self, ctx: &egui::Context, changed: Option<Region>) {
        let options = egui::TextureOptions::LINEAR;
        let [width, height] = self.size;
        let image = |region: Region| {
            let mut pixels = Vec::with_capacity(region.size()[0] * region.size()[1]);
            for y in region.y0..region.y1 {
                let row = &self.alpha[y * width + region.x0..y * width + region.x1];
                pixels.extend(row.iter().map(|alpha| Color32::from_white_alpha(*alpha)));
            }
            egui::ColorImage::new(region.size(), pixels)
        };
        // A texture lives in the context it was made in.
        if self.home.as_ref() != Some(ctx) {
            self.texture = None;
            self.home = Some(ctx.clone());
        }
        match &mut self.texture {
            Some(texture) if texture.size() == self.size => {
                if let Some(region) = changed {
                    texture.set_partial([region.x0, region.y0], image(region), options);
                }
            }
            _ => {
                let whole = Region {
                    x0: 0,
                    y0: 0,
                    x1: width,
                    y1: height,
                };
                self.texture =
                    Some(ctx.load_texture("character-studio-highlight", image(whole), options));
            }
        }
    }

    /// Draws the highlight in `colour`: one quad over the texels that are
    /// lit.
    pub(super) fn paint(&mut self, painter: &egui::Painter, colour: Color32) {
        self.shown = true;
        let (Some(texture), Some(lit), Some(rect)) = (&self.texture, self.lit, self.rect) else {
            return;
        };
        let [width, height] = self.size;
        let on_screen = Rect::from_min_max(
            rect.min + egui::vec2(lit.x0 as f32, lit.y0 as f32) * TEXEL,
            rect.min + egui::vec2(lit.x1 as f32, lit.y1 as f32) * TEXEL,
        );
        let uv = Rect::from_min_max(
            egui::pos2(lit.x0 as f32 / width as f32, lit.y0 as f32 / height as f32),
            egui::pos2(lit.x1 as f32 / width as f32, lit.y1 as f32 / height as f32),
        );
        painter.image(texture.id(), on_screen, uv, colour.to_opaque());
    }

    /// Nothing is highlighted this frame.
    pub(super) fn hide(&mut self) {
        self.shown = false;
    }

    /// Whether `ctx` has the highlight's texture (so it can be drawn again
    /// as it is).
    pub(super) fn ready(&self, ctx: &egui::Context) -> bool {
        self.texture.is_some() && self.home.as_ref() == Some(ctx)
    }

    /// The highlight as drawn this frame: the buffer's width, the part of
    /// the screen it covers and each texel's strength (0 to 1), row by row.
    /// `None` when none was drawn.
    #[cfg(test)]
    pub(super) fn drawn(&self) -> Option<(usize, Rect, Vec<f32>)> {
        let rect = self.rect.filter(|_| self.shown && self.texture.is_some())?;
        let strengths = self
            .alpha
            .iter()
            .map(|alpha| f32::from(*alpha) / 255.0)
            .collect();
        Some((self.size[0], rect, strengths))
    }
}

/// The weight of the surface `seen` shows, by `weights` (each surface's
/// points' weights): the triangle's, blended by where the texel is in it
/// ([`worn::blend`]). Nothing where there's no surface or it has no
/// weights.
fn weight(seen: &Seen, surfaces: &[Surface], weights: &[Option<Cow<[f32]>>]) -> f32 {
    if seen.surface == Seen::NOTHING {
        return 0.0;
    }
    let surface = seen.surface as usize;
    let Some(Some(weights)) = weights.get(surface) else {
        return 0.0;
    };
    let triangle = seen.triangle as usize;
    let Some(surface) = surfaces.get(surface) else {
        return 0.0;
    };
    let Some(&[a, b, c]) = surface.indices.get(triangle * 3..triangle * 3 + 3) else {
        return 0.0;
    };
    let [sa, sb] = seen.shares;
    worn::blend(
        weights,
        [a, b, c],
        [sa, sb, 1.0 - sa - sb],
        surface.fine.get(triangle).copied(),
    )
}

/// How strong the tint is where the blurred cover is `cover`: `fill` once
/// it's eased in, and up to `rim` more where the cover is about a half.
fn strength(cover: f32, fill: f32, rim: f32) -> f32 {
    let t = ((cover - FADE[0]) / (FADE[1] - FADE[0])).clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    // 1 at a half, nothing at either end, and flat there.
    let at_edge = (4.0 * t * (1.0 - t)).powi(2);
    fill * eased + rim * at_edge
}

/// Blurs `values` (rows of `width`) inside `region`: a box of
/// [`BLUR_RADIUS`] across then down, [`BLUR_PASSES`] times. Past the
/// region's edge its edge value repeats: nothing where the region has a
/// margin of its own, and at the buffer's edge a feature that goes on past
/// it doesn't fade there.
fn blur(values: &mut [f32], scratch: &mut Vec<f32>, width: usize, region: Region) {
    scratch.resize(values.len(), 0.0);
    let [across, down] = region.size();
    for _ in 0..BLUR_PASSES {
        for y in region.y0..region.y1 {
            box_line(values, scratch, y * width + region.x0, 1, across);
        }
        for x in region.x0..region.x1 {
            box_line(scratch, values, region.y0 * width + x, width, down);
        }
    }
}

/// One line of a box blur: `count` values of `from`, each `stride` after
/// the last from `start`, averaged over [`BLUR_RADIUS`] each way into the
/// same places of `to` (a running sum; the ends repeat).
fn box_line(from: &[f32], to: &mut [f32], start: usize, stride: usize, count: usize) {
    if count == 0 {
        return;
    }
    let at = |index: isize| from[start + index.clamp(0, count as isize - 1) as usize * stride];
    let radius = BLUR_RADIUS as isize;
    let share = 1.0 / (2 * BLUR_RADIUS + 1) as f32;
    let mut sum: f32 = (-radius..=radius).map(at).sum();
    for index in 0..count as isize {
        to[start + index as usize * stride] = sum * share;
        sum += at(index + radius + 1) - at(index - radius);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axcenity_render::Camera;
    use axcenity_render::glam::Vec3;
    use axcenity_scene::CameraView;

    /// A buffer of 200 by 150 texels looking down -z at the origin from 10
    /// units off.
    fn buffer() -> Depth {
        let camera = Camera {
            position: Vec3::new(0.0, 0.0, 10.0),
            look_at: Vec3::ZERO,
            fov_y_degrees: 30.0,
        };
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 300.0));
        let view = CameraView::new(camera, 400.0 / 300.0);
        Depth::new(view, rect, camera.position, camera.look_at, 30.0).recording()
    }

    /// A square of half-side `half` at height `z`, facing the camera.
    fn square(half: f32, z: f32) -> [Vec3; 4] {
        [
            Vec3::new(-half, -half, z),
            Vec3::new(half, -half, z),
            Vec3::new(half, half, z),
            Vec3::new(-half, half, z),
        ]
    }

    const QUAD: [u32; 6] = [0, 1, 2, 0, 2, 3];
    const BARE: Surface = Surface {
        indices: &QUAD,
        fine: &[],
    };

    fn layer(weights: Vec<Option<Vec<f32>>>) -> Layer<'static> {
        Layer {
            weights: weights
                .into_iter()
                .map(|weights| weights.map(Cow::Owned))
                .collect(),
            fill: 0.2,
            rim: 0.0,
        }
    }

    fn made(depth: &Depth, surfaces: &[Surface], layers: &[Layer]) -> (Mask, usize) {
        let mut mask = Mask::default();
        mask.compose(depth, surfaces, layers, 0.5);
        (mask, depth.size()[0])
    }

    #[test]
    fn the_mask_is_even_inside_and_fades_out_once_across_its_edge() {
        let mut depth = buffer();
        depth.add(0, &square(1.0, 0.0), &QUAD, 0.0);
        let (mask, width) = made(&depth, &[BARE], &[layer(vec![Some(vec![1.0; 4])])]);
        let full = (0.2_f32 * 255.0).round() as u8;
        // Along the middle row, from the square's middle out to the right.
        let row: Vec<u8> = (100..200).map(|x| mask.alpha[75 * width + x]).collect();
        assert_eq!(row[0], full, "full inside");
        assert_eq!(*row.last().unwrap(), 0, "nothing far outside");
        assert!(
            row.windows(2).all(|pair| pair[1] <= pair[0]),
            "it only falls going out: {row:?}"
        );
        // Not a hard edge: from full to nothing over 3 to 6 texels (6 to
        // 12 points).
        let fading = row.iter().filter(|a| **a > 0 && **a < full).count();
        assert!((3..=6).contains(&fading), "{fading} texels of fade");
        // The square's own outline (28 texels from the middle) is where
        // the tint is about half.
        let edge = row.iter().position(|a| *a <= full / 2).unwrap();
        assert!((27..=30).contains(&edge), "half at {edge}");
    }

    #[test]
    fn the_nearest_surface_alone_says_what_is_lit() {
        // A weighted square behind and a bare one a hair in front of its
        // left half: the bare one doesn't show it through, however near
        // the two are (there's no tolerance to flicker on).
        let mut depth = buffer();
        depth.add(0, &square(1.0, 0.0), &QUAD, 0.0);
        let mut near = square(1.0, 0.0001);
        near[1].x = 0.0;
        near[2].x = 0.0;
        depth.add(1, &near, &QUAD, 0.0);
        let (mask, width) = made(
            &depth,
            &[BARE, BARE],
            &[layer(vec![Some(vec![1.0; 4]), None])],
        );
        let full = (0.2_f32 * 255.0).round() as u8;
        for y in [50, 75, 100] {
            for x in 75..90 {
                assert_eq!(mask.alpha[y * width + x], 0, "covered at {x}, {y}");
            }
            for x in 108..120 {
                assert_eq!(mask.alpha[y * width + x], full, "shows at {x}, {y}");
            }
        }
        // Whichever is drawn first.
        let mut other = buffer();
        other.add(1, &near, &QUAD, 0.0);
        other.add(0, &square(1.0, 0.0), &QUAD, 0.0);
        let (again, _) = made(
            &other,
            &[BARE, BARE],
            &[layer(vec![Some(vec![1.0; 4]), None])],
        );
        assert_eq!(again.alpha, mask.alpha);
    }

    #[test]
    fn a_feature_s_opening_stays_open_and_a_stray_texel_is_smoothed_away() {
        // A square with a bare one in front of its middle (an eye in its
        // socket).
        let mut depth = buffer();
        depth.add(0, &square(1.0, 0.0), &QUAD, 0.0);
        depth.add(1, &square(0.4, 0.5), &QUAD, 0.0);
        let (mask, width) = made(
            &depth,
            &[BARE, BARE],
            &[layer(vec![Some(vec![1.0; 4]), None])],
        );
        assert_eq!(mask.alpha[75 * width + 100], 0, "the opening isn't lit");
        let full = (0.2_f32 * 255.0).round() as u8;
        assert_eq!(mask.alpha[75 * width + 119], full, "the skin round it is");

        // One texel of cover alone (a sliver of far cloth through a gap)
        // is no hard dot: its strength is spread thin.
        let mut mask = Mask::default();
        let mut depth = buffer();
        let speck = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.05, 0.0, 0.0),
            Vec3::new(0.0, 0.05, 0.0),
        ];
        depth.add(0, &speck, &[0, 1, 2], 0.0);
        let lit = depth
            .seen()
            .iter()
            .filter(|seen| seen.surface != Seen::NOTHING)
            .count();
        assert_eq!(lit, 1, "one texel of cover");
        let triangle = Surface {
            indices: &[0, 1, 2],
            fine: &[],
        };
        mask.compose(&depth, &[triangle], &[layer(vec![Some(vec![1.0; 3])])], 0.5);
        let most = mask.alpha.iter().copied().max().unwrap();
        assert!(most <= full / 4, "a speck's strength is {most} of {full}");
    }

    #[test]
    fn weights_blend_across_a_triangle_and_the_edge_is_where_they_cross() {
        // Weight 1 on the left corners, 0 on the right: lit on the left
        // half only, the edge down the middle.
        let mut depth = buffer();
        depth.add(0, &square(1.0, 0.0), &QUAD, 0.0);
        let (mask, width) = made(
            &depth,
            &[BARE],
            &[layer(vec![Some(vec![1.0, 0.0, 0.0, 1.0])])],
        );
        let full = (0.2_f32 * 255.0).round() as u8;
        assert_eq!(mask.alpha[75 * width + 80], full);
        assert_eq!(mask.alpha[75 * width + 120], 0);
        let middle = mask.alpha[75 * width + 100];
        assert!(middle > 0 && middle < full, "the edge is soft: {middle}");
    }

    #[test]
    fn two_layers_lie_one_over_the_other_and_a_cleared_mask_is_empty() {
        let mut depth = buffer();
        depth.add(0, &square(1.0, 0.0), &QUAD, 0.0);
        let weights = || vec![Some(vec![1.0; 4])];
        let (mut mask, width) = made(&depth, &[BARE], &[layer(weights()), layer(weights())]);
        let both = ((0.2_f32 + 0.2 - 0.04) * 255.0).round() as u8;
        assert_eq!(mask.alpha[75 * width + 100], both);
        // Made again with nothing to light: every texel goes back to none.
        let changed = mask.compose(&depth, &[BARE], &[], 0.5);
        assert!(changed.is_some(), "the old texels are cleared");
        assert!(mask.alpha.iter().all(|alpha| *alpha == 0));
        assert_eq!(mask.compose(&depth, &[BARE], &[], 0.5), None);
    }

    #[test]
    fn a_feature_that_runs_off_the_view_does_not_fade_at_its_edge() {
        let mut depth = buffer();
        depth.add(0, &square(4.0, 0.0), &QUAD, 0.0);
        let (mask, width) = made(&depth, &[BARE], &[layer(vec![Some(vec![1.0; 4])])]);
        let full = (0.2_f32 * 255.0).round() as u8;
        assert_eq!(mask.alpha[0], full);
        assert_eq!(mask.alpha[75 * width + width - 1], full);
    }
}
