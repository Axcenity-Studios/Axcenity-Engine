// Clouds (graphics Phase 26), shared by the scene's shaders (after
// `fog.wgsl`, before `lighting.wgsl`) and the volumetric fog's. No
// bindings: each caller passes in its own copy of the clouds. See `Clouds`
// in draw_list.rs and renderer/clouds.rs, whose `CloudsRaw` matches
// `Clouds` here. Colors are linear light.
//
// The clouds are a layer between two heights. Above each point of the
// ground, a smooth noise says how thick the cloud is (`cloud_cover`, 0 to
// 1): a cloud fills its layer from the bottom up to that share of it, so
// thick clouds stand tall and thin ones are wisps near the bottom. The sky
// pass looks through the layer (`clouded_sky`), and surfaces take the
// sun's light through it (`cloud_shadow`), from the same noise, so the
// shadows lie under the clouds and move with them.

struct Clouds {
    // x: how much of the sky is cloud, y: how much light the thickest
    // cloud takes away per stud, z: the layer's bottom, w: its top, in
    // studs.
    shape: vec4<f32>,
    // rgb: the clouds' own color, w: 1 while they're drawn.
    color: vec4<f32>,
    // xy: how far the wind has carried them along x and z, in studs. zw:
    // the middle of the traced clouds' texture of `cloud_sum`, in studs
    // from where the wind has carried them.
    offset: vec4<f32>,
    // x: steps through the layer (0 for a flat sheet), y: steps from each
    // toward the sun (0 to judge it by the cloud above), z: 1 for fine
    // detail, w: 1 while the sky pass reads them traced (cloud_trace.wgsl).
    quality: vec4<f32>,
}

// How many studs across the clouds' largest shapes are.
const CLOUD_SCALE: f32 = 1600.0;
// How many studs the clouds' fine detail repeats over.
const CLOUD_DETAIL_SCALE: f32 = 150.0;
// How far away clouds are seen, in studs: they fade into the sky toward it.
const CLOUD_FAR: f32 = 20000.0;
// The longest stretch of the layer a ray is marched through, in studs.
// Near the horizon a ray crosses many clouds, which hide what's beyond, and
// longer steps would miss their detail and show as streaks.
const CLOUD_MARCH: f32 = 4000.0;

// A number from 0 to 1 for each whole-numbered point (a hash).
fn cloud_hash2(cell: vec2<f32>) -> f32 {
    var p = fract(vec3<f32>(cell.xyx) * 0.1031);
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}

fn cloud_hash3(cell: vec3<f32>) -> f32 {
    var p = fract(cell * 0.1031);
    p += dot(p, p.zyx + 31.32);
    return fract((p.x + p.y) * p.z);
}

// Smooth noise from 0 to 1 (value noise).
fn cloud_noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = cloud_hash2(i);
    let b = cloud_hash2(i + vec2<f32>(1.0, 0.0));
    let c = cloud_hash2(i + vec2<f32>(0.0, 1.0));
    let d = cloud_hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn cloud_noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let x0 = mix(
        mix(cloud_hash3(i), cloud_hash3(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
        mix(cloud_hash3(i + vec3<f32>(0.0, 1.0, 0.0)), cloud_hash3(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
        u.y,
    );
    let x1 = mix(
        mix(cloud_hash3(i + vec3<f32>(0.0, 0.0, 1.0)), cloud_hash3(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
        mix(cloud_hash3(i + vec3<f32>(0.0, 1.0, 1.0)), cloud_hash3(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
        u.y,
    );
    return mix(x0, x1, u.z);
}

// How far the clouds' large shapes are pulled about before they're cut
// (in their own sizes), and how much longer than wide they lie: clouds
// are drawn out and torn by the air they ride in, not round.
const CLOUD_PULL: f32 = 0.9;
const CLOUD_STRETCH: f32 = 1.25;
// How wide the band of noise is over which a cloud goes from nothing to
// filling its layer: wide, so a cloud has a long thin veil round a
// thicker middle, and full cover still has thin places in it.
const CLOUD_EDGE: f32 = 0.42;

// How wide the square of `cloud_sum` the traced clouds keep in a texture
// is, in studs (`FILL_SPAN` in renderer/cloud_trace.rs): the furthest a
// ray is marched (`CLOUD_FAR` and `CLOUD_MARCH`) and its steps toward the
// sun, with room to move before it's made again.
const CLOUD_FILL_SPAN: f32 = 52428.8;

// The clouds' shapes at `at` (a level point in the clouds' own sizes,
// carried with them by the wind), mostly between 0.25 and 0.8: five
// octaves of noise, each turned so their grids don't line up, the finer
// ones ridged into strands. `cloud_fill` cuts it at a level set by the
// cover. It is `cloud_fill`'s whole cost and depends on nothing but the
// place, so the traced clouds read it from a texture.
fn cloud_sum(at: vec2<f32>) -> f32 {
    // Drawn out one way.
    var p = vec2<f32>(0.8 * at.x + 0.6 * at.y, (0.8 * at.y - 0.6 * at.x) * CLOUD_STRETCH);
    // Pulled about by a slower noise, so no edge runs straight or round.
    let pull = vec2<f32>(cloud_noise2(p * 0.7 + vec2<f32>(31.7, 11.3)),
        cloud_noise2(p * 0.7 + vec2<f32>(5.2, 47.1))) - 0.5;
    p += pull * CLOUD_PULL;
    let turn = mat2x2<f32>(0.8, 0.6, -0.6, 0.8);
    var sum = 0.0;
    var amplitude = 0.5;
    for (var octave = 0; octave < 5; octave++) {
        var noise = cloud_noise2(p);
        if octave >= 2 {
            // Ridges: the strands and tufts along a cloud's edge.
            noise = 1.0 - abs(2.0 * noise - 1.0);
        }
        sum += amplitude * noise;
        p = turn * p * 2.13 + vec2<f32>(17.1, 3.7);
        amplitude *= 0.5;
    }
    return sum / 0.96875;
}

// How much cloud there is where the shapes stand at `sum` (`cloud_sum`):
// nothing at 0 and below, filling the layer at 1, and above 1 more than
// fills it (the heavy middle of an overcast, `cloud_belly`).
fn cloud_cut(clouds: Clouds, sum: f32) -> f32 {
    // The noise mostly lies between 0.25 and 0.8: no cover cuts above all
    // of it, full cover below (and then the thin places are thin cloud,
    // not holes).
    let cut = 0.86 - 0.95 * clouds.shape.x;
    return (sum - cut) / CLOUD_EDGE;
}

// `cloud_fill(clouds, xz)`, how much cloud there is above the level point
// `xz`, is `cloud_cut` of `cloud_sum` there: worked out in cloud_fill.wgsl,
// read from a texture in cloud_trace.wgsl.

// How thick the cloud `fill` makes is, from 0 (clear sky) to 1 (filling
// the layer): mostly veil, thickening late.
fn cloud_thick(fill: f32) -> f32 {
    let thick = clamp(fill, 0.0, 1.0);
    return thick * thick * (1.5 - 0.5 * thick);
}

// How thick the cloud is above the level point `xz`, from 0 to 1.
fn cloud_cover(clouds: Clouds, xz: vec2<f32>) -> f32 {
    return cloud_thick(cloud_fill(clouds, xz));
}

// How much of the light scattered about inside it a cloud's underside
// shows where it's `fill` full: all of it up to a full layer, and down to
// 1 - CLOUD_BELLY where the cloud is heaviest. Without it an overcast is
// one flat grey from horizon to horizon: every part of it is too deep to
// see through, so nothing else tells its heavy parts from its thin ones.
const CLOUD_BELLY: f32 = 0.5;
fn cloud_belly(fill: f32) -> f32 {
    return 1.0 - CLOUD_BELLY * smoothstep(1.0, 1.75, fill);
}

// Where in the layer (x: the middle, y: half the depth, as shares of the
// layer) a cloud `cover` thick is: a thin one is a veil a third of the way
// up, and it grows both ways until it fills the layer.
fn cloud_span(cover: f32) -> vec2<f32> {
    return vec2<f32>(0.3 + 0.2 * cover, 0.5 * cover);
}

// How much light the cloud takes away per stud at `p`.
fn cloud_density(clouds: Clouds, p: vec3<f32>) -> f32 {
    return cloud_density_filled(clouds, p).x;
}

// `cloud_density` (x) and the `cloud_fill` it was made from (y).
fn cloud_density_filled(clouds: Clouds, p: vec3<f32>) -> vec2<f32> {
    let h = (p.y - clouds.shape.z) / (clouds.shape.w - clouds.shape.z);
    if h <= 0.0 || h >= 1.0 {
        return vec2<f32>(0.0);
    }
    let fill = cloud_fill(clouds, p.xz);
    let cover = cloud_thick(fill);
    if cover <= 0.0 {
        return vec2<f32>(0.0);
    }
    // Thickest through its middle, thinning to nothing above and below.
    let span = cloud_span(cover);
    var density = clamp((span.y - abs(h - span.x)) / (0.5 * span.y + 0.04), 0.0, 1.0);
    if clouds.quality.z > 0.5 && density > 0.0 {
        // Fine detail, drawn out the same way, eats the thin parts away to
        // wisps and leaves the thick middle whole.
        let q = (p - vec3<f32>(clouds.offset.x, 0.0, clouds.offset.y)) / CLOUD_DETAIL_SCALE;
        let drawn = vec3<f32>(0.8 * q.x + 0.6 * q.z, q.y * 1.6, (0.8 * q.z - 0.6 * q.x) * CLOUD_STRETCH);
        let detail = 0.65 * cloud_noise3(drawn) + 0.35 * cloud_noise3(drawn * 2.7 + 13.1);
        density = clamp(density * 1.7 - detail * (1.45 - 0.75 * density), 0.0, 1.0);
    }
    return vec2<f32>(density * clouds.shape.y, fill);
}

// How much of the sun's light gets through `depth` of cloud (optical
// depth). Light scattered many times inside it gets further than a single
// path suggests, so a share fades much more slowly.
fn cloud_light_through(depth: f32) -> f32 {
    return 0.7 * exp(-depth) + 0.3 * exp(-0.25 * depth);
}

// The sun's light that reaches a point deep in cloud by scattering many
// times on the way, coming from every side: what makes the underside of an
// overcast sky grey and not black, brighter where the cloud is thinner.
// `direct` is `cloud_light_through` of the cloud between the point and the
// sun, from which the depth is read back (its slow share, which is all
// that's left of it by then); a cloud that lets through one part in
// 1 + 0.075 × its depth is the two-stream answer for cloud droplets.
const CLOUD_GLOW: f32 = 0.11;
fn cloud_glow(direct: f32, toward_sun: vec3<f32>) -> f32 {
    let depth = max(-4.0 * log(max(direct, 1e-6) / 0.3), 0.0);
    // A sun near the horizon has too far to go through the layer for any
    // of it to come out underneath: an overcast at sunset is grey, not lit
    // orange from inside.
    return CLOUD_GLOW * smoothstep(0.03, 0.4, toward_sun.y) / (1.0 + 0.075 * depth);
}

// How much of the sun's light the clouds scatter toward a viewer looking
// along `dir`: mostly forward (the silver lining toward the sun), a little
// back. Scaled so a thick cloud lit from above is about as bright as a
// white surface.
fn cloud_phase(dir: vec3<f32>, toward_sun: vec3<f32>) -> f32 {
    let cos_theta = dot(dir, toward_sun);
    let forward = (1.0 - 0.25) / pow(1.25 - cos_theta, 1.5);
    let back = (1.0 - 0.09) / pow(1.09 + 0.6 * cos_theta, 1.5);
    return (0.8 * forward + 0.2 * back) / 3.14159265;
}

// How much of the sun's light reaches `position` through the clouds: 1
// in the open, less in a cloud's shadow. The sun's path is taken through
// the cloud above the point where it crosses the layer, low down where
// the clouds are.
fn cloud_shadow(clouds: Clouds, position: vec3<f32>, toward_sun: vec3<f32>) -> f32 {
    if clouds.color.w == 0.0 || toward_sun.y <= 0.0 || position.y >= clouds.shape.w {
        return 1.0;
    }
    let thickness = clouds.shape.w - clouds.shape.z;
    let crossing = max(clouds.shape.z + 0.25 * thickness, position.y);
    let at = position.xz + toward_sun.xz * (crossing - position.y) / max(toward_sun.y, 0.05);
    let cover = cloud_cover(clouds, at);
    let depth = clouds.shape.y * cover * cover * thickness / max(toward_sun.y, 0.25);
    return cloud_light_through(depth);
}

// The sun's light reaching `p`, inside the clouds: through `steps` steps
// toward the sun, or, with none, through as much cloud as stands above it.
fn cloud_sunlight(clouds: Clouds, p: vec3<f32>, toward_sun: vec3<f32>, steps: u32) -> f32 {
    let thickness = clouds.shape.w - clouds.shape.z;
    let up = max(toward_sun.y, 0.25);
    if steps == 0u {
        let h = (p.y - clouds.shape.z) / thickness;
        let span = cloud_span(cloud_cover(clouds, p.xz));
        let above = clamp(span.x + span.y - h, 0.0, 2.0 * span.y) * thickness;
        return cloud_light_through(clouds.shape.y * above / up);
    }
    // To the top of the layer along the sun's way, but no further than
    // twice its thickness.
    let length = min((clouds.shape.w - p.y) / up, 2.0 * thickness);
    let step = length / f32(steps);
    var depth = 0.0;
    for (var i = 0u; i < steps; i++) {
        depth += cloud_density(clouds, p + toward_sun * (f32(i) + 0.5) * step) * step;
    }
    return cloud_light_through(depth);
}

// The sky's own light on cloud a share `h` of the way up the layer: more
// from overhead the higher up it is. The horizon's colour reaches the
// underside only weakly and greyed (it comes a long way through the cloud
// below), or a storm at sunset is lit orange-brown from underneath by a
// glow it hides.
fn cloud_skylight(sky_above: vec3<f32>, sky_around: vec3<f32>, h: f32) -> vec3<f32> {
    let around = mix(vec3<f32>(dot(sky_around, vec3<f32>(0.2126, 0.7152, 0.0722))), sky_around, 0.35);
    return mix(around, sky_above, 0.45 + 0.55 * h) * (0.5 + 0.5 * h);
}

// What's seen looking along `dir` (normalized) from `eye` through the
// clouds, in front of `sky`: the sky dimmed by them, plus the sun's light
// (`sun`, toward `toward_sun`) and the sky's (`sky_above` above, `sky_around`
// around the horizon) that they scatter. `jitter` (0 to 1, changing from
// pixel to pixel) staggers the steps so they don't show as bands.
fn clouded_sky(
    clouds: Clouds,
    eye: vec3<f32>,
    dir: vec3<f32>,
    sky: vec3<f32>,
    toward_sun: vec3<f32>,
    sun: vec3<f32>,
    sky_above: vec3<f32>,
    sky_around: vec3<f32>,
    jitter: f32,
) -> vec3<f32> {
    let layer = cloud_layer(clouds, eye, dir, toward_sun, sun, sky_above, sky_around, jitter);
    return sky * layer.a + layer.rgb;
}

// `clouded_sky` without the sky behind: the light the clouds add (rgb) and
// the share of the sky behind them that shows through (a), so the sky is
// `sky * a + rgb`. The traced clouds keep this, at half the frame's size,
// and the sky pass lays it over its own sky.
fn cloud_layer(
    clouds: Clouds,
    eye: vec3<f32>,
    dir: vec3<f32>,
    toward_sun: vec3<f32>,
    sun: vec3<f32>,
    sky_above: vec3<f32>,
    sky_around: vec3<f32>,
    jitter: f32,
) -> vec4<f32> {
    let clear = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    if clouds.color.w == 0.0 {
        return clear;
    }
    let bottom = clouds.shape.z;
    let top = clouds.shape.w;
    let thickness = top - bottom;
    let phase = cloud_phase(dir, toward_sun);
    let color = clouds.color.rgb;
    let steps = u32(clouds.quality.x);

    // A flat sheet a quarter of the way up, where the clouds are.
    if steps == 0u {
        let height = bottom + 0.25 * thickness;
        let t = (height - eye.y) / dir.y;
        if abs(dir.y) < 1e-4 || t <= 0.0 || t >= CLOUD_FAR {
            return clear;
        }
        let p = eye + dir * t;
        let fill = cloud_fill(clouds, p.xz);
        let cover = cloud_thick(fill);
        let fade = 1.0 - smoothstep(0.25 * CLOUD_FAR, CLOUD_FAR, t);
        let alpha = (1.0 - exp(-clouds.shape.y * cover * thickness * 0.5)) * cover * fade;
        let through_sun = cloud_light_through(clouds.shape.y * cover * cover * thickness / max(toward_sun.y, 0.25));
        let light = color * (sun * phase * through_sun
            + mix(sky_around, sky_above, 0.5) * 0.75 * cloud_belly(fill));
        return vec4<f32>(light * alpha, 1.0 - alpha);
    }

    // Where the ray is inside the layer.
    var near = 0.0;
    var far = 0.0;
    if abs(dir.y) < 1e-4 {
        if eye.y <= bottom || eye.y >= top {
            return clear;
        }
        far = CLOUD_FAR;
    } else {
        let to_bottom = (bottom - eye.y) / dir.y;
        let to_top = (top - eye.y) / dir.y;
        near = max(min(to_bottom, to_top), 0.0);
        far = min(max(to_bottom, to_top), CLOUD_FAR);
    }
    far = min(far, near + CLOUD_MARCH);
    if far <= near {
        return clear;
    }
    let step = (far - near) / f32(steps);
    let light_steps = u32(clouds.quality.y);
    var through = 1.0;
    var light = vec3<f32>(0.0);
    for (var i = 0u; i < steps; i++) {
        let p = eye + dir * (near + (f32(i) + jitter) * step);
        let filled = cloud_density_filled(clouds, p);
        let density = filled.x;
        if density <= 0.0 {
            continue;
        }
        let h = (p.y - bottom) / thickness;
        let direct = cloud_sunlight(clouds, p, toward_sun, light_steps);
        let lit = sun * phase * direct + (sun * cloud_glow(direct, toward_sun)
            + cloud_skylight(sky_above, sky_around, h)) * cloud_belly(filled.y);
        // Each step's own light, dimmed by the cloud in front of it
        // (integrated exactly over the step, so it conserves energy).
        let step_through = exp(-density * step);
        light += through * color * lit * (1.0 - step_through);
        through *= step_through;
        if through < 0.01 {
            break;
        }
    }
    // Far clouds fade into the sky behind them.
    let fade = 1.0 - smoothstep(0.25 * CLOUD_FAR, CLOUD_FAR, near);
    return vec4<f32>(light * fade, mix(1.0, through, fade));
}
