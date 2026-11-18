// Skin shading (`Shading::Skin`, `docs/rendering/skin.md`): compiled with
// shader.wgsl, whose material bindings (`maps`, the normal map, and the
// specular map's slot, which holds skin's defined normal map) and
// `lit_pixel` it works with, after lighting.wgsl, whose lights, shadows
// and sky it takes.
//
// - Diffuse: light enters skin and comes out a few millimetres away, red
//   furthest. So the diffuse light is lit by the normal map read blurred
//   (pores are smaller than that distance; red reads it blurrier still),
//   and spread across the line where the light ends by a table
//   pre-integrated from human skin's diffusion profile over a ring of the
//   surface's curvature (Penner and Borshukov, "Pre-Integrated Skin
//   Shading", GPU Pro 2, 2011; the profile is d'Eon and Luebke's six
//   Gaussians, GPU Gems 3, chapter 14). A shadow map's soft edge is
//   widened in red the same way; clouds and contact shadows stay grey.
// - Specular: two GGX lobes, a sharp and a broad one, on the full normal
//   map, reflecting 2.8% head-on in white (index of refraction 1.4). Their
//   roughness is widened by how fast the normal turns within a pixel
//   (Kaplanyan, Hill, Patney and Lefohn, "Filtering Distributions of
//   Normals for Shading Antialiasing", 2016), so pores don't glitter with
//   or without TAA.
//
// `Maps::shading` for skin: x SHADING_SKIN; y the defined normal map's
// share (0 without one); z how far light spreads (`SkinShading::scatter`,
// 1 is human skin's); w 1 if the defined normal map is BC5.

// Skin's reflectance head-on (index of refraction 1.4).
const SKIN_F0: f32 = 0.028;
// The two specular lobes' roughness, as a multiple of the material's, and
// the sharp lobe's share (Unreal's dual specular defaults: 0.75, 1.3 and a
// mix of 0.85 to the broad one).
const SKIN_SHARP: f32 = 0.75;
const SKIN_BROAD: f32 = 1.3;
const SKIN_SHARP_SHARE: f32 = 0.15;
// The engine's scale for the diffusion profile: a stud is 0.35 m.
const SKIN_MM_PER_UNIT: f32 = 350.0;
// How blurred the normal map is for diffuse light at scatter 1: the mip
// level of a 6-texel footprint (a texel of the realistic body is about
// 0.4 mm on the face and 0.9 mm on the body), and red a level and a half
// blurrier. Never sharper than one level past the pixel's own footprint,
// so a far face isn't grained by its pixel-sized pores either.
const SKIN_DIFFUSE_LEVEL: f32 = 2.5;
const SKIN_RED_LEVELS: f32 = 1.5;
// The most red's blurrier normal brings it over what green's normal gives
// red (in a fold the sculpt's map turns from the light, the blurrier
// normal faces it again; unheld, the fold filled with saturated red).
const SKIN_RED_FILL: f32 = 1.5;
// How much of the full normal map's detail green and blue diffuse light
// keep (red none): blue light scatters least, so it shows the most.
const SKIN_DETAIL: vec3<f32> = vec3<f32>(0.0, 0.1, 0.2);
// What each channel of a shadow's edge is raised to at scatter 1: below 1
// widens it, so the edge of a shadow on skin is redder (light scatters
// in under it).
const SKIN_PENUMBRA: vec3<f32> = vec3<f32>(0.55, 0.85, 0.95);
// The most red a shadow's edge has over its green: red at most half as
// bright again, and a little, so deep inside a shadow skin doesn't glow.
const SKIN_PENUMBRA_RED: vec2<f32> = vec2<f32>(1.5, 0.02);
// Over how much cosine, from where the triangles' own light ends, a
// normal map's relief comes back in full: below it, diffuse light is held
// to what the triangles get (`skin_light`).
const SKIN_FACING: f32 = 0.15;
// The table's smallest row: a 4 mm radius. The curvature is the
// triangles' (screen-space derivatives), which jumps at a nostril's rim
// or a crease's edge; sharper rows only redden those.
const SKIN_SMALLEST_ROW: f32 = 1.0;
// Specular antialiasing: the variance of a pixel's filter, and the most
// roughness it adds (alpha squared), as Filament has them.
const SKIN_FILTER_VARIANCE: f32 = 0.15915494;
const SKIN_MOST_WIDENING: f32 = 0.18;

// What a skin pixel's neighbourhood says, worked out before anything
// branches (derivatives need it): how many texels of the normal map a
// pixel spans (a mip level), how curved the surface is (1 / its radius,
// in world units; 0 where it is flat or hollow), and how fast the shading
// normal turns across a pixel.
struct SkinFootprint {
    level: f32,
    curvature: f32,
    variance: f32,
}

fn skin_footprint(in: VertexOut, n: vec3<f32>, facing: f32) -> SkinFootprint {
    var out = SkinFootprint(0.0, 0.0, 0.0);
    // `maps` is uniform, so every pixel takes the same branch.
    if maps.shading.x != SHADING_SKIN {
        return out;
    }
    let size = vec2<f32>(textureDimensions(normal_texture));
    let across = dpdx(in.uv) * size;
    let down = dpdy(in.uv) * size;
    out.level = 0.5 * log2(max(max(dot(across, across), dot(down, down)), 1e-8));
    // Signed: how far the normal turns along each step across the
    // surface (1 / radius on a ball). A crease (an ear's fold, a nostril,
    // where the nose meets the lip) turns the other way and counts as flat:
    // the table is pre-integrated round a ball, whose light reaches past
    // where it ends; drawn so, a crease took a band of saturated red.
    let geometric = normalize(in.normal) * facing;
    let right = dpdx(in.world_position);
    let below = dpdy(in.world_position);
    let bend = dot(dpdx(geometric), right) + dot(dpdy(geometric), below);
    let step = dot(right, right) + dot(below, below);
    out.curvature = max(bend, 0.0) / max(step, 1e-14);
    let turn_x = dpdx(n);
    let turn_y = dpdy(n);
    out.variance = dot(turn_x, turn_x) + dot(turn_y, turn_y);
    return out;
}

// The defined normal map's texel at `uv`, for `surface_normal`: read only
// on skin with a share of it (`maps` is uniform, so this branch is too).
fn skin_defined_texel(uv: vec2<f32>) -> vec3<f32> {
    if maps.shading.x == SHADING_SKIN && maps.shading.y > 0.0 {
        return textureSample(specular_texture, material_sampler, uv).xyz;
    }
    return vec3<f32>(0.5, 0.5, 1.0);
}

// The surface normal with both normal maps read at mip `level`: blurred,
// for diffuse light.
fn skin_normal_at(in: VertexOut, facing: f32, level: f32) -> vec3<f32> {
    let texel = textureSampleLevel(normal_texture, material_sampler, in.uv, level).xyz;
    var defined = vec3<f32>(0.5, 0.5, 1.0);
    if maps.shading.y > 0.0 {
        defined = textureSampleLevel(specular_texture, material_sampler, in.uv, level).xyz;
    }
    return mapped_normal(normalize(in.normal) * facing, in.tangent * facing, texel, defined);
}

// Perceptual roughness widened by how fast the normal turns within a
// pixel (`variance`), so detail finer than a pixel reads as roughness.
fn skin_widened(roughness: f32, variance: f32) -> f32 {
    let alpha = roughness * roughness;
    let widened = alpha * alpha + min(2.0 * SKIN_FILTER_VARIANCE * variance, SKIN_MOST_WIDENING);
    return sqrt(sqrt(saturate(widened)));
}

// The pre-integrated table at cosine `n_dot_l`, on `row` (0 for a 2 mm
// radius, each row doubling it, 6 for 128 mm), bilinear.
fn skin_table(n_dot_l: f32, row: f32) -> vec3<f32> {
    let column = (clamp(n_dot_l, -1.0, 1.0) * 0.5 + 0.5) * f32(SKIN_COLUMNS - 1u);
    let c = min(u32(column), SKIN_COLUMNS - 2u);
    let r = min(u32(row), SKIN_ROWS - 2u);
    let along = column - f32(c);
    let between = clamp(row - f32(r), 0.0, 1.0);
    let at = r * SKIN_COLUMNS + c;
    let near = mix(SKIN_TABLE[at], SKIN_TABLE[at + 1u], along);
    let far = mix(SKIN_TABLE[at + SKIN_COLUMNS], SKIN_TABLE[at + SKIN_COLUMNS + 1u], along);
    return mix(near, far, between);
}

// How much of a light a shadow's edge lets through on skin, by channel:
// `shadow` the shadow map's (or rays') visibility, widened in red only
// across its soft edge (4v(1 - v) is 1 half way across it and 0 in full
// light and full shadow), and red held to `SKIN_PENUMBRA_RED` of green.
fn skin_penumbra(shadow: f32, scatter: f32) -> vec3<f32> {
    let v = saturate(shadow);
    let widened = pow(vec3<f32>(v), pow(SKIN_PENUMBRA, vec3<f32>(scatter)));
    var tint = mix(vec3<f32>(v), widened, 4.0 * v * (1.0 - v));
    tint.r = min(tint.r, tint.g * SKIN_PENUMBRA_RED.x + SKIN_PENUMBRA_RED.y);
    return tint;
}

// The diffuse and specular light one light leaves on skin, before its
// colour and strength (`Arrival::radiance`): `n` the full normal,
// `diffuse_n` and `red_n` the blurred ones, `row` the table's row (or
// below 0 for no spreading at all), `sharp` and `broad` the lobes'
// roughness. Only `parts.shadow` reddens; `parts.veil` (clouds, contact
// shadows) dims every channel alike.
fn skin_light(
    n: vec3<f32>,
    diffuse_n: vec3<f32>,
    red_n: vec3<f32>,
    geometric: vec3<f32>,
    v: vec3<f32>,
    parts: ArrivalParts,
    base_color: vec3<f32>,
    row: f32,
    sharp: f32,
    broad: f32,
) -> vec3<f32> {
    let arrival = parts.arrival;
    let l = arrival.l;
    let h = normalize(v + l);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0001);
    let n_dot_h = max(dot(n, h), 0.0);
    let fresnel = fresnel_schlick(max(dot(v, h), 0.0), vec3<f32>(SKIN_F0)).x;

    var spread = vec3<f32>(max(dot(diffuse_n, l), 0.0));
    var shadow = vec3<f32>(arrival.visible);
    if row >= 0.0 {
        // Green and blue keep a little of the surface's detail (blue,
        // which scatters least, the most); red spreads at least as far as
        // they do, so a blurrier red normal facing away (in a nostril, an
        // ear's fold) never leaves a cyan fringe, and its blurrier normal
        // adds at most `SKIN_RED_FILL` of what green's normal gives red: a
        // few millimetres of spreading, not a fold the sculpt's normal
        // map turns away from the light filled with saturated red.
        let green = skin_table(dot(normalize(mix(diffuse_n, n, SKIN_DETAIL.g)), l), row);
        let blue = skin_table(dot(normalize(mix(diffuse_n, n, SKIN_DETAIL.b)), l), row);
        let red = clamp(skin_table(dot(red_n, l), row).r, green.r, green.r * SKIN_RED_FILL);
        spread = vec3<f32>(red, green.g, blue.b);
        shadow = skin_penumbra(parts.shadow, maps.shading.z) * parts.veil;
    }
    // Light reaches only a surface that faces it. Past where the
    // triangles' own light ends, a normal map's slope toward the light (a
    // navel's inner wall in the body's shadowed half) gets no more than
    // the triangles do, which the table still spreads past that line by
    // the surface's curvature; over `SKIN_FACING` of cosine on the lit side
    // the relief comes back. Unheld, the wall glowed orange in shadow
    // (Standard's `bump_shadowing` ends its light at the same line).
    let own_cos = dot(geometric, l);
    var own = vec3<f32>(max(own_cos, 0.0));
    if row >= 0.0 {
        own = skin_table(own_cos, row);
    }
    spread = mix(min(spread, own), spread, smoothstep(0.0, SKIN_FACING, own_cos));
    let diffuse = (1.0 - fresnel) * base_color / PI * spread * shadow;

    let lobes = mix(distribution_ggx(n_dot_h, broad), distribution_ggx(n_dot_h, sharp),
        SKIN_SHARP_SHARE);
    let specular = lobes * geometry_smith(n_dot_v, n_dot_l, mix(broad, sharp, SKIN_SHARP_SHARE))
        * fresnel / (4.0 * n_dot_v * n_dot_l + 0.0001) * n_dot_l
        * arrival.visible * bump_shadowing(geometric, n, l);
    return diffuse + vec3<f32>(specular);
}

// The light a skin pixel gives back toward the camera: the sun, the
// lamps listed for its tile and the sky, as `light_surface` gathers them.
fn skin_surface(
    in: VertexOut,
    facing: f32,
    geometric: vec3<f32>,
    shaded: Surface,
    roughness: f32,
    occlusion: f32,
    solid: bool,
    footprint: SkinFootprint,
) -> vec3<f32> {
    let weather = weathered(geometric, shaded.normal, shaded.base_color, 0.0, roughness);
    let n = weather.normal;
    let base_color = weather.base_color;
    let world_position = in.world_position;
    let pixel = in.clip_position.xy;
    let v = normalize(frame.camera_position.xyz - world_position);
    let n_dot_v = max(dot(n, v), 0.0001);

    // The blurred normals for diffuse light, and the table's row from the
    // surface's radius over how far light spreads (none: Lambert's light).
    let scatter = maps.shading.z;
    var diffuse_n = n;
    var red_n = n;
    var row = -1.0;
    if scatter > 0.0 {
        if maps.settings.x != 0.0 {
            let level = max(footprint.level + 1.0, SKIN_DIFFUSE_LEVEL + log2(scatter));
            diffuse_n = skin_normal_at(in, facing, level);
            red_n = skin_normal_at(in, facing, level + SKIN_RED_LEVELS);
        }
        let radius_mm = 1.0 / max(footprint.curvature / SKIN_MM_PER_UNIT * scatter, 1e-6);
        row = clamp(log2(radius_mm / 2.0), SKIN_SMALLEST_ROW, f32(SKIN_ROWS - 1u));
    }

    let sharp = skin_widened(clamp(weather.roughness * SKIN_SHARP, 0.04, 1.0), footprint.variance);
    let broad = skin_widened(clamp(weather.roughness * SKIN_BROAD, 0.04, 1.0), footprint.variance);

    let sun = sun_arrival_parts(world_position, geometric, pixel, solid);
    var lit = skin_light(n, diffuse_n, red_n, geometric, v, sun, base_color, row, sharp, broad)
        * sun.arrival.radiance;
    let lamps = tile_lamps(pixel);
    var slot = 0u;
    for (var i = 0u; i < lamps.count; i++) {
        let lamp = lamp_arrival_parts(lamps, i, slot, world_position, geometric, pixel, solid);
        slot += lamp_slots(lamps, i);
        if lamp.arrival.visible > 0.0 {
            lit += skin_light(n, diffuse_n, red_n, geometric, v, lamp, base_color, row, sharp,
                broad) * lamp.arrival.radiance;
        }
    }

    // Light from the sky, and a little flat ambient light on top.
    let average = mix(broad, sharp, SKIN_SHARP_SHARE);
    let reflection = reflect(-v, n);
    let environment = frame.sky_top.w;
    let reflectance = ambient_specular(vec3<f32>(SKIN_F0), average, n_dot_v);
    let ambient = occlusion * (
        frame.ambient.rgb * (base_color + reflectance)
        + surrounding_diffuse(world_position, diffuse_n, environment) * base_color
        + traced_reflected(world_position, reflection, average, environment, pixel, solid)
            * reflectance);
    return lit + ambient;
}

// -- Pores and fine lines ----------------------------------------------------
//
// Detail finer than the skin's maps hold (a texel of the realistic face is
// 0.4 mm, and its baked grain's pores are about 2 mm apart): pores, a
// rounded pit with a low rim at a jittered point in half the cells of a
// grid in texture space, and two crossing sets of faint fine lines, the contours of a
// stretched value noise. Worked out per pixel from the texture
// coordinates, so they stay on the skin as it moves; their slope is found
// analytically and tilts the normal by the screen-space change of the
// position and texture coordinates (Mikkelsen, "Bump Mapping
// Unparametrized Surfaces on the GPU", 2010, as the scar's relief), which
// needs no tangents. A pore also darkens the albedo a little (its
// cavity). Drawn where a cell spans more than about two pixels, fading
// out to none at about one: smaller, it would alias, and the skin's
// roughness and its antialiasing (`skin_widened`) stand for it. As strong
// as the normal map (`Maps::settings.x`, which `detail_strength` scales).
// Only the full normal gets it: diffuse light's blurred normals don't, as
// pores are smaller than the distance light travels under skin.

// Cells across texture space: 0.7 mm on the realistic face (a texel of
// its 2048 maps is 0.4 mm there), so about 100 pores to a square
// centimetre; 1.6 mm on its body. (0.5 mm cells were under two pixels on
// a cheek seen close, and faded out.)
const SKIN_PORE_CELLS: f32 = 1150.0;
// The share of cells with a pore, a pore's radius (in cells: 0.3 to
// 0.5 mm across on the face), the deepest one's depth (mm), and how much
// a pore darkens the albedo at its deepest.
const SKIN_PORE_SHARE: f32 = 0.5;
const SKIN_PORE_RADIUS: vec2<f32> = vec2<f32>(0.22, 0.36);
const SKIN_PORE_DEPTH_MM: f32 = 0.05;
const SKIN_PORE_CAVITY: f32 = 0.2;
// A pore's rim stands this share of its depth above the skin, 0.8 to 1.3
// of its radius out: lit on the side toward a light.
const SKIN_PORE_RIM: f32 = 0.3;
// How much pores vary over the skin: their depth and cavity from this
// share to all of it, by a noise about 25 cells (18 mm on the face) across,
// so some patches show larger pores than others.
const SKIN_PORE_PATCHES: f32 = 0.6;
// Fine lines: their noise's frequency along and across them (per cell),
// half their width (in the noise's value), and their depth (mm).
const SKIN_LINE_FREQUENCY: vec2<f32> = vec2<f32>(0.15, 0.5);
const SKIN_LINE_HALF_WIDTH: f32 = 0.1;
const SKIN_LINE_DEPTH_MM: f32 = 0.015;
// Over how many cells to a pixel the detail fades: whole below the
// first, none above the second.
const SKIN_PORE_FADE: vec2<f32> = vec2<f32>(0.45, 0.9);

// Two random numbers 0 to 1 for integer cell `c` (pcg2d: Jarzynski and
// Olano, "Hash Functions for GPU Rendering", JCGT 2020).
fn skin_hash(c: vec2<i32>) -> vec2<f32> {
    var v = bitcast<vec2<u32>>(c) * 1664525u + 1013904223u;
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v = v ^ (v >> vec2<u32>(16u));
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v = v ^ (v >> vec2<u32>(16u));
    return vec2<f32>(v >> vec2<u32>(8u)) / 16777216.0;
}

// Value noise (0 to 1) at `p` in x, its slope in y and z: quintic between
// the lattice's points, so the slope is smooth (Quilez's derivation).
fn skin_noise(p: vec2<f32>) -> vec3<f32> {
    let i = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
    let a = skin_hash(i).x;
    let b = skin_hash(i + vec2<i32>(1, 0)).x;
    let c = skin_hash(i + vec2<i32>(0, 1)).x;
    let d = skin_hash(i + vec2<i32>(1, 1)).x;
    let k = a - b - c + d;
    let value = a + (b - a) * u.x + (c - a) * u.y + k * u.x * u.y;
    return vec3<f32>(value, du * vec2<f32>(b - a + k * u.y, c - a + k * u.x));
}

// The pores at `p` (in cells): their height in x (0 level, -1 the
// deepest pit), its slope per cell in y and z, and in w how deep the pits
// are here (0 to 1, for their cavity: the rims don't count).
fn skin_pores(p: vec2<f32>) -> vec4<f32> {
    let cell = vec2<i32>(floor(p));
    var out = vec4<f32>(0.0);
    // A pore's point is in the middle 60% of its cell and its rim under
    // 0.5 of a cell out, so only the eight cells round this one reach it.
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let c = cell + vec2<i32>(i, j);
            let kind = skin_hash(c + vec2<i32>(7919, 104729));
            if kind.x > SKIN_PORE_SHARE {
                continue;
            }
            let radius = mix(SKIN_PORE_RADIUS.x, SKIN_PORE_RADIUS.y, kind.y);
            let d = p - (vec2<f32>(c) + 0.2 + 0.6 * skin_hash(c));
            let s = length(d) / radius;
            if s >= 1.3 {
                continue;
            }
            // The larger pores are deeper.
            let depth = 0.5 + 0.5 * kind.y;
            if s < 1.0 {
                // A rounded pit, -(1 - s^2)^2: its walls are steepest a
                // little inside its edge.
                let k = 1.0 - s * s;
                out.x -= depth * k * k;
                out.w += depth * k * k;
                out = vec4<f32>(out.x, out.yz + depth * 4.0 * k * d / (radius * radius), out.w);
            }
            if s > 0.8 {
                // A low rim round it, (1 - t^2)^2 across 0.8 to 1.3 of the
                // radius (t from -1 to 1).
                let t = (s - 1.05) / 0.25;
                let m = 1.0 - t * t;
                let rim = SKIN_PORE_RIM * depth;
                out.x += rim * m * m;
                let along = -16.0 * rim * m * t;
                out = vec4<f32>(out.x, out.yz + along * d / (s * radius * radius), out.w);
            }
        }
    }
    return out;
}

// The fine lines at `p` (in cells): their height in x (0 level, -1 a
// groove's floor, -2 where two cross) and its slope per cell in y and z.
// Two sets, about 70 degrees apart, as skin's furrows cross.
fn skin_lines(p: vec2<f32>) -> vec3<f32> {
    var out = vec3<f32>(0.0);
    for (var family = 0; family < 2; family++) {
        let angle = select(-0.55, 0.65, family == 1);
        let along = vec2<f32>(cos(angle), sin(angle));
        let across = vec2<f32>(-along.y, along.x);
        let q = vec2<f32>(dot(p, along), dot(p, across)) * SKIN_LINE_FREQUENCY + f32(family) * 17.3;
        let noise = skin_noise(q);
        // A groove where the noise crosses its middle: -(1 - x^2)^2 of how
        // far from it, in half widths.
        let e = noise.x - 0.5;
        let x = abs(e) / SKIN_LINE_HALF_WIDTH;
        if x >= 1.0 {
            continue;
        }
        let g = 1.0 - x * x;
        out.x -= g * g;
        let slope = 4.0 * g * x * sign(e) / SKIN_LINE_HALF_WIDTH;
        let change = noise.yz * SKIN_LINE_FREQUENCY;
        out = vec3<f32>(out.x, out.yz + slope * (change.x * along + change.y * across));
    }
    return out;
}

// The normal `n` (at `position`, texture coordinates `uv`) tilted by the
// pores and fine lines, and in w how much the albedo darkens in a pore.
fn skin_micro(n: vec3<f32>, position: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    // (Every derivative before anything branches.)
    let uv_x = dpdx(uv);
    let uv_y = dpdy(uv);
    let across = dpdx(position);
    let down = dpdy(position);
    let cells = SKIN_PORE_CELLS * sqrt(max(dot(uv_x, uv_x), dot(uv_y, uv_y)));
    let shown = (1.0 - smoothstep(SKIN_PORE_FADE.x, SKIN_PORE_FADE.y, cells)) * maps.settings.x;
    let r1 = cross(down, n);
    let r2 = cross(n, across);
    let det = dot(across, r1);
    if shown <= 0.0 || abs(det) < 1e-20 {
        return vec4<f32>(n, 0.0);
    }
    let p = uv * SKIN_PORE_CELLS;
    let variation = mix(SKIN_PORE_PATCHES, 1.0, skin_noise(p * 0.04 + 41.7).x);
    let pores = skin_pores(p) * variation;
    let lines = skin_lines(p);
    // The height's slope per unit of texture space, in world units.
    let slope = (pores.yz * SKIN_PORE_DEPTH_MM + lines.yz * SKIN_LINE_DEPTH_MM)
        * (SKIN_PORE_CELLS / SKIN_MM_PER_UNIT) * shown;
    // Its gradient over the surface: the surface gradients of u and v
    // (times `det`), weighted by the slope along each.
    let gradient = sign(det) * (slope.x * (uv_x.x * r1 + uv_y.x * r2)
        + slope.y * (uv_x.y * r1 + uv_y.y * r2));
    let tilted = abs(det) * n - gradient;
    if dot(tilted, tilted) < 1e-20 {
        return vec4<f32>(n, 0.0);
    }
    return vec4<f32>(normalize(tilted), clamp(pores.w * SKIN_PORE_CAVITY * shown, 0.0, 0.5));
}

// -- Makeup and markings (ADR-0042) ----------------------------------------
//
// Drawn into the albedo and roughness before any light, so skin's
// diffusion and two specular lobes light them like the skin they are on:
// markings in the skin (freckles, moles, a scar: pigment, multiplied),
// then makeup over it from foundation up to brow fill (a thin layer mixed
// over the skin by its coverage), in the skin references' order
// (`references/skin/README.md` section 2). A layer keeps part of the
// skin's own fine variation (pores, mottling, the lips' lines), so it
// reads as makeup on skin, not paint. Masks (each channel a layer):
// `a` foundation, blush, eyeshadow, eyeliner; `b` lips, brow fill, scar;
// `c` freckles, their ranks, moles, their ranks. Values: `Maps::params`,
// `Maps::toon` and `Maps::specular.w` (renderer/lighting.rs,
// `skin_layer_uniform`).

// The skin's own tone: its albedo read over about 32 texels (3 cm on the
// realistic body, 1.3 cm on its face).
const SKIN_TONE_LEVEL: f32 = 5.0;
// The scale a layer's kept detail is measured against: about 8 texels.
const SKIN_DETAIL_LEVEL: f32 = 3.0;
// How much of the skin's variation shows through: foundation hides most
// of it (that is what it is for), other makeup is thinner.
const SKIN_FOUNDATION_DETAIL: f32 = 0.35;
const SKIN_MAKEUP_DETAIL: f32 = 0.8;
// What a freckle, a mole and a scar do to the albedo at full opacity:
// melanin (brown, darker in blue), more of it in a mole; a scar has less
// pigment and a little more red.
const SKIN_FRECKLE: vec3<f32> = vec3<f32>(0.62, 0.48, 0.38);
const SKIN_MOLE: vec3<f32> = vec3<f32>(0.32, 0.22, 0.17);
const SKIN_SCAR: vec3<f32> = vec3<f32>(1.28, 1.12, 1.12);
// A scar is smoother than the skin round it.
const SKIN_SCAR_ROUGHNESS: f32 = 0.32;

// How high a scar stands at full opacity, in world units (a stud is
// 0.35 m): a healed scar is a little raised, about a fifth of a millimetre.
const SKIN_SCAR_HEIGHT: f32 = 0.0006;

// `n` tilted by a scar standing `scar` (0 to 1) of `SKIN_SCAR_HEIGHT` out of
// the skin at `position`, by the screen-space change of both (Mikkelsen,
// "Bump Mapping Unparametrized Surfaces on the GPU", 2010): no tangents or
// height map needed, so the mask's own coverage is the height.
fn skin_scar_relief(n: vec3<f32>, position: vec3<f32>, scar: f32) -> vec3<f32> {
    let height = scar * SKIN_SCAR_HEIGHT;
    let across = dpdx(position);
    let down = dpdy(position);
    let r1 = cross(down, n);
    let r2 = cross(n, across);
    let det = dot(across, r1);
    let gradient = sign(det) * (dpdx(height) * r1 + dpdy(height) * r2);
    let tilted = abs(det) * n - gradient;
    if dot(tilted, tilted) < 1e-20 || abs(det) < 1e-20 {
        return n;
    }
    return normalize(tilted);
}

// Whether a spot of a marking shows: its `rank` (0 to 1, one per spot)
// offset by the marking's seed is below its amount, with a soft step so
// the edge of the amount doesn't flicker.
fn skin_spot_shown(rank: f32, amount: f32, seed: f32) -> f32 {
    // (At amount 1 every spot shows: the soft step would fade the last 2%.)
    if amount >= 1.0 {
        return 1.0;
    }
    let order = fract(rank + seed);
    return 1.0 - smoothstep(amount - 0.02, amount, order);
}

// The albedo and roughness (in w) with the makeup and markings over them:
// `albedo` and `roughness` as the material's maps give them, `tint` the
// material's colour (its tone), `a`, `b` and `c` the masks' texels.
fn skin_layered(
    albedo: vec3<f32>,
    roughness: f32,
    tint: vec3<f32>,
    uv: vec2<f32>,
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
) -> vec4<f32> {
    let own_tone = tint * textureSampleLevel(base_color_texture, material_sampler, uv,
        SKIN_TONE_LEVEL).rgb;
    let near = tint * textureSampleLevel(base_color_texture, material_sampler, uv,
        SKIN_DETAIL_LEVEL).rgb;
    // The skin's own variation at this texel, against its neighbourhood.
    let detail = clamp(albedo / max(near, vec3<f32>(1e-4)), vec3<f32>(0.5), vec3<f32>(2.0));
    var colour = albedo;
    var rough = roughness;

    // Markings, in the skin. Each spot is a little darker or lighter than
    // the next (by its rank).
    let freckle = c.r * skin_spot_shown(c.g, maps.params[7].z, maps.toon.w)
        * maps.params[7].w * (0.65 + 0.35 * fract(c.g * 7.31 + 0.17));
    colour *= mix(vec3<f32>(1.0), SKIN_FRECKLE, freckle);
    let mole = c.b * skin_spot_shown(c.a, maps.toon.x, maps.specular.w) * maps.toon.y;
    colour *= mix(vec3<f32>(1.0), SKIN_MOLE, mole);
    let scar = b.b * maps.toon.z;
    colour *= mix(vec3<f32>(1.0), SKIN_SCAR, scar);
    rough = mix(rough, SKIN_SCAR_ROUGHNESS, scar);

    // Makeup, foundation first.
    let masks = array<f32, 6>(a.r, a.g, a.b, a.a, b.r, b.g);
    let roughness_of = array<f32, 6>(maps.params[6].x, maps.params[6].y, maps.params[6].z,
        maps.params[6].w, maps.params[7].x, maps.params[7].y);
    for (var i = 0; i < 6; i++) {
        let layer = maps.params[i];
        let cover = masks[i] * layer.a;
        if cover <= 0.0 {
            continue;
        }
        var pigment = layer.rgb;
        if layer.r < 0.0 {
            pigment = own_tone;
        }
        let keep = select(SKIN_MAKEUP_DETAIL, SKIN_FOUNDATION_DETAIL, i == 0);
        pigment *= mix(vec3<f32>(1.0), detail, keep);
        colour = mix(colour, pigment, cover);
        if roughness_of[i] >= 0.0 {
            rough = mix(rough, roughness_of[i], cover);
        }
    }
    return vec4<f32>(colour, rough);
}

// The pre-integrated table: diffuse light against the cosine of the angle
// to the light (33 columns, -1 to 1) on a ring of radius 2, 4, 8, 16, 32,
// 64 and 128 mm (7 rows), red, green and blue. Written by the render
// crate's test `the_skin_table_is_skins_profile_pre_integrated`, which
// prints it again if it is wrong.
const SKIN_COLUMNS: u32 = 33u;
const SKIN_ROWS: u32 = 7u;
var<private> SKIN_TABLE: array<vec3<f32>, 231> = array<vec3<f32>, 231>(
    // 2 mm
    vec3<f32>(0.0037, 0.0000, 0.0000), vec3<f32>(0.0043, 0.0000, 0.0000), vec3<f32>(0.0049, 0.0000, 0.0000),
    vec3<f32>(0.0055, 0.0000, 0.0000), vec3<f32>(0.0063, 0.0000, 0.0000), vec3<f32>(0.0071, 0.0000, 0.0000),
    vec3<f32>(0.0080, 0.0000, 0.0000), vec3<f32>(0.0090, 0.0001, 0.0000), vec3<f32>(0.0102, 0.0001, 0.0000),
    vec3<f32>(0.0115, 0.0002, 0.0000), vec3<f32>(0.0131, 0.0003, 0.0000), vec3<f32>(0.0149, 0.0006, 0.0001),
    vec3<f32>(0.0171, 0.0010, 0.0001), vec3<f32>(0.0199, 0.0020, 0.0004), vec3<f32>(0.0236, 0.0039, 0.0012),
    vec3<f32>(0.0294, 0.0084, 0.0041), vec3<f32>(0.0470, 0.0259, 0.0206), vec3<f32>(0.0899, 0.0707, 0.0665),
    vec3<f32>(0.1446, 0.1285, 0.1260), vec3<f32>(0.2014, 0.1888, 0.1876), vec3<f32>(0.2592, 0.2502, 0.2497),
    vec3<f32>(0.3175, 0.3120, 0.3120), vec3<f32>(0.3762, 0.3740, 0.3744), vec3<f32>(0.4351, 0.4362, 0.4368),
    vec3<f32>(0.4943, 0.4984, 0.4992), vec3<f32>(0.5536, 0.5606, 0.5615), vec3<f32>(0.6131, 0.6229, 0.6239),
    vec3<f32>(0.6727, 0.6852, 0.6863), vec3<f32>(0.7324, 0.7475, 0.7487), vec3<f32>(0.7922, 0.8097, 0.8111),
    vec3<f32>(0.8520, 0.8720, 0.8735), vec3<f32>(0.9119, 0.9343, 0.9359), vec3<f32>(0.9719, 0.9966, 0.9983),
    // 4 mm
    vec3<f32>(0.0002, 0.0000, 0.0000), vec3<f32>(0.0003, 0.0000, 0.0000), vec3<f32>(0.0003, 0.0000, 0.0000),
    vec3<f32>(0.0004, 0.0000, 0.0000), vec3<f32>(0.0005, 0.0000, 0.0000), vec3<f32>(0.0007, 0.0000, 0.0000),
    vec3<f32>(0.0009, 0.0000, 0.0000), vec3<f32>(0.0012, 0.0000, 0.0000), vec3<f32>(0.0016, 0.0000, 0.0000),
    vec3<f32>(0.0021, 0.0000, 0.0000), vec3<f32>(0.0028, 0.0000, 0.0000), vec3<f32>(0.0036, 0.0000, 0.0000),
    vec3<f32>(0.0048, 0.0001, 0.0000), vec3<f32>(0.0064, 0.0002, 0.0000), vec3<f32>(0.0086, 0.0005, 0.0001),
    vec3<f32>(0.0122, 0.0019, 0.0006), vec3<f32>(0.0244, 0.0130, 0.0103), vec3<f32>(0.0742, 0.0644, 0.0631),
    vec3<f32>(0.1327, 0.1254, 0.1250), vec3<f32>(0.1925, 0.1875, 0.1874), vec3<f32>(0.2529, 0.2498, 0.2499),
    vec3<f32>(0.3138, 0.3123, 0.3124), vec3<f32>(0.3749, 0.3747, 0.3748), vec3<f32>(0.4363, 0.4371, 0.4373),
    vec3<f32>(0.4978, 0.4996, 0.4998), vec3<f32>(0.5594, 0.5620, 0.5623), vec3<f32>(0.6212, 0.6245, 0.6247),
    vec3<f32>(0.6830, 0.6869, 0.6872), vec3<f32>(0.7449, 0.7494, 0.7497), vec3<f32>(0.8068, 0.8118, 0.8122),
    vec3<f32>(0.8687, 0.8743, 0.8746), vec3<f32>(0.9307, 0.9367, 0.9371), vec3<f32>(0.9926, 0.9992, 0.9996),
    // 8 mm
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0001, 0.0000, 0.0000),
    vec3<f32>(0.0001, 0.0000, 0.0000), vec3<f32>(0.0002, 0.0000, 0.0000), vec3<f32>(0.0004, 0.0000, 0.0000),
    vec3<f32>(0.0007, 0.0000, 0.0000), vec3<f32>(0.0013, 0.0000, 0.0000), vec3<f32>(0.0023, 0.0000, 0.0000),
    vec3<f32>(0.0042, 0.0003, 0.0000), vec3<f32>(0.0122, 0.0065, 0.0051), vec3<f32>(0.0666, 0.0627, 0.0625),
    vec3<f32>(0.1271, 0.1250, 0.1250), vec3<f32>(0.1885, 0.1875, 0.1875), vec3<f32>(0.2503, 0.2499, 0.2500),
    vec3<f32>(0.3124, 0.3124, 0.3125), vec3<f32>(0.3746, 0.3749, 0.3750), vec3<f32>(0.4369, 0.4374, 0.4375),
    vec3<f32>(0.4992, 0.4999, 0.4999), vec3<f32>(0.5616, 0.5624, 0.5624), vec3<f32>(0.6239, 0.6249, 0.6249),
    vec3<f32>(0.6863, 0.6874, 0.6874), vec3<f32>(0.7487, 0.7498, 0.7499), vec3<f32>(0.8111, 0.8123, 0.8124),
    vec3<f32>(0.8735, 0.8748, 0.8749), vec3<f32>(0.9359, 0.9373, 0.9374), vec3<f32>(0.9983, 0.9998, 0.9999),
    // 16 mm
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0001, 0.0000, 0.0000), vec3<f32>(0.0004, 0.0000, 0.0000),
    vec3<f32>(0.0011, 0.0000, 0.0000), vec3<f32>(0.0061, 0.0032, 0.0026), vec3<f32>(0.0636, 0.0625, 0.0625),
    vec3<f32>(0.1253, 0.1250, 0.1250), vec3<f32>(0.1875, 0.1875, 0.1875), vec3<f32>(0.2499, 0.2500, 0.2500),
    vec3<f32>(0.3124, 0.3125, 0.3125), vec3<f32>(0.3748, 0.3750, 0.3750), vec3<f32>(0.4373, 0.4375, 0.4375),
    vec3<f32>(0.4998, 0.5000, 0.5000), vec3<f32>(0.5623, 0.5625, 0.5625), vec3<f32>(0.6247, 0.6250, 0.6250),
    vec3<f32>(0.6872, 0.6875, 0.6875), vec3<f32>(0.7497, 0.7500, 0.7500), vec3<f32>(0.8122, 0.8125, 0.8125),
    vec3<f32>(0.8746, 0.8750, 0.8750), vec3<f32>(0.9371, 0.9375, 0.9375), vec3<f32>(0.9996, 0.9999, 1.0000),
    // 32 mm
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0002, 0.0000, 0.0000), vec3<f32>(0.0031, 0.0016, 0.0013), vec3<f32>(0.0627, 0.0625, 0.0625),
    vec3<f32>(0.1250, 0.1250, 0.1250), vec3<f32>(0.1875, 0.1875, 0.1875), vec3<f32>(0.2500, 0.2500, 0.2500),
    vec3<f32>(0.3125, 0.3125, 0.3125), vec3<f32>(0.3750, 0.3750, 0.3750), vec3<f32>(0.4375, 0.4375, 0.4375),
    vec3<f32>(0.4999, 0.5000, 0.5000), vec3<f32>(0.5624, 0.5625, 0.5625), vec3<f32>(0.6249, 0.6250, 0.6250),
    vec3<f32>(0.6874, 0.6875, 0.6875), vec3<f32>(0.7499, 0.7500, 0.7500), vec3<f32>(0.8124, 0.8125, 0.8125),
    vec3<f32>(0.8749, 0.8750, 0.8750), vec3<f32>(0.9374, 0.9375, 0.9375), vec3<f32>(0.9999, 1.0000, 1.0000),
    // 64 mm
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0015, 0.0008, 0.0006), vec3<f32>(0.0625, 0.0625, 0.0625),
    vec3<f32>(0.1250, 0.1250, 0.1250), vec3<f32>(0.1875, 0.1875, 0.1875), vec3<f32>(0.2500, 0.2500, 0.2500),
    vec3<f32>(0.3125, 0.3125, 0.3125), vec3<f32>(0.3750, 0.3750, 0.3750), vec3<f32>(0.4375, 0.4375, 0.4375),
    vec3<f32>(0.5000, 0.5000, 0.5000), vec3<f32>(0.5625, 0.5625, 0.5625), vec3<f32>(0.6250, 0.6250, 0.6250),
    vec3<f32>(0.6875, 0.6875, 0.6875), vec3<f32>(0.7500, 0.7500, 0.7500), vec3<f32>(0.8125, 0.8125, 0.8125),
    vec3<f32>(0.8750, 0.8750, 0.8750), vec3<f32>(0.9375, 0.9375, 0.9375), vec3<f32>(1.0000, 1.0000, 1.0000),
    // 128 mm
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0000, 0.0000, 0.0000),
    vec3<f32>(0.0000, 0.0000, 0.0000), vec3<f32>(0.0008, 0.0004, 0.0003), vec3<f32>(0.0625, 0.0625, 0.0625),
    vec3<f32>(0.1250, 0.1250, 0.1250), vec3<f32>(0.1875, 0.1875, 0.1875), vec3<f32>(0.2500, 0.2500, 0.2500),
    vec3<f32>(0.3125, 0.3125, 0.3125), vec3<f32>(0.3750, 0.3750, 0.3750), vec3<f32>(0.4375, 0.4375, 0.4375),
    vec3<f32>(0.5000, 0.5000, 0.5000), vec3<f32>(0.5625, 0.5625, 0.5625), vec3<f32>(0.6250, 0.6250, 0.6250),
    vec3<f32>(0.6875, 0.6875, 0.6875), vec3<f32>(0.7500, 0.7500, 0.7500), vec3<f32>(0.8125, 0.8125, 0.8125),
    vec3<f32>(0.8750, 0.8750, 0.8750), vec3<f32>(0.9375, 0.9375, 0.9375), vec3<f32>(1.0000, 1.0000, 1.0000),
);
