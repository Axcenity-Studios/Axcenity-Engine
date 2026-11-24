// Cloth shading (`Shading::Cloth`, `docs/rendering/cloth.md`): compiled
// with shader.wgsl (its `maps` and `lit_pixel`), after lighting.wgsl (its
// lights, shadows and sky).
//
// The material's own diffuse and specular light under a sheen layer, as
// glTF's KHR_materials_sheen defines it: the Charlie distribution
// (Estevez and Kulla, "Production Friendly Microfacet Sheen BRDF", 2017)
// with Neubelt and Pettineo's visibility ("Crafting a Next-Gen Material
// Pipeline for The Order: 1886", 2013), which brightens toward grazing
// angles, where light skims the fibres standing off the cloth, and adds
// next to nothing head-on. What the sheen reflects is taken from the layer
// under it by the sheen's directional albedo (`CLOTH_SHEEN_ALBEDO`), so a
// sheen never adds light that wasn't there. The specular highlight can be
// stretched along a direction (anisotropic GGX, Burley 2012, with Heitz's
// height-correlated visibility, as Filament has it), for satin, silk and
// brushed fibres; with no anisotropy it is Standard's own.
//
// `Maps::shading` for cloth: x SHADING_CLOTH; y the sheen's roughness; z
// the anisotropy (-1 to 1: along the direction, or across it); w the
// direction's turn from the texture's U, in radians. `Maps::toon.xyz` is
// the sheen's colour, in linear light (cloth has no toon values).

// The sheen's directional albedo: how much of the light reaching it a
// white sheen reflects, by the cosine to the viewer (columns, 0 to 1) and
// its roughness (rows, 0.1 to 1), integrated over the hemisphere and held
// to 1. The render test `the_sheen_albedo_table_is_charlies_integrated`
// recomputes it and prints it again if it is wrong.
const CLOTH_COLUMNS: u32 = 16u;
const CLOTH_ROWS: u32 = 8u;
const CLOTH_SHEEN_ALBEDO: array<f32, 128> = array<f32, 128>(
    1.0000, 1.0000, 0.5829, 0.2381, 0.0914, 0.0322, 0.0101, 0.0028, 0.0007, 0.0001, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000,
    1.0000, 1.0000, 0.7262, 0.4997, 0.3435, 0.2336, 0.1560, 0.1016, 0.0641, 0.0387, 0.0221, 0.0117, 0.0056, 0.0022, 0.0006, 0.0001,
    1.0000, 0.8490, 0.6545, 0.5148, 0.4073, 0.3218, 0.2527, 0.1966, 0.1507, 0.1134, 0.0832, 0.0590, 0.0398, 0.0250, 0.0137, 0.0056,
    0.8542, 0.7182, 0.5886, 0.4920, 0.4143, 0.3496, 0.2945, 0.2469, 0.2054, 0.1692, 0.1373, 0.1093, 0.0847, 0.0631, 0.0443, 0.0279,
    0.7390, 0.6381, 0.5411, 0.4679, 0.4079, 0.3568, 0.3122, 0.2726, 0.2371, 0.2049, 0.1755, 0.1486, 0.1238, 0.1009, 0.0797, 0.0600,
    0.6678, 0.5862, 0.5078, 0.4484, 0.3993, 0.3572, 0.3200, 0.2865, 0.2560, 0.2278, 0.2017, 0.1772, 0.1542, 0.1324, 0.1116, 0.0918,
    0.6209, 0.5510, 0.4842, 0.4334, 0.3916, 0.3555, 0.3234, 0.2945, 0.2679, 0.2431, 0.2199, 0.1980, 0.1771, 0.1571, 0.1378, 0.1192,
    0.5886, 0.5263, 0.4670, 0.4222, 0.3852, 0.3533, 0.3250, 0.2993, 0.2757, 0.2537, 0.2330, 0.2133, 0.1944, 0.1762, 0.1586, 0.1414,
);
// The smoothest sheen drawn (the table's first row).
const CLOTH_SMOOTHEST_SHEEN: f32 = 0.1;
// The narrowest highlight an anisotropic lobe takes, along either axis
// (a GGX alpha).
const CLOTH_NARROWEST: f32 = 0.002;

// The sheen's directional albedo at `n_dot_v` for `roughness` (the
// table, read between its entries).
fn cloth_sheen_albedo(n_dot_v: f32, roughness: f32) -> f32 {
    let x = clamp(n_dot_v, 0.0, 1.0) * f32(CLOTH_COLUMNS - 1u);
    let y = clamp((roughness - CLOTH_SMOOTHEST_SHEEN) / (1.0 - CLOTH_SMOOTHEST_SHEEN), 0.0, 1.0)
        * f32(CLOTH_ROWS - 1u);
    let x0 = min(u32(x), CLOTH_COLUMNS - 2u);
    let y0 = min(u32(y), CLOTH_ROWS - 2u);
    let fx = x - f32(x0);
    let fy = y - f32(y0);
    let at = y0 * CLOTH_COLUMNS + x0;
    let top = mix(CLOTH_SHEEN_ALBEDO[at], CLOTH_SHEEN_ALBEDO[at + 1u], fx);
    let bottom = mix(CLOTH_SHEEN_ALBEDO[at + CLOTH_COLUMNS],
        CLOTH_SHEEN_ALBEDO[at + CLOTH_COLUMNS + 1u], fx);
    return mix(top, bottom, fy);
}

// The Charlie distribution of sheen roughness `roughness` (its alpha is
// the roughness squared, as glTF's sheen has it).
fn cloth_charlie(n_dot_h: f32, roughness: f32) -> f32 {
    let inverse = 1.0 / (roughness * roughness);
    let sin2 = max(1.0 - n_dot_h * n_dot_h, 0.0);
    return (2.0 + inverse) * pow(sin2, inverse * 0.5) / (2.0 * PI);
}

// Neubelt and Pettineo's visibility for sheen.
fn cloth_neubelt(n_dot_v: f32, n_dot_l: f32) -> f32 {
    return clamp(1.0 / (4.0 * (n_dot_l + n_dot_v - n_dot_l * n_dot_v)), 0.0, 1.0);
}

// The surface's own light from `l` (as `shade`), with its highlight
// stretched by `anisotropy` along `t` (positive) or `b` (negative).
fn cloth_base(
    n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, t: vec3<f32>, b: vec3<f32>,
    base_color: vec3<f32>, metalness: f32, roughness: f32, f0: vec3<f32>, anisotropy: f32,
) -> vec3<f32> {
    if anisotropy == 0.0 {
        return shade(n, v, l, base_color, metalness, roughness, f0);
    }
    let h = normalize(v + l);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0001);
    let n_dot_h = max(dot(n, h), 0.0);
    let fresnel = fresnel_schlick(max(dot(v, h), 0.0), f0);
    let a = roughness * roughness;
    let at = max(a * (1.0 + anisotropy), CLOTH_NARROWEST);
    let ab = max(a * (1.0 - anisotropy), CLOTH_NARROWEST);
    let th = dot(t, h);
    let bh = dot(b, h);
    let d_term = th * th / (at * at) + bh * bh / (ab * ab) + n_dot_h * n_dot_h;
    let distribution = 1.0 / (PI * at * ab * d_term * d_term);
    let lambda_v = n_dot_l * length(vec3<f32>(at * dot(t, v), ab * dot(b, v), n_dot_v));
    let lambda_l = n_dot_v * length(vec3<f32>(at * dot(t, l), ab * dot(b, l), n_dot_l));
    let visibility = 0.5 / (lambda_v + lambda_l + 1e-5);
    let specular = distribution * visibility * fresnel;
    let diffuse = (1.0 - fresnel) * (1.0 - metalness) * base_color / PI;
    return (diffuse + specular) * n_dot_l;
}

// The light a cloth pixel gives back toward the camera: the sun, the
// lamps listed for its tile and the sky, as `light_surface` gathers them,
// under the sheen.
fn cloth_surface(
    in: VertexOut,
    facing: f32,
    geometric: vec3<f32>,
    shaded: Surface,
    metalness: f32,
    roughness: f32,
    occlusion: f32,
    dielectric: vec3<f32>,
    solid: bool,
) -> vec3<f32> {
    let weather = weathered(in.world_position, geometric, shaded.normal, shaded.base_color, metalness, roughness);
    let n = weather.normal;
    let base_color = weather.base_color;
    let metal = weather.metalness;
    let rough = weather.roughness;
    let world_position = in.world_position;
    let pixel = in.clip_position.xy;
    let bumped = maps.settings.x != 0.0;
    let v = normalize(frame.camera_position.xyz - world_position);
    let n_dot_v = max(dot(n, v), 0.0001);
    let f0 = mix(dielectric, base_color, metal);
    let sheen = maps.toon.xyz;
    let sheen_roughness = max(maps.shading.y, CLOTH_SMOOTHEST_SHEEN);
    let anisotropy = maps.shading.z;

    // The direction a highlight stretches along: the texture's U (the
    // mesh's tangent, made perpendicular to the normal), turned about
    // the normal.
    var t = vec3<f32>(0.0);
    var b = vec3<f32>(0.0);
    if anisotropy != 0.0 {
        let tangent = in.tangent.xyz * facing;
        var along = tangent - n * dot(n, tangent);
        if dot(along, along) < 1e-12 {
            along = cross(n, select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0),
                abs(n.x) > 0.9));
        }
        let u = normalize(along);
        let w = cross(n, u);
        let turn = maps.shading.w;
        t = u * cos(turn) + w * sin(turn);
        b = cross(n, t);
    }

    // What the sheen reflects isn't there for the layer under it.
    let sheen_strength = max(sheen.r, max(sheen.g, sheen.b));
    let under = 1.0 - sheen_strength * cloth_sheen_albedo(n_dot_v, sheen_roughness);

    let sun_parts = sun_arrival_parts(world_position, geometric, pixel, solid);
    let sun = sun_parts.arrival;
    var lit = cloth_light(n, v, sun.l, t, b, base_color, metal, rough, f0, anisotropy, sheen,
        sheen_roughness, under) * sun.radiance * sun.visible * bumps(bumped, geometric, n, sun.l);
    let flash = lightning_arrival(world_position, sun.l, sun_parts.shadow);
    if flash.radiance.b > 0.0 {
        lit += cloth_light(n, v, flash.l, t, b, base_color, metal, rough, f0, anisotropy, sheen,
            sheen_roughness, under) * flash.radiance * flash.visible
            + lightning_fill(world_position, n, base_color * (1.0 - metal), flash);
    }
    let lamps = tile_lamps(pixel);
    var slot = 0u;
    for (var i = 0u; i < lamps.count; i++) {
        let lamp = lamp_arrival(lamps, i, slot, world_position, geometric, pixel, solid);
        slot += lamp_slots(lamps, i);
        if lamp.visible > 0.0 {
            lit += cloth_light(n, v, lamp.l, t, b, base_color, metal, rough, f0, anisotropy,
                sheen, sheen_roughness, under) * lamp.radiance * lamp.visible
                * bumps(bumped, geometric, n, lamp.l);
        }
    }

    // The sky: the layer under the sheen as Standard has it (its
    // reflection bent toward the stretched highlight's direction), and
    // the sheen's share of the sky round the normal.
    var reflect_n = n;
    if anisotropy != 0.0 {
        let stretched = select(b, t, anisotropy > 0.0);
        let across = cross(stretched, v);
        let bent = cross(across, stretched);
        if dot(bent, bent) > 1e-8 {
            reflect_n = normalize(mix(n, normalize(bent), abs(anisotropy) * (1.0 - rough) * 0.5));
        }
    }
    let environment = frame.sky_top.w;
    let reflectance = ambient_specular(f0, rough, n_dot_v);
    let diffuse_color = base_color * (1.0 - metal);
    let sky = surrounding_diffuse(world_position, n, environment);
    let sheen_sky = sheen * cloth_sheen_albedo(n_dot_v, sheen_roughness);
    let ambient = occlusion * (
        under * (frame.ambient.rgb * (diffuse_color + reflectance)
            + sky * diffuse_color
            + traced_reflected(world_position, reflect(-v, reflect_n), rough, environment, pixel,
                solid) * reflectance)
        + (frame.ambient.rgb + sky) * sheen_sky);
    return lit + ambient;
}

// One light on cloth: the layer under the sheen, dimmed by what the sheen
// takes (`under`), and the sheen, times the cosine of the angle of
// arrival.
fn cloth_light(
    n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, t: vec3<f32>, b: vec3<f32>,
    base_color: vec3<f32>, metalness: f32, roughness: f32, f0: vec3<f32>, anisotropy: f32,
    sheen: vec3<f32>, sheen_roughness: f32, under: f32,
) -> vec3<f32> {
    let base = cloth_base(n, v, l, t, b, base_color, metalness, roughness, f0, anisotropy);
    if all(sheen == vec3<f32>(0.0)) {
        return base;
    }
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0001);
    let h = normalize(v + l);
    let lobe = cloth_charlie(max(dot(n, h), 0.0), sheen_roughness)
        * cloth_neubelt(n_dot_v, n_dot_l);
    return base * under + sheen * lobe * n_dot_l;
}
