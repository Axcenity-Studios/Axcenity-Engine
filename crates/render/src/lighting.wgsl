// Lighting shared by every scene shader: the frame's bindings (group 0),
// the sun and its shadows, point and spot lights, and the sky that lights
// and reflects. Compiled after `sky.wgsl`, `fog.wgsl` and `clouds.wgsl`. `shader.wgsl` (parts and models)
// and `terrain.wgsl` are each compiled after this file, and both light
// their surfaces with `light_surface`.
// Colors here are linear light, and can be brighter than 1 (stage 5 maps them down).

struct Frame {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    // Normalized direction toward the sun.
    sun_direction: vec4<f32>,
    // Sun color times intensity.
    sun_radiance: vec4<f32>,
    // Ambient color times intensity.
    ambient: vec4<f32>,
    // Turns world space to the sun's view of each shadow cascade (one layer
    // of the shadow map each, nearest the camera first).
    cascades: array<mat4x4<f32>, 4>,
    // The world size of one shadow map texel in each cascade.
    cascade_texels: vec4<f32>,
    // How many studs a cascade's depth range (0 to 1) covers.
    cascade_depths: vec4<f32>,
    // x: 1 if shadows are on, y: how wide the sun looks (penumbra width per
    // stud between caster and receiver), z: how many cascades, w: how far
    // from the camera shadows reach.
    shadow: vec4<f32>,
    // Turns clip space back to world space, to find the sky's direction.
    inv_view_proj: mat4x4<f32>,
    // The sky's colors. The top color's w is the environment intensity.
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    sky_ground: vec4<f32>,
    // x: how many lights there are, y: the size of a light tile in pixels,
    // z: how many tiles across.
    lights: vec4<u32>,
    // Turns world space to each shadow-casting spot light's view, by its
    // layer of `spot_shadow_map`.
    spot_shadows: array<mat4x4<f32>, 8>,
    // Turns world space to the camera's view space.
    view: mat4x4<f32>,
    // x: how strongly screen-space ambient occlusion darkens ambient light
    // (0 while it's off), zw: 1 over the scene's width and height in pixels.
    ambient_occlusion: vec4<f32>,
    // x: 1 when the environment maps light and are reflected (otherwise
    // the gradient is worked out per pixel), y: the reflection cubemap's
    // last mip level (the roughest), z: 1 when the sky is `sky_image`,
    // w: the sky image's width in pixels.
    environment: vec4<f32>,
    // The sky's diffuse light while the environment maps are on: spherical
    // harmonics (rgb each), already blurred over the hemisphere and divided
    // by pi, so `sky_irradiance` gives what a white matte surface returns.
    sky_light: array<vec4<f32>, 9>,
    // x: how many of `probes` are drawn, y: 1 while water reflects the
    // screen, z: 1 while it sees through its copy of the scene (water.wgsl).
    reflection_probes: vec4<u32>,
    // The reflection probes drawn, smallest box first.
    probes: array<Probe, MAX_PROBES>,
    // Turns world space to each face of each shadow-casting point light's
    // view, six per light (+X, -X, +Y, -Y, +Z, -Z), by its layer of
    // `point_shadow_map`.
    point_shadows: array<mat4x4<f32>, 24>,
    // x: 1 while contact shadows are drawn, y: how far toward a light they
    // look in studs, z: how thick what's on screen is taken to be.
    contact: vec4<f32>,
    // x: how many of `volumes` are drawn, y: 1 to leave the sun's disc out
    // of the sky (while baking light probes: the sun lights surfaces
    // directly, so a disc seen by a probe would count it twice).
    light_probes: vec4<u32>,
    // The light probe volumes drawn, smallest box first.
    volumes: array<Volume, MAX_VOLUMES>,
    // x: seconds (`DrawList::time`), for surface shaders that move. yzw:
    // the wind in studs a second (`DrawList::wind`), for swaying plants.
    clock: vec4<f32>,
    // x: how bright the stars are (0 by day), y: 1 when the sky image is an
    // atmosphere's, so the sun or moon and the stars are drawn over it, z:
    // how much of its light a creator's sky image gives (dimmer at night;
    // 1 otherwise), also scaling the environment maps made from it.
    night: vec4<f32>,
    // What fades into the distance (fog.wgsl).
    fog: Fog,
    // The layer of cloud in the sky (clouds.wgsl).
    clouds: Clouds,
    // x: how wet surfaces are, y: how much snow covers them, each from 0
    // to 1, z: snow layer thickness in studs, w: 1 while `weather_field` is
    // populated for this frame (`weathered`).
    weather: vec4<f32>,
    // xy: world X and Z of `weather_field` cell (0, 0), z: cell size in studs,
    // w: grid side length in cells.
    weather_bounds: vec4<f32>,
    // x: how hard rain falls now, y: how hard snow falls now, each from 0
    // to 1 (`rain_dimples`, `snow_surface`), z: how many of `weather_boxes`
    // are drawn, w: how far the snow's prints have filled in (`weather_print`).
    precipitation: vec4<f32>,
    // x: 1 while ray traced sun shadows are drawn, y: the scene's width in
    // pixels, z: how far they reach in studs (`ray_traced_sun`), w: where
    // the lamps' half of `ray_shadow_mask` starts, or 0 while lamps aren't
    // traced (`ray_traced_lamp`).
    ray_shadows: vec4<f32>,
    // Ray traced reflections (RL4, `traced_reflected`): x the roughness
    // from which they aren't traced (0 while off), y how many scene pixels
    // a traced one stands for each way, z the traced width in pixels, w
    // unused.
    ray_reflections: vec4<f32>,
    // x: which of 64 frames this is while TAA is on, 0 otherwise
    // (`shading_noise`); y: 1 while TAA is on (hair cards' hashed alpha,
    // hair.wgsl).
    noise: vec4<f32>,
    // The boxes that set the weather inside them by hand (WeatherVolumes),
    // in the order they apply (`weather_sample`).
    weather_boxes: array<WeatherBox, MAX_WEATHER_BOXES>,
    // The snow's print map, kept in `weather_field` after the field's cells:
    // xy: world X and Z of its cell (0, 0), z: its cells to a stud (0 while
    // there is none), w: its side in cells (`weather_print`).
    weather_prints: vec4<f32>,
}

// The most weather boxes drawn at once. Must match `MAX_WEATHER_BOXES` in
// draw_list.rs.
const MAX_WEATHER_BOXES: u32 = 8u;

// A box that sets the weather inside it. Must match `WeatherBoxRaw` in
// renderer/types.rs.
struct WeatherBox {
    // The rows of the world to the box's own space (studs, its middle at 0).
    x: vec4<f32>,
    y: vec4<f32>,
    z: vec4<f32>,
    // xyz: half its size, w: the studs over which it fades across its faces.
    extent: vec4<f32>,
    // x: the share of the world's wetness inside it, y: of its snow, each
    // from 0 to 2.
    values: vec4<f32>,
}

// The most light probe volumes drawn at once. Must match `MAX_VOLUMES` in
// renderer/light_probes.rs.
const MAX_VOLUMES: u32 = 8u;

// A box of the world lit by its grid of baked light probes in place of the
// sky's diffuse light. Must match `VolumeRaw` in renderer/light_probes.rs.
struct Volume {
    // Turns world space to the box's own, one row each (as `Probe.to_box`).
    to_box: array<vec4<f32>, 3>,
    // xyz: half the box's size, w: how far inside its faces it fades in.
    extent: vec4<f32>,
    // xyz: how many probes stand along the box's own axes, each in the
    // middle of its cell, w: its first probe in `light_probes`.
    grid: vec4<u32>,
    // xyz: how much of its probes' sky light to add, per channel (the sky
    // drawn now over the one it was baked with, minus one), w: 1 to add it
    // (roadmap R18.3).
    sky: vec4<f32>,
    // x: where its first probe's sky light is in `light_probes`.
    sky_first: vec4<u32>,
}

// The most reflection probes drawn at once. Must match `MAX_PROBES` in
// renderer/probes.rs.
const MAX_PROBES: u32 = 8u;

// A box of the world that reflects its bake in place of the sky. Must
// match `ProbeRaw` in renderer/probes.rs.
struct Probe {
    // Turns world space to the box's own (its middle at 0, its sides along
    // the axes), one row each. Each row's xyz is also one of the box's axes
    // in world space.
    to_box: array<vec4<f32>, 3>,
    // xyz: half the box's size, w: how far inside its faces it fades in.
    extent: vec4<f32>,
    // x: its layer of `probe_maps`.
    layer: vec4<u32>,
}

// A point, spot or rect light. Must match `LightRaw` in renderer.rs.
struct Light {
    // xyz: position, w: range.
    position_range: vec4<f32>,
    // rgb: color times brightness, w: cosine of the cone's edge, or -2 for
    // a point light, -3 for a rect light.
    radiance_cone: vec4<f32>,
    // xyz: the direction a spot light points (a rect light faces), w:
    // cosine where its edge starts to fade.
    direction_inner: vec4<f32>,
    // x: a spot light's layer of `spot_shadow_map`, or a point light's place
    // in `point_shadow_map` (its six layers start at six times it), or -1
    // if it casts no shadows, y: the width of one shadow texel per stud
    // from the light, z: 0 if it casts no shadows at all, otherwise 1 plus
    // how far from it ray traced shadows start.
    shadow: vec4<f32>,
    // A rect light's right, as long as half its width, and w: half its
    // height.
    rect: vec4<f32>,
}

// The most lights one tile of the screen lists. Must match `LIGHTS_PER_TILE`
// in renderer.rs.
const LIGHTS_PER_TILE: u32 = 64u;

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var shadow_map: texture_depth_2d_array;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;
@group(0) @binding(3) var<storage, read> lights: array<Light>;
// For each screen tile, `LIGHTS_PER_TILE + 1` numbers: how many lights
// reach it, then their indexes in `lights`.
@group(0) @binding(4) var<storage, read> tile_lights: array<u32>;
// One layer per shadow-casting spot light: its depth as the light sees it.
// (Binding 5 is the shadow pass's own, in shader.wgsl.)
@group(0) @binding(6) var spot_shadow_map: texture_depth_2d_array;
// Screen-space ambient occlusion at half the scene's size (ssao.wgsl): 1
// in the open, toward 0 in corners. White while it's off.
@group(0) @binding(7) var screen_occlusion_map: texture_2d<f32>;
@group(0) @binding(8) var screen_occlusion_sampler: sampler;
// (Binding 9 is the frame's joints, in shader.wgsl.)
// The sky as surfaces reflect it, blurrier at each mip level: level
// `roughness * frame.environment.y` for a surface of that roughness.
// A blank 1×1 cube while the environment maps are off.
@group(0) @binding(10) var environment_map: texture_cube<f32>;
@group(0) @binding(11) var environment_sampler: sampler;
// The sky image (see `equirect_uv`), or a blank 1×1 texture without one.
@group(0) @binding(12) var sky_image: texture_2d<f32>;
// Each reflection probe's bake, blurred like `environment_map`, one cube
// per probe (its `layer`). A blank 1×1 cube while none are drawn.
@group(0) @binding(13) var probe_maps: texture_cube_array<f32>;
// Six layers per shadow-casting point light, one for each way along each
// axis: its depth as the light sees it that way.
@group(0) @binding(14) var point_shadow_map: texture_depth_2d_array;
// The prepass's depth (solid items only), for contact shadows. A 1×1
// stand-in while they're off.
@group(0) @binding(15) var prepass_depth: texture_depth_2d;
// Every light probe of the volumes drawn, four entries each: its light as
// spherical harmonics (constant, then along y, z and x; rgb), blurred over
// the hemisphere and divided by pi, like `Frame.sky_light`. Probes run x
// first, then y, then z. One blank probe while there are none.
@group(0) @binding(16) var<storage, read> light_probes: array<vec4<f32>>;
// Ray traced sun shadows (ray_shadows.wgsl), a u32 per scene pixel: the
// surface's distance from the camera in the top 24 bits, how much sunlight
// reaches it in the low 8. One blank entry while they're off. Then, from
// `frame.ray_shadows.w` while lamps are traced, a u32 per pixel for its
// lamps: 4 bits each (0 in shadow, 15 lit), in the order its tile lists
// the lamps that cast shadows, at most `TRACED_LAMPS`.
@group(0) @binding(17) var<storage, read> ray_shadow_mask: array<u32>;
// Ray traced reflections (ray_reflections.wgsl), two u32s a traced pixel:
// rg, then b and how much it counts, as half floats. One blank entry while
// they're off.
@group(0) @binding(18) var<storage, read> ray_reflections: array<vec2<u32>>;
// World-space 2D weather field around the camera: per cell (z * size + x),
// x = highest sky-occluding roof Y (-1e9 if open sky), y = manual surface
// mask (-1 interior, 0 auto, +1 exterior), z = snow footprint depth (0..1,
// the average over the cell), w = how high the snow banks there. After the
// field's cells, while `frame.weather_prints.z` isn't 0, the snow's print
// map: four of its cells to an entry, row by row (`print_cell`).
@group(0) @binding(19) var<storage, read> weather_field: array<vec4<f32>>;

// The most lamps a pixel has ray traced shadows for. Must match
// `TRACED_LAMPS` in ray_shadows.wgsl.
const TRACED_LAMPS: u32 = 8u;

// Points spread evenly over a disc of radius 1 (a Poisson disc), for
// sampling the shadow map around a point. `private` so it can be indexed
// in a loop.
var<private> POISSON: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>(-0.94201624, -0.39906216), vec2<f32>(0.94558609, -0.76890725),
    vec2<f32>(-0.09418410, -0.92938870), vec2<f32>(0.34495938, 0.29387760),
    vec2<f32>(-0.91588581, 0.45771432), vec2<f32>(-0.81544232, -0.87912464),
    vec2<f32>(-0.38277543, 0.27676845), vec2<f32>(0.97484398, 0.75648379),
    vec2<f32>(0.44323325, -0.97511554), vec2<f32>(0.53742981, -0.47373420),
    vec2<f32>(-0.26496911, -0.41893023), vec2<f32>(0.79197514, 0.19090188),
    vec2<f32>(-0.24188840, 0.99706507), vec2<f32>(-0.81409955, 0.91437590),
    vec2<f32>(0.19984126, 0.78641367), vec2<f32>(0.14383161, -0.14100790),
);
// The widest a shadow's soft edge is blurred, in shadow map texels.
const MAX_FILTER_TEXELS: f32 = 32.0;
// How far from a receiver blockers are looked for, in studs.
const BLOCKER_SEARCH_STUDS: f32 = 30.0;

// A number from 0 to 1 that changes from pixel to pixel without a visible
// pattern (interleaved gradient noise). It turns the sample pattern per
// pixel, so blurred edges show fine grain instead of banding.
fn pixel_noise(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

// `pixel_noise` for the shadow filters and contact shadows, moved on each
// frame while TAA is on (`frame.noise.x`, 0 otherwise), so TAA averages
// their per-pixel turn into a smooth edge instead of keeping one fixed
// pattern, which shows as stipple and diagonal hatching. 5.588238 is
// Jimenez's step for interleaved gradient noise over frames.
fn shading_noise(pixel: vec2<f32>) -> f32 {
    return pixel_noise(pixel + 5.588238 * frame.noise.x);
}

// Sunlight reaching `world_position` as seen by cascade `index`: x is 1 in
// the open and 0 in shadow, y how far toward the cascade's edge the point
// is (0 inside, rising to 1 at the edge), or -1 if the cascade doesn't
// cover the point.
//
// Soft shadows the way real ones are (percentage-closer soft shadows):
// first the map is searched for what blocks the sun, and the further the
// blocker is from the surface, the wider the blur. So a shadow is crisp
// where a part touches the ground and softens further away.
fn cascade_visibility(
    index: u32,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    rotation: mat2x2<f32>,
    slope: f32,
) -> vec2<f32> {
    let texel = frame.cascade_texels[index];
    // Pushed off the surface so it doesn't shadow itself.
    let shifted = world_position + normal * (texel * 1.5);
    let light = frame.cascades[index] * vec4<f32>(shifted, 1.0);
    let uv = vec2<f32>(light.x * 0.5 + 0.5, 0.5 - light.y * 0.5);
    let receiver = light.z;
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || receiver > 1.0 || receiver < 0.0 {
        return vec2<f32>(1.0, -1.0);
    }
    let edge = smoothstep(0.8, 1.0, max(abs(light.x), abs(light.y)));
    let size = f32(textureDimensions(shadow_map).x);
    let depth_range = frame.cascade_depths[index];
    let sun_size = frame.shadow.y;
    let layer = i32(index);
    // One texel of depth, as the map stores it.
    let texel_depth = texel / depth_range;

    // 1. Blockers: how far, on average, the things shading this point are.
    let search = clamp(sun_size * BLOCKER_SEARCH_STUDS / texel, 1.0, MAX_FILTER_TEXELS);
    let search_bias = texel_depth * (1.0 + slope);
    let last = i32(size) - 1;
    var blockers = 0.0;
    var blocker_depth = 0.0;
    for (var k = 0; k < 16; k++) {
        let offset = rotation * POISSON[k] * search;
        let coord = clamp(vec2<i32>(uv * size + offset), vec2<i32>(0), vec2<i32>(last));
        let depth = textureLoad(shadow_map, coord, layer, 0);
        if depth < receiver - search_bias {
            blockers += 1.0;
            blocker_depth += depth;
        }
    }
    if blockers == 0.0 {
        return vec2<f32>(1.0, edge);
    }
    blocker_depth /= blockers;

    // 2. The soft edge's width, from how far light travels past the blocker.
    let penumbra = (receiver - blocker_depth) * depth_range * sun_size;
    let radius = clamp(penumbra / texel, 1.0, MAX_FILTER_TEXELS);

    // 3. Averages comparisons over that width. Each comparison is itself
    // filtered between four texels, so even radius 1 has smooth edges.
    let bias = texel_depth * (1.0 + 0.5 * radius * slope);
    var lit = 0.0;
    for (var k = 0; k < 16; k++) {
        let offset = rotation * POISSON[k] * (radius / size);
        lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + offset, layer, receiver - bias);
    }
    return vec2<f32>(lit / 16.0, edge);
}

// Ray traced sunlight at `pixel` (`frame.ray_shadows`), if the mask was
// traced from this very surface: from 0 in shadow to 1 in the open, or -1
// where it has nothing for it (see-through surfaces, the edge of something
// in front, past the reach, or while ray traced shadows are off).
fn ray_traced_sun(world_position: vec3<f32>, pixel: vec2<f32>) -> f32 {
    if frame.ray_shadows.x == 0.0 {
        return -1.0;
    }
    let at = vec2<u32>(pixel);
    let index = at.y * u32(frame.ray_shadows.y) + at.x;
    if index >= arrayLength(&ray_shadow_mask) {
        return -1.0;
    }
    let packed = ray_shadow_mask[index];
    let traced = bitcast<f32>(packed & 0xFFFFFF00u);
    let here = distance(frame.camera_position.xyz, world_position);
    if !(traced > 0.0) || abs(traced - here) > 0.02 * here + 0.05 {
        return -1.0;
    }
    return f32(packed & 255u) / 255.0;
}

// How much of the lamp in `slot` (of the pixel's lamps that cast shadows,
// in its tile's order) reaches `world_position`, ray traced: from 0 to 1,
// or -1 where the mask has nothing for it (as `ray_traced_sun`, or a lamp
// past the first `TRACED_LAMPS`).
fn ray_traced_lamp(slot: u32, world_position: vec3<f32>, pixel: vec2<f32>) -> f32 {
    let first = u32(frame.ray_shadows.w);
    if first == 0u || slot >= TRACED_LAMPS || ray_traced_sun(world_position, pixel) < 0.0 {
        return -1.0;
    }
    let at = vec2<u32>(pixel);
    let index = first + at.y * u32(frame.ray_shadows.y) + at.x;
    if index >= arrayLength(&ray_shadow_mask) {
        return -1.0;
    }
    return f32((ray_shadow_mask[index] >> (slot * 4u)) & 15u) / 15.0;
}

// How much sunlight reaches `world_position`: 1 in the open, 0 in shadow.
// Ray traced where `ray_traced_sun` has it, fading into the shadow maps
// toward its reach; otherwise the shadow maps (`cascade_sun_visibility`).
fn sun_visibility(world_position: vec3<f32>, normal: vec3<f32>, pixel: vec2<f32>) -> f32 {
    let traced = ray_traced_sun(world_position, pixel);
    if traced < 0.0 {
        return cascade_sun_visibility(world_position, normal, pixel);
    }
    let reach = frame.ray_shadows.z;
    let away = distance(frame.camera_position.xyz, world_position);
    let blend = smoothstep(reach * 0.85, reach, away);
    if blend <= 0.0 {
        return traced;
    }
    return mix(traced, cascade_sun_visibility(world_position, normal, pixel), blend);
}

// How much sunlight reaches `world_position` by the shadow maps: 1 in the
// open, 0 in shadow. `normal` is the surface's own (unmapped) normal, used to push the point
// off the surface so it doesn't shadow itself, and `pixel` where it is on
// screen. The nearest cascade covering the point is used, blended into the
// next one near its edge so the change in sharpness doesn't show.
fn cascade_sun_visibility(world_position: vec3<f32>, normal: vec3<f32>, pixel: vec2<f32>) -> f32 {
    if frame.shadow.x == 0.0 {
        return 1.0;
    }
    let angle = shading_noise(pixel) * 6.2831853;
    let rotation = mat2x2<f32>(cos(angle), sin(angle), -sin(angle), cos(angle));
    // How steeply the surface faces away from the sun (the tangent of the
    // angle), for depth bias: tilted surfaces need more.
    let n_dot_l = clamp(dot(normal, frame.sun_direction.xyz), 0.05, 1.0);
    let slope = min(sqrt(1.0 - n_dot_l * n_dot_l) / n_dot_l, 3.0);

    let count = u32(frame.shadow.z);
    var lit = 1.0;
    for (var i = 0u; i < count; i++) {
        let here = cascade_visibility(i, world_position, normal, rotation, slope);
        if here.y < 0.0 {
            continue;
        }
        lit = here.x;
        if here.y > 0.0 && i + 1u < count {
            let next = cascade_visibility(i + 1u, world_position, normal, rotation, slope);
            if next.y >= 0.0 {
                lit = mix(here.x, next.x, here.y);
            }
        }
        break;
    }
    // Shadows fade out toward their distance limit instead of ending sharply.
    let reach = frame.shadow.w;
    let fade = 1.0 - smoothstep(reach * 0.8, reach, distance(frame.camera_position.xyz, world_position));
    return mix(1.0, lit, fade);
}

// The width of `soft_sun_visibility`'s filter, in texels of the shadow map.
const SOFT_SUN_FILTER_TEXELS: f32 = 3.0;

// `sun_visibility`, cheaply: four filtered comparisons in the nearest
// cascade covering the point, with no search for what blocks it. For light
// that has already scattered on its way through something (a leaf), whose
// shadow is soft whatever casts it. Costs a tenth of the full lookup.
fn soft_sun_visibility(world_position: vec3<f32>, normal: vec3<f32>, pixel: vec2<f32>) -> f32 {
    if frame.shadow.x == 0.0 {
        return 1.0;
    }
    let angle = shading_noise(pixel) * 6.2831853;
    let rotation = mat2x2<f32>(cos(angle), sin(angle), -sin(angle), cos(angle));
    let spread = SOFT_SUN_FILTER_TEXELS / f32(textureDimensions(shadow_map).x);
    let count = u32(frame.shadow.z);
    var lit = 1.0;
    for (var i = 0u; i < count; i++) {
        let texel = frame.cascade_texels[i];
        let light = frame.cascades[i] * vec4<f32>(world_position + normal * (texel * 1.5), 1.0);
        let uv = vec2<f32>(light.x * 0.5 + 0.5, 0.5 - light.y * 0.5);
        let receiver = light.z;
        if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || receiver > 1.0 || receiver < 0.0 {
            continue;
        }
        let bias = 2.0 * texel / frame.cascade_depths[i];
        var sum = 0.0;
        for (var k = 0; k < 4; k++) {
            let offset = rotation * POISSON[k * 4] * spread;
            sum += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + offset, i32(i),
                receiver - bias);
        }
        lit = sum / 4.0;
        break;
    }
    let reach = frame.shadow.w;
    let fade = 1.0 - smoothstep(reach * 0.8, reach, distance(frame.camera_position.xyz, world_position));
    return mix(1.0, lit, fade);
}

// How much of a lamp's light reaches `point`, by layer `layer` of `map`,
// drawn through `view`: 1 in the open, 0 in its shadow, and 1 outside the
// view. Four comparisons around the point, each filtered between four
// texels like the sun's, are averaged.
fn lamp_shadow(
    map: texture_depth_2d_array,
    view: mat4x4<f32>,
    layer: i32,
    point: vec3<f32>,
    rotation: mat2x2<f32>,
) -> f32 {
    let clip = view * vec4<f32>(point, 1.0);
    if clip.w <= 0.0 {
        return 1.0;
    }
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || ndc.z > 1.0 || ndc.z < 0.0 {
        return 1.0;
    }
    let size = f32(textureDimensions(map).x);
    var lit = 0.0;
    for (var k = 0; k < 4; k++) {
        let corner = vec2<f32>(f32(k & 1), f32(k >> 1u)) * 2.0 - 1.0;
        let offset = rotation * corner * (0.75 / size);
        lit += textureSampleCompareLevel(map, shadow_sampler, uv + offset, layer, ndc.z);
    }
    return lit / 4.0;
}

// How much of a spot or point light reaches `world_position`: 1 in the
// open, 0 in its shadow. `to_light` is the way to the light, `normal` the
// surface's own (unmapped) normal. The point is pushed off the surface and
// toward the light by amounts that grow with its distance from the light,
// as the map's texels do, so it doesn't shadow itself. A point light's is
// looked up in the face of its six that looks the way the point is: the
// one along its longest axis from the light.
fn lamp_visibility(
    light: Light,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    to_light: vec3<f32>,
    rotation: mat2x2<f32>,
) -> f32 {
    let index = i32(light.shadow.x);
    if index < 0 {
        return 1.0;
    }
    let texel = length(to_light) * light.shadow.y;
    let shifted = world_position + normal * (texel * 1.5) + normalize(to_light) * (texel * 2.0);
    if light.radiance_cone.w > -1.5 {
        return lamp_shadow(spot_shadow_map, frame.spot_shadows[index], index, shifted, rotation);
    }
    let away = shifted - light.position_range.xyz;
    let size = abs(away);
    var face = select(5, 4, away.z > 0.0);
    if size.x >= size.y && size.x >= size.z {
        face = select(1, 0, away.x > 0.0);
    } else if size.y >= size.z {
        face = select(3, 2, away.y > 0.0);
    }
    let layer = index * 6 + face;
    return lamp_shadow(point_shadow_map, frame.point_shadows[layer], layer, shifted, rotation);
}

// How many steps contact shadows take toward a light.
const CONTACT_STEPS: u32 = 12u;

// How much light from the way `toward` (normalized) reaches the solid
// surface at `world_position`, by contact shadows: a short march toward
// the light through the prepass's depth, for the small gaps the shadow
// maps are too coarse to see. A step that's behind what's on screen there
// (by less than its thickness) is blocked. Darkest where the blocker is
// nearest, fading to 1 at `reach` studs. 1 while they're off.
fn contact_visibility(
    world_position: vec3<f32>,
    normal: vec3<f32>,
    toward: vec3<f32>,
    reach: f32,
    pixel: vec2<f32>,
) -> f32 {
    if frame.contact.x == 0.0 || reach <= 0.0 || dot(normal, toward) <= 0.0 {
        return 1.0;
    }
    let size = vec2<i32>(textureDimensions(prepass_depth));
    let camera = frame.camera_position.xyz;
    // Off the surface, by more the further away it is as depth gets coarser.
    let bias = 0.01 + distance(camera, world_position) * 0.002;
    let start = world_position + normal * bias;
    let step = toward * (reach / f32(CONTACT_STEPS));
    let jitter = shading_noise(pixel);
    for (var i = 0u; i < CONTACT_STEPS; i++) {
        let along = f32(i) + jitter;
        let p = start + step * along;
        let clip = frame.view_proj * vec4<f32>(p, 1.0);
        if clip.w <= 0.0 {
            break;
        }
        let ndc = clip.xyz / clip.w;
        if any(abs(ndc.xy) >= vec2<f32>(1.0)) {
            break;
        }
        let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        let texel = min(vec2<i32>(uv * vec2<f32>(size)), size - vec2<i32>(1));
        let depth = textureLoad(prepass_depth, texel, 0);
        if depth >= 1.0 {
            continue;
        }
        // What's on screen there, back in world space.
        let seen = frame.inv_view_proj * vec4<f32>(ndc.xy, depth, 1.0);
        let behind = distance(camera, p) - distance(camera, seen.xyz / seen.w);
        if behind > bias && behind < frame.contact.z {
            return along / f32(CONTACT_STEPS);
        }
    }
    return 1.0;
}

// How much ambient and sky light reaches the solid surface drawn at
// `pixel`, by screen-space ambient occlusion: 1 in the open, less in
// corners, raised to the world's strength. 1 while it's off. See-through
// surfaces aren't in the prepass, so they mustn't use this.
fn screen_occlusion(pixel: vec2<f32>) -> f32 {
    let uv = pixel * frame.ambient_occlusion.zw;
    let open = textureSampleLevel(screen_occlusion_map, screen_occlusion_sampler, uv, 0.0).r;
    return exp2(frame.ambient_occlusion.x * log2(max(open, 0.001)));
}

// What the depth and normal prepass writes for a surface facing `normal`
// (world space): that direction in view space, and 1 (a character's items
// write 0.5 there, for projected decals; the sky stays 0).
fn prepass_normal(normal: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(normalize((frame.view * vec4<f32>(normal, 0.0)).xyz), 1.0);
}

// How many microfacets point along `h` (GGX / Trowbridge-Reitz).
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

// How much microfacets shadow each other (Smith with Schlick-GGX).
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = r * r / 8.0;
    let g_v = n_dot_v / (n_dot_v * (1.0 - k) + k);
    let g_l = n_dot_l / (n_dot_l * (1.0 - k) + k);
    return g_v * g_l;
}

// How much light reflects instead of entering the surface, by angle.
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

// The sky's color looking along `dir`: ground below the horizon, then a
// gradient up to the top of the sky.
fn sky_gradient(dir: vec3<f32>) -> vec3<f32> {
    return gradient_sky(dir, frame.sky_top.rgb, frame.sky_horizon.rgb, frame.sky_ground.rgb);
}

// What's drawn as the sky looking along `dir`, where one pixel covers
// `spread` radians of it: the sky image, or the gradient, plus the sun (or
// the moon at night: whichever lights the scene) and the stars. The disc is
// far brighter than 1, so it blooms. A creator's sky image has its own sun
// (or none), so nothing is drawn over it; an atmosphere's has none.
fn sky_radiance(dir: vec3<f32>, spread: f32) -> vec3<f32> {
    var sky: vec3<f32>;
    if frame.environment.z > 0.5 {
        // The image's level whose pixels are about the size of the screen's,
        // so its detail doesn't shimmer. Toward straight up and down its
        // columns crowd together, so a blurrier level is read there (at
        // most 4 levels blurrier, so the very top isn't the whole image's
        // average).
        let across = max(sqrt(max(1.0 - dir.y * dir.y, 0.0)), 0.0625);
        let level = log2(max(spread * frame.environment.w / (2.0 * PI) / across, 1.0));
        sky = textureSampleLevel(sky_image, environment_sampler, equirect_uv(dir), level).rgb
            * frame.night.z;
        if frame.night.y < 0.5 {
            return sky;
        }
    } else {
        sky = sky_gradient(dir);
    }
    if frame.light_probes.y != 0u {
        return sky;
    }
    let toward_sun = dot(dir, frame.sun_direction.xyz);
    // Hidden by the ground once it sets.
    let above = smoothstep(-0.004, 0.004, dir.y);
    let disc = smoothstep(0.99965, 0.99985, toward_sun) * above;
    let glow = pow(max(toward_sun, 0.0), 200.0) * 0.4;
    return sky + frame.sun_radiance.rgb * (glow + disc * 8.0) + stars(dir, spread) * frame.night.x;
}

// A number from 0 to 1 for each whole-numbered point (a hash).
fn cell_hash(cell: vec3<f32>) -> vec4<f32> {
    var p = fract(cell * vec3<f32>(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yxz + 33.33);
    let q = fract((p.xxy + p.yzz) * p.zyx);
    return vec4<f32>(q, fract(dot(q, vec3<f32>(17.13, 31.71, 11.37))));
}

// The stars' light looking along `dir`: a few of the cells of a grid around
// the viewer hold a star, at a point of their own, about a pixel wide
// (`spread`) and of its own brightness, fading out toward the horizon.
fn stars(dir: vec3<f32>, spread: f32) -> vec3<f32> {
    if dir.y <= 0.0 {
        return vec3<f32>(0.0);
    }
    let scaled = dir * 150.0;
    let cell = floor(scaled);
    let hash = cell_hash(cell);
    if hash.w < 0.993 {
        return vec3<f32>(0.0);
    }
    let star = normalize(cell + 0.15 + hash.xyz * 0.7);
    let width = max(spread, 0.0004) * 1.2;
    let near = 1.0 - smoothstep(0.0, width, acos(clamp(dot(dir, star), -1.0, 1.0)));
    // Most are faint; a few are bright.
    let brightness = pow((hash.w - 0.993) / 0.007, 3.0);
    let tint = mix(vec3<f32>(0.75, 0.85, 1.0), vec3<f32>(1.0, 0.9, 0.75), hash.x);
    return tint * near * (0.15 + 1.85 * brightness) * smoothstep(0.0, 0.15, dir.y);
}

// Light from the whole sky that reaches a surface facing `n`: a smooth
// blend from the ground's color to the sky's average.
fn environment_diffuse(n: vec3<f32>) -> vec3<f32> {
    let sky = mix(frame.sky_horizon.rgb, frame.sky_top.rgb, 0.5);
    return mix(frame.sky_ground.rgb, sky, smoothstep(-0.7, 0.7, n.y));
}

// What a surface reflects looking along `dir`: the sharp sky when smooth,
// blurring toward its smooth average as roughness grows.
fn environment_specular(dir: vec3<f32>, roughness: f32) -> vec3<f32> {
    return mix(sky_gradient(dir), environment_diffuse(dir), smoothstep(0.1, 0.9, roughness));
}

// The sky's light reaching a surface facing `n`, from its spherical
// harmonics. Never below 0, which a few coefficients can ring under.
fn sky_irradiance(n: vec3<f32>) -> vec3<f32> {
    let light = frame.sky_light[0].rgb * 0.282095
        + frame.sky_light[1].rgb * (0.488603 * n.y)
        + frame.sky_light[2].rgb * (0.488603 * n.z)
        + frame.sky_light[3].rgb * (0.488603 * n.x)
        + frame.sky_light[4].rgb * (1.092548 * n.x * n.y)
        + frame.sky_light[5].rgb * (1.092548 * n.y * n.z)
        + frame.sky_light[6].rgb * (0.315392 * (3.0 * n.z * n.z - 1.0))
        + frame.sky_light[7].rgb * (1.092548 * n.x * n.z)
        + frame.sky_light[8].rgb * (0.546274 * (n.x * n.x - n.y * n.y));
    return max(light, vec3<f32>(0.0));
}

// The sky's light as it arrives under the clouds. The environment maps and
// the harmonics hold the clear sky (clouds aren't in them), so under an
// overcast everything was still lit, and every puddle mirrored, a clear
// blue sky the clouds hid. From broken cloud on (`OVERCAST_FROM`: none at
// the first cover, all of it from the second) the sky's light turns the
// grey of its own brightness, tinted half-way to the clouds' color, and a
// thick layer keeps `OVERCAST_LIGHT` of it. A thin veil (little `Density`)
// changes nothing.
const OVERCAST_FROM: vec2<f32> = vec2<f32>(0.4, 0.9);
const OVERCAST_LIGHT: f32 = 0.8;
fn under_clouds(light: vec3<f32>) -> vec3<f32> {
    let clouds = frame.clouds;
    if clouds.color.w == 0.0 {
        return light;
    }
    // How much of the layer's light is scattered on its way down.
    let thick = 1.0 - exp(-clouds.shape.y * (clouds.shape.w - clouds.shape.z) * 0.25);
    let share = smoothstep(OVERCAST_FROM.x, OVERCAST_FROM.y, clouds.shape.x) * thick;
    if share <= 0.0 {
        return light;
    }
    let grey = dot(light, vec3<f32>(0.2126, 0.7152, 0.0722));
    let overcast = grey * mix(vec3<f32>(1.0), clouds.color.rgb, 0.5) * mix(1.0, OVERCAST_LIGHT, thick);
    return mix(light, overcast, share);
}

// The sky's diffuse light on a surface facing `n`: from the environment
// maps' harmonics while they're on, otherwise from the gradient, as it
// arrives under the clouds.
fn sky_diffuse(n: vec3<f32>) -> vec3<f32> {
    if frame.environment.x > 0.5 {
        return under_clouds(sky_irradiance(n));
    }
    return under_clouds(environment_diffuse(n));
}

// What a surface of `roughness` reflects looking along `dir`: the
// reflection cubemap's level for that roughness while the maps are on,
// otherwise the gradient.
fn sky_specular(dir: vec3<f32>, roughness: f32) -> vec3<f32> {
    if frame.environment.x > 0.5 {
        let level = roughness * frame.environment.y;
        return under_clouds(textureSampleLevel(environment_map, environment_sampler, dir, level).rgb
            * frame.night.z);
    }
    return under_clouds(environment_specular(dir, roughness));
}

// The diffuse light reaching a surface at `position` facing `n` from all
// around: each light probe volume whose box it's inside, from the eight
// probes around it (blended by how near each is), fading in over the
// volume's blend distance from its faces. Smaller boxes come first, and
// the bigger ones and then the sky (times the environment intensity) fill
// what they leave. A probe marked unusable (the w of its first coefficient
// over a half: 1 when baked inside a solid, 2 when ray traced GI found it
// inside something or hasn't traced it yet; ray_gi.wgsl) is left out and
// the others share its part; where all eight are, the volume gives way.
fn surrounding_diffuse(position: vec3<f32>, n: vec3<f32>, environment: f32) -> vec3<f32> {
    var color = vec3<f32>(0.0);
    var left = 1.0;
    let count = min(frame.light_probes.x, MAX_VOLUMES);
    for (var i = 0u; i < count; i++) {
        let volume = frame.volumes[i];
        let p = vec4<f32>(position, 1.0);
        let local = vec3<f32>(dot(volume.to_box[0], p), dot(volume.to_box[1], p), dot(volume.to_box[2], p));
        let half_size = volume.extent.xyz;
        let inside = min(min(half_size.x - abs(local.x), half_size.y - abs(local.y)), half_size.z - abs(local.z));
        if inside <= 0.0 {
            continue;
        }
        // Where it is in the grid, in probes: between the two probes nearest
        // it along each axis, held at the outermost ones near the faces.
        let counts = vec3<f32>(volume.grid.xyz);
        let cell = clamp((local / (2.0 * half_size) + 0.5) * counts - 0.5, vec3<f32>(0.0), counts - 1.0);
        let low = vec3<u32>(floor(cell));
        let high = min(low + 1u, volume.grid.xyz - 1u);
        let t = cell - floor(cell);
        var c0 = vec3<f32>(0.0);
        var c1 = vec3<f32>(0.0);
        var c2 = vec3<f32>(0.0);
        var c3 = vec3<f32>(0.0);
        // A probe left out (its first w over a half: 1 for one baked
        // inside a solid, R18.2; 2 for one inside something by ray traced
        // GI's rays, or not traced yet) gives its share to the others.
        var valid = 0.0;
        for (var corner = 0u; corner < 8u; corner++) {
            let far = vec3<bool>((corner & 1u) != 0u, (corner & 2u) != 0u, (corner & 4u) != 0u);
            let at = select(low, high, far);
            let shares = select(1.0 - t, t, far);
            let index = at.x + volume.grid.x * (at.y + volume.grid.y * at.z);
            let probe = 4u * (volume.grid.w + index);
            let first = light_probes[probe];
            let second = light_probes[probe + 1u];
            let share = shares.x * shares.y * shares.z * select(1.0, 0.0, first.w > 0.5);
            valid += share;
            c0 += share * first.rgb;
            c1 += share * second.rgb;
            c2 += share * light_probes[probe + 2u].rgb;
            c3 += share * light_probes[probe + 3u].rgb;
            // The sky's share follows the sky drawn now, unless real-time
            // or ray traced GI traced the probe again (its second w is 1).
            if volume.sky.w > 0.5 && second.w < 0.5 {
                let sky = 4u * (volume.sky_first.x + index);
                let scaled = share * volume.sky.xyz;
                c0 += scaled * light_probes[sky].rgb;
                c1 += scaled * light_probes[sky + 1u].rgb;
                c2 += scaled * light_probes[sky + 2u].rgb;
                c3 += scaled * light_probes[sky + 3u].rgb;
            }
        }
        // All eight left out: what's around the volume lights it.
        if valid <= 0.0001 {
            continue;
        }
        let weight = left * smoothstep(0.0, 1.0, inside / max(volume.extent.w, 0.001));
        let light = (c0 * 0.282095 + (c1 * n.y + c2 * n.z + c3 * n.x) * 0.488603) / valid;
        color += weight * max(light, vec3<f32>(0.0));
        left -= weight;
        if left <= 0.001 {
            break;
        }
    }
    return color + left * environment * sky_diffuse(n);
}

// What a surface at `position` of `roughness` reflects looking along
// `dir`: each reflection probe whose box it's inside, read where the
// reflection leaves the box (so near walls line up with the real ones:
// parallax-corrected), fading in over the probe's blend distance from its
// faces. Smaller boxes come first and the bigger ones and then the sky
// (times the environment intensity) fill what they leave.
fn reflected(position: vec3<f32>, dir: vec3<f32>, roughness: f32, environment: f32) -> vec3<f32> {
    var color = vec3<f32>(0.0);
    var left = 1.0;
    let count = min(frame.reflection_probes.x, MAX_PROBES);
    for (var i = 0u; i < count; i++) {
        let probe = frame.probes[i];
        let p = vec4<f32>(position, 1.0);
        let local = vec3<f32>(dot(probe.to_box[0], p), dot(probe.to_box[1], p), dot(probe.to_box[2], p));
        let half_size = probe.extent.xyz;
        let inside = min(min(half_size.x - abs(local.x), half_size.y - abs(local.y)), half_size.z - abs(local.z));
        if inside <= 0.0 {
            continue;
        }
        let weight = left * smoothstep(0.0, 1.0, inside / max(probe.extent.w, 0.001));
        // The reflection in the box's own space, and how far along it each
        // pair of faces is: the nearer face it's heading for leaves first.
        let d = vec3<f32>(
            dot(probe.to_box[0].xyz, dir),
            dot(probe.to_box[1].xyz, dir),
            dot(probe.to_box[2].xyz, dir),
        );
        let safe = select(d, vec3<f32>(1e-6), abs(d) < vec3<f32>(1e-6));
        let exits = (sign(safe) * half_size - local) / safe;
        let hit = local + d * min(min(exits.x, exits.y), exits.z);
        // From the box's middle, where it was baked, back in world space.
        let toward = probe.to_box[0].xyz * hit.x + probe.to_box[1].xyz * hit.y + probe.to_box[2].xyz * hit.z;
        let level = roughness * frame.environment.y;
        color += weight * textureSampleLevel(probe_maps, environment_sampler, toward, i32(probe.layer.x), level).rgb;
        left -= weight;
        if left <= 0.001 {
            break;
        }
    }
    return color + left * environment * sky_specular(dir, roughness);
}

// The low 16 bits of `bits` as a half float, positive and finite (as
// ray_reflections.wgsl writes them). By hand: `unpack2x16float` needs a
// capability the scene shader doesn't ask every GPU for.
fn half_float(bits: u32) -> f32 {
    let exponent = (bits >> 10u) & 0x1fu;
    let mantissa = bits & 0x3ffu;
    if exponent == 0u {
        return f32(mantissa) * exp2(-24.0);
    }
    return bitcast<f32>(((exponent + 112u) << 23u) | (mantissa << 13u));
}

// What a surface at `world_position` sees reflected along `dir`: the ray
// traced reflection at its `pixel` (RL4) where one was traced for a `solid`
// surface smoother than the cutoff, fading into the reflection probes and
// the sky (`reflected`) toward the cutoff and where it counts less.
fn traced_reflected(
    world_position: vec3<f32>,
    dir: vec3<f32>,
    roughness: f32,
    environment: f32,
    pixel: vec2<f32>,
    solid: bool,
) -> vec3<f32> {
    let probes = reflected(world_position, dir, roughness, environment);
    let cutoff = frame.ray_reflections.x;
    if !solid || roughness >= cutoff {
        return probes;
    }
    let scale = max(u32(frame.ray_reflections.y), 1u);
    let at = vec2<u32>(max(pixel, vec2<f32>(0.0))) / scale;
    let index = at.y * u32(frame.ray_reflections.z) + at.x;
    if index >= arrayLength(&ray_reflections) {
        return probes;
    }
    let packed = ray_reflections[index];
    let rg = vec2<f32>(half_float(packed.x), half_float(packed.x >> 16u));
    let b_weight = vec2<f32>(half_float(packed.y), half_float(packed.y >> 16u));
    let fade = 1.0 - roughness / cutoff;
    let weight = clamp(b_weight.y, 0.0, 1.0) * fade * fade;
    return mix(probes, vec3<f32>(rg, b_weight.x), weight);
}

// Average specular reflection of light arriving from every direction
// (Karis' analytic approximation). Multiplies the environment's color to
// give what a surface reflects of it.
fn ambient_specular(f0: vec3<f32>, roughness: f32, n_dot_v: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * n_dot_v)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * ab.x + ab.y;
}

// How a surface reflects light arriving from direction `l`: the diffuse and
// specular parts together, times the cosine of the angle of arrival.
// Multiply by the light's color to get what the surface gives back.
fn shade(
    n: vec3<f32>, v: vec3<f32>, l: vec3<f32>,
    base_color: vec3<f32>, metalness: f32, roughness: f32, f0: vec3<f32>,
) -> vec3<f32> {
    let h = normalize(v + l);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0001);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);
    let fresnel = fresnel_schlick(v_dot_h, f0);
    let specular = distribution_ggx(n_dot_h, roughness) * geometry_smith(n_dot_v, n_dot_l, roughness)
        * fresnel / (4.0 * n_dot_v * n_dot_l + 0.0001);
    // Light that isn't reflected enters the surface and scatters (metals absorb it).
    let diffuse = (1.0 - fresnel) * (1.0 - metalness) * base_color / PI;
    return (diffuse + specular) * n_dot_l;
}

// The light a surface point gives back toward the camera: the sun (with
// its shadow), the point and spot lights listed for the point's screen
// tile, and the sky. `n` is the shading normal (tipped by any normal map),
// `geometric` the surface's own, `pixel` where it is on screen,
// `occlusion` how much ambient and sky light reaches it (texture maps and
// screen-space ambient occlusion together), `dielectric` how much a
// non-metal reflects head-on (usually 4%), and `solid` whether it's in the
// prepass (see-through surfaces aren't), so contact shadows can fall on it.
// `bumped` says `n` comes from a normal map, so the bumps shadow each other
// as the light grazes them (`bump_shadowing`); a shading normal leaned any
// other way (a leaf's crown, a surface shader's) is lit as it says.
// The weather changes the surface first (`weathered`).
fn light_surface(
    world_position: vec3<f32>,
    geometric: vec3<f32>,
    n: vec3<f32>,
    pixel: vec2<f32>,
    base_color: vec3<f32>,
    metalness: f32,
    roughness: f32,
    occlusion: f32,
    dielectric: vec3<f32>,
    solid: bool,
    bumped: bool,
) -> vec3<f32> {
    let weather = weathered(world_position, geometric, n, base_color, metalness, roughness);
    return shade_surface(world_position, geometric, weather.normal, pixel, weather.base_color,
        weather.metalness, weather.roughness, occlusion, dielectric, solid, bumped)
        + snow_glow(weather.snow);
}

// Light goes into snow, is scattered about inside it and comes out again
// somewhere else, the blue of it going furthest: so snow in shadow isn't
// the grey of a shaded wall, but glows a cold blue with the sky's light
// that fell on the snow around it. `snow` is how much of the surface snow
// covers; this is that light, to add to what the surface reflects. (In the
// sun it's a few parts in a hundred of what the snow gives back.)
const SNOW_GLOW: f32 = 0.12;
const SNOW_GLOW_COLOR: vec3<f32> = vec3<f32>(0.45, 0.7, 1.0);
fn snow_glow(snow: f32) -> vec3<f32> {
    if snow <= 0.0 {
        return vec3<f32>(0.0);
    }
    return snow * SNOW_GLOW * SNOW_GLOW_COLOR * SNOW_COLOR
        * (sky_diffuse(vec3<f32>(0.0, 1.0, 0.0)) * frame.sky_top.w + frame.ambient.rgb);
}

// How much darker a soaked porous surface is: water fills its pores, so
// less light scatters back out of it. Smooth, sealed surfaces (paint, glass,
// glazed clay) have no pores to fill and keep WET_SEALED of that change.
const WET_DARKENING: f32 = 0.45;
const WET_SEALED: f32 = 0.4;
// How much deeper a soaked surface's colors are (its saturation, added).
const WET_SATURATION: f32 = 0.3;
// How rough standing water is: a puddle is smooth.
const WET_ROUGHNESS: f32 = 0.08;
// Out of a puddle, a soaked surface keeps this share of its own roughness
// (a film of water fills only its finest bumps), and is never smoother
// than WET_FILM_ROUGHNESS for being wet. Smooth as a puddle everywhere,
// walls, hillsides and bumpy ground mirrored the sky at every glancing
// angle and came out paler wet than dry.
const WET_FILM: f32 = 0.45;
const WET_FILM_ROUGHNESS: f32 = 0.2;
// A coarse surface (bare ground, rough plaster) keeps this share instead:
// the water sinks into it and stands between its grains, and at a glancing
// angle it's the grains that are seen. With the smooth film's share the
// open ground mirrored the whole sky and a wet yard looked iced over.
const WET_FILM_COARSE: f32 = 0.68;
// The film's shine is seen out to the first distance (studs from the
// camera) and gone by the second: a far hillside is rough at the size of
// a pixel however wet its stones are, and shining it drew every ridge and
// gully of a mountain range as a bright line at dusk.
const WET_SHEEN_REACH: vec2<f32> = vec2<f32>(250.0, 900.0);
// The ground is never smoother than this for a film of water. Packed earth
// is smooth enough dry for the film to make a mirror of it, and the sky
// that mirrors comes from the sky's image, which nothing stands in front
// of: a yard shone pale right up to the foot of the house that should
// have darkened it. Its puddles are what mirror.
const WET_FILM_GROUND: f32 = 0.38;

// How rough a film of water leaves a surface `roughness` rough, `away`
// studs from the camera.
fn weather_film(roughness: f32, away: f32) -> f32 {
    let keep = mix(WET_FILM, WET_FILM_COARSE, smoothstep(0.6, 0.95, roughness));
    let film = min(roughness, max(roughness * keep, WET_FILM_ROUGHNESS));
    return mix(film, roughness, smoothstep(WET_SHEEN_REACH.x, WET_SHEEN_REACH.y, away));
}
// Water runs down walls in streaks: between them a wall is this much as wet.
const WET_WALL_DRY: f32 = 0.5;
// Rain is blown in under an eave: a wall is as wet as the open ground this
// many studs out from it, times this share. A porch deeper than that keeps
// its wall dry, and so does a room (out from an inside wall is still inside).
const WET_BLOWN: vec2<f32> = vec2<f32>(3.0, 0.85);

// The share of the world's wetness at `world_position` on a surface facing
// `n`, given the `share` straight above decides: a wall takes the rain
// blown in from in front of it (`WET_BLOWN`).
fn weather_blown(world_position: vec3<f32>, n: vec3<f32>, share: f32) -> f32 {
    let level = length(n.xz);
    if frame.weather.x <= 0.0 || level < 0.5 || share >= 0.99 {
        return share;
    }
    let out_there = world_position + vec3<f32>(n.x, 0.0, n.z) / level * WET_BLOWN.x;
    let blown = min(weather_sample(out_there).x, 1.0) * WET_BLOWN.y * smoothstep(0.5, 0.8, level);
    return max(share, blown);
}
// The share of level ground under puddles when soaked, and how wet it must
// be for the first of them.
const PUDDLE_COVER: f32 = 0.4;
const PUDDLE_START: f32 = 0.35;
// Puddles thin out from the first distance (studs from the camera) and are
// gone by the second: far off each is smaller than a pixel, and one that
// mirrors the sky beside one that doesn't only glitters.
const PUDDLE_REACH: vec2<f32> = vec2<f32>(90.0, 260.0);
// Raindrops ring the water where they land. Rings are drawn out to the
// first distance (studs from the camera) and gone by the second, where
// each is smaller than a pixel.
const DIMPLE_REACH: vec2<f32> = vec2<f32>(28.0, 70.0);
// How far a ring tips the water's surface at most (the tangent of its
// slope), and how much of that shows in a film of water out of a puddle.
const DIMPLE_SLOPE: f32 = 0.55;
const DIMPLE_FILM: f32 = 0.3;

// How much of a puddle lies at `place` (level, in studs) at `wetness`:
// water gathers in patches a few studs across, which spread and join as
// it gets wetter. None below PUDDLE_START.
fn puddle_at(place: vec2<f32>, wetness: f32) -> f32 {
    let level = PUDDLE_COVER * smoothstep(PUDDLE_START, 1.0, wetness);
    if level <= 0.0 {
        return 0.0;
    }
    let lows = 0.65 * cloud_noise2(place * 0.09) + 0.35 * cloud_noise2(place * 0.31 + 17.0);
    return 1.0 - smoothstep(level - 0.05, level, lows);
}

// Two numbers from 0 to 1 for a cell, the same each time.
fn weather_hash2(cell: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(cloud_hash2(cell), cloud_hash2(cell.yx + vec2<f32>(19.19, 7.31)));
}

// The rings of one sheet of raindrops at `place` (studs, level): the ground
// is cut into squares `size` studs across, and in each a drop lands `rate`
// times a second, somewhere new each time, as often as `rain` (0 to 1) says.
// Its ring spreads and dies away inside its square. Returns how the ring
// tips the water's surface (x and z, as the tangent of the slope).
fn dimple_sheet(place: vec2<f32>, time: f32, size: f32, rate: f32, seed: f32, rain: f32) -> vec2<f32> {
    let p = place / size + vec2<f32>(seed * 13.7, seed * 5.3);
    let cell = floor(p);
    let within = fract(p) - 0.5;
    // Each square keeps its own beat, so neighbours don't land together.
    let beat = time * rate + cloud_hash2(cell + seed) * 9.0;
    let drop = floor(beat);
    let age = fract(beat);
    let chance = weather_hash2(cell + drop * 0.61 + seed * 3.1);
    if chance.x > rain {
        return vec2<f32>(0.0);
    }
    let centre = (weather_hash2(cell * 1.7 + drop * 0.37 + seed) - 0.5) * 0.4;
    let outward = within - centre;
    let away = length(outward);
    // The ring reaches 0.3 of the square, so it never crosses its edge.
    let across = (away - age * 0.3) / (0.02 + 0.035 * age);
    // The slope of a ridge: up its inside, down its outside.
    let slope = across * exp(-across * across) * (1.0 - age) * (1.0 - age);
    return outward / max(away, 1e-4) * slope * (0.6 + 0.8 * chance.y);
}

// How falling rain (`rain`, 0 to 1) tips level water at `place`: three
// sheets of rings of different sizes and beats, so no grid shows.
fn rain_dimples(place: vec2<f32>, rain: f32) -> vec2<f32> {
    let time = frame.clock.x;
    return (dimple_sheet(place, time, 1.1, 1.7, 1.0, rain)
        + dimple_sheet(place, time, 0.7, 2.3, 2.0, rain)
        + dimple_sheet(place, time, 0.45, 3.1, 3.0, rain * 0.8)) * DIMPLE_SLOPE;
}

// Snow's color (linear) and roughness, lying and trodden.
const SNOW_COLOR: vec3<f32> = vec3<f32>(0.86, 0.88, 0.92);
const SNOW_COMPACTED_COLOR: vec3<f32> = vec3<f32>(0.62, 0.68, 0.78);
const SNOW_ROUGHNESS: f32 = 0.72;
// How ragged the snow's edge is: how much of the cover the noise adds or
// takes at the thin edge of it. Whole cover on level ground has no gaps.
const SNOW_RAGGED: f32 = 0.9;
// How far the grain of the snow's surface tips it (the tangent of the
// slope), and the distances (studs from the camera) it fades out between.
const SNOW_GRAIN: f32 = 0.11;
const SNOW_GRAIN_REACH: vec2<f32> = vec2<f32>(18.0, 55.0);
// The share of the snow's grains that glint, how smooth a glinting grain
// is, and the distances its glints fade out between.
const SNOW_GLINTS: f32 = 0.035;
const SNOW_GLINT_ROUGHNESS: f32 = 0.1;
const SNOW_GLINT_REACH: vec2<f32> = vec2<f32>(30.0, 90.0);
// How much darker the ground is under the thin, melting edge of snow.
const SNOW_EDGE_DAMP: f32 = 0.6;

// The height of the snow's grain at `place`, from 0 to 1: small lumps and
// the wind's finer marks on them.
fn snow_grain(place: vec2<f32>) -> f32 {
    return 0.5 * cloud_noise2(place * 1.3) + 0.3 * cloud_noise2(place * 4.3 + 9.0)
        + 0.2 * cloud_noise2(place * 13.0 + 5.0);
}

// How deep snow lies at `place` compared with its thickness: the wind
// heaps it in long drifts, from 0.7 to 1.3 of it.
fn snow_drift(place: vec2<f32>) -> f32 {
    return 0.7 + 0.6 * cloud_noise2(place * 0.045 + 3.0);
}

// How much snow lies (0 to 1) at `place` on a surface facing `up` (its
// normal's height) under `cover`: full on a flat top from two thirds
// cover, and at full cover down to about 37 degrees of slope; nothing
// steeper than about 66 degrees. Thin cover lies in patches.
fn snow_lying(place: vec2<f32>, up: f32, cover: f32) -> f32 {
    let ragged = 0.6 * cloud_noise2(place * 0.7 + 31.0) + 0.4 * cloud_noise2(place * 2.9);
    let thin = 1.0 - smoothstep(0.55, 1.0, cover);
    return clamp(1.5 * cover - 2.5 * (1.0 - up) + (ragged - 0.5) * SNOW_RAGGED * (0.35 + 0.65 * thin),
        0.0, 1.0);
}

// How much a surface lying along `lie` holds snow in the wind, as the
// height of a normal: the wind (`Frame::clock.yzw`) drives snow onto what
// faces it, so a slope or a wall on the windward side counts as flatter (by
// up to `SNOW_DRIVEN.x` of the way to level in a gale: none in air stiller
// than `SNOW_DRIVEN.y` studs a second, all of it from `SNOW_DRIVEN.z`), and
// one in the lee as a little steeper. Still air leaves the normal's height.
const SNOW_DRIVEN: vec3<f32> = vec3<f32>(0.55, 0.5, 14.0);
fn snow_driven(lie: vec3<f32>) -> f32 {
    let wind = frame.clock.yzw;
    let speed = length(wind.xz);
    if speed <= SNOW_DRIVEN.y {
        return lie.y;
    }
    let drive = SNOW_DRIVEN.x * smoothstep(SNOW_DRIVEN.y, SNOW_DRIVEN.z, speed);
    let facing = -dot(lie.xz, wind.xz) / speed;
    let steep = 1.0 - clamp(lie.y, 0.0, 1.0);
    return lie.y + drive * steep * select(0.35 * facing, facing, facing > 0.0);
}

// A surface as the weather leaves it.
struct Weathered {
    base_color: vec3<f32>,
    metalness: f32,
    roughness: f32,
    normal: vec3<f32>,
    // How much of it snow covers, from 0 to 1.
    snow: f32,
}

// Samples one cell of `weather_field`, clamped to the grid.
fn weather_cell(ix: i32, iz: i32, size: i32) -> vec4<f32> {
    let cx = clamp(ix, 0, size - 1);
    let cz = clamp(iz, 0, size - 1);
    return weather_field[u32(cz * size + cx)];
}

// Exterior exposure (0 = sheltered interior, 1 = exposed to sky) for one
// `weather_field` cell at `world_y`.
fn weather_cell_exposure(cell: vec4<f32>, world_y: f32, ground: bool) -> f32 {
    if cell.y < -0.5 {
        return 0.0;
    }
    if cell.y > 0.5 {
        return 1.0;
    }
    if cell.x <= -1.0e8 {
        return 1.0;
    }
    let open = smoothstep(cell.x - 0.6, cell.x - 0.1, world_y);
    if !ground {
        return open;
    }
    // The ground under something low (a bench, a barrow, a rock that
    // overhangs) isn't sheltered by it: snow drifts in under it and round
    // its foot. Only what stands well over the ground keeps it bare.
    return max(open, 1.0 - smoothstep(SHELTER_LOW.x, SHELTER_LOW.y, cell.x - world_y));
}

// How far over the ground (studs) what's above it starts to shelter it, and
// from where it shelters it wholly (`weather_cell_exposure`).
const SHELTER_LOW: vec2<f32> = vec2<f32>(2.5, 4.5);
// Snow drifts against what stands in it: the ground's snow is heaped by
// this share of how far the thing stands over the ground around it (the
// field's `bank`, at most 3 studs), up to this share of the snow's thickness.
const SNOW_BANK: vec2<f32> = vec2<f32>(0.4, 0.65);

// (exposure, footprint depth) at `world_position` from `weather_field`
// alone: 1 and 0 outside it or while there is none.
fn weather_field_sample(world_position: vec3<f32>, ground: bool) -> vec3<f32> {
    if frame.weather.w <= 0.5 || frame.weather_bounds.w <= 0.0 || frame.weather_bounds.z <= 0.0 {
        return vec3<f32>(1.0, 0.0, 0.0);
    }
    let size = i32(frame.weather_bounds.w);
    let grid = (world_position.xz - frame.weather_bounds.xy) / frame.weather_bounds.z - 0.5;
    if grid.x < -0.5 || grid.y < -0.5 || grid.x > f32(size) - 0.5 || grid.y > f32(size) - 0.5 {
        return vec3<f32>(1.0, 0.0, 0.0);
    }
    let x0 = i32(floor(grid.x));
    let z0 = i32(floor(grid.y));
    let f = fract(grid);
    let c00 = weather_cell(x0, z0, size);
    let c10 = weather_cell(x0 + 1, z0, size);
    let c01 = weather_cell(x0, z0 + 1, size);
    let c11 = weather_cell(x0 + 1, z0 + 1, size);
    let e00 = weather_cell_exposure(c00, world_position.y, ground);
    let e10 = weather_cell_exposure(c10, world_position.y, ground);
    let e01 = weather_cell_exposure(c01, world_position.y, ground);
    let e11 = weather_cell_exposure(c11, world_position.y, ground);
    // Fading out toward the field's edge, so a drift doesn't end in a step
    // where the field does.
    let edge = min(min(grid.x, grid.y), min(f32(size) - 1.0 - grid.x, f32(size) - 1.0 - grid.y));
    return vec3<f32>(mix(mix(e00, e10, f.x), mix(e01, e11, f.x), f.y),
        clamp(mix(mix(c00.z, c10.z, f.x), mix(c01.z, c11.z, f.x), f.y), 0.0, 1.0),
        max(mix(mix(c00.w, c10.w, f.x), mix(c01.w, c11.w, f.x), f.y), 0.0) * clamp(edge / 6.0, 0.0, 1.0));
}

// How far inside weather box `b` `p` is: whole inside, fading to none
// across its faces (half its fade outside them, half inside, as
// `dust_inside`).
fn weather_box_inside(b: WeatherBox, p: vec3<f32>) -> f32 {
    let q = vec4<f32>(p, 1.0);
    let local = vec3<f32>(dot(b.x, q), dot(b.y, q), dot(b.z, q));
    let half_size = b.extent.xyz;
    let inside = min(min(half_size.x - abs(local.x), half_size.y - abs(local.y)), half_size.z - abs(local.z));
    if b.extent.w <= 0.0 {
        return select(0.0, 1.0, inside > 0.0);
    }
    return saturate(inside / b.extent.w + 0.5);
}

// `shares` (x of the world's wetness, y of its snow) at `world_position`
// with the weather boxes over them: each in turn takes its own shares'
// place by how far inside it the point is, so the last one there wins.
fn weather_boxed(shares: vec2<f32>, world_position: vec3<f32>) -> vec2<f32> {
    var out = shares;
    let count = min(u32(frame.precipitation.z), MAX_WEATHER_BOXES);
    for (var i = 0u; i < count; i++) {
        let b = frame.weather_boxes[i];
        let weight = weather_box_inside(b, world_position);
        if weight > 0.0 {
            out = mix(out, b.values.xy, weight);
        }
    }
    return out;
}

// The weather at `world_position`: x the share of the world's wetness it
// has, y how deep a footprint is pressed there, z the share of the world's
// snow. The shares start as how exposed to the sky the place is (0 under a
// roof, in an Interior GraphicsVolume or a cell painted Interior, 1 in the
// open: `weather_field`), and each weather box (a WeatherVolume) there
// sets its own in their place, from 0 to 2.
fn weather_sample(world_position: vec3<f32>) -> vec3<f32> {
    return weather_sample_as(world_position, false).xyz;
}

// `weather_sample` for the ground (`ground`), which what's low doesn't
// shelter (`weather_cell_exposure`), with w how high the snow banks there
// against what stands in it, in studs (0 for anything but the ground).
fn weather_sample_as(world_position: vec3<f32>, ground: bool) -> vec4<f32> {
    var exposure = 1.0;
    let dust_count = min(frame.fog.dust_count.x, MAX_DUST);
    for (var i = 0u; i < dust_count; i++) {
        let b = frame.fog.dust[i];
        if b.color.w > 0.5 {
            exposure *= 1.0 - dust_inside(b, world_position);
        }
    }
    let field = weather_field_sample(world_position, ground);
    let shares = weather_boxed(vec2<f32>(exposure * field.x), world_position);
    let thickness = select(0.35, frame.weather.z, frame.weather.z > 0.0);
    let bank = select(0.0, min(field.z * SNOW_BANK.x, SNOW_BANK.y * thickness) * min(shares.y, 1.0), ground);
    return vec4<f32>(shares.x, field.y, shares.y, bank);
}

// Snow with a thickness lies on a solid item as a slab: its corners that
// face up are lifted (shader.wgsl's `item_vertex`), and the mesh's skirt
// (renderer/snow_caps.rs) is the slab's sides. How steep a face still holds
// it (its normal's height: none below the first, all of it from the
// second), the share of the ground's thickness that stays on things, and
// the lift in studs by which a lifted pixel is all snow.
const SNOW_CAP_SLOPE: vec2<f32> = vec2<f32>(0.45, 0.8);
const SNOW_CAP_DEPTH: f32 = 0.7;
const SNOW_CAP_WHOLE: f32 = 0.04;
// The least and most of that depth a place holds (`snow_cap_uneven`).
const SNOW_CAP_UNEVEN: vec2<f32> = vec2<f32>(0.6, 1.15);
// How ragged the foot of a slab's side is: up to this share of the slab's
// depth, the thing's own side shows through where the snow hasn't built out
// to its edge, in tongues this many studs across.
const SNOW_CAP_RAGGED: vec2<f32> = vec2<f32>(0.55, 0.45);

// The share of the slab's depth that lies over `place`: deeper here and
// shallower there, by the place alone (so corners that meet still rise
// together). A slab of one depth on every chair, pot and sill looked cut
// from a sheet.
fn snow_cap_uneven(place: vec2<f32>) -> f32 {
    return mix(SNOW_CAP_UNEVEN.x, SNOW_CAP_UNEVEN.y,
        0.6 * cloud_noise2(place * 0.9 + 11.0) + 0.4 * cloud_noise2(place.yx * 2.7));
}

// How much of a point the snow lifted by `rise` studs is snow (shader.wgsl's
// `pixel_surface`): all of the slab's top; its side from a ragged line up;
// and none where there's no snow at this very point (under a roof or in a
// weather box that a big thing's corners are outside of).
fn snow_cap_cover(world_position: vec3<f32>, rise: f32) -> f32 {
    if rise <= 0.0 {
        return 0.0;
    }
    let depth = max(smoothstep(0.2, 0.9, frame.weather.y) * frame.weather.z * SNOW_CAP_DEPTH
        * snow_cap_uneven(world_position.xz), 0.001);
    let here = min(weather_sample(world_position).z, 1.0);
    let tongues = cloud_noise2(world_position.xz / SNOW_CAP_RAGGED.y + world_position.y * 0.7)
        * 0.65 + cloud_noise2(world_position.zx / (SNOW_CAP_RAGGED.y * 0.37)) * 0.35;
    let foot = SNOW_CAP_RAGGED.x * depth * tongues;
    return smoothstep(foot, foot + SNOW_CAP_WHOLE, rise) * smoothstep(0.02, 0.3, here);
}

// How far, in studs, the snow lifts a corner at `world_position` whose
// normal's height is `up`: nothing without cover or thickness, on a steep
// face, or under shelter, and up to twice as far in a weather box whose
// snow is deeper. It depends on nothing else, so corners in one place that
// face one way rise together.
fn snow_cap(world_position: vec3<f32>, up: f32) -> f32 {
    let cover = frame.weather.y;
    let thickness = frame.weather.z;
    if cover <= 0.0 || thickness <= 0.0 || up <= SNOW_CAP_SLOPE.x {
        return 0.0;
    }
    // Thin cover is a dusting; it has its whole depth by full cover, as
    // the ground's has (terrain.wgsl).
    return smoothstep(SNOW_CAP_SLOPE.x, SNOW_CAP_SLOPE.y, up) * smoothstep(0.2, 0.9, cover)
        * thickness * SNOW_CAP_DEPTH * snow_cap_uneven(world_position.xz)
        * weather_sample(world_position).z;
}

// How deep the snow lies at `world_position`, as a share of its thickness:
// what `weather_field` and the weather boxes leave of it. (Interior
// GraphicsVolumes aren't counted: the snow's normal doesn't bank at their
// faces. Nor are prints: their shape is in the print map, `weather_print`.)
fn weather_snow_depth(world_position: vec3<f32>, ground: bool) -> f32 {
    let field = weather_field_sample(world_position, ground);
    let share = weather_boxed(vec2<f32>(field.x), world_position).y;
    let thickness = select(0.35, frame.weather.z, frame.weather.z > 0.0);
    // The drift against what stands in it, as a share of the thickness.
    let bank = select(0.0, min(field.z * SNOW_BANK.x, SNOW_BANK.y * thickness) / thickness
        * min(share, 1.0), ground);
    return share + bank;
}

// How the depth of the snow changes over the ground at `world_position`
// (x and z, a share of its thickness a stud): it banks up to nothing where
// shelter begins and steps where a weather box's snow does.
fn weather_snow_gradient(world_position: vec3<f32>, ground: bool) -> vec2<f32> {
    let field = frame.weather.w > 0.5 && frame.weather_bounds.w > 0.0 && frame.weather_bounds.z > 0.0;
    if !field && frame.precipitation.z < 0.5 {
        return vec2<f32>(0.0);
    }
    // Half a cell of the field, or half a stud without one.
    let reach = select(0.5, 0.5 * frame.weather_bounds.z, field);
    let depth = vec4<f32>(weather_snow_depth(world_position + vec3<f32>(reach, 0.0, 0.0), ground),
        weather_snow_depth(world_position - vec3<f32>(reach, 0.0, 0.0), ground),
        weather_snow_depth(world_position + vec3<f32>(0.0, 0.0, reach), ground),
        weather_snow_depth(world_position - vec3<f32>(0.0, 0.0, reach), ground));
    return vec2<f32>(depth.x - depth.y, depth.z - depth.w) / (2.0 * reach);
}

// The snow's print map. How many levels a cell's numbers have to one whole
// depth (must match `WEATHER_PRINT_LEVELS` in draw_list.rs), how far a print
// presses the snow down at most (a share of its depth: trodden snow is
// packed, not gone), how high the rim pushed out round a deep print stands
// (a share of the snow's depth), and the studs from the map's edge over
// which prints fade out.
const PRINT_LEVELS: f32 = 1024.0;
const PRINT_PRESS: f32 = 0.85;
const PRINT_RIM: f32 = 0.14;
const PRINT_FADE: f32 = 4.0;
// How strongly a print's walls tip the snow's normal (1 is the true slope),
// the steepest they tip it (the tangent of the slope), how much darker its
// walls are for the sky they don't see, and its floor at the snow's whole
// depth.
const PRINT_RELIEF: f32 = 1.25;
const PRINT_STEEPEST: f32 = 2.6;
const PRINT_WALL_SHADE: f32 = 0.2;
const PRINT_FLOOR_SHADE: f32 = 0.1;
// How far a print's floor takes the packed snow's color at its whole depth.
const PRINT_PACKED: f32 = 0.7;

// How far the snow is pressed down in print cell (`ix`, `iz`), as a share
// of its depth: positive in a print, negative on the rim round one. `first`
// is the map's first entry in `weather_field` and `fill` how far prints
// have filled in since they were stamped.
fn print_cell(ix: i32, iz: i32, size: i32, first: u32, fill: f32) -> f32 {
    let index = u32(clamp(iz, 0, size - 1) * size + clamp(ix, 0, size - 1));
    // Two 12-bit numbers in one whole f32: the press, then the rim.
    let packed = u32(weather_field[first + index / 4u][index % 4u]);
    let press = max(f32(packed & 0xFFFu) / PRINT_LEVELS - fill, 0.0);
    let rim = max(f32(packed >> 12u) / PRINT_LEVELS - fill, 0.0);
    return press - PRINT_RIM * rim * (1.0 - saturate(press * 8.0));
}

// The snow's prints at `world_position`: x how far it's pressed down there
// (a share of its depth: positive in a print, negative on a rim), yz how
// that changes over the ground along x and z (a stud), w how much the map
// counts there (1 inside it, fading to 0 at its edge and with no map).
// Read through a smooth 3 by 3 kernel (a quadratic B-spline), so a print's
// walls have no facets a cell wide.
fn weather_print(world_position: vec3<f32>) -> vec4<f32> {
    let cells = frame.weather_prints.z;
    if frame.weather.w <= 0.5 || cells <= 0.0 || frame.weather_prints.w <= 0.0 {
        return vec4<f32>(0.0);
    }
    let size = i32(frame.weather_prints.w);
    let at = (world_position.xz - frame.weather_prints.xy) * cells;
    let edge = min(min(at.x, at.y), min(f32(size) - at.x, f32(size) - at.y)) / cells;
    if edge <= 0.0 {
        return vec4<f32>(0.0);
    }
    let first = u32(frame.weather_bounds.w * frame.weather_bounds.w);
    let fill = frame.precipitation.w;
    // The nearest cell's middle, and how far from it, from -0.5 to 0.5.
    let grid = at - 0.5;
    let near = round(grid);
    let t = grid - near;
    let x0 = i32(near.x);
    let z0 = i32(near.y);
    // Each axis's three weights, and how they change.
    let wx = vec3<f32>(0.5 * (0.5 - t.x) * (0.5 - t.x), 0.75 - t.x * t.x, 0.5 * (0.5 + t.x) * (0.5 + t.x));
    let wz = vec3<f32>(0.5 * (0.5 - t.y) * (0.5 - t.y), 0.75 - t.y * t.y, 0.5 * (0.5 + t.y) * (0.5 + t.y));
    let dx = vec3<f32>(t.x - 0.5, -2.0 * t.x, t.x + 0.5);
    let dz = vec3<f32>(t.y - 0.5, -2.0 * t.y, t.y + 0.5);
    var pressed = 0.0;
    var slope = vec2<f32>(0.0);
    for (var j = 0; j < 3; j++) {
        for (var i = 0; i < 3; i++) {
            let here = print_cell(x0 + i - 1, z0 + j - 1, size, first, fill);
            pressed += wx[i] * wz[j] * here;
            slope += vec2<f32>(dx[i] * wz[j], wx[i] * dz[j]) * here;
        }
    }
    return vec4<f32>(pressed, slope * cells, saturate(edge / PRINT_FADE));
}

// How wet (x) and how pooled under a puddle (y) a surface at
// `world_position` facing `up` (its normal's height) is, each from 0 to 1,
// given the share of the world's wetness it has (`weather_sample`'s x: how
// exposed to the sky it is, or what a weather box sets, where above 1 it's
// wetter than the open, up to soaked). Water runs off undersides and down
// walls in streaks, and stands only on level ground (`pools`), in puddles.
fn weather_wet(world_position: vec3<f32>, up: f32, share: f32, pools: bool) -> vec2<f32> {
    if frame.weather.x <= 0.0 || share <= 0.001 {
        return vec2<f32>(0.0);
    }
    let wetness = min(frame.weather.x * max(share, 1.0), 1.0);
    let exposure = min(share, 1.0);
    let place = world_position.xz;
    let wall = 1.0 - smoothstep(0.3, 0.7, abs(up));
    let streaks = cloud_noise2(vec2<f32>(dot(place, vec2<f32>(2.9, 2.3)), world_position.y * 0.35));
    let run = mix(1.0, mix(WET_WALL_DRY, 1.0, smoothstep(0.3, 0.7, streaks)), wall);
    let wet = wetness * exposure * smoothstep(-1.0, -0.2, up) * run;
    if !pools {
        return vec2<f32>(wet, 0.0);
    }
    let away = distance(frame.camera_position.xyz, world_position);
    let pooled = smoothstep(0.93, 0.995, up) * puddle_at(place, wetness) * exposure
        * (1.0 - smoothstep(PUDDLE_REACH.x, PUDDLE_REACH.y, away));
    return vec2<f32>(wet, pooled);
}

// How falling rain tips the water on a surface at `world_position` facing
// `up`, `wet` and `pooled` as `weather_wet` gives them (x and z, the
// tangent of the slope). Nothing while no rain falls, on slopes or far away.
fn weather_dimples(world_position: vec3<f32>, up: f32, wet: f32, pooled: f32) -> vec2<f32> {
    let rain = frame.precipitation.x;
    if rain <= 0.0 || wet <= 0.0 || up < 0.6 {
        return vec2<f32>(0.0);
    }
    let away = distance(frame.camera_position.xyz, world_position);
    if away >= DIMPLE_REACH.y {
        return vec2<f32>(0.0);
    }
    let shown = smoothstep(0.6, 0.9, up) * wet * mix(DIMPLE_FILM, 1.0, pooled)
        * (1.0 - smoothstep(DIMPLE_REACH.x, DIMPLE_REACH.y, away));
    return rain_dimples(world_position.xz, rain) * shown;
}

// What the depth and normal prepass keeps of a weathered surface, for ray
// traced reflections: how rough the water on it leaves it, and its normal
// with the rain's rings on it. `weathered` makes the same of both.
struct WeatheredPrepass {
    roughness: f32,
    normal: vec3<f32>,
}

fn weathered_prepass(world_position: vec3<f32>, geometric: vec3<f32>, roughness: f32) -> WeatheredPrepass {
    var out = WeatheredPrepass(roughness, geometric);
    if frame.weather.x <= 0.0 {
        return out;
    }
    let sample = weather_sample(world_position);
    let up = geometric.y;
    let water = weather_wet(world_position, up, weather_blown(world_position, geometric, sample.x), true);
    if water.x <= 0.0 {
        return out;
    }
    // Snow over it is matte again.
    let snow = snow_lying(world_position.xz, up, frame.weather.y) * min(sample.z, 1.0);
    let film = weather_film(roughness, distance(frame.camera_position.xyz, world_position));
    let soaked = mix(film, min(roughness, WET_ROUGHNESS), water.y);
    out.roughness = mix(mix(roughness, soaked, water.x), max(roughness, SNOW_ROUGHNESS), snow);
    let tilt = weather_dimples(world_position, up, water.x, water.y) * (1.0 - snow);
    out.normal = normalize(geometric + vec3<f32>(tilt.x, 0.0, tilt.y));
    return out;
}

// The weather on a surface (`frame.weather`): wet surfaces are darker,
// deeper in color and glossier, undersides less so, and falling rain rings
// the water on them; snow covers surfaces facing up, the deeper it is the
// further down their slopes, and never walls. Sheltered surfaces under
// roofs, inside Interior volumes, or painted Interior stay dry and bare,
// and a weather box (a WeatherVolume) sets how wet and snowy what's in it is.
// What's pressed into the snow shows as it's shaped (`weather_print`): a
// hollow with a packed, bluer floor and walls that catch the light. Dry and
// bare, the surface is left as it is.
fn weathered(
    world_position: vec3<f32>,
    geometric: vec3<f32>,
    n: vec3<f32>,
    base_color: vec3<f32>,
    metalness: f32,
    roughness: f32,
) -> Weathered {
    return weathered_lying(world_position, geometric, true, false, n, base_color, metalness, roughness);
}

// `light_surface` for the ground (the terrain): snow lies on it under what's
// low and banks against what stands in it (`weather_sample_as`).
fn light_ground(
    world_position: vec3<f32>,
    geometric: vec3<f32>,
    n: vec3<f32>,
    pixel: vec2<f32>,
    base_color: vec3<f32>,
    metalness: f32,
    roughness: f32,
    occlusion: f32,
    dielectric: vec3<f32>,
) -> vec3<f32> {
    let weather = weathered_lying(world_position, geometric, true, true, n, base_color, metalness,
        roughness);
    return shade_surface(world_position, geometric, weather.normal, pixel, weather.base_color,
        weather.metalness, weather.roughness, occlusion, dielectric, true, false)
        + snow_glow(weather.snow);
}

// `weathered` for a surface on which rain and snow lie as on one facing
// `lie`: its own normal for most, and for a leaf the way its place in its
// plant's crown faces, since snow lies on a bush and not on each card of
// it. `pools` is whether water can stand on it in puddles.
fn weathered_lying(
    world_position: vec3<f32>,
    lie: vec3<f32>,
    pools: bool,
    ground: bool,
    n: vec3<f32>,
    base_color: vec3<f32>,
    metalness: f32,
    roughness: f32,
) -> Weathered {
    var out = Weathered(base_color, metalness, roughness, n, 0.0);
    let wetness = frame.weather.x;
    let cover = frame.weather.y;
    if wetness <= 0.0 && cover <= 0.0 {
        return out;
    }
    let sample = weather_sample_as(world_position, ground);
    // The shares of the world's wetness and snow here: how exposed to the
    // sky it is, or what a weather box sets (up to 2).
    let wet_share = weather_blown(world_position, lie, sample.x);
    let snow_share = sample.z;
    if wet_share <= 0.001 && snow_share <= 0.001 {
        return out;
    }
    // What's pressed into the snow here. The field's cell says whether
    // anything is near (its average, which also stands in for the print
    // map outside it and without one), so untrodden snow reads nothing more.
    var print = vec4<f32>(0.0);
    if sample.y > 0.0005 && cover > 0.0 {
        print = weather_print(world_position);
    }
    let footprint = mix(sample.y, saturate(print.x), print.w);
    let thickness = select(0.35, frame.weather.z, frame.weather.z > 0.0);
    let up = lie.y;
    let place = world_position.xz;
    let away = distance(frame.camera_position.xyz, world_position);

    // Snow first, since what it covers shows through only at its edge.
    // No whiter than whole: a deeper share only lifts it (`snow_cap`,
    // terrain.wgsl).
    let lying = snow_lying(place, snow_driven(lie), cover) * min(snow_share, 1.0);
    // Deep footprints thin the snow slightly on thin layers so ground peeks through.
    let snow_thin = footprint * clamp(0.45 - 0.15 * thickness, 0.1, 0.45);
    let snow = clamp(lying * (1.0 - snow_thin), 0.0, 1.0);

    // Wet: rain, and meltwater under the thin edge of snow.
    let water = weather_wet(world_position, up, wet_share, pools);
    // (None without snow: the ragged edge's noise alone isn't snow melting.)
    let melt = smoothstep(0.0, 0.25, lying) * (1.0 - smoothstep(0.5, 0.95, lying)) * SNOW_EDGE_DAMP
        * step(0.0005, cover);
    let wet = max(water.x, melt);
    let pooled = water.y;
    let porous = (1.0 - metalness) * mix(WET_SEALED, 1.0, smoothstep(0.25, 0.7, roughness));
    let soaked = wet * porous;
    let darker = base_color * mix(1.0, WET_DARKENING, soaked);
    let grey = dot(darker, vec3<f32>(0.2126, 0.7152, 0.0722));
    out.base_color = max(mix(vec3<f32>(grey), darker, 1.0 + WET_SATURATION * soaked), vec3<f32>(0.0));
    // Water stands only on level ground, in puddles; elsewhere it's a film.
    var film = weather_film(roughness, away);
    if ground {
        film = max(film, min(roughness, WET_FILM_GROUND));
    }
    let slick = mix(film, min(roughness, WET_ROUGHNESS), pooled);
    out.roughness = mix(roughness, slick, water.x);
    // A puddle's top is level whatever lies under it, but for the rings
    // falling drops leave on it.
    let dimples = weather_dimples(world_position, up, water.x, pooled);
    out.normal = normalize(mix(n, lie, pooled * water.x) + vec3<f32>(dimples.x, 0.0, dimples.y));
    if snow <= 0.0 {
        return out;
    }

    // Its surface: grain that tips it, a few grains that glint, and a
    // darker, bluer cast in the hollows of the grain and where it's trodden.
    var tilt = vec2<f32>(0.0);
    var hollow = 0.0;
    var snow_rough = mix(SNOW_ROUGHNESS, 0.45, footprint * 0.6);
    let grain_shown = 1.0 - smoothstep(SNOW_GRAIN_REACH.x, SNOW_GRAIN_REACH.y, away);
    if grain_shown > 0.0 {
        let here = snow_grain(place);
        let apart = 0.08;
        tilt = vec2<f32>(here - snow_grain(place + vec2<f32>(apart, 0.0)),
            here - snow_grain(place + vec2<f32>(0.0, apart))) / apart * SNOW_GRAIN * grain_shown;
        hollow = (1.0 - here) * grain_shown;
    }
    let glint_shown = 1.0 - smoothstep(SNOW_GLINT_REACH.x, SNOW_GLINT_REACH.y, away);
    if glint_shown > 0.0 {
        // Grains a few pixels across however far away: twice as large for
        // each doubling of the distance.
        let fine = exp2(-floor(log2(max(away, 1.0)))) * 96.0;
        let grain = world_position * fine;
        let cell = floor(grain);
        let chance = cloud_hash3(cell);
        let within = fract(grain) - 0.5;
        if chance < SNOW_GLINTS && dot(within, within) < 0.12 {
            // A flat crystal lying its own way.
            let facing = vec2<f32>(cloud_hash3(cell + 17.0), cloud_hash3(cell + 43.0)) - 0.5;
            tilt += facing * 1.6 * glint_shown;
            snow_rough = mix(snow_rough, SNOW_GLINT_ROUGHNESS, glint_shown);
        }
    }
    // Banked up to nothing where shelter begins, stepping where a weather
    // box's snow is deeper,
    // and, on the ground, rising in a drift against what stands in it.
    if abs(snow_share - 1.0) > 0.005 || sample.w > 0.001 {
        tilt -= weather_snow_gradient(world_position, ground) * clamp(thickness, 0.15, 2.0) * 1.6;
    }
    // And dipping into what's pressed into it, by the print map's own
    // slope: the terrain's corners are a stud apart, so it's the normal
    // that shows a print's shape, as a hollow with a lit and a shaded wall.
    var wall = 0.0;
    if print.w > 0.0 {
        var dip = print.yz * (PRINT_PRESS * clamp(thickness, 0.15, 2.0) * min(snow_share, 2.0)
            * PRINT_RELIEF * print.w);
        let steep = length(dip);
        if steep > PRINT_STEEPEST {
            dip *= PRINT_STEEPEST / steep;
        }
        tilt += dip;
        wall = saturate(steep / PRINT_STEEPEST);
    }
    let fresh = frame.precipitation.y;
    let lit = SNOW_COLOR * (1.0 + 0.04 * fresh) * (1.0 - 0.1 * hollow);
    // Packed and bluer in a print's floor, and a little shaded where its
    // walls and its depth hide the sky.
    let snow_albedo = mix(lit, SNOW_COMPACTED_COLOR, footprint * PRINT_PACKED)
        * (1.0 - PRINT_WALL_SHADE * wall - PRINT_FLOOR_SHADE * footprint * saturate(thickness));
    // Thin snow is see-through: the ground shows until it's a few grains deep.
    let hidden = smoothstep(0.2, 0.8, snow);
    out.base_color = mix(out.base_color, snow_albedo, hidden);
    out.metalness = mix(metalness, 0.0, hidden);
    out.roughness = mix(out.roughness, snow_rough, hidden);
    let flatten = clamp(snow * (0.75 + 0.25 * clamp(thickness, 0.0, 1.5)), 0.0, 1.0);
    let rest = normalize(mix(out.normal, lie, flatten));
    out.normal = normalize(rest + vec3<f32>(tilt.x, 0.0, tilt.y) * hidden);
    out.snow = hidden;
    return out;
}

// Light arriving at a surface point from one light: its direction `l`
// (toward the light), its color and strength there, and how much of it
// isn't shadowed.
struct Arrival {
    l: vec3<f32>,
    radiance: vec3<f32>,
    visible: f32,
}

// An `Arrival` with its visibility in parts, for skin (skin.wgsl), which
// reddens only a shadow's soft edge: `shadow` the shadow map's or the
// rays', whose edge is a penumbra, and `veil` what clouds and contact
// shadows let through, which stays grey. `arrival.visible` is the
// product, worked out as `Arrival`s always were.
struct ArrivalParts {
    arrival: Arrival,
    shadow: f32,
    veil: f32,
}

// The sun at `world_position`: its shadow, the clouds' and, for `solid`
// surfaces (in the prepass) without ray traced shadows, contact shadows.
fn sun_arrival(world_position: vec3<f32>, geometric: vec3<f32>, pixel: vec2<f32>, solid: bool)
    -> Arrival {
    return sun_arrival_parts(world_position, geometric, pixel, solid).arrival;
}

// `sun_arrival`, with its visibility in parts.
fn sun_arrival_parts(world_position: vec3<f32>, geometric: vec3<f32>, pixel: vec2<f32>,
    solid: bool) -> ArrivalParts {
    let shadow = sun_visibility(world_position, geometric, pixel);
    let clouds = cloud_shadow(frame.clouds, world_position, frame.sun_direction.xyz);
    var contact = 1.0;
    // Not where rays traced the sun: they find what contact shadows look
    // for, and their fixed per-pixel steps only add a speckle.
    if solid && ray_traced_sun(world_position, pixel) < 0.0 {
        contact = contact_visibility(world_position, geometric, frame.sun_direction.xyz,
            frame.contact.y, pixel);
    }
    // (In this order, as `sun_arrival` always multiplied them.)
    let visible = shadow * clouds * contact;
    return ArrivalParts(Arrival(frame.sun_direction.xyz, frame.sun_radiance.rgb, visible),
        shadow, clouds * contact);
}

// Lightning (Lighting's `Weather.Lightning`, `Frame::noise.z`: the chance
// of a flash in each `LIGHTNING_BEAT` seconds, 0 for none). A flash is a
// stroke that dies away in a tenth of a second, a second one after it and
// sometimes a third. xz: where in the cloud it is, as a level offset from
// straight along the sun's way (-1 to 1 each), w: how bright it is now (0
// between flashes). Worked out from the clock alone, so every pixel and
// every pass sees the same flash.
const LIGHTNING_BEAT: f32 = 2.6;
const LIGHTNING_COLOR: vec3<f32> = vec3<f32>(0.64, 0.77, 1.0);
// How bright a whole flash is on what faces it (the sun's intensity is 3),
// the share of that which comes from the whole cloud overhead and not from
// the stroke, and how bright the cloud round the stroke is.
const LIGHTNING_LIGHT: f32 = 3.4;
const LIGHTNING_FILL: f32 = 0.3;
const LIGHTNING_SKY: f32 = 1.8;
// A stroke is near the viewer: its light is whole out to the first distance
// from the camera, in studs, and down to `LIGHTNING_FAR` of itself by the
// second, so a flash doesn't light a far mountain range like the yard.
const LIGHTNING_REACH: vec2<f32> = vec2<f32>(250.0, 2200.0);
const LIGHTNING_FAR: f32 = 0.22;
// The share of the cloud's light that reaches what's under a roof.
const LIGHTNING_INDOORS: f32 = 0.1;
fn lightning() -> vec4<f32> {
    let rate = frame.noise.z;
    var flash = vec4<f32>(0.0);
    if rate <= 0.0 {
        return flash;
    }
    let time = frame.clock.x;
    // This beat's flash, and the last one's if it's still dying away.
    for (var back = 0.0; back < 2.0; back += 1.0) {
        let beat = floor(time / LIGHTNING_BEAT) - back;
        if cloud_hash2(vec2<f32>(beat, 71.3)) >= rate {
            continue;
        }
        let age = time - (beat + 0.85 * cloud_hash2(vec2<f32>(beat, 13.7))) * LIGHTNING_BEAT;
        if age < 0.0 || age > 1.5 {
            continue;
        }
        let again = cloud_hash2(vec2<f32>(beat, 29.1));
        let third = 0.24 + 0.22 * again;
        var glow = exp(-age * 15.0);
        glow += 0.75 * step(0.1, age) * exp(-(age - 0.1) * 19.0);
        glow += again * step(third, age) * exp(-(age - third) * 8.0);
        let strength = (0.55 + 0.45 * cloud_hash2(vec2<f32>(beat, 47.9))) * glow;
        if strength > flash.w {
            flash = vec4<f32>(2.0 * cloud_hash2(vec2<f32>(beat, 5.3)) - 1.0, 0.0,
                2.0 * cloud_hash2(vec2<f32>(beat, 91.7)) - 1.0, strength);
        }
    }
    return flash;
}

// A flash's own light at `world_position`, as an `Arrival` every kind of
// surface lights itself by as it does by a lamp. The stroke's light comes
// the sun's (or the moon's) way, `toward`, and is shadowed as the sun is
// (`shadow`: the shadow maps' or the rays' share, without the clouds'), so
// a flash throws the hard shadows the overcast hid, and comes in through
// windows; nothing here could shadow light from any other side. Nothing
// arrives between flashes (`visible` 0 and no radiance).
fn lightning_arrival(world_position: vec3<f32>, toward: vec3<f32>, shadow: f32) -> Arrival {
    let flash = lightning();
    if flash.w <= 0.0 {
        return Arrival(toward, vec3<f32>(0.0), 0.0);
    }
    let away = distance(frame.camera_position.xyz, world_position);
    let reach = mix(1.0, LIGHTNING_FAR, smoothstep(LIGHTNING_REACH.x, LIGHTNING_REACH.y, away));
    return Arrival(toward,
        LIGHTNING_COLOR * (LIGHTNING_LIGHT * flash.w * reach * (1.0 - LIGHTNING_FILL)), shadow);
}

// The rest of a flash on a matte surface of `color` facing `n`: the light
// of the whole lit cloud overhead, unshadowed, and only `LIGHTNING_INDOORS`
// of it under a roof or in an interior (the weather's own shelter:
// `weather_field`'s roof heights and Interior volumes). `stroke` is the
// flash's `lightning_arrival` there.
fn lightning_fill(world_position: vec3<f32>, n: vec3<f32>, color: vec3<f32>, stroke: Arrival)
    -> vec3<f32> {
    var open = weather_field_sample(world_position, false).x;
    let dust_count = min(frame.fog.dust_count.x, MAX_DUST);
    for (var i = 0u; i < dust_count; i++) {
        let b = frame.fog.dust[i];
        if b.color.w > 0.5 {
            open *= 1.0 - dust_inside(b, world_position);
        }
    }
    return stroke.radiance * (LIGHTNING_FILL / (1.0 - LIGHTNING_FILL))
        * color * ((0.5 + 0.5 * n.y) / PI * mix(LIGHTNING_INDOORS, 1.0, open));
}

// A flash's light inside the clouds, looking along `dir` from `eye`: the
// cloud glows round where the stroke is and, faintly, all over.
fn lightning_sky(eye: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    let clouds = frame.clouds;
    if clouds.color.w == 0.0 || dir.y <= 0.01 {
        return vec3<f32>(0.0);
    }
    let flash = lightning();
    if flash.w <= 0.0 {
        return vec3<f32>(0.0);
    }
    let level = max(clouds.shape.z + 0.3 * (clouds.shape.w - clouds.shape.z) - eye.y, 1.0);
    let seen = eye.xz + dir.xz * (level / dir.y);
    let sun = frame.sun_direction.xyz;
    let stroke = eye.xz + sun.xz * (level / max(sun.y, 0.3)) + flash.xz * 1400.0;
    let cloud = cloud_cover(clouds, seen);
    let near = exp(-distance(seen, stroke) / 700.0);
    return LIGHTNING_COLOR * (LIGHTNING_SKY * flash.w * (0.12 + 0.88 * near) * (0.2 + 0.8 * cloud)
        * smoothstep(0.01, 0.12, dir.y));
}

// The point and spot lights listed for `pixel`'s screen tile: where they
// start in `tile_lights`, how many there are, and a turn for their shadow
// filters, by the pixel's noise.
struct TileLamps {
    first: u32,
    count: u32,
    rotation: mat2x2<f32>,
}

fn tile_lamps(pixel: vec2<f32>) -> TileLamps {
    let angle = shading_noise(pixel) * 6.2831853;
    let rotation = mat2x2<f32>(cos(angle), sin(angle), -sin(angle), cos(angle));
    let tile_size = max(frame.lights.y, 1u);
    let tile = vec2<u32>(pixel) / tile_size;
    let first = (tile.y * frame.lights.z + tile.x) * (LIGHTS_PER_TILE + 1u);
    var count = 0u;
    if frame.lights.x > 0u {
        count = min(tile_lights[first], LIGHTS_PER_TILE);
    }
    return TileLamps(first, count, rotation);
}

// How `light` reaches `world_position`: xyz the way to where its light
// comes from (the light, or the nearest point of a rect light), w how
// strong it is there, 0 beyond its range, outside its cone or behind a rect
// light. Surfaces and volumetric fog share it, so they always agree.
fn lamp_reach(light: Light, world_position: vec3<f32>) -> vec4<f32> {
    let to_center = light.position_range.xyz - world_position;
    let center_squared = dot(to_center, to_center);
    let range = light.position_range.w;
    if center_squared >= range * range {
        return vec4<f32>(to_center, 0.0);
    }
    // Fades to exactly 0 at the range, measured from the middle, as the
    // screen tiles that list the light are.
    let edge = clamp(1.0 - pow(sqrt(center_squared) / range, 4.0), 0.0, 1.0);
    var to_light = to_center;
    var facing = 1.0;
    if light.radiance_cone.w < -2.5 {
        // A rect light shines from the nearest point of its rectangle,
        // out of its front, weaker at a slant (as a softbox does).
        let normal = light.direction_inner.xyz;
        let from_center = -to_center;
        if dot(from_center, normal) <= 0.0 {
            return vec4<f32>(to_center, 0.0);
        }
        let half_width = length(light.rect.xyz);
        let right = light.rect.xyz / max(half_width, 0.0001);
        let up = cross(normal, right);
        let nearest = light.position_range.xyz
            + right * clamp(dot(from_center, right), -half_width, half_width)
            + up * clamp(dot(from_center, up), -light.rect.w, light.rect.w);
        to_light = nearest - world_position;
        facing = clamp(dot(normalize(-to_light + normal * 0.0001), normal), 0.0, 1.0);
    }
    let distance_squared = dot(to_light, to_light);
    // Inverse-square falloff.
    var strength = edge * edge * facing / (distance_squared + 1.0);
    if light.radiance_cone.w > -1.5 {
        let l = to_light / max(sqrt(distance_squared), 0.0001);
        strength *= smoothstep(light.radiance_cone.w, light.direction_inner.w,
            dot(-l, light.direction_inner.xyz));
    }
    return vec4<f32>(to_light, strength);
}

// Whether the `i`th of `lamps` casts shadows, and so takes a slot of the
// pixel's ray traced lamps: 1 if it does.
fn lamp_slots(lamps: TileLamps, i: u32) -> u32 {
    return select(0u, 1u, lights[tile_lights[lamps.first + 1u + i]].shadow.z > 0.0);
}

// The `i`th of `lamps` at `world_position`, `slot` of those casting
// shadows before it (for its ray traced shadow). Beyond its range or
// outside its cone, nothing arrives (`visible` 0).
fn lamp_arrival(lamps: TileLamps, i: u32, slot: u32, world_position: vec3<f32>,
    geometric: vec3<f32>, pixel: vec2<f32>, solid: bool) -> Arrival {
    return lamp_arrival_parts(lamps, i, slot, world_position, geometric, pixel, solid).arrival;
}

// `lamp_arrival`, with its visibility in parts (`veil` is its contact
// shadow's).
fn lamp_arrival_parts(lamps: TileLamps, i: u32, slot: u32, world_position: vec3<f32>,
    geometric: vec3<f32>, pixel: vec2<f32>, solid: bool) -> ArrivalParts {
    var arrival = Arrival(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0), 0.0);
    let light = lights[tile_lights[lamps.first + 1u + i]];
    let reach = lamp_reach(light, world_position);
    let strength = reach.w;
    if strength <= 0.0 {
        return ArrivalParts(arrival, 0.0, 1.0);
    }
    let to_light = reach.xyz;
    let dist = length(to_light);
    let l = to_light / max(dist, 0.0001);
    // Shadow maps are drawn from the light's middle.
    let to_middle = light.position_range.xyz - world_position;
    var visible = 1.0;
    var traced = -1.0;
    if light.shadow.z > 0.0 {
        traced = ray_traced_lamp(slot, world_position, pixel);
    }
    if traced >= 0.0 {
        // Ray traced (LE1), fading into its map toward the reach.
        let away = distance(frame.camera_position.xyz, world_position);
        let blend = smoothstep(frame.ray_shadows.z * 0.85, frame.ray_shadows.z, away);
        visible = traced;
        if blend > 0.0 {
            visible = mix(traced,
                lamp_visibility(light, world_position, geometric, to_middle, lamps.rotation), blend);
        }
    } else {
        visible = lamp_visibility(light, world_position, geometric, to_middle, lamps.rotation);
    }
    // Contact shadows for lamps that cast shadows by maps, up to the lamp.
    var contact = 1.0;
    if solid && light.shadow.x >= 0.0 && traced < 0.0 {
        contact = contact_visibility(world_position, geometric, l, min(frame.contact.y, dist),
            pixel);
    }
    arrival.l = l;
    arrival.radiance = light.radiance_cone.rgb * strength;
    arrival.visible = visible * contact;
    return ArrivalParts(arrival, visible, contact);
}

// `light_surface` for a surface the weather has already changed.
// How much of a light toward `l` a normal-mapped surface keeps where its
// mapped normal `n` leans away from its own `geometric` one: the bumps
// shadow each other as the light grazes the surface (Conty Estevez, Lecocq
// and Stein, "A Microfacet-Based Shadowing Function to Solve the Bump
// Terminator Problem", Ray Tracing Gems, 2019), and nothing is lit where
// the surface itself faces away. Without it each bump's own n·l decides
// alone, and the line where light ends breaks up into lit and unlit
// speckles. Only for a normal map's normal (`shade_surface`'s `bumped`):
// it would darken any other leaned normal, such as a leaf's crown's.
fn bump_shadowing(geometric: vec3<f32>, n: vec3<f32>, l: vec3<f32>) -> f32 {
    let cos_i = dot(geometric, l);
    if cos_i <= 0.0 {
        return 0.0;
    }
    let cos_d = min(abs(dot(geometric, n)), 1.0);
    if cos_d >= 1.0 {
        return 1.0;
    }
    // The bumps' slope as a microfacet roughness (alpha squared): the
    // spread of normals whose mean leans `cos_d` off the surface.
    let tan2_d = (1.0 - cos_d * cos_d) / max(cos_d * cos_d, 1e-6);
    let alpha2 = clamp(0.125 * tan2_d, 0.0, 1.0);
    let cos2_i = max(cos_i * cos_i, 1e-6);
    let tan2_i = (1.0 - cos2_i) / cos2_i;
    return 2.0 / (1.0 + sqrt(1.0 + alpha2 * tan2_i));
}

// `bump_shadowing` for a normal map's normal (`bumped`), 1 for any other.
fn bumps(bumped: bool, geometric: vec3<f32>, n: vec3<f32>, l: vec3<f32>) -> f32 {
    if !bumped {
        return 1.0;
    }
    return bump_shadowing(geometric, n, l);
}

fn shade_surface(
    world_position: vec3<f32>,
    geometric: vec3<f32>,
    n: vec3<f32>,
    pixel: vec2<f32>,
    base_color: vec3<f32>,
    metalness: f32,
    roughness: f32,
    occlusion: f32,
    dielectric: vec3<f32>,
    solid: bool,
    bumped: bool,
) -> vec3<f32> {
    let v = normalize(frame.camera_position.xyz - world_position);
    let n_dot_v = max(dot(n, v), 0.0001);
    // Metals reflect in their own color.
    let f0 = mix(dielectric, base_color, metalness);

    let sun_parts = sun_arrival_parts(world_position, geometric, pixel, solid);
    let sun_light = sun_parts.arrival;
    let sun = shade(n, v, sun_light.l, base_color, metalness, roughness, f0)
        * sun_light.radiance * sun_light.visible * bumps(bumped, geometric, n, sun_light.l);

    // Point and spot lights: only the ones listed for this pixel's tile.
    var lit = vec3<f32>(0.0);
    let lamps = tile_lamps(pixel);
    var slot = 0u;
    for (var i = 0u; i < lamps.count; i++) {
        let lamp = lamp_arrival(lamps, i, slot, world_position, geometric, pixel, solid);
        slot += lamp_slots(lamps, i);
        if lamp.visible > 0.0 {
            lit += shade(n, v, lamp.l, base_color, metalness, roughness, f0) * lamp.radiance
                * lamp.visible * bumps(bumped, geometric, n, lamp.l);
        }
    }
    let flash = lightning_arrival(world_position, sun_light.l, sun_parts.shadow);
    if flash.radiance.b > 0.0 {
        // A stroke is a long way across: no mirror-sharp highlight of it.
        lit += shade(n, v, flash.l, base_color, metalness, max(roughness, 0.4), f0)
            * flash.radiance * flash.visible
            + lightning_fill(world_position, n, base_color * (1.0 - metalness), flash);
    }

    // Light from the sky, and a little flat ambient light on top.
    let reflection = reflect(-v, n);
    let environment = frame.sky_top.w;
    let reflectance = ambient_specular(f0, roughness, n_dot_v);
    let diffuse_color = base_color * (1.0 - metalness);
    let ambient = occlusion * (
        frame.ambient.rgb * (diffuse_color + reflectance)
        + surrounding_diffuse(world_position, n, environment) * diffuse_color
        + traced_reflected(world_position, reflection, roughness, environment, pixel, solid)
            * reflectance);
    return sun + lit + ambient;
}
