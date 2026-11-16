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
        per_base
            .entry(description.base.clone())
            .or_default()
            .push(name.to_owned());
    }
    for (id, base) in &library.bases {
        let names = per_base.get(id).map(Vec::as_slice).unwrap_or_default();
        let has = |part: &str| names.iter().any(|name| name.contains(part));
        assert!(has("-body-default"), "{id} has no body benchmarks");
        assert!(has("-face-neutral"), "{id} has no face benchmarks");
        assert!(has("-outfit-underwear"), "{id} has no outfit benchmarks");
        assert!(
            names.iter().all(|name| name.starts_with(&base.family)),
            "{id}: benchmarks are named <family>-<frame>-<case>"
        );
    }
}
