//! Group as Mesh: parts (MeshParts, Parts, UnionOperations) made into one
//! MeshPart whose model is one new `.glb` with as few triangles as look
//! exactly the same ([`crate::optimize`]).
//!
//! Unlike Union, nothing is cut, and no part has to be closed: each part's
//! triangles are put in the first part's frame with their own looks (the
//! model's materials, textures and uvs; a Part's color, metalness,
//! roughness and see-through), parts that look the same share one
//! material, and the optimizer takes away what can't be seen. What each
//! part collided as is kept as the model's own collision pieces (`_COL`
//! objects), so the grouped part collides as its parts did.

use std::path::Path;

use axcenity_assets::{
    AlphaMode, Assets, Channel, Image, MODELS_FOLDER, MapSettings, ModelMaterial, StandardMaterial,
};
use axcenity_assets::{safe_file_name, unique_file_name};
use axcenity_core::{ClassName, Color3, Instance, InstanceId, Material, Value, Vector3, World};
use glam::{EulerRot, Quat, Vec2, Vec3};
use serde_json::json;

use crate::generate::{part_mesh, srgb_to_linear};
use crate::glb::{EmbeddedImage, EmbeddedMaps, File, MaterialMaps, Vertices, surface_triangles};
use crate::optimize::{LookInfo, OptimizeReport, PartInfo, optimize, used_vertices};
use crate::{Mesh, Surface};

/// The name of the grouped model's one object; its collision pieces are
/// `Group_COL1`, `Group_COL2`, …
const OBJECT: &str = "Group";

/// Whether Group as Mesh preserves each object's materials & textures as
/// separate slots, or overwrites them all with one shared material.
#[derive(Clone, Debug)]
pub enum GroupMode {
    /// Keep every distinct material and texture as its own material slot.
    MultiMaterial,
    /// Overwrite every part's look with this single material.
    SingleMaterial(Box<GroupLook>),
}

/// One look of a grouped part: a model material's values (its own
/// textures left out), linked project texture files (`maps`), any embedded
/// PNG textures (`embedded`), and optional `SurfaceAppearance` properties
/// when a source used maps glTF alone cannot represent.
#[derive(Clone, Debug)]
pub struct GroupLook {
    pub material: ModelMaterial,
    pub maps: MaterialMaps,
    pub embedded: EmbeddedMaps,
    /// Extra `SurfaceAppearance` properties (besides `MaterialName`) to attach
    /// on the grouped `MeshPart` when a source uses DirectX normals, opacity
    /// or specular maps, or separate/inverted metalness & roughness channels.
    pub surface_properties: Option<Vec<(&'static str, Value)>>,
}

impl GroupLook {
    /// A look with `material`'s values and `maps`, its own textures left
    /// out (they're named by `maps`).
    pub fn new(material: &ModelMaterial, maps: MaterialMaps) -> GroupLook {
        GroupLook {
            material: ModelMaterial {
                base_color_texture: None,
                normal_texture: None,
                metalness_texture: None,
                roughness_texture: None,
                occlusion_texture: None,
                emissive_texture: None,
                opacity_texture: None,
                specular_texture: None,
                ..material.clone()
            },
            maps,
            embedded: EmbeddedMaps::default(),
            surface_properties: None,
        }
    }

    /// True if this look uses any linked or embedded texture map.
    pub fn has_textures(&self) -> bool {
        !self.maps.is_empty() || !self.embedded.is_empty() || self.surface_properties.is_some()
    }

    /// A readable label for UI summaries and material slot names.
    pub fn display_name(&self) -> String {
        let name = self.material.name.trim();
        if !name.is_empty() {
            return name.to_owned();
        }
        if let Some(path) = &self.maps.base_color
            && let Some(stem) = Path::new(path).file_stem().and_then(|s| s.to_str())
        {
            return stem.to_owned();
        }
        "Material".to_owned()
    }

    /// A plain untextured Plastic look for single-material overwrites.
    pub fn plain_plastic() -> GroupLook {
        let preset = Material::Plastic.preset();
        let [r, g, b] = linear(Color3::new(0.64, 0.64, 0.64));
        GroupLook::new(
            &ModelMaterial {
                name: "Plastic".to_owned(),
                base_color: [r, g, b, 1.0],
                metalness: preset.metalness as f32,
                roughness: preset.roughness as f32,
                ..ModelMaterial::default()
            },
            MaterialMaps::default(),
        )
    }

    /// Looks with the same key are the same material.
    pub fn key(&self) -> String {
        format!(
            "{:?}\u{0}{:?}\u{0}{:?}\u{0}{:?}",
            self.maps, self.embedded, self.surface_properties, self.material
        )
    }
}

/// A part ready to group: its triangles in its own space at its `Size`
/// (each `Surface::Outer(i)` names `looks[i]`), where it is, and the
/// convex pieces it collides as (corners in its own space).
#[derive(Clone, Debug)]
pub struct GroupSource {
    pub name: String,
    pub mesh: Mesh,
    pub looks: Vec<GroupLook>,
    pub position: Vec3,
    pub rotation: Quat,
    /// False for a part that's wholly see-through (`Transparency` 1): it
    /// isn't drawn, but it still collides.
    pub drawn: bool,
    pub collision: Vec<Vec<Vec3>>,
}

/// The grouped model: its triangles centered on `position` in the frame
/// `rotation`, their looks, its collision pieces, the box it fills (the
/// MeshPart's `Size`) and what the optimizer did.
#[derive(Clone, Debug)]
pub struct Grouped {
    pub mesh: Mesh,
    pub looks: Vec<GroupLook>,
    pub collision: Vec<Vec<Vec3>>,
    pub position: Vec3,
    pub rotation: Quat,
    pub size: Vec3,
    pub stats: GroupStats,
}

/// How much smaller grouping made the parts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupStats {
    pub parts: usize,
    pub triangles_before: usize,
    pub triangles_after: usize,
    pub vertices_before: usize,
    pub vertices_after: usize,
    pub report: OptimizeReport,
}

impl GroupStats {
    /// "Grouped 12 parts: 4,812 → 1,206 triangles, 9,624 → 1,130
    /// vertices", then what each pass took away.
    pub fn summary(&self) -> String {
        let r = &self.report;
        let mut passes = Vec::new();
        for (count, what) in [
            (r.pressed, "pressed together"),
            (r.hidden, "hidden inside"),
            (r.merged, "merged in flat faces"),
            (r.duplicates, "duplicates"),
            (r.degenerate, "with no area"),
        ] {
            if count > 0 {
                passes.push(format!("{} {what}", thousands(count)));
            }
        }
        let mut text = format!(
            "Grouped {} parts: {} → {} triangles, {} → {} vertices",
            self.parts,
            thousands(self.triangles_before),
            thousands(self.triangles_after),
            thousands(self.vertices_before),
            thousands(self.vertices_after),
        );
        if !passes.is_empty() {
            text.push_str(&format!(" (triangles taken away: {})", passes.join(", ")));
        }
        text.push('.');
        text
    }
}

/// `n` with commas between thousands.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (k, digit) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The Part, MeshPart or UnionOperation `part` ready to group, and notes
/// about anything of its look that can't be carried into the model (each
/// a sentence naming the part). An error names the part when it can't be
/// grouped at all: a Capsule, or a model that can't be loaded.
pub fn group_source(
    world: &World,
    assets: &Assets,
    part: InstanceId,
) -> Result<(GroupSource, Vec<String>), String> {
    group_source_with(world, assets, part, |_| None)
}

/// [`group_source`], with a callback `built_in_maps` that can supply embedded
/// procedural PBR textures for built-in Part materials (`Wood`, `Brick`, …)
/// so Parts keep their textures when grouped.
pub fn group_source_with(
    world: &World,
    assets: &Assets,
    part: InstanceId,
    mut built_in_maps: impl FnMut(Material) -> Option<EmbeddedMaps>,
) -> Result<(GroupSource, Vec<String>), String> {
    let instance = world
        .get(part)
        .ok_or_else(|| "A part to group isn't in the game any more".to_owned())?;
    let name = instance.name().to_owned();
    let vector = |property| {
        instance
            .property(property)
            .and_then(Value::as_vector3)
            .map(|v| Vec3::new(v.x, v.y, v.z))
    };
    let number = |property| instance.property(property).and_then(Value::as_number);
    let size = vector("Size").unwrap_or(Vec3::ONE).abs();
    let visible = (1.0 - number("Transparency").unwrap_or(0.0) as f32).clamp(0.0, 1.0);
    let collides = instance.property("CanCollide").and_then(Value::as_bool) != Some(false);
    let mut notes = Vec::new();

    let (mesh, looks, collision) = if instance.class().has_model() {
        let mesh_id = instance
            .property("MeshId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let model = assets
            .model_blocking(mesh_id)
            .ok_or_else(|| format!("`{name}` has no model to group (MeshId `{mesh_id}`)"))?;
        let resolved = assets.resolve(mesh_id);
        let file = resolved.split('#').next().unwrap_or_default();
        let folder = file.rsplit_once('/').map_or("", |(folder, _)| folder);
        // The model in its file's units, stretched to the part's Size.
        let mut mesh = Mesh::from_model(&model);
        let native = Vec3::from(model.size);
        let stretch = Vec3::select(native.cmpgt(Vec3::ZERO), size / native, Vec3::ONE);
        let center = Vec3::from(model.center);
        let scale = Vec3::select(native.cmpgt(Vec3::ZERO), native, Vec3::ONE);
        for position in &mut mesh.positions {
            *position = (*position - center) * stretch;
        }
        for normal in &mut mesh.normals {
            *normal = (*normal / stretch).normalize_or_zero();
        }
        // The loaded model's own corners (fitted into its 1×1×1 cube) in
        // the part's space.
        let fitted = |points: &[[f32; 3]]| -> Vec<Vec3> {
            points
                .iter()
                .map(|&p| Vec3::from(p) * scale * stretch)
                .collect()
        };
        let one_color = (instance.property("UsePartColor").and_then(Value::as_bool) == Some(true))
            .then(|| instance.property("Color").and_then(Value::as_color3))
            .flatten();
        let appearances: Vec<&Instance> = instance
            .children()
            .iter()
            .filter_map(|&child| world.get(child))
            .filter(|child| child.class() == ClassName::SurfaceAppearance)
            .collect();
        let variant_path = instance
            .property("MaterialVariant")
            .and_then(Value::as_str)
            .filter(|variant| !variant.is_empty());
        let variant = variant_path.and_then(|path| assets.material(path));
        if variant_path.is_some() && variant.is_none() {
            notes.push(format!(
                "`{name}`'s MaterialVariant couldn't be loaded: it's grouped in its model's own materials."
            ));
        }
        let looks: Vec<GroupLook> = model
            .primitives
            .iter()
            .map(|primitive| {
                let appearance = appearances.iter().find(|appearance| {
                    appearance.property("MaterialName").and_then(Value::as_str)
                        == Some(primitive.material.name.as_str())
                });
                let mut look = match (appearance, &variant) {
                    (Some(appearance), _) => {
                        let (look, lost) = surface_look(&primitive.material, appearance);
                        if let Some(lost) = lost {
                            notes.push(format!(
                                "`{name}`'s SurfaceAppearance `{}` {lost}",
                                appearance.name()
                            ));
                        }
                        look
                    }
                    (None, Some(standard)) => {
                        variant_look(standard, variant_path.unwrap_or_default(), None)
                    }
                    (None, None) => {
                        let (look, problem) = model_primitive_look(&primitive.material, folder);
                        if let Some(problem) = problem {
                            notes.push(format!(
                                "`{name}`'s material `{}` {problem}",
                                primitive.material.name
                            ));
                        }
                        look
                    }
                };
                if let Some(color) = one_color {
                    let [r, g, b] = linear(color);
                    look.material.base_color = [r, g, b, look.material.base_color[3]];
                    look.maps.base_color = None;
                    look.embedded.base_color = None;
                }
                see_through(&mut look.material, visible);
                look
            })
            .collect();
        if model.rig.is_some() {
            notes.push(format!(
                "`{name}` is animated; it's grouped standing as it does at rest."
            ));
        }
        let fidelity = match instance.property("CollisionFidelity") {
            Some(Value::EnumItem(item)) => item.item.clone(),
            _ => "Default".to_owned(),
        };
        let collision = if !collides {
            Vec::new()
        } else {
            match fidelity.as_str() {
                "Box" => vec![box_corners(size)],
                "Hull" | "PreciseConvexDecomposition" => vec![fitted(&model.hull_points)],
                _ if !model.collision.is_empty() => {
                    model.collision.iter().map(|piece| fitted(piece)).collect()
                }
                _ => vec![fitted(&model.hull_points)],
            }
        };
        (mesh, looks, collision)
    } else if instance.class().is_part() {
        let mut mesh = part_mesh(instance, "grouped")?;
        let kind = instance
            .property("Material")
            .and_then(Value::as_material)
            .unwrap_or(Material::Plastic);
        let appearance = instance.children().iter().find_map(|&child| {
            let child = world.get(child)?;
            (child.class() == ClassName::SurfaceAppearance).then_some(child)
        });
        let variant_path = appearance
            .is_none()
            .then(|| {
                instance
                    .property("MaterialVariant")
                    .and_then(Value::as_str)
                    .filter(|variant| !variant.is_empty())
            })
            .flatten();
        let variant = variant_path.and_then(|path| assets.material(path));
        let part_color = instance.property("Color").and_then(Value::as_color3);
        let mut look = if let Some(appearance) = appearance {
            let base = part_material(instance);
            let (mut look, lost) = surface_look(&base, appearance);
            if let Some(lost) = lost {
                notes.push(format!(
                    "`{name}`'s SurfaceAppearance `{}` {lost}",
                    appearance.name()
                ));
            }
            if look.material.name.is_empty() || look.material.name == kind.as_str() {
                look.material.name = appearance.name().to_owned();
            }
            look
        } else if let Some(standard) = variant {
            variant_look(&standard, variant_path.unwrap_or_default(), part_color)
        } else {
            let mut material = part_material(instance);
            if variant_path.is_some() {
                notes.push(format!(
                    "`{name}`'s MaterialVariant couldn't be loaded: it's grouped in its color."
                ));
            }
            let embedded = built_in_maps(kind).unwrap_or_default();
            if embedded.is_empty() && has_built_in_texture(kind) {
                notes.push(format!(
                    "`{name}`'s {} texture isn't carried: it's grouped in its color.",
                    kind.as_str()
                ));
            } else if !embedded.is_empty() {
                material.occlusion_strength = 0.75;
            }
            let mut look = GroupLook::new(&material, MaterialMaps::default());
            look.embedded = embedded;
            look
        };
        see_through(&mut look.material, visible);
        if look.has_textures() {
            let tile = number("TileSize").unwrap_or(8.0) as f32;
            if tile.is_finite() && tile > 0.0 {
                tile_part_uvs(&mut mesh, tile);
            }
        }
        let collision = if collides {
            vec![distinct(&mesh.positions)]
        } else {
            Vec::new()
        };
        (mesh, vec![look], collision)
    } else {
        return Err(format!("`{name}` isn't a part, so it can't be grouped"));
    };
    if instance
        .children()
        .iter()
        .filter_map(|&child| world.get(child))
        .any(|child| child.class() == ClassName::Decal)
    {
        notes.push(format!(
            "`{name}`'s Decals stay with it but aren't drawn on the grouped part."
        ));
    }

    let orientation = instance
        .property("Orientation")
        .and_then(Value::as_vector3)
        .unwrap_or_default();
    let [x, y, z] = [orientation.x, orientation.y, orientation.z].map(f32::to_radians);
    Ok((
        GroupSource {
            name,
            mesh,
            looks,
            position: vector("Position").unwrap_or_default(),
            rotation: Quat::from_euler(EulerRot::YXZ, y, x, z),
            drawn: visible > 0.0,
            collision,
        },
        notes,
    ))
}

/// Scales a Part's `[0..1]` per-face UVs so textures repeat every `tile` studs,
/// matching `shader.wgsl`'s `DrawItem::uv_tile` projection.
fn tile_part_uvs(mesh: &mut Mesh, tile: f32) {
    let n = mesh.positions.len();
    let mut scales: Vec<Option<Vec2>> = vec![None; n];
    for tri in &mesh.triangles {
        let [i0, i1, i2] = tri.corners.map(|c| c as usize);
        if i0 >= n
            || i1 >= n
            || i2 >= n
            || i0 >= mesh.uvs.len()
            || i1 >= mesh.uvs.len()
            || i2 >= mesh.uvs.len()
        {
            continue;
        }
        let dp1 = mesh.positions[i1] - mesh.positions[i0];
        let dp2 = mesh.positions[i2] - mesh.positions[i0];
        let duv1 = mesh.uvs[i1] - mesh.uvs[i0];
        let duv2 = mesh.uvs[i2] - mesh.uvs[i0];
        let det = duv1.x * duv2.y - duv1.y * duv2.x;
        if det.abs() < 1e-6 {
            continue;
        }
        let inv_det = 1.0 / det;
        let tangent = (dp1 * duv2.y - dp2 * duv1.y) * inv_det;
        let bitangent = (dp2 * duv1.x - dp1 * duv2.x) * inv_det;
        let scale = Vec2::new(tangent.length() / tile, bitangent.length() / tile);
        for idx in [i0, i1, i2] {
            scales[idx].get_or_insert(scale);
        }
    }
    for (uv, scale) in mesh.uvs.iter_mut().zip(scales) {
        if let Some(scale) = scale {
            *uv *= scale;
        }
    }
}

/// Converts a loaded model's `ModelMaterial` into a `GroupLook`, preserving
/// both file-linked maps and embedded `.glb` images (encoded as PNG).
fn model_primitive_look(
    material: &ModelMaterial,
    folder: &str,
) -> (GroupLook, Option<&'static str>) {
    let within = |file: &str| {
        if folder.is_empty() {
            file.to_owned()
        } else {
            format!("{folder}/{file}")
        }
    };
    let maps = MaterialMaps::of(material).map(within);
    let encode_if_embedded = |img: Option<&std::sync::Arc<Image>>| -> Option<EmbeddedImage> {
        let img = img?;
        if img.file.is_some() {
            return None;
        }
        EmbeddedImage::from_rgba(img.width, img.height, img.rgba())
    };
    let metal_rough_img = [&material.metalness_texture, &material.roughness_texture]
        .into_iter()
        .flatten()
        .map(|c| &c.image)
        .next();
    let embedded = EmbeddedMaps {
        base_color: encode_if_embedded(material.base_color_texture.as_ref()),
        metal_rough: encode_if_embedded(metal_rough_img),
        normal: encode_if_embedded(material.normal_texture.as_ref()),
        occlusion: encode_if_embedded(material.occlusion_texture.as_ref().map(|c| &c.image)),
        emissive: encode_if_embedded(material.emissive_texture.as_ref()),
    };
    let mut look = GroupLook::new(material, maps);
    look.embedded = embedded;
    let problem = not_carried_without_embedded_check(material);
    (look, problem)
}

/// Converts a `MaterialVariant`'s resolved `StandardMaterial` into a
/// `GroupLook` (optionally tinted by a Part's `Color`).
fn variant_look(standard: &StandardMaterial, path: &str, part_color: Option<Color3>) -> GroupLook {
    let name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.strip_suffix(".material.json").unwrap_or(s))
        .filter(|s| !s.is_empty())
        .unwrap_or("Material")
        .to_owned();
    let tint = part_color.map_or([1.0, 1.0, 1.0], |c| [c.r, c.g, c.b]);
    let srgb = std::array::from_fn(|i| standard.color[i] * tint[i]);
    let base_color = [
        srgb_to_linear(srgb[0]),
        srgb_to_linear(srgb[1]),
        srgb_to_linear(srgb[2]),
        standard.opacity.clamp(0.0, 1.0),
    ];
    let emissive = standard
        .emissive_color
        .map(|c| srgb_to_linear(c) * standard.emissive_strength.max(0.0));
    let alpha_mode = if standard.alpha_cutoff > 0.0 {
        AlphaMode::Mask {
            cutoff: standard.alpha_cutoff,
        }
    } else if base_color[3] < 1.0 {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    };
    let m = &standard.maps;
    let mut maps = MaterialMaps {
        base_color: m.color.as_ref().map(|s| s.path.clone()),
        normal: m.normal.as_ref().map(|s| s.path.clone()),
        emissive: m.emissive.as_ref().map(|s| s.path.clone()),
        occlusion: m
            .occlusion
            .as_ref()
            .filter(|s| s.channel == Channel::Red && !s.invert)
            .map(|s| s.path.clone()),
        metal_rough: None,
    };
    let fits_mr = match (&m.metalness, &m.roughness) {
        (Some(me), Some(ro)) => {
            me.path == ro.path
                && me.channel == Channel::Blue
                && ro.channel == Channel::Green
                && !me.invert
                && !ro.invert
        }
        (Some(me), None) => me.channel == Channel::Blue && !me.invert && standard.roughness == 0.0,
        (None, Some(ro)) => ro.channel == Channel::Green && !ro.invert && standard.metalness == 0.0,
        (None, None) => true,
    };
    let mut metalness = standard.metalness;
    let mut roughness = standard.roughness;
    if fits_mr {
        if m.metalness.is_some() {
            metalness = 1.0;
        }
        if m.roughness.is_some() {
            roughness = 1.0;
        }
        maps.metal_rough = m
            .metalness
            .as_ref()
            .or(m.roughness.as_ref())
            .map(|s| s.path.clone());
    }
    let needs_surface = !fits_mr
        || m.normal.as_ref().is_some_and(|s| s.direct_x)
        || m.occlusion
            .as_ref()
            .is_some_and(|s| s.channel != Channel::Red || s.invert)
        || m.opacity.is_some()
        || m.specular.is_some()
        || emissive.iter().any(|&c| c > 1.0);
    let surface_properties = needs_surface.then(|| variant_surface_properties(standard, srgb));
    let material = ModelMaterial {
        name,
        base_color,
        metalness,
        roughness,
        emissive,
        normal_scale: m.normal.as_ref().map_or(1.0, |s| s.strength),
        occlusion_strength: m.occlusion.as_ref().map_or(1.0, |s| s.strength),
        alpha_mode,
        double_sided: standard.double_sided,
        uv_scale: standard.tiling,
        uv_offset: standard.offset,
        uv_rotation: 0.0,
        ..ModelMaterial::default()
    };
    let mut look = GroupLook::new(&material, maps);
    look.surface_properties = surface_properties;
    look
}

fn channel_to_core(channel: Channel) -> axcenity_core::TextureChannel {
    match channel {
        Channel::Red => axcenity_core::TextureChannel::Red,
        Channel::Green => axcenity_core::TextureChannel::Green,
        Channel::Blue => axcenity_core::TextureChannel::Blue,
        Channel::Alpha => axcenity_core::TextureChannel::Alpha,
    }
}

fn variant_surface_properties(
    standard: &StandardMaterial,
    srgb: [f32; 3],
) -> Vec<(&'static str, Value)> {
    let m = &standard.maps;
    let map_path = |opt: &Option<MapSettings>| {
        Value::String(opt.as_ref().map_or_else(String::new, |s| s.path.clone()))
    };
    let mut props = vec![
        (
            "Color",
            Value::Color3(Color3::new(srgb[0], srgb[1], srgb[2])),
        ),
        ("ColorMap", map_path(&m.color)),
        ("NormalMap", map_path(&m.normal)),
        (
            "NormalStrength",
            Value::Number(f64::from(m.normal.as_ref().map_or(1.0, |s| s.strength))),
        ),
        (
            "NormalMapDirectX",
            Value::Bool(m.normal.as_ref().is_some_and(|s| s.direct_x)),
        ),
        ("MetalnessMap", map_path(&m.metalness)),
        (
            "MetalnessChannel",
            Value::TextureChannel(channel_to_core(
                m.metalness.as_ref().map_or(Channel::Red, |s| s.channel),
            )),
        ),
        ("RoughnessMap", map_path(&m.roughness)),
        (
            "RoughnessChannel",
            Value::TextureChannel(channel_to_core(
                m.roughness.as_ref().map_or(Channel::Red, |s| s.channel),
            )),
        ),
        (
            "InvertRoughness",
            Value::Bool(m.roughness.as_ref().is_some_and(|s| s.invert)),
        ),
        ("AmbientOcclusionMap", map_path(&m.occlusion)),
        (
            "AmbientOcclusionChannel",
            Value::TextureChannel(channel_to_core(
                m.occlusion.as_ref().map_or(Channel::Red, |s| s.channel),
            )),
        ),
        (
            "AmbientOcclusionStrength",
            Value::Number(f64::from(m.occlusion.as_ref().map_or(1.0, |s| s.strength))),
        ),
        ("EmissiveMap", map_path(&m.emissive)),
        (
            "EmissiveStrength",
            Value::Number(f64::from(standard.emissive_strength)),
        ),
        ("OpacityMap", map_path(&m.opacity)),
        (
            "OpacityChannel",
            Value::TextureChannel(channel_to_core(
                m.opacity.as_ref().map_or(Channel::Red, |s| s.channel),
            )),
        ),
        (
            "InvertOpacity",
            Value::Bool(m.opacity.as_ref().is_some_and(|s| s.invert)),
        ),
        ("SpecularMap", map_path(&m.specular)),
        (
            "Tiling",
            Value::Vector2(axcenity_core::Vector2::new(
                standard.tiling[0],
                standard.tiling[1],
            )),
        ),
        (
            "Offset",
            Value::Vector2(axcenity_core::Vector2::new(
                standard.offset[0],
                standard.offset[1],
            )),
        ),
    ];
    props.retain(|(name, _)| !name.is_empty());
    props
}

/// The built-in materials drawn with maps of their own (`scene`'s
/// surfaces), which a model's material can't name without baking.
fn has_built_in_texture(kind: Material) -> bool {
    matches!(
        kind,
        Material::Wood
            | Material::WoodPlanks
            | Material::Marble
            | Material::Granite
            | Material::Concrete
            | Material::Slate
            | Material::Brick
            | Material::Fabric
            | Material::Sand
            | Material::Metal
            | Material::Ice
            | Material::Rubber
    )
}

/// Why some of `material` can't be written into a glTF file (embedded images
/// are now encoded into the `.glb`), or `None` when all of it can.
fn not_carried_without_embedded_check(material: &ModelMaterial) -> Option<&'static str> {
    let channel = |texture: &Option<axcenity_assets::ChannelTexture>, wanted: Channel| {
        texture
            .as_ref()
            .is_some_and(|texture| texture.channel != wanted || texture.invert)
    };
    if material.normal_flip_green {
        return Some(
            "has a DirectX-style normal map, grouped as a glTF one (its bumps look turned).",
        );
    }
    if channel(&material.metalness_texture, Channel::Blue)
        || channel(&material.roughness_texture, Channel::Green)
    {
        return Some(
            "keeps metalness or roughness in other channels than glTF's; they may look different.",
        );
    }
    if material.opacity_texture.is_some() || material.specular_texture.is_some() {
        return Some("has an opacity or specular map, which isn't carried.");
    }
    None
}

/// A model's `material` with the SurfaceAppearance `appearance`'s maps in
/// place of its own, as the scene draws it (`scene`'s `surface_material`),
/// and what of it a glTF file can't hold, finishing a sentence.
fn surface_look(material: &ModelMaterial, appearance: &Instance) -> (GroupLook, Option<String>) {
    use axcenity_core::TextureChannel as C;
    let path = |name| {
        appearance
            .property(name)
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
    };
    let number = |name, default: f32| {
        appearance
            .property(name)
            .and_then(Value::as_number)
            .map_or(default, |n| n as f32)
    };
    let flag = |name| appearance.property(name).and_then(Value::as_bool) == Some(true);
    let channel = |name| {
        appearance
            .property(name)
            .and_then(Value::as_texture_channel)
            .unwrap_or(C::Red)
    };
    let mut lost: Vec<&str> = Vec::new();
    let mut look = GroupLook::new(material, MaterialMaps::default());
    let values = &mut look.material;
    if let Some(tint) = appearance.property("Color").and_then(Value::as_color3) {
        let [r, g, b] = linear(tint);
        values.base_color = [
            values.base_color[0] * r,
            values.base_color[1] * g,
            values.base_color[2] * b,
            values.base_color[3],
        ];
    }
    look.maps.base_color = path("ColorMap");
    look.maps.normal = path("NormalMap");
    values.normal_scale = number("NormalStrength", 1.0);
    if look.maps.normal.is_some() && flag("NormalMapDirectX") {
        lost.push("a DirectX-style normal map (its bumps look turned)");
    }
    // glTF keeps metalness in blue and roughness in green of one texture.
    let metal = path("MetalnessMap");
    let rough = path("RoughnessMap");
    let fits = match (&metal, &rough) {
        (Some(m), Some(r)) => {
            m == r
                && channel("MetalnessChannel") == C::Blue
                && channel("RoughnessChannel") == C::Green
        }
        (Some(_), None) => channel("MetalnessChannel") == C::Blue && values.roughness == 0.0,
        (None, Some(_)) => channel("RoughnessChannel") == C::Green && values.metalness == 0.0,
        (None, None) => true,
    } && !(rough.is_some() && flag("InvertRoughness"));
    if fits {
        if metal.is_some() {
            values.metalness = 1.0;
        }
        if rough.is_some() {
            values.roughness = 1.0;
        }
        look.maps.metal_rough = metal.or(rough);
    } else {
        lost.push("metalness or roughness maps (glTF keeps them in one texture's blue and green)");
    }
    if let Some(occlusion) = path("AmbientOcclusionMap") {
        if channel("AmbientOcclusionChannel") == C::Red {
            look.maps.occlusion = Some(occlusion);
            values.occlusion_strength = number("AmbientOcclusionStrength", 1.0);
        } else {
            lost.push("an ambient occlusion map in another channel than red");
        }
    }
    look.maps.emissive = path("EmissiveMap");
    // A glow map on a material that doesn't glow glows in the map's colors.
    let glow = if look.maps.emissive.is_some() && material.emissive.iter().all(|&c| c <= 0.0) {
        [1.0; 3]
    } else {
        material.emissive
    };
    let strength = number("EmissiveStrength", 1.0).max(0.0);
    values.emissive = glow.map(|c| c * strength);
    if values.emissive.iter().any(|&c| c > 1.0) {
        lost.push("glow brighter than glTF's 1");
    }
    if path("OpacityMap").is_some() || path("SpecularMap").is_some() {
        lost.push("an opacity or specular map");
    }
    let vector = |name, default: [f32; 2]| {
        appearance
            .property(name)
            .and_then(Value::as_vector2)
            .map_or(default, |v| [v.x, v.y])
    };
    values.uv_scale = vector("Tiling", [1.0, 1.0]);
    values.uv_offset = vector("Offset", [0.0, 0.0]);
    values.uv_rotation = 0.0;
    let had_extra = !lost.is_empty();
    if had_extra {
        let mut props: Vec<(&'static str, Value)> = Vec::new();
        for (prop_name, value) in appearance.properties() {
            if prop_name != "MaterialName" {
                props.push((prop_name, value.clone()));
            }
        }
        look.surface_properties = Some(props);
    }
    // Non-glTF maps are carried via attached SurfaceAppearance children on the
    // grouped MeshPart, so no maps are lost.
    (look, None)
}

/// A Part's look as a model material: its `Color`, `Metalness` and
/// `Roughness`, its Material's opacity, and its color as light if the
/// Material glows.
fn part_material(part: &Instance) -> ModelMaterial {
    let color = part
        .property("Color")
        .and_then(Value::as_color3)
        .unwrap_or(Color3::new(0.64, 0.64, 0.64));
    let kind = part
        .property("Material")
        .and_then(Value::as_material)
        .unwrap_or(Material::Plastic);
    let preset = kind.preset();
    let number = |name, default: f64| {
        part.property(name)
            .and_then(Value::as_number)
            .unwrap_or(default) as f32
    };
    let [r, g, b] = linear(color);
    ModelMaterial {
        name: kind.as_str().to_owned(),
        // A glowing part shows its color as light, not lit by the sun.
        base_color: if preset.glows {
            [0.0, 0.0, 0.0, preset.opacity]
        } else {
            [r, g, b, preset.opacity]
        },
        metalness: number("Metalness", preset.metalness),
        roughness: number("Roughness", preset.roughness),
        emissive: if preset.glows { [r, g, b] } else { [0.0; 3] },
        alpha_mode: if preset.opacity < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        ..ModelMaterial::default()
    }
}

/// `color` (sRGB, as picked) in linear light, as glTF wants.
fn linear(color: Color3) -> [f32; 3] {
    [color.r, color.g, color.b].map(srgb_to_linear)
}

/// Makes `material` as see-through as a part showing `visible` of itself
/// (1 minus its `Transparency`).
fn see_through(material: &mut ModelMaterial, visible: f32) {
    if visible < 1.0 {
        material.base_color[3] *= visible;
        material.alpha_mode = AlphaMode::Blend;
    }
}

/// The 8 corners of a box of `size` centered on the origin.
fn box_corners(size: Vec3) -> Vec<Vec3> {
    let half = size / 2.0;
    (0..8)
        .map(|k| {
            let sign = |bit: u32| if k & bit == 0 { -1.0 } else { 1.0 };
            Vec3::new(half.x * sign(1), half.y * sign(2), half.z * sign(4))
        })
        .collect()
}

/// Each of `points` once.
fn distinct(points: &[Vec3]) -> Vec<Vec3> {
    let mut seen = std::collections::HashSet::new();
    points
        .iter()
        .copied()
        .filter(|p| seen.insert(crate::position_key(*p)))
        .collect()
}

/// True if `material` is fully opaque: not see-through or cut out.
fn opaque(material: &ModelMaterial) -> bool {
    material.alpha_mode == AlphaMode::Opaque && material.base_color[3] >= 1.0
}

/// `sources` grouped: put in the first one's frame, optimized, and
/// centered. Its `position` and `rotation` place it where the parts were.
/// Preserves all distinct materials and textures as separate slots.
pub fn group(sources: &[GroupSource]) -> Result<Grouped, String> {
    group_with_mode(sources, GroupMode::MultiMaterial)
}

/// [`group`], choosing whether to keep every distinct material/texture
/// (`GroupMode::MultiMaterial`) or overwrite all parts with one shared
/// material (`GroupMode::SingleMaterial`).
pub fn group_with_mode(sources: &[GroupSource], mode: GroupMode) -> Result<Grouped, String> {
    let first = sources
        .first()
        .ok_or_else(|| "Pick at least two parts to group".to_owned())?;
    let to_frame = first.rotation.inverse();
    let mut looks: Vec<GroupLook> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    let single_look = match mode {
        GroupMode::MultiMaterial => None,
        GroupMode::SingleMaterial(look) => {
            keys.push(look.key());
            looks.push(*look);
            Some(0u32)
        }
    };
    let mut mesh = Mesh::default();
    let mut part_of = Vec::new();
    let mut parts = Vec::new();
    let mut stats = GroupStats {
        parts: sources.len(),
        ..GroupStats::default()
    };
    let mut collision = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        // Parts turned as the first is aren't turned at all, so their
        // corners land exactly.
        let turn = if source.rotation == first.rotation {
            Quat::IDENTITY
        } else {
            to_frame * source.rotation
        };
        let shift = to_frame * (source.position - first.position);
        for piece in &source.collision {
            collision.push(piece.iter().map(|&p| turn * p + shift).collect::<Vec<_>>());
        }
        if !source.drawn || source.mesh.is_empty() {
            parts.push(PartInfo::default());
            continue;
        }
        // Parts that look the same share one material (or all share the
        // chosen single look in SingleMaterial mode).
        let look_of: Vec<u32> = match single_look {
            Some(only) => vec![only; source.looks.len().max(1)],
            None => source
                .looks
                .iter()
                .map(|look| {
                    let key = look.key();
                    let found = keys.iter().position(|seen| *seen == key);
                    found.unwrap_or_else(|| {
                        keys.push(key);
                        looks.push(look.clone());
                        looks.len() - 1
                    }) as u32
                })
                .collect(),
        };
        let solid = match single_look {
            Some(_) => {
                opaque(&looks[0].material)
                    && source.mesh.open_edges() == 0
                    && source.mesh.volume().abs() > 0.0
            }
            None => {
                source.looks.iter().all(|look| opaque(&look.material))
                    && source.mesh.open_edges() == 0
                    && source.mesh.volume().abs() > 0.0
            }
        };
        parts.push(PartInfo { solid });
        stats.triangles_before += source.mesh.triangles.len();
        stats.vertices_before += used_vertices(&source.mesh);
        let first_vertex = mesh.positions.len() as u32;
        for k in 0..source.mesh.positions.len() {
            mesh.push_vertex(
                turn * source.mesh.positions[k],
                (turn * source.mesh.normals[k]).normalize_or_zero(),
                source.mesh.uvs.get(k).copied().unwrap_or(Vec2::ZERO),
            );
        }
        for triangle in &source.mesh.triangles {
            let look = match triangle.surface {
                Surface::Outer(i) => look_of.get(i as usize).copied(),
                Surface::Interior => None,
            };
            let Some(look) = look.or_else(|| look_of.first().copied()) else {
                continue;
            };
            mesh.triangles.push(crate::Triangle {
                corners: triangle.corners.map(|c| c + first_vertex),
                surface: Surface::Outer(look),
            });
            part_of.push(index as u32);
        }
        // Positions after the shift, kept apart from the rotation so a
        // part at the origin is exactly where it was.
        for position in &mut mesh.positions[first_vertex as usize..] {
            *position += shift;
        }
    }
    if mesh.is_empty() {
        return Err("Nothing of the parts can be seen, so there's nothing to draw".to_owned());
    }
    let infos: Vec<LookInfo> = looks
        .iter()
        .map(|look| LookInfo {
            plain: !look.has_textures(),
            double_sided: look.material.double_sided,
        })
        .collect();
    let (mut mesh, report) = optimize(&mesh, &part_of, &parts, &infos);
    stats.report = report;
    let mut looks = only_used(&mut mesh, looks);
    make_material_names_unique(&mut looks);
    stats.triangles_after = mesh.triangles.len();
    stats.vertices_after = written_vertices(&mesh);

    let (min, max) = mesh.bounds();
    let middle = (min + max) / 2.0;
    for position in &mut mesh.positions {
        *position -= middle;
    }
    for piece in &mut collision {
        for point in piece.iter_mut() {
            *point -= middle;
        }
    }
    collision.retain(|piece| piece.len() >= 3);
    Ok(Grouped {
        mesh,
        looks,
        collision,
        position: first.position + first.rotation * middle,
        rotation: first.rotation,
        size: (max - min).max(Vec3::splat(1e-3)),
        stats,
    })
}

/// Gives every look in `looks` a unique, non-empty `material.name` so each
/// material slot in the grouped `.glb` can be targeted individually.
fn make_material_names_unique(looks: &mut [GroupLook]) {
    let mut used = std::collections::HashSet::new();
    for look in looks {
        let base = look.display_name();
        let mut candidate = base.clone();
        let mut n = 2usize;
        while !used.insert(candidate.clone()) {
            candidate = format!("{base} {n}");
            n += 1;
        }
        look.material.name = candidate;
    }
}

/// The looks `mesh`'s triangles use, in order, with its surfaces
/// renumbered to match.
fn only_used(mesh: &mut Mesh, looks: Vec<GroupLook>) -> Vec<GroupLook> {
    let mut new_index = vec![None; looks.len()];
    let mut kept = Vec::new();
    for triangle in &mut mesh.triangles {
        if let Surface::Outer(index) = triangle.surface {
            let index = index as usize;
            if index >= looks.len() {
                continue;
            }
            let new = *new_index[index].get_or_insert_with(|| {
                kept.push(looks[index].clone());
                kept.len() - 1
            });
            triangle.surface = Surface::Outer(new as u32);
        }
    }
    kept
}

/// How many vertices the `.glb` holds: each look's primitive has its own.
fn written_vertices(mesh: &Mesh) -> usize {
    let mut seen = std::collections::HashSet::new();
    for triangle in &mesh.triangles {
        for corner in triangle.corners {
            seen.insert((look_index(triangle), corner));
        }
    }
    seen.len()
}

/// `triangle`'s look's index (`u32::MAX` for an inside face).
fn look_index(triangle: &crate::Triangle) -> u32 {
    match triangle.surface {
        Surface::Outer(index) => index,
        Surface::Interior => u32::MAX,
    }
}

impl Grouped {
    /// `rotation` as an `Orientation` in degrees (Z, then X, then Y), each
    /// rounded to 3 decimals as the editor shows them.
    pub fn orientation(&self) -> Vector3 {
        let (y, x, z) = self.rotation.to_euler(EulerRot::YXZ);
        let tidy = |radians: f32| (radians.to_degrees() * 1000.0).round() / 1000.0 + 0.0;
        Vector3::new(tidy(x), tidy(y), tidy(z))
    }

    /// The project folder the `.glb` goes in: `assets/models`, or the
    /// folder holding both it and every texture a look links, since a
    /// model's textures must be inside its own folder.
    pub fn folder(&self) -> String {
        let mut folder = MODELS_FOLDER.to_owned();
        for file in self.looks.iter().flat_map(|look| look.maps.files()) {
            let parent = file.rsplit_once('/').map_or("", |(parent, _)| parent);
            folder = common_folder(&folder, parent);
        }
        folder
    }

    /// The grouped model as a `.glb` written into the project folder
    /// `folder`: one object, `Group`, with a primitive per look (each
    /// material linking its textures relative to `folder`), and
    /// each collision piece as an object `Group_COL1`, `Group_COL2`, …
    /// that isn't drawn.
    pub fn glb(&self, folder: &str) -> Vec<u8> {
        let mut file = File::default();
        let materials: Vec<usize> = self
            .looks
            .iter()
            .map(|look| {
                let maps = look.maps.map(|path| relative_path(folder, path));
                file.add_material_with_embedded(&look.material, &maps, &look.embedded)
            })
            .collect();
        let mut surfaces: Vec<u32> = self.mesh.triangles.iter().map(look_index).collect();
        surfaces.sort_unstable();
        surfaces.dedup();
        let primitives: Vec<serde_json::Value> = surfaces
            .into_iter()
            .filter_map(|index| {
                let material = *materials.get(index as usize)?;
                let vertices = surface_triangles(&self.mesh, Surface::Outer(index));
                Some(file.add_primitive(&vertices, material))
            })
            .collect();
        let mut nodes = vec![json!({ "name": OBJECT, "mesh": 0 })];
        let mut meshes = vec![json!({ "name": OBJECT, "primitives": primitives })];
        for (k, piece) in self.collision.iter().enumerate() {
            let name = format!("{OBJECT}_COL{}", k + 1);
            // Only the corners count (the hull of them), so any triangles
            // through them do: a fan.
            let vertices = Vertices {
                positions: piece.clone(),
                normals: vec![Vec3::Y; piece.len()],
                indices: (1..piece.len() as u32 - 1)
                    .flat_map(|i| [0, i, i + 1])
                    .collect(),
                ..Vertices::default()
            };
            let primitive = file.add_primitive(&vertices, materials.first().copied().unwrap_or(0));
            nodes.push(json!({ "name": name, "mesh": meshes.len() }));
            meshes.push(json!({ "name": name, "primitives": [primitive] }));
        }
        let roots = (0..nodes.len()).collect();
        file.finish(nodes, meshes, roots)
    }
}

/// The deepest folder holding both `a` and `b` (project folders, `/`
/// between them; "" is the project itself).
fn common_folder(a: &str, b: &str) -> String {
    let parts = |path: &str| -> Vec<String> {
        path.split('/')
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect()
    };
    let (a, b) = (parts(a), parts(b));
    a.iter()
        .zip(&b)
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

/// `path` (a project path inside the folder `folder`) relative to it.
fn relative_path(folder: &str, path: &str) -> String {
    let folder = folder.trim_matches('/');
    let path = path.trim_matches('/');
    if folder.is_empty() {
        return path.to_owned();
    }
    path.strip_prefix(folder)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(path)
        .to_owned()
}

/// Writes a grouped model's `.glb` as `<folder>/<name>.group.glb` (see
/// [`Grouped::folder`]; numbered if that's taken) and returns its
/// project-relative path.
pub fn save_group(
    project: &Path,
    assets: &Assets,
    name: &str,
    grouped: &Grouped,
) -> Result<String, String> {
    let folder = grouped.folder();
    let full = if folder.is_empty() {
        project.to_path_buf()
    } else {
        project.join(&folder)
    };
    std::fs::create_dir_all(&full)
        .map_err(|err| format!("Could not make the folder `{folder}`: {err}"))?;
    let stem = safe_file_name(name).unwrap_or_else(|| "Group".to_owned());
    let file = unique_file_name(&format!("{stem}.group.glb"), |file| {
        full.join(file).exists()
    });
    let path = if folder.is_empty() {
        file
    } else {
        format!("{folder}/{file}")
    };
    std::fs::write(project.join(&path), grouped.glb(&folder))
        .map_err(|err| format!("Could not write `{path}`: {err}"))?;
    assets.forget(&path);
    Ok(path)
}
