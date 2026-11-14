//! What parameter values do to the body's shapes.

use crate::fixtures;
use crate::{
    Colour, Description, MaterialValue, PartFit, WornItem, body_shape_weights, parameter_values,
    resolve, span,
};

#[track_caller]
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}

#[test]
fn a_span_maps_and_clamps() {
    close(span(0.25, [0.0, 1.0], None), 0.25);
    close(span(1.7, [0.0, 1.0], None), 1.0);
    close(span(-0.4, [0.0, 1.0], None), 0.0);
    // Downwards: slim is the negative side of build.
    close(span(-0.4, [0.0, -1.0], Some([0.0, 1.0])), 0.4);
    close(span(0.5, [0.0, -1.0], Some([0.0, 1.0])), 0.0);
    // A bipolar shape keeps its sign.
    close(span(-0.6, [-1.0, 1.0], None), -0.6);
    // Onto another span.
    close(span(1.0, [0.5, 1.5], Some([0.0, 2.0])), 1.0);
    // An empty span gives its start rather than dividing by zero.
    close(span(3.0, [1.0, 1.0], Some([0.2, 0.9])), 0.2);
}

#[test]
fn parameters_take_their_defaults_clamp_and_keep_to_their_limits() {
    let base = fixtures::base();
    let mut description = Description::new("Ada", "test/body");
    let values = parameter_values(&base, &description);
    assert_eq!(values.len(), base.parameters.len());
    close(values["height"], 1.0);
    close(values["frame"], 0.0);
    description.parameters.insert("build".to_owned(), -3.0);
    description.parameters.insert("fat".to_owned(), 1.0);
    description.parameters.insert("muscle".to_owned(), 1.0);
    description.parameters.insert("unknown".to_owned(), 1.0);
    let values = parameter_values(&base, &description);
    close(values["build"], -1.0);
    // fat + muscle ≤ 1.5: both scaled down in proportion.
    close(values["fat"], 0.75);
    close(values["muscle"], 0.75);
    assert!(!values.contains_key("unknown"));
}

#[test]
fn shapes_take_their_drivers_corners_and_hidden_regions() {
    let base = fixtures::base();
    let library = fixtures::library();
    let mut description = Description::new("Ada", "test/body");
    description.parameters.insert("frame".to_owned(), 0.5);
    description.parameters.insert("fat".to_owned(), 0.4);
    description.parameters.insert("build".to_owned(), -0.3);
    description
        .parameters
        .insert("eye_size_left".to_owned(), -0.5);
    let weights = body_shape_weights(&base, &description, &library);
    close(weights["Frame_Feminine"], 0.5);
    close(weights["Fat"], 0.4);
    close(weights["Heavy"], 0.0);
    close(weights["Slim"], 0.3);
    close(weights["Eye_Size_Left"], -0.5);
    close(weights["Eye_Size_Right"], 0.0);
    close(weights["Feminine_Fat"], 0.2); // 0.5 × 0.4
    close(weights["Mask_Torso"], 0.0); // nothing worn
    // Every shape the manifest names is there, used or not.
    assert_eq!(weights.len(), 9);
    // The T-shirt hides the torso; an item the library lacks is skipped.
    description.items = vec![WornItem::new("test/tee"), WornItem::new("test/cape")];
    let weights = body_shape_weights(&base, &description, &library);
    close(weights["Mask_Torso"], 1.0);
}

fn layer_of<'a>(resolved: &'a crate::Resolved, shape: &str) -> &'a axcenity_assets::IdentityLayer {
    resolved
        .identity
        .layers
        .iter()
        .find(|layer| layer.shapes.iter().any(|(name, _)| name == shape))
        .unwrap_or_else(|| panic!("no layer has `{shape}`"))
}

#[test]
fn a_description_resolves_to_its_body_parts_and_items() {
    let library = fixtures::library();
    let mut description = Description::new("Ada", "test/body");
    description.items = vec![
        WornItem::new("test/hair"),
        WornItem::new("test/tee"),
        WornItem::new("test/watch"),
    ];
    let resolved = resolve(&library, &description).expect("the base is in the library");
    assert_eq!(resolved.base, "test/body");
    assert_eq!(resolved.body.name, "Body");
    assert_eq!(resolved.body.model, "test/Body.glb#Rig/Body");
    assert_eq!(resolved.body.item, None);
    let parts: Vec<(&str, &str)> = resolved
        .parts
        .iter()
        .map(|part| (part.name.as_str(), part.model.as_str()))
        .collect();
    assert_eq!(
        parts,
        [
            // The base model's own parts first, then the items as worn.
            ("Eyes", "test/Body.glb#Rig/Eyes"),
            ("Cards", "test/items/Hair.glb#Cards"),
            ("Tee", "test/items/Tee.glb#Rig/Tee"),
            // A whole file is named after its item.
            ("watch", "test/items/Watch.glb"),
        ]
    );
    assert_eq!(resolved.parts[1].item.as_deref(), Some("test/hair"));
    assert_eq!(resolved.parts[1].slot.as_deref(), Some("hair"));
    assert_eq!(resolved.parts[2].fit, PartFit::Skinned);
    assert_eq!(
        resolved.parts[3].fit,
        PartFit::Socket("LeftWristSocket".to_owned())
    );
    assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
}

#[test]
fn items_that_cannot_be_worn_are_left_out_and_said() {
    let mut library = fixtures::library();
    let mut other = fixtures::item(fixtures::TEE);
    other.id = "test/tee-two".to_owned();
    library.add_item("test/items/tee-two.item.json", other);
    let mut foreign = fixtures::item(fixtures::TEE);
    foreign.id = "test/foreign".to_owned();
    foreign.bases = vec!["test/other-body".to_owned()];
    library.add_item("test/items/foreign.item.json", foreign);
    let mut description = Description::new("Ada", "test/body");
    description.items = vec![
        WornItem::new("test/tee"),
        WornItem::new("test/cape"),
        WornItem::new("test/foreign"),
        // The top slot holds one item: the later one is worn.
        WornItem::new("test/tee-two"),
        // The wrist slot holds several.
        WornItem::new("test/watch"),
        WornItem::new("test/watch"),
    ];
    let resolved = resolve(&library, &description).unwrap();
    let items: Vec<&str> = resolved
        .parts
        .iter()
        .filter_map(|part| part.item.as_deref())
        .collect();
    assert_eq!(items, ["test/tee-two", "test/watch", "test/watch"]);
    let said: Vec<&str> = resolved
        .problems
        .iter()
        .map(|problem| problem.at.as_str())
        .collect();
    assert_eq!(said, ["items[0]", "items[1]", "items[2]"]);
    // An unknown base can't be resolved at all.
    let nobody = Description::new("Ada", "test/nobody");
    assert_eq!(resolve(&library, &nobody).unwrap_err().at, "base");
}

#[test]
fn bones_attributes_and_materials_take_the_same_spans_as_shapes() {
    let library = fixtures::library();
    let mut description = Description::new("Ada", "test/body");
    description.parameters.insert("frame".to_owned(), 0.5);
    description.parameters.insert("height".to_owned(), 1.2);
    description.parameters.insert("tone".to_owned(), -0.4);
    let resolved = resolve(&library, &description).unwrap();
    // Offsets scale with the mapped weight; scales go from 1 toward theirs.
    let hips = &resolved.bones["Hips"];
    close(hips.offset[1], 0.035);
    close(hips.offset[2], 0.005);
    close(hips.scale, 1.0);
    let eye = &resolved.bones["LeftEye"];
    assert_eq!(eye.offset, [0.0; 3]);
    close(eye.scale, 1.06);
    close(resolved.attributes["BodyHeight"], 1.2);
    let MaterialValue::Number(melanin) = resolved.materials["Skin"]["melanin"] else {
        panic!("melanin is a number");
    };
    close(melanin, -0.4);
    // The description's own values go over what parameters drive, and
    // values no parameter drives are kept.
    description.materials.insert(
        "Skin".to_owned(),
        [
            ("melanin".to_owned(), MaterialValue::Number(0.25)),
            (
                "tint".to_owned(),
                MaterialValue::Colour(Colour::parse("#ffeedd").unwrap()),
            ),
        ]
        .into(),
    );
    let resolved = resolve(&library, &description).unwrap();
    assert_eq!(
        resolved.materials["Skin"]["melanin"],
        MaterialValue::Number(0.25)
    );
    assert!(resolved.materials["Skin"].contains_key("tint"));
    // At their defaults nothing is driven: a bone that doesn't move isn't
    // listed, and an attribute at its neutral 1 still is (it's set back).
    let plain = resolve(&library, &Description::new("Ada", "test/body")).unwrap();
    assert!(plain.bones.is_empty(), "{:?}", plain.bones);
    close(plain.attributes["BodyHeight"], 1.0);
}

#[test]
fn the_identity_has_a_layer_per_cause_so_each_mesh_reseats_or_is_carried() {
    let library = fixtures::library();
    let mut description = Description::new("Ada", "test/body");
    description.parameters.insert("frame".to_owned(), 1.0);
    description.parameters.insert("fat".to_owned(), 0.5);
    description.items = vec![WornItem::new("test/tee")];
    let resolved = resolve(&library, &description).unwrap();
    // The frame's layer: its shape and its bones together.
    let frame = layer_of(&resolved, "Frame_Feminine");
    assert_eq!(frame.shapes, [("Frame_Feminine".to_owned(), 1.0)]);
    let bones: Vec<&str> = frame.bones.iter().map(|bone| bone.bone.as_str()).collect();
    assert_eq!(bones, ["Hips", "LeftEye"]);
    // A corner is its own layer, with no bones.
    let corner = layer_of(&resolved, "Feminine_Fat");
    assert_eq!(corner.shapes, [("Feminine_Fat".to_owned(), 0.5)]);
    assert!(corner.bones.is_empty());
    // A hidden region's mask is baked at 1, a shown one's at 0.
    assert_eq!(layer_of(&resolved, "Mask_Torso").shapes[0].1, 1.0);
    // Every shape the manifest names is an identity shape, used or not,
    // and the layers add up to the body's shape weights.
    let mut summed: std::collections::BTreeMap<String, f32> = Default::default();
    for (name, weight) in resolved.identity.layers.iter().flat_map(|l| &l.shapes) {
        *summed.entry(name.clone()).or_default() += weight;
    }
    assert_eq!(summed, resolved.shapes);
    assert_eq!(
        resolved.shapes,
        body_shape_weights(&fixtures::base(), &description, &library)
    );
    // The same description gives the same identity key.
    let again = resolve(&library, &description).unwrap();
    assert_eq!(again.identity.key(), resolved.identity.key());
    description.parameters.insert("fat".to_owned(), 0.6);
    let changed = resolve(&library, &description).unwrap();
    assert_ne!(changed.identity.key(), resolved.identity.key());
}

#[test]
fn a_worn_items_values_go_over_its_variants() {
    let library = fixtures::library();
    let mut description = Description::new("Ada", "test/body");
    let mut tee = WornItem::new("test/tee");
    tee.variant = Some("navy".to_owned());
    description.items = vec![tee.clone()];
    let resolved = resolve(&library, &description).unwrap();
    let navy = MaterialValue::Colour(Colour::parse("#1a2340").unwrap());
    assert_eq!(resolved.parts[1].materials["Shirt"]["tint"], navy);
    tee.materials.insert(
        "Shirt".to_owned(),
        [("roughness_scale".to_owned(), MaterialValue::Number(1.2))].into(),
    );
    description.items = vec![tee];
    let resolved = resolve(&library, &description).unwrap();
    assert_eq!(resolved.parts[1].materials["Shirt"]["tint"], navy);
    assert_eq!(
        resolved.parts[1].materials["Shirt"]["roughness_scale"],
        MaterialValue::Number(1.2)
    );
}
