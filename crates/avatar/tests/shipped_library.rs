//! The repository's character library (`assets/characters/`): its
//! catalogue is current, every manifest is sound, and every shape, bone and
//! material a manifest names is in the model it describes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use axcenity_assets::{ModelFormat, Pick, load_models};
use axcenity_avatar::{
    BaseManifest, Catalogue, Description, Library, ModelContents, ModelRef, Problem, WornItem,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn read() -> Library {
    let (library, catalogue, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{}", lines(&problems));
    assert!(catalogue.is_some());
    library
}

fn lines(problems: &[Problem]) -> String {
    problems
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every manifest file under `folder`, relative to the library's root.
fn manifest_files(folder: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            manifest_files(&path, out);
        } else {
            let name = path.file_name().unwrap().to_string_lossy();
            if name.ends_with(".base.json") || name.ends_with(".item.json") {
                let relative = path.strip_prefix(root()).unwrap();
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

/// A model in the library: its file's library path and the object picked.
type ModelKey = (String, Option<String>);

/// The models every manifest names, each base model's body first, then its
/// parts.
fn named_models(library: &Library) -> Vec<(String, ModelRef)> {
    let mut models = Vec::new();
    for base in library.bases.values() {
        for object in std::iter::once(&base.model.body).chain(&base.model.parts) {
            let model = ModelRef {
                file: base.model.file.clone(),
                object: Some(object.clone()),
            };
            models.push((base.id.clone(), model));
        }
    }
    for item in library.items.values() {
        for model in item.models() {
            models.push((item.id.clone(), model.clone()));
        }
    }
    models
}

fn key(library: &Library, owner: &str, model: &ModelRef) -> ModelKey {
    let file = library
        .path_of(owner, &model.file)
        .unwrap_or_else(|| panic!("{owner}: {} leaves the library", model.file));
    (file, model.object.clone())
}

/// The contents of every model the library's manifests name, read with the
/// engine's own loader, each file once.
fn read_models(library: &Library) -> BTreeMap<ModelKey, ModelContents> {
    load_named_models(library)
        .into_iter()
        .map(|(key, model)| (key, ModelContents::of(&model)))
        .collect()
}

/// Every model the library's manifests name, loaded by the engine, each
/// file once.
fn load_named_models(library: &Library) -> BTreeMap<ModelKey, axcenity_assets::Model> {
    let mut by_file: BTreeMap<String, Vec<Option<String>>> = BTreeMap::new();
    for (owner, model) in named_models(library) {
        let (file, object) = key(library, &owner, &model);
        let picks = by_file.entry(file).or_default();
        if !picks.contains(&object) {
            picks.push(object);
        }
    }
    let mut out = BTreeMap::new();
    for (file, objects) in by_file {
        let bytes = std::fs::read(root().join(&file)).unwrap_or_else(|err| panic!("{file}: {err}"));
        let picks: Vec<Pick> = objects
            .iter()
            .map(|object| match object {
                Some(object) => Pick::Objects(vec![object.clone()]),
                None => Pick::Whole,
            })
            .collect();
        let loaded = load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
            .unwrap_or_else(|err| panic!("{file}: {err}"));
        for (object, model) in objects.into_iter().zip(loaded) {
            assert!(
                !model.primitives.is_empty(),
                "{object:?} picks nothing in {file}"
            );
            out.insert((file.clone(), object), model);
        }
    }
    out
}

fn body_contents(
    library: &Library,
    models: &BTreeMap<ModelKey, ModelContents>,
    base: &BaseManifest,
) -> (ModelContents, Vec<ModelContents>) {
    let get = |object: &String| {
        let model = ModelRef {
            file: base.model.file.clone(),
            object: Some(object.clone()),
        };
        models[&key(library, &base.id, &model)].clone()
    };
    (
        get(&base.model.body),
        base.model.parts.iter().map(get).collect(),
    )
}

#[test]
fn the_shipped_library_reads_and_is_sound() {
    let library = read();
    assert!(library.base("axcenity/stylized").is_some());
    assert_eq!(lines(&library.validate()), "");
    // The catalogue is current: what `catalogue.py` writes is what the
    // manifests say, and it lists every manifest in the folder.
    let (_, catalogue, _) = Library::read(&root());
    assert_eq!(
        catalogue.unwrap(),
        Catalogue::of(&library),
        "run `python -I assets/characters/catalogue.py`"
    );
    let mut files = Vec::new();
    manifest_files(&root(), &mut files);
    files.sort();
    let mut listed: Vec<String> = library
        .bases
        .keys()
        .chain(library.items.keys())
        .map(|id| library.manifest_path(id).unwrap().to_owned())
        .collect();
    listed.sort();
    assert_eq!(files, listed, "every manifest is in the catalogue");
}

#[test]
fn every_manifest_entry_names_shapes_bones_and_materials_its_model_has() {
    let library = read();
    let models = read_models(&library);
    for base in library.bases.values() {
        let (body, parts) = body_contents(&library, &models, base);
        assert!(
            body.shapes.len() > 10 && body.bones.len() > 50,
            "{}",
            base.id
        );
        assert_eq!(lines(&base.check_model(&body, &parts)), "", "{}", base.id);
    }
    for item in library.items.values() {
        assert!(!item.models().is_empty(), "{}", item.id);
        let mut together = ModelContents::default();
        for model in item.models() {
            let one = &models[&key(&library, &item.id, model)];
            together.shapes.extend(one.shapes.iter().cloned());
            together.bones.extend(one.bones.iter().cloned());
            together.materials.extend(one.materials.iter().cloned());
        }
        for base_id in &item.bases {
            let base = library.base(base_id).unwrap();
            let (body, _) = body_contents(&library, &models, base);
            assert_eq!(
                lines(&item.check_model(&together, &body)),
                "",
                "{} on {base_id}",
                item.id
            );
        }
    }
}

/// An item's pose correctives (wave 5, ADR-0044) are in its manifest and,
/// with the same rules, in its models' files, which the runtime reads: the
/// two agree, and no model drives a shape its manifest doesn't list.
#[test]
fn every_items_pose_correctives_are_its_models() {
    let library = read();
    let models = load_named_models(&library);
    for item in library.items.values() {
        let own = item
            .models()
            .into_iter()
            .map(|model| &models[&key(&library, &item.id, model)]);
        if let Err(why) = axcenity_avatar::fit::rules_match(own, &item.correctives) {
            panic!("{}: {why}", item.id);
        }
    }
}

#[test]
fn every_base_models_default_character_is_sound() {
    let library = read();
    for base in library.bases.values() {
        let mut description = Description::new("New Character", &base.id);
        description.items = base.defaults.items.iter().map(WornItem::new).collect();
        assert_eq!(lines(&description.validate(&library)), "", "{}", base.id);
    }
}

#[test]
fn every_format_1_character_migrates_to_a_sound_description() {
    let library = read();
    let looks = [
        r#"{"format_version": 1}"#,
        r#"{"format_version": 1, "name": "Casual", "build": -0.4, "hair": "Hair_Bob", "outerwear": null}"#,
        r#"{"format_version": 1, "name": "Athlete", "muscle": 0.9, "hair": "Hair_Buzz", "outerwear": null}"#,
        r#"{"format_version": 1, "name": "Long hair", "build": 0.5, "hair": "Hair_Long"}"#,
        r#"{"format_version": 1, "name": "Base body", "hair": "Hair_Crop", "top": null, "outerwear": null, "bottom": "Underwear_Briefs", "shoes": null}"#,
        r#"{"format_version": 1, "hair": "Hair_Ponytail", "top": "Underwear_Top"}"#,
    ];
    for text in looks {
        let description = Description::from_json(text).unwrap();
        assert_eq!(lines(&description.validate(&library)), "", "{text}");
    }
    // Every format 1 part became an item the library has.
    for (part, id) in axcenity_avatar::V1_ITEMS {
        assert!(library.item(id).is_some(), "{part} → {id}");
    }
}

/// The benchmark characters (`assets/characters/benchmarks/`, written by
/// its `make.py`; overhaul CO2): each is a sound description of a base
/// model in the library, named after its file, and resolves with nothing
/// left out. Every base model has its set: bodies, faces, outfits and hair
/// where it has them.
#[test]
fn every_benchmark_character_is_sound() {
    let library = read();
    let folder = root().join("benchmarks");
    let mut per_base: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in std::fs::read_dir(&folder).unwrap() {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        let Some(name) = file.strip_suffix(".character.json") else {
            continue;
        };
        let text = std::fs::read_to_string(&path).unwrap();
        let description =
            Description::from_json(&text).unwrap_or_else(|err| panic!("{file}: {err}"));
        assert_eq!(description.name, name, "{file} is named after its file");
        let problems = description.validate(&library);
        assert!(problems.is_empty(), "{file}:\n{}", lines(&problems));
        let resolved = axcenity_avatar::resolve(&library, &description)
            .unwrap_or_else(|problem| panic!("{file}: {problem}"));
        assert!(
            resolved.problems.is_empty(),
            "{file}:\n{}",
            lines(&resolved.problems)
        );
        // The file says what is drawn: what a parameter's choices wear (the
        // realistic body's underwear) is already what it lists, so the
        // resolver changes nothing.
        let mut worn = description.clone();
        axcenity_avatar::wear_choices(&mut worn, &library);
        let ids = |d: &Description| d.items.iter().map(|w| w.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&worn), ids(&description), "{file} wears its choices");
        // A cast member other than the average sets something.
        if name.contains("-cast-") && !name.ends_with("-cast-average") {
            assert!(!description.parameters.is_empty(), "{file} sets nothing");
        }
        per_base
            .entry(description.base.clone())
            .or_default()
            .push(name.to_owned());
    }
    for (id, base) in &library.bases {
        let names = per_base.get(id).map(Vec::as_slice).unwrap_or_default();
        let has = |part: &str| names.iter().any(|name| name.contains(part));
        assert!(has("-body-default"), "{id} has no body benchmarks");
        // The neutral face is the cast's average where the two were the
        // same character (refinement R1, plan 11.6).
        assert!(
            has("-face-neutral") || has("-cast-average"),
            "{id} has no face benchmarks"
        );
        assert!(has("-outfit-underwear"), "{id} has no outfit benchmarks");
        // The hips' two ends (R3: they narrow as well as widen).
        if base.parameters.iter().any(|p| p.id == "hip_width") {
            assert!(
                has("-body-narrowest-hips") && has("-body-widest-hips"),
                "{id} has no hip benchmarks"
            );
        }
        assert!(
            names.iter().all(|name| name.starts_with(&base.family)),
            "{id}: benchmarks are named <family>-<frame>-<case>"
        );
        // The cast (brief §17.1): each person on each frame, and natural
        // asymmetry where the base has sided parameters to show it with.
        let frames: &[&str] = if base.parameters.iter().any(|p| p.id == "frame") {
            &["masculine", "feminine"]
        } else {
            &["body"]
        };
        let sided = base.parameters.iter().any(|p| p.mirror.is_some());
        for frame in frames {
            for person in BENCHMARK_CAST {
                let name = format!("{}-{frame}-cast-{person}", base.family);
                assert!(names.contains(&name), "{id} has no {name}");
            }
            let asymmetry = format!("{}-{frame}-cast-asymmetry", base.family);
            assert_eq!(names.contains(&asymmetry), sided, "{asymmetry}");
        }
    }
}

/// Every preset file (`*.preset.json`) under the library, relative to
/// its root.
fn preset_files(folder: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            preset_files(&path, out);
        } else if path.to_string_lossy().ends_with(".preset.json") {
            let relative = path.strip_prefix(root()).unwrap();
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// The shipped presets (plan section 11.3; brief §4.1, §5.1): each is in
/// the catalogue, sound for its kind, resolves with nothing left out, and
/// applied to its base model's default character gives a sound character
/// that matches it.
#[test]
fn every_shipped_preset_is_sound_listed_and_resolves() {
    use axcenity_avatar::{PresetKind, apply_preset, check_preset, matches_preset};
    let library = read();
    let mut files = Vec::new();
    preset_files(&root(), &mut files);
    files.sort();
    let mut listed: Vec<String> = library
        .presets
        .keys()
        .map(|id| library.preset_path(id).unwrap().to_owned())
        .collect();
    listed.sort();
    assert_eq!(files, listed, "every preset is in the catalogue");
    let mut kinds: BTreeMap<(String, PresetKind), usize> = BTreeMap::new();
    for (id, preset) in &library.presets {
        let problems = check_preset(preset, &library);
        assert!(problems.is_empty(), "{id}:\n{}", lines(&problems));
        let resolved = axcenity_avatar::resolve(&library, preset).unwrap();
        assert!(
            resolved.problems.is_empty(),
            "{id}: {:?}",
            resolved.problems
        );
        let base = library.base(&preset.base).unwrap();
        let mut character = Description::new("New Character", &base.id);
        character.items = base.defaults.items.iter().map(WornItem::new).collect();
        apply_preset(&mut character, preset, &library).unwrap();
        assert_eq!(lines(&character.validate(&library)), "", "{id}");
        assert!(matches_preset(&character, preset, &library), "{id}");
        assert!(character.preset().is_none(), "{id}: the block stays behind");
        let kind = preset.preset().unwrap().kind;
        *kinds.entry((preset.base.clone(), kind)).or_default() += 1;
    }
    let count = |kind| kinds.get(&("axcenity/realistic".to_owned(), kind)).copied();
    // Seven builds for each frame (Custom is "none matches"), nine faces,
    // the six characters the editor had in code.
    assert_eq!(count(PresetKind::Body), Some(14));
    assert_eq!(count(PresetKind::Face), Some(9));
    assert_eq!(count(PresetKind::Character), Some(6));
}

/// The realistic base model's parameters say what they are (R1): a
/// sentence, a level in step with `advanced`, and a feature it has.
#[test]
fn the_realistic_parameters_say_what_they_are() {
    use axcenity_avatar::Level;
    let library = read();
    let base = library.base("axcenity/realistic").unwrap();
    // (75 at R1; R4 added the face's 14.)
    assert_eq!(base.parameters.len(), 89);
    for parameter in &base.parameters {
        let id = &parameter.id;
        let about = parameter.about.as_deref().unwrap_or_default();
        assert!(about.len() > 10 && about.ends_with('.'), "{id}: `{about}`");
        let level = parameter
            .level
            .unwrap_or_else(|| panic!("{id} has no level"));
        assert_eq!(level == Level::Advanced, parameter.advanced, "{id}");
        let feature = parameter
            .feature
            .as_deref()
            .unwrap_or_else(|| panic!("{id}"));
        assert!(base.feature(feature).is_some(), "{id}: {feature}");
    }
    for id in ["frame", "muscle", "fat", "height"] {
        assert_eq!(base.parameter(id).unwrap().level(), Level::Quick, "{id}");
    }
    let features: Vec<&str> = base.features.iter().map(|f| f.id.as_str()).collect();
    for id in [
        "head",
        "neck",
        "shoulders",
        "chest",
        "arms",
        "hands",
        "abdomen",
        "waist",
        "back",
        "hips",
        "thighs",
        "calves",
        "feet",
        "eyes",
        "brows",
        "nose",
        "cheeks",
        "jaw",
        "chin",
        "lips",
        "forehead",
        "ears",
    ] {
        assert!(features.contains(&id), "no feature `{id}`: {features:?}");
    }
    for feature in &base.features {
        assert!(
            !feature.anchors.is_empty() && feature.radius > 0.0,
            "{}",
            feature.id
        );
    }
    // R2 wrote the handles (`handles.py`); `realistic_handles.rs` checks them.
    assert!(!base.handles.is_empty(), "the realistic body's handles");
}

/// Randomize on the shipped base models (R1; R10's 1,000 seeds): every
/// character validates and resolves, and a locked parameter holds.
#[test]
fn a_thousand_random_characters_on_each_shipped_base_are_sound() {
    use axcenity_avatar::{Scope, randomize};
    let library = read();
    let locks: std::collections::BTreeSet<String> = ["nose_length".to_owned()].into();
    for base in library.bases.values() {
        let mut start = Description::new("Npc", &base.id);
        start.items = base.defaults.items.iter().map(WornItem::new).collect();
        start.parameters.insert("nose_length".to_owned(), 0.25);
        let locks = if base.parameter("nose_length").is_some() {
            locks.clone()
        } else {
            start.parameters.clear();
            Default::default()
        };
        for seed in 0..1000 {
            let character = randomize(&start, &Scope::All, &locks, seed, &library);
            let problems = character.validate(&library);
            assert!(
                problems.is_empty(),
                "{} seed {seed}:\n{}",
                base.id,
                lines(&problems)
            );
            let resolved = axcenity_avatar::resolve(&library, &character).unwrap();
            assert!(resolved.problems.is_empty(), "{} seed {seed}", base.id);
            if !locks.is_empty() {
                assert_eq!(character.parameters["nose_length"], 0.25);
            }
        }
    }
}

/// An outfit preset never takes off what a parameter's choices wear (the
/// realistic body's underwear), even one that wears nothing.
#[test]
fn an_outfit_preset_leaves_the_chosen_underwear() {
    let library = read();
    let empty = Description::from_json(
        r#"{ "format_version": 2, "name": "Nothing", "base": "axcenity/realistic",
             "preset": { "id": "test/nothing", "kind": "outfit", "about": "Nothing on." } }"#,
    )
    .unwrap();
    let mut character = Description::new("Ada", "axcenity/realistic");
    character.parameters.insert("frame".to_owned(), 1.0);
    axcenity_avatar::wear_choices(&mut character, &library);
    let before = character.items.clone();
    assert_eq!(before.len(), 2, "briefs and a sports bra");
    axcenity_avatar::apply_preset(&mut character, &empty, &library).unwrap();
    assert_eq!(character.items, before);
}

/// The benchmark cast every base has on each frame (brief §17.1;
/// `benchmarks/make.py`'s `CAST`). `asymmetry` joins it on a base with
/// sided parameters.
const BENCHMARK_CAST: [&str; 8] = [
    "average",
    "slim",
    "athletic",
    "muscular",
    "heavyset",
    "broad-face",
    "narrow-face",
    "distinctive",
];

/// R10's Done-when (plan 11.5), on both shipped families: 1,000 seeded
/// randomizations of each, spread over every scope Character Studio
/// offers (everything, each category, each feature, a list of
/// parameters), starting from the family's default character and each of
/// its presets, each with a different set of locks. Every character
/// validates and resolves with no problem, and nothing locked changes:
/// a locked parameter keeps its value and a locked slot keeps what it
/// wears.
#[test]
fn every_scope_on_each_shipped_family_draws_sound_characters_and_keeps_its_locks() {
    use axcenity_avatar::{Category, Scope, parameter_values, randomize};
    use std::collections::BTreeSet;
    let library = read();
    let families = [axcenity_avatar::V1_BASE, "axcenity/realistic"];
    for family in families {
        let base = library.base(family).expect("both families ship");
        let mut scopes = vec![Scope::All];
        scopes.extend(
            [
                Category::Body,
                Category::Face,
                Category::Skin,
                Category::Hair,
                Category::Clothing,
                Category::Accessories,
            ]
            .map(Scope::Category),
        );
        scopes.extend(base.features.iter().map(|f| Scope::Feature(f.id.clone())));
        let ids: Vec<String> = base.parameters.iter().map(|p| p.id.clone()).collect();
        scopes.push(Scope::Parameters(ids.iter().step_by(5).cloned().collect()));
        // The default character and every preset on this family.
        let mut starts = vec![{
            let mut start = Description::new("Npc", family);
            start.items = base.defaults.items.iter().map(WornItem::new).collect();
            start
        }];
        for preset in library.presets.values().filter(|p| p.base == family) {
            let mut start = starts[0].clone();
            if axcenity_avatar::apply_preset(&mut start, preset, &library).is_ok() {
                starts.push(start);
            }
        }
        assert!(starts.len() > 1, "{family} has presets");
        // A slot a parameter's choices fill (the realistic underwear) always
        // wears the choice's item, so it can't be locked on its own: the
        // studio's slot locks leave those out too.
        let chosen = axcenity_avatar::chosen_slots(&library, family);
        let slots: Vec<String> = base
            .slots
            .iter()
            .map(|s| s.id.clone())
            .filter(|id| !chosen.contains(id))
            .collect();
        let mut changed_any = vec![false; scopes.len()];
        for seed in 0..1000u64 {
            let at = seed as usize;
            let scope = &scopes[at % scopes.len()];
            let start = &starts[(at / scopes.len()) % starts.len()];
            // A different set of locks each time: about one parameter in
            // seven, and one slot.
            let mut locks: BTreeSet<String> = ids.iter().skip(at % 7).step_by(7).cloned().collect();
            locks.insert(slots[at % slots.len()].clone());
            let drawn = randomize(start, scope, &locks, seed, &library);
            let problems = drawn.validate(&library);
            assert!(
                problems.is_empty(),
                "{family} {scope:?} seed {seed}:\n{}",
                lines(&problems)
            );
            let resolved = axcenity_avatar::resolve(&library, &drawn).unwrap();
            assert!(
                resolved.problems.is_empty(),
                "{family} {scope:?} seed {seed}: {}",
                lines(&resolved.problems)
            );
            let (before, after) = (
                parameter_values(base, start),
                parameter_values(base, &drawn),
            );
            for id in locks.iter().filter(|id| base.parameter(id).is_some()) {
                assert_eq!(
                    before.get(id),
                    after.get(id),
                    "{family} {scope:?} seed {seed}: locked `{id}` changed"
                );
            }
            let slot = &slots[at % slots.len()];
            let worn = |d: &Description| -> Vec<WornItem> {
                d.items
                    .iter()
                    .filter(|w| {
                        library
                            .item(&w.id)
                            .is_some_and(|i| i.slots().any(|s| s == slot))
                    })
                    .cloned()
                    .collect()
            };
            assert_eq!(
                worn(start),
                worn(&drawn),
                "{family} {scope:?} seed {seed}: locked slot `{slot}` changed"
            );
            if &drawn != start {
                changed_any[at % scopes.len()] = true;
            }
        }
        // Everything, and the body and face, do draw something new.
        for (index, scope) in scopes.iter().enumerate() {
            let named = matches!(
                scope,
                Scope::All | Scope::Category(Category::Body) | Scope::Category(Category::Face)
            );
            assert!(
                !named || changed_any[index],
                "{family}: {scope:?} never changed anything"
            );
        }
    }
}

/// Wave 5, hiding under clothing: every mask an item names is there,
/// beside it, an image of the size its model is masked at: the body's
/// texture square (`fit::BODY_MASK`), or a mesh without texture
/// coordinates by number (`fit::NUMBERED_WIDTH` a row, a texel per
/// triangle), and it hides something.
#[test]
fn every_hide_map_is_there_and_fits_its_model() {
    use axcenity_avatar::fit::{BODY_MASK, BodyMesh, LAYER_MASK, NUMBERED_WIDTH};
    let library = read();
    let assets = axcenity_assets::Assets::new(root());
    let mut checked = 0;
    for (id, item) in &library.items {
        for hide in &item.hide_maps {
            let path = library.path_of(id, &hide.map).unwrap();
            let image = axcenity_assets::read_texture(&root(), &path)
                .unwrap_or_else(|err| panic!("{id}: {path}: {err}"));
            let mesh_id = match &hide.under {
                None => {
                    let base = library.base(&item.bases[0]).unwrap();
                    let file = library.path_of(&base.id, &base.model.file).unwrap();
                    format!("{file}#{}", base.model.body)
                }
                Some(under) => {
                    let lower = library
                        .item(under)
                        .unwrap_or_else(|| panic!("{id}: {under}"));
                    let model = lower.model.as_ref().unwrap();
                    let file = library.path_of(under, &model.file).unwrap();
                    format!("{file}#{}", model.object.clone().unwrap_or_default())
                }
            };
            let mesh = BodyMesh::from_model(&assets.model_blocking(&mesh_id).unwrap()).unwrap();
            let (width, height) = if mesh.uvs.is_empty() {
                let rows = (mesh.triangles.len() as u32).div_ceil(NUMBERED_WIDTH);
                (NUMBERED_WIDTH, rows)
            } else if hide.under.is_none() {
                (BODY_MASK, BODY_MASK)
            } else {
                (LAYER_MASK, LAYER_MASK)
            };
            assert_eq!((image.width, image.height), (width, height), "{id}: {path}");
            assert!(
                image.rgba().chunks_exact(4).any(|px| px[0] >= 128),
                "{id}: {path} hides nothing"
            );
            checked += 1;
        }
    }
    println!("{checked} masks");
}
