//! Tests for the Model Studio upgrades:
//! 1. Surface Properties presets, auto-mass calculation, overrides & sidecar round-trip
//! 2. Capsule collision generation
//! 3. Merged LOD Ribbon & Auto screen-size switching / locking
//! 4. Face material painting & slot reuse (no per-face draw call explosion)
//! 5. Stencil / Draw backface culling & occlusion check
//! 6. Depth-tested Edit-mode visibility vs. X-Ray toggle
//! 7. WeightTool::Normalize radial brush
//! 8. One-click "Game Ready" validation checks

use axcenity_assets::{ModelFormat, ModelMaterial};
use axcenity_mesh_edit::paint::{BrushFalloff, Dab};
use axcenity_mesh_edit::rig::{Armature, Skin, WeightBrush, WeightPainter, WeightTool};
use axcenity_mesh_edit::{
    ModelDoc, PrimitiveSettings, PrimitiveShape, Reference, SurfacePreset, SurfaceProperties,
    TexelMap, smart_project,
};
use axcenity_render::glam::{Vec2, Vec3};

use crate::{CollisionShape, StudioDoc};

fn cube_doc() -> StudioDoc {
    StudioDoc::new(
        "assets/models/Cube.glb".to_owned(),
        ModelDoc::primitive(&PrimitiveSettings::of(PrimitiveShape::Cube)),
        false,
    )
}

#[test]
fn surface_properties_presets_compute_mass_and_round_trip_through_sidecar() {
    let mut doc = cube_doc();
    // Default cube is 2x2x2 = 8 cubic units.
    let volume = doc.model.objects[0].bounding_box_volume();
    assert!((volume - 8.0).abs() < 1e-3, "2x2x2 cube volume is 8");

    let wood = SurfaceProperties {
        preset: SurfacePreset::WoodPlank,
        ..SurfaceProperties::default()
    };
    assert!(doc.set_surface_properties(wood.clone()));
    assert_eq!(doc.undo_label(), Some("Surface Properties"));
    assert!(
        (doc.model.objects[0].surface.effective_mass(volume) - 8.0 * 650.0).abs() < 1e-2,
        "Wood_Plank mass = volume * 650"
    );
    assert_eq!(
        doc.model.objects[0].surface.effective_decal_set(),
        "decals/impact_wood"
    );

    // Manual override takes precedence over preset calculation.
    let custom = SurfaceProperties {
        preset: SurfacePreset::Concrete,
        mass_override: Some(42.5),
        friction_override: Some(0.95),
        bounciness_override: Some(0.3),
        decal_set_override: Some("decals/custom_scorch".to_owned()),
        sound_set_override: None,
    };
    assert!(doc.set_surface_properties(custom.clone()));
    assert!((doc.model.objects[0].surface.effective_mass(volume) - 42.5).abs() < 1e-4);
    assert!((doc.model.objects[0].surface.effective_friction() - 0.95).abs() < 1e-4);
    assert!((doc.model.objects[0].surface.effective_bounciness() - 0.3).abs() < 1e-4);
    assert_eq!(
        doc.model.objects[0].surface.effective_decal_set(),
        "decals/custom_scorch"
    );
    assert_eq!(
        doc.model.objects[0].surface.effective_sound_set(),
        "sounds/impact_concrete"
    );

    // Sidecar round-trip preserves surface properties.
    let glb = doc.model.write_glb();
    let sidecar = doc
        .model
        .write_sidecar(&glb)
        .expect("non-default surface properties require sidecar");
    let mut reopened = ModelDoc::open(&glb, std::path::Path::new("."), ModelFormat::Gltf).unwrap();
    reopened.read_sidecar(&sidecar, &glb).unwrap();
    assert_eq!(reopened.objects[0].surface, custom);
}

#[test]
fn capsule_collision_generates_convex_piece_and_undoes() {
    let mut doc = cube_doc();
    doc.generate_collision(CollisionShape::Capsule)
        .expect("capsule collision succeeds on cube");
    assert_eq!(doc.undo_label(), Some("Generate Capsule Collision"));
    assert_eq!(doc.model.collision_pieces(0).len(), 1);
    assert!(doc.show_collision);

    assert!(doc.undo());
    assert!(doc.model.collision_pieces(0).is_empty());
}

#[test]
fn lod_ribbon_auto_swaps_by_screen_size_and_locks_when_selected() {
    let mut doc = cube_doc();
    // Without LOD copies, Auto always resolves to 0.
    assert_eq!(doc.lod_preview, None);
    assert_eq!(doc.active_lod_level(0.01), 0);

    doc.generate_lods().expect("generate LODs on cube");
    assert_eq!(doc.model.lod_copies(0).len(), 2);

    // Auto mode dynamically switches by screen size.
    assert_eq!(doc.active_lod_level(0.8), 0);
    assert_eq!(doc.active_lod_level(0.001), 2);

    // Clicking a specific LOD locks it regardless of camera distance.
    doc.lod_preview = Some(1);
    assert_eq!(doc.active_lod_level(0.9), 1);
    assert_eq!(doc.active_lod_level(0.001), 1);
}

#[test]
fn face_painting_reuses_material_slots_without_per_face_explosion() {
    let mut doc = cube_doc();
    assert_eq!(doc.model.objects[0].materials.len(), 1);

    let brick = ModelMaterial {
        name: "Brick".to_owned(),
        base_color: [0.7, 0.25, 0.2, 1.0],
        roughness: 0.85,
        ..ModelMaterial::default()
    };
    let slot_a = doc
        .paint_faces_with_material(0, &[0], brick.clone())
        .expect("painted face 0");
    let slot_b = doc
        .paint_faces_with_material(0, &[1, 2], brick)
        .expect("painted faces 1 and 2");

    // Painting multiple faces with the same material reuses one slot.
    assert_eq!(slot_a, slot_b);
    assert_eq!(doc.model.objects[0].materials.len(), 2);
    let counts = doc.slot_face_counts(0);
    assert_eq!(counts, vec![3, 3]);

    // Replacing a slot updates all faces using it without adding slots.
    let concrete = ModelMaterial {
        name: "Concrete".to_owned(),
        base_color: [0.5, 0.5, 0.5, 1.0],
        ..ModelMaterial::default()
    };
    assert!(doc.replace_material_slot(0, slot_a, concrete));
    assert_eq!(doc.model.objects[0].materials.len(), 2);
    assert_eq!(doc.model.objects[0].materials[slot_a].name, "Concrete");
}

#[test]
fn stencil_and_draw_brushes_cull_backfaces_and_occluded_surfaces() {
    let mut mesh = PrimitiveSettings::of(PrimitiveShape::Cube).build();
    let all_faces: Vec<u32> = (0..mesh.face_count() as u32).collect();
    smart_project(&mut mesh, &all_faces, 66.0_f32.to_radians(), 0.02, 0).unwrap();

    let texels = TexelMap::build(&mesh, 0, 128);
    let occluder = Reference::new(&mesh);
    // Dab on the +Z front face with huge radius that reaches the -Z back face.
    let dab = Dab {
        center: Vec3::new(0.0, 0.0, 1.0),
        normal: Vec3::Z,
        radius: 5.0,
        strength: 1.0,
        motion: Vec3::ZERO,
        mirror: [false; 3],
    };
    let camera = Vec3::new(0.0, 0.0, 6.0);
    let touches = texels.touches_visible(
        &dab,
        BrushFalloff::Constant,
        Some(camera),
        Some(&occluder),
        &|_| Some(Vec2::ZERO),
    );
    assert!(!touches.is_empty(), "front +Z face receives paint");

    // None of the touched texels may belong to the -Z back face.
    let back_face = (0..mesh.face_count() as u32)
        .find(|&f| mesh.face_normal(f).z < -0.9)
        .unwrap();
    let back_set = std::collections::BTreeSet::from([back_face]);
    let back_texels: std::collections::BTreeSet<u32> =
        texels.face_texels(&back_set).into_iter().collect();
    assert!(
        touches.iter().all(|t| !back_texels.contains(&t.texel)),
        "back-facing -Z face must not be painted from +Z camera"
    );
}

#[test]
fn depth_tested_visibility_hides_back_vertices_unless_xray_is_on() {
    let mesh = PrimitiveSettings::of(PrimitiveShape::Cube).build();
    let occluder = Reference::new(&mesh);
    let camera = Vec3::new(0.0, 0.0, 6.0);

    let front_v = Vec3::new(0.0, 0.0, 1.0);
    let back_v = Vec3::new(0.0, 0.0, -1.0);

    // Depth-tested (default): front is visible, occluded back is hidden.
    assert!(crate::view::point_visible(
        Some(&occluder),
        camera,
        front_v,
        Some(Vec3::Z)
    ));
    assert!(!crate::view::point_visible(
        Some(&occluder),
        camera,
        back_v,
        Some(Vec3::NEG_Z)
    ));

    // X-Ray (occluder = None): both front and back are visible.
    assert!(crate::view::point_visible(
        None,
        camera,
        back_v,
        Some(Vec3::NEG_Z)
    ));
}

#[test]
fn weight_normalize_brush_normalizes_touched_vertices() {
    let mesh = PrimitiveSettings::of(PrimitiveShape::Cube).build();
    let mut arm = Armature::default();
    arm.add_bone("Root", Vec3::ZERO, Vec3::Y, None);
    arm.add_bone("Tip", Vec3::Y, Vec3::Y * 2.0, Some(0));

    // Start with un-normalized weights (sum = 2.0 on every vertex).
    let weights = vec![vec![(0u16, 1.2), (1u16, 0.8)]; mesh.vertex_count()];
    let mut skin = Skin::new(&mesh, weights);
    let painter = WeightPainter::new(&mesh);
    let brush = WeightBrush {
        tool: WeightTool::Normalize,
        radius: 10.0,
        strength: 1.0,
        falloff: BrushFalloff::Constant,
        auto_normalize: false,
    };
    assert!(painter.dab(&mut skin, &brush, 0, Vec3::ZERO, 10.0));
    for v in 0..mesh.vertex_count() {
        let sum: f32 = skin.weights[v].iter().map(|&(_, w)| w).sum();
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "vertex {v} weights normalized to 1.0 (got {sum})"
        );
    }
}

#[test]
fn game_ready_validation_flags_missing_collision_and_lods_then_passes() {
    let mut doc = cube_doc();
    let initial = doc.validate_game_ready();
    assert!(!initial.collision_ok());
    assert!(!initial.lods_ok());
    assert!(initial.normals_ok());
    assert!(!initial.all_ok());

    doc.generate_collision(CollisionShape::Hull).unwrap();
    doc.generate_lods().unwrap();

    let after = doc.validate_game_ready();
    assert!(after.collision_ok());
    assert!(after.lods_ok());
    assert!(after.normals_ok());
    assert!(after.all_ok());
}
