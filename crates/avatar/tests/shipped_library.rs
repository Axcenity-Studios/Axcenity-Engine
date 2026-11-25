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
        for (object, model) in objects.into_iter().zip(&loaded) {
            assert!(
                !model.primitives.is_empty(),
                "{object:?} picks nothing in {file}"
            );
            out.insert((file.clone(), object), ModelContents::of(model));
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
    // One family: the human base.
    assert_eq!(library.bases.keys().collect::<Vec<_>>(), ["axcenity/human"]);
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

#[test]
fn every_base_models_default_character_is_sound() {
    let library = read();
    for base in library.bases.values() {
        let mut description = Description::new("New Character", &base.id);
        description.items = base.defaults.items.iter().map(WornItem::new).collect();
        assert_eq!(lines(&description.validate(&library)), "", "{}", base.id);
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
    let of_feature = |base: &str, feature: &str| {
        library
            .presets
            .values()
            .filter(|preset| preset.base == base)
            .filter_map(|preset| preset.preset())
            .filter(|block| block.feature.as_deref() == Some(feature))
            .count()
    };
    // The human base (`assets/characters/human/presets.py`): its two
    // characters, six builds, eight faces, seven face shapes and noses.
    let human = |kind| kinds.get(&("axcenity/human".to_owned(), kind)).copied();
    assert_eq!(human(PresetKind::Character), Some(2));
    assert_eq!(human(PresetKind::Body), Some(6));
    assert_eq!(human(PresetKind::Face), Some(8));
    for (feature, presets) in [("nose", 7), ("face_shape", 7)] {
        assert_eq!(of_feature("axcenity/human", feature), presets, "{feature}");
    }
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
/// human body's underwear), even one that wears nothing.
#[test]
fn an_outfit_preset_leaves_the_chosen_underwear() {
    let library = read();
    let empty = Description::from_json(
        r#"{ "format_version": 2, "name": "Nothing", "base": "axcenity/human",
             "preset": { "id": "test/nothing", "kind": "outfit", "about": "Nothing on." } }"#,
    )
    .unwrap();
    let mut character = Description::new("Ada", "axcenity/human");
    character.parameters.insert("frame".to_owned(), 1.0);
    axcenity_avatar::wear_choices(&mut character, &library);
    let before = character.items.clone();
    assert_eq!(before.len(), 2, "boxer briefs and a bandeau");
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
    let families = [axcenity_avatar::DEFAULT_BASE];
    for family in families {
        let base = library.base(family).expect("the human base ships");
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

/// ADR-0048: every colour map a shipped base offers is an image the
/// engine reads, all of a material the same size; and the human base's
/// skins are MakeHuman's 22, each drawn by the characters it was made for:
/// the sex of the skin follows the body type, its age the Age slider, and
/// Randomize never draws the painted bodysuit.
#[test]
fn the_human_bases_skins_follow_the_body_type_and_the_age() {
    use axcenity_avatar::{Scope, randomize, resolve};
    let library = read();
    let assets = axcenity_assets::Assets::new(root());
    for (id, base) in &library.bases {
        let mut sizes: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
        for map in &base.material_maps {
            let path = library.path_of(id, &map.colour_map).unwrap();
            let image = assets
                .texture(&path)
                .unwrap_or_else(|| panic!("{path}: {:?}", assets.take_errors()));
            let size = (image.width, image.height);
            assert_eq!(*sizes.entry(&map.material).or_insert(size), size, "{path}");
        }
    }

    let human = library.base("axcenity/human").expect("shipped");
    let files: std::collections::BTreeSet<&str> = human
        .material_maps
        .iter()
        .map(|map| map.colour_map.as_str())
        .collect();
    assert_eq!(files.len(), 22, "MakeHuman's textured skins");
    let skin = |values: &[(&str, f32)]| {
        let mut character = Description::new("Skin", "axcenity/human");
        for (id, value) in values {
            character.parameters.insert((*id).to_owned(), *value);
        }
        let resolved = resolve(&library, &character).unwrap();
        assert_eq!(resolved.colour_maps.len(), 1);
        resolved.colour_maps["Skin"].clone()
    };
    // The default character: the model file's own skin, as a map.
    assert_eq!(skin(&[]), "human/skins/young_caucasian_male.jpg");
    assert_eq!(
        skin(&[("frame", 1.0)]),
        "human/skins/young_caucasian_female.jpg"
    );
    for (choice, tone) in [(0.0, "african"), (1.0, "asian"), (2.0, "caucasian")] {
        for (frame, sex) in [(0.0, "male"), (1.0, "female")] {
            // Young up to a quarter of the Age slider, old from three
            // quarters, middle-aged between.
            for (age, word) in [
                (0.0, "young"),
                (0.2, "young"),
                (0.3, "middleage"),
                (0.7, "middleage"),
                (0.8, "old"),
                (1.0, "old"),
            ] {
                assert_eq!(
                    skin(&[("skin", choice), ("frame", frame), ("age", age)]),
                    format!("human/skins/{word}_{tone}_{sex}.jpg"),
                );
            }
        }
    }
    // The second Caucasian pair is young only: older, the first pair's.
    assert_eq!(
        skin(&[("skin", 3.0), ("frame", 1.0)]),
        "human/skins/young_caucasian_female2.jpg"
    );
    assert_eq!(
        skin(&[("skin", 3.0), ("age", 0.2)]),
        "human/skins/young_caucasian_male2.jpg"
    );
    assert_eq!(
        skin(&[("skin", 3.0), ("age", 0.6)]),
        "human/skins/middleage_caucasian_male.jpg"
    );
    assert_eq!(
        skin(&[("skin", 3.0), ("age", 1.0)]),
        "human/skins/old_caucasian_male.jpg"
    );
    // The bodysuit is painted on: it stays on at every age.
    for age in [0.0, 0.5, 1.0] {
        assert_eq!(
            skin(&[("skin", 4.0), ("frame", 1.0), ("age", age)]),
            "human/skins/young_caucasian_female_special_suit.jpg"
        );
    }

    // Between two ages the skin fades from one photograph to the next
    // (each map's `blend`): none of the other at an age a skin was made
    // for, a half of it where the two are as near, and the same mix from
    // either side of that, so nothing snaps as the Age slider moves.
    let blend = |values: &[(&str, f32)]| {
        let mut character = Description::new("Skin", "axcenity/human");
        for (id, value) in values {
            character.parameters.insert((*id).to_owned(), *value);
        }
        let resolved = resolve(&library, &character).unwrap();
        let own = resolved.colour_maps["Skin"].clone();
        let toward = resolved.colour_blends.get("Skin").cloned();
        (own, toward)
    };
    for age in [0.0, 0.5, 1.0] {
        assert_eq!(blend(&[("age", age)]).1, None, "age {age}");
    }
    let young = "human/skins/young_caucasian_male.jpg".to_owned();
    let middle = "human/skins/middleage_caucasian_male.jpg".to_owned();
    let (own, toward) = blend(&[("age", 0.1)]);
    let (other, amount) = toward.expect("between two skins");
    assert_eq!((own, other), (young.clone(), middle.clone()));
    assert!((amount - 0.2).abs() < 1e-5, "{amount}");
    let (own, toward) = blend(&[("age", 0.4)]);
    let (other, amount) = toward.expect("between two skins");
    assert_eq!((own, other), (middle.clone(), young.clone()));
    assert!((amount - 0.2).abs() < 1e-5, "{amount}");
    // Just either side of the middle: half of each, whichever is "own".
    let before = blend(&[("age", 0.2499)]).1.unwrap();
    let after = blend(&[("age", 0.2501)]).1.unwrap();
    assert_eq!((before.0, after.0), (middle, young));
    assert!((before.1 - 0.5).abs() < 1e-3 && (after.1 - 0.5).abs() < 1e-3);
    // The bodysuit is one image at every age: nothing to fade to.
    assert_eq!(blend(&[("skin", 4.0), ("age", 0.3)]).1, None);
    // The second Caucasian pair ages into the first pair's skins.
    let (own, toward) = blend(&[("skin", 3.0), ("age", 0.1)]);
    assert_eq!(own, "human/skins/young_caucasian_male2.jpg");
    assert_eq!(
        toward.unwrap().0,
        "human/skins/middleage_caucasian_male.jpg"
    );

    let start = Description::new("Npc", "axcenity/human");
    let mut drawn: std::collections::BTreeSet<String> = Default::default();
    for seed in 0..1000 {
        let character = randomize(&start, &Scope::All, &Default::default(), seed, &library);
        assert!(character.parameters["skin"] <= 3.0, "seed {seed}");
        drawn.extend(
            resolve(&library, &character)
                .unwrap()
                .colour_maps
                .into_values(),
        );
    }
    assert_eq!(
        drawn.len(),
        20,
        "every photographed skin is drawn: {drawn:?}"
    );
}

/// ADR-0047 stage 5: the human base's hairstyles (`hair.py`). Each is a
/// groom with a mesh in the hair slot, credited, with a picture, on the
/// Hair shading model; it follows the head's shape and none of the face's
/// expressions; and every colour variant of it resolves on both body types.
#[test]
fn the_human_bases_hairstyles_follow_the_head_and_not_its_expressions() {
    use axcenity_avatar::{ItemKind, ShadingModel, resolve};
    let library = read();
    let human = library.base("axcenity/human").expect("shipped");
    let hair: Vec<_> = library
        .items
        .values()
        .filter(|item| item.kind == ItemKind::Groom && item.bases.contains(&human.id))
        .collect();
    assert!(hair.len() >= 5, "{} hairstyles", hair.len());
    // The standard facial controls the base has shapes for.
    let controls = [
        "Blink_",
        "Eye_",
        "Brow_",
        "Nose_",
        "Mouth_",
        "Lip_",
        "Upper_Lip_",
        "Lower_Lip_",
    ];
    for item in hair {
        let id = &item.id;
        assert_eq!(item.slot, "hair", "{id}");
        assert!(
            item.is_draft(),
            "{id}: a converted asset is a draft until reviewed"
        );
        let groom = item.groom.as_ref().expect("a groom");
        assert!(groom.mesh.is_some() && item.model.is_none(), "{id}");
        let source = item
            .source
            .as_ref()
            .unwrap_or_else(|| panic!("{id}: no credit"));
        assert!(
            matches!(source.licence.as_str(), "CC0-1.0" | "CC-BY-4.0") && !source.author.is_empty(),
            "{id}: {source:?}"
        );
        let picture = item.thumbnail.as_deref().expect("a picture");
        let picture = library.path_of(id, picture).unwrap();
        assert!(root().join(&picture).is_file(), "{picture}");
        assert_eq!(item.materials.len(), 1, "{id}");
        assert_eq!(item.materials[0].shading, ShadingModel::Hair, "{id}");
        let axcenity_avatar::Fit::Shapes { follows } = &item.fit else {
            panic!("{id}: fitted by its shapes");
        };
        assert!(follows.contains(&"Frame_Feminine".to_owned()), "{id}");
        let expressions: Vec<&String> = follows
            .iter()
            .filter(|shape| controls.iter().any(|control| shape.starts_with(control)))
            .collect();
        assert!(expressions.is_empty(), "{id} follows {expressions:?}");
        assert!(item.variants.len() >= 2, "{id}: colours to pick");
        for variant in &item.variants {
            for frame in [0.0, 1.0] {
                let mut character = Description::new("Hair", &human.id);
                character.parameters.insert("frame".to_owned(), frame);
                let mut worn = WornItem::new(id.clone());
                worn.variant = Some(variant.id.clone());
                character.items = vec![worn];
                let problems = character.validate(&library);
                assert!(
                    problems.is_empty(),
                    "{id} {}: {}",
                    variant.id,
                    lines(&problems)
                );
                let resolved = resolve(&library, &character).unwrap();
                assert!(
                    resolved.problems.is_empty(),
                    "{id}: {:?}",
                    resolved.problems
                );
                let part = resolved
                    .parts
                    .iter()
                    .find(|part| part.item.as_deref() == Some(id.as_str()))
                    .unwrap_or_else(|| panic!("{id} is worn"));
                assert_eq!(part.slot.as_deref(), Some("hair"));
                assert_eq!(
                    part.materials.get("Hair").map_or(0, |values| values.len()),
                    variant
                        .materials
                        .get("Hair")
                        .map_or(0, |values| values.len()),
                    "{id} {}",
                    variant.id
                );
            }
        }
    }
}

/// Paul, 2026-11-24: Randomize draws the human base's hairstyles though
/// they are drafts (its hair slot says `random_drafts`), so a random
/// character isn't always bald; it still never draws a draft garment, and
/// the other bases' drafts are never drawn.
#[test]
fn randomize_draws_the_human_bases_hairstyles_and_no_other_draft() {
    use axcenity_avatar::{Scope, randomize};
    let library = read();
    for base in library.bases.values() {
        let start = Description::new("Npc", &base.id);
        let mut hair: std::collections::BTreeSet<String> = Default::default();
        let mut bald = 0;
        for seed in 0..400 {
            let character = randomize(&start, &Scope::All, &Default::default(), seed, &library);
            let mut has_hair = false;
            for worn in &character.items {
                let item = library.item(&worn.id).unwrap();
                has_hair |= item.slot == "hair";
                // What a parameter's choice wears (the underwear) isn't drawn.
                let chosen = axcenity_avatar::chosen_slots(&library, &base.id);
                let drawn_draft = item.is_draft() && !chosen.contains(&item.slot);
                let allowed = base.slot(&item.slot).is_some_and(|slot| slot.random_drafts);
                assert!(
                    !drawn_draft || allowed,
                    "{} seed {seed} wears the draft {}",
                    base.id,
                    item.id
                );
                if item.slot == "hair" {
                    hair.insert(worn.id.clone());
                }
            }
            bald += usize::from(!has_hair);
        }
        if base.id == "axcenity/human" {
            assert_eq!(hair.len(), 10, "every shipped hairstyle is drawn: {hair:?}");
            // One choice in six is no hair at all.
            assert!((30..110).contains(&bald), "{bald} of 400 are bald");
        }
    }
}

/// The human base's face is laid out for a person (`face.py`): every
/// slider has a name that isn't MakeHuman's target name and fits
/// Character Studio's panel uncut (22 characters), every face page is a
/// feature with a handle, and every handle can be dragged: each of its
/// drives names a slider of its own feature's page and a direction on the
/// screen in at least one framing.
#[test]
fn the_human_bases_face_has_readable_names_features_and_handles() {
    let library = read();
    let base = library.base("axcenity/human").expect("the human base");
    for parameter in &base.parameters {
        let label = &parameter.label;
        assert!(
            label.chars().count() <= 22,
            "`{label}` is cut short in the panel"
        );
        assert!(
            !label.contains('_') && !label.contains("Incr") && !label.contains("Trans"),
            "`{label}` reads as a target's name"
        );
    }
    let pages: Vec<&str> = base
        .groups
        .iter()
        .filter(|group| group.category == axcenity_avatar::Category::Face)
        .map(|group| group.id.as_str())
        .collect();
    assert_eq!(pages.len(), 10, "{pages:?}");
    for page in &pages {
        assert!(base.feature(page).is_some(), "{page} is a feature");
        assert!(
            base.handles.iter().any(|handle| handle.feature == *page),
            "{page} has a handle"
        );
    }
    assert_eq!(base.handles.len(), 24);
    for handle in &base.handles {
        assert!(!handle.drives.is_empty(), "{}", handle.id);
        for drive in &handle.drives {
            let parameter = base.parameter(&drive.parameter).expect("its slider");
            assert_eq!(parameter.group, handle.feature, "{}", handle.id);
            assert!(
                !drive.screen.is_empty(),
                "{}: {} can't be dragged in any framing",
                handle.id,
                drive.parameter
            );
            assert!(drive.travel >= 0.0008, "{}", handle.id);
        }
        if let Some(mirror) = &handle.mirror {
            let other = base.handle(mirror).expect("its mirror");
            assert_eq!(other.mirror.as_deref(), Some(handle.id.as_str()));
        }
    }
}

/// The human base's layering (ADR-0045 on MakeHuman's clothes, Paul's
/// test of 2026-11-25: underwear came through the outfits). Every garment
/// of the base says what it hides beneath it: its cover map's green is lit
/// wherever its red is (cloth is never left drawn where the skin isn't)
/// and its blue wherever its green is. The underwear cut from the body
/// has the shape that presses it onto the skin under other clothes, and
/// wears it only then; a converted garment carries its corners' places on
/// the skin, so the engine tucks its edge under what is worn over it (the
/// socks of the shoes under the jeans). Under a garment that takes its
/// place the underwear isn't drawn at all (`Slot::hidden_by`).
/// And the frame moves the twist bones with the limbs they are inside.
#[test]
fn the_human_bases_underwear_goes_under_its_clothes() {
    use axcenity_avatar::{WornItem, resolve};
    let library = read();
    let assets = axcenity_assets::Assets::new(root());
    let mut garments = 0;
    for (id, item) in &library.items {
        if !item.bases.iter().any(|base| base == "axcenity/human") {
            continue;
        }
        let Some(cover) = &item.cover_map else {
            continue;
        };
        garments += 1;
        let path = library.path_of(id, cover).unwrap();
        let image = assets
            .texture(&path)
            .unwrap_or_else(|| panic!("{path}: {:?}", assets.take_errors()));
        let (mut red, mut green, mut bare, mut untucked) = (0, 0, 0, 0);
        for texel in image.rgba().chunks_exact(4) {
            let lit = |channel: usize| texel[channel] >= axcenity_assets::COVERED;
            red += usize::from(lit(0));
            green += usize::from(lit(1));
            bare += usize::from(lit(0) && !lit(1));
            untucked += usize::from(lit(1) && !lit(2));
        }
        // (A garment of net hides no skin: its red is empty.)
        assert!(green > 0 && green >= red, "{id}: red {red}, green {green}");
        assert_eq!(bare, 0, "{id}: skin hidden where cloth beneath isn't");
        assert_eq!(untucked, 0, "{id}: blue is at least green");
    }
    assert!(garments >= 50, "the underwear and the wardrobe: {garments}");

    let model = |id: &str| {
        let item = &library.items[id];
        let reference = item.model.as_ref().unwrap();
        let file = library.path_of(id, &reference.file).unwrap();
        let bytes = std::fs::read(root().join(&file)).unwrap();
        let pick = Pick::Objects(vec![reference.object.clone().unwrap()]);
        load_models(&bytes, Path::new("."), ModelFormat::Gltf, &[pick])
            .unwrap()
            .remove(0)
    };
    const BRIEFS: &str = "axcenity/human-boxer-briefs";
    const BANDEAU: &str = "axcenity/human-bandeau";
    for id in [BRIEFS, BANDEAU] {
        let contents = ModelContents::of(&model(id));
        assert!(
            contents
                .shapes
                .contains(axcenity_avatar::fit::SHRINK_UNDER_SHAPE),
            "{id} can be pressed onto the skin"
        );
        assert!(!model(id).has_skin_uvs(), "{id} lies on the skin");
    }
    for id in [
        "makehuman/female-casualsuit01",
        "makehuman/male-casualsuit01",
        "makehuman/shoes01",
        "makehuman/shoes04",
    ] {
        let garment = model(id);
        assert!(garment.has_skin_uvs(), "{id} knows its place on the skin");
        for primitive in &garment.primitives {
            for uv in &primitive.skin_uvs {
                assert!(
                    uv.iter()
                        .all(|c| c.is_finite() && (-0.01..=1.01).contains(c)),
                    "{id}: {uv:?}"
                );
            }
        }
    }

    // Worn: pressed only under something, and hidden by what is over it.
    let mut character = Description::new("Layers", "axcenity/human");
    character.parameters.insert("frame".into(), 1.0);
    character.items = vec![WornItem::new(BRIEFS), WornItem::new(BANDEAU)];
    let alone = resolve(&library, &character).unwrap();
    for part in alone.parts.iter().filter(|part| part.item.is_some()) {
        assert!(part.under.is_empty(), "{}", part.name);
        assert!(
            !part
                .shapes
                .contains_key(axcenity_avatar::fit::SHRINK_UNDER_SHAPE)
        );
    }
    // Under shoes, which take neither's place: both drawn, pressed onto
    // the skin and left out where the shoes' cover map says.
    let mut shod = character.clone();
    shod.items.push(WornItem::new("makehuman/shoes04"));
    let shod = resolve(&library, &shod).unwrap();
    assert_eq!(shod.problems, Vec::new());
    for id in [BRIEFS, BANDEAU] {
        let part = shod
            .parts
            .iter()
            .find(|part| part.item.as_deref() == Some(id))
            .unwrap();
        assert_eq!(
            part.shapes
                .get(axcenity_avatar::fit::SHRINK_UNDER_SHAPE)
                .copied(),
            Some(1.0),
            "{id}"
        );
        assert_eq!(
            part.under,
            ["human/wardrobe/shoes04/Shoes04.cover.png"],
            "{id}"
        );
    }

    // Paul, 2026-11-25: where a garment is worn over the underwear, the
    // underwear isn't drawn at all (`Slot::hidden_by`): a bottom takes
    // the briefs off, a top the bandeau, an outfit both. It is still worn
    // (no problem says otherwise) and comes back when the garment goes.
    let human = library.base("axcenity/human").unwrap();
    assert_eq!(
        human.slot("underwear_bottom").unwrap().hidden_by,
        ["bottom"]
    );
    assert_eq!(human.slot("underwear_top").unwrap().hidden_by, ["top"]);
    let drawn = |garment: &str| {
        let mut worn = character.clone();
        worn.items.push(WornItem::new(garment));
        let resolved = resolve(&library, &worn).unwrap();
        assert_eq!(resolved.problems, Vec::new(), "{garment}");
        let has = |id: &str| {
            resolved
                .parts
                .iter()
                .any(|part| part.item.as_deref() == Some(id))
        };
        assert!(has(garment), "{garment} is drawn");
        (has(BRIEFS), has(BANDEAU))
    };
    for (garment, briefs, bandeau) in [
        // An outfit fills the top and the bottom.
        ("makehuman/female-casualsuit01", false, false),
        ("makehuman/jean-shorts", false, true),
        ("makehuman/female-panties-01", false, true),
        ("makehuman/camisole-top", true, false),
        ("makehuman/crudefemaletshirt", true, false),
        ("makehuman/stockings-white-l", true, true),
        ("makehuman/fedora", true, true),
    ] {
        assert_eq!(drawn(garment), (briefs, bandeau), "{garment}");
    }

    let frame = human.parameter("frame").unwrap();
    for side in ["Left", "Right"] {
        for bone in [
            "UpperArmTwist",
            "LowerArmTwist",
            "UpperLegTwist",
            "LowerLegTwist",
        ] {
            let twist = format!("{side}{bone}");
            let inside = format!("{side}{}", bone.trim_end_matches("Twist"));
            let offset = |name: &str| {
                frame
                    .bones
                    .iter()
                    .find(|change| change.bone == name)
                    .and_then(|change| change.offset)
                    .unwrap_or_else(|| panic!("the frame moves {name}"))
            };
            let (a, b) = (offset(&twist), offset(&inside));
            let apart = (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f32>().sqrt();
            assert!(apart < 0.03, "{twist} goes with {inside}: {apart}");
        }
    }
}
