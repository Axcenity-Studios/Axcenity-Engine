//! Character Studio overhaul, CO5: each of the nine body sliders the
//! realistic base model's proportion parameters set (`BodyHeight`,
//! `LegLength`, …) moves the realistic body's skin, on the body and on
//! both of its simpler copies (levels of detail), at both ends of the
//! manifest's range (0.5 and 1.5).

use std::path::Path;

use axcenity_anim::{BODY_SLIDERS, BodyShape};
use axcenity_assets::{Model, ModelFormat, Pick, load_models};
use axcenity_core::{ClassName, Value, World};

fn body() -> Model {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/characters/realistic/RealisticParts.glb");
    let bytes = std::fs::read(file).unwrap();
    let picks = [Pick::Objects(vec!["AxcenityRealistic/Body".to_owned()])];
    load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
        .unwrap()
        .remove(0)
}

fn farthest(a: &[[f32; 3]], b: &[[f32; 3]]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(p, q)| glam::Vec3::from(*p).distance(glam::Vec3::from(*q)))
        .fold(0.0, f32::max)
}

#[test]
fn every_body_slider_moves_the_realistic_body_and_its_simpler_copies() {
    let model = body();
    let rig = model.rig.clone().unwrap();
    assert_eq!(model.primitives[0].lods.len(), 2);
    let plain = model.pose_shaped(&rig.rest_pose(), &[]).unwrap();
    let mut world = World::new();
    let part = world.create(ClassName::MeshPart).unwrap();
    for slider in BODY_SLIDERS {
        for value in [0.5, 1.5] {
            world
                .set_attribute(part, slider, Some(Value::Number(value)))
                .unwrap();
            let shape = BodyShape::read(&world, part).expect("a slider away from 1");
            let mut pose = rig.rest_pose();
            let scales = shape.apply(&rig, &mut pose);
            let posed = model.pose_shaped(&pose, &scales).unwrap();
            for level in 0..3 {
                let far = farthest(
                    &posed.primitive(&model, 0, level).positions,
                    &plain.primitive(&model, 0, level).positions,
                );
                assert!(
                    far > 0.005,
                    "{slider} at {value}, level of detail {level}: moved {far} m"
                );
            }
            world.set_attribute(part, slider, None).unwrap();
        }
    }
}
