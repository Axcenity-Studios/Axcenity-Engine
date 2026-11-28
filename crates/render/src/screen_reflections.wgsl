// Screen-space reflections (renderer/screen_reflections.rs,
// docs/rendering/reflections.md): from each glossy solid pixel of the
// prepass (smoother than the cutoff, its roughness kept in the prepass
// normal's w as 1 plus it, `fs_prepass` in shader.wgsl and
// `fs_terrain_prepass` in terrain.wgsl), a ray along the mirrored view is
// marched across the screen through the prepass's depth. Where it passes
// behind what the screen shows, by no more than that thing's thickness, it
// hit it: what was seen there is read from last frame's picture (the
// scene's color target before this frame draws over it), where that point
// was on last frame's screen. Written as half floats, two u32s a traced
// pixel, in the same buffer the scene's `shade_surface` reads ray traced
// reflections from (frame binding 18, lighting.wgsl's `traced_reflected`),
// which puts them over the reflection probes and the sky. A miss, a ray
// that leaves the screen, a hit on something's far side, or a pixel not
// traced writes a weight of 0: the probes and the sky stay.

// Must match `ScreenReflectionUniform` in renderer/screen_reflections.rs.
struct ScreenReflections {
    // The scene's clip space (as drawn this frame, jittered with TAA) to
    // world space, and back.
    clip_to_world: mat4x4<f32>,
    world_to_clip: mat4x4<f32>,
    // World space to last frame's clip space, where its picture was drawn
    // (without TAA's jitter: TAA's result has none).
    previous: mat4x4<f32>,
    // The camera's view space to world space (for the prepass normals).
    view_to_world: mat4x4<f32>,
    // xyz: the camera, w: unused.
    camera: vec4<f32>,
    // x: the camera's forward, as a direction (xyz of the view's -Z).
    forward: vec4<f32>,
    // x, y: the traced size in pixels, z: how many scene pixels one stands
    // for each way, w: the frame's number (for the steps' offset).
    size: vec4<u32>,
    // x: the roughness from which surfaces aren't traced, y: the farthest
    // a ray goes in studs, zw: the projection's depth terms, so a depth
    // `d` is `w / (d + z)` studs in front of the camera.
    settings: vec4<f32>,
}

@group(0) @binding(0) var<uniform> reflections: ScreenReflections;
@group(0) @binding(1) var reflection_depth: texture_depth_2d;
@group(0) @binding(2) var reflection_normals: texture_2d<f32>;
// Last frame's picture, linear light, before bloom and tone mapping.
@group(0) @binding(3) var previous_scene: texture_2d<f32>;
@group(0) @binding(4) var previous_sampler: sampler;
// Two u32s a traced pixel: rg, then b and the weight, as half floats.
@group(0) @binding(5) var<storage, read_write> traced_reflections: array<vec2<u32>>;

// How many steps a ray takes at most, how many pixels the first is, and
// how much longer each is than the one before: 40 steps reach about 2,700
// pixels, across a 2560-wide screen.
const SSR_STEPS: u32 = 40u;
const SSR_FIRST_STEP: f32 = 1.5;
const SSR_GROWTH: f32 = 1.14;
// Halvings that narrow a crossing down to the surface crossed.
const SSR_REFINE: u32 = 5u;
// How thick what's on the screen is taken to be: this many studs, or this
// share of its distance from the camera if more. A ray further behind it
// than that went behind it, not into it.
const SSR_THICKNESS: vec2<f32> = vec2<f32>(0.3, 0.035);
// The share of the screen's half-width over which a hit fades out at the
// screen's edges.
const SSR_EDGE: f32 = 0.12;

// A number from 0 to 1 for each pixel, different each frame (interleaved
// gradient noise, Jimenez 2014).
fn ssr_noise(pixel: vec2<f32>, frame_number: u32) -> f32 {
    let p = pixel + 5.588238 * f32(frame_number % 64u);
    return fract(52.9829189 * fract(dot(p, vec2<f32>(0.06711056, 0.00583715))));
}

// How far in front of the camera, in studs along its forward, the surface
// at depth `d` is (the sky, at 1, is infinitely far).
fn ssr_distance(d: f32) -> f32 {
    if d >= 1.0 {
        return 1e9;
    }
    // Both terms are negative (the far plane's depth term is about -1).
    return reflections.settings.w / min(d + reflections.settings.z, -1e-9);
}

fn ssr_depth_at(pixel: vec2<f32>, full: vec2<i32>) -> f32 {
    let at = clamp(vec2<i32>(pixel), vec2<i32>(0), full - 1);
    return textureLoad(reflection_depth, at, 0);
}

// How much a hit at `uv` (0 to 1 across the screen) counts for being near
// the screen's edges.
fn ssr_edge(uv: vec2<f32>) -> f32 {
    let from_middle = abs(uv * 2.0 - 1.0);
    let edge = 1.0 - smoothstep(vec2<f32>(1.0 - SSR_EDGE), vec2<f32>(1.0), from_middle);
    return edge.x * edge.y;
}

@compute @workgroup_size(8, 8)
fn cs_screen_reflections(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = reflections.size;
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let out = id.y * size.x + id.x;
    traced_reflections[out] = vec2<u32>(0u);
    let full = vec2<i32>(textureDimensions(reflection_depth));
    let pixel = min(vec2<i32>(id.xy * size.z), full - 1);
    let depth = textureLoad(reflection_depth, pixel, 0);
    // The sky.
    if depth >= 1.0 {
        return;
    }
    // Only plain solid pixels keep their roughness (1 plus it); a
    // character's (0.5) and a cut-out's (0.875) aren't traced.
    let texel = textureLoad(reflection_normals, pixel, 0);
    if texel.w < 0.99 {
        return;
    }
    let roughness = texel.w - 1.0;
    if roughness >= reflections.settings.x {
        return;
    }
    let fsize = vec2<f32>(full);
    let uv = (vec2<f32>(pixel) + 0.5) / fsize;
    let world4 = reflections.clip_to_world * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let world = world4.xyz / world4.w;
    let camera = reflections.camera.xyz;
    let away = distance(world, camera);
    let n = normalize((reflections.view_to_world * vec4<f32>(normalize(texel.xyz), 0.0)).xyz);
    let v = (camera - world) / max(away, 1e-4);
    let dir = reflect(-v, n);
    if dot(dir, n) <= 0.0 {
        return;
    }
    // Off the surface, by more the further away it is as depth gets coarser.
    let start = world + n * (0.02 + away * 0.002);
    // The ray's end: its reach, or just short of the camera's plane.
    var reach = reflections.settings.y;
    let h0 = reflections.world_to_clip * vec4<f32>(start, 1.0);
    var h1 = reflections.world_to_clip * vec4<f32>(start + dir * reach, 1.0);
    let near = 0.05;
    if h1.w < near {
        reach *= (h0.w - near) / max(h0.w - h1.w, 1e-6);
        h1 = reflections.world_to_clip * vec4<f32>(start + dir * reach, 1.0);
    }
    // In pixels across the screen, interpolated with 1 over w so the
    // distance along the ray stays right (perspective).
    let k0 = 1.0 / h0.w;
    let k1 = 1.0 / h1.w;
    let s0 = (h0.xy * k0 * vec2<f32>(0.5, -0.5) + 0.5) * fsize;
    let s1 = (h1.xy * k1 * vec2<f32>(0.5, -0.5) + 0.5) * fsize;
    let across = s1 - s0;
    let pixels = max(length(across), 1e-3);
    // Where along the ray it leaves the screen.
    var leave = 1.0;
    for (var axis = 0; axis < 2; axis++) {
        let d = across[axis];
        if d > 1e-4 {
            leave = min(leave, (fsize[axis] - s0[axis]) / d);
        } else if d < -1e-4 {
            leave = min(leave, -s0[axis] / d);
        }
    }
    let noise = ssr_noise(vec2<f32>(pixel), size.w);
    var stride = SSR_FIRST_STEP * (0.5 + noise);
    var travelled = 0.0;
    var before = 0.0;
    var found = false;
    var hit_at = 0.0;
    var steps = 0u;
    for (var i = 0u; i < SSR_STEPS; i++) {
        travelled += stride;
        stride *= SSR_GROWTH;
        let t = min(travelled / pixels, leave);
        let ray_away = 1.0 / mix(k0, k1, t);
        let scene_away = ssr_distance(ssr_depth_at(mix(s0, s1, t), full));
        steps = i;
        if ray_away > scene_away {
            // Crossed behind what the screen shows since the last step:
            // narrowed down to where it crossed.
            var low = before;
            var high = t;
            for (var j = 0u; j < SSR_REFINE; j++) {
                let middle = 0.5 * (low + high);
                let m_ray = 1.0 / mix(k0, k1, middle);
                let m_scene = ssr_distance(ssr_depth_at(mix(s0, s1, middle), full));
                if m_ray > m_scene {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            let h_ray = 1.0 / mix(k0, k1, high);
            let h_scene = ssr_distance(ssr_depth_at(mix(s0, s1, high), full));
            let thickness = max(SSR_THICKNESS.x, h_scene * SSR_THICKNESS.y);
            if h_ray - h_scene <= thickness {
                found = true;
                hit_at = high;
                break;
            }
            // Gone behind something: keep looking past it.
        }
        before = t;
        if t >= leave {
            break;
        }
    }
    if !found {
        return;
    }
    let hit_pixel = mix(s0, s1, hit_at);
    let hit_texel = clamp(vec2<i32>(hit_pixel), vec2<i32>(0), full - 1);
    let hit_depth = textureLoad(reflection_depth, hit_texel, 0);
    let hit_uv = (vec2<f32>(hit_texel) + 0.5) / fsize;
    // What was hit faces the ray (it's the side the reflection would see),
    // not away from it (its back, which the screen doesn't show).
    let hit_normal_texel = textureLoad(reflection_normals, hit_texel, 0);
    var facing = 1.0;
    if hit_normal_texel.w > 0.25 {
        let hit_n = normalize((reflections.view_to_world
            * vec4<f32>(normalize(hit_normal_texel.xyz), 0.0)).xyz);
        facing = 1.0 - smoothstep(0.1, 0.5, dot(hit_n, dir));
    }
    // Where it was on last frame's screen.
    let hit4 = reflections.clip_to_world
        * vec4<f32>(hit_uv.x * 2.0 - 1.0, 1.0 - hit_uv.y * 2.0, hit_depth, 1.0);
    let hit_world = hit4.xyz / hit4.w;
    let before_clip = reflections.previous * vec4<f32>(hit_world, 1.0);
    if before_clip.w <= 1e-4 {
        return;
    }
    let before_uv = before_clip.xy / before_clip.w * vec2<f32>(0.5, -0.5) + 0.5;
    if any(before_uv < vec2<f32>(0.0)) || any(before_uv > vec2<f32>(1.0)) {
        return;
    }
    // Rays coming back toward the camera see what's behind it, rarely on
    // the screen, and graze what they find.
    let toward = dot(dir, reflections.forward.xyz);
    let ahead = smoothstep(-0.5, -0.1, toward);
    let end = 1.0 - smoothstep(0.8, 1.0, f32(steps) / f32(SSR_STEPS));
    let weight = facing * ahead * end * ssr_edge(hit_uv) * ssr_edge(before_uv);
    if weight <= 0.001 {
        return;
    }
    // A rougher surface's reflection is blurred: read over the footprint of
    // its lobe (roughness squared), wider the further the ray went.
    let lobe = roughness * roughness;
    let spread = min(lobe * length(hit_pixel - s0) * 0.5, 24.0) / fsize;
    var color = textureSampleLevel(previous_scene, previous_sampler, before_uv, 0.0).rgb;
    if max(spread.x, spread.y) * fsize.x > 1.0 {
        let angle = noise * 6.2831853;
        let a = vec2<f32>(cos(angle), sin(angle));
        let b = vec2<f32>(-a.y, a.x);
        color += textureSampleLevel(previous_scene, previous_sampler, before_uv + a * spread, 0.0).rgb;
        color += textureSampleLevel(previous_scene, previous_sampler, before_uv - a * spread, 0.0).rgb;
        color += textureSampleLevel(previous_scene, previous_sampler, before_uv + b * spread, 0.0).rgb;
        color += textureSampleLevel(previous_scene, previous_sampler, before_uv - b * spread, 0.0).rgb;
        color *= 0.2;
    }
    let kept = min(max(color, vec3<f32>(0.0)), vec3<f32>(60000.0));
    traced_reflections[out] = vec2<u32>(pack2x16float(kept.rg),
        pack2x16float(vec2<f32>(kept.b, clamp(weight, 0.0, 1.0))));
}
