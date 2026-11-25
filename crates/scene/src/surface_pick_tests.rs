//! Refinement R2: rays against the human base's body as it is drawn, posed
//! and skinned with its identity worn live.

use std::path::Path;
use std::sync::OnceLock;

use axcenity_assets::{
    Identity, IdentityLayer, Model, ModelFormat, ModelShading, Pick, Posed, load_models,
};
use axcenity_render::glam::{Mat4, Quat, Vec3};

use super::*;

/// The human base's body, loaded once for every test here.
fn body() -> &'static Model {
    static BODY: OnceLock<Model> = OnceLock::new();
    BODY.get_or_init(|| {
        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/characters/human/HumanParts.glb");
        let bytes = std::fs::read(file).unwrap();
        let picks = [Pick::Objects(vec!["AxcenityHuman/Body".to_owned()])];
        load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
            .unwrap()
            .remove(0)
    })
}

/// Where the character stands: 5 studs tall, turned to face -Z, a little
/// off the origin (as a game places it).
fn placement() -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::splat(5.0),
        Quat::from_rotation_y(std::f32::consts::PI),
        Vec3::new(1.5, 2.5, -0.5),
    )
}

/// World units in a metre of the body's file.
fn per_metre(model: &Model) -> f32 {
    fitted_per_metre(model).unwrap() * 5.0
}

/// The body in its rest pose, with `shapes` worn live as an identity.
fn posed_with(model: &Model, shapes: &[(&str, f32)]) -> Posed {
    let rig = model.rig.as_ref().unwrap();
    let mut pose = rig.rest_pose();
    let identity = Identity {
        layers: vec![IdentityLayer {
            shapes: shapes
                .iter()
                .map(|(name, weight)| ((*name).to_owned(), *weight))
                .collect(),
            bones: Vec::new(),
        }],
    };
    let live = model.live(&identity);
    live.settle(&mut pose);
    let mut posed = model.pose(&pose).unwrap();
    live.dress(&mut posed);
    posed
}

/// The nose tip's corner: 12.2 cm in front of the head bone's head
/// (measured on the base's default body).
fn nose_tip(model: &Model) -> Corner {
    let anchor = rest_anchor(model, "Head", Vec3::new(0.0, 0.0006, 0.1217)).unwrap();
    nearest_corner(model, anchor).unwrap()
}

#[test]
fn a_ray_at_a_known_point_of_the_posed_body_hits_within_a_millimetre() {
    let model = body();
    let posed = posed_with(model, &[]);
    let corner = nose_tip(model);
    let (point, normal) = corner_drawn(model, Some(&posed), placement(), corner).unwrap();
    assert!(normal.length() > 0.99);
    // From half a stud out along the skin's normal, straight back at it.
    let ray = Ray {
        origin: point + normal * 0.5,
        direction: -normal,
    };
    let hit = ray_model(model, Some(&posed), placement(), ray).expect("a hit");
    let millimetre = 0.001 * per_metre(model);
    assert!(
        hit.point.distance(point) < millimetre,
        "{} mm from the corner",
        hit.point.distance(point) / millimetre
    );
    assert!((hit.distance - 0.5).abs() < millimetre);
    assert!(hit.normal.dot(normal) > 0.5, "the skin's own side");
    // The human base keeps its regions in a map beside the model, not
    // on its corners: a hit names none.
    assert_eq!(hit.region, None);
    // At rest, the hit is the corner as it was built.
    let rest = hit.at_rest(model).unwrap();
    let built = Vec3::from(model.primitives[corner.primitive].positions[corner.index]);
    assert!(rest.distance(built) * 1000.0 / fitted_per_metre(model).unwrap() < 1.0);

    // The same far down the limbs: a foot's sole, from below.
    let lowest = model.primitives[0]
        .positions
        .iter()
        .enumerate()
        .min_by(|a, b| a.1[1].total_cmp(&b.1[1]))
        .map(|(index, _)| Corner {
            primitive: 0,
            index,
        })
        .unwrap();
    let (foot, _) = corner_drawn(model, Some(&posed), placement(), lowest).unwrap();
    let up = Ray {
        origin: foot - Vec3::Y * 0.3,
        direction: Vec3::Y,
    };
    let hit = ray_model(model, Some(&posed), placement(), up).expect("the sole");
    assert!(
        hit.point.distance(foot) < 0.05,
        "{:?} vs {foot:?}",
        hit.point
    );
}

#[test]
fn the_identity_s_weights_change_where_the_ray_hits() {
    let model = body();
    let plain = posed_with(model, &[]);
    let corner = nose_tip(model);
    let (point, normal) = corner_drawn(model, Some(&plain), placement(), corner).unwrap();
    // Straight at the tip from in front of the face.
    let ray = Ray {
        origin: point + normal * 0.5,
        direction: -normal,
    };
    let before = ray_model(model, Some(&plain), placement(), ray).unwrap();
    // A longer nose with its tip pushed out, worn live.
    let longer = posed_with(
        model,
        &[
            ("Face_Nose_Scale_Depth_Incr", 1.0),
            ("Face_Nose_Trans_Forward", 1.0),
        ],
    );
    let after = ray_model(model, Some(&longer), placement(), ray).unwrap();
    let millimetre = 0.001 * per_metre(model);
    let moved = (before.distance - after.distance) / millimetre;
    assert!(moved > 2.0, "the tip came {moved} mm nearer");
    // The corner moved with it, and the ray still meets the skin there.
    let (tip, _) = corner_drawn(model, Some(&longer), placement(), corner).unwrap();
    assert!(tip.distance(point) / millimetre > 2.0);
    // Without a pose the rest mesh is tested: the same as the plain pose.
    let rest = ray_model(model, None, placement(), ray).unwrap();
    assert!((rest.distance - before.distance).abs() < millimetre);
}

#[test]
fn a_ray_that_misses_the_body_returns_none() {
    let model = body();
    let posed = posed_with(model, &[]);
    let corner = nose_tip(model);
    let (point, normal) = corner_drawn(model, Some(&posed), placement(), corner).unwrap();
    // Pointing away from the face.
    let away = Ray {
        origin: point + normal * 0.5,
        direction: normal,
    };
    assert_eq!(ray_model(model, Some(&posed), placement(), away), None);
    // Passing well beside the body.
    let beside = Ray {
        origin: Vec3::new(30.0, 3.0, -10.0),
        direction: Vec3::Z,
    };
    assert_eq!(ray_model(model, Some(&posed), placement(), beside), None);
}

/// A handle's anchor finds the skin's corner nearest it in the file's
/// metres, not in the fitted box (whose axes are scaled apart: the body is
/// much wider, arms out, than it is deep). Points 3 mm off corners spread
/// over the skin find the corner a search in metres finds.
#[test]
fn the_nearest_corner_is_nearest_in_the_file_s_metres() {
    let model = body();
    let fit = model.rig.as_ref().unwrap().fit;
    let unfit = fit.inverse();
    let tip = nose_tip(model);
    let ahead = rest_anchor(model, "Head", Vec3::new(0.0, 0.0006, 0.1267)).unwrap();
    assert_eq!(nearest_corner(model, ahead), Some(tip));
    let skin: Vec<(Corner, Vec3)> = model
        .primitives
        .iter()
        .enumerate()
        .filter(|(_, mesh)| mesh.material.shading == ModelShading::Skin)
        .flat_map(|(primitive, mesh)| {
            mesh.positions.iter().enumerate().map(move |(index, at)| {
                (
                    Corner { primitive, index },
                    unfit.transform_point3(Vec3::from(*at)),
                )
            })
        })
        .collect();
    let mut wrong = 0;
    let tried = skin.len().div_ceil(211);
    for (_, at) in skin.iter().step_by(211) {
        let near = *at + Vec3::new(0.0017, -0.0017, 0.0017);
        let expected = skin
            .iter()
            .min_by(|a, b| a.1.distance(near).total_cmp(&b.1.distance(near)))
            .unwrap();
        let found = nearest_corner(model, fit.transform_point3(near)).unwrap();
        let found_at = skin.iter().find(|(corner, _)| *corner == found).unwrap().1;
        // A tie (corners repeated at seams) is as near either way.
        if found_at.distance(near) > expected.1.distance(near) + 1e-6 {
            wrong += 1;
        }
    }
    assert_eq!(wrong, 0, "{wrong} of {tried} points found a farther corner");
}

/// The ray meets the skin where the pose puts it, not where it rests: with
/// the head turned 30 degrees, a ray at the turned nose tip hits it within
/// a millimetre, and the same ray against the rest mesh does not.
#[test]
fn a_ray_at_a_turned_head_meets_the_posed_skin() {
    let model = body();
    let rig = model.rig.as_ref().unwrap();
    let mut pose = rig.rest_pose();
    let head = rig.bone("Head").unwrap();
    pose[head].rotation = Quat::from_rotation_y(30f32.to_radians()) * pose[head].rotation;
    let turned = model.pose(&pose).unwrap();
    let corner = nose_tip(model);
    let (point, normal) = corner_drawn(model, Some(&turned), placement(), corner).unwrap();
    let (resting, _) = corner_drawn(model, None, placement(), corner).unwrap();
    let millimetre = 0.001 * per_metre(model);
    assert!(
        point.distance(resting) / millimetre > 20.0,
        "the turn moved the tip"
    );
    let ray = Ray {
        origin: point + normal * 0.5,
        direction: -normal,
    };
    let hit = ray_model(model, Some(&turned), placement(), ray).expect("a hit");
    assert!(hit.point.distance(point) < millimetre);
    let rest = ray_model(model, None, placement(), ray);
    assert!(rest.is_none_or(|rest| rest.point.distance(point) > 5.0 * millimetre));
}
