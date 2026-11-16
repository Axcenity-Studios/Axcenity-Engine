//! Manifests against their models: every shape, bone and material a
//! manifest names must exist in the model it describes, and every material
//! of the model must have a shading model.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{BaseManifest, Fit, ItemManifest, Problem};

/// A mesh in a glTF file: the file (relative to the manifest) and the
/// object's path in it (a `MeshId` pick, such as
/// `AxcenityHumanoid/Shirt_Tee`); the whole file when `object` is unset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRef {
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
}

/// What a loaded model has that manifests name: its blend shapes, its
/// skeleton's bones and its materials, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModelContents {
    pub shapes: BTreeSet<String>,
    pub bones: BTreeSet<String>,
    pub materials: BTreeSet<String>,
}

impl ModelContents {
    pub fn of(model: &axcenity_assets::Model) -> Self {
        Self {
            shapes: model.morph_targets.iter().map(|t| t.name.clone()).collect(),
            bones: model
                .rig
                .iter()
                .flat_map(|rig| rig.bones.iter().map(|bone| bone.name.clone()))
                .collect(),
            materials: model
                .primitives
                .iter()
                .map(|primitive| primitive.material.name.clone())
                .collect(),
        }
    }
}

impl BaseManifest {
    /// Checks the manifest against its model: `body` is the body object's
    /// contents and `parts` each always-worn part's, in
    /// [`crate::BaseModel::parts`] order. Every shape a parameter, corner
    /// or region names is a body shape; every bone a driver, socket, the
    /// face or an added bone names is a body bone; the materials listed
    /// are exactly those of the body and its parts.
    pub fn check_model(&self, body: &ModelContents, parts: &[ModelContents]) -> Vec<Problem> {
        let mut problems = Vec::new();
        let p = &mut problems;
        if parts.len() != self.model.parts.len() {
            p.push(Problem::new(
                "model.parts",
                format!(
                    "lists {} parts; contents were given for {}",
                    self.model.parts.len(),
                    parts.len()
                ),
            ));
        }
        let shape = |p: &mut Vec<Problem>, at: String, name: &str| {
            if !body.shapes.contains(name) {
                p.push(Problem::new(at, format!("the body has no shape `{name}`")));
            }
        };
        let bone = |p: &mut Vec<Problem>, at: String, name: &str| {
            if !body.bones.contains(name) {
                p.push(Problem::new(
                    at,
                    format!("the skeleton has no bone `{name}`"),
                ));
            }
        };
        for (index, parameter) in self.parameters.iter().enumerate() {
            for (k, driver) in parameter.shapes.iter().enumerate() {
                shape(p, format!("parameters[{index}].shapes[{k}]"), &driver.shape);
            }
            for (k, driver) in parameter.bones.iter().enumerate() {
                bone(p, format!("parameters[{index}].bones[{k}]"), &driver.bone);
            }
        }
        for (index, corner) in self.corners.iter().enumerate() {
            shape(p, format!("corners[{index}]"), &corner.shape);
        }
        for (index, region) in self.regions.iter().enumerate() {
            if let Some(mask) = &region.mask_shape {
                shape(p, format!("regions[{index}]"), mask);
            }
        }
        for (index, socket) in self.sockets.iter().enumerate() {
            bone(p, format!("sockets[{index}]"), &socket.bone);
        }
        if let Some(face) = &self.face {
            for (index, eye) in face.eye_bones.iter().enumerate() {
                bone(p, format!("face.eye_bones[{index}]"), eye);
            }
            if let Some(jaw) = &face.jaw_bone {
                bone(p, "face.jaw_bone".to_owned(), jaw);
            }
        }
        for (index, added) in self.skeleton.added_bones.iter().enumerate() {
            let at = format!("skeleton.added_bones[{index}]");
            bone(p, at.clone(), &added.name);
            bone(p, at, &added.parent);
        }
        let mut materials = body.materials.clone();
        for part in parts {
            materials.extend(part.materials.iter().cloned());
        }
        let listed: BTreeSet<String> = self.materials.iter().map(|m| m.name.clone()).collect();
        check_materials(p, &listed, &materials, "the body and its parts");
        problems
    }
}

impl ItemManifest {
    /// Checks the item against its model (`item`, its contents; for a
    /// groom, every tier's together) and the body it fits (`body`): the
    /// materials listed are exactly the model's, the shapes it follows
    /// exist on it and on the body, and its bones are the body's.
    pub fn check_model(&self, item: &ModelContents, body: &ModelContents) -> Vec<Problem> {
        let mut problems = Vec::new();
        let p = &mut problems;
        if let Fit::Shapes { follows } = &self.fit {
            for (index, shape) in follows.iter().enumerate() {
                let at = format!("fit.follows[{index}]");
                if !item.shapes.contains(shape) {
                    p.push(Problem::new(
                        &at,
                        format!("the item has no shape `{shape}`"),
                    ));
                }
                if !body.shapes.contains(shape) {
                    p.push(Problem::new(at, format!("the body has no shape `{shape}`")));
                }
            }
        }
        for bone in item.bones.difference(&body.bones) {
            p.push(Problem::new(
                "model",
                format!("its skeleton has the bone `{bone}`, which the body's hasn't"),
            ));
        }
        let listed: BTreeSet<String> = self.materials.iter().map(|m| m.name.clone()).collect();
        check_materials(p, &listed, &item.materials, "its model");
        problems
    }
}

/// The materials a manifest lists are exactly those its model has.
fn check_materials(
    p: &mut Vec<Problem>,
    listed: &BTreeSet<String>,
    found: &BTreeSet<String>,
    model: &str,
) {
    for name in listed.difference(found) {
        p.push(Problem::new(
            "materials",
            format!("`{name}` isn't a material of {model}"),
        ));
    }
    for name in found.difference(listed) {
        p.push(Problem::new(
            "materials",
            format!("{model} has the material `{name}`, which has no shading model here"),
        ));
    }
}
