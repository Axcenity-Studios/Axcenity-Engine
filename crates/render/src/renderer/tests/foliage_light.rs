//! Foliage under an overcast sky (2026-11-26, the Overgrown Street
//! showcase, `lit_leaf` in shader.wgsl): a crown's leaves keep most of the
//! sky's light that screen-space ambient occlusion would take from a solid
//! ball of the same shape, the sky comes through a leaf from its far side (a
//! crown seen from below glows, so does a low plant with the sky behind it),
//! never more than the leaf doesn't reflect, a low plant is darker toward
//! its foot, and small things standing on the ground darken the ground at
//! their foot.

use std::sync::Arc;

use glam::Vec3;

use super::*;
use crate::GraphicsSettings;
use crate::graphics::{AmbientOcclusion, AntiAliasing, Quality};

/// The frame size, square.
const SIZE: u32 = 128;
/// The showcase's `AmbientOcclusionIntensity`.
const OCCLUSION_STRENGTH: f32 = 1.6;

/// An overcast sky (bright and grey above, the ground dark below) and no
/// sun, seen from `eye`.
fn overcast(items: Vec<DrawItem>, eye: Vec3, at: Vec3) -> DrawList {
    let mut list = list_of(items);
    list.sky = Sky {
        top_color: [0.78, 0.8, 0.8],
        horizon_color: [0.72, 0.74, 0.74],
        ground_color: [0.12, 0.13, 0.1],
        environment_intensity: 1.0,
        ..Sky::default()
    };
    list.sun.intensity = 0.0;
    list.shadows.enabled = false;
    list.ambient.occlusion = OCCLUSION_STRENGTH;
    list.camera = Camera {
        position: eye,
        look_at: at,
        fov_y_degrees: 40.0,
    };
    list
}

fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    list: &DrawList,
    occlusion: AmbientOcclusion,
) -> Vec<u8> {
    let mut renderer = TextureRenderer::new(device, queue, SIZE, SIZE);
    renderer.set_graphics(&GraphicsSettings {
        anti_aliasing: Some(AntiAliasing::Off),
        ambient_occlusion: Some(occlusion),
        contact_shadows: Some(false),
        ..GraphicsSettings::preset(Quality::High)
    });
    renderer.render(list).unwrap();
    renderer.read_pixels()
}

/// Where `point` is drawn in a `SIZE`-square frame of `list`, in pixels.
fn on_screen(list: &DrawList, point: Vec3) -> (usize, usize) {
    let clip = view_projection(list, 1.0) * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    (
        ((ndc.x * 0.5 + 0.5) * SIZE as f32) as usize,
        ((0.5 - ndc.y * 0.5) * SIZE as f32) as usize,
    )
}

/// The linear light of the green channel (post-processing is off, so the
/// frame is sRGB of it), averaged over a `half`-pixel-wide square round
/// `middle`.
fn green(pixels: &[u8], middle: (usize, usize), half: usize) -> f32 {
    let mut total = 0.0;
    let mut count = 0.0;
    for y in middle.1 - half..=middle.1 + half {
        for x in middle.0 - half..=middle.0 + half {
            let level = f32::from(pixels[(y * SIZE as usize + x) * 4 + 1]) / 255.0;
            total += linear_color([level; 3]).x;
            count += 1.0;
        }
    }
    total / count
}

/// Cards in the unit cube as one mesh (a tree's leaves are one primitive
/// of its model, so its item's box is the crown's).
struct Cards {
    key: u64,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    tangents: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Cards {
    fn empty(key: u64) -> Self {
        Cards {
            key,
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            tangents: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// A card with these four corners (counter-clockwise seen from
    /// `normal`'s side).
    fn add(&mut self, corners: [Vec3; 4], normal: Vec3) {
        let first = self.positions.len() as u32;
        let across = (corners[1] - corners[0]).normalize();
        for (corner, uv) in
            corners
                .into_iter()
                .zip([[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]])
        {
            self.positions.push(corner.to_array());
            self.normals.push(normal.to_array());
            self.uvs.push(uv);
            self.tangents.push([across.x, across.y, across.z, 1.0]);
        }
        self.indices
            .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }

    /// `count` square cards `2 × half` wide scattered through the ball
    /// filling the cube, each facing its own way: a crown of leaves.
    fn crown(count: usize, half: f32) -> Self {
        let mut state = 0x9e37_79b9_u32;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as f32 / u32::MAX as f32 * 2.0 - 1.0
        };
        let mut crown = Cards::empty(0xc0_5e);
        while crown.positions.len() < count * 4 {
            let middle = Vec3::new(next(), next(), next()) * (0.5 - half);
            if middle.length() > 0.5 - half {
                continue;
            }
            let normal = Vec3::new(next(), next(), next()).normalize_or(Vec3::Y);
            let across = normal.any_orthonormal_vector() * half;
            let up = normal.cross(across);
            crown.add(
                [
                    middle - across - up,
                    middle + across - up,
                    middle + across + up,
                    middle - across + up,
                ],
                normal,
            );
        }
        crown
    }

    /// A tuft of grass: `blades` thin blades from round its foot leaning
    /// out to its top.
    fn tuft(blades: usize) -> Self {
        let mut tuft = Cards::empty(0x7_0f7);
        for i in 0..blades {
            let angle = i as f32 * 2.399_963;
            let out = Vec3::new(angle.cos(), 0.0, angle.sin());
            let across = Vec3::new(-out.z, 0.0, out.x);
            let foot = out * 0.05 - Vec3::Y * 0.5;
            let tip = out * 0.4 + Vec3::Y * 0.5;
            tuft.add(
                [
                    foot - across * 0.03,
                    foot + across * 0.03,
                    tip + across * 0.01,
                    tip - across * 0.01,
                ],
                out,
            );
        }
        tuft
    }
}

impl MeshSource for Cards {
    fn key(&self) -> u64 {
        self.key
    }
    fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }
    fn normals(&self) -> &[[f32; 3]] {
        &self.normals
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &self.uvs
    }
    fn tangents(&self) -> &[[f32; 4]] {
        &self.tangents
    }
    fn indices(&self) -> &[u32] {
        &self.indices
    }
}

/// A green leaf letting `translucency` through. At 0 it isn't a leaf at
/// all (lit as Standard), so leaves are compared with a leaf letting next
/// to nothing through.
fn leaf(translucency: f32) -> Material {
    Material {
        base_color: [0.25, 0.45, 0.15],
        roughness: 0.8,
        double_sided: true,
        translucency,
        ..Material::default()
    }
}

/// A leaf that lets next to nothing through: lit as a leaf, by its crown.
const BARELY: f32 = 0.001;

/// `mesh` filling a box of `size` at `middle`, in `material`.
fn thing(mesh: Shape, size: Vec3, middle: Vec3, material: Material) -> DrawItem {
    DrawItem {
        transform: Mat4::from_scale_rotation_translation(size, glam::Quat::IDENTITY, middle),
        material,
        casts_shadow: false,
        ..item(mesh, Vec3::ZERO, 1.0)
    }
}

/// A 6-stud crown of 900 leaf cards (0.6 studs across) 12 studs off.
fn crown(material: Material) -> DrawItem {
    thing(
        Shape::Mesh(Arc::new(Cards::crown(900, 0.05))),
        Vec3::splat(6.0),
        CROWN,
        material,
    )
}
const CROWN: Vec3 = Vec3::new(0.0, 0.0, -12.0);
/// Where the crown is seen from: level with it, and from under it.
const BESIDE: Vec3 = Vec3::ZERO;
const BELOW: Vec3 = Vec3::new(0.0, -12.0, -11.0);

/// The middle of the crown (linear green), with ambient occlusion off and
/// on.
fn crown_middle(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    material: Material,
    eye: Vec3,
) -> (f32, f32) {
    let list = overcast(vec![crown(material)], eye, CROWN);
    let at = on_screen(&list, CROWN);
    let off = green(&draw(device, queue, &list, AmbientOcclusion::Off), at, 8);
    let on = green(&draw(device, queue, &list, AmbientOcclusion::High), at, 8);
    (off, on)
}

#[test]
fn a_crowns_leaves_keep_most_of_the_sky_that_screen_space_occlusion_takes_from_solid_cards() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    for (view, eye) in [("beside", BESIDE), ("below", BELOW)] {
        // The same crown of cards that let nothing through (Standard): the
        // prepass sees a heap of solid cards and darkens it as one.
        let (off, on) = crown_middle(&device, &queue, leaf(0.0), eye);
        let solid = on / off;
        eprintln!("{view}, solid cards: {off} off, {on} on ({solid})");
        assert!(
            solid < 0.85,
            "{view}: solid cards darken: {on} against {off}"
        );
        // Leaves: the crown shades its own inside (`CANOPY_DEPTH_SHADE`);
        // screen-space occlusion counts for half its strength
        // (`LEAF_SCREEN_OCCLUSION`). Before (2026-11-26), 0.86 beside, only
        // 0.05 more than the solid cards.
        let (off, on) = crown_middle(&device, &queue, leaf(0.42), eye);
        eprintln!("{view}, leaves: {off} off, {on} on ({})", on / off);
        assert!(
            on > off * 0.82 && on / off > solid + 0.1,
            "{view}: leaves keep their light: {on} against {off}"
        );
        assert!(
            on < off,
            "{view}: and still darken a little: {on} against {off}"
        );
    }
}

#[test]
fn a_crown_seen_from_below_glows_with_the_overcast_sky_coming_through() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let (_, barely) = crown_middle(&device, &queue, leaf(BARELY), BELOW);
    let (_, through) = crown_middle(&device, &queue, leaf(0.42), BELOW);
    eprintln!("from below: {barely} letting nothing through, {through} letting 0.42 through");
    assert!(through > barely * 1.3, "{through} against {barely}");
}

/// An upright card `height` studs tall standing at `foot`, facing the
/// camera level with its middle, the sky behind it: one blade of a tuft or
/// one leaf of a plant, its item's box the plant's. Its linear green at
/// 5%, 50% and 90% of its height.
fn upright_card(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    height: f32,
    translucency: f32,
) -> [f32; 3] {
    let foot = Vec3::new(0.0, 0.0, -2.0 * height);
    let middle = foot + Vec3::Y * (0.5 * height);
    let card = thing(
        Shape::Mesh(Arc::new(Square)),
        Vec3::new(height * 0.6, height, height * 0.6),
        middle,
        leaf(translucency),
    );
    let list = overcast(vec![card], Vec3::Y * (0.5 * height), middle);
    let pixels = draw(device, queue, &list, AmbientOcclusion::Off);
    [0.05, 0.5, 0.9].map(|up| green(&pixels, on_screen(&list, foot + Vec3::Y * (up * height)), 1))
}

#[test]
fn a_low_plant_with_the_sky_behind_it_glows_with_what_comes_through() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // A low plant is lit like the ground (its light lifted up,
    // `LEAF_GROUND_LIFT`), but the sky comes through it from behind: before,
    // that was lifted too, so it let through the ground's dark and a
    // translucent tuft came out darker than one letting nothing through.
    // Its middle and foot have the sky behind them; its top, in the crown's
    // way of seeing it, the inside of the plant (`leaf_canopy`).
    let barely = upright_card(&device, &queue, 1.5, BARELY);
    let through = upright_card(&device, &queue, 1.5, 0.42);
    eprintln!("low card: {barely:?} letting nothing through, {through:?} letting 0.42 through");
    for (part, at) in [("foot", 0), ("middle", 1)] {
        assert!(
            through[at] > barely[at] * 1.15,
            "{part}: {through:?} against {barely:?}"
        );
    }
}

#[test]
fn a_low_plant_is_darker_toward_its_foot() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // Its own leaves and stems stand in the way there, and the foot is what
    // meets the ground (`LEAF_FOOT_SHADE_LOW`). Before, 0.8 of its top.
    let [foot, _, top] = upright_card(&device, &queue, 1.5, 0.42);
    eprintln!("low card: foot {foot}, top {top}");
    assert!(foot < top * 0.65, "foot {foot} against top {top}");
    assert!(
        foot > top * 0.3,
        "but not black: foot {foot} against top {top}"
    );
}

#[test]
fn a_leaf_gives_back_no_more_light_than_reaches_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // An even white sky all round and no sun: whatever way a surface faces,
    // as much light reaches each side of it. A white card gives it all back;
    // a pale leaf letting all it can through gives back what it reflects
    // and what comes through, which together must be no more. Before
    // (2026-11-26) it gave back up to half as much again (the green of what
    // comes through is 1.12 times the leaf's own). Seen at the crown's edge
    // (`leaf_canopy`) and mid-height, where neither the crown's inside nor
    // its foot shades it.
    let card = |material: Material| {
        let mut list = list_of(vec![thing(
            Shape::Block,
            Vec3::new(8.0, 8.0, 0.1),
            Vec3::new(0.0, 0.0, -6.0),
            material,
        )]);
        list.sky = Sky {
            top_color: [1.0; 3],
            horizon_color: [1.0; 3],
            ground_color: [1.0; 3],
            environment_intensity: 0.5,
            ..Sky::default()
        };
        list.sun.intensity = 0.0;
        list.shadows.enabled = false;
        let pixels = draw(&device, &queue, &list, AmbientOcclusion::Off);
        green(&pixels, on_screen(&list, Vec3::new(3.7, 0.0, -6.0)), 1)
    };
    let pale = Material {
        base_color: [1.0; 3],
        ..leaf(1.0)
    };
    let barely = Material {
        base_color: [1.0; 3],
        ..leaf(BARELY)
    };
    let white = Material {
        base_color: [1.0; 3],
        roughness: 1.0,
        ..Material::default()
    };
    let (pale, barely, white) = (card(pale), card(barely), card(white));
    eprintln!("a pale leaf {pale} (letting nothing through {barely}), a white card {white}");
    assert!(
        pale > barely,
        "light comes through: {pale} against {barely}"
    );
    assert!(pale <= white * 1.03, "{pale} against {white}");
}

/// A wide grey floor and `things` on it, seen from 6 studs off and 4 up.
fn on_the_ground(things: Vec<DrawItem>) -> DrawList {
    let floor = thing(
        Shape::Block,
        Vec3::new(60.0, 1.0, 60.0),
        Vec3::new(0.0, -0.5, -10.0),
        Material {
            base_color: [0.35; 3],
            roughness: 1.0,
            ..Material::default()
        },
    );
    let mut items = vec![floor];
    items.extend(things);
    overcast(items, Vec3::new(0.0, 4.0, -4.0), Vec3::new(0.0, 0.3, -10.0))
}

const FOOT: Vec3 = Vec3::new(0.0, 0.0, -10.0);

/// The floor's linear green `gaps` studs in front of whatever stands at
/// `FOOT`, with ambient occlusion off and on.
fn floor_in_front(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    standing: DrawItem,
    gaps: &[f32],
) -> Vec<(f32, f32)> {
    let list = on_the_ground(vec![standing]);
    let off = draw(device, queue, &list, AmbientOcclusion::Off);
    let on = draw(device, queue, &list, AmbientOcclusion::High);
    gaps.iter()
        .map(|gap| {
            let at = on_screen(&list, FOOT + Vec3::Z * *gap);
            (green(&off, at, 0), green(&on, at, 0))
        })
        .collect()
}

#[test]
fn small_things_standing_on_the_ground_darken_it_at_their_foot() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let grey = Material {
        base_color: [0.5; 3],
        roughness: 1.0,
        ..Material::default()
    };
    // A post a quarter of a stud thick (a barricade's leg, a tyre seen
    // edge-on): darkest right at its foot, nothing a stud off.
    let post = thing(
        Shape::Block,
        Vec3::new(0.25, 1.5, 0.25),
        FOOT + Vec3::Y * 0.75,
        grey.clone(),
    );
    let floor = floor_in_front(&device, &queue, post, &[0.14, 0.3, 1.0]);
    eprintln!("in front of a post: {floor:?}");
    assert!(floor[0].1 < floor[0].0 * 0.8, "at its foot: {floor:?}");
    assert!(
        floor[1].1 < floor[1].0 * 0.95,
        "a little further: {floor:?}"
    );
    assert!(
        floor[2].1 > floor[2].0 * 0.98,
        "a stud off, none: {floor:?}"
    );

    // A concrete barrier: the crease along its foot.
    let barrier = thing(
        Shape::Block,
        Vec3::new(3.0, 1.2, 1.0),
        FOOT + Vec3::Y * 0.6,
        grey,
    );
    let floor = floor_in_front(&device, &queue, barrier, &[0.55, 1.2]);
    eprintln!("in front of a barrier: {floor:?}");
    assert!(floor[0].1 < floor[0].0 * 0.6, "at its foot: {floor:?}");
    assert!(
        floor[1].1 > floor[1].0 * 0.98,
        "further off, none: {floor:?}"
    );

    // A tuft of grass (cut-out blades are in the prepass) darkens the
    // ground round it a little, its leaves' own softer occlusion
    // (`LEAF_SCREEN_OCCLUSION`) notwithstanding: that's only what leaves
    // take, not what they cast.
    let tuft = thing(
        Shape::Mesh(Arc::new(Cards::tuft(90))),
        Vec3::splat(1.2),
        FOOT + Vec3::Y * 0.6,
        leaf(0.42),
    );
    let floor = floor_in_front(&device, &queue, tuft, &[0.45, 1.2]);
    eprintln!("in front of a tuft: {floor:?}");
    assert!(floor[0].1 < floor[0].0 * 0.95, "beside it: {floor:?}");
    assert!(
        floor[1].1 > floor[1].0 * 0.98,
        "further off, none: {floor:?}"
    );
}

/// The 6-stud crown of leaf cards (`crown`) beside the camera, with the sun
/// right behind the camera, so some cards of a smooth leaf send its
/// highlight straight back. `hangs` sways it from its top as a drape or
/// hanging ivy does (`Sway::hanging`). Its linear green over its middle:
/// the mean, and the brightest pixels' (the top 2%).
fn crown_in_the_sun(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    material: Material,
    hangs: bool,
) -> (f32, f32) {
    let mut item = crown(material);
    if hangs {
        item.sway = Some(crate::Sway::hanging(CROWN.y + 3.0, 6.0, 0.3, 0.0));
    }
    let mut list = overcast(vec![item], BESIDE, CROWN);
    list.sun.intensity = 3.0;
    list.sun.direction = Vec3::Z;
    let pixels = draw(device, queue, &list, AmbientOcclusion::Off);
    let (cx, cy) = on_screen(&list, CROWN);
    let mut greens = Vec::new();
    for y in cy - 16..=cy + 16 {
        for x in cx - 16..=cx + 16 {
            let level = f32::from(pixels[(y * SIZE as usize + x) * 4 + 1]) / 255.0;
            greens.push(linear_color([level; 3]).x);
        }
    }
    greens.sort_by(f32::total_cmp);
    let mean = greens.iter().sum::<f32>() / greens.len() as f32;
    let top = &greens[greens.len() * 98 / 100..];
    (mean, top.iter().sum::<f32>() / top.len() as f32)
}

#[test]
fn a_glossy_leaf_keeps_its_own_sheen_and_a_hanging_one_is_still_a_leaf() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let smooth = |glossy_leaf| Material {
        roughness: 0.1,
        glossy_leaf,
        ..leaf(0.42)
    };
    // A leaf is at least 0.72 rough (seen edge-on, a matte leaf mustn't
    // mirror a white sky), whatever its material says: a glossy one (ivy,
    // `axcenity_glossy_leaf`) keeps its own, and its highlights come back.
    let (_, matte) = crown_in_the_sun(&device, &queue, smooth(false), false);
    let (glossy_mean, glossy) = crown_in_the_sun(&device, &queue, smooth(true), false);
    eprintln!("a 0.1-rough crown's highlights: {matte} as any leaf, {glossy} glossy");
    assert!(glossy > matte * 1.3, "glossy {glossy} against {matte}");
    // Rougher than the least, the flag changes nothing.
    let rough = |glossy_leaf| Material {
        roughness: 0.95,
        glossy_leaf,
        ..leaf(0.42)
    };
    let (plain, _) = crown_in_the_sun(&device, &queue, rough(false), false);
    let (flagged, _) = crown_in_the_sun(&device, &queue, rough(true), false);
    assert!(
        (plain - flagged).abs() <= plain * 0.01,
        "{plain} against {flagged}"
    );

    // Hanging, a leaf is lit as thin cloth (a drape lets light through
    // in its own colours), unless it's a glossy leaf: hanging ivy is lit
    // as the leaf it is.
    let (hanging_glossy, _) = crown_in_the_sun(&device, &queue, smooth(true), true);
    assert!(
        (hanging_glossy - glossy_mean).abs() <= glossy_mean * 0.05,
        "hanging {hanging_glossy} against standing {glossy_mean}"
    );
    let (hanging_plain, _) = crown_in_the_sun(&device, &queue, rough(false), true);
    eprintln!("a hanging crown: {hanging_glossy} glossy, {hanging_plain} plain (standing {plain})");
    assert!(
        (hanging_plain - plain).abs() > plain * 0.05,
        "a hanging plain leaf is cloth: {hanging_plain} against {plain}"
    );
}
