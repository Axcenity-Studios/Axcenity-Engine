//! Character Studio refinement R2 (direct manipulation): each handle of
//! the realistic body, applied in the player as a game applies a
//! description, stays on the skin at both ends of every parameter it
//! drives, and its measured `axis` and `travel` are how that skin moves,
//! so one centimetre of drag moves the surface about one centimetre.
//!
//! A handle is found as Character Studio finds it: its anchor at rest
//! (`scene::rest_anchor`), the skin's corner there (`nearest_corner`), and
//! that corner where the frame's pose draws it (`corner_drawn`), the pose
//! carrying the identity live (ADR-0041). The tests hold the data to the
//! skin, not that corner to itself: the anchor must lie on the skin at
//! rest, and at a parameter's end the handle the data predicts (the
//! anchor where the start draws it, plus the drive's `axis` times `travel`
//! times the share of the range the value moved) must lie on the skin the
//! player applied for that end. A wrong anchor, axis or travel fails.
//!
//! The anchor is carried by its skin corner, not by its bone alone: the
//! feminine frame's shapes move the face up to 22 mm and the shoulders 36
//! mm off where the head's or the shoulder's bone would carry them, and a
//! hip corner shared with the thigh's twist bone sits 5 mm off its bone's
//! carry even on the masculine frame (Review fixes 1 in the hand-back).

use std::sync::Arc;

use axcenity_avatar::{BaseManifest, Description, Handle};
use axcenity_core::Value;
use axcenity_gameplay::description::{apply_description, default_description, library};
use axcenity_render::glam::{Mat4, Vec3};
use axcenity_scene::{
    corner_drawn, fitted_per_metre, nearest_corner, placement, rest_anchor, to_metres,
};

use super::character_capture::{Stage, repository_library};
use super::*;

const BASE: &str = "axcenity/realistic";

/// One realistic character on a stage, posed still (its Animator plays
/// nothing, so the pose is the rest pose with the identity on it).
struct Character {
    stage: Stage,
    world: World,
    animators: Animators,
    character: InstanceId,
    body: InstanceId,
}

impl Character {
    fn new(name: &str) -> Self {
        let stage = Stage::new(name, &repository_library()).expect("the stage");
        let mut world = World::new();
        let workspace = world.workspace();
        let character = axcenity_project::default_character(&mut world, Default::default());
        world.set_parent(character, Some(workspace)).unwrap();
        let shipped = library(&stage.assets);
        let applied = apply_description(
            &mut world,
            &stage.assets,
            character,
            &default_description(&shipped, BASE),
        )
        .unwrap();
        let animator = world.find_first_child(applied.body, "Animator").unwrap();
        world
            .set_property(animator, "StateMachine", Value::String(String::new()))
            .unwrap();
        Self {
            stage,
            world,
            animators: Animators::new(),
            character,
            body: applied.body,
        }
    }

    fn base(&self) -> BaseManifest {
        library(&self.stage.assets).base(BASE).unwrap().clone()
    }

    /// Applies `description` and poses it.
    fn wear(&mut self, description: &Description) {
        let applied = apply_description(
            &mut self.world,
            &self.stage.assets,
            self.character,
            description,
        )
        .unwrap();
        assert!(applied.problems.is_empty(), "{:?}", applied.problems);
        for _ in 0..2 {
            self.animators
                .update(&mut self.world, &self.stage.assets, 1.0 / 60.0);
        }
    }

    fn model(&self) -> Arc<axcenity_assets::Model> {
        let mesh = self
            .world
            .get(self.body)
            .and_then(|body| body.property("MeshId"))
            .and_then(Value::as_str)
            .unwrap()
            .to_owned();
        self.stage.assets.model_blocking(&mesh).unwrap()
    }

    /// Where `handle` is drawn by the studio (its skin corner as posed).
    fn handle(&self, handle: &Handle) -> Vec3 {
        let model = self.model();
        let posed = self.animators.poses().get(&self.body).cloned();
        let transform = placement(&self.world, self.body).unwrap();
        let offset = Vec3::from(handle.anchor.offset);
        let anchor = rest_anchor(&model, &handle.anchor.bone, offset).unwrap();
        let corner = nearest_corner(&model, anchor).unwrap();
        corner_drawn(&model, posed.as_deref(), transform, corner)
            .unwrap()
            .0
    }

    /// Where the data puts `handle`'s anchor now: its skin corner as the
    /// frame's pose draws it, plus the anchor's own offset from that corner
    /// at rest (so an anchor off the skin stays off it).
    fn anchored(&self, handle: &Handle) -> Vec3 {
        let model = self.model();
        let offset = Vec3::from(handle.anchor.offset);
        let anchor = rest_anchor(&model, &handle.anchor.bone, offset).unwrap();
        let corner = nearest_corner(&model, anchor).unwrap();
        let at = Vec3::from(model.primitives[corner.primitive].positions[corner.index]);
        let transform = placement(&self.world, self.body).unwrap();
        self.handle(handle) + transform.transform_vector3(anchor - at)
    }

    /// The body's skin as posed: every triangle of its skin-shaded
    /// primitives, corners in the world.
    fn skin(&self) -> Vec<[Vec3; 3]> {
        let model = self.model();
        let posed = self.animators.poses().get(&self.body).cloned();
        let transform = placement(&self.world, self.body).unwrap();
        let mut triangles = Vec::new();
        for (index, primitive) in model.primitives.iter().enumerate() {
            if primitive.material.shading != axcenity_assets::ModelShading::Skin {
                continue;
            }
            let corners = axcenity_scene::corners_drawn(&model, posed.as_deref(), transform, index);
            for triangle in primitive.indices.as_chunks::<3>().0 {
                triangles.push(triangle.map(|corner| corners[corner as usize]));
            }
        }
        triangles
    }

    /// World units in a metre of the body's file.
    fn per_metre(&self) -> f32 {
        let transform = placement(&self.world, self.body).unwrap();
        fitted_per_metre(&self.model()).unwrap() * transform.transform_vector3(Vec3::X).length()
    }

    fn transform(&self) -> Mat4 {
        let fit = self.model().rig.as_ref().unwrap().fit;
        placement(&self.world, self.body).unwrap() * fit
    }
}

/// The closest point of triangle `abc` to `p` (Ericson, "Real-Time
/// Collision Detection", 5.1.5).
fn closest_on_triangle(p: Vec3, [a, b, c]: [Vec3; 3]) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denominator = 1.0 / (va + vb + vc);
    a + ab * (vb * denominator) + ac * (vc * denominator)
}

fn distance_to_surface(point: Vec3, triangles: &[[Vec3; 3]]) -> f32 {
    triangles
        .iter()
        .map(|triangle| closest_on_triangle(point, *triangle).distance(point))
        .fold(f32::INFINITY, f32::min)
}

/// A handle's anchor names a point of the skin: at rest, the skin's
/// nearest corner (the one the studio draws the handle on) is within 2 mm
/// of it. An anchor moved to another bone, or off the skin, fails here.
#[test]
fn every_handle_s_anchor_is_on_the_skin_at_rest() {
    let character = Character::new("handles-anchored");
    let base = character.base();
    let model = character.model();
    assert!(!base.handles.is_empty(), "the realistic body has handles");
    for handle in &base.handles {
        let offset = Vec3::from(handle.anchor.offset);
        let anchor = rest_anchor(&model, &handle.anchor.bone, offset).unwrap();
        let corner = nearest_corner(&model, anchor).unwrap();
        let at = Vec3::from(model.primitives[corner.primitive].positions[corner.index]);
        let off = ((at - anchor) * to_metres(&model)).length();
        assert!(
            off <= 0.002,
            "{}: its anchor is {:.2} mm from the nearest skin corner",
            handle.id,
            off * 1000.0
        );
    }
}

/// The R2 "Done when": every handle stays within 2 mm of the applied
/// body's skin at both ends of each parameter it drives, on both frames.
/// The handle at an end is predicted from the data (plan 11.3), not found
/// on the skin: its anchor where the start description draws it, plus the
/// drive's `axis` times `travel` times the share of the range the value
/// moved from its default. It is measured against the skin's triangles as
/// the player applies and poses that end. Each frame uses its own drive
/// (`Drive::on_frame`): the waist has a shape per frame, so the feminine
/// waist's travel is its own; every other drive was measured on the
/// masculine frame, so the feminine frame is the stricter case for them.
#[test]
fn every_handle_stays_on_the_skin_at_its_parameters_ends() {
    let mut character = Character::new("handles-on-skin");
    let base = character.base();
    assert!(!base.handles.is_empty(), "the realistic body has handles");
    let shipped = library(&character.stage.assets);
    let mut report = Vec::new();
    let mut off = Vec::new();
    for frame in [0.0, 1.0] {
        let mut start = default_description(&shipped, BASE);
        start.parameters.insert("frame".into(), frame);
        character.wear(&start);
        let from: Vec<Vec3> = base.handles.iter().map(|h| character.anchored(h)).collect();
        let to_world = character.transform();
        for (handle, from) in base.handles.iter().zip(from) {
            for drive in &handle.drives {
                let parameter = base.parameter(&drive.parameter).unwrap();
                let span = parameter.range[1] - parameter.range[0];
                for end in parameter.range {
                    let mut description = start.clone();
                    description.parameters.insert(drive.parameter.clone(), end);
                    character.wear(&description);
                    let share = (end - parameter.default) / span;
                    let (axis, travel) = drive.on_frame(frame);
                    let point =
                        from + to_world.transform_vector3(Vec3::from(axis) * travel * share);
                    let mm = distance_to_surface(point, &character.skin()) / character.per_metre()
                        * 1000.0;
                    let line = format!(
                        "frame {frame} {} {} at {end}: {mm:.2} mm",
                        handle.id, drive.parameter
                    );
                    if mm > 2.0 {
                        off.push(line.clone());
                    }
                    report.push(line);
                }
            }
        }
    }
    println!("{}", report.join("\n"));
    assert!(
        off.is_empty(),
        "more than 2 mm off the skin:\n{}",
        off.join("\n")
    );
}

/// Each drive's `axis` and `travel` (measured by `handles.py` on the
/// model) are how the applied skin under the handle moves from the
/// parameter's lowest value to its highest, on the masculine frame: the
/// distance within 5% and the direction within about 8 degrees (they
/// agree to a tenth of a millimetre when this was written: `handles.py`
/// works the body sliders' bone changes out as `anim` makes them). So a
/// drag that follows the skin on the screen is one centimetre of skin for
/// one centimetre of drag.
#[test]
fn every_drive_moves_the_skin_by_its_measured_travel() {
    let mut character = Character::new("handles-travel");
    let base = character.base();
    let shipped = library(&character.stage.assets);
    let start = default_description(&shipped, BASE);
    let mut report = Vec::new();
    for handle in &base.handles {
        for drive in &handle.drives {
            let parameter = base.parameter(&drive.parameter).unwrap();
            let mut ends = Vec::new();
            for end in parameter.range {
                let mut description = start.clone();
                description.parameters.insert(drive.parameter.clone(), end);
                character.wear(&description);
                ends.push(character.handle(handle));
            }
            let moved = ends[1] - ends[0];
            let expected = character
                .transform()
                .transform_vector3(Vec3::from(drive.axis) * drive.travel);
            let ratio = moved.length() / expected.length();
            let along = moved.normalize_or_zero().dot(expected.normalize_or_zero());
            report.push(format!(
                "{} {}: {:.1} mm measured, {:.1} mm in the data, cos {along:.2}",
                handle.id,
                drive.parameter,
                moved.length() / character.per_metre() * 1000.0,
                drive.travel * 1000.0
            ));
            assert!(
                (0.95..=1.05).contains(&ratio) && along >= 0.99,
                "{}",
                report.last().unwrap()
            );
        }
    }
    println!("{}", report.join("\n"));
}
