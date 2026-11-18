// Hair shading (`Shading::Hair`, `docs/rendering/hair.md`): compiled with
// shader.wgsl, whose material bindings (`maps`, and the metalness and
// specular maps' slots, which hold a card atlas's data and flow maps) and
// `lit_pixel` it works with, after lighting.wgsl, whose lights, shadows
// and sky it takes.
//
// A Marschner-style model in Karis's approximation ("Physically Based
// Hair Shading in Unreal", SIGGRAPH 2016), chosen over Kajiya-Kay with two
// shifted lobes because its three lobes come from the fibre itself: R, a
// white reflection off the cuticle, shifted toward the root by twice the
// cuticle's tilt; TT, light passing through the fibre, which gives hair
// lit from behind its glow; TRT, light reflected once inside, shifted
// toward the tip by four times the tilt, wider, and tinted by the fibre's
// absorption (d'Eon, Francois, Hill, Letteri and Aubry, "An Energy-
// Conserving Hair Reflectance Model", 2011; Chiang, Bitterli, Tappan and
// Burley, 2016, whose colour mapping it shares with Blender's Principled
// Hair BSDF, the reference renders' model). Light scattered among many
// strands is Karis's soft Kajiya-Kay term. It is lit along the strand,
// not the card's normal: a card is a sheet of strands, not a surface.
//
// The colour is the material's (base colour times the atlas's colour),
// moved along the two-pigment absorption curve (eumelanin and
// pheomelanin, Donner and Jensen's coefficients as pbrt v4 gives them) by
// the material values. Neutral values leave it as made.
//
// `Maps::shading` for hair: x SHADING_HAIR; y which maps are bound (1 the
// atlas's data in the metalness map's place, 2 the flow map in the
// specular map's place); z the cuticle tilt's scale (`shift`); w the
// secondary's strength. `Maps::params` holds the colour values
// (renderer/lighting.rs, `hair_uniform`).

// The cuticle's tilt, in radians: 2°, Blender's Principled Hair default
// and inside Marschner's measured 2° to 5°.
const HAIR_TILT: f32 = 0.0349;
// A fibre's reflectance head-on (index of refraction 1.55).
const HAIR_F0: f32 = 0.0465;
// Chiang and others' mapping from colour to absorption at an azimuthal
// roughness of 0.3: absorption = (ln colour / this)².
const HAIR_COLOUR_SCALE: f32 = 5.888;
// Absorption per unit of each pigment, red, green and blue (Donner and
// Jensen 2006, as pbrt v4 gives them).
const HAIR_EUMELANIN: vec3<f32> = vec3<f32>(0.419, 0.697, 1.37);
const HAIR_PHEOMELANIN: vec3<f32> = vec3<f32>(0.187, 0.4, 1.05);
// Melanin 1 multiplies the absorption by 2 to this (8), -1 divides it.
const HAIR_MELANIN_RANGE: f32 = 3.0;
// Pheomelanin's share of the melanin that `redness` 1 and -1 leave at
// least and at most when the colour has next to none (white hair stays
// white; a trace of melanin turns strawberry, not grey).
const HAIR_TRACE: f32 = 0.02;
// How much lighter tips get at `tip_lightening` 1 (a melanin offset), and
// over which part of the strand (root 0, tip 1) it comes in.
const HAIR_TIP_LIGHTENING: f32 = 1.5;
const HAIR_TIP_START: f32 = 0.4;
// How much light scattered among many strands comes back (Karis's
// "scatter"): a quarter, set against Blender's Chiang model on a ball of
// strands (at 1, 30° off the highlight the ball stayed 35 to 50% as
// bright where Blender's falls to 3 to 10%, and read flat). And how much
// each strand's tilt differs from the next (by the atlas's per-strand
// value), so highlights break up as real ones do.
const HAIR_SCATTER: f32 = 0.25;
const HAIR_SHIFT_SPREAD: f32 = 0.3;
// The atlas's data where a material has none: halfway along every strand,
// a middle per-strand value, no streak, no occlusion of its own.
const HAIR_NO_DATA: vec4<f32> = vec4<f32>(0.5, 0.5, 0.0, 1.0);
// How wide the band of opacities round a hair card's cutoff is that its
// hashed alpha grades under TAA (`hair_cut_threshold`): 0.2 to 0.8 at
// glTF's default cutoff of 0.5.
const HAIR_HASH_BAND: f32 = 0.6;

fn hair_has_data() -> bool {
    return (u32(maps.shading.y + 0.5) & 1u) != 0u;
}

fn hair_has_flow() -> bool {
    return (u32(maps.shading.y + 0.5) & 2u) != 0u;
}

// The amounts of eumelanin and pheomelanin whose absorption is nearest
// `sigma` (least squares, neither below 0).
fn hair_pigments(sigma: vec3<f32>) -> vec2<f32> {
    let ee = dot(HAIR_EUMELANIN, HAIR_EUMELANIN);
    let pp = dot(HAIR_PHEOMELANIN, HAIR_PHEOMELANIN);
    let ep = dot(HAIR_EUMELANIN, HAIR_PHEOMELANIN);
    let es = dot(HAIR_EUMELANIN, sigma);
    let ps = dot(HAIR_PHEOMELANIN, sigma);
    let det = ee * pp - ep * ep;
    var e = (pp * es - ep * ps) / det;
    var p = (ee * ps - ep * es) / det;
    if p < 0.0 {
        p = 0.0;
        e = es / ee;
    }
    if e < 0.0 {
        e = 0.0;
        p = ps / pp;
    }
    return max(vec2<f32>(e, p), vec2<f32>(0.0));
}

// The hair's colour at a pixel `along` its strand (root 0, tip 1), in a
// streak by `streak` (0 to 1): the material's colour `made` moved along
// the pigment curve by melanin (and the root's, tips' and streaks'
// offsets) and redness, then dyed and tinted. Neutral values give `made`.
fn hair_colour(made: vec3<f32>, along: f32, streak: f32) -> vec3<f32> {
    let values = maps.params;
    var melanin = values[0].x;
    if values[1].w != 0.0 {
        melanin += values[1].w * (1.0 - smoothstep(0.0, max(values[2].x, 0.001), along));
    }
    melanin -= HAIR_TIP_LIGHTENING * values[0].z * smoothstep(HAIR_TIP_START, 1.0, along);
    melanin += values[2].z * values[2].y * streak;
    let redness = values[0].y;
    var colour = made;
    if melanin != 0.0 || redness != 0.0 {
        let c = clamp(made, vec3<f32>(0.002), vec3<f32>(0.998));
        let root = log(c) / -HAIR_COLOUR_SCALE;
        var sigma = root * root;
        if redness != 0.0 {
            // Redness moves pheomelanin's share of the same melanin toward
            // all of it (1) or none (-1), as Blender's "melanin redness"
            // splits it; what the two pigments don't explain is kept.
            let pigments = hair_pigments(sigma);
            let total = pigments.x + pigments.y;
            let rest = sigma - pigments.x * HAIR_EUMELANIN - pigments.y * HAIR_PHEOMELANIN;
            let share = pigments.y / max(total, 1e-6);
            let moved = select(mix(share, 0.0, -redness), mix(share, 1.0, redness),
                redness > 0.0);
            let amount = max(total, HAIR_TRACE * abs(redness));
            sigma = max(rest + amount * ((1.0 - moved) * HAIR_EUMELANIN
                + moved * HAIR_PHEOMELANIN), vec3<f32>(0.0));
        }
        sigma *= exp2(HAIR_MELANIN_RANGE * melanin);
        colour = exp(-sqrt(sigma) * HAIR_COLOUR_SCALE);
    }
    colour = mix(colour, values[1].xyz, values[0].w);
    return colour * values[3].xyz;
}

// Along the strand toward its root, in world space: the flow map's
// direction (or the texture's +V) in the card's tangent space, reversed.
// From the item's own normal and tangent, not turned to face the camera,
// so both sides of a card agree which way the root is.
fn hair_toward_root(in: VertexOut, flow_texel: vec4<f32>) -> vec3<f32> {
    let n = normalize(in.normal);
    let along_u = in.tangent.xyz - n * dot(n, in.tangent.xyz);
    if dot(along_u, along_u) < 1e-12 {
        return vec3<f32>(0.0, 1.0, 0.0);
    }
    let u = normalize(along_u);
    let v = cross(n, u) * select(1.0, -1.0, in.tangent.w < 0.0);
    var flow = vec2<f32>(0.0, 1.0);
    if hair_has_flow() {
        flow = flow_texel.xy * 2.0 - 1.0;
    }
    let tip = u * flow.x + v * flow.y;
    if dot(tip, tip) < 1e-12 {
        return -v;
    }
    return -normalize(tip);
}

// A unit-area Gaussian of `width` at `x`.
fn hair_gaussian(width: f32, x: f32) -> f32 {
    let w = max(width, 1e-3);
    return exp(-0.5 * x * x / (w * w)) / (2.5066283 * w);
}

fn hair_fresnel(cos_theta: f32) -> f32 {
    return HAIR_F0 + (1.0 - HAIR_F0) * pow(1.0 - clamp(cos_theta, 0.0, 1.0), 5.0);
}

// Light from `l` scattered toward `v` by strands running along `t`
// (toward the root), `visible` of it not shadowed: the three lobes and the
// light among many strands, before the light's colour and strength.
fn hair_scatter(
    t: vec3<f32>,
    v: vec3<f32>,
    l: vec3<f32>,
    colour: vec3<f32>,
    roughness: f32,
    tilt: f32,
    secondary: f32,
    visible: f32,
) -> vec3<f32> {
    let sin_l = clamp(dot(t, l), -1.0, 1.0);
    let sin_v = clamp(dot(t, v), -1.0, 1.0);
    let cos_d = max(cos(0.5 * abs(asin(sin_v) - asin(sin_l))), 0.05);
    let lp = l - sin_l * t;
    let vp = v - sin_v * t;
    let cos_phi = dot(lp, vp) * inverseSqrt(dot(lp, lp) * dot(vp, vp) + 1e-4);
    let cos_half_phi = sqrt(clamp(0.5 + 0.5 * cos_phi, 0.0, 1.0));
    let b = max(roughness * roughness, 0.002);
    let refracted = 1.19 / cos_d + 0.36 * cos_d;
    var s = vec3<f32>(0.0);

    // R: off the cuticle, white, toward the root.
    let alpha = -2.0 * tilt;
    let sa = sin(alpha);
    let ca = cos(alpha);
    let offset = 2.0 * sa * (ca * cos_half_phi * sqrt(clamp(1.0 - sin_v * sin_v, 0.0, 1.0))
        + sa * sin_v);
    let r = hair_gaussian(b * 1.4142136 * cos_half_phi, sin_l + sin_v - offset)
        * 0.25 * cos_half_phi * hair_fresnel(sqrt(clamp(0.5 + 0.5 * dot(v, l), 0.0, 1.0)));
    s += vec3<f32>(r);

    // TT: through the fibre, for light from behind.
    let a = 1.0 / refracted;
    let h = cos_half_phi * (1.0 + a * (0.6 - 0.8 * cos_phi));
    let f_tt = hair_fresnel(cos_d * sqrt(clamp(1.0 - h * h, 0.0, 1.0)));
    let through = pow(colour, vec3<f32>(0.5 * sqrt(clamp(1.0 - h * h * a * a, 0.0, 1.0)) / cos_d));
    s += hair_gaussian(0.5 * b, sin_l + sin_v - tilt) * exp(-3.65 * cos_phi - 3.98)
        * (1.0 - f_tt) * (1.0 - f_tt) * through;

    // TRT: once round inside, tinted, wider, toward the tip.
    let f_trt = hair_fresnel(cos_d * 0.5);
    let inside = pow(colour, vec3<f32>(0.8 / cos_d));
    s += secondary * hair_gaussian(2.0 * b, sin_l + sin_v - 4.0 * tilt)
        * exp(17.0 * cos_phi - 16.78) * (1.0 - f_trt) * (1.0 - f_trt) * f_trt * inside;

    // Among many strands: a soft Kajiya-Kay diffuse, wrapped round a
    // normal facing the camera across the strand, more saturated where
    // light comes through more hair (in shadow).
    let across = v - t * sin_v;
    var facing = vec3<f32>(0.0, 0.0, 1.0);
    if dot(across, across) > 1e-8 {
        facing = normalize(across);
    }
    let wrapped = clamp((dot(facing, l) + 1.0) * 0.5, 0.0, 1.0);
    let spread = (1.0 / PI) * wrapped * (1.0 - abs(sin_l)) * HAIR_SCATTER;
    let luma = max(dot(colour, vec3<f32>(0.2126, 0.7152, 0.0722)), 1e-4);
    let deeper = pow(max(colour / luma, vec3<f32>(1e-4)), vec3<f32>(1.0 - visible));
    s += sqrt(colour) * spread * deeper;
    return max(s, vec3<f32>(0.0));
}

// The light a hair pixel gives back toward the camera: the sun, the lamps
// listed for its tile and the sky, as `light_surface` gathers them.
// `data_texel` and `flow_texel` are the atlas's maps at the pixel.
fn hair_surface(
    in: VertexOut,
    facing: f32,
    shaded: Surface,
    roughness: f32,
    occlusion: f32,
    solid: bool,
    data_texel: vec4<f32>,
    flow_texel: vec4<f32>,
) -> vec3<f32> {
    let data = select(HAIR_NO_DATA, data_texel, hair_has_data());
    // (Never quite black: the lobes raise it to powers.)
    let colour = max(hair_colour(shaded.base_color, data.r, data.b), vec3<f32>(1e-4));
    let t = hair_toward_root(in, flow_texel);
    let geometric = normalize(in.normal) * facing;
    let world_position = in.world_position;
    let pixel = in.clip_position.xy;
    let v = normalize(frame.camera_position.xyz - world_position);
    let tilt = HAIR_TILT * maps.shading.z * (1.0 + HAIR_SHIFT_SPREAD * (2.0 * data.g - 1.0));
    let secondary = maps.shading.w;

    let sun = sun_arrival(world_position, geometric, pixel, solid);
    var lit = hair_scatter(t, v, sun.l, colour, roughness, tilt, secondary, sun.visible)
        * sun.radiance * sun.visible;
    let lamps = tile_lamps(pixel);
    var slot = 0u;
    for (var i = 0u; i < lamps.count; i++) {
        let lamp = lamp_arrival(lamps, i, slot, world_position, geometric, pixel, solid);
        slot += lamp_slots(lamps, i);
        if lamp.visible > 0.0 {
            lit += hair_scatter(t, v, lamp.l, colour, roughness, tilt, secondary, lamp.visible)
                * lamp.radiance * lamp.visible;
        }
    }

    // The sky, round a normal facing the camera across the strand, darker
    // where the atlas says strands are deep in a clump.
    let across = v - t * dot(v, t);
    var around = geometric;
    if dot(across, across) > 1e-8 {
        around = normalize(across);
    }
    let environment = frame.sky_top.w;
    let sheen_roughness = max(roughness, 0.3);
    let reflectance = ambient_specular(vec3<f32>(HAIR_F0), sheen_roughness,
        max(dot(around, v), 1e-4));
    let ambient = occlusion * data.a * (
        frame.ambient.rgb * (colour + reflectance)
        + surrounding_diffuse(world_position, around, environment) * colour
        + traced_reflected(world_position, reflect(-v, around), sheen_roughness, environment,
            pixel, solid) * reflectance);
    return lit + ambient;
}

// The opacity under which a pixel of a cut-out is cut: its cutoff, or for
// a hair card while TAA is on (`frame.noise.y`) a threshold from 0 to 1
// that changes from pixel to pixel and frame to frame (hashed alpha: the
// interleaved gradient noise that `shading_noise` moves each frame, offset
// by the atlas's per-strand value so crossing cards don't cut alike), so
// TAA averages a card's soft edges and thin strands into their coverage.
fn hair_cut_threshold(in: VertexOut, cutoff: f32, strand: f32) -> f32 {
    if maps.shading.x != SHADING_HAIR || frame.noise.y == 0.0 || cutoff <= 0.0 {
        return cutoff;
    }
    // Each pixel's thresholds step by the golden ratio from frame to frame,
    // so a few frames already cover 0 to 1 evenly and TAA's blend settles
    // with little left to flicker; neighbours start apart (interleaved
    // gradient noise), so a frame's pattern is fine-grained.
    let layer = floor(strand * 16.0) * 0.618034;
    let noise = fract(pixel_noise(in.clip_position.xy) + 0.618034 * frame.noise.x + layer);
    // Over the band round the cutoff only: below it a strand's fringe is
    // cut, above it drawn, every frame. Hashing every opacity from 0 to 1
    // flickered more (5.73 against 4.98 of 255 on the test's ramp card:
    // TAA blends a tenth of each frame in, so a pixel drawn one frame and
    // cut the next swings by a tenth of its brightness), and real atlases
    // are whole or clear but at their strands' edges.
    return clamp(cutoff + HAIR_HASH_BAND * (noise - 0.5), 0.002, 0.998);
}
