//! The looks of the built-in materials: Wood's grain, Marble's veins, Brick's
//! weathered courses, Brushed Metal, Crystalline Ice, and so on. Each is four
//! small tiling PBR maps (color, normal, roughness, ambient occlusion) made
//! procedurally the first time a part of that material is drawn, so neither the
//! engine nor a project carries image files for them. The part's `Color` tints
//! the color map, and its `Roughness` scales the roughness map, as with any
//! other map.

use std::sync::{Arc, OnceLock};

use axcenity_core::Material as Kind;
use axcenity_render::{Channel, ChannelMap, Material, NormalMap, TextureSource};

/// How many pixels wide and tall each map is.
const SIZE: usize = 512;
/// Texture keys of the built-in surfaces. Project images count up from 1,
/// so they never reach these. Each material reserves 4 consecutive keys
/// (color, normal, roughness, occlusion).
const KEY_BASE: u64 = 0xB017_0000_0000_0000;

/// One built-in material's PBR maps.
pub(crate) struct Surface {
    color: Arc<dyn TextureSource>,
    normal: Arc<dyn TextureSource>,
    roughness: Arc<dyn TextureSource>,
    occlusion: Arc<dyn TextureSource>,
}

impl Surface {
    /// Puts these maps onto `material`.
    pub(crate) fn apply(&self, material: &mut Material) {
        material.base_color_texture = Some(self.color.clone());
        material.maps.normal = Some(NormalMap {
            texture: self.normal.clone(),
            scale: 1.0,
            flip_green: false,
        });
        material.maps.roughness = Some(ChannelMap {
            texture: self.roughness.clone(),
            channel: Channel::Red,
            invert: false,
        });
        material.maps.occlusion = Some(ChannelMap {
            texture: self.occlusion.clone(),
            channel: Channel::Red,
            invert: false,
        });
        material.maps.occlusion_strength = 0.75;
    }
}

/// The RGBA pixel buffers of a built-in procedural material (`Wood`, `Brick`,
/// `Concrete`, …), used when baking a Part's built-in material into a grouped
/// `.glb` mesh.
pub struct BuiltInMaps {
    pub width: u32,
    pub height: u32,
    pub color: Vec<u8>,
    pub normal: Vec<u8>,
    /// glTF metallic-roughness packing: blue = 255 (scaled by `metallicFactor`),
    /// green = roughness (scaled by `roughnessFactor`).
    pub metal_rough: Vec<u8>,
    pub occlusion: Vec<u8>,
}

/// Returns the procedural RGBA textures for a built-in material (`Wood`,
/// `Brick`, `Concrete`, `Metal`, …), or `None` for flat materials (`Plastic`,
/// `SmoothPlastic`, `Glass`, `Neon`).
pub fn built_in_maps(kind: Kind) -> Option<BuiltInMaps> {
    let surface = of(kind)?;
    let (width, height) = surface.color.size();
    let rough_src = surface.roughness.rgba();
    let mut metal_rough = Vec::with_capacity(rough_src.len());
    for px in rough_src.chunks_exact(4) {
        // Red unused (255), Green = roughness, Blue = 255 so metallicFactor applies cleanly.
        metal_rough.extend_from_slice(&[255, px[0], 255, 255]);
    }
    Some(BuiltInMaps {
        width,
        height,
        color: surface.color.rgba().to_vec(),
        normal: surface.normal.rgba().to_vec(),
        metal_rough,
        occlusion: surface.occlusion.rgba().to_vec(),
    })
}

/// `kind`'s maps, or `None` for a material drawn flat (`Plastic`,
/// `SmoothPlastic`, `Glass`, `Neon`). Made once, on first use.
pub(crate) fn of(kind: Kind) -> Option<&'static Surface> {
    static MADE: [OnceLock<Option<Surface>>; Kind::ALL.len()] =
        [const { OnceLock::new() }; Kind::ALL.len()];
    let index = Kind::ALL.iter().position(|&item| item == kind)?;
    MADE[index].get_or_init(|| make(kind, index)).as_ref()
}

/// A map held in memory.
struct Map {
    key: u64,
    rgba: Vec<u8>,
}

impl TextureSource for Map {
    fn key(&self) -> u64 {
        self.key
    }
    fn size(&self) -> (u32, u32) {
        (SIZE as u32, SIZE as u32)
    }
    fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// What a material is at one point of its tile.
#[derive(Clone, Copy)]
struct Texel {
    /// How far the surface stands out, in pixels of the map (drives normal map).
    height: f32,
    /// sRGB albedo, 0 to 1 (multiplied by the part's `Color`).
    color: [f32; 3],
    /// Multiplies the part's `Roughness`, 0 to 1.
    roughness: f32,
    /// Ambient occlusion, 0 (deep crevice) to 1 (fully open).
    occlusion: f32,
}

fn make(kind: Kind, index: usize) -> Option<Surface> {
    let shade = shader_of(kind)?;
    let texels: Vec<Texel> = (0..SIZE * SIZE)
        .map(|i| {
            let (x, y) = (i % SIZE, i / SIZE);
            shade(x as f32 / SIZE as f32, y as f32 / SIZE as f32)
        })
        .collect();
    let mut color = Vec::with_capacity(texels.len() * 4);
    let mut normal = Vec::with_capacity(texels.len() * 4);
    let mut roughness = Vec::with_capacity(texels.len() * 4);
    let mut occlusion = Vec::with_capacity(texels.len() * 4);
    let height = |x: usize, y: usize| texels[(y % SIZE) * SIZE + (x % SIZE)].height;
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    for (i, texel) in texels.iter().enumerate() {
        let (x, y) = (i % SIZE, i / SIZE);
        let xm = x + SIZE - 1;
        let xp = x + 1;
        let ym = y + SIZE - 1;
        let yp = y + 1;

        color.extend(texel.color.map(byte));
        color.push(255);

        // Tileable 3×3 Sobel gradient produces smoother, cleaner normals than a
        // 2-tap difference. Sloping up to the right tips left; V runs down the
        // image, so sloping up downward tips up (OpenGL green-up normal map).
        let dx = (height(xp, ym) + 2.0 * height(xp, y) + height(xp, yp))
            - (height(xm, ym) + 2.0 * height(xm, y) + height(xm, yp));
        let dy = (height(xm, yp) + 2.0 * height(x, yp) + height(xp, yp))
            - (height(xm, ym) + 2.0 * height(x, ym) + height(xp, ym));
        let (nx, ny, nz) = (-dx * 0.25, dy * 0.25, 1.0);
        let length = (nx * nx + ny * ny + nz * nz).sqrt();
        normal.extend([nx / length, ny / length, nz / length].map(|n: f32| byte(n * 0.5 + 0.5)));
        normal.push(255);

        let rough = byte(texel.roughness);
        roughness.extend([rough, rough, rough, 255]);

        let ao = byte(texel.occlusion);
        occlusion.extend([ao, ao, ao, 255]);
    }
    let map = |slot: u64, rgba: Vec<u8>| -> Arc<dyn TextureSource> {
        Arc::new(Map {
            key: KEY_BASE + index as u64 * 4 + slot,
            rgba,
        })
    };
    Some(Surface {
        color: map(0, color),
        normal: map(1, normal),
        roughness: map(2, roughness),
        occlusion: map(3, occlusion),
    })
}

/// How to work out each point of `kind`'s tile.
fn shader_of(kind: Kind) -> Option<Box<dyn Fn(f32, f32) -> Texel>> {
    Some(match kind {
        Kind::Wood => Box::new(wood()),
        Kind::WoodPlanks => Box::new(planks()),
        Kind::Marble => Box::new(marble()),
        Kind::Granite => Box::new(granite()),
        Kind::Concrete => Box::new(concrete()),
        Kind::Slate => Box::new(slate()),
        Kind::Brick => Box::new(brick()),
        Kind::Fabric => Box::new(fabric()),
        Kind::Sand => Box::new(sand()),
        Kind::Metal => Box::new(metal()),
        Kind::Ice => Box::new(ice()),
        Kind::Rubber => Box::new(rubber()),
        Kind::Plastic | Kind::SmoothPlastic | Kind::Glass | Kind::Neon => return None,
    })
}

const TAU: f32 = std::f32::consts::TAU;

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn smooth(from: f32, to: f32, value: f32) -> f32 {
    let t = ((value - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// C2-continuous quintic interpolation avoids grid-aligned creases in normals.
fn quintic(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn xorshift(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / 16_777_216.0
}

/// Value noise on a grid that wraps around, so what's made from it tiles.
struct Noise {
    columns: usize,
    rows: usize,
    values: Vec<f32>,
}

impl Noise {
    fn new(seed: u32, columns: usize, rows: usize) -> Noise {
        let mut state = seed.wrapping_mul(2_654_435_761) | 1;
        let values = (0..columns * rows).map(|_| xorshift(&mut state)).collect();
        Noise {
            columns,
            rows,
            values,
        }
    }

    /// From 0 to 1, at `u`, `v` (each wrapping at 1).
    fn at(&self, u: f32, v: f32) -> f32 {
        let (x, y) = (
            u.rem_euclid(1.0) * self.columns as f32,
            v.rem_euclid(1.0) * self.rows as f32,
        );
        let (cell_x, cell_y) = (x.floor(), y.floor());
        let (tx, ty) = (quintic(x - cell_x), quintic(y - cell_y));
        let value = |dx: usize, dy: usize| {
            let column = (cell_x as usize + dx) % self.columns;
            let row = (cell_y as usize + dy) % self.rows;
            self.values[row * self.columns + column]
        };
        let top = value(0, 0) + (value(1, 0) - value(0, 0)) * tx;
        let bottom = value(0, 1) + (value(1, 1) - value(0, 1)) * tx;
        top + (bottom - top) * ty
    }
}

/// Several [`Noise`] layers (FBM), each twice as fine and half as strong as
/// the one before. From 0 to 1.
struct Layers(Vec<Noise>);

impl Layers {
    fn new(seed: u32, columns: usize, rows: usize, count: u32) -> Layers {
        Layers(
            (0..count)
                .map(|i| Noise::new(seed + i * 29, columns << i, rows << i))
                .collect(),
        )
    }

    fn at(&self, u: f32, v: f32) -> f32 {
        let (mut sum, mut total, mut strength) = (0.0, 0.0, 1.0);
        for layer in &self.0 {
            sum += layer.at(u, v) * strength;
            total += strength;
            strength *= 0.5;
        }
        sum / total
    }

    /// Ridged multi-fractal noise (sharp creases / cracks / veins), 0 to 1.
    fn ridged(&self, u: f32, v: f32) -> f32 {
        let (mut sum, mut total, mut strength) = (0.0, 0.0, 1.0);
        for layer in &self.0 {
            let ridge = 1.0 - (layer.at(u, v) * 2.0 - 1.0).abs();
            sum += ridge * ridge * strength;
            total += strength;
            strength *= 0.5;
        }
        sum / total
    }
}

/// Tileable Voronoi / Worley cellular noise on a `columns × rows` grid.
struct Cellular {
    columns: usize,
    rows: usize,
    /// Per-cell `(jx, jy, id)` with jitter in `[0.1, 0.9]` and `id` in `[0, 1)`.
    cells: Vec<(f32, f32, f32)>,
}

#[derive(Clone, Copy)]
struct CellHit {
    /// Distance to the nearest feature point (in cell units).
    f1: f32,
    /// Distance to the nearest cell border (`f2 - f1`).
    edge: f32,
    /// Stable pseudo-random value in `[0, 1)` for the owning cell.
    id: f32,
}

impl Cellular {
    fn new(seed: u32, columns: usize, rows: usize) -> Cellular {
        let mut state = seed.wrapping_mul(1_597_334_677) | 1;
        let cells = (0..columns * rows)
            .map(|_| {
                let jx = 0.08 + xorshift(&mut state) * 0.84;
                let jy = 0.08 + xorshift(&mut state) * 0.84;
                let id = xorshift(&mut state);
                (jx, jy, id)
            })
            .collect();
        Cellular {
            columns,
            rows,
            cells,
        }
    }

    fn at(&self, u: f32, v: f32) -> CellHit {
        let x = u.rem_euclid(1.0) * self.columns as f32;
        let y = v.rem_euclid(1.0) * self.rows as f32;
        let cx = x.floor() as isize;
        let cy = y.floor() as isize;
        let cols = self.columns as isize;
        let rows = self.rows as isize;

        let mut d1 = f32::MAX;
        let mut d2 = f32::MAX;
        let mut best_id = 0.0;

        for oy in -1..=1 {
            let ny = (cy + oy).rem_euclid(rows) as usize;
            let base_y = (cy + oy) as f32;
            for ox in -1..=1 {
                let nx = (cx + ox).rem_euclid(cols) as usize;
                let (jx, jy, id) = self.cells[ny * self.columns + nx];
                let dx = (cx + ox) as f32 + jx - x;
                let dy = base_y + jy - y;
                let d = (dx * dx + dy * dy).sqrt();
                if d < d1 {
                    d2 = d1;
                    d1 = d;
                    best_id = id;
                } else if d < d2 {
                    d2 = d;
                }
            }
        }

        CellHit {
            f1: d1,
            edge: (d2 - d1).max(0.0),
            id: best_id,
        }
    }
}

/// Noise maps shared by Wood and WoodPlanks.
struct WoodMaps {
    warp_a: Layers,
    warp_b: Layers,
    fibers: Noise,
    pores: Noise,
}

impl WoodMaps {
    fn new(seed: u32) -> Self {
        Self {
            warp_a: Layers::new(seed, 3, 2, 4),
            warp_b: Layers::new(seed + 6, 6, 3, 3),
            fibers: Noise::new(seed + 12, 220, 10),
            pores: Noise::new(seed + 18, 256, 32),
        }
    }

    /// Organic wood growth rings + fine vessel pores along V.
    fn sample(&self, u: f32, v: f32, rings: f32, offset: f32) -> (f32, f32, f32) {
        let wu = self.warp_a.at(u + offset, v * 0.5 + offset) * 2.0 - 1.0;
        let wv = self.warp_b.at(u * 1.5 + offset, v + offset) * 2.0 - 1.0;
        // Slight cathedral arch curvature + natural growth irregularity.
        let phase = rings * u + 1.45 * wu + 0.35 * wv + offset * 3.7;
        let wave = (TAU * phase).sin() * 0.5 + 0.5;
        // Asymmetric earlywood / latewood profile: sharper dark rings, broader light bands.
        let ring = wave.powf(0.65);
        let fiber = self.fibers.at(u + offset + wu * 0.015, v);
        let pore = smooth(0.76, 0.9, self.pores.at(u + offset + wu * 0.01, v));
        (ring, fiber, pore)
    }
}

fn wood() -> impl Fn(f32, f32) -> Texel {
    let maps = WoodMaps::new(11);
    let rays = Layers::new(31, 4, 16, 2);
    move |u, v| {
        let (ring, fiber, pore) = maps.sample(u, v, 10.0, 0.0);
        let ray = rays.at(u, v) * 0.12;
        let shade = (ring * 0.52 + fiber * 0.36 + ray).clamp(0.0, 1.0);
        let mut color = mix([0.48, 0.31, 0.18], [0.82, 0.62, 0.41], shade);
        // Fine dark open wood vessels add crisp tactile realism up close.
        color = mix(color, [0.28, 0.16, 0.09], pore * 0.55);
        Texel {
            height: ring * 0.65 + fiber * 0.55 - pore * 1.4,
            color,
            roughness: (0.76 + (1.0 - ring) * 0.14 + pore * 0.16).clamp(0.0, 1.0),
            occlusion: (1.0 - pore * 0.42 - (1.0 - ring) * 0.08).clamp(0.45, 1.0),
        }
    }
}

fn planks() -> impl Fn(f32, f32) -> Texel {
    const BOARDS: f32 = 4.0;
    let maps = WoodMaps::new(37);
    let tone = Noise::new(53, 16, 4);
    let wear = Layers::new(59, 6, 6, 3);
    move |u, v| {
        let across = u * BOARDS;
        let board = across.floor();
        let inside = across - board;
        // Staggered butt joints along each plank (two plank lengths per tile).
        let shift = tone.at((board + 0.5) / 16.0 + 0.07, 0.25);
        let along_all = (v * 2.0 + shift).rem_euclid(2.0);
        let seg = along_all.floor();
        let along = along_all - seg;

        let plank_seed = tone.at((board + 0.5) / 16.0, (seg + 0.5) / 4.0);
        let offset = (board + 1.0) * 0.37 + seg * 0.53;
        let (ring, fiber, pore) = maps.sample(inside / BOARDS, v, 3.0, offset);

        // Crisp recessed seams with gentle bevels along both plank edges & end joints.
        let side_dist = inside.min(1.0 - inside);
        let end_dist = along.min(1.0 - along) * 2.6;
        let seam_dist = side_dist.min(end_dist);
        let groove = smooth(0.008, 0.022, seam_dist);
        let bevel = smooth(0.015, 0.055, seam_dist);

        // Subtle recessed fasteners near plank ends.
        let nail_y = (along - 0.07).abs().min((along - 0.93).abs()) * 2.0;
        let nail_x = (inside - 0.22).abs().min((inside - 0.78).abs());
        let nail_d = (nail_x * nail_x + nail_y * nail_y).sqrt();
        let nail = 1.0 - smooth(0.012, 0.022, nail_d);

        let board_tint = lerp(0.84, 1.08, plank_seed);
        let w = wear.at(u, v);
        let shade = (ring * 0.48 + fiber * 0.34 + w * 0.18).clamp(0.0, 1.0);
        let mut wood_col = mix([0.44, 0.28, 0.16], [0.80, 0.60, 0.39], shade);
        wood_col = wood_col.map(|c| (c * board_tint).clamp(0.0, 1.0));
        wood_col = mix(wood_col, [0.24, 0.14, 0.08], pore * 0.45);
        // Dark nail heads & deep dark seam shadow.
        wood_col = mix(wood_col, [0.16, 0.15, 0.14], nail * 0.75);
        let color = mix([0.14, 0.09, 0.05], wood_col, groove);

        Texel {
            height: groove * 2.2 + bevel * 1.4 + ring * 0.45 + fiber * 0.4
                - pore * 0.9
                - nail * 1.6,
            color,
            roughness: (0.74 + (1.0 - w) * 0.14 + (1.0 - groove) * 0.22 + pore * 0.12)
                .clamp(0.0, 1.0),
            occlusion: (0.28 + groove * 0.57 + bevel * 0.15 - pore * 0.25 - nail * 0.35)
                .clamp(0.2, 1.0),
        }
    }
}

fn marble() -> impl Fn(f32, f32) -> Texel {
    let warp_a = Layers::new(61, 3, 3, 5);
    let warp_b = Layers::new(67, 5, 5, 4);
    let clouds = Layers::new(71, 3, 3, 4);
    let micro = Noise::new(73, 128, 128);
    move |u, v| {
        let wu = warp_a.at(u, v) * 2.0 - 1.0;
        let wv = warp_b.at(u + 0.31, v + 0.17) * 2.0 - 1.0;
        let du = u + wu * 0.22;
        let dv = v + wv * 0.22;

        // Primary Calacatta veins with organic domain warping.
        let primary_wave = (TAU * (du * 1.0 + dv * 2.0 + warp_a.at(du, dv) * 1.1))
            .sin()
            .abs();
        let primary = (1.0 - smooth(0.0, 0.14, primary_wave)).powf(1.6);

        // Secondary delicate branching mineral filaments.
        let branch_wave = (TAU * (du * 3.0 - dv * 2.0 + warp_b.at(dv, du) * 1.5))
            .sin()
            .abs();
        let branch = (1.0 - smooth(0.0, 0.055, branch_wave)) * 0.55;

        // Soft smoky mineral halo around primary veins.
        let halo = (1.0 - smooth(0.0, 0.36, primary_wave)) * 0.22;
        let cloud = clouds.at(du, dv);
        let stone = mix([0.88, 0.87, 0.85], [0.98, 0.975, 0.96], cloud);
        let stone = mix(stone, [0.72, 0.72, 0.74], halo);
        let vein = (primary + branch).clamp(0.0, 1.0);
        let color = mix(stone, [0.34, 0.35, 0.39], vein * 0.85);
        // Subtle warm gold/taupe mineral trace along the sharpest vein core.
        let core = (1.0 - smooth(0.0, 0.035, primary_wave)) * 0.3;
        let color = mix(color, [0.56, 0.50, 0.42], core);

        let grain = micro.at(u, v);
        Texel {
            // Very subtle micro-recess along mineral veins so specular highlights catch them.
            height: -vein * 0.35 + grain * 0.06,
            color,
            roughness: (0.52 + vein * 0.36 + grain * 0.08).clamp(0.0, 1.0),
            occlusion: (1.0 - vein * 0.14).clamp(0.7, 1.0),
        }
    }
}

fn granite() -> impl Fn(f32, f32) -> Texel {
    let coarse = Cellular::new(79, 44, 44);
    let fine = Cellular::new(83, 110, 110);
    let macro_var = Layers::new(89, 5, 5, 3);
    let sparkle = Noise::new(97, 220, 220);
    move |u, v| {
        let c1 = coarse.at(u, v);
        let c2 = fine.at(u, v);
        let blotch = macro_var.at(u, v);

        // Interlocking mineral crystals: feldspar (warm grey), quartz (pale), biotite (dark).
        let crystal = if c1.id < 0.24 {
            // Dark biotite mica.
            mix([0.14, 0.14, 0.15], [0.24, 0.24, 0.26], c2.id)
        } else if c1.id < 0.62 {
            // Feldspar (subtle warm-neutral stone tone).
            mix([0.48, 0.46, 0.45], [0.72, 0.69, 0.66], c2.id)
        } else {
            // Pale crystalline quartz.
            mix([0.74, 0.74, 0.73], [0.92, 0.91, 0.89], c2.id)
        };

        // Secondary fine crystal flecks break up larger grains.
        let fine_fleck = if c2.id < 0.18 {
            [0.12, 0.12, 0.13]
        } else if c2.id > 0.84 {
            [0.94, 0.93, 0.92]
        } else {
            crystal
        };
        let mut color = mix(crystal, fine_fleck, 0.45);
        color = color.map(|ch| (ch * lerp(0.88, 1.08, blotch)).clamp(0.0, 1.0));

        // Glinting mica micro-facets catch specular light.
        let glint = smooth(0.82, 0.94, sparkle.at(u, v));
        color = mix(color, [0.97, 0.96, 0.95], glint * 0.45);

        let edge_ao = smooth(0.0, 0.16, c1.edge);
        let facet_h = (1.0 - c1.f1).clamp(0.0, 1.0) * 0.9 + (1.0 - c2.f1).clamp(0.0, 1.0) * 0.45;
        Texel {
            height: facet_h * edge_ao,
            color,
            roughness: (0.86 - glint * 0.52 + (1.0 - edge_ao) * 0.12).clamp(0.2, 1.0),
            occlusion: (0.76 + edge_ao * 0.24).clamp(0.65, 1.0),
        }
    }
}

fn concrete() -> impl Fn(f32, f32) -> Texel {
    let clouds = Layers::new(101, 4, 4, 5);
    let trowel = Layers::new(103, 12, 4, 3);
    let aggregate = Cellular::new(107, 68, 68);
    let pits = Cellular::new(109, 36, 36);
    let cracks = Layers::new(113, 6, 6, 4);
    let grit = Noise::new(127, 240, 240);
    move |u, v| {
        let cloud = clouds.at(u, v);
        let sweep = trowel.at(u, v);
        let fine = grit.at(u, v);
        let agg = aggregate.at(u, v);

        // Air-bubble bug holes scattered selectively across cells.
        let pit_cell = pits.at(u, v);
        let pit = if pit_cell.id > 0.72 {
            1.0 - smooth(0.08, 0.24, pit_cell.f1)
        } else {
            0.0
        };

        // Very faint weathered hairline cracks.
        let ridge = cracks.ridged(u + cloud * 0.08, v + sweep * 0.08);
        let crack = smooth(0.89, 0.965, ridge) * 0.65;

        let shade = cloud * 0.48 + sweep * 0.22 + fine * 0.2 + agg.id * 0.1;
        let mut color = mix([0.52, 0.52, 0.50], [0.78, 0.775, 0.755], shade);
        color = mix(
            color,
            [0.26, 0.26, 0.25],
            (pit * 0.75 + crack * 0.55).min(0.85),
        );

        Texel {
            height: fine * 0.65 + sweep * 0.5 + (1.0 - agg.f1) * 0.35 - pit * 3.0 - crack * 2.0,
            color,
            roughness: (0.86 + fine * 0.1 + pit * 0.12).clamp(0.0, 1.0),
            occlusion: (1.0 - pit * 0.58 - crack * 0.42).clamp(0.3, 1.0),
        }
    }
}

fn slate() -> impl Fn(f32, f32) -> Texel {
    let warp = Layers::new(131, 4, 4, 4);
    let flakes = Cellular::new(137, 10, 18);
    let chips = Layers::new(139, 12, 24, 4);
    let fine = Noise::new(149, 190, 190);
    move |u, v| {
        let w = warp.at(u, v);
        let layered = v * 8.0 + w * 2.0;
        let step = layered.fract();
        let lip = smooth(0.0, 0.09, step);

        // Overlapping cleaved stone plates give authentic fractured slate planes.
        let plate = flakes.at(u + w * 0.07, v + w * 0.05);
        let plate_edge = smooth(0.0, 0.12, plate.edge);
        let chip = chips.at(u, v);
        let grain = fine.at(u, v);

        let tone = (plate.id * 0.38 + chip * 0.42 + w * 0.2).clamp(0.0, 1.0);
        let mut color = mix([0.22, 0.25, 0.29], [0.49, 0.53, 0.58], tone);
        // Subtle mineral sheen highlight on raised fractured shelves.
        color = mix(color, [0.60, 0.63, 0.67], smooth(0.75, 0.95, step) * 0.28);
        // Deep shadows in cleft steps and flake borders.
        let crevice = lip * plate_edge;
        color = mix([0.11, 0.13, 0.16], color, crevice);

        Texel {
            height: step * 3.2 * lip + plate.id * 1.8 * plate_edge + chip * 1.4 + grain * 0.35,
            color,
            roughness: (0.72 + (1.0 - plate.id) * 0.16 + (1.0 - crevice) * 0.2 + grain * 0.08)
                .clamp(0.0, 1.0),
            occlusion: (0.35 + crevice * 0.65).clamp(0.3, 1.0),
        }
    }
}

fn brick() -> impl Fn(f32, f32) -> Texel {
    const COURSES: f32 = 8.0;
    const PER_COURSE: f32 = 4.0;
    let tone = Noise::new(151, 8, 8);
    let edge_warp = Layers::new(157, 16, 16, 3);
    let grit = Layers::new(163, 72, 72, 3);
    let pits = Cellular::new(167, 84, 84);
    move |u, v| {
        // Gentle edge irregularity so bricks feel hand-laid rather than synthetic.
        let wu = (edge_warp.at(u, v) - 0.5) * 0.008;
        let wv = (edge_warp.at(u + 0.37, v + 0.61) - 0.5) * 0.008;
        let uu = (u + wu).rem_euclid(1.0);
        let vv = (v + wv).rem_euclid(1.0);

        let row = (vv * COURSES).floor();
        let shift = if (row as i32).rem_euclid(2) == 0 {
            0.0
        } else {
            0.5
        };
        let across = (uu * PER_COURSE + shift).rem_euclid(PER_COURSE);
        let column = across.floor();
        let (x, y) = (across - column, vv * COURSES - row);

        // Distance to nearest mortar joint in normalized brick-height units.
        let edge_x = x.min(1.0 - x) * (COURSES / PER_COURSE) * 2.0;
        let edge_y = y.min(1.0 - y);
        let edge = edge_x.min(edge_y);

        let brick_mask = smooth(0.055, 0.105, edge);
        let bevel = smooth(0.075, 0.19, edge);

        // Per-brick kiln-fired color variation across warm clay tones.
        let b1 = tone.at((column + 0.5) / 8.0, (row + 0.5) / 8.0);
        let b2 = tone.at((column + 3.5) / 8.0, (row + 2.5) / 8.0);
        let rough = grit.at(u, v);
        let pit_cell = pits.at(u, v);
        let pit = if pit_cell.id > 0.78 {
            1.0 - smooth(0.1, 0.28, pit_cell.f1)
        } else {
            0.0
        };

        let warm = mix(
            [0.46, 0.19, 0.13],
            [0.76, 0.36, 0.24],
            b1 * 0.72 + rough * 0.28,
        );
        let fired = mix(warm, [0.34, 0.16, 0.12], smooth(0.72, 0.95, b2) * 0.45);
        let fired = mix(fired, [0.24, 0.12, 0.09], pit * 0.35);

        let mortar = mix([0.56, 0.54, 0.50], [0.74, 0.72, 0.67], rough);
        let color = mix(mortar, fired, brick_mask);

        Texel {
            height: brick_mask * 3.4 + bevel * 1.8 + rough * 1.0 - pit * 1.1,
            color,
            roughness: (0.86 + rough * 0.1 + (1.0 - brick_mask) * 0.12).clamp(0.0, 1.0),
            occlusion: (0.34 + brick_mask * 0.46 + bevel * 0.2 - pit * 0.22).clamp(0.28, 1.0),
        }
    }
}

fn fabric() -> impl Fn(f32, f32) -> Texel {
    const THREADS: f32 = 56.0;
    let tone = Layers::new(173, 5, 5, 3);
    let slub = Noise::new(179, 48, 48);
    let fuzz = Noise::new(181, 240, 240);
    move |u, v| {
        let su = u * THREADS;
        let sv = v * THREADS;
        let cu = su.floor() as i32;
        let cv = sv.floor() as i32;
        let fu = su.fract();
        let fv = sv.fract();

        // 2×2 basket/twill weave feels richer and less moire-prone than a 1×1 grid.
        let over = ((cu + cv) & 1) == 0;
        let (across, along) = if over { (fu, fv) } else { (fv, fu) };

        // Rounded yarn profile across the thread + arched crest along the crossover.
        let width_profile = (across * std::f32::consts::PI).sin().clamp(0.0, 1.0);
        let arch = (along * std::f32::consts::PI).sin().clamp(0.0, 1.0);
        // Subtle 2-ply yarn twist running diagonally across the thread.
        let twist = ((across * 3.0 + along * 2.0) * TAU).sin() * 0.12 + 0.88;
        let thread = width_profile.powf(0.55) * (0.45 + 0.55 * arch) * twist;

        let warp_weft_tint = if over { 1.0 } else { 0.93 };
        let macro_shade = tone.at(u, v);
        let yarn_var = slub.at(
            if over { cu as f32 / THREADS } else { u },
            if over { v } else { cv as f32 / THREADS },
        );
        let micro = fuzz.at(u, v);

        let shade = (thread * 0.68 + macro_shade * 0.18 + yarn_var * 0.14) * warp_weft_tint;
        let color = mix([0.52, 0.52, 0.53], [0.95, 0.95, 0.94], shade);

        Texel {
            height: thread * 1.45 + micro * 0.18,
            color,
            roughness: (0.9 + (1.0 - thread) * 0.1).clamp(0.0, 1.0),
            occlusion: (0.42 + thread * 0.58).clamp(0.38, 1.0),
        }
    }
}

fn sand() -> impl Fn(f32, f32) -> Texel {
    let warp = Layers::new(191, 3, 3, 4);
    let drift = Layers::new(193, 4, 4, 3);
    let grains = Noise::new(197, 256, 256);
    let sparkles = Noise::new(199, 256, 256);
    move |u, v| {
        let w = warp.at(u, v);
        // Wind-blown asymmetric ripples: steeper windward face, gentler leeward slope.
        let phase = (6.0 * v + u + 1.2 * w).rem_euclid(1.0);
        let ripple = if phase < 0.32 {
            smooth(0.0, 0.32, phase)
        } else {
            1.0 - smooth(0.32, 1.0, phase)
        };
        let grain = grains.at(u, v);
        let glint = smooth(0.88, 0.97, sparkles.at(u, v));
        let macro_tone = drift.at(u, v);

        let shade = macro_tone * 0.35 + ripple * 0.3 + grain * 0.35;
        let mut color = mix([0.70, 0.59, 0.41], [0.94, 0.85, 0.66], shade);
        color = mix(color, [1.0, 0.98, 0.92], glint * 0.45);

        Texel {
            height: ripple * 2.4 + grain * 0.85,
            color,
            roughness: (0.96 - glint * 0.58).clamp(0.3, 1.0),
            occlusion: (0.78 + ripple * 0.22).clamp(0.7, 1.0),
        }
    }
}

fn metal() -> impl Fn(f32, f32) -> Texel {
    // Brushed/machined metal: subtle directional micro-scratches and faint patina.
    // Neutral bright albedo (~0.94..1.0) so the part's Color stays faithful.
    let brush_fine = Noise::new(211, 256, 12);
    let brush_med = Noise::new(223, 96, 6);
    let patina = Layers::new(227, 4, 4, 3);
    move |u, v| {
        let streak = brush_fine.at(u, v) * 0.6 + brush_med.at(u, v) * 0.4;
        let wear = patina.at(u, v);
        let albedo = 0.93 + streak * 0.05 + wear * 0.02;
        Texel {
            height: streak * 0.32 + wear * 0.1,
            color: [albedo, albedo, (albedo + 0.006).min(1.0)],
            roughness: (0.76 + (1.0 - streak) * 0.16 + (1.0 - wear) * 0.08).clamp(0.0, 1.0),
            occlusion: (0.93 + streak * 0.07).clamp(0.85, 1.0),
        }
    }
}

fn ice() -> impl Fn(f32, f32) -> Texel {
    // Crystalline frosted ice: internal Voronoi fractures, ridged frost veins,
    // and glossy-to-frosted roughness contrast.
    let crystals = Cellular::new(229, 8, 8);
    let micro_cells = Cellular::new(233, 24, 24);
    let frost = Layers::new(239, 5, 5, 4);
    let cracks = Layers::new(241, 6, 6, 4);
    move |u, v| {
        let f = frost.at(u, v);
        let c1 = crystals.at(u + f * 0.06, v + f * 0.06);
        let c2 = micro_cells.at(u, v);
        let crack = 1.0 - smooth(0.0, 0.055, c1.edge);
        let ridge = smooth(0.82, 0.96, cracks.ridged(u, v));
        let frost_patch = smooth(0.45, 0.82, f);

        let clear = [0.85, 0.94, 0.99];
        let frosted = [0.95, 0.98, 1.0];
        let mut color = mix(clear, frosted, frost_patch * 0.65 + c1.id * 0.2);
        // Bright white internal fracture lines.
        color = mix(
            color,
            [0.99, 1.0, 1.0],
            (crack * 0.7 + ridge * 0.45).min(1.0),
        );

        Texel {
            height: c1.id * 0.55 + (1.0 - c2.f1) * 0.3 - crack * 1.2 - ridge * 0.6,
            color,
            // Smooth glassy crystal faces with rougher frosted patches and cracks.
            roughness: (0.35 + frost_patch * 0.45 + crack * 0.25).clamp(0.15, 1.0),
            occlusion: (0.85 + (1.0 - crack * 0.25) * 0.15).clamp(0.65, 1.0),
        }
    }
}

fn rubber() -> impl Fn(f32, f32) -> Texel {
    // Fine molded tactile pebble grain with neutral matte albedo so Part Color tints cleanly.
    let pebbles = Cellular::new(251, 96, 96);
    let macro_var = Layers::new(257, 5, 5, 3);
    let micro = Noise::new(263, 240, 240);
    move |u, v| {
        let p = pebbles.at(u, v);
        let dome = (1.0 - smooth(0.15, 0.68, p.f1)).clamp(0.0, 1.0);
        let m = macro_var.at(u, v);
        let grain = micro.at(u, v);
        let tone = 0.86 + dome * 0.1 + m * 0.04;
        Texel {
            height: dome * 0.85 + grain * 0.2,
            color: [tone, tone, tone],
            roughness: (0.88 + (1.0 - dome) * 0.1 + grain * 0.04).clamp(0.0, 1.0),
            occlusion: (0.78 + dome * 0.22).clamp(0.7, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surfaces() -> impl Iterator<Item = (Kind, &'static Surface)> {
        Kind::ALL.into_iter().filter_map(|k| Some((k, of(k)?)))
    }

    #[test]
    fn the_textured_materials_have_maps_and_the_flat_ones_have_none() {
        let textured: Vec<Kind> = surfaces().map(|(kind, _)| kind).collect();
        assert_eq!(textured.len(), 12);
        for kind in [Kind::Plastic, Kind::SmoothPlastic, Kind::Glass, Kind::Neon] {
            assert!(of(kind).is_none());
        }
        for kind in [Kind::Metal, Kind::Ice, Kind::Rubber] {
            assert!(of(kind).is_some(), "{kind:?} should have a PBR surface");
        }
    }

    #[test]
    fn each_built_in_surface_looks_different() {
        let looks: Vec<&[u8]> = surfaces().map(|(_, s)| s.color.rgba()).collect();
        for (i, a) in looks.iter().enumerate() {
            for b in &looks[i + 1..] {
                let differing = a.iter().zip(b.iter()).filter(|(x, y)| x != y).count();
                assert!(differing > a.len() / 2, "two surfaces look the same");
            }
        }
    }

    #[test]
    fn maps_have_the_right_size_and_their_own_keys() {
        let mut keys = Vec::new();
        for (_, surface) in surfaces() {
            for map in [
                &surface.color,
                &surface.normal,
                &surface.roughness,
                &surface.occlusion,
            ] {
                assert_eq!(map.size(), (SIZE as u32, SIZE as u32));
                assert_eq!(map.rgba().len(), SIZE * SIZE * 4);
                keys.push(map.key());
            }
        }
        let count = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), count);
    }

    #[test]
    fn noise_and_cellular_wrap_around_so_tiles_meet() {
        let noise = Layers::new(5, 4, 4, 3);
        let cells = Cellular::new(7, 6, 6);
        for i in 0..32 {
            let t = i as f32 / 32.0;
            assert!((noise.at(0.0, t) - noise.at(1.0, t)).abs() < 1e-4);
            assert!((noise.at(t, 0.0) - noise.at(t, 1.0)).abs() < 1e-4);
            let a = cells.at(0.0, t);
            let b = cells.at(1.0, t);
            assert!((a.f1 - b.f1).abs() < 1e-4);
            assert!((a.edge - b.edge).abs() < 1e-4);
            let c = cells.at(t, 0.0);
            let d = cells.at(t, 1.0);
            assert!((c.f1 - d.f1).abs() < 1e-4);
            assert!((c.edge - d.edge).abs() < 1e-4);
        }
    }
}
