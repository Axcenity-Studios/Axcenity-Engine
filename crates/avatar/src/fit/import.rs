//! The import flow (CO9): a third-party garment made in a base body's
//! rest pose (glTF or OBJ, roughly aligned) becomes an item of a character
//! library. It is aligned to the body, fitted ([`fit_garment`]: binding,
//! weights, the body's shapes, push-out), its regions and slot found from
//! what it covers, drawn as a thumbnail, and written as a model, a binding
//! file, a thumbnail and an item manifest beside them; the library's
//! catalogue lists it. An import is always a **draft** (`quality`) and
//! **premium** (`tier`) until reviewed.

use std::path::{Path, PathBuf};

use axcenity_assets::Assets;
use glam::Vec3;

use super::cover::{COVER_SIZE, CoverReport, Proof, cover_map, skin_uvs};
use super::coverage::{RegionMap, classify, coverage, hidden_regions, triangle_regions};
use super::glb::{FittedModel, PieceMaps, write_fitted_glb};
use super::hiding::pose_safe_hides;
use super::pose::{FinishPose, fit_poses};
use super::proxy::ProxyKind;
use super::surface::Surface;
use super::thumbnail::{thumbnail, write_png};
use super::transfer::{FitOptions, FitReport, HomeFrame, carried_shapes, fit_garment, garment_at};
use super::validate::validate_garment;
use super::{BodyMesh, FitError, GarmentMesh, is_mask_shape};
use crate::{
    BaseManifest, CatalogueItem, ContentTier, Description, Fit, ItemKind, ItemManifest, Library,
    Limitation, MaterialSlot, ModelRef, Quality, ShadingModel, Source, body_shape_weights,
};

/// What to import and where.
#[derive(Clone, Debug)]
pub struct ImportRequest {
    /// The garment's file (`.glb`, `.gltf` or `.obj`).
    pub source: PathBuf,
    /// The character library's folder (with its `catalogue.json`).
    pub library: PathBuf,
    /// The base model it is made for, by ID.
    pub base: String,
    /// The new item's ID and label.
    pub id: String,
    pub label: String,
    /// Where in the library its files go (a folder, `/` between names).
    pub folder: String,
    /// Who made it and its licence (an import must say).
    pub source_info: Source,
    /// Its slot, or `None` to classify it from what it covers.
    pub slot: Option<String>,
    /// Its layer (`None`: 1, a base layer).
    pub layer: Option<u8>,
    /// Its clearance from the body (metres; used for chord-sag validation).
    pub margin: f32,
    /// Retained for compatibility; ADR-0046 uses non-destructive `Shrink_Under`
    /// and UV cover maps instead of destructive rest-mesh push-out.
    pub under: Vec<String>,
    /// A sentence for the browser.
    pub about: Option<String>,
    /// Other slots it fills besides its own (a jumpsuit or a dress given
    /// the top slot: `bottom`), added to any its coverage says.
    pub also_fills: Vec<String>,
    /// Tags for the browser's search and kind filter, after `imported`
    /// (the frame it is cut for: `masculine`, `feminine`).
    pub tags: Vec<String>,
    /// Known limits (the frame it is cut for, as a range of the base's
    /// `frame` parameter: the browser's "Fits this body").
    pub limitations: Vec<Limitation>,
    /// What the runtime does to a pose before skinning (the twist and
    /// helper bones: `axcenity_anim::twist::drive`), so the poses it is
    /// fitted in are the runtime's ([`FinishPose`]). `None`: the fit's
    /// poses turn only the limbs.
    pub finish_pose: Option<FinishPose>,
    /// Home frame the garment was authored on (no cross-frame rest warping).
    pub home_frame: HomeFrame,
    /// Proxy surface to bind against (`Body` or `Skirt`).
    pub proxy_kind: ProxyKind,
    /// When `true`, excludes `LeftArm`/`RightArm` during part assignment so
    /// sleeveless garments bind strictly to `Trunk`.
    pub sleeveless: bool,
    /// When `true`, treats the source mesh as already aligned to its canonical
    /// home frame and skips legacy `align()` (0.00 mm rest drift).
    pub skip_align: bool,
    /// Generates non-destructive `Shrink_Under` morph toward Layer Envelope `layer - 1`.
    pub generate_shrink_under: bool,
    /// When `true`, rejects garments that fail [`validate_garment`] (non-manifold
    /// edges, degenerate triangles, or opening chord-sag violations).
    pub strict_validate: bool,
    /// Carries the texture maps of the garment's file (a `.glb`'s base
    /// colour, normal and metal-roughness maps) into the item's model,
    /// each made no larger than this many pixels a side and stored as
    /// JPEG. `None`: the item has its materials' plain colours only.
    pub texture_size: Option<u32>,
}

/// The regions of a body's map round the hips: where a tucked top is hidden
/// whole under a bottom (a cover map's blue).
const ROUND_THE_HIPS: [&str; 3] = ["waist", "hips", "thighs_top"];

/// The indices of [`ROUND_THE_HIPS`] in `base`'s region map.
fn round_the_hips(base: &BaseManifest) -> Vec<u8> {
    base.regions
        .iter()
        .filter(|region| ROUND_THE_HIPS.contains(&region.id.as_str()))
        .filter_map(|region| region.map_index)
        .collect()
}

/// How the garment was moved onto the body: each corner `p` became
/// `p * scale + offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alignment {
    pub scale: f32,
    pub offset: Vec3,
    /// The file's units in metres (1, 0.01, 0.001 or 0.0254): part of
    /// `scale`.
    pub units: f32,
    /// How unevenly it stood off the body before the fine fit and after
    /// it (metres; see [`align`]).
    pub spread_before: f32,
    pub spread_after: f32,
}

/// What an import made.
#[derive(Clone, Debug)]
pub struct Imported {
    pub manifest: ItemManifest,
    /// The manifest's path in the library.
    pub manifest_path: String,
    pub alignment: Alignment,
    pub report: FitReport,
    /// The share of each region it covers, by region ID.
    pub coverage: Vec<(String, f32)>,
    /// Regions it covers whole at rest but doesn't hide, because their
    /// skin comes out from under it in a fit pose
    /// ([`pose_safe_hides`](super::pose_safe_hides)), with the farthest
    /// it came out (metres).
    pub unhidden: Vec<(String, f32)>,
    /// What its cover map hides, in body triangles (ADR-0045).
    pub cover: CoverReport,
}

fn library_error(message: impl Into<String>) -> FitError {
    FitError::Library(message.into())
}

/// The units a garment's file may be in, in metres.
const UNITS: [f32; 4] = [1.0, 0.01, 0.001, 0.0254];
/// The fine fit's bounds: the scale within this share of its size, and
/// the offset within these distances (up and down; across and front to
/// back).
const FINE_SCALE: f32 = 0.08;
const FINE_UP: f32 = 0.06;
const FINE_ACROSS: f32 = 0.03;
/// The fine fit is taken only when it stands the garment off the body this
/// much more evenly: a share of the spread before, and at least this much
/// (metres).
const FINE_GAIN: f32 = 0.8;
const FINE_GAIN_LEAST: f32 = 0.002;
/// How many of the garment's corners the fine fit measures.
const FINE_SAMPLES: usize = 400;

/// Aligns `garment` to `body`, in three steps:
///
/// 1. **Units.** Of metres, centimetres, millimetres and inches, the one
///    that makes it no taller than the body and closest to the body's
///    width and depth over the heights it spans.
/// 2. **Rough place.** Centred on the body across and front to back over
///    the heights it spans, unless it stood off the body more evenly where
///    it came (a garment already placed). Its height is kept: a garment's
///    file is in the body's rest pose, standing on the same floor.
/// 3. **Fine fit (bounding landmarks).** A garment made for the body
///    stands off it by about the same ease everywhere: its scale (within
///    8%, about its middle) and offset (within 6 cm up or down and 3 cm
///    across or front to back) are searched for the place where its
///    distance from the skin is most even (the root mean square distance
///    from the median) and nothing is inside the skin. It is taken when it
///    is at least 20% and 2 mm more even than before, so a garment already
///    in place stays where it is. (It can't tell a size a little bigger
///    from a more even ease: a tee with more ease over the torso than round
///    the sleeves comes back about 2% too big.)
pub fn align(garment: &mut GarmentMesh, body: &BodyMesh) -> Alignment {
    let (lo, hi) = garment.bounds();
    let units = units_of(lo, hi, body);
    garment.transform(units, Vec3::ZERO);
    let (lo, hi) = (lo * units, hi * units);
    let (band_lo, band_hi) = band(body, lo.y, hi.y);
    let middle = (lo + hi) / 2.0;
    let mut centred = (band_lo + band_hi) / 2.0 - middle;
    centred.y = 0.0;

    let surface = body.surface(&[]);
    let positions = garment.positions();
    let step = (positions.len() / FINE_SAMPLES).max(1);
    let samples: Vec<Vec3> = positions.iter().step_by(step).copied().collect();
    // Every candidate is a scale about the garment's middle as it came and
    // a shift: p -> middle + (p - middle) * scale + shift.
    let spread = |scale: f32, shift: Vec3| unevenness(&surface, &samples, middle, scale, shift);
    let as_came = spread(1.0, Vec3::ZERO);
    let rough = spread(1.0, centred);
    let (before, start) = if rough < as_came {
        (rough, centred)
    } else {
        (as_came, Vec3::ZERO)
    };

    // Scale and height on a coarse grid, then each of scale, across, up
    // and front to back nudged by halving steps while that helps.
    let mut best = (before, 1.0f32, start);
    for k in -4..=4 {
        for j in -6..=6 {
            let scale = 1.0 + FINE_SCALE * k as f32 / 4.0;
            let shift = start + Vec3::Y * (FINE_UP * j as f32 / 6.0);
            let value = spread(scale, shift);
            if value < best.0 {
                best = (value, scale, shift);
            }
        }
    }
    let lowest = Vec3::new(-FINE_ACROSS, -FINE_UP, -FINE_ACROSS) + start;
    let highest = Vec3::new(FINE_ACROSS, FINE_UP, FINE_ACROSS) + start;
    let mut steps = [0.01f32, 0.01, 0.01, 0.01];
    while steps.iter().any(|s| *s > 0.002) {
        for (axis, step) in steps.iter_mut().enumerate() {
            if *step <= 0.002 {
                continue;
            }
            let mut moved = false;
            for sign in [-1.0f32, 1.0] {
                let (_, mut scale, mut shift) = best;
                if axis == 0 {
                    scale = (scale + sign * *step).clamp(1.0 - FINE_SCALE, 1.0 + FINE_SCALE);
                } else {
                    shift[axis - 1] += sign * *step;
                    shift = shift.clamp(lowest, highest);
                }
                let value = spread(scale, shift);
                if value < best.0 {
                    best = (value, scale, shift);
                    moved = true;
                    break;
                }
            }
            if !moved {
                *step /= 2.0;
            }
        }
    }
    let (mut after, mut scale, mut shift) = best;
    if after > FINE_GAIN * before || before - after < FINE_GAIN_LEAST {
        (after, scale, shift) = (before, 1.0, start);
    }
    let offset = middle * (1.0 - scale) + shift;
    garment.transform(scale, offset);
    Alignment {
        scale: units * scale,
        offset,
        units,
        spread_before: before,
        spread_after: after,
    }
}

/// The body's box over the heights `low` to `high` (its whole box when it
/// has nothing there).
fn band(body: &BodyMesh, low: f32, high: f32) -> (Vec3, Vec3) {
    let fold = |inside: &dyn Fn(&Vec3) -> bool| {
        body.positions
            .iter()
            .filter(|p| inside(p))
            .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            })
    };
    let (lo, hi) = fold(&|p| p.y >= low && p.y <= high);
    if lo.x.is_finite() {
        (lo, hi)
    } else {
        fold(&|_| true)
    }
}

/// The units a garment whose box is `lo` to `hi` is in: see [`align`].
fn units_of(lo: Vec3, hi: Vec3, body: &BodyMesh) -> f32 {
    let (body_lo, body_hi) = band(body, f32::NEG_INFINITY, f32::INFINITY);
    let tallest = (body_hi.y - body_lo.y) * 1.15;
    let size = (hi - lo).max(Vec3::splat(1e-6));
    let mut best = (f32::INFINITY, 1.0);
    for units in UNITS {
        if size.y * units > tallest {
            continue;
        }
        let (band_lo, band_hi) = band(body, lo.y * units, hi.y * units);
        let band_size = (band_hi - band_lo).max(Vec3::splat(1e-6));
        let misfit =
            (size.x * units / band_size.x).ln().abs() + (size.z * units / band_size.z).ln().abs();
        if misfit < best.0 {
            best = (misfit, units);
        }
    }
    best.1
}

/// How unevenly `samples`, scaled by `scale` about `centre` and moved by
/// `shift`, stand off `surface`: the root mean square distance of their
/// clearances from the median, plus three times how far they are inside it on average,
/// plus 5 cm for each share of them not over the skin (farther than 12 cm
/// from it, or beside it: a part sunk into the body or pulled off it).
fn unevenness(surface: &Surface, samples: &[Vec3], centre: Vec3, scale: f32, shift: Vec3) -> f32 {
    let mut clearances: Vec<f32> = samples
        .iter()
        .filter_map(|p| {
            let p = centre + (*p - centre) * scale + shift;
            let hit = surface.closest(p, 0.12)?;
            hit.is_over(p).then(|| hit.signed(p))
        })
        .collect();
    if clearances.is_empty() {
        return f32::INFINITY;
    }
    let off = 1.0 - clearances.len() as f32 / samples.len() as f32;
    clearances.sort_by(f32::total_cmp);
    let median = clearances[clearances.len() / 2];
    let count = clearances.len() as f32;
    let spread = (clearances.iter().map(|c| (c - median).powi(2)).sum::<f32>() / count).sqrt();
    let inside = clearances.iter().map(|c| (-c).max(0.0)).sum::<f32>() / count;
    spread + 3.0 * inside + 0.05 * off
}

/// The combinations of `base`'s body shape weights a garment must clear,
/// as the sliders check (`assets/characters/humanoid/qa/measure.py`,
/// `slider_pairs`) pairs the body's parameters: on both frames (when the
/// base has a `frame` parameter), each build parameter (overall and by
/// region) at either end alone, and with every other build parameter at
/// either end; and every body preset on both frames. Mask shapes are left
/// out. For one garment, [`extremes_near`](super::extremes_near) keeps
/// what can reach it.
pub fn extremes_of(base: &BaseManifest, library: &Library) -> Vec<Vec<(String, f32)>> {
    let frames: Vec<Option<f32>> = match base.parameter("frame") {
        Some(frame) => vec![Some(frame.range[0]), Some(frame.range[1])],
        None => vec![None],
    };
    let build: Vec<&crate::Parameter> = base
        .parameters
        .iter()
        .filter(|p| p.id != "frame" && p.choices.is_empty())
        .filter(|p| matches!(p.group.as_str(), "build" | "build_regions"))
        .collect();
    let mut sets: Vec<Vec<(String, f32)>> = Vec::new();
    for frame in &frames {
        let at = |values: &[(&str, f32)]| {
            let mut description = Description::new("extreme", &base.id);
            if let Some(frame) = frame {
                description.parameters.insert("frame".to_owned(), *frame);
            }
            for (id, value) in values {
                description.parameters.insert((*id).to_owned(), *value);
            }
            weights_of(base, &description, library)
        };
        for (i, a) in build.iter().enumerate() {
            for end_a in a.range {
                sets.push(at(&[(a.id.as_str(), end_a)]));
                for b in &build[i + 1..] {
                    for end_b in b.range {
                        sets.push(at(&[(a.id.as_str(), end_a), (b.id.as_str(), end_b)]));
                    }
                }
            }
        }
        for preset in library.presets.values() {
            if preset.base != base.id
                || preset.preset().map(|p| p.kind) != Some(crate::PresetKind::Body)
            {
                continue;
            }
            let mut description = Description::new("extreme", &base.id);
            description.parameters.extend(preset.parameters.clone());
            if let Some(frame) = frame {
                description.parameters.insert("frame".to_owned(), *frame);
            }
            sets.push(weights_of(base, &description, library));
        }
    }
    sets.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    sets.dedup();
    sets
}

/// The body states a garment on `base` is posed in when it is fitted
/// ([`FitOptions::pose_states`](super::FitOptions)): on both ends of the
/// frame, the frame alone, the build's four corners (muscle and fat at
/// either end), and every body preset. (The posed solve checks every pose
/// on each, so these are a few bodies far apart, not the [`extremes_of`]
/// pairs.)
pub fn pose_states_of(base: &BaseManifest, library: &Library) -> Vec<Vec<(String, f32)>> {
    pose_states_for_frame(base, library, HomeFrame::Both)
}

/// Frame-scoped pose states for `home_frame`: masculine garments prove coverage
/// on masculine states, feminine garments on feminine states, and `Both` on both.
pub fn pose_states_for_frame(
    base: &BaseManifest,
    library: &Library,
    home_frame: HomeFrame,
) -> Vec<Vec<(String, f32)>> {
    let frames: Vec<Option<f32>> = match base.parameter("frame") {
        Some(frame) => match home_frame {
            HomeFrame::Masculine => vec![Some(frame.range[0])],
            HomeFrame::Feminine => vec![Some(frame.range[1])],
            HomeFrame::Both => vec![Some(frame.range[0]), Some(frame.range[1])],
        },
        None => vec![None],
    };
    let mut sets: Vec<Vec<(String, f32)>> = Vec::new();
    for frame in &frames {
        let mut description = Description::new("pose", &base.id);
        if let Some(frame) = frame {
            description.parameters.insert("frame".to_owned(), *frame);
        }
        sets.push(weights_of(base, &description, library));
        if base.parameter("muscle").is_some() && base.parameter("fat").is_some() {
            for muscle in [-1.0, 1.0] {
                for fat in [-1.0, 1.0] {
                    let mut corner = description.clone();
                    corner.parameters.insert("muscle".to_owned(), muscle);
                    corner.parameters.insert("fat".to_owned(), fat);
                    sets.push(weights_of(base, &corner, library));
                }
            }
        }
        for preset in library.presets.values() {
            if preset.base != base.id
                || preset.preset().map(|p| p.kind) != Some(crate::PresetKind::Body)
            {
                continue;
            }
            let mut description = Description::new("pose", &base.id);
            description.parameters.extend(preset.parameters.clone());
            if let Some(frame) = frame {
                description.parameters.insert("frame".to_owned(), *frame);
            }
            sets.push(weights_of(base, &description, library));
        }
    }
    sets.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    sets.dedup();
    sets
}

fn weights_of(
    base: &BaseManifest,
    description: &Description,
    library: &Library,
) -> Vec<(String, f32)> {
    body_shape_weights(base, description, library)
        .into_iter()
        .filter(|(name, weight)| *weight != 0.0 && !is_mask_shape(name))
        .collect()
}

/// The library's base manifest `id`, its manifest's path and the body's
/// `MeshId` inside the library.
fn base_of<'a>(library: &'a Library, id: &str) -> Result<(&'a BaseManifest, String), FitError> {
    let base = library
        .base(id)
        .ok_or_else(|| library_error(format!("the library has no base model `{id}`")))?;
    let file = library
        .path_of(id, &base.model.file)
        .ok_or_else(|| library_error("the base model's file isn't inside the library"))?;
    Ok((base, format!("{file}#{}", base.model.body)))
}

/// The base body's region map, if it has one.
pub(crate) fn region_map(library: &Library, root: &Path, base: &BaseManifest) -> Option<RegionMap> {
    let path = library.path_of(&base.id, base.model.regions_map.as_deref()?)?;
    let image = axcenity_assets::read_texture(root, &path).ok()?;
    Some(RegionMap::from_rgba(
        image.width,
        image.height,
        image.rgba(),
    ))
}

/// Imports a garment: see the module documentation. Returns what was
/// written; nothing is written when the garment can't be read or fitted.
pub fn import_garment(request: &ImportRequest) -> Result<Imported, FitError> {
    let root = &request.library;
    let (library, catalogue, problems) = Library::read(root);
    let mut catalogue = catalogue.ok_or_else(|| {
        library_error(format!(
            "`{}` isn't a character library: {problems:?}",
            root.display()
        ))
    })?;
    let (base, body_id) = base_of(&library, &request.base)?;
    let assets = Assets::new(root);
    let body_model = assets
        .model_blocking(&body_id)
        .ok_or_else(|| FitError::Model(format!("the body `{body_id}` doesn't load")))?;
    let body = BodyMesh::from_model(&body_model)?;

    let source = axcenity_assets::load_model_file(&request.source)
        .map_err(|err| FitError::Model(format!("`{}`: {err}", request.source.display())))?;
    let mut garment = GarmentMesh::from_model(&source);
    if garment.is_empty() {
        return Err(FitError::Empty);
    }
    let alignment = if request.skip_align {
        Alignment {
            scale: 1.0,
            offset: Vec3::ZERO,
            units: 1.0,
            spread_before: 0.0,
            spread_after: 0.0,
        }
    } else {
        align(&mut garment, &body)
    };
    let validation = validate_garment(&garment, request.margin);
    if request.strict_validate && !validation.is_clean() {
        return Err(FitError::Model(format!(
            "`{}` failed canonical source validation: {}",
            request.source.display(),
            validation.warnings.join("; ")
        )));
    }

    let map = region_map(&library, root, base)
        .ok_or_else(|| library_error(format!("`{}` has no region map", base.id)))?;
    let regions = triangle_regions(&body, &map);

    let options = FitOptions {
        margin: request.margin,
        layer: request.layer.unwrap_or(1),
        home_frame: request.home_frame,
        proxy_kind: request.proxy_kind,
        sleeveless: request.sleeveless,
        shapes: carried_shapes(base, &body),
        poses: fit_poses(&body.rig, request.finish_pose),
        pose_states: pose_states_for_frame(base, &library, request.home_frame),
        generate_shrink_under: request.generate_shrink_under,
        ..FitOptions::default()
    };
    let fitted = fit_garment(&body, &garment, &options)?;

    // For `HomeFrame::Feminine`, evaluate coverage, pose_safe_hides, and cover_map
    // against the feminine rest body (with `Frame_Feminine` baked into rest and
    // removed from shapes) so feminine garments are never tested against the wider
    // masculine rest body.
    let home_body: BodyMesh = if request.home_frame == HomeFrame::Feminine {
        let mut hb = body.clone();
        let hw = request.home_frame.rest_weights();
        let (hp, hn) = body.shaped(&hw);
        hb.positions = hp;
        hb.normals = hn;
        hb.shapes.retain(|s| s.name != "Frame_Feminine");
        hb
    } else {
        body.clone()
    };

    // What it covers: its regions and slot.
    let surface = Surface::new(
        fitted.garment.positions(),
        fitted.garment.normals(),
        fitted.garment.triangles(),
    );
    let covered = coverage(&home_body, &regions, &surface);
    // Hidden only where the cloth keeps the skin covered in every pose:
    // hidden skin out from under the cloth is a hole.
    let (hides, unhidden) = pose_safe_hides(
        base,
        &home_body,
        &fitted,
        &regions,
        hidden_regions(base, &covered),
        &options.poses,
        &options.pose_states,
    );
    let guess = classify(base, &covered);
    let slot = match (&request.slot, &guess) {
        (Some(slot), _) => slot.clone(),
        (None, Some(guess)) => guess.slot.clone(),
        (None, None) => {
            return Err(library_error(
                "it covers no region a slot is for (torso, hips, thighs or feet): give its slot",
            ));
        }
    };
    let mut also_fills = match (&request.slot, guess) {
        (None, Some(guess)) => guess.also_fills,
        _ => Vec::new(),
    };
    for other in &request.also_fills {
        if *other != slot && !also_fills.contains(other) {
            also_fills.push(other.clone());
        }
    }
    let mut tags = vec!["imported".to_owned()];
    for tag in &request.tags {
        if !tags.contains(tag) {
            tags.push(tag.clone());
        }
    }

    // The files.
    let object = object_name(&request.label);
    let armature = base
        .model
        .body
        .rsplit_once('/')
        .map_or("Armature", |(armature, _)| armature);
    let folder = root.join(&request.folder);
    std::fs::create_dir_all(&folder).map_err(|err| FitError::Write {
        path: folder.display().to_string(),
        message: err.to_string(),
    })?;
    let write = |name: &str, bytes: &[u8]| -> Result<(), FitError> {
        let path = folder.join(name);
        std::fs::write(&path, bytes).map_err(|err| FitError::Write {
            path: path.display().to_string(),
            message: err.to_string(),
        })
    };
    let glb = write_fitted_glb(&FittedModel {
        fitted: &fitted,
        rig: &body.rig,
        armature,
        object: &object,
        shading: "cloth",
        maps: &match request.texture_size {
            Some(size) => source_maps(&request.source, &fitted.garment, size),
            None => Vec::new(),
        },
        skin_uvs: &skin_uvs(&body, &fitted.binding),
        // Drop sub-0.1 mm sparse delta tails to keep fitted `.glb` files compact.
        least_move: 1e-4,
    });
    write(&format!("{object}.glb"), &glb)?;
    let mut binding = fitted.binding.clone();
    binding.body = body_id.clone();
    write(
        &format!("{object}.binding.json"),
        binding.to_json().as_bytes(),
    )?;
    // What it hides when worn: the skin under it, and the cloth of what
    // is worn beneath it (ADR-0045).
    let rest = fitted.garment.positions();
    let cloth_at = |weights: &[(String, f32)]| garment_at(&rest, &fitted.shapes, weights);
    let (cover, cover_report) = cover_map(
        &home_body,
        (&regions, &round_the_hips(base)),
        &surface,
        (&fitted.joints, &fitted.weights),
        Some(&Proof {
            poses: &options.poses,
            states: &options.pose_states,
            cloth_at: &cloth_at,
        }),
        COVER_SIZE,
    );
    let cover_map =
        (cover_report.skin > 0 || cover_report.cloth > 0).then(|| format!("{object}.cover.png"));
    if let Some(name) = &cover_map {
        write_png(&folder.join(name), cover.size, cover.size, &cover.rgba)?;
    }
    const THUMBNAIL: u32 = 256;
    let picture = thumbnail(&fitted.garment, THUMBNAIL);
    write_png(
        &folder.join(format!("{object}.png")),
        THUMBNAIL,
        THUMBNAIL,
        &picture,
    )?;

    let mut materials: Vec<MaterialSlot> = Vec::new();
    for (index, piece) in fitted.garment.pieces.iter().enumerate() {
        let name = if piece.material.is_empty() {
            format!("Cloth{}", index + 1)
        } else {
            piece.material.clone()
        };
        if materials.iter().all(|m| m.name != name) {
            materials.push(MaterialSlot::new(name, ShadingModel::Cloth));
        }
    }
    let manifest = ItemManifest {
        format_version: crate::ITEM_VERSION,
        id: request.id.clone(),
        label: request.label.clone(),
        kind: ItemKind::Garment,
        slot,
        layer: Some(request.layer.unwrap_or(1)),
        tier: ContentTier::Premium,
        bases: vec![base.id.clone()],
        model: Some(ModelRef {
            file: format!("{object}.glb"),
            object: Some(format!("{armature}/{object}")),
        }),
        hides,
        cover_map,
        materials,
        variants: Vec::new(),
        fit: Fit::Binding {
            data: Some(format!("{object}.binding.json")),
            margin: request.margin,
        },
        groom: None,
        tags,
        thumbnail: Some(format!("{object}.png")),
        source: Some(request.source_info.clone()),
        about: request.about.clone(),
        parameters: Vec::new(),
        also_fills,
        requires: Vec::new(),
        excludes: Vec::new(),
        limitations: request.limitations.clone(),
        adjust: None,
        quality: Some(Quality::Draft),
    };
    let mut problems = manifest.validate();
    problems.extend(manifest.check_against(base));
    if !problems.is_empty() {
        return Err(library_error(format!(
            "the item made doesn't check: {problems:?}"
        )));
    }
    let manifest_name = format!("{object}.item.json");
    write(&manifest_name, manifest.to_json().as_bytes())?;
    let manifest_path = format!("{}/{manifest_name}", request.folder.trim_end_matches('/'));

    // Listed in the catalogue: in place of an earlier import of the same
    // ID, or at the end (the catalogue's own order is kept).
    let entry = CatalogueItem::of(&manifest_path, &manifest);
    match catalogue.items.iter_mut().find(|e| e.id == manifest.id) {
        Some(earlier) => *earlier = entry,
        None => catalogue.items.push(entry),
    }
    std::fs::write(root.join("catalogue.json"), catalogue.to_json()).map_err(|err| {
        FitError::Write {
            path: "catalogue.json".to_owned(),
            message: err.to_string(),
        }
    })?;

    let coverage = base
        .regions
        .iter()
        .filter_map(|region| {
            let index = region.map_index?;
            Some((region.id.clone(), covered.share(index)))
        })
        .collect();
    Ok(Imported {
        manifest,
        manifest_path,
        alignment,
        report: fitted.report,
        coverage,
        unhidden,
        cover: cover_report,
    })
}

/// Makes the cover map of the library's item `id` again from its fitted
/// model (ADR-0045), without fitting it again: for an item imported
/// before cover maps, or after the rule for them changed. Writes
/// `<model>.cover.png` beside the manifest and names it there. The item
/// must be fitted by binding to a base model with a region map; a model
/// made before the fit wrote each corner's place on the skin still gets
/// its map (it hides what is under it, and is not hidden itself).
pub fn remake_cover_map(
    root: &Path,
    id: &str,
    finish_pose: Option<FinishPose>,
) -> Result<CoverReport, FitError> {
    let (library, _, _) = Library::read(root);
    let item = library
        .item(id)
        .ok_or_else(|| library_error(format!("the library has no item `{id}`")))?;
    let model = item
        .model
        .as_ref()
        .ok_or_else(|| library_error(format!("`{id}` has no model")))?;
    let base_id = item
        .bases
        .first()
        .ok_or_else(|| library_error(format!("`{id}` fits no base model")))?;
    let (base, body_id) = base_of(&library, base_id)?;
    let assets = Assets::new(root);
    let body_model = assets
        .model_blocking(&body_id)
        .ok_or_else(|| FitError::Model(format!("the body `{body_id}` doesn't load")))?;
    let body = BodyMesh::from_model(&body_model)?;
    let file = library
        .path_of(id, &model.file)
        .ok_or_else(|| library_error(format!("`{id}`'s model isn't in the library")))?;
    let mesh_id = match &model.object {
        Some(object) => format!("{file}#{object}"),
        None => file.clone(),
    };
    let loaded = assets
        .model_blocking(&mesh_id)
        .ok_or_else(|| FitError::Model(format!("`{mesh_id}` doesn't load")))?;
    // The fitted model as a skinned mesh in the file's space: its corners
    // and their skin weights.
    let worn = BodyMesh::from_model(&loaded)?;
    let surface = Surface::new(
        worn.positions.clone(),
        worn.normals.clone(),
        worn.triangles.clone(),
    );
    let map = region_map(&library, root, base)
        .ok_or_else(|| library_error(format!("`{}` has no region map", base.id)))?;
    let regions = triangle_regions(&body, &map);
    // Tried in the fit's poses, the garment's own copies of the body's
    // shapes putting its corners where the fit left them.
    let poses = fit_poses(&body.rig, finish_pose);
    let states = pose_states_of(base, &library);
    let cloth_at = |weights: &[(String, f32)]| worn.shaped(weights).0;
    let (cover, report) = cover_map(
        &body,
        (&regions, &round_the_hips(base)),
        &surface,
        (&worn.joints, &worn.weights),
        Some(&Proof {
            poses: &poses,
            states: &states,
            cloth_at: &cloth_at,
        }),
        COVER_SIZE,
    );
    let manifest_path = library
        .manifest_path(id)
        .ok_or_else(|| library_error(format!("`{id}` has no manifest")))?;
    let folder = root.join(manifest_path);
    let folder = folder.parent().unwrap_or(root);
    let stem = Path::new(&model.file)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("item");
    let name = format!("{stem}.cover.png");
    write_png(&folder.join(&name), cover.size, cover.size, &cover.rgba)?;
    if item.cover_map.as_deref() != Some(name.as_str()) {
        let mut manifest = item.clone();
        manifest.cover_map = Some(name);
        let path = root.join(manifest_path);
        std::fs::write(&path, manifest.to_json()).map_err(|err| FitError::Write {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
    }
    Ok(report)
}

/// The texture maps of each of `garment`'s pieces, read from the `.glb`
/// at `source` by the piece's material name, each made no larger than
/// `size` pixels a side and encoded as JPEG. A file that isn't a `.glb`,
/// or a map that can't be read, gives none (the item keeps its plain
/// colour).
fn source_maps(source: &Path, garment: &GarmentMesh, size: u32) -> Vec<PieceMaps> {
    let none = || vec![PieceMaps::default(); garment.pieces.len()];
    let Ok(bytes) = std::fs::read(source) else {
        return none();
    };
    // A `.glb`: a 12-byte header, then a JSON chunk and a binary chunk,
    // each with an 8-byte header (its length and kind).
    let word = |at: usize| -> Option<usize> {
        Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?) as usize)
    };
    if bytes.get(..4) != Some(b"glTF") {
        return none();
    }
    let Some(json_length) = word(12) else {
        return none();
    };
    let Some(document) = bytes
        .get(20..20 + json_length)
        .and_then(|text| serde_json::from_slice::<serde_json::Value>(text).ok())
    else {
        return none();
    };
    let bin_start = 20 + json_length + 8;
    let image_bytes = |texture: &serde_json::Value| -> Option<&[u8]> {
        let texture = &document["textures"][texture["index"].as_u64()? as usize];
        let image = &document["images"][texture["source"].as_u64()? as usize];
        let view = &document["bufferViews"][image["bufferView"].as_u64()? as usize];
        let start = bin_start + view["byteOffset"].as_u64().unwrap_or(0) as usize;
        bytes.get(start..start + view["byteLength"].as_u64()? as usize)
    };
    let shrunk = |texture: &serde_json::Value, quality: u8| -> Option<Vec<u8>> {
        let image = image::load_from_memory(image_bytes(texture)?).ok()?;
        let image = if image.width().max(image.height()) > size {
            image.resize(size, size, image::imageops::FilterType::Triangle)
        } else {
            image
        };
        let mut out = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
        image.to_rgb8().write_with_encoder(encoder).ok()?;
        Some(out)
    };
    garment
        .pieces
        .iter()
        .map(|piece| {
            let Some(material) = document["materials"]
                .as_array()
                .and_then(|list| list.iter().find(|m| m["name"] == piece.material.as_str()))
            else {
                return PieceMaps::default();
            };
            let pbr = &material["pbrMetallicRoughness"];
            PieceMaps {
                colour: shrunk(&pbr["baseColorTexture"], 90),
                normal: shrunk(&material["normalTexture"], 93),
                metal_rough: shrunk(&pbr["metallicRoughnessTexture"], 88),
            }
        })
        .collect()
}

/// An object name from a label: its words run together, each capitalised
/// (`Test tee` is `TestTee`), letters and digits only.
fn object_name(label: &str) -> String {
    let name: String = label
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect();
    if name.is_empty() {
        "Garment".to_owned()
    } else {
        name
    }
}
