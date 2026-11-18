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

use super::coverage::{RegionMap, classify, coverage, hidden_regions, triangle_regions};
use super::glb::{FittedModel, write_fitted_glb};
use super::surface::Surface;
use super::thumbnail::{thumbnail, write_png};
use super::transfer::{FitOptions, FitReport, Layer, carried_shapes, fit_garment};
use super::{BodyMesh, FitError, GarmentMesh, is_mask_shape};
use crate::{
    BaseManifest, CatalogueItem, ContentTier, Description, Fit, ItemKind, ItemManifest, Library,
    MaterialSlot, ModelRef, Quality, ShadingModel, Source, body_shape_weights,
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
    /// Its clearance from the body and lower layers (metres).
    pub margin: f32,
    /// Items it is worn over (by ID): it is pushed clear of each.
    pub under: Vec<String>,
    /// A sentence for the browser.
    pub about: Option<String>,
}

/// How the garment was moved onto the body: each corner `p` became
/// `p * scale + offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alignment {
    pub scale: f32,
    pub offset: Vec3,
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
}

fn library_error(message: impl Into<String>) -> FitError {
    FitError::Library(message.into())
}

/// Aligns `garment` to `body`: its units to the body's (a file in
/// centimetres or millimetres is scaled down), and centred on the body
/// across and front to back over the heights it spans. Its height is
/// kept: it is made in the body's rest pose.
pub fn align(garment: &mut GarmentMesh, body: &BodyMesh) -> Alignment {
    let (body_lo, body_hi) = body
        .positions
        .iter()
        .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(*p), hi.max(*p))
        });
    let (lo, hi) = garment.bounds();
    let tall = (hi.y - lo.y).max(1e-6);
    let body_tall = (body_hi.y - body_lo.y).max(1e-6);
    let scale = match tall / body_tall {
        r if r > 200.0 => 0.001,
        r if r > 20.0 => 0.01,
        _ => 1.0,
    };
    let (lo, hi) = (lo * scale, hi * scale);
    // The body's middle, across and front to back, at the garment's
    // heights.
    let band: Vec<&Vec3> = body
        .positions
        .iter()
        .filter(|p| p.y >= lo.y && p.y <= hi.y)
        .collect();
    let (band_lo, band_hi) = if band.is_empty() {
        (body_lo, body_hi)
    } else {
        band.iter()
            .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(a, b), p| {
                (a.min(**p), b.max(**p))
            })
    };
    let body_middle = (band_lo + band_hi) / 2.0;
    let middle = (lo + hi) / 2.0;
    let offset = Vec3::new(body_middle.x - middle.x, 0.0, body_middle.z - middle.z);
    garment.transform(scale, offset);
    Alignment { scale, offset }
}

/// The combinations of `base`'s body shape weights a garment must clear:
/// each build parameter (overall and by region) at either end, the
/// overall muscle and fat ends together, and every body preset, each on
/// both frames (when the base has a `frame` parameter). Mask shapes are
/// left out.
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
        let start = || {
            let mut description = Description::new("extreme", &base.id);
            if let Some(frame) = frame {
                description.parameters.insert("frame".to_owned(), *frame);
            }
            description
        };
        for parameter in &build {
            for end in parameter.range {
                let mut description = start();
                description.parameters.insert(parameter.id.clone(), end);
                sets.push(weights_of(base, &description, library));
            }
        }
        if let (Some(muscle), Some(fat)) = (base.parameter("muscle"), base.parameter("fat")) {
            for m in muscle.range {
                for f in fat.range {
                    let mut description = start();
                    description.parameters.insert("muscle".to_owned(), m);
                    description.parameters.insert("fat".to_owned(), f);
                    sets.push(weights_of(base, &description, library));
                }
            }
        }
        for preset in library.presets.values() {
            if preset.base != base.id
                || preset.preset().map(|p| p.kind) != Some(crate::PresetKind::Body)
            {
                continue;
            }
            let mut description = start();
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
    let alignment = align(&mut garment, &body);

    // The layers it is worn over.
    let mut under = Vec::new();
    for id in &request.under {
        let item = library
            .item(id)
            .ok_or_else(|| library_error(format!("the library has no item `{id}`")))?;
        let model = item
            .model
            .as_ref()
            .ok_or_else(|| library_error(format!("`{id}` has no model")))?;
        let file = library
            .path_of(id, &model.file)
            .ok_or_else(|| library_error(format!("`{id}`'s model isn't in the library")))?;
        let mesh_id = match &model.object {
            Some(object) => format!("{file}#{object}"),
            None => file,
        };
        let loaded = assets
            .model_blocking(&mesh_id)
            .ok_or_else(|| FitError::Model(format!("`{mesh_id}` doesn't load")))?;
        under.push(Layer::of_body_mesh(id, &BodyMesh::from_model(&loaded)?));
    }

    let options = FitOptions {
        margin: request.margin,
        shapes: carried_shapes(base, &body),
        under,
        extremes: extremes_of(base, &library),
        ..FitOptions::default()
    };
    let fitted = fit_garment(&body, &garment, &options)?;

    // What it covers: its regions and slot.
    let surface = Surface::new(
        fitted.garment.positions(),
        fitted.garment.normals(),
        fitted.garment.triangles(),
    );
    let map = region_map(&library, root, base)
        .ok_or_else(|| library_error(format!("`{}` has no region map", base.id)))?;
    let regions = triangle_regions(&body, &map);
    let covered = coverage(&body, &regions, &surface);
    let hides = hidden_regions(base, &covered);
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
    let also_fills = match (&request.slot, guess) {
        (None, Some(guess)) => guess.also_fills,
        _ => Vec::new(),
    };

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
    });
    write(&format!("{object}.glb"), &glb)?;
    let mut binding = fitted.binding.clone();
    binding.body = body_id.clone();
    write(
        &format!("{object}.binding.json"),
        binding.to_json().as_bytes(),
    )?;
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
        materials,
        variants: Vec::new(),
        fit: Fit::Binding {
            data: Some(format!("{object}.binding.json")),
            margin: request.margin,
        },
        groom: None,
        tags: vec!["imported".to_owned()],
        thumbnail: Some(format!("{object}.png")),
        source: Some(request.source_info.clone()),
        about: request.about.clone(),
        parameters: Vec::new(),
        also_fills,
        requires: Vec::new(),
        excludes: Vec::new(),
        limitations: Vec::new(),
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

    // Listed in the catalogue, replacing an earlier import of the same ID.
    catalogue.items.retain(|entry| entry.id != manifest.id);
    catalogue
        .items
        .push(CatalogueItem::of(&manifest_path, &manifest));
    catalogue.items.sort_by(|a, b| a.id.cmp(&b.id));
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
    })
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
