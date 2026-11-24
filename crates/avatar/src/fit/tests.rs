//! Fitting on a made-up body: a closed tube with two bones, a shape that
//! fattens it, a corrective that bulges one side and a mask shape; a
//! sleeve round its middle, partly sunk into it.

use std::f32::consts::TAU;
use std::sync::Arc;

use axcenity_assets::{Bone, BoneTransform, Joint, Rig};
use glam::{Mat4, Vec3};

use super::*;

const RADIUS: f32 = 0.15;
const SIDES: usize = 48;
const RINGS: usize = 41;

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The bulge of `Corrective_Bump` at height `y` and angle `angle`.
fn bump(y: f32, angle: f32) -> f32 {
    0.03 * (-((y - 0.5) / 0.06).powi(2)).exp() * angle.cos().max(0.0).powi(2)
}

/// A closed tube 1 m tall: its sides (rings of `SIDES` corners, the
/// texture's U round it and V down it) and two flat caps.
pub(super) fn tube_body() -> BodyMesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut triangles = Vec::new();
    for ring in 0..RINGS {
        let y = ring as f32 / (RINGS - 1) as f32;
        for side in 0..=SIDES {
            let angle = side as f32 / SIDES as f32 * TAU;
            let out = Vec3::new(angle.cos(), 0.0, angle.sin());
            positions.push(out * RADIUS + Vec3::Y * y);
            normals.push(out);
            uvs.push([side as f32 / SIDES as f32, 1.0 - y]);
        }
    }
    let row = SIDES + 1;
    for ring in 0..RINGS - 1 {
        for side in 0..SIDES {
            let a = (ring * row + side) as u32;
            let b = a + 1;
            let c = a + row as u32;
            let d = c + 1;
            // Counter-clockwise seen from outside.
            triangles.push([a, c, b]);
            triangles.push([b, c, d]);
        }
    }
    for (y, normal) in [(0.0f32, -Vec3::Y), (1.0, Vec3::Y)] {
        let centre = positions.len() as u32;
        positions.push(Vec3::Y * y);
        // The caps are mapped onto the band nearest them.
        let v = 1.0 - y.clamp(0.01, 0.99);
        normals.push(normal);
        uvs.push([0.5, v]);
        let first = positions.len() as u32;
        for side in 0..SIDES {
            let angle = side as f32 / SIDES as f32 * TAU;
            positions.push(Vec3::new(angle.cos(), 0.0, angle.sin()) * RADIUS + Vec3::Y * y);
            normals.push(normal);
            uvs.push([0.5, v]);
        }
        for side in 0..SIDES as u32 {
            let (a, b) = (first + side, first + (side + 1) % SIDES as u32);
            if y == 0.0 {
                triangles.push([centre, a, b]);
            } else {
                triangles.push([centre, b, a]);
            }
        }
    }
    let joints = vec![[0u16, 1, 0, 0]; positions.len()];
    let weights = positions
        .iter()
        .map(|p| {
            let chest = smoothstep(0.4, 0.6, p.y);
            [1.0 - chest, chest, 0.0, 0.0]
        })
        .collect();
    let radial = |p: &Vec3| Vec3::new(p.x, 0.0, p.z).normalize_or_zero();
    let shape = |name: &str, offset: &dyn Fn(&Vec3) -> Vec3| BodyShape {
        name: name.to_owned(),
        corners: (0..positions.len() as u32).collect(),
        offsets: positions.iter().map(offset).collect(),
        normals: vec![Vec3::ZERO; positions.len()],
    };
    let shapes = vec![
        shape("Fat", &|p| radial(p) * 0.02),
        shape("Corrective_Bump", &|p| {
            radial(p) * bump(p.y, p.z.atan2(p.x))
        }),
        shape("Mask_Under", &|p| radial(p) * -0.005),
    ];
    let rest = |y: f32| BoneTransform {
        translation: Vec3::Y * y,
        ..BoneTransform::IDENTITY
    };
    let rig = Rig {
        bones: vec![
            Bone {
                name: "Hips".to_owned(),
                parent: None,
                rest: rest(0.0),
            },
            Bone {
                name: "Chest".to_owned(),
                parent: Some(0),
                rest: rest(0.5),
            },
        ],
        joints: vec![
            Joint {
                bone: 0,
                inverse_bind: Mat4::IDENTITY,
            },
            Joint {
                bone: 1,
                inverse_bind: Mat4::from_translation(-Vec3::Y * 0.5),
            },
        ],
        fit: Mat4::IDENTITY,
    };
    BodyMesh {
        positions,
        normals,
        uvs,
        triangles,
        joints,
        weights,
        shapes,
        rig: Arc::new(rig),
    }
}

/// An open sleeve of `radius` from 0.3 m to 0.7 m, sunk to 0.148 m (into
/// the body) on its -X side when `dented`.
fn sleeve(radius: f32, dented: bool) -> GarmentMesh {
    let (sides, rings) = (40usize, 17usize);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for ring in 0..rings {
        let y = 0.3 + 0.4 * ring as f32 / (rings - 1) as f32;
        for side in 0..sides {
            let angle = side as f32 / sides as f32 * TAU;
            let out = Vec3::new(angle.cos(), 0.0, angle.sin());
            let dent = if dented && angle.cos() < -0.9 && (y - 0.5).abs() < 0.08 {
                0.148
            } else {
                radius
            };
            positions.push(out * dent + Vec3::Y * y);
            normals.push(out);
            uvs.push([side as f32 / sides as f32, y]);
        }
    }
    let mut indices = Vec::new();
    for ring in 0..rings - 1 {
        for side in 0..sides {
            let a = (ring * sides + side) as u32;
            let b = (ring * sides + (side + 1) % sides) as u32;
            let c = a + sides as u32;
            let d = b + sides as u32;
            indices.extend([a, c, b, b, c, d]);
        }
    }
    GarmentMesh {
        pieces: vec![GarmentPiece {
            material: "Cotton".to_owned(),
            colour: [0.2, 0.3, 0.6, 1.0],
            roughness: 0.8,
            positions,
            normals,
            uvs,
            indices,
        }],
    }
}

fn options() -> FitOptions {
    FitOptions {
        shapes: vec!["Fat".to_owned(), "Corrective_Bump".to_owned()],
        extremes: vec![
            vec![("Fat".to_owned(), 1.0), ("Corrective_Bump".to_owned(), 1.0)],
            vec![("Fat".to_owned(), -0.5)],
        ],
        ..FitOptions::default()
    }
}

/// How far each of `points` is outside `surface` (along its normal at
/// the closest point).
fn clearances(surface: &Surface, points: &[Vec3]) -> Vec<f32> {
    points
        .iter()
        .map(|p| surface.closest(*p, 1.0).expect("near").signed(*p))
        .collect()
}

fn least(values: &[f32]) -> f32 {
    values.iter().copied().fold(f32::INFINITY, f32::min)
}

#[test]
fn the_closest_point_is_the_one_every_triangle_agrees_on() {
    let body = tube_body();
    let surface = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    let mut seed = 7u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for _ in 0..300 {
        let p =
            Vec3::new(next() - 0.5, next() * 1.2 - 0.1, next() - 0.5) * Vec3::new(0.6, 1.0, 0.6);
        let hit = surface.closest(p, 10.0).expect("a hit");
        let brute = body
            .triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|v| body.positions[v as usize]);
                closest_on_triangle(p, a, b, c).0.distance(p)
            })
            .fold(f32::INFINITY, f32::min);
        assert!(
            (hit.distance - brute).abs() < 1e-5,
            "{p}: {} against {brute}",
            hit.distance
        );
    }
}

#[test]
fn a_binding_puts_every_vertex_back_where_it_was() {
    let body = tube_body();
    let garment = sleeve(0.16, false).positions();
    let surface = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    let binding = bind(&surface, &garment, &[]);
    assert_eq!(binding.vertices.len(), garment.len());
    let placed = place(&binding, &body.positions, &body.normals, &body.triangles);
    for (a, b) in placed.iter().zip(&garment) {
        assert!(a.distance(*b) < 1e-5, "{a} against {b}");
    }
    // Stored and read back, it is the same binding.
    let read = Binding::from_json(&binding.to_json()).unwrap();
    assert_eq!(read.vertices.len(), binding.vertices.len());
    assert_eq!(read.vertices[17].triangle, binding.vertices[17].triangle);
}

#[test]
fn a_garment_takes_the_bodys_weights_at_its_bound_points_and_they_add_up_to_one() {
    let body = tube_body();
    let fitted = fit_garment(&body, &sleeve(0.16, false), &options()).unwrap();
    for ((joints, weights), p) in fitted
        .joints
        .iter()
        .zip(&fitted.weights)
        .zip(fitted.garment.positions())
    {
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "{weights:?}");
        let chest: f32 = joints
            .iter()
            .zip(weights)
            .filter(|(j, _)| **j == 1)
            .map(|(_, w)| w)
            .sum();
        // The tube's own weight at that height, give or take a ring.
        let expected = smoothstep(0.4, 0.6, p.y);
        assert!(
            (chest - expected).abs() < 0.08,
            "at {p}: {chest} against {expected}"
        );
    }
}

#[test]
fn a_garment_inside_the_body_is_pushed_out_by_its_margin_at_rest() {
    let body = tube_body();
    let fitted = fit_garment(&body, &sleeve(0.153, true), &options()).unwrap();
    let rest = body.surface(&[]);
    let clear = least(&clearances(&rest, &fitted.garment.positions()));
    assert!(clear > 0.002 - 2e-4, "{clear} m clear");
    let pushed = &fitted.report.layers[0];
    assert_eq!(pushed.layer, "body");
    // The dent is 2 mm inside a 0.15 m tube drawn with flat sides: about
    // 4 mm to its margin.
    assert!(pushed.pushed > 0 && pushed.most > 0.0035, "{pushed:?}");
    // The push spreads: next to the dent, it is smooth.
    assert!(fitted.report.inside_most <= 1e-4, "{:?}", fitted.report);
}

#[test]
fn carried_shapes_follow_the_body_and_clear_it_at_full_weight() {
    let body = tube_body();
    let fitted = fit_garment(&body, &sleeve(0.153, false), &options()).unwrap();
    let names: Vec<&str> = fitted.shapes.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Fat", "Corrective_Bump"]);
    assert_eq!(fitted.report.shapes_carried, 2);
    // Fat moves the sleeve out with the skin, about 2 cm.
    let fat = &fitted.shapes[0];
    let rest = fitted.garment.positions();
    for (p, o) in rest.iter().zip(&fat.offsets) {
        let out = o.dot(Vec3::new(p.x, 0.0, p.z).normalize());
        assert!((out - 0.02).abs() < 0.003, "{out}");
    }
    // At each shape's full weight it is still clear of the body.
    for shape in &fitted.shapes {
        let surface = body.surface(&[(shape.name.clone(), 1.0)]);
        let moved: Vec<Vec3> = rest
            .iter()
            .zip(&shape.offsets)
            .map(|(p, o)| *p + *o)
            .collect();
        let clear = least(&clearances(&surface, &moved));
        assert!(clear > 0.002 - 2e-4, "{}: {clear}", shape.name);
    }
    // And at the extremes.
    for extreme in &options().extremes {
        let surface = body.surface(extreme);
        let moved = transfer::garment_at(&rest, &fitted.shapes, extreme);
        let clear = least(&clearances(&surface, &moved));
        assert!(clear > 0.002 - 2e-4, "{extreme:?}: {clear}");
    }
}

#[test]
fn mask_shapes_are_never_carried() {
    assert!(is_mask_shape("Mask_Underwear_Bra"));
    assert!(!is_mask_shape("Masculine_Fat_Up"));
}

#[test]
fn an_outer_layer_stays_outside_the_lower_one_at_rest_and_at_every_extreme() {
    let body = tube_body();
    let inner = fit_garment(&body, &sleeve(0.152, false), &options()).unwrap();
    let mut outer_options = options();
    outer_options.under = vec![Layer::of_fitted("inner", &inner)];
    // Made as tight as the inner one: it must be pushed out over it.
    let outer = fit_garment(&body, &sleeve(0.152, false), &outer_options).unwrap();
    let layer = &outer.report.layers[1];
    assert_eq!(layer.layer, "inner");
    assert!(layer.pushed > 0, "{layer:?}");
    let states: Vec<Vec<(String, f32)>> = std::iter::once(Vec::new())
        .chain(outer_options.shapes.iter().map(|s| vec![(s.clone(), 1.0)]))
        .chain(outer_options.extremes.iter().cloned())
        .collect();
    for state in &states {
        let under = transfer::garment_at(&inner.garment.positions(), &inner.shapes, state);
        let surface = Surface::new(under, inner.garment.normals(), inner.garment.triangles());
        let over = transfer::garment_at(&outer.garment.positions(), &outer.shapes, state);
        let clear = least(&clearances(&surface, &over));
        assert!(
            clear > 0.002 - 3e-4,
            "{state:?}: {clear} m over the lower layer"
        );
    }
}

#[test]
fn coverage_finds_the_regions_a_garment_covers_whole() {
    let body = tube_body();
    let fitted = fit_garment(&body, &sleeve(0.16, false), &FitOptions::default()).unwrap();
    // Five bands of 0.2 m down the tube's texture: regions 1 to 5 from
    // the top.
    let map = RegionMap {
        width: 1,
        height: 5,
        texels: vec![1, 2, 3, 4, 5],
    };
    let regions = triangle_regions(&body, &map);
    let surface = Surface::new(
        fitted.garment.positions(),
        fitted.garment.normals(),
        fitted.garment.triangles(),
    );
    let covered = coverage(&body, &regions, &surface);
    // 0.4 to 0.6 m (region 3) is wholly inside the sleeve's 0.3 to 0.7.
    assert!(covered.share(3) >= HIDE_SHARE, "{covered:?}");
    assert!(
        covered.share(2) > 0.2 && covered.share(2) < 0.9,
        "{covered:?}"
    );
    assert!(covered.share(1) < 0.05, "{covered:?}");
}

/// ADR-0045: a garment's cover map lights the skin under it, set back
/// from its openings, less for cloth worn beneath it than for the skin;
/// and a garment's corners find their place on that map.
#[test]
fn a_cover_map_lights_the_skin_under_a_garment_set_back_from_its_openings() {
    let body = tube_body();
    // A sleeve from 0.3 to 0.7 m, two centimetres off the skin.
    let fitted = fit_garment(&body, &sleeve(0.17, false), &FitOptions::default()).unwrap();
    let surface = Surface::new(
        fitted.garment.positions(),
        fitted.garment.normals(),
        fitted.garment.triangles(),
    );
    // Every triangle has a region (none is the head's).
    let regions = vec![1u8; body.triangles.len()];
    let (map, report) = cover_map(
        &body,
        &regions,
        &surface,
        (&fitted.joints, &fitted.weights),
        None,
        256,
    );
    assert_eq!(map.rgba.len(), 256 * 256 * 4);
    assert!(report.covered > 0);
    assert!(
        report.skin > 0 && report.skin < report.cloth && report.cloth < report.covered,
        "{report:?}"
    );
    // The tube's texture runs down it: V is 1 - height.
    let at = |height: f32, channel: usize| {
        let y = (((1.0 - height) * 256.0) as usize).min(255);
        map.rgba[(y * 256 + 128) * 4 + channel]
    };
    // Its middle is hidden, skin and cloth.
    assert_eq!((at(0.5, 0), at(0.5, 1)), (255, 255));
    // Past its ends nothing is.
    for height in [0.2, 0.8] {
        assert_eq!((at(height, 0), at(height, 1)), (0, 0), "at {height}");
    }
    // Just inside an opening (three and a half centimetres up from the
    // hem, the cloth two off): the skin is still drawn, since it would
    // be seen under the hem; cloth worn beneath is already left out.
    assert_eq!(at(0.335, 0), 0, "skin by the hem stays");
    assert_eq!(at(0.335, 1), 255, "cloth by the hem goes");
    // Covered at all (blue) right up to the hem.
    assert_eq!(at(0.31, 2), 255);

    // Cloth that doesn't move with the skin under it hides none of the
    // skin (the same sleeve following a bone the skin there doesn't),
    // though it still hides cloth worn beneath it.
    let elsewhere = vec![[7u16, 0, 0, 0]; fitted.joints.len()];
    let (_, loose) = cover_map(
        &body,
        &regions,
        &surface,
        (&elsewhere, &fitted.weights),
        None,
        256,
    );
    assert_eq!(loose.skin, 0);
    assert_eq!((loose.covered, loose.cloth), (report.covered, report.cloth));

    // Tried in poses, the same cloth hides the skin it stays over: all
    // of it in a pose that moves nothing, whatever bone it follows.
    let lower = vec![[0u16, 0, 0, 0]; fitted.joints.len()];
    let whole = vec![[1.0f32, 0.0, 0.0, 0.0]; fitted.joints.len()];
    let rest = fitted.garment.positions();
    let cloth_at = |_: &[(String, f32)]| rest.clone();
    let still = FitPose {
        name: "still".to_owned(),
        bones: body.rig.rest_pose(),
        correctives: Vec::new(),
    };
    let (_, proved) = cover_map(
        &body,
        &regions,
        &surface,
        (&lower, &whole),
        Some(&Proof {
            poses: std::slice::from_ref(&still),
            states: &[],
            cloth_at: &cloth_at,
        }),
        256,
    );
    assert_eq!(proved, report);
    // In a pose that lifts the body's upper half out of the sleeve (the
    // sleeve staying with the lower half), the skin that comes out from
    // under it stays drawn, and the skin still under it is hidden.
    let mut lifted = still.clone();
    lifted.name = "lifted".to_owned();
    lifted.bones[body.rig.joints[1].bone].translation.y += 0.3;
    let (map, torn) = cover_map(
        &body,
        &regions,
        &surface,
        (&lower, &whole),
        Some(&Proof {
            poses: &[still, lifted],
            states: &[],
            cloth_at: &cloth_at,
        }),
        256,
    );
    let at = |height: f32, channel: usize| {
        let y = (((1.0 - height) * 256.0) as usize).min(255);
        map.rgba[(y * 256 + 128) * 4 + channel]
    };
    assert!(torn.skin > 0 && torn.skin < report.skin, "{torn:?}");
    assert_eq!(at(0.42, 0), 255, "skin that stays under the sleeve goes");
    assert_eq!(at(0.6, 0), 0, "skin lifted out of the sleeve stays");
    // The cloth worn beneath is hidden as before.
    assert_eq!(torn.cloth, report.cloth);

    // Each of the garment's corners lies over the skin at its own height.
    let uvs = skin_uvs(&body, &fitted.binding);
    assert_eq!(uvs.len(), fitted.garment.len());
    for (uv, p) in uvs.iter().zip(fitted.garment.positions()) {
        assert!((1.0 - uv[1] - p.y).abs() < 0.02, "{uv:?} for {p:?}");
    }
}

/// A web (cloth joined across a joint, drawn out when it bends) comes out
/// of a garment when the fit is asked to cut them, and only then.
#[test]
fn a_web_drawn_out_by_a_pose_is_cut_when_asked() {
    let body = tube_body();
    // The tube bent at its upper bone: the sleeve's corners follow one
    // bone or the other, and the cloth between them is drawn out.
    let bent = {
        let mut bones = body.rig.rest_pose();
        let last = bones.len() - 1;
        bones[last].translation += Vec3::new(0.6, 0.0, 0.0);
        FitPose {
            name: "pulled".to_owned(),
            bones,
            correctives: Vec::new(),
        }
    };
    let whole = fit_garment(&body, &sleeve(0.16, false), &FitOptions::default()).unwrap();
    assert_eq!(whole.report.webs_cut, 0);
    let cut = fit_garment(
        &body,
        &sleeve(0.16, false),
        &FitOptions {
            poses: vec![bent],
            cut_webs: 2.5,
            ..FitOptions::default()
        },
    )
    .unwrap();
    assert!(cut.report.webs_cut > 0, "{:?}", cut.report.webs_cut);
    assert_eq!(
        cut.garment.triangles().len() + cut.report.webs_cut,
        whole.garment.triangles().len()
    );
    // Its corners are all still there: the weights and shapes still fit.
    assert_eq!(cut.garment.len(), whole.garment.len());
}

/// A grid of corners, `profile` (x, y) extruded along Z over `zs`, with
/// triangles facing up (+Y) where the profile goes +X: a strip of skin or
/// a sheet of cloth.
fn extruded(profile: &[(f32, f32)], zs: &[f32]) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let mut positions = Vec::new();
    for &(x, y) in profile {
        for &z in zs {
            positions.push(Vec3::new(x, y, z));
        }
    }
    let row = zs.len() as u32;
    let mut triangles = Vec::new();
    for i in 0..profile.len() as u32 - 1 {
        for j in 0..row - 1 {
            let a = i * row + j;
            let (b, c, d) = (a + 1, a + row, a + row + 1);
            // (Z step) × (X step) faces up.
            triangles.push([a, b, c]);
            triangles.push([b, d, c]);
        }
    }
    (positions, triangles)
}

/// From `a` to `b` in steps of about 1 cm (both ends included).
fn steps(a: f32, b: f32) -> Vec<f32> {
    let n = ((b - a).abs() / 0.01).ceil().max(1.0) as usize;
    (0..=n).map(|k| a + (b - a) * k as f32 / n as f32).collect()
}

/// Skin with a narrow crease along Z (two walls 6 cm apart at the top,
/// 10 cm deep, facing each other, as an arm's underside and the chest's
/// side do at an armpit). Region 1 is the top, region 2 the crease.
fn creased_skin() -> (BodyMesh, Vec<u8>) {
    let mut profile: Vec<(f32, f32)> = steps(-0.2, -0.03).into_iter().map(|x| (x, 0.1)).collect();
    profile.pop();
    profile.extend(steps(0.1, 0.0).into_iter().map(|y| (-0.3 * y, y)));
    profile.pop();
    profile.extend(steps(0.0, 0.1).into_iter().map(|y| (0.3 * y, y)));
    profile.pop();
    profile.extend(steps(0.03, 0.2).into_iter().map(|x| (x, 0.1)));
    let (positions, triangles) = extruded(&profile, &steps(-0.2, 0.2));
    let normals = vertex_normals(&positions, &triangles);
    let regions = triangles
        .iter()
        .map(|t| {
            let middle = t.iter().map(|v| positions[*v as usize]).sum::<Vec3>() / 3.0;
            if middle.y < 0.099 { 2 } else { 1 }
        })
        .collect();
    let mut body = tube_body();
    body.uvs = Vec::new();
    body.joints = vec![[0; 4]; positions.len()];
    body.weights = vec![[1.0, 0.0, 0.0, 0.0]; positions.len()];
    body.shapes = Vec::new();
    body.positions = positions;
    body.normals = normals;
    body.triangles = triangles;
    (body, regions)
}

/// A flat sheet of cloth 1 cm over the creased skin's top, wider and
/// longer than it (bridging the crease, as a sleeve's underarm does), less
/// the triangles whose middle `hole` takes out.
fn sheet(hole: impl Fn(Vec3) -> bool) -> Surface {
    let profile: Vec<(f32, f32)> = steps(-0.25, 0.25).into_iter().map(|x| (x, 0.11)).collect();
    let (positions, triangles) = extruded(&profile, &steps(-0.25, 0.25));
    let triangles: Vec<[u32; 3]> = triangles
        .into_iter()
        .filter(|t| !hole(t.iter().map(|v| positions[*v as usize]).sum::<Vec3>() / 3.0))
        .collect();
    let normals = vertex_normals(&positions, &triangles);
    Surface::new(positions, normals, triangles)
}

/// A crease the garment closes over (an armpit under a sleeve) is under
/// it, though no cloth lies along its walls' normals (each faces the
/// other wall); a hole in the garment over plain skin still leaves that
/// skin uncovered.
#[test]
fn coverage_counts_a_crease_the_garment_closes_over() {
    let (body, regions) = creased_skin();
    let covered = coverage(&body, &regions, &sheet(|_| false));
    assert!(covered.share(1) >= HIDE_SHARE, "{covered:?}");
    assert!(covered.share(2) >= HIDE_SHARE, "{covered:?}");
    // A 6 × 8 cm hole over the top's skin (away from the crease).
    let holed = sheet(|m| (0.08..0.16).contains(&m.x) && (-0.04..0.04).contains(&m.z));
    let covered = coverage(&body, &regions, &holed);
    assert!(covered.share(1) < 0.97, "{covered:?}");
    assert!(covered.share(2) >= HIDE_SHARE, "{covered:?}");
}

#[test]
fn a_fitted_garment_written_as_gltf_loads_with_its_skeleton_weights_and_shapes() {
    let body = tube_body();
    let fitted = fit_garment(&body, &sleeve(0.153, true), &options()).unwrap();
    let bytes = write_fitted_glb(&FittedModel {
        fitted: &fitted,
        rig: &body.rig,
        armature: "Armature",
        object: "Sleeve",
        shading: "cloth",
        maps: &[],
        skin_uvs: &[],
        least_move: 0.0,
    });
    let model = axcenity_assets::load_model(
        &bytes,
        std::path::Path::new("."),
        axcenity_assets::ModelFormat::Gltf,
    )
    .expect("it loads");
    let names: Vec<&str> = model
        .morph_targets
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, ["Fat", "Corrective_Bump"]);
    let rig = model.rig.as_ref().expect("skinned");
    let bones: Vec<&str> = rig.bones.iter().map(|b| b.name.as_str()).collect();
    assert!(
        bones.contains(&"Hips") && bones.contains(&"Chest"),
        "{bones:?}"
    );
    // Read back as a body would be, it is where it was fitted, with the
    // same shapes and weights.
    let read = BodyMesh::from_model(&model).unwrap();
    let rest = fitted.garment.positions();
    assert_eq!(read.positions.len(), rest.len());
    for (a, b) in read.positions.iter().zip(&rest) {
        assert!(a.distance(*b) < 1e-4, "{a} against {b}");
    }
    for (shape, carried) in read.shapes.iter().zip(&fitted.shapes) {
        let dense = shape.dense(rest.len());
        for (a, b) in dense.iter().zip(&carried.offsets) {
            assert!(a.distance(*b) < 1e-4, "{}: {a} against {b}", shape.name);
        }
    }
    for (w, fitted) in read.weights.iter().zip(&fitted.weights) {
        let sum: f32 = w.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
        assert!((w[0] - fitted[0]).abs() < 1e-4);
    }
}

#[test]
fn a_thumbnail_draws_the_garment_on_a_clear_background() {
    let picture = thumbnail(&sleeve(0.16, false), 64);
    assert_eq!(picture.len(), 64 * 64 * 4);
    let drawn = picture.chunks_exact(4).filter(|px| px[3] == 255).count();
    let share = drawn as f32 / (64.0 * 64.0);
    assert!(share > 0.3 && share < 0.95, "{share}");
}

#[test]
fn alignment_brings_a_garment_in_centimetres_and_off_centre_onto_the_body() {
    let body = tube_body();
    let original = sleeve(0.16, false);
    let mut moved = original.clone();
    moved.transform(100.0, Vec3::new(4.0, 0.0, -7.0));
    let alignment = align(&mut moved, &body);
    assert!((alignment.scale - 0.01).abs() < 1e-6, "{alignment:?}");
    for (a, b) in moved.positions().iter().zip(original.positions()) {
        assert!(a.distance(b) < 1e-3, "{a} against {b}");
    }
}

/// A ring round the tube's foot, `height` tall and a little wider than
/// the tube: what a pair of shoes (or socks) is to a body.
fn foot_ring(height: f32) -> GarmentMesh {
    let mut garment = sleeve(0.17, false);
    for piece in &mut garment.pieces {
        for p in &mut piece.positions {
            // 0.3..0.7 m down to 0..height.
            p.y = (p.y - 0.3) / 0.4 * height;
        }
    }
    garment
}

#[test]
fn a_vertex_far_from_the_body_is_bound_to_its_closest_point() {
    let body = tube_body();
    let surface = body.surface(&[]);
    // A long coat's hem or a cape stands well off the body: half a metre
    // out from the tube's side, and a metre and a half below its foot.
    let far = [Vec3::new(RADIUS + 0.5, 0.5, 0.0), Vec3::new(0.0, -1.5, 0.0)];
    let binding = bind(&surface, &far, &[]);
    let placed = place(&binding, &body.positions, &body.normals, &body.triangles);
    for (a, b) in placed.iter().zip(&far) {
        assert!(a.distance(*b) < 1e-4, "{a} against {b}");
    }
    let hit = surface.closest(far[0], f32::MAX).expect("a closest point");
    assert!((hit.distance - 0.5).abs() < 2e-3, "{hit:?}");
    assert!(surface.closest(far[1], f32::INFINITY).is_some());
}

#[test]
fn alignment_finds_the_units_of_short_garments() {
    let body = tube_body();
    // Shoes 12 cm tall, socks 30 cm tall: in centimetres, millimetres and
    // inches.
    for (height, scale) in [
        (0.12, 0.01),
        (0.12, 0.001),
        (0.30, 0.001),
        (0.30, 0.01),
        (0.12, 0.0254),
        (0.30, 1.0),
    ] {
        let original = foot_ring(height);
        let mut moved = original.clone();
        moved.transform(1.0 / scale, Vec3::ZERO);
        let alignment = align(&mut moved, &body);
        assert!(
            (alignment.scale - scale).abs() < 1e-6 * scale.max(1.0) + 0.02 * scale,
            "{height} m made in units of {scale} m: scaled by {}",
            alignment.scale
        );
        for (a, b) in moved.positions().iter().zip(original.positions()) {
            assert!(a.distance(b) < 0.01, "{a} against {b}");
        }
    }
}

#[test]
fn corners_moved_by_the_push_out_take_the_normals_of_where_they_are() {
    let body = tube_body();
    let mut dented = sleeve(0.153, true);
    // A dent 1.5 cm deep (into the 0.15 m tube), with normals that agree
    // with its shape: tilted round its edges.
    for p in &mut dented.pieces[0].positions {
        let across = Vec3::new(p.x, 0.0, p.z);
        if across.length() < 0.15 {
            *p = across.normalize() * 0.135 + Vec3::Y * p.y;
        }
    }
    let triangles = dented.triangles();
    let normals = vertex_normals(&dented.positions(), &triangles);
    dented.pieces[0].normals = normals.clone();
    let fitted = fit_garment(&body, &dented, &options()).unwrap();
    let now = vertex_normals(&fitted.garment.positions(), &triangles);
    let moved: Vec<usize> = dented
        .positions()
        .iter()
        .zip(fitted.garment.positions())
        .enumerate()
        .filter(|(_, (a, b))| a.distance(*b) > 0.001)
        .map(|(k, _)| k)
        .collect();
    assert!(!moved.is_empty());
    let mut changed = 0;
    for &k in &moved {
        let written = fitted.garment.pieces[0].normals[k];
        assert!(
            written.angle_between(now[k]) < 5f32.to_radians(),
            "corner {k}: written {written}, its surface's {}",
            now[k]
        );
        if normals[k].angle_between(now[k]) > 10f32.to_radians() {
            changed += 1;
        }
    }
    // The dent's edges really were tilted, so the test sees a change.
    assert!(changed > 0, "no normal had to change");
    // Corners nothing moved, with nothing moved round them, keep the
    // normals they came with.
    let before = dented.positions();
    let after = fitted.garment.positions();
    let moved_at: Vec<Vec3> = before
        .iter()
        .zip(&after)
        .filter(|(a, b)| a.distance(**b) > 1e-7)
        .map(|(a, _)| *a)
        .collect();
    let mut kept = 0;
    for (k, p) in before.iter().enumerate() {
        if moved_at.iter().all(|m| m.distance(*p) > 0.05) {
            let written = fitted.garment.pieces[0].normals[k];
            assert!(written.distance(normals[k]) < 1e-4, "corner {k}");
            kept += 1;
        }
    }
    assert!(kept > 100, "{kept}");
}
