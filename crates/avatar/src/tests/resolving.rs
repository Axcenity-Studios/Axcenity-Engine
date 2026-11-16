//! What parameter values do to the body's shapes.

use crate::fixtures;
use crate::{Description, WornItem, body_shape_weights, parameter_values, span};

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
