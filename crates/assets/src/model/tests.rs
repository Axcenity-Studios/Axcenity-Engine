use super::*;

/// The example project's model: a square pyramid 4 wide and 3 tall,
/// with a checkered texture, all embedded in one `.gltf` file.
fn example_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-mesh/assets/models")
}

fn example_bytes() -> Vec<u8> {
    std::fs::read(example_folder().join("pyramid.gltf")).unwrap()
}

#[test]
fn a_fluttering_material_is_a_leaf_unless_it_says_otherwise() {
    let of = |text: &str| {
        let raw = serde_json::value::RawValue::from_string(text.to_owned()).unwrap();
        super::wind_of(&Some(raw))
    };
    assert_eq!(
        of(r#"{"axcenity_flutter":0.2}"#).translucency,
        super::LEAF_TRANSLUCENCY
    );
    let opaque = of(r#"{"axcenity_flutter":0.2,"axcenity_translucency":0}"#);
    assert_eq!(opaque.translucency, 0.0);
    assert_eq!(of(r#"{"axcenity_sway":0.1}"#).translucency, 0.0);
    assert_eq!(of(r#"{"axcenity_translucency":3}"#).translucency, 1.0);
}

#[test]
fn a_gltf_materials_extras_say_it_is_skin_and_name_its_defined_normal_map() {
    let of = |text: &str| {
        let raw = serde_json::value::RawValue::from_string(text.to_owned()).unwrap();
        super::shading_of(&Some(raw))
    };
    assert_eq!(
        of(r#"{"axcenity_shading": "skin", "axcenity_defined_normal": {"index": 3}}"#),
        (ModelShading::Skin, Some(3))
    );
    assert_eq!(
        of(r#"{"axcenity_shading": "skin"}"#),
        (ModelShading::Skin, None)
    );
    // A defined map only means something on skin; an unknown model (a
    // newer file's "eye", say) draws as standard.
    assert_eq!(
        of(r#"{"axcenity_defined_normal": {"index": 3}}"#),
        (ModelShading::Standard, None)
    );
    assert_eq!(
        of(r#"{"axcenity_shading": "eye"}"#),
        (ModelShading::Standard, None)
    );
    assert_eq!(
        of(r#"{"axcenity_shading": 7, "axcenity_sway": 0.2}"#),
        (ModelShading::Standard, None)
    );
    assert_eq!(super::shading_of(&None), (ModelShading::Standard, None));
}

/// A skin names its makeup and marking masks (ADR-0042): three textures,
/// in order, or none.
#[test]
fn a_skins_extras_name_its_three_layer_masks() {
    let of = |text: &str| {
        let raw = serde_json::value::RawValue::from_string(text.to_owned()).unwrap();
        super::skin_masks_of(&Some(raw))
    };
    assert_eq!(
        of(r#"{"axcenity_shading": "skin",
              "axcenity_skin_masks": [{"index": 4}, {"index": 6}, {"index": 5}]}"#),
        vec![4, 6, 5]
    );
    // Two or four: a file this version can't read the layout of.
    assert!(
        of(r#"{"axcenity_shading": "skin", "axcenity_skin_masks": [{"index": 4}, {"index": 6}]}"#)
            .is_empty()
    );
    assert!(of(r#"{"axcenity_shading": "skin", "axcenity_skin_masks": 3}"#).is_empty());
    // Only skin has them.
    assert!(
        of(r#"{"axcenity_skin_masks": [{"index": 4}, {"index": 6}, {"index": 5}]}"#).is_empty()
    );
    assert!(super::skin_masks_of(&None).is_empty());
}

/// The realistic skin's masks, as the build writes them (`makeup.py`):
/// three images the size of its maps, each channel a layer that covers
/// some of the skin and not all of it, and the spots' channels many
/// small separate spots.
#[test]
fn the_realistic_skin_has_its_makeup_and_marking_masks() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters/realistic");
    let bytes = std::fs::read(folder.join("RealisticParts.glb")).unwrap();
    let model = load_model(&bytes, &folder, ModelFormat::Gltf).unwrap();
    let skin = model
        .primitives
        .iter()
        .map(|primitive| &primitive.material)
        .find(|material| material.name == "Skin")
        .expect("a Skin material");
    assert_eq!(skin.skin_masks.len(), 3, "three masks");
    let albedo = skin.base_color_texture.as_ref().unwrap();
    // Foundation, blush, eyeshadow, eyeliner; lips, brow fill, scar, (none);
    // freckles and their ranks, moles and their ranks.
    let names = [
        ["foundation", "blush", "eyeshadow", "eyeliner"],
        ["lips", "brow fill", "scar", ""],
        ["freckles", "freckle ranks", "moles", "mole ranks"],
    ];
    for (mask, names) in skin.skin_masks.iter().zip(names) {
        assert_eq!((mask.width, mask.height), (albedo.width, albedo.height));
        let texels = mask.rgba();
        let count = f64::from(mask.width * mask.height);
        for (channel, name) in names.iter().enumerate() {
            let covered = texels
                .chunks_exact(4)
                .filter(|texel| texel[channel] > 127)
                .count() as f64;
            if name.is_empty() {
                assert_eq!(covered, 0.0, "the spare channel is empty");
                continue;
            }
            let share = covered / count;
            assert!(
                share > 0.00002 && share < 0.25,
                "{name} covers {share} of the map"
            );
        }
    }
    // Spots: each freckle and mole is its own small patch.
    let markings = skin.skin_masks[2].rgba();
    let (width, height) = (
        skin.skin_masks[2].width as usize,
        skin.skin_masks[2].height as usize,
    );
    for (channel, least, largest) in [(0, 2000, 60), (2, 40, 300)] {
        let on: Vec<bool> = markings
            .chunks_exact(4)
            .map(|texel| texel[channel] > 127)
            .collect();
        let (count, biggest) = patches(&on, width, height);
        assert!(count >= least, "channel {channel}: {count} spots");
        assert!(
            biggest <= largest,
            "channel {channel}: a spot of {biggest} texels"
        );
    }
}

/// How many 4-connected patches `on` has, and the largest one's texels.
fn patches(on: &[bool], width: usize, height: usize) -> (usize, usize) {
    let mut seen = vec![false; on.len()];
    let (mut count, mut biggest) = (0, 0);
    for start in 0..on.len() {
        if !on[start] || seen[start] {
            continue;
        }
        count += 1;
        let mut size = 0;
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(at) = stack.pop() {
            size += 1;
            let (x, y) = (at % width, at / width);
            let mut neighbours = Vec::with_capacity(4);
            if x > 0 {
                neighbours.push(at - 1);
            }
            if x + 1 < width {
                neighbours.push(at + 1);
            }
            if y > 0 {
                neighbours.push(at - width);
            }
            if y + 1 < height {
                neighbours.push(at + width);
            }
            for next in neighbours {
                if on[next] && !seen[next] {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
        biggest = biggest.max(size);
    }
    (count, biggest)
}

/// FNV-1a, 64 bits: the build (`makeup.py`, `digest`) records each mask's
/// bytes by it, so a test can tell the file's masks are the ones measured.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The eyeliner's wing continues the lash line out of the eye's outer
/// corner (R5 review: it rose 60 to 70 degrees, a spike beside the eye).
/// The build measures each side's wing on the face at rest, in the eye's
/// own frame (`makeup.py`, `wing_report`): it leaves from the outer corner
/// of the lid's skin the liner runs along (within 2 mm of where the lids'
/// margins meet), runs 5 to 7 mm along the skin, and rises 10 to 30
/// degrees from the line through the eye's corners, on the face's plane
/// and along the skin. The masks in the model file are the ones measured.
#[test]
fn the_eyeliner_wing_continues_the_lash_line_out_of_the_corner() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters/realistic");
    let build: serde_json::Value =
        serde_json::from_slice(&std::fs::read(folder.join("Realistic.build.json")).unwrap())
            .unwrap();
    let makeup = &build["textures"]["makeup"];
    let wings = makeup["wing"]
        .as_array()
        .expect("the build measures the wing");
    assert_eq!(wings.len(), 2, "a wing each side");
    for wing in wings {
        let number = |key: &str| wing[key].as_f64().unwrap_or(f64::NAN);
        let (rise, along_skin) = (number("rise_degrees"), number("skin_degrees"));
        let (length, corner) = (number("length_mm"), number("from_corner_mm"));
        assert!(
            (10.0..=30.0).contains(&rise),
            "the wing rises {rise} degrees"
        );
        assert!(
            (0.0..=30.0).contains(&along_skin),
            "along the skin the wing turns {along_skin} degrees from the corners' line"
        );
        assert!(
            (5.0..=7.0).contains(&length),
            "the wing is {length} mm long"
        );
        assert!(corner <= 2.0, "the wing leaves {corner} mm from the corner");
    }
    // The model file's masks are the ones the build measured.
    let bytes = std::fs::read(folder.join("RealisticParts.glb")).unwrap();
    let gltf = gltf::Gltf::from_slice(&bytes).unwrap();
    let blob = gltf.blob.as_deref().expect("a binary chunk");
    let digests = makeup["digests"].as_object().expect("the masks' digests");
    assert_eq!(digests.len(), 3);
    for (file, digest) in digests {
        let stem = file.trim_end_matches(".png");
        let image = gltf
            .images()
            .find(|image| image.name() == Some(stem))
            .unwrap_or_else(|| panic!("{stem} in the model file"));
        let gltf::image::Source::View { view, .. } = image.source() else {
            panic!("{stem} is embedded");
        };
        let png = &blob[view.offset()..view.offset() + view.length()];
        assert_eq!(
            format!("{:016x}", fnv1a(png)),
            digest.as_str().unwrap(),
            "{file} in the model file isn't the mask the build measured"
        );
    }
}

/// The skin's grain reads as pores, not cracks (R5, wave 2's open issue):
/// the build measures its bump's height on the belly (`skin.py`,
/// `grain_shape`). Pores, isolated pits in a level surface, give the
/// height a long low tail: its skewness is strongly negative. Wave 2's
/// grooves along every cell's edge measured +0.05 (as much low as high).
#[test]
fn the_realistic_skins_grain_is_pores_not_a_crackle_network() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters/realistic");
    let build: serde_json::Value =
        serde_json::from_slice(&std::fs::read(folder.join("Realistic.build.json")).unwrap())
            .unwrap();
    let grain = &build["textures"]["grain"];
    let skew = grain["height_skew"]
        .as_f64()
        .expect("the build measures the grain's height");
    assert!(skew < -1.5, "the grain's height skews {skew}: no pores");
    let pores = grain["pores_per_cm2"].as_f64().expect("pores per cm2");
    assert!((0.5..=20.0).contains(&pores), "{pores} pores per cm2");
}

/// The realistic body's skin, as the build writes it: marked skin, with
/// its smooth normal map as glTF's own and the defined one beside it.
#[test]
fn the_realistic_skin_is_marked_skin_with_a_smooth_and_a_defined_normal_map() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters/realistic");
    let bytes = std::fs::read(folder.join("RealisticParts.glb")).unwrap();
    let model = load_model(&bytes, &folder, ModelFormat::Gltf).unwrap();
    let skin = model
        .primitives
        .iter()
        .map(|primitive| &primitive.material)
        .find(|material| material.name == "Skin")
        .expect("a Skin material");
    assert_eq!(skin.shading, ModelShading::Skin);
    let smooth = skin.normal_texture.as_ref().expect("a smooth normal map");
    let defined = skin
        .defined_normal_texture
        .as_ref()
        .expect("a defined normal map");
    assert_eq!(
        (smooth.width, smooth.height),
        (defined.width, defined.height)
    );
    assert_ne!(smooth.rgba(), defined.rgba(), "the two maps differ");
    // Every other material draws as standard (the mouth shares the maps).
    for primitive in &model.primitives {
        if primitive.material.name != "Skin" {
            assert_eq!(
                primitive.material.shading,
                ModelShading::Standard,
                "{}",
                primitive.material.name
            );
        }
    }
    // The manifest's `materials[].shading` and the file agree on what is
    // skin (architecture.md §10): every material the manifest calls skin
    // is marked skin in the file, and only those. (The file marks only
    // skin so far: an "eye" or "hair" material draws as standard.)
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(folder.join("Realistic.base.json")).unwrap())
            .unwrap();
    let said_skin: Vec<&str> = manifest["materials"]
        .as_array()
        .expect("the manifest's materials")
        .iter()
        .filter(|material| material["shading"] == "skin")
        .map(|material| material["name"].as_str().unwrap())
        .collect();
    let mut marked_skin: Vec<&str> = model
        .primitives
        .iter()
        .filter(|primitive| primitive.material.shading == ModelShading::Skin)
        .map(|primitive| primitive.material.name.as_str())
        .collect();
    marked_skin.sort_unstable();
    marked_skin.dedup();
    assert_eq!(said_skin, marked_skin, "the manifest's skin and the file's");
}

/// The navel stays on every body: the smooth normal map (what fat makes
/// of the defined one) keeps it where the build found it
/// (`Realistic.build.json`'s `textures.navel_uv`), so the two maps agree
/// round it.
#[test]
fn the_realistic_skins_smooth_normal_map_keeps_the_navel() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters/realistic");
    let build: serde_json::Value =
        serde_json::from_slice(&std::fs::read(folder.join("Realistic.build.json")).unwrap())
            .unwrap();
    let navel = &build["textures"]["navel_uv"];
    let (u, v) = (
        navel[0].as_f64().expect("the navel's u"),
        navel[1].as_f64().expect("the navel's v"),
    );
    let bytes = std::fs::read(folder.join("RealisticParts.glb")).unwrap();
    let model = load_model(&bytes, &folder, ModelFormat::Gltf).unwrap();
    let skin = model
        .primitives
        .iter()
        .map(|primitive| &primitive.material)
        .find(|material| material.name == "Skin")
        .expect("a Skin material");
    let smooth = skin.normal_texture.as_ref().unwrap();
    let defined = skin.defined_normal_texture.as_ref().unwrap();
    let (width, height) = (smooth.width as i64, smooth.height as i64);
    let (x, y) = ((u * width as f64) as i64, (v * height as f64) as i64);
    // Within 12 texels (about a centimetre: the navel and its rim), the
    // mean difference of every channel, in steps of 255.
    let reach = 12;
    let (mut sum, mut count, mut most) = (0.0, 0.0, 0u8);
    // How far the defined map leans off flat (128 in red and green) there:
    // the navel's walls, so the place the build chose isn't a flat spot
    // (78 at the navel; 19 to 35 on the belly 40 to 120 texels beside it).
    let mut leaning = 0u8;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            if dx * dx + dy * dy > reach * reach {
                continue;
            }
            let at = (((y + dy).clamp(0, height - 1) * width + (x + dx).clamp(0, width - 1)) * 4)
                as usize;
            for c in 0..3 {
                let apart = smooth.rgba()[at + c].abs_diff(defined.rgba()[at + c]);
                sum += f64::from(apart);
                most = most.max(apart);
                count += 1.0;
            }
            for c in 0..2 {
                leaning = leaning.max(defined.rgba()[at + c].abs_diff(128));
            }
        }
    }
    let mean = sum / count;
    eprintln!("at the navel ({x}, {y}) the defined map leans up to {leaning} steps off flat");
    assert!(
        leaning >= 50,
        "the navel's place ({x}, {y}) is a dimple in the defined map: it leans {leaning} steps at most"
    );
    assert!(
        mean < 1.5 && most < 12,
        "the smooth map flattens the navel at ({x}, {y}): the maps differ by {mean:.2} on average, {most} at most"
    );
}

#[test]
fn the_example_pyramid_loads_fitted_into_the_unit_cube() {
    let model = load_model(&example_bytes(), &example_folder(), ModelFormat::Gltf).unwrap();
    assert_eq!(model.size, [4.0, 3.0, 4.0]);
    assert_eq!(model.primitives.len(), 1);
    let primitive = &model.primitives[0];
    assert_eq!(primitive.normals.len(), primitive.positions.len());
    assert_eq!(primitive.uvs.len(), primitive.positions.len());
    for corner in &primitive.positions {
        assert!(corner.iter().all(|c| c.abs() <= 0.5 + 1e-6), "{corner:?}");
    }
    // The apex and the 4 base corners.
    assert_eq!(model.hull_points.len(), 5);
    let texture = primitive.material.base_color_texture.as_ref().unwrap();
    assert_eq!(
        texture.rgba().len(),
        texture.width as usize * texture.height as usize * 4
    );
}

#[test]
fn each_load_is_a_different_model() {
    let a = load_model(&example_bytes(), &example_folder(), ModelFormat::Gltf).unwrap();
    let b = load_model(&example_bytes(), &example_folder(), ModelFormat::Gltf).unwrap();
    assert_ne!(a, b);
    assert_ne!(a.primitives[0].key(), b.primitives[0].key());
}

fn error(text: &str) -> String {
    load_model(text.as_bytes(), &example_folder(), ModelFormat::Gltf)
        .unwrap_err()
        .to_string()
}

#[test]
fn broken_files_give_errors_not_crashes() {
    assert!(error("not a model").contains("isn't a valid glTF file"));
    assert!(error("{}").contains("isn't a valid glTF file"));
    // Valid JSON, but the mesh points at an accessor that isn't there.
    // `gltf` panics while checking it, which is caught.
    let bad_accessor = r#"{ "asset": { "version": "2.0" },
        "meshes": [ { "primitives": [ { "attributes": { "POSITION": 7 } } ] } ] }"#;
    let bad_accessor = error(bad_accessor);
    assert!(
        bad_accessor.contains("isn't a valid glTF file") || bad_accessor == "the file is damaged",
        "{bad_accessor}"
    );
    // A model with nothing to draw.
    assert_eq!(
        error(r#"{ "asset": { "version": "2.0" } }"#),
        "it has no triangles to draw"
    );
}

#[test]
fn files_outside_the_folder_or_missing_are_named() {
    let using = |uri: &str| {
        format!(
            r#"{{ "asset": {{ "version": "2.0" }},
                "buffers": [ {{ "uri": "{uri}", "byteLength": 36 }} ] }}"#
        )
    };
    assert!(error(&using("../../secret.bin")).contains("`../../secret.bin`, which isn't inside"));
    assert!(error(&using("C:/secret.bin")).contains("isn't inside"));
    assert!(error(&using("%2e%2e/secret.bin")).contains("isn't inside"));
    assert!(error(&using("gone%20away.bin")).contains("`gone%20away.bin`, which isn't there"));
}

#[test]
fn formats_come_from_the_extension() {
    assert_eq!(
        ModelFormat::of(Path::new("a/chair.GLB")),
        Some(ModelFormat::Gltf)
    );
    assert_eq!(
        ModelFormat::of(Path::new("tree.gltf")),
        Some(ModelFormat::Gltf)
    );
    assert_eq!(
        ModelFormat::of(Path::new("Robot.Fbx")),
        Some(ModelFormat::Fbx)
    );
    assert_eq!(ModelFormat::of(Path::new("notes.txt")), None);
    assert_eq!(ModelFormat::of(Path::new("fbx")), None);
}

#[test]
fn uris_are_percent_decoded() {
    assert_eq!(
        percent_decode("my%20model.bin").as_deref(),
        Some("my model.bin")
    );
    assert_eq!(percent_decode("bad%2"), None);
    assert_eq!(percent_decode("bad%zz"), None);
}

#[test]
fn a_gltf_materials_maps_are_all_read() {
    let folder = std::env::temp_dir().join("axcenity-gltf-maps");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    // One triangle in the XY plane: positions, normals, then UVs.
    let floats: [f32; 24] = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, //
        0.0, 1.0, 1.0, 1.0, 0.0, 0.0,
    ];
    let bytes: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    std::fs::write(folder.join("tri.bin"), bytes).unwrap();
    for name in ["color", "orm", "normal", "glow"] {
        std::fs::write(folder.join(format!("{name}.png")), checker_png()).unwrap();
    }
    let gltf = r#"{
        "asset": { "version": "2.0" },
        "buffers": [ { "uri": "tri.bin", "byteLength": 96 } ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 72, "byteLength": 24 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0, 0, 0], "max": [1, 1, 0] },
            { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" },
            { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" }
        ],
        "images": [ { "uri": "color.png" }, { "uri": "orm.png" },
                    { "uri": "normal.png" }, { "uri": "glow.png" } ],
        "textures": [ { "source": 0 }, { "source": 1 }, { "source": 2 }, { "source": 3 } ],
        "materials": [ {
            "name": "Rusty",
            "pbrMetallicRoughness": {
                "baseColorTexture": { "index": 0 },
                "metallicRoughnessTexture": { "index": 1 }
            },
            "normalTexture": { "index": 2, "scale": 0.5 },
            "occlusionTexture": { "index": 1, "strength": 0.7 },
            "emissiveTexture": { "index": 3, "texCoord": 1 }
        } ],
        "meshes": [ { "primitives": [ {
            "attributes": { "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 },
            "material": 0
        } ] } ]
    }"#;
    let model = load_model(gltf.as_bytes(), &folder, ModelFormat::Gltf).unwrap();
    let primitive = &model.primitives[0];
    let material = &primitive.material;
    assert_eq!(material.name, "Rusty");
    assert!(material.base_color_texture.is_some());
    assert_eq!(material.normal_scale, 0.5);
    // One "ORM" image: occlusion in red, roughness in green, metalness in blue.
    let orm = |map: &Option<ChannelTexture>| {
        let map = map.as_ref().unwrap();
        (map.channel, Arc::as_ptr(&map.image))
    };
    let (metal, image) = orm(&material.metalness_texture);
    assert_eq!(metal, Channel::Blue);
    assert_eq!(orm(&material.roughness_texture), (Channel::Green, image));
    assert_eq!(orm(&material.occlusion_texture), (Channel::Red, image));
    assert_eq!(material.occlusion_strength, 0.7);
    // Placed by another set of texture coordinates, so left out.
    assert!(material.emissive_texture.is_none());
    // Tangents point along U (+X here), with the texture's "up" along +Y.
    assert_eq!(primitive.tangents.len(), 3);
    for tangent in &primitive.tangents {
        assert!((Vec3::new(tangent[0], tangent[1], tangent[2]) - Vec3::X).length() < 1e-5);
        assert_eq!(tangent[3], 1.0);
    }
}

/// A glTF triangle whose base color texture has `transform` (its
/// `KHR_texture_transform`, as JSON) and whose material is `double_sided`.
fn transformed_triangle(name: &str, transform: &str, double_sided: bool) -> ModelMaterial {
    let folder = std::env::temp_dir().join(format!("axcenity-gltf-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let floats: [f32; 24] = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, //
        0.0, 1.0, 1.0, 1.0, 0.0, 0.0,
    ];
    let bytes: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    std::fs::write(folder.join("tri.bin"), bytes).unwrap();
    std::fs::write(folder.join("leaf.png"), checker_png()).unwrap();
    let gltf = format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": ["KHR_texture_transform"],
        "buffers": [ {{ "uri": "tri.bin", "byteLength": 96 }} ],
        "bufferViews": [
            {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
            {{ "buffer": 0, "byteOffset": 36, "byteLength": 36 }},
            {{ "buffer": 0, "byteOffset": 72, "byteLength": 24 }}
        ],
        "accessors": [
            {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0, 0, 0], "max": [1, 1, 0] }},
            {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" }},
            {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" }}
        ],
        "images": [ {{ "uri": "leaf.png" }} ],
        "textures": [ {{ "source": 0 }} ],
        "materials": [ {{
            "doubleSided": {double_sided},
            "pbrMetallicRoughness": {{
                "baseColorTexture": {{ "index": 0,
                    "extensions": {{ "KHR_texture_transform": {transform} }} }}
            }}
        }} ],
        "meshes": [ {{ "primitives": [ {{
            "attributes": {{ "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 }},
            "material": 0
        }} ] }} ]
    }}"#
    );
    let model = load_model(gltf.as_bytes(), &folder, ModelFormat::Gltf).unwrap();
    model.primitives[0].material.clone()
}

#[test]
fn a_gltf_texture_transform_and_double_sided_material_are_read() {
    let material = transformed_triangle(
        "transform",
        r#"{ "offset": [0.5, 0.25], "rotation": 0.3, "scale": [2, 3] }"#,
        true,
    );
    assert!(material.base_color_texture.is_some());
    assert_eq!(material.uv_scale, [2.0, 3.0]);
    assert_eq!(material.uv_rotation, 0.3);
    assert_eq!(material.uv_offset, [0.5, 0.25]);
    assert!(material.double_sided);

    // Without the extension or `doubleSided`: as before.
    let plain = transformed_triangle("no-transform", "{}", false);
    assert_eq!(
        (plain.uv_scale, plain.uv_rotation, plain.uv_offset),
        ([1.0, 1.0], 0.0, [0.0, 0.0])
    );
    assert!(!plain.double_sided);
    // A transform onto another coordinate set isn't for the set drawn.
    let elsewhere =
        transformed_triangle("other-set", r#"{ "scale": [2, 2], "texCoord": 1 }"#, false);
    assert_eq!(elsewhere.uv_scale, [1.0, 1.0]);
}

#[test]
fn data_uris_are_decoded() {
    assert_eq!(
        data_uri("data:application/octet-stream;base64,SGk="),
        Some(b"Hi".to_vec())
    );
    assert_eq!(data_uri("buffer.bin"), None);
}

#[test]
fn embedded_images_are_named_for_copying_out() {
    // The example pyramid's texture is in its data: URI buffer.
    let files = embedded_files(&example_bytes(), ModelFormat::Gltf).unwrap();
    assert_eq!(files.len(), 1);
    let (name, png) = &files[0];
    assert!(image::guess_format(png).is_ok(), "{name}");
    let model = load_model(&example_bytes(), &example_folder(), ModelFormat::Gltf).unwrap();
    let texture = model.primitives[0].material.base_color_texture.as_ref();
    assert_eq!(texture.unwrap().file, Some(format!("textures/{name}")));
}

#[test]
fn missing_normals_are_worked_out_facing_outward() {
    // One triangle in the XY plane, counter-clockwise seen from +Z.
    let normals = smooth_normals(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[0, 1, 2],
    );
    assert_eq!(normals, vec![[0.0, 0.0, 1.0]; 3]);
}

#[test]
fn a_files_own_lod1_object_is_the_simpler_copy_and_doesnt_size_the_model() {
    let folder = std::env::temp_dir().join("axcenity-gltf-lods");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    // A square in the XY plane (two triangles), then one triangle.
    let floats: [f32; 27] = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    ];
    let bytes: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    std::fs::write(folder.join("rock.bin"), bytes).unwrap();
    // The copy is moved 5 along Z: if it counted, the model would be 5 deep.
    let gltf = r#"{
        "asset": { "version": "2.0" },
        "buffers": [ { "uri": "rock.bin", "byteLength": 108 } ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 72 },
            { "buffer": 0, "byteOffset": 72, "byteLength": 36 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 6, "type": "VEC3",
              "min": [0, 0, 0], "max": [1, 1, 0] },
            { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0, 0, 0], "max": [1, 1, 0] }
        ],
        "meshes": [
            { "primitives": [ { "attributes": { "POSITION": 0 } } ] },
            { "primitives": [ { "attributes": { "POSITION": 1 } } ] }
        ],
        "nodes": [
            { "name": "Rock", "mesh": 0 },
            { "name": "Rock_LOD1", "mesh": 1, "translation": [0, 0, 5] }
        ],
        "scenes": [ { "nodes": [0, 1] } ],
        "scene": 0
    }"#;
    let model = load_model(gltf.as_bytes(), &folder, ModelFormat::Gltf).unwrap();
    assert_eq!(
        model.primitives.len(),
        1,
        "the copy isn't drawn as its own part"
    );
    assert_eq!(model.size, [1.0, 1.0, 0.0]);
    assert_eq!(model.hull_points.len(), 4);
    let full = &model.primitives[0];
    assert_eq!(full.indices.len(), 6);
    assert_eq!(full.lods.len(), 1, "the file's copy, not simplified ones");
    assert_eq!(full.lods[0].indices.len(), 3);
    // Placed by the full model's fit, so 5 units back is 5 units back.
    assert!(
        full.lods[0]
            .positions
            .iter()
            .all(|p| (p[2] - 5.0).abs() < 1e-5)
    );

    // Picking the full object by name brings its copy with it.
    let picked = load_models(
        gltf.as_bytes(),
        &folder,
        ModelFormat::Gltf,
        &[Pick::Objects(vec!["Rock".to_owned()])],
    )
    .unwrap();
    assert_eq!(picked[0].primitives.len(), 1);
    assert_eq!(picked[0].primitives[0].lods.len(), 1);
    assert_eq!(picked[0].primitives[0].lods[0].indices.len(), 3);
    let _ = std::fs::remove_dir_all(&folder);
}

/// Writes `rock.bin` into `folder` and returns a glTF file using it: a
/// 1×1 square `Rock` with two collision pieces. `Rock_COL` is a triangle
/// moved 5 along Z and doubled in X and Y, holding a hidden `Inner` of its
/// own; `Rock_col2` is the square again. Neither is drawn.
pub(super) fn rock_with_pieces(folder: &Path) -> &'static str {
    std::fs::create_dir_all(folder).unwrap();
    let floats: [f32; 27] = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    ];
    let bytes: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    std::fs::write(folder.join("rock.bin"), bytes).unwrap();
    r#"{
        "asset": { "version": "2.0" },
        "buffers": [ { "uri": "rock.bin", "byteLength": 108 } ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 72 },
            { "buffer": 0, "byteOffset": 72, "byteLength": 36 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 6, "type": "VEC3",
              "min": [0, 0, 0], "max": [1, 1, 0] },
            { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0, 0, 0], "max": [1, 1, 0] }
        ],
        "meshes": [
            { "primitives": [ { "attributes": { "POSITION": 0 } } ] },
            { "primitives": [ { "attributes": { "POSITION": 1 } } ] }
        ],
        "nodes": [
            { "name": "Rock", "mesh": 0 },
            { "name": "Rock_COL", "mesh": 1, "translation": [0, 0, 5], "scale": [2, 2, 1],
              "children": [3] },
            { "name": "Rock_col2", "mesh": 0 },
            { "name": "Inner", "mesh": 1 }
        ],
        "scenes": [ { "nodes": [0, 1, 2] } ],
        "scene": 0
    }"#
}

#[test]
fn a_files_col_objects_are_its_collision_pieces_not_drawn_and_not_sizing_it() {
    let folder = std::env::temp_dir().join(format!("axcenity-gltf-col-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let gltf = rock_with_pieces(&folder);
    let model = load_model(gltf.as_bytes(), &folder, ModelFormat::Gltf).unwrap();
    assert_eq!(model.primitives.len(), 1, "pieces aren't drawn");
    assert_eq!(model.size, [1.0, 1.0, 0.0], "pieces don't size the model");
    assert_eq!(model.hull_points.len(), 4);
    assert_eq!(model.collision.len(), 2);
    // Fitted like the triangles: the square's middle is the origin.
    let mut first = model.collision[0].clone();
    first.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(
        first,
        vec![
            // `Inner`, inside `Rock_COL`, is part of the same piece.
            [-0.5, -0.5, 5.0],
            [-0.5, -0.5, 5.0],
            [-0.5, 1.5, 5.0],
            [-0.5, 1.5, 5.0],
            [1.5, -0.5, 5.0],
            [1.5, -0.5, 5.0],
        ]
    );
    assert_eq!(model.collision[1].len(), 6);
    assert!(model.collision[1].iter().all(|p| p[2] == 0.0));

    // Picking the object by name brings its pieces with it.
    let picked = load_models(
        gltf.as_bytes(),
        &folder,
        ModelFormat::Gltf,
        &[Pick::Objects(vec!["Rock".to_owned()])],
    )
    .unwrap();
    assert_eq!(picked[0].primitives.len(), 1);
    assert_eq!(picked[0].collision.len(), 2);
    // Objects inside a picked one come only if picked too, as everywhere.
    assert_eq!(picked[0].collision[0].len(), 3);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_gltf_materials_extras_say_how_it_moves_in_the_wind() {
    let wind = |text: &str| {
        let raw = serde_json::value::RawValue::from_string(text.to_owned()).unwrap();
        let material = super::wind_of(&Some(raw));
        (material.sway, material.flutter)
    };
    assert_eq!(
        wind(r#"{"axcenity_sway": 0.4, "axcenity_flutter": 0.2, "other": "kept out"}"#),
        (0.4, 0.2)
    );
    // Past the limits, below 0, or the wrong sort of value.
    assert_eq!(
        wind(r#"{"axcenity_sway": 9, "axcenity_flutter": 3}"#),
        (2.0, super::MAX_FLUTTER)
    );
    assert_eq!(wind(r#"{"axcenity_sway": -1}"#), (0.0, 0.0));
    assert_eq!(wind(r#"{"axcenity_sway": "lots"}"#), (0.0, 0.0));
    let none = super::wind_of(&None);
    assert_eq!((none.sway, none.flutter), (0.0, 0.0));
}

#[test]
fn the_same_embedded_texture_in_two_models_is_one_image() {
    let bytes = checker_png();
    let project = Path::new("project");
    let shared = embedded_key(Some(project), &bytes);
    let image = Arc::new(Image::new(2, 2, vec![255; 16]));
    share_embedded(shared, &image);
    let found = shared_embedded(shared).expect("still in use");
    assert!(Arc::ptr_eq(&found, &image));
    assert!(shared_embedded(embedded_key(Some(project), b"other bytes")).is_none());
    // Not with another project: a cooked image reads its own project's files.
    let elsewhere = embedded_key(Some(Path::new("other project")), &bytes);
    assert!(shared_embedded(elsewhere).is_none());
    // It's let go with the last model that used it.
    drop((image, found));
    assert!(shared_embedded(shared).is_none());
}

/// Writes `face.bin` into `folder` and returns a glTF file using it: two
/// meshes, `Skin` (a 2×2 square in the XY plane, facing +Z) and `Lash` (a
/// triangle above it), with blend shapes named as Blender writes them
/// (`extras.targetNames`). `Blink_Left` lifts the square's corner 2 by 1
/// along Z and moves the lash down 1 along Y; `Smile` moves the square's
/// corner 1 right by 1 and turns its normal, and rests at 0.25.
pub(super) fn blinking_face(folder: &Path) -> &'static str {
    let _ = std::fs::remove_dir_all(folder);
    std::fs::create_dir_all(folder).unwrap();
    #[rustfmt::skip]
    let floats: Vec<f32> = vec![
        // 0: square positions (4).
        -1.0, -1.0, 0.0,  1.0, -1.0, 0.0,  1.0, 1.0, 0.0,  -1.0, 1.0, 0.0,
        // 48: square normals.
        0.0, 0.0, 1.0,  0.0, 0.0, 1.0,  0.0, 0.0, 1.0,  0.0, 0.0, 1.0,
        // 96: square Blink_Left moves.
        0.0, 0.0, 0.0,  0.0, 0.0, 0.0,  0.0, 0.0, 1.0,  0.0, 0.0, 0.0,
        // 144: square Smile moves.
        0.0, 0.0, 0.0,  1.0, 0.0, 0.0,  0.0, 0.0, 0.0,  0.0, 0.0, 0.0,
        // 192: square Smile normal changes.
        0.0, 0.0, 0.0,  0.6, 0.0, -0.2,  0.0, 0.0, 0.0,  0.0, 0.0, 0.0,
        // 240: lash positions (3).
        -1.0, 1.5, 0.0,  1.0, 1.5, 0.0,  0.0, 2.0, 0.0,
        // 276: lash Blink_Left moves.
        0.0, -1.0, 0.0,  0.0, -1.0, 0.0,  0.0, -1.0, 0.0,
    ];
    let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
    let mut bytes: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    bytes.extend(indices.iter().flat_map(|i| i.to_le_bytes()));
    std::fs::write(folder.join("face.bin"), &bytes).unwrap();
    r#"{
        "asset": { "version": "2.0" },
        "buffers": [ { "uri": "face.bin", "byteLength": 324 } ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 48 },
            { "buffer": 0, "byteOffset": 48, "byteLength": 48 },
            { "buffer": 0, "byteOffset": 96, "byteLength": 48 },
            { "buffer": 0, "byteOffset": 144, "byteLength": 48 },
            { "buffer": 0, "byteOffset": 192, "byteLength": 48 },
            { "buffer": 0, "byteOffset": 240, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 276, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 312, "byteLength": 12 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3",
              "min": [-1, -1, 0], "max": [1, 1, 0] },
            { "bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC3" },
            { "bufferView": 2, "componentType": 5126, "count": 4, "type": "VEC3",
              "min": [0, 0, 0], "max": [0, 0, 1] },
            { "bufferView": 3, "componentType": 5126, "count": 4, "type": "VEC3",
              "min": [0, 0, 0], "max": [1, 0, 0] },
            { "bufferView": 4, "componentType": 5126, "count": 4, "type": "VEC3" },
            { "bufferView": 5, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [-1, 1.5, 0], "max": [1, 2, 0] },
            { "bufferView": 6, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0, -1, 0], "max": [0, -1, 0] },
            { "bufferView": 7, "componentType": 5123, "count": 6, "type": "SCALAR" }
        ],
        "meshes": [
            { "name": "Skin",
              "primitives": [ {
                "attributes": { "POSITION": 0, "NORMAL": 1 },
                "indices": 7,
                "targets": [ { "POSITION": 2 }, { "POSITION": 3, "NORMAL": 4 } ]
              } ],
              "weights": [0, 0.25],
              "extras": { "targetNames": ["Blink_Left", "Smile"] } },
            { "name": "Lash",
              "primitives": [ {
                "attributes": { "POSITION": 5 },
                "targets": [ { "POSITION": 6 } ]
              } ],
              "extras": { "targetNames": ["Blink_Left"] } }
        ],
        "nodes": [ { "name": "Skin", "mesh": 0 }, { "name": "Lash", "mesh": 1 } ],
        "scenes": [ { "nodes": [0, 1] } ],
        "scene": 0
    }"#
}

#[test]
fn a_gltf_files_blend_shapes_are_read_and_shared_by_name() {
    let folder = std::env::temp_dir().join(format!("axcenity-gltf-face-{}", std::process::id()));
    let gltf = blinking_face(&folder);
    let model = load_model(gltf.as_bytes(), &folder, ModelFormat::Gltf).unwrap();
    let names: Vec<&str> = model
        .morph_targets
        .iter()
        .map(|target| target.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["Blink_Left", "Smile"],
        "one Blink_Left for both meshes"
    );
    assert_eq!(model.morph_targets[1].default_weight, 0.25);
    // The model is 2 wide and 3 tall (the lash reaches y = 2), flat in Z.
    assert_eq!(model.size, [2.0, 3.0, 0.0]);
    let skin = &model.primitives[0];
    let lash = &model.primitives[1];
    assert_eq!(skin.morphs.len(), 2);
    let blink = &skin.morphs[0];
    assert_eq!(blink.target, 0);
    assert_eq!(blink.corners, vec![2]);
    // Z is flat, so it isn't scaled: the lift stays 1.
    assert_eq!(blink.positions, vec![[0.0, 0.0, 1.0]]);
    let smile = &skin.morphs[1];
    assert_eq!(smile.corners, vec![1]);
    // X is fitted from 2 wide to 1, so the move halves.
    assert_eq!(smile.positions, vec![[0.5, 0.0, 0.0]]);
    assert!(
        Vec3::from(smile.normals[0]).length() > 0.1,
        "the file's normal change is kept"
    );
    assert_eq!(lash.morphs.len(), 1);
    assert_eq!(lash.morphs[0].target, 0);
    assert_eq!(lash.morphs[0].corners, vec![0, 1, 2]);
    // Y is fitted from 3 tall to 1.
    let down = lash.morphs[0].positions[0][1];
    assert!((down + 1.0 / 3.0).abs() < 1e-6, "{down}");

    // Leaning toward Blink_Left moves both meshes, before skinning.
    let mut posed = model.rest_posed();
    assert_eq!(posed.morph_weights(), [0.0, 0.25]);
    posed.set_morph_weights(&[1.0, 0.0]);
    let lifted = posed.primitive(&model, 0, 0);
    assert_eq!(lifted.positions[2][2], skin.positions[2][2] + 1.0);
    assert_eq!(lifted.positions[1], skin.positions[1], "Smile is off");
    let lowered = posed.primitive(&model, 1, 0);
    assert!((lowered.positions[0][1] - (lash.positions[0][1] + down)).abs() < 1e-6);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_blend_shape_accessor_without_data_reads_as_no_moves() {
    // Blender writes a shape that doesn't move a primitive as an accessor
    // with no buffer view and no sparse values: all zeros, by the spec.
    let folder =
        std::env::temp_dir().join(format!("axcenity-gltf-empty-shape-{}", std::process::id()));
    let gltf = blinking_face(&folder).replace(
        r#"{ "bufferView": 6, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0, -1, 0], "max": [0, -1, 0] }"#,
        r#"{ "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0, 0, 0], "max": [0, 0, 0] }"#,
    );
    assert!(
        !gltf.contains(r#""bufferView": 6"#),
        "the lash's moves are gone"
    );
    let model = load_model(gltf.as_bytes(), &folder, ModelFormat::Gltf).unwrap();
    let names: Vec<&str> = model
        .morph_targets
        .iter()
        .map(|target| target.name.as_str())
        .collect();
    assert_eq!(names, ["Blink_Left", "Smile"]);
    assert_eq!(model.primitives.len(), 2);
    let lash = model
        .primitives
        .iter()
        .find(|primitive| primitive.positions.len() == 3)
        .unwrap();
    assert!(lash.morphs.is_empty(), "the lash doesn't move");
    // Every other check still holds.
    let broken = gltf.replace(r#""bufferView": 7,"#, r#""bufferView": 99,"#);
    assert_ne!(broken, gltf);
    assert!(load_model(broken.as_bytes(), &folder, ModelFormat::Gltf).is_err());
    let _ = std::fs::remove_dir_all(&folder);
}
