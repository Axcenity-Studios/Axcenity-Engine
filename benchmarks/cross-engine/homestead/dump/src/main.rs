//! Writes what the engine draws of a project, for another engine to
//! rebuild: the engine's own loader and draw list, so the meshes, their
//! levels of detail, each item's place, its cull distance and whether it
//! casts a shadow are exactly what the renderer is given.
//!
//!     homestead-dump <project folder> <output folder> [sequence]
//!
//! It writes, in the output folder:
//!
//! - `meshes.bin`: every mesh drawn and its simpler copies.
//! - `items.bin`: every item (terrain chunks left out: the other engine
//!   draws the terrain with its own, from the project's height map).
//! - `textures/<name>.png`: base color, normal and emission maps as they
//!   are, and one "mask" map per material that has metalness, roughness or
//!   occlusion maps (red metalness, green occlusion, alpha smoothness).
//! - `scene.json`: the materials, lights, sun, and the look's numbers.
//! - `look.txt`: the sky, fog, clouds and post-processing as the renderer
//!   is given them, to read.
//! - `tour.csv`: the sequence's camera at each sixtieth of a second.
//!
//! Everything is in the engine's own space (right-handed, Y up, studs).

use std::collections::{BTreeMap, HashMap};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, anyhow};
use axcenity_anim::Animators;
use axcenity_anim::sequence::{SequenceRun, load_sequence};
use axcenity_assets::Assets;
use axcenity_render::glam::Vec3;
use axcenity_render::{
    Camera, Channel, ChannelMap, DrawItem, DrawList, Material, MeshSource, Shape, TextureSource,
};
use axcenity_scene::TerrainCache;
use serde_json::{Value, json};

/// What a texture is used as, which decides how the other engine reads it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Usage {
    Color,
    Normal,
}

struct Textures {
    /// By key: the image, its use, and whether a material needs its alpha.
    seen: BTreeMap<u64, (Arc<dyn TextureSource>, Usage, bool)>,
    /// Mask maps by what they were packed from.
    masks: BTreeMap<String, (String, u32, u32, Vec<u8>)>,
}

impl Textures {
    fn add(&mut self, texture: &Arc<dyn TextureSource>, usage: Usage, alpha: bool) -> String {
        let entry = self
            .seen
            .entry(texture.key())
            .or_insert_with(|| (texture.clone(), usage, false));
        entry.2 |= alpha;
        format!("{:016x}", texture.key())
    }
}

fn channel_index(channel: Channel) -> usize {
    match channel {
        Channel::Red => 0,
        Channel::Green => 1,
        Channel::Blue => 2,
        Channel::Alpha => 3,
    }
}

fn map_key(map: &Option<ChannelMap>) -> String {
    match map {
        Some(map) => format!(
            "{:016x}.{}.{}",
            map.texture.key(),
            channel_index(map.channel),
            map.invert
        ),
        None => "-".into(),
    }
}

/// One value of a channel map at (`u`, `v`) of a `width` × `height` mask,
/// or `fallback` without a map.
fn sample(map: &Option<ChannelMap>, x: u32, y: u32, width: u32, height: u32, fallback: f32) -> f32 {
    let Some(map) = map else {
        return fallback;
    };
    let (w, h) = map.texture.size();
    let sx = (x as u64 * w as u64 / width as u64).min(w as u64 - 1) as usize;
    let sy = (y as u64 * h as u64 / height as u64).min(h as u64 - 1) as usize;
    let value =
        map.texture.rgba()[(sy * w as usize + sx) * 4 + channel_index(map.channel)] as f32 / 255.0;
    if map.invert { 1.0 - value } else { value }
}

/// The mask map of a material with metalness, roughness or occlusion
/// maps: red metalness, green occlusion, blue nothing, alpha smoothness,
/// with the material's own factors already in. `None` for a material with
/// none of the three.
fn mask_of(material: &Material, textures: &mut Textures) -> Option<String> {
    let maps = &material.maps;
    if maps.metalness.is_none() && maps.roughness.is_none() && maps.occlusion.is_none() {
        return None;
    }
    let key = format!(
        "{}|{}|{}|{}|{}|{}",
        map_key(&maps.metalness),
        map_key(&maps.roughness),
        map_key(&maps.occlusion),
        material.metalness,
        material.roughness,
        maps.occlusion_strength
    );
    if let Some((name, ..)) = textures.masks.get(&key) {
        return Some(name.clone());
    }
    let (mut width, mut height) = (1, 1);
    for map in [&maps.metalness, &maps.roughness, &maps.occlusion]
        .into_iter()
        .flatten()
    {
        let (w, h) = map.texture.size();
        width = width.max(w);
        height = height.max(h);
    }
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    for y in 0..height {
        for x in 0..width {
            // A map multiplies the material's own value, as glTF has it.
            let metal = if maps.metalness.is_some() {
                sample(&maps.metalness, x, y, width, height, 1.0) * material.metalness
            } else {
                material.metalness
            };
            let rough = if maps.roughness.is_some() {
                sample(&maps.roughness, x, y, width, height, 1.0) * material.roughness
            } else {
                material.roughness
            };
            let occlusion = sample(&maps.occlusion, x, y, width, height, 1.0);
            let occlusion = 1.0 + (occlusion - 1.0) * maps.occlusion_strength;
            let at = ((y * width + x) * 4) as usize;
            pixels[at] = byte(metal);
            pixels[at + 1] = byte(occlusion);
            pixels[at + 2] = 0;
            pixels[at + 3] = byte(1.0 - rough);
        }
    }
    let name = format!("mask_{:04}", textures.masks.len());
    textures
        .masks
        .insert(key, (name.clone(), width, height, pixels));
    Some(name)
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8], alpha: bool) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    if alpha {
        encoder.set_color(png::ColorType::Rgba);
        encoder.write_header()?.write_image_data(rgba)?;
    } else {
        encoder.set_color(png::ColorType::Rgb);
        let rgb: Vec<u8> = rgba
            .chunks_exact(4)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
            .collect();
        encoder.write_header()?.write_image_data(&rgb)?;
    }
    Ok(())
}

fn u32le(out: &mut impl Write, value: u32) -> Result<()> {
    Ok(out.write_all(&value.to_le_bytes())?)
}

fn u64le(out: &mut impl Write, value: u64) -> Result<()> {
    Ok(out.write_all(&value.to_le_bytes())?)
}

fn floats(out: &mut impl Write, values: &[f32]) -> Result<()> {
    for value in values {
        out.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

/// One mesh: its key, its simpler copies' keys, then its corners.
fn write_mesh(out: &mut impl Write, mesh: &dyn MeshSource, lods: &[u64]) -> Result<()> {
    let positions = mesh.positions();
    u64le(out, mesh.key())?;
    u32le(out, lods.len() as u32)?;
    for lod in lods {
        u64le(out, *lod)?;
    }
    u32le(out, positions.len() as u32)?;
    u32le(out, mesh.indices().len() as u32)?;
    let has_uvs = mesh.uvs().len() == positions.len();
    let has_tangents = mesh.tangents().len() == positions.len();
    u32le(out, has_uvs as u32 | (has_tangents as u32) << 1)?;
    for position in positions {
        floats(out, position)?;
    }
    for normal in mesh.normals() {
        floats(out, normal)?;
    }
    if has_uvs {
        for uv in mesh.uvs() {
            floats(out, uv)?;
        }
    }
    if has_tangents {
        for tangent in mesh.tangents() {
            floats(out, tangent)?;
        }
    }
    for index in mesh.indices() {
        u32le(out, *index)?;
    }
    Ok(())
}

/// Every item the list draws but the terrain's own chunks, each with the
/// group it is in (0 for the list's own, then the chunks from 1).
fn drawn(list: &DrawList) -> Vec<(&DrawItem, u32)> {
    let mut items: Vec<(&DrawItem, u32)> = list.items.iter().map(|item| (item, 0)).collect();
    for (index, chunk) in list.chunks.iter().enumerate() {
        items.extend(chunk.items.iter().map(|item| (item, index as u32 + 1)));
    }
    items.retain(|(item, _)| item.material.terrain.is_none() && item.material.water.is_none());
    items
}

/// What changes while models and textures are still loading.
fn signature(list: &DrawList) -> (usize, usize, u64) {
    let items = drawn(list);
    let mut textures = 0usize;
    let mut keys = 0u64;
    for (item, _) in &items {
        if let Shape::Mesh(mesh) = &item.shape {
            keys = keys.wrapping_add(mesh.key());
        }
        let material = &item.material;
        textures += material.base_color_texture.is_some() as usize
            + material.maps.normal.is_some() as usize
            + material.maps.roughness.is_some() as usize;
    }
    (items.len(), textures, keys)
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let folder = PathBuf::from(args.next().context("the project folder")?);
    let out = PathBuf::from(args.next().context("the output folder")?);
    let sequence = args.next();
    std::fs::create_dir_all(out.join("textures"))?;

    let project = axcenity_project::load(&folder).map_err(|e| anyhow!("{e}"))?;
    for problem in &project.problems {
        eprintln!("load problem: {problem:?}");
    }
    let assets = Assets::new(&folder);
    let world = project.world.clone();
    let animators = Animators::new();
    let mut terrain = TerrainCache::new();
    // The view the project's own script starts from.
    let camera = Camera {
        position: Vec3::new(-5.0, 5.26, 72.0),
        look_at: Vec3::new(2.0, 5.56, 0.0),
        fov_y_degrees: 36.0,
    };

    // Models load on other threads and are left out until they are in:
    // build the list until it has stopped changing for three seconds.
    let mut list;
    let mut last = (0, 0, 0);
    let mut steady = 0;
    let started = std::time::Instant::now();
    loop {
        list = axcenity_scene::build_draw_list_posed(
            &world,
            &assets,
            camera,
            animators.poses(),
            &mut terrain,
        );
        let now = signature(&list);
        steady = if now == last && now.0 > 0 {
            steady + 1
        } else {
            0
        };
        last = now;
        if steady >= 15 {
            break;
        }
        if started.elapsed().as_secs() > 600 {
            return Err(anyhow!("the world was still loading after ten minutes"));
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    for error in assets.take_errors() {
        eprintln!("asset error: {error:?}");
    }
    let items = drawn(&list);
    eprintln!(
        "{} items ({} of the list's own, {} groups), loaded in {:.1} s",
        items.len(),
        list.items.len(),
        list.chunks.len(),
        started.elapsed().as_secs_f64()
    );

    // Meshes, with their simpler copies.
    let mut meshes: BTreeMap<u64, (Arc<dyn MeshSource>, Vec<u64>)> = BTreeMap::new();
    let mut built_in = 0;
    for (item, _) in &items {
        let Shape::Mesh(mesh) = &item.shape else {
            built_in += 1;
            continue;
        };
        if meshes.contains_key(&mesh.key()) {
            continue;
        }
        let mut lods = Vec::new();
        for level in 1..=mesh.lod_count() {
            if let Some(lod) = mesh.lod(level) {
                lods.push(lod.key());
                meshes.entry(lod.key()).or_insert((lod.clone(), Vec::new()));
            }
        }
        meshes.insert(mesh.key(), (mesh.clone(), lods));
    }
    if built_in > 0 {
        eprintln!("left out: {built_in} items drawn with built-in shapes");
    }
    let mut file = BufWriter::new(std::fs::File::create(out.join("meshes.bin"))?);
    u32le(&mut file, meshes.len() as u32)?;
    let mut triangles = 0;
    for (mesh, lods) in meshes.values() {
        write_mesh(&mut file, mesh.as_ref(), lods)?;
        triangles += mesh.indices().len() / 3;
    }
    file.flush()?;
    eprintln!("{} meshes, {} triangles in all", meshes.len(), triangles);

    // Materials, each once.
    let mut textures = Textures {
        seen: BTreeMap::new(),
        masks: BTreeMap::new(),
    };
    let mut materials: Vec<Value> = Vec::new();
    let mut material_ids: HashMap<String, u32> = HashMap::new();
    let mut item_materials = Vec::with_capacity(items.len());
    let mut notes: BTreeMap<&str, usize> = BTreeMap::new();
    for (item, _) in &items {
        let material = &item.material;
        let cut_out = material.alpha_cutoff > 0.0;
        let blended = material.opacity < 1.0;
        let key = format!(
            "{:?}|{:?}|{}|{}|{:?}|{}|{}|{:?}|{}|{}|{}|{}|{:?}|{:?}|{}|{}|{:?}|{:?}|{:?}",
            material.base_color,
            material.base_color_texture.as_ref().map(|t| t.key()),
            material.metalness,
            material.roughness,
            material.emissive,
            material.opacity,
            material.alpha_cutoff,
            material
                .maps
                .normal
                .as_ref()
                .map(|n| (n.texture.key(), n.scale, n.flip_green)),
            map_key(&material.maps.metalness),
            map_key(&material.maps.roughness),
            map_key(&material.maps.occlusion),
            material.maps.occlusion_strength,
            material.maps.emissive.as_ref().map(|t| t.key()),
            map_key(&material.maps.opacity),
            material.double_sided,
            material.translucency,
            material.uv_scale,
            material.uv_offset,
            std::mem::discriminant(&material.shading),
        );
        let id = match material_ids.get(&key) {
            Some(id) => *id,
            None => {
                if material.maps.opacity.is_some() {
                    *notes
                        .entry("materials with an opacity map (left out)")
                        .or_default() += 1;
                }
                if material.maps.specular.is_some() {
                    *notes
                        .entry("materials with a specular map (left out)")
                        .or_default() += 1;
                }
                if material.surface_shader.is_some() {
                    *notes
                        .entry("materials with a surface shader (left out)")
                        .or_default() += 1;
                }
                if material.uv_scale != [1.0, 1.0] || material.uv_offset != [0.0, 0.0] {
                    *notes.entry("materials with placed textures").or_default() += 1;
                }
                let base = material
                    .base_color_texture
                    .as_ref()
                    .map(|texture| textures.add(texture, Usage::Color, cut_out || blended));
                let normal = material
                    .maps
                    .normal
                    .as_ref()
                    .map(|normal| textures.add(&normal.texture, Usage::Normal, false));
                let emissive = material
                    .maps
                    .emissive
                    .as_ref()
                    .map(|texture| textures.add(texture, Usage::Color, false));
                let mask = mask_of(material, &mut textures);
                let id = materials.len() as u32;
                materials.push(json!({
                    "base_color": material.base_color,
                    "base_color_texture": base,
                    "metalness": material.metalness,
                    "roughness": material.roughness,
                    "emissive": material.emissive,
                    "emissive_texture": emissive,
                    "opacity": material.opacity,
                    "alpha_cutoff": material.alpha_cutoff,
                    "normal_texture": normal,
                    "normal_scale": material.maps.normal.as_ref().map(|n| n.scale),
                    "normal_flip_green": material.maps.normal.as_ref().map(|n| n.flip_green),
                    "mask_texture": mask,
                    "double_sided": material.double_sided,
                    "translucency": material.translucency,
                    "uv_scale": material.uv_scale,
                    "uv_offset": material.uv_offset,
                    "shading": format!("{:?}", std::mem::discriminant(&material.shading)),
                }));
                material_ids.insert(key, id);
                id
            }
        };
        item_materials.push(id);
    }
    for (note, count) in &notes {
        eprintln!("note: {count} {note}");
    }

    // Items.
    let mut file = BufWriter::new(std::fs::File::create(out.join("items.bin"))?);
    let with_mesh = items
        .iter()
        .filter(|(item, _)| matches!(item.shape, Shape::Mesh(_)))
        .count();
    u32le(&mut file, with_mesh as u32)?;
    let mut casting = 0;
    let mut swaying = 0;
    for ((item, group), material) in items.iter().zip(&item_materials) {
        let Shape::Mesh(mesh) = &item.shape else {
            continue;
        };
        u64le(&mut file, item.id)?;
        u64le(&mut file, mesh.key())?;
        u32le(&mut file, *material)?;
        u32le(&mut file, *group)?;
        floats(&mut file, &item.transform.to_cols_array())?;
        floats(&mut file, &[item.cull_distance])?;
        u32le(&mut file, item.casts_shadow as u32)?;
        let sway = item.sway.filter(|sway| !sway.is_still());
        match sway {
            Some(sway) => floats(
                &mut file,
                &[sway.base, sway.height, sway.strength, sway.flutter],
            )?,
            None => floats(&mut file, &[0.0; 4])?,
        }
        casting += item.casts_shadow as usize;
        swaying += sway.is_some() as usize;
    }
    file.flush()?;
    eprintln!("{with_mesh} items written: {casting} cast shadows, {swaying} sway");

    // Textures, on every core: encoding is most of the time.
    let mut jobs: Vec<(PathBuf, u32, u32, &[u8], bool)> = Vec::new();
    let mut texture_list = Vec::new();
    for (key, (texture, usage, alpha)) in &textures.seen {
        let (width, height) = texture.size();
        let name = format!("{key:016x}");
        jobs.push((
            out.join("textures").join(format!("{name}.png")),
            width,
            height,
            texture.rgba(),
            *alpha,
        ));
        texture_list.push(json!({
            "name": name, "width": width, "height": height, "alpha": alpha,
            "usage": match usage { Usage::Color => "color", Usage::Normal => "normal" },
        }));
    }
    for (name, width, height, pixels) in textures.masks.values() {
        jobs.push((
            out.join("textures").join(format!("{name}.png")),
            *width,
            *height,
            pixels,
            true,
        ));
        texture_list.push(json!({
            "name": name, "width": width, "height": height, "alpha": true, "usage": "mask",
        }));
    }
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some((path, width, height, rgba, alpha)) = jobs.get(index) else {
                        break;
                    };
                    if let Err(error) = write_png(path, *width, *height, rgba, *alpha) {
                        eprintln!("{}: {error}", path.display());
                    }
                }
            });
        }
    });
    eprintln!("{} textures written", jobs.len());

    // The lights and the look.
    let lights: Vec<Value> = list
        .lights
        .iter()
        .map(|light| {
            json!({
                "position": light.position.to_array(),
                "color": light.color,
                "brightness": light.brightness,
                "range": light.range,
                "cone": light.cone.is_some(),
                "shadows": light.point_shadows.is_some(),
            })
        })
        .collect();
    let scene = json!({
        "materials": materials,
        "textures": texture_list,
        "lights": lights,
        "sun": {
            "direction": list.sun.direction.to_array(),
            "color": list.sun.color,
            "intensity": list.sun.intensity,
        },
        "ambient": {
            "color": list.ambient.color,
            "intensity": list.ambient.intensity,
            "occlusion": list.ambient.occlusion,
        },
        "shadows": {
            "enabled": list.shadows.enabled,
            "distance": list.shadows.distance,
            "softness": list.shadows.softness,
        },
        "wind": list.wind.to_array(),
        "camera": {
            "position": camera.position.to_array(),
            "look_at": camera.look_at.to_array(),
            "fov": camera.fov_y_degrees,
        },
    });
    std::fs::write(
        out.join("scene.json"),
        serde_json::to_string_pretty(&scene)?,
    )?;
    let mut look = String::new();
    use std::fmt::Write as _;
    writeln!(look, "sun: {:#?}", list.sun)?;
    writeln!(look, "ambient: {:#?}", list.ambient)?;
    writeln!(look, "shadows: {:#?}", list.shadows)?;
    writeln!(look, "sky: {:#?}", list.sky)?;
    writeln!(look, "fog: {:#?}", list.fog)?;
    writeln!(look, "clouds: {:#?}", list.clouds)?;
    writeln!(look, "post: {:#?}", list.post)?;
    writeln!(look, "lens: {:#?}", list.lens)?;
    writeln!(look, "grading: {:#?}", list.grading)?;
    writeln!(look, "wind: {:#?}", list.wind)?;
    writeln!(look, "lights: {:#?}", list.lights)?;
    writeln!(look, "particle emitters: {}", list.particle_emitters.len())?;
    writeln!(look, "reflection probes: {}", list.reflection_probes.len())?;
    writeln!(
        look,
        "light probe volumes: {}",
        list.light_probe_volumes.len()
    )?;
    std::fs::write(out.join("look.txt"), look)?;

    // The sequence's camera, frame by frame.
    if let Some(sequence) = sequence {
        let file = load_sequence(&assets, &sequence).map_err(|e| anyhow!("{e}"))?;
        let duration = file.duration;
        let mut run = SequenceRun::new(file).map_err(|e| anyhow!("{e:?}"))?;
        let mut world = project.world.clone();
        let mut csv = String::from("frame,seconds,x,y,z,pitch,yaw,roll,fov\n");
        let frames = (duration as f64 * 60.0).round() as u32;
        for frame in 0..frames {
            let t = frame as f64 / 60.0;
            let applied = run.apply(&mut world, t);
            let Some(view) = applied.camera else {
                continue;
            };
            writeln!(
                csv,
                "{frame},{t:.5},{},{},{},{},{},{},{}",
                view.position.x,
                view.position.y,
                view.position.z,
                view.orientation.x,
                view.orientation.y,
                view.orientation.z,
                view.field_of_view
            )?;
        }
        std::fs::write(out.join("tour.csv"), csv)?;
        eprintln!("{frames} frames of {sequence} written");
    }
    Ok(())
}
