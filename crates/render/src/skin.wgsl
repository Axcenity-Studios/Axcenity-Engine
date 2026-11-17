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
//   Gaussians, GPU Gems 3, chapter 14). Shadow edges are widened in red
//   the same way.
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
// How much of the full normal map's detail green and blue diffuse light
// keep (red none): blue light scatters least, so it shows the most.
const SKIN_DETAIL: vec3<f32> = vec3<f32>(0.0, 0.1, 0.2);
// What each channel of a shadow's edge is raised to at scatter 1: below 1
// widens it, so the edge of a shadow on skin is redder (light scatters
// in under it).
const SKIN_PENUMBRA: vec3<f32> = vec3<f32>(0.55, 0.85, 0.95);
// Specular antialiasing: the variance of a pixel's filter, and the most
// roughness it adds (alpha squared), as Filament has them.
const SKIN_FILTER_VARIANCE: f32 = 0.15915494;
const SKIN_MOST_WIDENING: f32 = 0.18;

// What a skin pixel's neighbourhood says, worked out before anything
// branches (derivatives need it): how many texels of the normal map a
// pixel spans (a mip level), how curved the surface is (1 / its radius,
// in world units), and how fast the shading normal turns across a pixel.
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
    let geometric = normalize(in.normal) * facing;
    let bend = length(dpdx(geometric)) + length(dpdy(geometric));
    let step = length(dpdx(in.world_position)) + length(dpdy(in.world_position));
    out.curvature = bend / max(step, 1e-7);
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

// The diffuse and specular light one light leaves on skin, before its
// colour and strength (`Arrival::radiance`): `n` the full normal,
// `diffuse_n` and `red_n` the blurred ones, `row` the table's row (or
// below 0 for no spreading at all), `sharp` and `broad` the lobes'
// roughness.
fn skin_light(
    n: vec3<f32>,
    diffuse_n: vec3<f32>,
    red_n: vec3<f32>,
    geometric: vec3<f32>,
    v: vec3<f32>,
    arrival: Arrival,
    base_color: vec3<f32>,
    row: f32,
    sharp: f32,
    broad: f32,
) -> vec3<f32> {
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
        // ear's fold) never leaves a cyan fringe.
        let green = skin_table(dot(normalize(mix(diffuse_n, n, SKIN_DETAIL.g)), l), row);
        let blue = skin_table(dot(normalize(mix(diffuse_n, n, SKIN_DETAIL.b)), l), row);
        let red = max(skin_table(dot(red_n, l), row).r, green.r);
        spread = vec3<f32>(red, green.g, blue.b);
        shadow = pow(vec3<f32>(saturate(arrival.visible)),
            pow(SKIN_PENUMBRA, vec3<f32>(maps.shading.z)));
    }
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
        row = clamp(log2(radius_mm / 2.0), 0.0, f32(SKIN_ROWS - 1u));
    }

    let sharp = skin_widened(clamp(weather.roughness * SKIN_SHARP, 0.04, 1.0), footprint.variance);
    let broad = skin_widened(clamp(weather.roughness * SKIN_BROAD, 0.04, 1.0), footprint.variance);

    let sun = sun_arrival(world_position, geometric, pixel, solid);
    var lit = skin_light(n, diffuse_n, red_n, geometric, v, sun, base_color, row, sharp, broad)
        * sun.radiance;
    let lamps = tile_lamps(pixel);
    var slot = 0u;
    for (var i = 0u; i < lamps.count; i++) {
        let lamp = lamp_arrival(lamps, i, slot, world_position, geometric, pixel, solid);
        slot += lamp_slots(lamps, i);
        if lamp.visible > 0.0 {
            lit += skin_light(n, diffuse_n, red_n, geometric, v, lamp, base_color, row, sharp,
                broad) * lamp.radiance;
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
