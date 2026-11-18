//! Writing a fitted garment as a binary glTF (`.glb`): its pieces skinned
//! to a copy of the body's skeleton, with its carried shapes as morph
//! targets named as the body's, so the runtime drives them with the
//! body's weights.
//!
//! A `.glb` is a 12-byte header, a JSON chunk describing the scene and a
//! BIN chunk with the data the JSON points into. Morph targets are written
//! *sparse* (only the corners a shape moves), so the many identity shapes
//! cost little.

use axcenity_assets::Rig;
use glam::{Mat4, Vec3};
use serde_json::{Value, json};

use super::transfer::Fitted;

const FLOAT: u32 = 5126;
const UNSIGNED_INT: u32 = 5125;
const UNSIGNED_SHORT: u32 = 5123;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;

/// What to write: the fitted garment, the body's skeleton, the name of the
/// object holding the skeleton (`AxcenityRealistic`, so the garment's
/// object path is `AxcenityRealistic/<object>` like the body's) and the
/// garment object's name.
pub struct FittedModel<'a> {
    pub fitted: &'a Fitted,
    pub rig: &'a Rig,
    pub armature: &'a str,
    pub object: &'a str,
    /// The shading model each material is drawn with (`cloth`).
    pub shading: &'a str,
}

#[derive(Default)]
struct Buffer {
    bytes: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Buffer {
    fn view(&mut self, data: &[u8], target: Option<u32>) -> usize {
        while !self.bytes.len().is_multiple_of(4) {
            self.bytes.push(0);
        }
        let mut view = json!({
            "buffer": 0,
            "byteOffset": self.bytes.len(),
            "byteLength": data.len(),
        });
        if let Some(target) = target {
            view["target"] = json!(target);
        }
        self.bytes.extend_from_slice(data);
        self.views.push(view);
        self.views.len() - 1
    }

    fn accessor(&mut self, accessor: Value) -> usize {
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }

    fn vec3s(&mut self, values: &[Vec3], bounds: bool) -> usize {
        let data: Vec<u8> = values
            .iter()
            .flat_map(|v| v.to_array())
            .flat_map(f32::to_le_bytes)
            .collect();
        let view = self.view(&data, Some(ARRAY_BUFFER));
        let mut accessor = json!({
            "bufferView": view, "componentType": FLOAT, "count": values.len(), "type": "VEC3",
        });
        if bounds {
            let (lo, hi) = min_max(values);
            accessor["min"] = json!(lo.to_array());
            accessor["max"] = json!(hi.to_array());
        }
        self.accessor(accessor)
    }

    fn floats<const N: usize>(&mut self, values: &[[f32; N]], kind: &str) -> usize {
        let data: Vec<u8> = values
            .iter()
            .flatten()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let view = self.view(&data, Some(ARRAY_BUFFER));
        self.accessor(json!({
            "bufferView": view, "componentType": FLOAT, "count": values.len(), "type": kind,
        }))
    }

    fn joints(&mut self, values: &[[u16; 4]]) -> usize {
        let data: Vec<u8> = values
            .iter()
            .flatten()
            .flat_map(|j| j.to_le_bytes())
            .collect();
        let view = self.view(&data, Some(ARRAY_BUFFER));
        self.accessor(json!({
            "bufferView": view, "componentType": UNSIGNED_SHORT, "count": values.len(), "type": "VEC4",
        }))
    }

    fn indices(&mut self, values: &[u32]) -> usize {
        let data: Vec<u8> = values.iter().flat_map(|i| i.to_le_bytes()).collect();
        let view = self.view(&data, Some(ELEMENT_ARRAY_BUFFER));
        self.accessor(json!({
            "bufferView": view, "componentType": UNSIGNED_INT, "count": values.len(), "type": "SCALAR",
        }))
    }

    /// A sparse VEC3 accessor of `count` values, zero except `values` at
    /// `corners`. glTF wants at least one sparse value: a shape that moves
    /// nothing here keeps a zero at corner 0.
    fn sparse(&mut self, count: usize, corners: &[u32], values: &[Vec3]) -> usize {
        let (corners, values) = if corners.is_empty() {
            (vec![0u32], vec![Vec3::ZERO])
        } else {
            (corners.to_vec(), values.to_vec())
        };
        let index_data: Vec<u8> = corners.iter().flat_map(|i| i.to_le_bytes()).collect();
        let indices = self.view(&index_data, None);
        let value_data: Vec<u8> = values
            .iter()
            .flat_map(|v| v.to_array())
            .flat_map(f32::to_le_bytes)
            .collect();
        let view = self.view(&value_data, None);
        // Bounds over every value, the zeros included.
        let (lo, hi) = min_max(&values);
        let (lo, hi) = (lo.min(Vec3::ZERO), hi.max(Vec3::ZERO));
        self.accessor(json!({
            "componentType": FLOAT, "count": count, "type": "VEC3",
            "min": lo.to_array(), "max": hi.to_array(),
            "sparse": {
                "count": corners.len(),
                "indices": {"bufferView": indices, "componentType": UNSIGNED_INT},
                "values": {"bufferView": view},
            },
        }))
    }
}

fn min_max(values: &[Vec3]) -> (Vec3, Vec3) {
    values
        .iter()
        .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(*v), hi.max(*v))
        })
}

/// The `.glb` file's bytes.
pub fn write_fitted_glb(model: &FittedModel) -> Vec<u8> {
    let FittedModel {
        fitted,
        rig,
        armature,
        object,
        shading,
    } = model;
    let mut buffer = Buffer::default();

    // The skeleton: one node per bone, after the armature node (unless a
    // bone already has its name), then the garment's node.
    let own_armature = rig.bones.iter().all(|bone| bone.name != *armature);
    let first_bone = usize::from(own_armature);
    let mut nodes: Vec<Value> = Vec::new();
    if own_armature {
        nodes.push(json!({"name": armature, "children": []}));
    }
    for bone in &rig.bones {
        let rest = bone.rest;
        nodes.push(json!({
            "name": bone.name,
            "translation": rest.translation.to_array(),
            "rotation": rest.rotation.to_array(),
            "scale": rest.scale.to_array(),
        }));
    }
    let add_child = |nodes: &mut Vec<Value>, parent: usize, child: usize| {
        let children = nodes[parent]
            .as_object_mut()
            .expect("a node")
            .entry("children")
            .or_insert_with(|| json!([]));
        children.as_array_mut().expect("a list").push(json!(child));
    };
    let mut roots = Vec::new();
    for (index, bone) in rig.bones.iter().enumerate() {
        match bone.parent {
            Some(parent) => add_child(&mut nodes, first_bone + parent, first_bone + index),
            None => roots.push(first_bone + index),
        }
    }
    let holder = if own_armature {
        for root in &roots {
            add_child(&mut nodes, 0, *root);
        }
        0
    } else {
        first_bone
            + rig
                .bones
                .iter()
                .position(|bone| bone.name == *armature)
                .unwrap_or(0)
    };
    let mesh_node = nodes.len();
    nodes.push(json!({"name": object, "mesh": 0, "skin": 0}));
    add_child(&mut nodes, holder, mesh_node);
    let scene_roots: Vec<usize> = if own_armature { vec![0] } else { roots };

    let binds: Vec<[f32; 16]> = rig
        .joints
        .iter()
        .map(|joint| Mat4::to_cols_array(&joint.inverse_bind))
        .collect();
    let binds = buffer.floats(&binds, "MAT4");
    let skin = json!({
        "inverseBindMatrices": binds,
        "joints": rig.joints.iter().map(|joint| first_bone + joint.bone).collect::<Vec<_>>(),
    });

    // One primitive and one material per piece.
    let mut primitives = Vec::new();
    let mut materials = Vec::new();
    let mut start = 0usize;
    for (index, piece) in fitted.garment.pieces.iter().enumerate() {
        let count = piece.positions.len();
        let range = start..start + count;
        let mut attributes = json!({
            "POSITION": buffer.vec3s(&piece.positions, true),
            "NORMAL": buffer.vec3s(&normals_or_up(&piece.normals, count), false),
            "JOINTS_0": buffer.joints(&fitted.joints[range.clone()]),
            "WEIGHTS_0": buffer.floats(&fitted.weights[range.clone()], "VEC4"),
        });
        if piece.uvs.len() == count {
            attributes["TEXCOORD_0"] = json!(buffer.floats(&piece.uvs, "VEC2"));
        }
        let targets: Vec<Value> = fitted
            .shapes
            .iter()
            .map(|shape| {
                let moved: Vec<u32> = (0..count as u32)
                    .filter(|&k| shape.offsets[start + k as usize].length() > 0.0)
                    .collect();
                let offsets: Vec<Vec3> = moved
                    .iter()
                    .map(|&k| shape.offsets[start + k as usize])
                    .collect();
                let normals: Vec<Vec3> = moved
                    .iter()
                    .map(|&k| shape.normals[start + k as usize])
                    .collect();
                json!({
                    "POSITION": buffer.sparse(count, &moved, &offsets),
                    "NORMAL": buffer.sparse(count, &moved, &normals),
                })
            })
            .collect();
        let mut primitive = json!({
            "attributes": attributes,
            "indices": buffer.indices(&piece.indices),
            "material": index,
        });
        if !targets.is_empty() {
            primitive["targets"] = json!(targets);
        }
        primitives.push(primitive);
        let name = if piece.material.is_empty() {
            format!("Cloth{}", index + 1)
        } else {
            piece.material.clone()
        };
        materials.push(json!({
            "name": name,
            "doubleSided": true,
            "pbrMetallicRoughness": {
                "baseColorFactor": piece.colour,
                "metallicFactor": 0.0,
                "roughnessFactor": piece.roughness,
            },
            "extras": {"axcenity_shading": shading},
        }));
        start += count;
    }
    let mut mesh = json!({"name": object, "primitives": primitives});
    if !fitted.shapes.is_empty() {
        mesh["extras"] = json!({
            "targetNames": fitted.shapes.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
        });
        mesh["weights"] = json!(vec![0.0; fitted.shapes.len()]);
    }

    while !buffer.bytes.len().is_multiple_of(4) {
        buffer.bytes.push(0);
    }
    let document = json!({
        "asset": {"version": "2.0", "generator": "Axcenity Engine garment fitting (CO9)"},
        "scene": 0,
        "scenes": [{"nodes": scene_roots}],
        "nodes": nodes,
        "skins": [skin],
        "meshes": [mesh],
        "materials": materials,
        "buffers": [{"byteLength": buffer.bytes.len()}],
        "bufferViews": buffer.views,
        "accessors": buffer.accessors,
    });
    glb(&document, &buffer.bytes)
}

fn normals_or_up(normals: &[Vec3], count: usize) -> Vec<Vec3> {
    if normals.len() == count {
        normals.to_vec()
    } else {
        vec![Vec3::Y; count]
    }
}

/// A `.glb` of `document` and its binary chunk.
fn glb(document: &Value, bin: &[u8]) -> Vec<u8> {
    let mut text = serde_json::to_vec(document).unwrap_or_default();
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(bin);
    out
}
