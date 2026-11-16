//! A small library for tests: a base model with every kind of driver and
//! rule, a garment, a groom and an accessory, and the contents of their
//! models.

use crate::{BaseManifest, ItemManifest, Library, ModelContents};

pub const BASE: &str = r##"{
  "format_version": 1,
  "id": "test/body",
  "label": "Test body",
  "family": "test",
  "model": { "file": "Body.glb", "body": "Rig/Body", "parts": ["Rig/Eyes"] },
  "skeleton": {
    "convention": "humanoid", "height": 1.75, "pose": "a-pose-45",
    "added_bones": [{ "name": "LeftUpperArmTwist", "parent": "LeftUpperArm" }]
  },
  "groups": [
    { "id": "frame", "label": "Frame", "category": "body" },
    { "id": "build", "label": "Build", "category": "body" },
    { "id": "eyes", "label": "Eyes", "category": "face" },
    { "id": "skin", "label": "Skin", "category": "skin" }
  ],
  "parameters": [
    { "id": "frame", "label": "Masculine to feminine", "group": "frame", "range": [0, 1],
      "shapes": [{ "shape": "Frame_Feminine" }],
      "bones": [{ "bone": "Hips", "offset": [0, 0.07, 0.01] }, { "bone": "LeftEye", "scale": 1.12 }] },
    { "id": "fat", "label": "Fat", "group": "build", "range": [0, 1], "shapes": [{ "shape": "Fat" }] },
    { "id": "muscle", "label": "Muscle", "group": "build", "range": [0, 1], "shapes": [{ "shape": "Muscle" }] },
    { "id": "build", "label": "Slim to heavy", "group": "build", "range": [-1, 1],
      "shapes": [{ "shape": "Heavy" }, { "shape": "Slim", "from": [0, -1], "to": [0, 1] }] },
    { "id": "height", "label": "Height", "group": "build", "range": [0.5, 1.5], "default": 1,
      "attributes": [{ "attribute": "BodyHeight", "from": [0.5, 1.5] }] },
    { "id": "eye_size_left", "label": "Left eye size", "group": "eyes", "range": [-1, 1],
      "advanced": true, "mirror": "eye_size_right",
      "shapes": [{ "shape": "Eye_Size_Left", "from": [-1, 1] }] },
    { "id": "eye_size_right", "label": "Right eye size", "group": "eyes", "range": [-1, 1],
      "advanced": true, "mirror": "eye_size_left",
      "shapes": [{ "shape": "Eye_Size_Right", "from": [-1, 1] }] },
    { "id": "tone", "label": "Skin tone", "group": "skin", "range": [-1, 1],
      "material_values": [{ "material": "Skin", "value": "melanin", "from": [-1, 1] }] }
  ],
  "corners": [
    { "shape": "Feminine_Fat", "factors": [{ "parameter": "frame" }, { "parameter": "fat" }] }
  ],
  "constraints": [
    { "kind": "max_sum", "parameters": ["fat", "muscle"], "limit": 1.5 },
    { "kind": "follow", "source": "muscle", "target": "fat", "factor": -0.3 }
  ],
  "regions": [
    { "id": "torso", "label": "Torso", "mask_shape": "Mask_Torso" },
    { "id": "feet", "label": "Feet", "map_index": 7 }
  ],
  "slots": [
    { "id": "hair", "label": "Hairstyle", "category": "hair", "empty_label": "Bald" },
    { "id": "top", "label": "Top", "category": "clothing" },
    { "id": "wrist", "label": "Wrist", "category": "accessories", "multiple": true }
  ],
  "sockets": [{ "name": "LeftWristSocket", "bone": "LeftHand" }],
  "materials": [{ "name": "Skin", "shading": "skin" }, { "name": "Iris", "shading": "eye" }],
  "face": { "profile": "axcenity", "eye_bones": ["LeftEye", "RightEye"], "jaw_bone": "Jaw" },
  "defaults": { "items": ["test/tee"] }
}"##;

pub const TEE: &str = r##"{
  "format_version": 1,
  "id": "test/tee",
  "label": "T-shirt",
  "kind": "garment",
  "slot": "top",
  "layer": 1,
  "bases": ["test/body"],
  "model": { "file": "Tee.glb", "object": "Rig/Tee" },
  "hides": ["torso"],
  "materials": [{ "name": "Shirt", "shading": "cloth" }],
  "variants": [{ "id": "navy", "label": "Navy", "materials": { "Shirt": { "tint": "#1a2340" } } }],
  "fit": { "method": "binding", "margin": 0.003 },
  "tags": ["casual"],
  "thumbnail": "tee.png",
  "source": { "author": "Axcenity", "licence": "CC0-1.0" }
}"##;

pub const HAIR: &str = r##"{
  "format_version": 1,
  "id": "test/hair",
  "label": "Swept",
  "kind": "groom",
  "slot": "hair",
  "bases": ["test/body"],
  "materials": [{ "name": "Hair", "shading": "hair" }],
  "fit": { "method": "shapes", "follows": ["Frame_Feminine"] },
  "groom": { "cards": { "file": "Hair.glb", "object": "Cards" }, "strands": "Hair.groom" },
  "tier": "premium"
}"##;

pub const WATCH: &str = r##"{
  "format_version": 1,
  "id": "test/watch",
  "label": "Watch",
  "kind": "accessory",
  "slot": "wrist",
  "bases": ["test/body"],
  "model": { "file": "Watch.glb" },
  "materials": [{ "name": "Metal", "shading": "standard" }],
  "fit": { "method": "socket", "socket": "LeftWristSocket" }
}"##;

pub fn base() -> BaseManifest {
    BaseManifest::from_json(BASE).expect("the test base reads")
}

pub fn item(text: &str) -> ItemManifest {
    ItemManifest::from_json(text).expect("the test item reads")
}

pub fn library() -> Library {
    let mut library = Library::new();
    library.add_base("test/Body.base.json", base());
    library.add_item("test/items/tee.item.json", item(TEE));
    library.add_item("test/items/hair.item.json", item(HAIR));
    library.add_item("test/items/watch.item.json", item(WATCH));
    library
}

fn names(list: &[&str]) -> std::collections::BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

/// What the test body's model has.
pub fn body_contents() -> ModelContents {
    ModelContents {
        shapes: names(&[
            "Frame_Feminine",
            "Fat",
            "Muscle",
            "Heavy",
            "Slim",
            "Eye_Size_Left",
            "Eye_Size_Right",
            "Feminine_Fat",
            "Mask_Torso",
        ]),
        bones: names(&[
            "Root",
            "Hips",
            "LeftUpperArm",
            "LeftUpperArmTwist",
            "LeftHand",
            "LeftEye",
            "RightEye",
            "Jaw",
        ]),
        materials: names(&["Skin"]),
    }
}

/// What the test body's eyes have.
pub fn eyes_contents() -> ModelContents {
    ModelContents {
        bones: body_contents().bones,
        materials: names(&["Iris"]),
        ..ModelContents::default()
    }
}

/// What a model on the test skeleton with these shapes and materials has.
pub fn contents(shapes: &[&str], materials: &[&str]) -> ModelContents {
    ModelContents {
        shapes: names(shapes),
        bones: body_contents().bones,
        materials: names(materials),
    }
}
