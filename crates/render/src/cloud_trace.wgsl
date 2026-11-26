// The traced clouds (renderer/cloud_trace.rs), after clouds.wgsl: the sky's
// clouds worked out before the scene pass, only where sky shows, reading
// their shapes from a texture, and kept for the sky pass to lay over its
// sky (`fs_sky` in shader.wgsl). `fs_trace` works them out at half the
// frame's size; `fs_resolve` brings that to the frame's size, working a
// pixel out afresh wherever the half-size texels round it disagree. The
// same steps as `clouded_sky` in the sky pass, which still draws them
// itself wherever these didn't.

// Must match `TraceUniform` in renderer/cloud_trace.rs.
struct Trace {
    inv_view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    sun_direction: vec4<f32>,
    sun_radiance: vec4<f32>,
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    clouds: Clouds,
}

@group(0) @binding(0) var<uniform> trace: Trace;
// `cloud_sum` over a square `CLOUD_FILL_SPAN` wide round `clouds.offset.zw`.
@group(1) @binding(0) var fill_map: texture_2d<f32>;
@group(1) @binding(1) var fill_sampler: sampler;
// The prepass's depth: 1 where no solid item was drawn.
@group(1) @binding(2) var scene_depth: texture_depth_2d;
@group(1) @binding(3) var depth_sampler: sampler;
// `fs_trace`'s clouds, for `fs_resolve`.
@group(2) @binding(0) var half_clouds: texture_2d<f32>;

// How much cloud there is above the level point `xz`: cloud_fill.wgsl's,
// with `cloud_sum` read from the texture.
fn cloud_fill(clouds: Clouds, xz: vec2<f32>) -> f32 {
    let uv = (xz - clouds.offset.xy - clouds.offset.zw) / CLOUD_FILL_SPAN + 0.5;
    return cloud_cut(clouds, textureSampleLevel(fill_map, fill_sampler, uv, 0.0).r);
}

@vertex
fn vs_screen(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let xy = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    return vec4<f32>(xy, 0.0, 1.0);
}

// How many texels across `fill_map` is (`FILL_SIZE` in
// renderer/cloud_trace.rs).
const CLOUD_FILL_TEXELS: f32 = 4096.0;

// `cloud_sum` at each texel's place.
@fragment
fn fs_fill(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let at = trace.clouds.offset.zw + (position.xy / CLOUD_FILL_TEXELS - 0.5) * CLOUD_FILL_SPAN;
    return vec4<f32>(cloud_sum(at / CLOUD_SCALE), 0.0, 0.0, 1.0);
}

// The same noise as `pixel_noise` in lighting.wgsl.
fn pixel_noise(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

// The clouds along the way through the frame's pixel at `pixel` (its
// middle, or a corner between pixels), stepped from `jitter`.
fn traced_layer(pixel: vec2<f32>, full: vec2<f32>, jitter: f32) -> vec4<f32> {
    let clip = vec2<f32>(pixel.x / full.x * 2.0 - 1.0, 1.0 - pixel.y / full.y * 2.0);
    let far = trace.inv_view_proj * vec4<f32>(clip, 1.0, 1.0);
    let dir = normalize(far.xyz / far.w - trace.camera_position.xyz);
    return cloud_layer(trace.clouds, trace.camera_position.xyz, dir, trace.sun_direction.xyz,
        trace.sun_radiance.rgb, trace.sky_top.rgb, trace.sky_horizon.rgb, jitter);
}

// A texel here lies on the corner between four of the frame's pixels.
// `fs_resolve` reads the four texels round each pixel that shows sky, so a
// texel is wanted by the six by six pixels round its corner: `cloud_layer`
// if any of them shows sky, and otherwise an alpha below 0.
@fragment
fn fs_trace(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let full = vec2<f32>(textureDimensions(scene_depth));
    let corner = position.xy * 2.0;
    var open = false;
    for (var y = -1; y <= 1 && !open; y++) {
        for (var x = -1; x <= 1; x++) {
            // The four pixels round a corner two pixels along.
            let depths = textureGather(scene_depth, depth_sampler,
                (corner + 2.0 * vec2<f32>(f32(x), f32(y))) / full);
            if max(max(depths.x, depths.y), max(depths.z, depths.w)) >= 1.0 {
                open = true;
                break;
            }
        }
    }
    if !open {
        return vec4<f32>(0.0, 0.0, 0.0, -1.0);
    }
    return traced_layer(corner, full, pixel_noise(position.xy));
}

// How far apart the four half-size texels round a pixel may be (the light
// they add as tone mapping would leave it, and the share of sky they let
// through, each from 0 to 1) for the pixel to be read between them: about
// three of a picture's 255 levels. Further apart, the pixel is on a cloud's
// edge or where each step through the layer is long (toward the horizon),
// and what lies between the texels isn't what's there.
const CLOUD_REFINE: f32 = 0.012;

// The clouds of each of the frame's pixels that shows sky (the prepass's
// depth), and an alpha below 0 for the others: read between the half-size
// texels round it where they agree, and where they don't worked out for
// this pixel as the sky pass would.
@fragment
fn fs_resolve(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let left_out = vec4<f32>(0.0, 0.0, 0.0, -1.0);
    if textureLoad(scene_depth, vec2<i32>(position.xy), 0) < 1.0 {
        return left_out;
    }
    let size = vec2<i32>(textureDimensions(half_clouds));
    let at = position.xy * 0.5 - 0.5;
    let corner = vec2<i32>(floor(at));
    let part = at - floor(at);
    var sum = vec4<f32>(0.0);
    var least = vec4<f32>(1.0);
    var most = vec4<f32>(0.0);
    var agree = true;
    for (var i = 0; i < 4; i++) {
        let offset = vec2<i32>(i & 1, i >> 1);
        let layer = textureLoad(half_clouds, clamp(corner + offset, vec2<i32>(0), size - 1), 0);
        if layer.a < 0.0 {
            agree = false;
            break;
        }
        let seen = vec4<f32>(layer.rgb / (1.0 + layer.rgb), layer.a);
        least = min(least, seen);
        most = max(most, seen);
        sum += layer * mix(1.0 - part.x, part.x, f32(offset.x)) * mix(1.0 - part.y, part.y, f32(offset.y));
    }
    let apart = most - least;
    if agree && max(max(apart.x, apart.y), max(apart.z, apart.w)) <= CLOUD_REFINE {
        return sum;
    }
    return traced_layer(position.xy, vec2<f32>(textureDimensions(scene_depth)), pixel_noise(position.xy));
}
