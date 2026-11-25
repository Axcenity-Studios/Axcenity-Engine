//! Editing on the character (refinement R2, direct manipulation; brief
//! §3.4, §4.3 and §5.3; `references/ux/patterns.md` section 3). The
//! viewport is a control as well as a view:
//!
//! - **Hover** highlights the feature under the pointer (a soft mask
//!   drawn over the preview, [`mask`], never a material change) and names
//!   it in the status line. What's under the pointer is found by a ray against the skin as
//!   it's drawn (`scene::ray_parts`: posed, skinned, the identity worn
//!   live), and which feature that is from the manifest's `features`
//!   (their anchors, measured at rest, and the body regions they name).
//! - **A click** selects the feature: the same [`Intent::PickFeature`] as
//!   the navigation's, so its page and its controls open; the camera
//!   frames it when it's small on the screen ([`Intent::FocusFeature`]).
//! - **Handles**, the manifest's `handles` for the selected feature, are
//!   drawn on the skin (each on the corner its anchor names, where the
//!   frame's pose puts it). A primary drag that starts on one edits: it
//!   sends the slider's own intents ([`Intent::SetParameter`] or, with
//!   Symmetry, [`Intent::SetMirrored`]) as one gesture, so a drag is one
//!   undo step and Esc takes it back. Any other drag turns the camera as
//!   before, Alt with a drag always does, and the right button pans.
//! - **Only what the camera sees** is drawn: the overlay is egui's, over
//!   the preview, with no depth of its own, so the character as posed is
//!   rasterized into a small buffer on the CPU ([`depth`]). The highlight
//!   is read from it (each texel's nearest surface, the body's or a worn
//!   part's), and its depth of the body alone hides a handle behind other
//!   skin (the far jaw angle behind the cheek); a hidden handle can't be
//!   grabbed.
//! - **The switch** ([`Intent::SetHandles`], in the toolbar) turns all
//!   of it off: the viewport is the camera only.
//!
//! A drag means what the view shows (ux patterns P5): each drive works in
//! the framings its `screen` names, along the direction its skin moves on
//! the screen there. How far is the studio's one rule for every numeric
//! control (RA1, `numeric.rs`): the whole range takes 110 points of drag
//! along that direction, however near the camera is; Shift a quarter,
//! Ctrl double. Two drives of one handle are solved together, each held
//! still until its own part of the drag passes [`DEAD_ZONE`].

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axcenity_assets::Model;
use axcenity_avatar::BaseManifest;
use axcenity_core::ClassName;
use axcenity_gameplay::description as applier;
use axcenity_render::glam::{Mat4, Vec2, Vec3};
use axcenity_scene::{CameraView, Corner, Ray};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

use self::depth::Depth;
use self::mask::{Layer, Mask, Surface};
use self::worn::WornMap;
use super::intent::{Gesture, Intent};
use super::numeric;
use super::selection::Level;
use super::stage::Orbit;
use super::{Category, CharacterWorkspace};
use crate::theme;

/// How near a press must be to a handle to take it (points).
const GRAB: f32 = 11.0;
/// A handle's radius on the screen (points).
const DOT: f32 = 6.0;
/// How far (points) a drive's own part of a drag goes before it moves, so
/// a sideways drag that wobbles doesn't nudge the height too.
const DEAD_ZONE: f32 = 8.0;
/// A feature clicked while smaller than this share of the view's height
/// (its extent from its middle) is framed; one already well in view isn't.
const SMALL: f32 = 0.06;
/// How long framing a feature takes.
const FOCUS_SECONDS: f32 = 0.25;
/// A ray that meets another part first (an eyeball, the underwear) still
/// picks the skin behind it within this many metres.
const THROUGH: f32 = 0.03;
/// How far from a feature's anchors (times its radius) a point still
/// picks it.
const REACH: f32 = 1.2;
/// Where a point's weight for a feature is at least this, it's the
/// feature's: lit when the feature is, and picked as it.
const EDGE: f32 = 0.5;
/// A face feature's tint reaches this many times its radius from each of
/// its anchors and handles...
const REACH_TINT: f32 = 1.0;
/// ...and is full within this share of that reach, fading to nothing at it.
const FULL: f32 = 0.7;
/// Skin whose normal meets the feature's own skin within this many metres
/// is inside a hollow (a nostril) and isn't tinted.
const HOLLOW: f32 = 0.01;
/// A face feature's shape that moves strongly (a third of its most or
/// more) over this many times the skin its feature's shapes usually do
/// (their median) moves the feature together with its surroundings: where
/// it is, not what it is (the mouth's height and the philtrum's length move
/// the whole muzzle, the jaw's shift the lower face). It's left out of the
/// tint.
const BROAD: f32 = 4.0;
/// A face feature whose shapes light more than this many times the skin
/// the features usually do (their median) is a whole part of the head (the
/// face's shape, the neck), not a place on it: it keeps all of its skin and
/// takes none from the features inside it, which are what is picked there
/// (the smallest lit feature under the pointer).
const WHOLE: f32 = 5.0;
/// Two drives of one handle whose screen lines are closer than this
/// (degrees) can't share a drag: the second gives way.
const APART: f32 = 25.0;

mod depth;
mod mask;
mod worn;

/// A handle on skin more than this many metres (of the file) behind the
/// skin the camera sees there is hidden.
const HIDDEN: f32 = 0.003;
/// A part that can't carry a feature's highlight itself (the base's own:
/// underwear, brows, lashes; or one with no skeleton to follow the body
/// by) counts this many metres farther than it is, so the skin just under
/// it is what's seen there and lights up. A skinned garment carries the
/// weights of the skin under it ([`WornMap`]) and counts where it is.
const WORN: f32 = 0.015;
/// A ray that meets a worn item first still picks the skin behind it
/// within this many metres: a coat stands well off the body it dresses.
const THROUGH_WORN: f32 = 0.25;
/// How far round a handle (texels of the depth buffer, about two points
/// each) skin that shows lets it show: a handle at the skin's outline is a
/// little behind the skin just inside it.
const HANDLE_SIGHT: i32 = 3;
/// A corner of the skin lights up as far as it faces the camera: fully
/// from this much of straight on, fading to nothing edge on (the inside of
/// a nostril, which faces away, never does).
const FACING: f32 = 0.3;
/// How strong the selected feature's highlight is: inside, and that much
/// more along its edge.
const SELECTED: [f32; 2] = [0.10, 0.10];
/// The hovered feature's, a little stronger.
const HOVERED: [f32; 2] = [0.18, 0.14];

/// A feature's weights on a worn model's points, primitive by primitive
/// ([`WornMap::weights`]).
type ClothWeights = Arc<Vec<Vec<f32>>>;

/// Direct manipulation's editor state (never saved, never undone).
pub struct Direct {
    /// Editing on the character: hover, click to select, handles. On
    /// unless the person turns it off.
    pub on: bool,
    /// The feature under the pointer.
    pub hovered: Option<String>,
    /// The handle under the pointer.
    hovered_handle: Option<String>,
    /// The current character's part of the viewport, last frame.
    pub view_rect: Option<Rect>,
    /// The camera last frame, to turn screen points into rays.
    view: Option<CameraView>,
    /// The handles drawn last frame, where.
    shown: Vec<Shown>,
    drag: Option<HandleDrag>,
    focus: Option<Focus>,
    /// What a body model's features and handles are, worked out once per
    /// model and base model.
    prepared: HashMap<(u64, String), Arc<Prepared>>,
    /// For each body model and worn model (by their keys): which of the
    /// body's corners each of the worn model's points lies over, worked
    /// out once (`None`: the worn model can't follow the body).
    worn_maps: HashMap<(u64, u64), Option<Arc<WornMap>>>,
    /// The same by the body's shape at rest ([`rest_print`], kept for each
    /// body model in `shapes`): a body with other skin hidden is another
    /// model with the same corners, and what lies over them is the same.
    worn_by_shape: HashMap<(u64, u64), Option<Arc<WornMap>>>,
    shapes: HashMap<u64, u64>,
    /// A feature's weights on a worn model's points ([`WornMap::weights`]),
    /// by the body's features (where they're kept), the worn model and
    /// the feature: worked out the first time they're asked for.
    cloth: HashMap<(usize, u64, String), ClothWeights>,
    /// A line for the status bar while hovering or dragging.
    pub status: Option<String>,
    /// Where the pointer and the camera were when `hovered` was found.
    picked_at: Option<(Pos2, Orbit)>,
    /// What the camera saw of the character when it was last looked at,
    /// kept while nothing it's made from changes.
    sight: Option<Arc<Sight>>,
    /// How many times `sight` was made.
    sights: u64,
    /// The highlight, and what it was last made for.
    mask: Mask,
    mask_key: Option<MaskKey>,
}

impl Default for Direct {
    fn default() -> Self {
        Self {
            on: true,
            hovered: None,
            hovered_handle: None,
            view_rect: None,
            view: None,
            shown: Vec::new(),
            drag: None,
            focus: None,
            prepared: HashMap::new(),
            worn_maps: HashMap::new(),
            worn_by_shape: HashMap::new(),
            shapes: HashMap::new(),
            cloth: HashMap::new(),
            status: None,
            picked_at: None,
            sight: None,
            sights: 0,
            mask: Mask::default(),
            mask_key: None,
        }
    }
}

/// A handle as drawn: where, and each drive that works in this view with
/// how many points the skin moves on the screen per unit of its value.
#[derive(Clone, Debug)]
struct Shown {
    id: String,
    at: Pos2,
    live: Vec<(String, Vec2)>,
    at_limit: bool,
}

/// A handle being dragged.
#[derive(Clone, Debug)]
struct HandleDrag {
    handle: String,
    /// Where the pointer was last frame.
    from: Pos2,
    /// The drag so far (points; scaled by Shift and Ctrl as it went).
    travel: numeric::Travel<Vec2>,
    mirrored: bool,
    drives: Vec<DragDrive>,
    /// For each drive (its index), how much its value changes per point
    /// of drag (the inverse of the drives' screen moves).
    solve: Vec<Vec2>,
    /// Esc took it back: the rest of the drag does nothing.
    cancelled: bool,
    /// Whether any change was sent (so the release closes the step).
    sent: bool,
}

#[derive(Clone, Debug)]
struct DragDrive {
    parameter: String,
    start: f32,
    range: [f32; 2],
    /// Points on the screen per unit of value.
    gain: f32,
    /// Its own part of the drag has passed the dead zone.
    engaged: bool,
    last: f32,
}

/// The camera on its way to a feature.
#[derive(Clone, Copy, Debug)]
struct Focus {
    from: Orbit,
    to: Orbit,
    started: Instant,
}

/// A body model's features and handles, at rest in its fitted space.
struct Prepared {
    features: Vec<PreparedFeature>,
    handles: Vec<PreparedHandle>,
    /// Turns a step in the fitted space into the file's metres
    /// (`scene::to_metres`).
    metres: Vec3,
}

struct PreparedFeature {
    id: String,
    label: String,
    /// Each anchor: its bone (by index) and its point at rest.
    anchors: Vec<(usize, Vec3)>,
    /// How far round an anchor counts, in the file's metres.
    radius: f32,
    /// The body regions (map indices) that pick it.
    regions: Vec<u8>,
    /// How much each corner of each primitive belongs to it (the tint).
    weights: Vec<Vec<f32>>,
    /// Its tint is the skin its shapes move (a face feature's).
    shaped: bool,
}

/// A feature's tint, as [`tint_weights`] works it out.
struct Tint {
    weights: Vec<Vec<f32>>,
    shaped: bool,
}

impl Tint {
    fn of(weights: Vec<Vec<f32>>, shaped: bool) -> Self {
        Self { weights, shaped }
    }
}

struct PreparedHandle {
    id: String,
    feature: String,
    label: String,
    mirror: Option<String>,
    corner: Corner,
    /// The anchor's bone (by index) for turning a drive's axis as posed.
    bone: Option<usize>,
    drives: Vec<PreparedDrive>,
}

struct PreparedDrive {
    parameter: String,
    range: [f32; 2],
    /// The drive's travel along its axis, in fitted units, on the default
    /// frame; `frames` holds it on a frame whose own shapes move it
    /// otherwise (`Drive::frames`).
    travel: Vec3,
    frames: Vec<(f32, Vec3)>,
    /// Its screen direction (x right, y up) in each framing it works in.
    screen: Vec<(String, Vec2)>,
}

impl PreparedDrive {
    /// Its travel on `frame` (`Drive::on_frame`).
    fn travel_on(&self, frame: f32) -> Vec3 {
        self.frames
            .iter()
            .filter(|(at, _)| (at - frame).abs() < 0.5)
            .min_by(|a, b| (a.0 - frame).abs().total_cmp(&(b.0 - frame).abs()))
            .map_or(self.travel, |(_, travel)| *travel)
    }
}

/// How much each corner of each of `model`'s primitives belongs to a
/// feature, for its highlight and for picking it (ux patterns P6: from
/// data the build already has, no painted map). The feature's edge is
/// where this crosses [`EDGE`].
///
/// - A feature with body regions is its regions.
/// - One whose parameters drive shapes (the face's) is the skin those
///   shapes move strongly: each shape on its own (a bridge's shape moves
///   the bridge, a tip's the tip), full where it moves the skin a third of
///   its most and nothing where it moves it a sixth of that, near the
///   feature's anchors and handles ([`REACH_TINT`], [`FULL`]). A shape
///   that moves far more skin than its feature's others ([`BROAD`]) is
///   left out. The eyes' height, depth and spacing shapes move the whole
///   socket as one, out to the brows and the upper cheek, so the shapes
///   alone can't tell the eyes from those: [`compete`] gives that skin to
///   the brows and cheeks.
///   Smoothed over the mesh twice, so the edge runs smoothly across the
///   triangles rather than along them.
/// - Any other fades out from its anchors.
///
/// Only the skin carries weight, not the inside of the mouth. Distances
/// are in the file's metres (`radius` is), not in the fitted box.
fn tint_weights(
    model: &Model,
    base: &BaseManifest,
    feature: &str,
    anchors: &[(Vec3, f32)],
    regions: &[u8],
) -> Tint {
    let skin = |primitive: &axcenity_assets::Primitive| {
        primitive.material.shading == axcenity_assets::ModelShading::Skin
    };
    let any_skin = model.primitives.iter().any(skin);
    let tinted = |primitive: &axcenity_assets::Primitive| !any_skin || skin(primitive);
    let metres = axcenity_scene::to_metres(model);
    // How far a point at rest is from the feature's nearest anchor, in
    // shares of that anchor's reach (metres over metres).
    let from_anchors = |position: &[f32; 3]| {
        anchors
            .iter()
            .map(|(at, reach)| ((*at - Vec3::from(*position)) * metres).length() / reach)
            .fold(f32::INFINITY, f32::min)
    };
    let shapes: Vec<&str> = base
        .parameters
        .iter()
        .filter(|parameter| {
            base.feature_of(parameter)
                .is_some_and(|owner| owner.id == feature)
        })
        .flat_map(|parameter| parameter.shapes.iter().map(|driver| driver.shape.as_str()))
        .collect();
    let mut weights: Vec<Vec<f32>> = model
        .primitives
        .iter()
        .map(|primitive| vec![0.0; primitive.positions.len()])
        .collect();
    if !regions.is_empty() {
        for (primitive, out) in model.primitives.iter().zip(&mut weights) {
            if !tinted(primitive) {
                continue;
            }
            for (weight, region) in out.iter_mut().zip(&primitive.regions) {
                if regions.contains(region) {
                    *weight = 1.0;
                }
            }
        }
        return Tint::of(weights, false);
    }
    // Each named shape's largest move of the skin, then each corner's
    // strongest share of its shapes' largest moves.
    let named = |target: u16| {
        model
            .morph_targets
            .get(usize::from(target))
            .is_some_and(|target| shapes.contains(&target.name.as_str()))
    };
    let mut most: HashMap<u16, f32> = HashMap::new();
    for primitive in model.primitives.iter().filter(|p| tinted(p)) {
        for morph in primitive.morphs.iter().filter(|m| named(m.target)) {
            let largest = morph
                .positions
                .iter()
                .map(|delta| (Vec3::from(*delta) * metres).length())
                .fold(0.0, f32::max);
            let entry = most.entry(morph.target).or_default();
            *entry = entry.max(largest);
        }
    }
    // How much skin each shape moves strongly (square metres): the
    // triangles whose corners all move a third of its most or more.
    let mut area: HashMap<u16, f32> = HashMap::new();
    for primitive in model.primitives.iter().filter(|p| tinted(p)) {
        for morph in primitive.morphs.iter().filter(|m| named(m.target)) {
            let largest = most[&morph.target];
            let mut strong = vec![false; primitive.positions.len()];
            for (corner, delta) in morph.corners.iter().zip(&morph.positions) {
                if let Some(strong) = strong.get_mut(*corner as usize) {
                    *strong = (Vec3::from(*delta) * metres).length() >= largest / 3.0;
                }
            }
            let at = |corner: u32| Vec3::from(primitive.positions[corner as usize]) * metres;
            let moved: f32 = primitive
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .filter(|triangle| triangle.iter().all(|c| strong[*c as usize]))
                .map(|&[a, b, c]| (at(b) - at(a)).cross(at(c) - at(a)).length() / 2.0)
                .sum();
            *area.entry(morph.target).or_default() += moved;
        }
    }
    let mut areas: Vec<f32> = area.values().copied().filter(|a| *a > 0.0).collect();
    areas.sort_by(f32::total_cmp);
    let usual = areas.get(areas.len() / 2).copied().unwrap_or(0.0);
    let broad = |target: u16| area.get(&target).is_some_and(|a| *a > BROAD * usual);
    if most.values().any(|most| *most > 0.0) {
        for (primitive, out) in model.primitives.iter().zip(&mut weights) {
            if !tinted(primitive) {
                continue;
            }
            for morph in primitive
                .morphs
                .iter()
                .filter(|m| named(m.target) && !broad(m.target))
            {
                let largest = most[&morph.target].max(1e-9);
                for (corner, delta) in morph.corners.iter().zip(&morph.positions) {
                    let share = (Vec3::from(*delta) * metres).length() / largest;
                    // 0 at a sixth of the most, 1 at a third.
                    let strength = ((share - 1.0 / 6.0) * 6.0).clamp(0.0, 1.0);
                    if let Some(weight) = out.get_mut(*corner as usize) {
                        *weight = weight.max(strength);
                    }
                }
            }
            for (weight, position) in out.iter_mut().zip(&primitive.positions) {
                let near = (1.0 - from_anchors(position)) / (1.0 - FULL);
                *weight *= near.clamp(0.0, 1.0);
            }
            smooth(&primitive.positions, &primitive.indices, out, 2);
            hollows_out(primitive, out, metres);
        }
        return Tint::of(weights, true);
    }
    for (primitive, out) in model.primitives.iter().zip(&mut weights) {
        if !tinted(primitive) {
            continue;
        }
        for (weight, position) in out.iter_mut().zip(&primitive.positions) {
            let t = from_anchors(position) / 1.4;
            *weight = (1.0 - t * t).max(0.0);
        }
    }
    Tint::of(weights, false)
}

/// Face features share the skin between them: where another's tint is
/// stronger, a feature's fades, and its edge runs where the two are
/// equal. The eyes' shapes move the brows and the upper cheek too, but the
/// brows' and the cheeks' shapes move them more. Smoothed once more after,
/// so that shared edge runs smoothly too. A feature that is a whole part
/// of the head ([`WHOLE`]) shares with none: against it every smaller
/// feature sat at exactly the edge's weight wherever both were full, and
/// was hardly lit or picked (the human base's nose, eyes and cheeks under
/// its face shape, found 2026-11-25).
fn compete(model: &Model, mut features: Vec<PreparedFeature>) -> Vec<PreparedFeature> {
    let before: Vec<Option<Vec<Vec<f32>>>> = features
        .iter()
        .map(|feature| feature.shaped.then(|| feature.weights.clone()))
        .collect();
    // The skin each lights before sharing (the file's units, squared).
    let lit: Vec<f32> = before
        .iter()
        .map(|weights| {
            weights
                .as_ref()
                .map_or(0.0, |weights| lit_area(model, weights))
        })
        .collect();
    let mut usual: Vec<f32> = lit.iter().copied().filter(|area| *area > 0.0).collect();
    usual.sort_by(f32::total_cmp);
    let usual = usual.get(usual.len() / 2).copied().unwrap_or(0.0);
    let whole = |feature: usize| lit[feature] > WHOLE * usual;
    let rivals = |a: usize, b: usize| !whole(a) && !whole(b);
    for (index, feature) in features.iter_mut().enumerate() {
        if !feature.shaped {
            continue;
        }
        for (primitive, weights) in feature.weights.iter_mut().enumerate() {
            for (corner, weight) in weights.iter_mut().enumerate() {
                let other = before
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != index && rivals(index, *other))
                    .filter_map(|(_, weights)| weights.as_ref()?.get(primitive)?.get(corner))
                    .fold(0.0_f32, |most, weight| most.max(*weight));
                // EDGE where the two are equal, full a quarter above.
                *weight = weight.min((EDGE + 2.0 * (*weight - other)).clamp(0.0, 1.0));
            }
            if let Some(mesh) = model.primitives.get(primitive) {
                smooth(&mesh.positions, &mesh.indices, weights, 1);
            }
        }
    }
    features
}

/// The area of `model`'s triangles whose corners all have at least
/// [`EDGE`] of `weights` (a weight a corner of each primitive).
fn lit_area(model: &Model, weights: &[Vec<f32>]) -> f32 {
    model
        .primitives
        .iter()
        .zip(weights)
        .map(|(primitive, weights)| {
            let lit = |corner: u32| weights.get(corner as usize).is_some_and(|w| *w >= EDGE);
            let at = |corner: u32| {
                primitive
                    .positions
                    .get(corner as usize)
                    .map_or(Vec3::ZERO, |p| Vec3::from(*p))
            };
            primitive
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .filter(|triangle| triangle.iter().all(|corner| lit(*corner)))
                .map(|&[a, b, c]| (at(b) - at(a)).cross(at(c) - at(a)).length() / 2.0)
                .sum::<f32>()
        })
        .sum()
}

/// Takes the tint off the skin inside a hollow: a corner whose normal
/// meets the feature's own skin within [`HOLLOW`] (the inside of a nostril
/// faces its other wall, as do the eyes' corners; open skin faces the
/// air). A tinted hollow showed as a pale patch in a dark opening.
fn hollows_out(primitive: &axcenity_assets::Primitive, weights: &mut [f32], metres: Vec3) {
    let at = |corner: u32| {
        primitive
            .positions
            .get(corner as usize)
            .map(|p| Vec3::from(*p) * metres)
    };
    // The feature's triangles, by 2 cm cells of their middles.
    const CELL: f32 = 0.02;
    let cell = |point: Vec3| (point / CELL).floor().as_ivec3();
    let mut cells: HashMap<axcenity_render::glam::IVec3, Vec<[Vec3; 3]>> = HashMap::new();
    for triangle in primitive.indices.as_chunks::<3>().0 {
        if triangle
            .iter()
            .all(|corner| weights.get(*corner as usize).is_none_or(|w| *w <= 0.0))
        {
            continue;
        }
        let [Some(a), Some(b), Some(c)] = triangle.map(at) else {
            continue;
        };
        cells
            .entry(cell((a + b + c) / 3.0))
            .or_default()
            .push([a, b, c]);
    }
    let hollow: Vec<usize> = weights
        .iter()
        .enumerate()
        .filter(|(_, weight)| **weight > 0.0)
        .filter_map(|(corner, _)| {
            let point = at(corner as u32)?;
            let normal = (Vec3::from(*primitive.normals.get(corner)?) / metres).normalize_or_zero();
            let origin = point + normal * 0.0005;
            let home = cell(point);
            let mut near = (-1..=1).flat_map(|x| {
                (-1..=1).flat_map(move |y| {
                    (-1..=1).map(move |z| home + axcenity_render::glam::IVec3::new(x, y, z))
                })
            });
            near.any(|key| {
                cells.get(&key).is_some_and(|triangles| {
                    triangles.iter().any(|[a, b, c]| {
                        axcenity_scene::triangle_hit(origin, normal, *a, *b, *c)
                            .is_some_and(|(distance, _, _)| distance < HOLLOW)
                    })
                })
            })
            .then_some(corner)
        })
        .collect();
    for corner in hollow {
        weights[corner] = 0.0;
    }
}

/// Averages each corner's weight with its neighbours' (the corners it
/// shares a triangle with), `passes` times. Corners at one place (the two
/// sides of a seam) are kept equal, so the feature's edge crosses a seam
/// without a gap.
fn smooth(positions: &[[f32; 3]], indices: &[u32], weights: &mut [f32], passes: usize) {
    let count = weights.len().min(positions.len());
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); count];
    for triangle in indices.as_chunks::<3>().0 {
        for k in 0..3 {
            let (a, b) = (triangle[k], triangle[(k + 1) % 3]);
            if (a as usize) < count && (b as usize) < count {
                neighbours[a as usize].push(b);
                neighbours[b as usize].push(a);
            }
        }
    }
    let mut places: HashMap<[u32; 3], Vec<usize>> = HashMap::new();
    for (corner, position) in positions.iter().take(count).enumerate() {
        places
            .entry(position.map(f32::to_bits))
            .or_default()
            .push(corner);
    }
    let seams: Vec<Vec<usize>> = places.into_values().filter(|at| at.len() > 1).collect();
    for _ in 0..passes {
        let before = weights.to_vec();
        for (corner, around) in neighbours.iter().enumerate() {
            if around.is_empty() {
                continue;
            }
            let sum: f32 = around.iter().map(|other| before[*other as usize]).sum();
            weights[corner] = 0.5 * before[corner] + 0.5 * sum / around.len() as f32;
        }
        for seam in &seams {
            let mean = seam.iter().map(|corner| weights[*corner]).sum::<f32>() / seam.len() as f32;
            for corner in seam {
                weights[*corner] = mean;
            }
        }
    }
}

impl Prepared {
    fn new(model: &Model, base: &BaseManifest) -> Self {
        let rig = model.rig.as_deref();
        let bone_of = |name: &str| rig.and_then(|rig| rig.bone(name));
        let region_index = |id: &str| base.region(id).and_then(|region| region.map_index);
        let features = base
            .features
            .iter()
            .map(|feature| {
                let anchors: Vec<(usize, Vec3)> = feature
                    .anchors
                    .iter()
                    .filter_map(|anchor| {
                        let at = axcenity_scene::rest_anchor(
                            model,
                            &anchor.bone,
                            Vec3::from(anchor.offset),
                        )?;
                        Some((bone_of(&anchor.bone)?, at))
                    })
                    .collect();
                let regions: Vec<u8> = feature
                    .regions
                    .iter()
                    .filter_map(|id| region_index(id))
                    .collect();
                let radius = feature.radius;
                // The tint reaches out from the feature's anchors and from
                // its handles, which sit on its parts (an anchor is the
                // middle of what the feature moves: the nose's is off to one
                // side, so its reach alone would take half a nose).
                let mut reach: Vec<(Vec3, f32)> = anchors
                    .iter()
                    .map(|(_, at)| (*at, radius * REACH_TINT))
                    .collect();
                reach.extend(
                    base.handles
                        .iter()
                        .filter(|handle| handle.feature == feature.id)
                        .filter_map(|handle| {
                            axcenity_scene::rest_anchor(
                                model,
                                &handle.anchor.bone,
                                Vec3::from(handle.anchor.offset),
                            )
                            .map(|at| (at, radius * REACH_TINT))
                        }),
                );
                let Tint { weights, shaped } =
                    tint_weights(model, base, &feature.id, &reach, &regions);
                PreparedFeature {
                    id: feature.id.clone(),
                    label: feature.label.clone(),
                    anchors,
                    radius,
                    regions,
                    weights,
                    shaped,
                }
            })
            .collect();
        let features = compete(model, features);
        let handles = base
            .handles
            .iter()
            .filter_map(|handle| {
                let offset = Vec3::from(handle.anchor.offset);
                let at = axcenity_scene::rest_anchor(model, &handle.anchor.bone, offset)?;
                let corner = axcenity_scene::nearest_corner(model, at)?;
                let fit = rig.map_or(Mat4::IDENTITY, |rig| rig.fit);
                let drives = handle
                    .drives
                    .iter()
                    .filter_map(|drive| {
                        let parameter = base.parameter(&drive.parameter)?;
                        let sensitivity = drive.sensitivity.unwrap_or(1.0);
                        Some(PreparedDrive {
                            parameter: drive.parameter.clone(),
                            range: parameter.range,
                            travel: fit.transform_vector3(Vec3::from(drive.axis) * drive.travel)
                                / sensitivity,
                            frames: drive
                                .frames
                                .iter()
                                .map(|f| {
                                    let (axis, travel) = drive.on_frame(f.frame);
                                    (
                                        f.frame,
                                        fit.transform_vector3(Vec3::from(axis) * travel)
                                            / sensitivity,
                                    )
                                })
                                .collect(),
                            screen: drive
                                .screen
                                .iter()
                                .map(|(view, [x, y])| (view.clone(), Vec2::new(*x, *y)))
                                .collect(),
                        })
                    })
                    .collect();
                let label = handle.label.clone().unwrap_or_else(|| {
                    handle
                        .drives
                        .first()
                        .and_then(|drive| base.parameter(&drive.parameter))
                        .map_or_else(|| handle.id.clone(), |p| p.label.clone())
                });
                Some(PreparedHandle {
                    id: handle.id.clone(),
                    feature: handle.feature.clone(),
                    label,
                    mirror: handle.mirror.clone(),
                    corner,
                    bone: bone_of(&handle.anchor.bone),
                    drives,
                })
            })
            .collect();
        Self {
            features,
            handles,
            metres: axcenity_scene::to_metres(model),
        }
    }

    fn feature(&self, id: &str) -> Option<&PreparedFeature> {
        self.features.iter().find(|feature| feature.id == id)
    }

    fn handle(&self, id: &str) -> Option<&PreparedHandle> {
        self.handles.iter().find(|handle| handle.id == id)
    }

    /// The feature `hit` (a ray's hit on `model`'s skin) belongs to: of the
    /// features whose weight there is at least [`EDGE`] (inside their
    /// highlight), the smallest, so what's picked is what hovering shows
    /// (the nose, not the head round it). Where no tint reaches (the inside of
    /// an eye socket behind an eyeball), [`Self::feature_by_anchor`].
    fn feature_hit(
        &self,
        hit: &axcenity_scene::MeshHit,
        model: &Model,
    ) -> Option<&PreparedFeature> {
        let corners = hit.corners(model)?;
        let weight = |feature: &PreparedFeature| {
            let Some(weights) = feature.weights.get(hit.primitive) else {
                return 0.0;
            };
            corners
                .iter()
                .zip(hit.weights)
                .map(|(corner, share)| share * weights.get(corner.index).copied().unwrap_or(0.0))
                .sum::<f32>()
        };
        self.features
            .iter()
            .filter(|feature| weight(feature) >= EDGE)
            .min_by(|a, b| a.radius.total_cmp(&b.radius))
            .or_else(|| self.feature_by_anchor(hit.at_rest(model)?, hit.region))
    }

    /// The feature a point of the skin at rest belongs to by its anchors
    /// and regions, where no tint says. A feature that names body regions
    /// is picked by them; one that doesn't, by its anchors: within
    /// [`REACH`] of its radius (in the file's metres), the nearest for its
    /// size. A smaller feature near its anchors beats the region it lies in;
    /// with no region to go by, a feature more than twice another's size
    /// gives way to it.
    fn feature_by_anchor(&self, point: Vec3, region: Option<u8>) -> Option<&PreparedFeature> {
        let near = |feature: &PreparedFeature| {
            feature
                .anchors
                .iter()
                .map(|(_, at)| ((*at - point) * self.metres).length())
                .fold(f32::INFINITY, f32::min)
                / feature.radius.max(1e-6)
        };
        let by_region = self
            .features
            .iter()
            .filter(|f| region.is_some_and(|region| f.regions.contains(&region)))
            .min_by(|a, b| a.radius.total_cmp(&b.radius));
        let by_anchor: Vec<(f32, &PreparedFeature)> = self
            .features
            .iter()
            .filter(|f| f.regions.is_empty())
            .map(|f| (near(f), f))
            .filter(|(score, _)| *score <= REACH)
            .collect();
        match by_region {
            Some(region) => nearest(
                by_anchor
                    .into_iter()
                    .filter(|(_, f)| f.radius < region.radius),
            )
            .or(Some(region)),
            None => {
                let least = by_anchor
                    .iter()
                    .map(|(_, f)| f.radius)
                    .fold(f32::INFINITY, f32::min);
                nearest(
                    by_anchor
                        .into_iter()
                        .filter(|(_, f)| f.radius <= 2.0 * least),
                )
            }
        }
    }
}

/// The candidate with the least score.
fn nearest<'a>(
    candidates: impl Iterator<Item = (f32, &'a PreparedFeature)>,
) -> Option<&'a PreparedFeature> {
    candidates
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, feature)| feature)
}

impl Direct {
    /// Where the camera is going while it frames a feature.
    #[cfg(test)]
    pub fn focus_goal(&self) -> Option<Orbit> {
        self.focus.map(|focus| focus.to)
    }

    /// Whether a handle is being dragged.
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    fn shown(&self, id: &str) -> Option<&Shown> {
        self.shown.iter().find(|shown| shown.id == id)
    }

    /// The handle drawn nearest `point`, within reach.
    fn handle_near(&self, point: Pos2) -> Option<&Shown> {
        self.shown
            .iter()
            .map(|shown| (shown.at.distance(point), shown))
            .filter(|(distance, _)| *distance <= GRAB)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, shown)| shown)
    }

    /// Nothing is under the pointer until it's looked for again.
    pub(super) fn forget_hover(&mut self) {
        self.hovered = None;
        self.picked_at = None;
    }

    /// The camera moved by hand: framing stops where it is.
    pub(super) fn stop_focus(&mut self) {
        self.focus = None;
    }

    /// Moves the camera on toward the feature being framed.
    pub(super) fn step_focus(&mut self, orbit: &mut Orbit, now: Instant) {
        let Some(focus) = self.focus else {
            return;
        };
        let t = (now.duration_since(focus.started).as_secs_f32() / FOCUS_SECONDS).min(1.0);
        // Ease in and out.
        let t = t * t * (3.0 - 2.0 * t);
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        *orbit = Orbit {
            yaw: focus.from.yaw,
            pitch: focus.from.pitch,
            distance: lerp(focus.from.distance, focus.to.distance),
            target: focus.from.target.lerp(focus.to.target, t),
        };
        if t >= 1.0 {
            self.focus = None;
        }
    }
}

/// The framing a camera turned `yaw` degrees round the character is in,
/// by the manifest's names: within 22.5 degrees of the front or the back,
/// that; within 22.5 of a profile, `side`; between, `three_quarter`.
fn framing(yaw: f32) -> &'static str {
    let from_front = yaw.rem_euclid(360.0);
    let from_front = from_front.min(360.0 - from_front);
    match from_front {
        a if a < 22.5 => "front",
        a if a < 67.5 => "three_quarter",
        a if a < 112.5 => "side",
        a if a < 157.5 => "three_quarter",
        _ => "back",
    }
}

/// A screen point in `rect` for a point of the renderer's normalized view
/// (x and y from -1 to 1, y up).
fn to_rect(rect: Rect, ndc: Vec2) -> Pos2 {
    egui::pos2(
        rect.center().x + ndc.x * rect.width() * 0.5,
        rect.center().y - ndc.y * rect.height() * 0.5,
    )
}

fn from_rect(rect: Rect, point: Pos2) -> Vec2 {
    Vec2::new(
        (point.x - rect.center().x) / (rect.width() * 0.5),
        (rect.center().y - point.y) / (rect.height() * 0.5),
    )
}

/// What tells one body's corners at rest from another's: where they are.
fn rest_print(model: &Model) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for primitive in &model.primitives {
        primitive.positions.len().hash(&mut hasher);
        for position in &primitive.positions {
            position.map(f32::to_bits).hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// A part the character wears, as drawn this frame.
struct WornNow {
    model: Arc<Model>,
    posed: Option<Arc<axcenity_assets::Posed>>,
    transform: Mat4,
    /// Its corners on the body, when it follows the body.
    map: Option<Arc<WornMap>>,
}

type BodyNow = (
    axcenity_core::InstanceId,
    Arc<Model>,
    Option<Arc<axcenity_assets::Posed>>,
    Mat4,
);

/// What the camera sees of the character: made once for a camera, a pose
/// and what's worn, and kept until one of them changes.
struct Sight {
    made_for: SightKey,
    /// Which making this is ([`Direct::sights`]).
    stamp: u64,
    /// The body alone, its depth only: what hides a handle.
    skin: Depth,
    /// The body and what it wears, with what each texel shows. Its
    /// surfaces are the body's primitives in order, then each worn part's.
    all: Depth,
}

/// What a [`Sight`] is made from: the view, and each part drawn (the body
/// first) by its model, its pose ([`pose_print`]), where it stands and
/// whether it carries the highlight.
#[derive(Clone, Debug, PartialEq)]
struct SightKey {
    rect: Rect,
    orbit: Orbit,
    parts: Vec<(u64, Option<u64>, [f32; 16], bool)>,
}

/// What tells one pose of a model from another: its joints and how far it
/// leans toward each blend shape, which are all its corners are drawn
/// from. (A pose's own version is new every frame, even standing still.)
fn pose_print(posed: &axcenity_assets::Posed) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for joint in posed.joints() {
        joint.to_cols_array().map(f32::to_bits).hash(&mut hasher);
    }
    for weight in posed.morph_weights() {
        weight.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// What the highlight was made for: a [`Sight`] (its stamp), the body's
/// features (by where they're kept), and each feature lit with its
/// strengths.
#[derive(Clone, Debug, PartialEq)]
struct MaskKey {
    sight: u64,
    prepared: usize,
    lit: Vec<(String, [f32; 2])>,
}

/// The studio's category for a manifest category.
fn category_of(data: axcenity_avatar::Category) -> Option<Category> {
    Category::ALL
        .into_iter()
        .find(|category| category.data() == Some(data))
}

impl CharacterWorkspace {
    /// The preview's body, its model, its pose this frame and where it
    /// stands.
    fn body_now(&self) -> Option<BodyNow> {
        let body = applier::body_of(&self.world, self.character?)?;
        let mesh = self
            .world
            .get(body)?
            .property("MeshId")?
            .as_str()?
            .to_owned();
        let model = self.assets.model_blocking(&mesh)?;
        let posed = self
            .poses()
            .get(&body)
            .filter(|posed| posed.model_key == model.key())
            .cloned();
        let transform = axcenity_scene::placement(&self.world, body)?;
        Some((body, model, posed, transform))
    }

    /// What the character wears over `body` (whose model is `model`), as
    /// drawn this frame: the base's own parts and each worn item's.
    fn worn_now(&mut self, body: axcenity_core::InstanceId, model: &Model) -> Vec<WornNow> {
        let mut worn = Vec::new();
        for part in self
            .world
            .all_below(body, |class| class == ClassName::MeshPart)
        {
            let Some(transform) = axcenity_scene::placement(&self.world, part) else {
                continue;
            };
            let Some(worn_model) = self
                .world
                .get(part)
                .and_then(|instance| instance.property("MeshId")?.as_str().map(str::to_owned))
                .and_then(|mesh| self.assets.model_blocking(&mesh))
            else {
                continue;
            };
            let posed = self
                .poses()
                .get(&part)
                .filter(|posed| posed.model_key == worn_model.key())
                .cloned();
            // Only what the description wears carries a feature's tint: the
            // base's own parts (eyes, teeth, lashes) are no clothing.
            let item = self.world.get(part).is_some_and(|instance| {
                instance
                    .attribute(applier::DESCRIPTION_PART)
                    .and_then(|value| value.as_str())
                    .is_some_and(|key| !key.starts_with("base:"))
            });
            let map = if item {
                self.worn_map(model, &worn_model)
            } else {
                None
            };
            worn.push(WornNow {
                model: worn_model,
                posed,
                transform,
                map,
            });
        }
        worn
    }

    /// `worn`'s points on the body `model` ([`WornMap`]), worked out the
    /// first time the two are seen together; `None` when `worn` can't
    /// follow the body.
    fn worn_map(&mut self, model: &Model, worn: &Model) -> Option<Arc<WornMap>> {
        let key = (model.key(), worn.key());
        if let Some(map) = self.direct.worn_maps.get(&key) {
            return map.clone();
        }
        // What is no longer worn is forgotten with the rest.
        if self.direct.worn_maps.len() > 64 {
            self.direct.worn_maps.clear();
        }
        if self.direct.worn_by_shape.len() > 24 {
            self.direct.worn_by_shape.clear();
        }
        if self.direct.shapes.len() > 64 {
            self.direct.shapes.clear();
        }
        let shape = *self
            .direct
            .shapes
            .entry(model.key())
            .or_insert_with(|| rest_print(model));
        let map = self
            .direct
            .worn_by_shape
            .entry((shape, worn.key()))
            .or_insert_with(|| WornMap::new(model, worn).map(Arc::new))
            .clone();
        self.direct.worn_maps.insert(key, map.clone());
        map
    }

    /// `feature`'s weights on the points of worn model `worn`
    /// ([`WornMap::weights`]), kept from the first time they're asked
    /// for (a primitive the feature doesn't reach at all keeps none: most
    /// features are nowhere near most garments). `prepared` is where the
    /// body's features are kept.
    fn cloth(
        &mut self,
        prepared: &Arc<Prepared>,
        feature: &PreparedFeature,
        worn: &Model,
        map: &WornMap,
    ) -> ClothWeights {
        // What is no longer worn is forgotten with the rest.
        if self.direct.cloth.len() > 512 {
            self.direct.cloth.clear();
        }
        let key = (
            Arc::as_ptr(prepared) as usize,
            worn.key(),
            feature.id.clone(),
        );
        let weights = self.direct.cloth.entry(key).or_insert_with(|| {
            let mut weights = map.weights(&feature.weights);
            for weights in &mut weights {
                if weights.iter().all(|weight| *weight <= 0.0) {
                    *weights = Vec::new();
                }
            }
            Arc::new(weights)
        });
        Arc::clone(weights)
    }

    /// The feature lit at `hit` on worn model `worn`: of the features
    /// whose weight there is at least [`EDGE`], the smallest, as on the
    /// skin ([`Prepared::feature_hit`]). What's picked on cloth is what's
    /// lit on it.
    fn feature_on_cloth(
        &mut self,
        prepared: &Arc<Prepared>,
        worn: &Model,
        map: &WornMap,
        hit: &axcenity_scene::MeshHit,
    ) -> Option<String> {
        let corners = worn
            .primitives
            .get(hit.primitive)?
            .indices
            .get(hit.triangle * 3..hit.triangle * 3 + 3)?;
        let corners = [corners[0], corners[1], corners[2]];
        let fine = map.fine(hit.primitive).get(hit.triangle).copied();
        let mut best: Option<&PreparedFeature> = None;
        for feature in &prepared.features {
            let weights = self.cloth(prepared, feature, worn, map);
            let Some(weights) = weights.get(hit.primitive) else {
                continue;
            };
            if worn::blend(weights, corners, hit.weights, fine) >= EDGE
                && best.is_none_or(|best| feature.radius < best.radius)
            {
                best = Some(feature);
            }
        }
        best.map(|feature| feature.id.clone())
    }

    /// The body model's features and handles for the description's base.
    fn prepared(&mut self, model: &Model) -> Option<Arc<Prepared>> {
        let base_id = self.description.base.clone();
        let key = (model.key(), base_id.clone());
        if let Some(prepared) = self.direct.prepared.get(&key) {
            return Some(Arc::clone(prepared));
        }
        let library = self.library();
        let base = library.base(&base_id)?;
        let prepared = Arc::new(Prepared::new(model, base));
        self.direct.prepared.insert(key, Arc::clone(&prepared));
        Some(prepared)
    }

    /// A point of the body's skeleton at rest (fitted space) where the
    /// frame's pose carries it, in the world.
    fn carried(
        posed: Option<&axcenity_assets::Posed>,
        model: &Model,
        transform: Mat4,
        bone: usize,
        at: Vec3,
    ) -> Vec3 {
        let joint = model
            .rig
            .as_deref()
            .and_then(|rig| rig.joints.iter().position(|joint| joint.bone == bone));
        let moved = match (posed, joint) {
            (Some(posed), Some(joint)) => posed
                .joints()
                .get(joint)
                .map_or(at, |matrix| matrix.transform_point3(at)),
            _ => at,
        };
        transform.transform_point3(moved)
    }

    /// Where `feature`'s anchors are in the world, as posed: their middle,
    /// and how far the feature reaches round it.
    fn feature_extent(&mut self, feature: &str) -> Option<(Vec3, f32)> {
        let (_, model, posed, transform) = self.body_now()?;
        let prepared = self.prepared(&model)?;
        let feature = prepared.feature(feature)?;
        let points: Vec<Vec3> = feature
            .anchors
            .iter()
            .map(|(bone, at)| Self::carried(posed.as_deref(), &model, transform, *bone, *at))
            .collect();
        if points.is_empty() {
            return None;
        }
        let centre = points.iter().copied().sum::<Vec3>() / points.len() as f32;
        // World units in a metre of the file.
        let scale = axcenity_scene::fitted_per_metre(&model).unwrap_or(1.0)
            * transform.transform_vector3(Vec3::X).length();
        let radius = feature.radius * scale;
        let reach = points
            .iter()
            .map(|point| point.distance(centre) + radius)
            .fold(radius, f32::max);
        Some((centre, reach))
    }

    /// Where `feature` is in the world (its anchors' middle, as posed).
    #[cfg(test)]
    pub fn feature_centre(&mut self, feature: &str) -> Option<Vec3> {
        self.feature_extent(feature).map(|(centre, _)| centre)
    }

    /// Where handle `id` was drawn last frame, if it was.
    #[cfg(test)]
    pub fn handle_on_screen(&self, id: &str) -> Option<Pos2> {
        self.direct.shown(id).map(|shown| shown.at)
    }

    /// Handle `id`'s live drives as drawn last frame: each parameter and
    /// its screen vector (its direction, and its length the points of drag
    /// per unit of value).
    #[cfg(test)]
    pub fn handle_drives_on_screen(&self, id: &str) -> Vec<(String, Vec2)> {
        self.direct
            .shown(id)
            .map(|shown| shown.live.clone())
            .unwrap_or_default()
    }

    /// Where handle `id`'s point of the skin is on the screen, by last
    /// frame's camera, shown or not (to point at a feature).
    #[cfg(test)]
    pub fn handle_point(&mut self, id: &str) -> Option<Pos2> {
        let (view, rect) = (self.direct.view?, self.direct.view_rect?);
        let (_, model, posed, transform) = self.body_now()?;
        let prepared = self.prepared(&model)?;
        let handle = prepared.handle(id)?;
        let (point, _) =
            axcenity_scene::corner_drawn(&model, posed.as_deref(), transform, handle.corner)?;
        Some(to_rect(rect, view.project_point(point)?))
    }

    /// Where each handle drawn last frame is in the world (its point of
    /// the skin).
    #[cfg(test)]
    pub fn handles_drawn(&mut self) -> Vec<Vec3> {
        let Some((_, model, posed, transform)) = self.body_now() else {
            return Vec::new();
        };
        let Some(prepared) = self.prepared(&model) else {
            return Vec::new();
        };
        self.direct
            .shown
            .iter()
            .filter_map(|shown| prepared.handle(&shown.id))
            .filter_map(|handle| {
                axcenity_scene::corner_drawn(&model, posed.as_deref(), transform, handle.corner)
            })
            .map(|(point, _)| point)
            .collect()
    }

    /// The highlight drawn last frame: each texel's middle on the screen
    /// and its strength (0 to 1), row by row from the top, with how many
    /// texels a row has. Empty when none was drawn.
    #[cfg(test)]
    pub fn highlight(&self) -> (usize, Vec<(Pos2, f32)>) {
        let Some((width, rect, strengths)) = self.direct.mask.drawn() else {
            return (0, Vec::new());
        };
        let texels = strengths
            .into_iter()
            .enumerate()
            .map(|(at, strength)| {
                let middle = egui::vec2((at % width) as f32 + 0.5, (at / width) as f32 + 0.5);
                (rect.min + middle * depth::TEXEL, strength)
            })
            .collect();
        (width, texels)
    }

    /// How many times the highlight was made.
    #[cfg(test)]
    pub fn highlight_rebuilds(&self) -> usize {
        self.direct.mask.rebuilds
    }

    /// `feature`'s weight on what an exact ray through the screen point
    /// `at` meets first, by last frame's camera (not the overlay's own
    /// buffer: rays against the posed parts), whether that's a worn item,
    /// and how far the triangle met faces the camera (1: straight on). A
    /// worn item's weight is what its corners take from the skin; a part
    /// that takes none (the base's own) gives the skin's just behind it.
    /// `None` on the backdrop.
    #[cfg(test)]
    pub fn weight_in_sight(&mut self, feature: &str, at: Pos2) -> Option<(f32, bool, f32)> {
        let (view, rect) = (self.direct.view?, self.direct.view_rect?);
        let (body, model, posed, transform) = self.body_now()?;
        let prepared = self.prepared(&model)?;
        let feature = prepared.feature(feature)?;
        let ray = view.ray(from_rect(rect, at));
        let mut parts = vec![body];
        parts.extend(
            self.world
                .all_below(body, |class| class == ClassName::MeshPart),
        );
        let first =
            axcenity_scene::ray_parts(&self.world, &self.assets, self.poses(), &parts, ray)?;
        let blend = |hit: &axcenity_scene::MeshHit, of: &Model, weights: &[Vec<f32>]| {
            let (Some(corners), Some(weights)) = (hit.corners(of), weights.get(hit.primitive))
            else {
                return 0.0;
            };
            corners
                .iter()
                .zip(hit.weights)
                .map(|(corner, share)| share * weights.get(corner.index).copied().unwrap_or(0.0))
                .sum::<f32>()
        };
        let facing = |hit: &axcenity_scene::MeshHit| hit.normal.dot(-ray.direction).abs();
        if first.part == body {
            let weight = blend(&first.hit, &model, &feature.weights);
            return Some((weight, false, facing(&first.hit)));
        }
        let map = self
            .direct
            .worn_maps
            .get(&(model.key(), first.model.key()))
            .cloned()
            .flatten();
        if let Some(map) = map {
            let hit = &first.hit;
            let weights = map.weights(&feature.weights);
            let corners = &first.model.primitives[hit.primitive].indices[hit.triangle * 3..];
            let weight = worn::blend(
                &weights[hit.primitive],
                [corners[0], corners[1], corners[2]],
                hit.weights,
                map.fine(hit.primitive).get(hit.triangle).copied(),
            );
            return Some((weight, true, facing(hit)));
        }
        let per_metre = axcenity_scene::fitted_per_metre(&model).unwrap_or(1.0)
            * transform.transform_vector3(Vec3::X).length();
        let skin = axcenity_scene::ray_model(&model, posed.as_deref(), transform, ray)
            .filter(|skin| skin.distance - first.hit.distance <= WORN * per_metre);
        Some(match skin {
            Some(skin) => (blend(&skin, &model, &feature.weights), false, facing(&skin)),
            None => (0.0, false, 0.0),
        })
    }

    /// How strong the highlight is inside a feature that's selected, and
    /// one that's hovered.
    #[cfg(test)]
    pub fn highlight_strengths() -> [f32; 2] {
        [SELECTED[0], HOVERED[0]]
    }

    /// Whether `point` (in the world) can be seen from last frame's camera,
    /// found with exact rays against the posed body (not the overlay's own
    /// test): a ray through it, or through a point `slack` points beside it
    /// on the screen, meets nothing more than `depth` (in the file's metres)
    /// nearer the camera, or misses the body (`point` is at its outline).
    #[cfg(test)]
    pub fn in_sight(&mut self, point: Vec3, slack: f32, depth: f32) -> bool {
        let (Some(view), Some(rect)) = (self.direct.view, self.direct.view_rect) else {
            return false;
        };
        let Some((_, model, posed, transform)) = self.body_now() else {
            return false;
        };
        let eye = self.orbit.camera().position;
        let per_metre = axcenity_scene::fitted_per_metre(&model).unwrap_or(1.0)
            * transform.transform_vector3(Vec3::X).length();
        let Some(at) = view.project_point(point) else {
            return false;
        };
        let at = to_rect(rect, at);
        let far = (point - eye).length() - depth * per_metre;
        [
            egui::vec2(0.0, 0.0),
            egui::vec2(slack, 0.0),
            egui::vec2(-slack, 0.0),
            egui::vec2(0.0, slack),
            egui::vec2(0.0, -slack),
        ]
        .into_iter()
        .any(|offset| {
            let ray = view.ray(from_rect(rect, at + offset));
            axcenity_scene::ray_model(&model, posed.as_deref(), transform, ray)
                .is_none_or(|hit| hit.distance >= far)
        })
    }

    /// The skin inside `feature`'s edge at rest (its weight at least
    /// [`EDGE`]): its area in square centimetres and the size of its
    /// box in centimetres (x across, y up, z forward).
    #[cfg(test)]
    pub fn tint_extent(&mut self, feature: &str) -> Option<(f32, Vec3)> {
        let (_, model, _, _) = self.body_now()?;
        let prepared = self.prepared(&model)?;
        let feature = prepared.feature(feature)?;
        let metres = axcenity_scene::to_metres(&model);
        let (mut area, mut low, mut high) = (0.0, Vec3::INFINITY, Vec3::NEG_INFINITY);
        for (primitive, weights) in model.primitives.iter().zip(&feature.weights) {
            let at = |corner: u32| Vec3::from(primitive.positions[corner as usize]) * metres;
            for &[a, b, c] in primitive.indices.as_chunks::<3>().0 {
                if [a, b, c]
                    .iter()
                    .any(|corner| weights[*corner as usize] < EDGE)
                {
                    continue;
                }
                area += (at(b) - at(a)).cross(at(c) - at(a)).length() / 2.0;
                for corner in [a, b, c] {
                    low = low.min(at(corner));
                    high = high.max(at(corner));
                }
            }
        }
        (area > 0.0).then(|| (area * 1e4, (high - low) * 100.0))
    }

    /// Where handle `id`'s point of the skin is in the world this frame.
    #[cfg(test)]
    pub fn handle_world(&mut self, id: &str) -> Option<Vec3> {
        let (_, model, posed, transform) = self.body_now()?;
        let prepared = self.prepared(&model)?;
        let handle = prepared.handle(id)?;
        axcenity_scene::corner_drawn(&model, posed.as_deref(), transform, handle.corner)
            .map(|(point, _)| point)
    }

    /// Where `feature`'s anchor `index` is on the screen, by last frame's
    /// camera (a face feature's anchors are on its skin).
    #[cfg(test)]
    pub fn anchor_point(&mut self, feature: &str, index: usize) -> Option<Pos2> {
        let (view, rect) = (self.direct.view?, self.direct.view_rect?);
        let (_, model, posed, transform) = self.body_now()?;
        let prepared = self.prepared(&model)?;
        let (bone, at) = *prepared.feature(feature)?.anchors.get(index)?;
        let point = Self::carried(posed.as_deref(), &model, transform, bone, at);
        Some(to_rect(rect, view.project_point(point)?))
    }

    /// Frames `feature` when it's small on the screen or partly off it,
    /// over a quarter of a second; one well in view stays as it is.
    pub fn focus_feature(&mut self, feature: &str) {
        let Some((centre, reach)) = self.feature_extent(feature) else {
            return;
        };
        let camera = self.orbit.camera();
        let view = CameraView::new(camera, 1.0);
        let half = view.studs_per_half_screen(centre);
        let on_screen = view
            .project_point(centre)
            .is_some_and(|point| point.x.abs() < 0.8 && point.y.abs() < 0.8);
        if reach / half >= SMALL && on_screen {
            return;
        }
        // The feature's reach over about a third of the view's half-height.
        let tan = (camera.fov_y_degrees.to_radians() / 2.0).tan();
        let distance = (reach / (0.35 * tan)).clamp(1.2, 30.0);
        let to = Orbit {
            distance,
            target: centre,
            ..self.orbit
        };
        self.direct.focus = Some(Focus {
            from: self.orbit,
            to,
            started: Instant::now(),
        });
        self.view = None;
    }

    /// Selects `feature` (or none) as the navigation does, moving to its
    /// category's page when it's another's.
    pub(super) fn pick_feature(&mut self, feature: Option<String>) {
        if let Some(id) = &feature {
            let category = self
                .library()
                .base(&self.description.base)
                .and_then(|base| base.feature(id))
                .and_then(|feature| category_of(feature.category));
            if let Some(category) = category
                && category != self.selection.category
            {
                self.pick_category(category);
            }
        }
        self.selection.feature = feature;
    }

    /// The viewport's input for direct manipulation, before the camera's:
    /// a handle drag (begun, moved on or ended) and a click that selects.
    /// `rect` is the current character's part of the viewport.
    pub(super) fn direct_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        looking: bool,
    ) -> Vec<Intent> {
        let mut intents = Vec::new();
        let active = self.direct.on && !looking;
        let (origin, at, modifiers, down) = ui.input(|input| {
            (
                input.pointer.press_origin(),
                input.pointer.interact_pos(),
                input.modifiers,
                input.pointer.primary_down(),
            )
        });
        if active
            && self.direct.drag.is_none()
            && response.drag_started_by(egui::PointerButton::Primary)
            && !modifiers.alt
            && let Some(origin) = origin
            && let Some(shown) = self.direct.handle_near(origin).cloned()
        {
            self.begin_handle_drag(&shown, origin);
        }
        if self.direct.drag.is_some() {
            match (down, at) {
                (true, Some(at)) => intents.extend(self.handle_drag_to(at, modifiers)),
                _ => intents.extend(self.end_handle_drag()),
            }
            return intents;
        }
        if active && response.clicked_by(egui::PointerButton::Primary) {
            match self.direct.hovered.clone() {
                Some(feature) => {
                    if self.selection.level == Level::Quick {
                        intents.push(Intent::PickLevel(Level::Detailed));
                    }
                    if self.selection.feature.as_deref() != Some(feature.as_str()) {
                        intents.push(Intent::PickFeature(Some(feature.clone())));
                        intents.push(Intent::FocusFeature(feature));
                    }
                }
                None if self.selection.feature.is_some() => {
                    intents.push(Intent::PickFeature(None));
                }
                None => {}
            }
        }
        intents
    }

    fn begin_handle_drag(&mut self, shown: &Shown, origin: Pos2) {
        let library = self.library();
        let base = library.base(&self.description.base);
        let mut drives: Vec<DragDrive> = Vec::new();
        let mut lines: Vec<Vec2> = Vec::new();
        for (parameter, screen) in &shown.live {
            let Some(manifest) = base.and_then(|base| base.parameter(parameter)) else {
                continue;
            };
            let direction = screen.normalize_or_zero();
            // A drive on the line of one already taken gives way; two
            // directions are all a drag on a screen has.
            let apart = APART.to_radians().cos();
            if drives.len() == 2 || lines.iter().any(|line| line.dot(direction).abs() > apart) {
                continue;
            }
            lines.push(direction);
            let start = super::value_of(&self.description, base, parameter);
            drives.push(DragDrive {
                parameter: parameter.clone(),
                start,
                range: manifest.range,
                gain: screen.length(),
                engaged: false,
                last: start,
            });
        }
        let screens: Vec<Vec2> = shown
            .live
            .iter()
            .filter(|(parameter, _)| drives.iter().any(|drive| &drive.parameter == parameter))
            .map(|(_, screen)| *screen)
            .collect();
        // How much each value moves per point of drag: for one drive, the
        // drag along its screen line; for two, the two solved together so
        // the skin under the handle follows the pointer.
        let solve = match screens.as_slice() {
            [one] => vec![*one / one.length_squared().max(1e-9)],
            [a, b] => {
                let det = a.x * b.y - a.y * b.x;
                if det.abs() < 1e-6 {
                    drives.truncate(1);
                    vec![*a / a.length_squared().max(1e-9)]
                } else {
                    // The inverse of the matrix with columns `a` and `b`.
                    vec![Vec2::new(b.y, -b.x) / det, Vec2::new(-a.y, a.x) / det]
                }
            }
            _ => Vec::new(),
        };
        if drives.is_empty() {
            return;
        }
        self.direct.drag = Some(HandleDrag {
            handle: shown.id.clone(),
            from: origin,
            travel: numeric::Travel::default(),
            mirrored: self.symmetry || self.selection.level == Level::Quick,
            drives,
            solve,
            cancelled: false,
            sent: false,
        });
    }

    /// The values for the pointer at `at` with `modifiers` held, sent as
    /// the drag's changes.
    fn handle_drag_to(&mut self, at: Pos2, modifiers: egui::Modifiers) -> Vec<Intent> {
        let Some(drag) = &mut self.direct.drag else {
            return Vec::new();
        };
        if drag.cancelled {
            return Vec::new();
        }
        let step = at - drag.from;
        drag.from = at;
        drag.travel.add(Vec2::new(step.x, step.y), modifiers);
        let mut intents = Vec::new();
        for (drive, solve) in drag.drives.iter_mut().zip(&drag.solve) {
            // Its own part of the drag, in points as the pointer went.
            if (solve.dot(drag.travel.raw) * drive.gain).abs() >= DEAD_ZONE {
                drive.engaged = true;
            }
            if !drive.engaged {
                continue;
            }
            let (low, high) = (
                drive.range[0].min(drive.range[1]),
                drive.range[0].max(drive.range[1]),
            );
            let value = (drive.start + solve.dot(drag.travel.scaled)).clamp(low, high);
            if value == drive.last {
                continue;
            }
            drive.last = value;
            drag.sent = true;
            intents.push(set(&drive.parameter, value, drag.mirrored, Gesture::Drag));
        }
        intents
    }

    /// Ends the drag: the last change again, closing the step (nothing
    /// when nothing changed).
    fn end_handle_drag(&mut self) -> Vec<Intent> {
        let Some(drag) = self.direct.drag.take() else {
            return Vec::new();
        };
        if drag.cancelled || !drag.sent {
            return Vec::new();
        }
        drag.drives
            .iter()
            .rev()
            .find(|drive| drive.last != drive.start)
            .map(|drive| set(&drive.parameter, drive.last, drag.mirrored, Gesture::Commit))
            .into_iter()
            .collect()
    }

    /// Esc during a handle drag: the rest of it does nothing.
    pub(super) fn cancel_handle_drag(&mut self) {
        if let Some(drag) = &mut self.direct.drag {
            drag.cancelled = true;
        }
    }

    /// After the frame's pose: what's under the pointer, the selected
    /// feature's handles, and the overlay drawn over the preview in `rect`.
    pub(super) fn direct_paint(&mut self, ui: &egui::Ui, rect: Rect, looking: bool) {
        let camera = self.orbit.camera();
        let view = CameraView::new(camera, rect.width() / rect.height().max(1.0));
        self.direct.view = Some(view);
        self.direct.view_rect = Some(rect);
        self.direct.shown.clear();
        self.direct.status = None;
        self.direct.mask.hide();
        if !self.direct.on || looking {
            self.direct.forget_hover();
            self.direct.hovered_handle = None;
            return;
        }
        let Some((body, model, posed, transform)) = self.body_now() else {
            return;
        };
        let Some(prepared) = self.prepared(&model) else {
            return;
        };
        let (pointer, any_down) =
            ui.input(|input| (input.pointer.hover_pos(), input.pointer.any_down()));
        let pointer = pointer.filter(|point| rect.contains(*point));
        // What the camera sees of the character, looked at when there's
        // something to draw: the highlight lies on the skin and the
        // clothes over it, the handles on the skin whatever is worn over
        // it (a handle under a coat is still the way to change the body:
        // it shows on the coat).
        let mut seeing: Option<(Vec<WornNow>, Arc<Sight>)> = None;
        if self.selection.feature.is_some() {
            let worn = self.worn_now(body, &model);
            let sight = self.sight(&model, posed.as_deref(), transform, view, rect, &worn);
            seeing = Some((worn, sight));
        }
        let tolerance = HIDDEN
            * axcenity_scene::fitted_per_metre(&model).unwrap_or(1.0)
            * transform.transform_vector3(Vec3::X).length();

        // The selected feature's handles.
        let framing = framing(self.orbit.yaw);
        let to_screen = |point: Vec3| view.project_point(point).map(|ndc| to_rect(rect, ndc));
        let dragged = self.direct.drag.as_ref().map(|drag| drag.handle.clone());
        let dragged_mirror = dragged
            .as_deref()
            .and_then(|id| prepared.handle(id))
            .and_then(|handle| handle.mirror.clone());
        let library = self.library();
        let base = library.base(&self.description.base);
        let frame = super::value_of(&self.description, base, "frame");
        if let Some(selected) = self.selection.feature.clone() {
            for handle in prepared.handles.iter().filter(|h| h.feature == selected) {
                // While one is dragged, the others hide (ux patterns P8),
                // all but its mirror when Symmetry moves it.
                if let Some(dragged) = &dragged
                    && *dragged != handle.id
                    && !(self.direct.drag.as_ref().is_some_and(|d| d.mirrored)
                        && dragged_mirror.as_deref() == Some(handle.id.as_str()))
                {
                    continue;
                }
                let Some((point, normal)) = axcenity_scene::corner_drawn(
                    &model,
                    posed.as_deref(),
                    transform,
                    handle.corner,
                ) else {
                    continue;
                };
                let toward = (camera.position - point).normalize_or_zero();
                // On the far side, or behind other skin (the far jaw angle
                // behind the cheek); one on the outline (a jaw's corner
                // seen from the front) still shows.
                if normal.dot(toward) < -0.5
                    || seeing
                        .as_ref()
                        .is_some_and(|(_, sight)| sight.skin.hidden(point, tolerance, HANDLE_SIGHT))
                {
                    continue;
                }
                let Some(at) = to_screen(point).filter(|at| rect.contains(*at)) else {
                    continue;
                };
                // The joint that carries the anchor turns its axis as posed.
                let turn = match (posed.as_deref(), handle.bone, model.rig.as_deref()) {
                    (Some(posed), Some(bone), Some(rig)) => rig
                        .joints
                        .iter()
                        .position(|joint| joint.bone == bone)
                        .and_then(|joint| posed.joints().get(joint).copied())
                        .unwrap_or(Mat4::IDENTITY),
                    _ => Mat4::IDENTITY,
                };
                let mut live = Vec::new();
                let mut at_limit = false;
                for drive in &handle.drives {
                    let Some((_, data)) = drive.screen.iter().find(|(name, _)| name == framing)
                    else {
                        continue;
                    };
                    let travel =
                        transform.transform_vector3(turn.transform_vector3(drive.travel_on(frame)));
                    let moved = to_screen(point + travel)
                        .map(|end| Vec2::new(end.x - at.x, end.y - at.y))
                        .unwrap_or(Vec2::ZERO);
                    // The direction: the skin's projected move, even when
                    // it's short; the data's only when it doesn't show at
                    // all, its x turned for a camera on the character's
                    // left (the data's framings are from its right). How
                    // far: the studio's one rule, the whole range in 110
                    // points whatever the zoom (`numeric.rs`).
                    let direction = if moved.length() >= 0.01 {
                        moved.normalize()
                    } else {
                        let across = if self.orbit.yaw.rem_euclid(360.0) > 180.0 {
                            -data.x
                        } else {
                            data.x
                        };
                        Vec2::new(across, -data.y).normalize_or_zero()
                    };
                    let screen = direction * numeric::points_per_unit(drive.range);
                    live.push((drive.parameter.clone(), screen));
                    let value = super::value_of(&self.description, base, &drive.parameter);
                    let (low, high) = (
                        drive.range[0].min(drive.range[1]),
                        drive.range[0].max(drive.range[1]),
                    );
                    at_limit |= value <= low + 1e-4 || value >= high - 1e-4;
                }
                if live.is_empty() {
                    continue;
                }
                self.direct.shown.push(Shown {
                    id: handle.id.clone(),
                    at,
                    live,
                    at_limit,
                });
            }
        }

        // What's under the pointer: a handle first, else a feature. Kept
        // as it was while a button is down or a handle is dragged.
        if !any_down && self.direct.drag.is_none() {
            self.direct.hovered_handle = pointer
                .and_then(|point| self.direct.handle_near(point))
                .map(|shown| shown.id.clone());
            // A handle under the pointer names its own feature (one on the
            // border of two regions, or beside a nostril, is still its
            // feature's).
            if let Some(feature) = self
                .direct
                .hovered_handle
                .as_deref()
                .and_then(|id| prepared.handle(id))
                .map(|handle| handle.feature.clone())
            {
                self.direct.picked_at = pointer.map(|point| (point, self.orbit));
                self.direct.hovered = Some(feature);
            }
            // The ray is the frame's dearest part: it's cast again only
            // when the pointer or the camera moves (the idle sway alone
            // doesn't change which feature is under a still pointer).
            let asked = pointer.map(|point| (point, self.orbit));
            if self.direct.hovered_handle.is_none() && asked != self.direct.picked_at {
                self.direct.picked_at = asked;
                self.direct.hovered = match pointer {
                    Some(point) => {
                        let ray = view.ray(from_rect(rect, point));
                        self.feature_under(
                            ray,
                            body,
                            &model,
                            posed.as_deref(),
                            transform,
                            &prepared,
                        )
                    }
                    None => None,
                };
            }
        }

        // The overlay: the highlight of the selected feature (fainter) and
        // of the hovered one (stronger), then the handles on top. One
        // feature that is both is lit as both. None while a handle is
        // dragged.
        let painter = ui.painter_at(rect);
        let palette = theme::palette(ui.ctx());
        let mut lit: Vec<(&PreparedFeature, [f32; 2])> = Vec::new();
        if self.direct.drag.is_none() {
            let features = [
                (self.selection.feature.as_deref(), SELECTED),
                (self.direct.hovered.as_deref(), HOVERED),
            ];
            for (feature, [fill, rim]) in features {
                let Some(feature) = feature.and_then(|id| prepared.feature(id)) else {
                    continue;
                };
                match lit.iter_mut().find(|(other, _)| other.id == feature.id) {
                    // One tint over the other.
                    Some((_, [under, under_rim])) => {
                        *under = *under + fill - *under * fill;
                        *under_rim = *under_rim + rim - *under_rim * rim;
                    }
                    None => lit.push((feature, [fill, rim])),
                }
            }
        }
        if !lit.is_empty() {
            let (worn, sight) = match seeing {
                Some(seeing) => seeing,
                None => {
                    let worn = self.worn_now(body, &model);
                    let sight = self.sight(&model, posed.as_deref(), transform, view, rect, &worn);
                    (worn, sight)
                }
            };
            // Made again only when what it shows changed: the camera, the
            // pose, what's worn, or which features are lit.
            let key = MaskKey {
                sight: sight.stamp,
                prepared: Arc::as_ptr(&prepared) as usize,
                lit: lit
                    .iter()
                    .map(|(feature, strengths)| (feature.id.clone(), *strengths))
                    .collect(),
            };
            if self.direct.mask_key.as_ref() != Some(&key) || !self.direct.mask.ready(ui.ctx()) {
                let mut surfaces: Vec<Surface> = model
                    .primitives
                    .iter()
                    .map(|primitive| Surface {
                        indices: &primitive.indices,
                        fine: &[],
                    })
                    .collect();
                for part in &worn {
                    let primitives = part.model.primitives.iter().enumerate();
                    surfaces.extend(primitives.map(|(index, primitive)| Surface {
                        indices: &primitive.indices,
                        fine: part.map.as_ref().map_or(&[], |map| map.fine(index)),
                    }));
                }
                // Each lit feature's weights on each worn part, then on
                // everything the buffer holds.
                let mut cloths = Vec::new();
                for (feature, _) in &lit {
                    let cloth: Vec<_> = worn
                        .iter()
                        .map(|part| {
                            let map = part.map.as_ref()?;
                            Some(self.cloth(&prepared, feature, &part.model, map))
                        })
                        .collect();
                    cloths.push(cloth);
                }
                let layers: Vec<Layer> = lit
                    .iter()
                    .zip(&cloths)
                    .map(|((feature, [fill, rim]), cloth)| Layer {
                        weights: shown_weights(
                            feature,
                            &model,
                            posed.as_deref(),
                            transform,
                            camera.position,
                            &worn,
                            cloth,
                        ),
                        fill: *fill,
                        rim: *rim,
                    })
                    .collect();
                self.direct
                    .mask
                    .rebuild(ui.ctx(), &sight.all, &surfaces, &layers, EDGE);
                self.direct.mask_key = Some(key);
            }
            self.direct.mask.paint(&painter, palette.accent);
        }
        for shown in &self.direct.shown {
            let hovered = self.direct.hovered_handle.as_deref() == Some(shown.id.as_str())
                || dragged.as_deref() == Some(shown.id.as_str());
            let radius = if hovered { DOT * 1.35 } else { DOT };
            let ring = if shown.at_limit {
                palette.warning
            } else {
                palette.accent_solid
            };
            painter.circle_filled(
                shown.at + egui::vec2(0.0, 1.0),
                radius + 1.5,
                Color32::from_black_alpha(70),
            );
            painter.circle(shown.at, radius, Color32::WHITE, Stroke::new(2.0, ring));
            // The directions a drag works in, on the handle under the
            // pointer (ux patterns step 6).
            if hovered && dragged.is_none() {
                for (_, screen) in &shown.live {
                    let direction = screen.normalize_or_zero();
                    let arrow = egui::vec2(direction.x, direction.y) * (radius + 9.0);
                    for sign in [1.0, -1.0] {
                        let tip = shown.at + arrow * sign;
                        painter.line_segment(
                            [shown.at + arrow * sign * 0.55, tip],
                            Stroke::new(2.0, ring),
                        );
                    }
                }
            }
        }
        self.direct.status = self.direct_status(&prepared);
    }

    /// What the camera sees of the character this frame: its body as
    /// posed and the parts `worn` (each as drawn), rasterized over `rect`.
    /// The last one made is handed back while the view, the poses and
    /// what's worn are the same. A part that carries the weights of the
    /// skin under it counts where it is; any other, [`WORN`] farther.
    fn sight(
        &mut self,
        model: &Model,
        posed: Option<&axcenity_assets::Posed>,
        transform: Mat4,
        view: CameraView,
        rect: Rect,
        worn: &[WornNow],
    ) -> Arc<Sight> {
        let part = |model: &Model, posed: Option<&axcenity_assets::Posed>, at: Mat4, carries| {
            let pose = posed
                .filter(|posed| posed.model_key == model.key())
                .map(pose_print);
            (model.key(), pose, at.to_cols_array(), carries)
        };
        let mut parts = vec![part(model, posed, transform, true)];
        parts.extend(worn.iter().map(|worn| {
            part(
                &worn.model,
                worn.posed.as_deref(),
                worn.transform,
                worn.map.is_some(),
            )
        }));
        let made_for = SightKey {
            rect,
            orbit: self.orbit,
            parts,
        };
        if let Some(sight) = &self.direct.sight
            && sight.made_for == made_for
        {
            return Arc::clone(sight);
        }
        let camera = self.orbit.camera();
        let per_metre = axcenity_scene::fitted_per_metre(model).unwrap_or(1.0)
            * transform.transform_vector3(Vec3::X).length();
        let mut all = Depth::new(
            view,
            rect,
            camera.position,
            camera.look_at,
            camera.fov_y_degrees,
        )
        .recording();
        let mut surface = 0;
        for (index, primitive) in model.primitives.iter().enumerate() {
            let corners = axcenity_scene::corners_drawn(model, posed, transform, index);
            all.add(surface, &corners, &primitive.indices, 0.0);
            surface += 1;
        }
        let skin = all.depths();
        for part in worn {
            let slack = if part.map.is_some() {
                0.0
            } else {
                WORN * per_metre
            };
            for (index, primitive) in part.model.primitives.iter().enumerate() {
                let corners = axcenity_scene::corners_drawn(
                    &part.model,
                    part.posed.as_deref(),
                    part.transform,
                    index,
                );
                all.add(surface, &corners, &primitive.indices, slack);
                surface += 1;
            }
        }
        self.direct.sights += 1;
        let sight = Arc::new(Sight {
            made_for,
            stamp: self.direct.sights,
            skin,
            all,
        });
        self.direct.sight = Some(Arc::clone(&sight));
        sight
    }

    /// The feature `ray` meets on the character: the skin's; on a worn
    /// item, the one lit there (the cloth carries the weights of the skin
    /// under it, and that skin itself is often left out of the body); else
    /// the skin's just behind the part it meets first.
    fn feature_under(
        &mut self,
        ray: Ray,
        body: axcenity_core::InstanceId,
        model: &Model,
        posed: Option<&axcenity_assets::Posed>,
        transform: Mat4,
        prepared: &Arc<Prepared>,
    ) -> Option<String> {
        let mut parts = vec![body];
        parts.extend(
            self.world
                .all_below(body, |class| class == ClassName::MeshPart),
        );
        let first =
            axcenity_scene::ray_parts(&self.world, &self.assets, self.poses(), &parts, ray)?;
        let hit = if first.part == body {
            first.hit
        } else {
            let item = self.world.get(first.part).is_some_and(|instance| {
                instance
                    .attribute(applier::DESCRIPTION_PART)
                    .and_then(|value| value.as_str())
                    .is_some_and(|key| !key.starts_with("base:"))
            });
            if item
                && let Some(map) = self.worn_map(model, &first.model)
                && let Some(feature) =
                    self.feature_on_cloth(prepared, &first.model, &map, &first.hit)
            {
                return Some(feature);
            }
            let per_metre = axcenity_scene::fitted_per_metre(model).unwrap_or(1.0)
                * transform.transform_vector3(Vec3::X).length();
            // Through what the description wears the skin is found much
            // farther back than through the base's own parts (an eyeball).
            let through = if item { THROUGH_WORN } else { THROUGH };
            let skin = axcenity_scene::ray_model(model, posed, transform, ray)
                .filter(|skin| skin.distance - first.hit.distance <= through * per_metre);
            match skin {
                Some(skin) => skin,
                // A part of the base with no skin close behind it (an
                // eyeball in an open socket) is its own place at rest:
                // the feature whose anchors that is nearest.
                None if !item => {
                    // Each model is fitted to its own box: through the
                    // part's file space into the body's fitted one.
                    let at = first.hit.at_rest(&first.model)?;
                    let part_fit = first.model.rig.as_ref()?.fit;
                    let body_fit = model.rig.as_ref()?.fit;
                    let at = body_fit.transform_point3(part_fit.inverse().transform_point3(at));
                    return prepared
                        .feature_by_anchor(at, None)
                        .map(|feature| feature.id.clone());
                }
                None => return None,
            }
        };
        prepared
            .feature_hit(&hit, model)
            .map(|feature| feature.id.clone())
    }

    /// The status line while hovering or dragging.
    fn direct_status(&self, prepared: &Prepared) -> Option<String> {
        let library = self.library();
        let base = library.base(&self.description.base);
        let label = |parameter: &str| {
            base.and_then(|base| base.parameter(parameter))
                .map_or_else(|| parameter.to_owned(), |p| p.label.clone())
        };
        let way = |screen: &Vec2| {
            if screen.x.abs() > 2.0 * screen.y.abs() {
                "left and right"
            } else if screen.y.abs() > 2.0 * screen.x.abs() {
                "up and down"
            } else {
                "along the arrows"
            }
        };
        if let Some(drag) = &self.direct.drag {
            let handle = prepared.handle(&drag.handle)?;
            let values: Vec<String> = drag
                .drives
                .iter()
                // The description's value, as the sliders show it (the
                // drag's own runs a frame ahead of what's applied).
                .map(|drive| {
                    let value = super::value_of(&self.description, base, &drive.parameter);
                    format!("{} {value:.2}", label(&drive.parameter))
                })
                .collect();
            return Some(format!(
                "{}: {} · Esc to cancel",
                handle.label,
                values.join(" · ")
            ));
        }
        if let Some(id) = &self.direct.hovered_handle {
            let handle = prepared.handle(id)?;
            let shown = self.direct.shown(id)?;
            let ways: Vec<String> = shown
                .live
                .iter()
                .map(|(parameter, screen)| format!("{}: {}", way(screen), label(parameter)))
                .collect();
            return Some(format!("{} · drag {}", handle.label, ways.join(" · ")));
        }
        let feature = prepared.feature(self.direct.hovered.as_deref()?)?;
        Some(format!("{} · click to select it", feature.label))
    }
}

/// The intent a slider sends for `parameter` at `value`.
fn set(parameter: &str, value: f32, mirrored: bool, gesture: Gesture) -> Intent {
    let id = parameter.to_owned();
    if mirrored {
        Intent::SetMirrored { id, value, gesture }
    } else {
        Intent::SetParameter { id, value, gesture }
    }
}

/// `feature`'s weights as they show this frame, for each surface of the
/// [`Sight`] (the body's primitives, then each of `worn`'s): `None` for
/// one that carries none.
///
/// A corner of the skin counts only as much as it faces the camera (the
/// posed normal: [`FACING`]): the inside of a nostril or of the lips'
/// seam, which faces away, drops out. A worn part's corners carry what
/// the skin under them does (`cloth`: [`WornMap::weights`] for each of
/// `worn`, `None` for a part that carries none), whichever way they face
/// (cloth is seen from both sides).
fn shown_weights<'a>(
    feature: &PreparedFeature,
    model: &Model,
    posed: Option<&axcenity_assets::Posed>,
    transform: Mat4,
    camera: Vec3,
    worn: &[WornNow],
    cloth: &'a [Option<ClothWeights>],
) -> Vec<Option<Cow<'a, [f32]>>> {
    let carries = |weights: &[f32]| weights.iter().any(|weight| *weight >= EDGE);
    let mut shown = Vec::new();
    for index in 0..model.primitives.len() {
        let weights = feature.weights.get(index).filter(|w| carries(w));
        shown.push(weights.map(|weights| {
            let corners = axcenity_scene::corners_drawn(model, posed, transform, index);
            let normals = axcenity_scene::normals_drawn(model, posed, transform, index);
            let shown: Vec<f32> = weights
                .iter()
                .zip(corners.iter().zip(&normals))
                .map(|(weight, (point, normal))| {
                    if *weight <= 0.0 {
                        return 0.0;
                    }
                    let facing = normal.dot((camera - *point).normalize_or_zero());
                    weight * (facing / FACING).clamp(0.0, 1.0)
                })
                .collect();
            Cow::Owned(shown)
        }));
    }
    for (part, weights) in worn.iter().zip(cloth) {
        for index in 0..part.model.primitives.len() {
            shown.push(
                weights
                    .as_ref()
                    .and_then(|weights| weights.get(index))
                    .filter(|weights| carries(weights))
                    .map(|weights| Cow::Borrowed(weights.as_slice())),
            );
        }
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_camera_s_turn_names_its_framing() {
        assert_eq!(framing(0.0), "front");
        assert_eq!(framing(12.0), "front");
        assert_eq!(framing(45.0), "three_quarter");
        assert_eq!(framing(90.0), "side");
        assert_eq!(framing(270.0), "side");
        assert_eq!(framing(315.0), "three_quarter");
        assert_eq!(framing(180.0), "back");
        assert_eq!(framing(-10.0), "front");
    }

    #[test]
    fn screen_points_and_the_view_s_points_turn_into_each_other() {
        let rect = Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(400.0, 300.0));
        for ndc in [Vec2::ZERO, Vec2::new(1.0, 1.0), Vec2::new(-0.5, 0.25)] {
            let back = from_rect(rect, to_rect(rect, ndc));
            assert!((back - ndc).length() < 1e-5);
        }
        assert_eq!(to_rect(rect, Vec2::new(-1.0, 1.0)), rect.left_top());
    }
}
