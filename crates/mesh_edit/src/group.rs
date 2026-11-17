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

use axcenity_assets::{AlphaMode, Assets, Channel, MODELS_FOLDER, ModelMaterial};
use axcenity_assets::{safe_file_name, unique_file_name};
use axcenity_core::{ClassName, Color3, Instance, InstanceId, Material, Value, Vector3, World};
use glam::{EulerRot, Quat, Vec2, Vec3};
use serde_json::json;

use crate::generate::{part_mesh, srgb_to_linear};
use crate::glb::{File, MaterialMaps, Vertices, surface_triangles};
use crate::optimize::{LookInfo, OptimizeReport, PartInfo, optimize, used_vertices};
use crate::{Mesh, Surface};

/// The name of the grouped model's one object; its collision pieces are
/// `Group_COL1`, `Group_COL2`, …
const OBJECT: &str = "Group";

/// One look of a grouped part: a model material's values (its own
/// textures left out) and the texture files it links, by project path.
#[derive(Clone, Debug)]
pub struct GroupLook {
    pub material: ModelMaterial,
    pub maps: MaterialMaps,
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
        }
    }

    /// Looks with the same key are the same material.
    fn key(&self) -> String {
        format!("{:?}\u{0}{:?}", self.maps, self.material)
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
        let looks: Vec<GroupLook> = model
            .primitives
            .iter()
            .map(|primitive| {
                let appearance = appearances.iter().find(|appearance| {
                    appearance.property("MaterialName").and_then(Value::as_str)
                        == Some(primitive.material.name.as_str())
                });
                let mut look = match appearance {
                    Some(appearance) => {
                        let (look, lost) = surface_look(&primitive.material, appearance);
                        if let Some(lost) = lost {
                            notes.push(format!(
                                "`{name}`'s SurfaceAppearance `{}` {lost}",
                                appearance.name()
                            ));
                        }
                        look
                    }
                    None => {
                        if let Some(problem) = not_carried(&primitive.material) {
                            notes.push(format!(
                                "`{name}`'s material `{}` {problem}",
                                primitive.material.name
                            ));
                        }
                        let within = |file: &str| {
                            if folder.is_empty() {
                                file.to_owned()
                            } else {
                                format!("{folder}/{file}")
                            }
                        };
                        GroupLook::new(
                            &primitive.material,
                            MaterialMaps::of(&primitive.material).map(within),
                        )
                    }
                };
                if let Some(color) = one_color {
                    let [r, g, b] = linear(color);
                    look.material.base_color = [r, g, b, look.material.base_color[3]];
                    look.maps.base_color = None;
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
        let variant = instance
            .property("MaterialVariant")
            .and_then(Value::as_str)
            .is_some_and(|variant| !variant.is_empty());
        if variant {
            notes.push(format!(
                "`{name}`'s MaterialVariant isn't carried: it's grouped in its model's own materials."
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
        let mesh = part_mesh(instance, "grouped")?;
        let kind = instance
            .property("Material")
            .and_then(Value::as_material)
            .unwrap_or(Material::Plastic);
        if has_built_in_texture(kind) {
            notes.push(format!(
                "`{name}`'s {} texture isn't carried: it's grouped in its color.",
                kind.as_str()
            ));
        }
        if has_appearance(world, instance)
            || instance
                .property("MaterialVariant")
                .and_then(Value::as_str)
                .is_some_and(|variant| !variant.is_empty())
        {
            notes.push(format!(
                "`{name}`'s MaterialVariant or SurfaceAppearance isn't carried: it's grouped in its color."
            ));
        }
        let mut material = part_material(instance);
        see_through(&mut material, visible);
        let collision = if collides {
            vec![distinct(&mesh.positions)]
        } else {
            Vec::new()
        };
        let look = GroupLook::new(&material, MaterialMaps::default());
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

/// True if `instance` has a SurfaceAppearance inside it.
fn has_appearance(world: &World, instance: &Instance) -> bool {
    instance
        .children()
        .iter()
        .filter_map(|&child| world.get(child))
        .any(|child| child.class() == ClassName::SurfaceAppearance)
}

/// The built-in materials drawn with maps of their own (`scene`'s
/// surfaces), which a model's material can't name.
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
    )
}

/// Why some of `material` can't be written into a glTF file, finishing a
/// sentence, or `None` when all of it can.
fn not_carried(material: &ModelMaterial) -> Option<&'static str> {
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
    let lost = [
        &material.base_color_texture,
        &material.normal_texture,
        &material.emissive_texture,
    ]
    .into_iter()
    .flatten()
    .chain(
        [
            &material.metalness_texture,
            &material.roughness_texture,
            &material.occlusion_texture,
        ]
        .into_iter()
        .flatten()
        .map(|texture| &texture.image),
    )
    .any(|image| image.file.is_none());
    lost.then_some("has a texture with no file of its own, which isn't carried.")
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
    let lost =
        (!lost.is_empty()).then(|| format!("has {}, which isn't carried.", lost.join(" and ")));
    (look, lost)
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
pub fn group(sources: &[GroupSource]) -> Result<Grouped, String> {
    let first = sources
        .first()
        .ok_or_else(|| "Pick at least two parts to group".to_owned())?;
    let to_frame = first.rotation.inverse();
    let mut looks: Vec<GroupLook> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
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
        // Parts that look the same share one material.
        let look_of: Vec<u32> = source
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
            .collect();
        let solid = source.looks.iter().all(|look| opaque(&look.material))
            && source.mesh.open_edges() == 0
            && source.mesh.volume().abs() > 0.0;
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
            plain: look.maps.is_empty(),
            double_sided: look.material.double_sided,
        })
        .collect();
    let (mut mesh, report) = optimize(&mesh, &part_of, &parts, &infos);
    stats.report = report;
    let looks = only_used(&mut mesh, looks);
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
                file.add_material_with(&look.material, &maps)
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
