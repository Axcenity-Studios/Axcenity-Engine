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

/// The test base with every field of the R1 contracts (plan section 11.3):
/// parameter metadata, group and slot `about` and `icon`, material slot
/// labels, values and roles, features and handles. Same ID as [`BASE`], so
/// the items above fit it too.
pub const RICH_BASE: &str = r##"{
  "format_version": 1,
  "id": "test/body",
  "label": "Test body",
  "family": "test",
  "model": { "file": "Body.glb", "body": "Rig/Body", "parts": ["Rig/Eyes"] },
  "skeleton": { "convention": "humanoid", "height": 1.75, "pose": "a-pose-45" },
  "groups": [
    { "id": "frame", "label": "Frame", "category": "body", "about": "Male or female.", "icon": "body" },
    { "id": "build", "label": "Build", "category": "body" },
    { "id": "eyes", "label": "Eyes", "category": "face", "icon": "eye" },
    { "id": "skin", "label": "Skin", "category": "skin" }
  ],
  "parameters": [
    { "id": "frame", "label": "Body type", "group": "frame", "range": [0, 1], "level": "quick",
      "about": "Male or female.",
      "shapes": [{ "shape": "Frame_Feminine" }],
      "choices": [{ "value": 0, "label": "Male" }, { "value": 1, "label": "Female" }] },
    { "id": "fat", "label": "Fat", "group": "build", "range": [0, 1], "level": "quick",
      "about": "How much body fat.", "feature": "torso", "random": [0, 0.6],
      "shapes": [{ "shape": "Fat" }] },
    { "id": "muscle", "label": "Muscle", "group": "build", "range": [0, 1], "level": "detailed",
      "feature": "torso", "shapes": [{ "shape": "Muscle" }] },
    { "id": "height", "label": "Height", "group": "build", "range": [0.5, 1.5], "default": 1,
      "attributes": [{ "attribute": "BodyHeight", "from": [0.5, 1.5] }] },
    { "id": "eye_size_left", "label": "Left eye size", "group": "eyes", "range": [-1, 1],
      "advanced": true, "level": "advanced", "mirror": "eye_size_right",
      "about": "How big the left eye is.",
      "shapes": [{ "shape": "Eye_Size_Left", "from": [-1, 1] }] },
    { "id": "eye_size_right", "label": "Right eye size", "group": "eyes", "range": [-1, 1],
      "advanced": true, "mirror": "eye_size_left",
      "shapes": [{ "shape": "Eye_Size_Right", "from": [-1, 1] }] },
    { "id": "tone", "label": "Skin tone", "group": "skin", "range": [-1, 1],
      "material_values": [{ "material": "Skin", "value": "melanin", "from": [-1, 1] }] }
  ],
  "constraints": [
    { "kind": "max_sum", "parameters": ["fat", "muscle"], "limit": 1.5 }
  ],
  "regions": [
    { "id": "torso", "label": "Torso", "mask_shape": "Mask_Torso" },
    { "id": "feet", "label": "Feet", "map_index": 7 }
  ],
  "slots": [
    { "id": "hair", "label": "Hairstyle", "category": "hair", "empty_label": "Bald",
      "about": "What grows on the head.", "icon": "hair" },
    { "id": "top", "label": "Top", "category": "clothing" },
    { "id": "bottom", "label": "Bottoms", "category": "clothing" },
    { "id": "outerwear", "label": "Outerwear", "category": "clothing" },
    { "id": "wrist", "label": "Wrist", "category": "accessories", "multiple": true }
  ],
  "sockets": [{ "name": "LeftWristSocket", "bone": "LeftHand" }],
  "materials": [
    { "name": "Skin", "shading": "skin", "label": "Skin", "values": ["melanin", "tint"], "role": "primary" },
    { "name": "Iris", "shading": "eye" }
  ],
  "features": [
    { "id": "torso", "label": "Torso", "category": "body", "about": "Chest and belly.", "icon": "torso",
      "anchors": [{ "bone": "Hips", "offset": [0, 0.25, 0.1] }], "radius": 0.2, "regions": ["torso"] },
    { "id": "eyes", "label": "Eyes", "category": "face",
      "anchors": [{ "bone": "LeftEye", "offset": [0, 0, 0.012] }, { "bone": "RightEye", "offset": [0, 0, 0.012] }],
      "radius": 0.02 }
  ],
  "handles": [
    { "id": "eye_size_left", "feature": "eyes", "mirror": "eye_size_right", "label": "Left eye",
      "anchor": { "bone": "LeftEye", "offset": [0.012, 0, 0.01] },
      "drives": [{ "parameter": "eye_size_left", "axis": [1, 0, 0], "travel": 0.004,
                   "screen": { "front": [1, 0] }, "sensitivity": 1.5 }],
      "views": ["front", "three_quarter"] },
    { "id": "eye_size_right", "feature": "eyes", "mirror": "eye_size_left",
      "anchor": { "bone": "RightEye", "offset": [-0.012, 0, 0.01] },
      "drives": [{ "parameter": "eye_size_right", "axis": [-1, 0, 0], "travel": 0.004 }] }
  ],
  "face": { "profile": "axcenity", "eye_bones": ["LeftEye", "RightEye"], "jaw_bone": "Jaw" }
}"##;

/// A groom with the item capabilities: its own parameters, labelled
/// material slots offering some values, variants with a picture and
/// parameter values, and an item it can't be worn with.
pub const BOB: &str = r##"{
  "format_version": 1,
  "id": "test/bob",
  "label": "Bob",
  "kind": "groom",
  "slot": "hair",
  "bases": ["test/body"],
  "about": "A chin-length bob.",
  "materials": [
    { "name": "Hair", "shading": "hair", "label": "Colour", "values": ["tint", "melanin", "redness"], "role": "primary" },
    { "name": "HairSecondary", "shading": "hair", "label": "Highlights", "values": ["tint"], "role": "accent" }
  ],
  "parameters": [
    { "id": "length", "label": "Length", "range": [0, 1], "default": 0.5, "level": "detailed",
      "about": "From jaw to shoulder.", "random": [0.2, 0.8],
      "shapes": [{ "shape": "Hair_Length" }],
      "material_values": [{ "material": "Hair", "value": "melanin", "from": [0, 1], "to": [0, 0.2] }] },
    { "id": "parting", "label": "Parting", "range": [0, 2], "default": 0,
      "choices": [{ "value": 0, "label": "Middle" }, { "value": 1, "label": "Left" }, { "value": 2, "label": "Right" }],
      "shapes": [{ "shape": "Hair_Part_Left", "from": [0, 1] }, { "shape": "Hair_Part_Right", "from": [1, 2], "to": [0, 1] }] }
  ],
  "variants": [
    { "id": "blonde", "label": "Blonde", "thumbnail": "bob-blonde.png",
      "materials": { "Hair": { "melanin": -0.6 } }, "parameters": { "length": 0.3 } }
  ],
  "excludes": ["test/hood"],
  "limitations": [{ "note": "Clips through high collars.", "parameter": "length", "range": [0, 0.7] }],
  "fit": { "method": "shapes", "follows": ["Frame_Feminine"] },
  "groom": { "cards": { "file": "Bob.glb", "object": "Cards" } }
}"##;

/// A hood the bob can't be worn with.
pub const HOOD: &str = r##"{
  "format_version": 1,
  "id": "test/hood",
  "label": "Hood",
  "kind": "garment",
  "slot": "outerwear",
  "layer": 3,
  "bases": ["test/body"],
  "model": { "file": "Hood.glb" },
  "materials": [{ "name": "Cloth", "shading": "cloth" }],
  "fit": { "method": "binding" }
}"##;

/// A dress: a top that fills the bottom slot too.
pub const DRESS: &str = r##"{
  "format_version": 1,
  "id": "test/dress",
  "label": "Dress",
  "kind": "garment",
  "slot": "top",
  "layer": 1,
  "also_fills": ["bottom"],
  "bases": ["test/body"],
  "model": { "file": "Dress.glb" },
  "materials": [{ "name": "Cloth", "shading": "cloth", "label": "Fabric", "role": "primary" }],
  "fit": { "method": "binding" }
}"##;

pub const SKIRT: &str = r##"{
  "format_version": 1,
  "id": "test/skirt",
  "label": "Skirt",
  "kind": "garment",
  "slot": "bottom",
  "layer": 1,
  "bases": ["test/body"],
  "model": { "file": "Skirt.glb" },
  "materials": [{ "name": "Cloth", "shading": "cloth" }],
  "fit": { "method": "binding" }
}"##;

/// A charm on a socket: it needs the watch, and a person may move, turn
/// and scale it inside its `adjust`.
pub const CHARM: &str = r##"{
  "format_version": 1,
  "id": "test/charm",
  "label": "Charm",
  "kind": "accessory",
  "slot": "wrist",
  "bases": ["test/body"],
  "model": { "file": "Charm.glb" },
  "materials": [{ "name": "Metal", "shading": "standard" }],
  "requires": ["test/watch"],
  "fit": { "method": "socket", "socket": "LeftWristSocket" },
  "adjust": {
    "offset": [[-0.01, 0.01], [-0.01, 0.01], [-0.02, 0.02]],
    "turn": [[-30, 30], [-180, 180], [-30, 30]],
    "scale": [0.8, 1.25]
  }
}"##;

pub fn rich_base() -> BaseManifest {
    BaseManifest::from_json(RICH_BASE).expect("the rich test base reads")
}

/// The rich base with the old items and the new ones.
pub fn rich_library() -> Library {
    let mut library = Library::new();
    library.add_base("test/Body.base.json", rich_base());
    for (file, text) in [
        ("tee", TEE),
        ("hair", HAIR),
        ("watch", WATCH),
        ("bob", BOB),
        ("hood", HOOD),
        ("dress", DRESS),
        ("skirt", SKIRT),
        ("charm", CHARM),
    ] {
        library.add_item(format!("test/items/{file}.item.json"), item(text));
    }
    library
}

/// A preset file's text: a format 2 description with a `preset` block.
pub fn preset(kind: &str, id: &str, body: &str) -> crate::Description {
    let text = format!(
        r#"{{ "format_version": 2, "name": "{id}", "base": "test/body", {body}
             "preset": {{ "id": "test/{id}", "kind": "{kind}", "about": "A test preset." }} }}"#
    );
    crate::Description::from_json(&text).expect("the test preset reads")
}
