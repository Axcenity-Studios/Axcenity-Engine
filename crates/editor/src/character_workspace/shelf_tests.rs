//! The start page's side of Character Studio: Recent, the overview, delete,
//! packs (make, edit, rename, export, import, add to a project) and the
//! pictures.

use super::packs::{self, Pack};
use super::thumbnails::{self, Painter, Subject, Thumbnails};
use super::*;
use crate::character_studio::test_folder;

/// Saves a character named `name` on the realistic body in `folder`.
fn save(folder: &Path, name: &str, frame: f32) -> PathBuf {
    let mut description = Description::new(name, DEFAULT_BASE);
    description.parameters.insert("frame".into(), frame);
    let path = folder
        .join(CHARACTERS_FOLDER)
        .join(format!("{name}.character.json"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, description.to_json()).unwrap();
    path
}

#[test]
fn recent_puts_the_newest_first_and_drops_missing_files() {
    let folder = test_folder("recent");
    let [a, b, c] = ["Ada", "Bo", "Cy"].map(|name| save(&folder, name, 0.0));
    for file in [&a, &b, &c, &a] {
        recent::remember(&folder, file);
    }
    assert_eq!(
        recent::recent(&folder),
        vec![a.clone(), c.clone(), b.clone()]
    );
    // A file that's gone drops out; one outside `characters/` isn't kept.
    std::fs::remove_file(&c).unwrap();
    let elsewhere = folder.join("Elsewhere.character.json");
    std::fs::write(&elsewhere, "{}").unwrap();
    recent::remember(&folder, &elsewhere);
    assert_eq!(recent::recent(&folder), vec![a.clone(), b.clone()]);
    recent::forget(&folder, &a);
    assert_eq!(recent::recent(&folder), vec![b.clone()]);
    // It keeps at most `MAX_RECENT`.
    for n in 0..recent::MAX_RECENT + 5 {
        let file = save(&folder, &format!("N{n}"), 0.0);
        recent::remember(&folder, &file);
    }
    let kept = recent::recent(&folder);
    assert_eq!(kept.len(), recent::MAX_RECENT);
    assert!(kept[0].ends_with(format!("N{}.character.json", recent::MAX_RECENT + 4)));
    let _ = std::fs::remove_dir_all(&folder);
}

/// Opening a saved character and saving one both put it first in Recent.
#[test]
fn opening_and_saving_put_a_character_first_in_recent() {
    let folder = test_folder("recent-workspace");
    let mut workspace = CharacterWorkspace::open_preset(&folder, 0).unwrap();
    workspace.edit(|description| description.name = "First".into());
    let first = workspace.save().unwrap();
    // A second character (a rename would move First's file: audit D-03).
    workspace.new_character(0);
    workspace.edit(|description| description.name = "Second".into());
    let second = workspace.save().unwrap();
    assert_eq!(recent::recent(&folder), vec![second.clone(), first.clone()]);
    assert_eq!(workspace.saved_files, vec![first.clone(), second.clone()]);
    CharacterWorkspace::open_file(&folder, &first).unwrap();
    assert_eq!(recent::recent(&folder), vec![first, second]);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_overview_names_the_body_type_hair_and_what_it_wears() {
    let folder = test_folder("overview");
    let workspace = CharacterWorkspace::open_preset(&folder, 1).unwrap();
    let library = workspace.library();
    let female = shelf::overview(workspace.description(), &library);
    assert_eq!(female.body, "Female · Realistic", "{female:?}");
    assert!(female.look.starts_with("Bald · "), "{female:?}");
    assert!(female.look.contains("Sports bra"), "{female:?}");

    let athletic = presets()[2].description("Athletic", &library);
    let athletic = shelf::overview(&athletic, &library);
    assert_eq!(athletic.body, "Male · Realistic · 2 values set");
    assert!(athletic.look.contains("Boxer briefs"), "{athletic:?}");

    // The stylized default character wears its hair and clothes.
    let stylized = applier::default_description(&library, axcenity_avatar::V1_BASE);
    let stylized = shelf::overview(&stylized, &library);
    assert!(stylized.body.contains("Stylized"), "{stylized:?}");
    assert!(!stylized.look.starts_with("Bald"), "{stylized:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// Delete removes the file, its pictures, its Recent entry and its place in
/// packs; other characters are left alone.
#[test]
fn deleting_a_character_removes_its_file_picture_recent_entry_and_pack_place() {
    let folder = test_folder("delete");
    let ada = save(&folder, "Ada", 0.0);
    let bo = save(&folder, "Bo", 1.0);
    recent::remember(&folder, &bo);
    recent::remember(&folder, &ada);
    let text = std::fs::read_to_string(&ada).unwrap();
    let picture = thumbnails::character_picture(&folder, &ada, &text).unwrap();
    let older = thumbnails::character_picture(&folder, &ada, "an older version").unwrap();
    let bos = thumbnails::character_picture(&folder, &bo, "Bo").unwrap();
    std::fs::create_dir_all(picture.parent().unwrap()).unwrap();
    for file in [&picture, &older, &bos] {
        std::fs::write(file, b"png").unwrap();
    }
    packs::save_pack(
        &folder,
        None,
        "Duo",
        vec!["Ada.character.json".into(), "Bo.character.json".into()],
    )
    .unwrap();

    shelf::delete_character(&folder, &ada).unwrap();
    assert!(!ada.exists());
    assert!(!picture.exists() && !older.exists(), "its pictures go");
    assert!(bos.exists(), "another's picture stays");
    assert_eq!(recent::recent(&folder), vec![bo.clone()]);
    assert_eq!(
        packs::packs(&folder)[0].characters,
        vec!["Bo.character.json".to_owned()]
    );
    let listed: Vec<String> = shelf::list(&folder).into_iter().map(|c| c.name).collect();
    assert_eq!(listed, vec!["Bo".to_owned()]);
    let _ = std::fs::remove_dir_all(&folder);
}

/// A pack is made from saved characters, edited, renamed, exported as one
/// file and imported elsewhere (and again, without copies), and added to a
/// game project.
#[test]
fn a_pack_is_made_edited_renamed_exported_imported_and_added_to_a_project() {
    let folder = test_folder("packs");
    for (name, frame) in [("Ada", 1.0), ("Bo", 0.0), ("Cy", 0.0)] {
        save(&folder, name, frame);
    }
    let file = |name: &str| format!("{name}.character.json");
    assert!(packs::save_pack(&folder, None, " ", vec![file("Ada")]).is_err());
    assert!(packs::save_pack(&folder, None, "Empty", Vec::new()).is_err());
    let pack = packs::save_pack(&folder, None, "Villagers", vec![file("Ada"), file("Bo")]).unwrap();
    assert_eq!(packs::packs(&folder), vec![pack.clone()]);
    assert_eq!(pack.members(&folder).len(), 2);

    // Edit its characters, then rename it: the old file goes.
    let pack = packs::save_pack(
        &folder,
        Some(&pack.path),
        "Villagers",
        vec![file("Ada"), file("Bo"), file("Cy")],
    )
    .unwrap();
    packs::save_pack(&folder, None, "Guards", vec![file("Cy")]).unwrap();
    let clash = packs::save_pack(&folder, Some(&pack.path), "Guards", vec![file("Ada")]);
    assert!(clash.unwrap_err().contains("already a pack"));
    let renamed =
        packs::save_pack(&folder, Some(&pack.path), "Town", pack.characters.clone()).unwrap();
    assert!(!pack.path.exists());
    let names: Vec<String> = packs::packs(&folder).into_iter().map(|p| p.name).collect();
    assert_eq!(names, ["Guards", "Town"]);

    // Export: one plain JSON file with the characters in full.
    let exported = folder.join(packs::export_file_name(&renamed));
    assert!(exported.ends_with("Town.characterpack.json"));
    packs::export_pack(&folder, &renamed, &exported).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&exported).unwrap()).unwrap();
    assert_eq!(json["format_version"], packs::EXPORT_VERSION);
    assert_eq!(json["kind"], packs::EXPORT_KIND);
    assert_eq!(json["characters"].as_array().unwrap().len(), 3);
    assert_eq!(json["characters"][0]["name"], "Ada");

    // Import on another computer: the characters and the pack are there.
    let other = test_folder("packs-imported");
    let imported = packs::import_pack(&other, &exported).unwrap();
    assert_eq!(imported.name, "Town");
    assert_eq!(imported.characters, renamed.characters);
    let read = |folder: &Path, name: &str| {
        Description::from_json(
            &std::fs::read_to_string(folder.join(CHARACTERS_FOLDER).join(file(name))).unwrap(),
        )
        .unwrap()
    };
    assert_eq!(read(&other, "Ada"), read(&folder, "Ada"));
    // Again: the same characters are used, the pack gets a number.
    let again = packs::import_pack(&other, &exported).unwrap();
    assert_eq!(again.name, "Town 2");
    assert_eq!(again.characters, renamed.characters);
    assert_eq!(shelf::list(&other).len(), 3);
    // A different character of the same name is kept apart.
    save(&other, "Ada", 0.0);
    let third = packs::import_pack(&other, &exported).unwrap();
    assert!(third.characters.contains(&file("Ada 2")), "{third:?}");
    assert_eq!(read(&other, "Ada 2").name, "Ada 2");
    // Not a pack, or a newer one, is refused.
    std::fs::write(other.join("bad.json"), r#"{"name": "x"}"#).unwrap();
    assert!(packs::import_pack(&other, &other.join("bad.json")).is_err());
    std::fs::write(
        other.join("newer.json"),
        r#"{"format_version": 9, "kind": "axcenity.character-pack", "name": "x", "characters": []}"#,
    )
    .unwrap();
    assert!(
        packs::import_pack(&other, &other.join("newer.json"))
            .unwrap_err()
            .contains("newer")
    );

    // Add to Project: its characters in the project's descriptions folder,
    // and the library they need.
    let projects = test_folder("packs-project");
    let project = axcenity_project::create_project(&projects, "Game").unwrap();
    let written = packs::add_to_project(&folder, &renamed, &project).unwrap();
    assert_eq!(written.len(), 3);
    let added = project.join(packs::PROJECT_FOLDER).join(file("Bo"));
    assert_eq!(
        Description::from_json(&std::fs::read_to_string(added).unwrap()).unwrap(),
        read(&folder, "Bo")
    );
    assert!(
        project
            .join(axcenity_avatar::LIBRARY_FOLDER)
            .join("catalogue.json")
            .is_file()
    );
    assert!(
        packs::add_to_project(&folder, &renamed, &other).is_err(),
        "not a project"
    );

    // Deleting a pack keeps its characters.
    packs::delete_pack(&renamed).unwrap();
    assert_eq!(packs::packs(&folder).len(), 1);
    assert_eq!(shelf::list(&folder).len(), 3);
    for folder in [folder, other, projects] {
        let _ = std::fs::remove_dir_all(folder);
    }
}

#[test]
fn a_pictures_key_follows_the_file_and_presets_have_their_own() {
    let folder = test_folder("picture-keys");
    let file = folder.join(CHARACTERS_FOLDER).join("Ada.character.json");
    let one = thumbnails::character_picture(&folder, &file, "one").unwrap();
    assert_eq!(
        one,
        thumbnails::character_picture(&folder, &file, "one").unwrap()
    );
    assert_ne!(
        one,
        thumbnails::character_picture(&folder, &file, "two").unwrap()
    );
    let presets: Vec<PathBuf> = (0..presets().len())
        .map(|index| thumbnails::preset_picture(&folder, index).unwrap())
        .collect();
    assert!(
        presets
            .iter()
            .all(|path| !path.starts_with(one.parent().unwrap()))
    );
    assert!(thumbnails::preset_picture(&folder, 99).is_none());
}

/// A GPU device for drawing pictures, or `None` on a machine without one.
fn gpu() -> Option<(axcenity_render::wgpu::Device, axcenity_render::wgpu::Queue)> {
    let instance = axcenity_render::adapters::instance();
    let adapter = pollster::block_on(axcenity_render::adapters::adapter(&instance, None)).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// The fraction of a picture's middle column that isn't the backdrop at
/// its edge: the character standing there.
fn covered(pixels: &[u8]) -> f32 {
    let (width, height) = thumbnails::PICTURE_SIZE;
    let at = |x: u32, y: u32| {
        let i = ((y * width + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    };
    let edge = at(4, height / 2);
    let rows = height / 5..height * 4 / 5;
    let hit = rows
        .clone()
        .filter(|y| {
            at(width / 2, *y)
                .iter()
                .zip(&edge)
                .any(|(a, b)| a.abs_diff(*b) > 12)
        })
        .count();
    hit as f32 / rows.len() as f32
}

/// Pictures are the workspace's preview drawn offscreen: a character and
/// a preset are drawn, written where the cache says, drawn again under a
/// new key when the file changes (the old one removed), and drawn on the
/// page's thread too.
#[test]
fn pictures_are_drawn_cached_and_drawn_again_when_the_file_changes() {
    let Some((device, queue)) = gpu() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let folder = test_folder("pictures");
    let mut painter = Painter::new(&folder, &device, &queue).unwrap();
    let ada = save(&folder, "Ada", 1.0);
    let text = std::fs::read_to_string(&ada).unwrap();
    let description = Description::from_json(&text).unwrap();
    let out = thumbnails::character_picture(&folder, &ada, &text).unwrap();
    painter
        .paint_to(&thumbnails::Job {
            subject: Subject::Character(description.clone()),
            out: out.clone(),
        })
        .unwrap();
    let decoded = image::open(&out).unwrap().to_rgba8();
    assert_eq!(
        (decoded.width(), decoded.height()),
        thumbnails::PICTURE_SIZE
    );
    assert!(covered(decoded.as_raw()) > 0.6, "the character is drawn");
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        std::fs::copy(&out, snapshots.join("character-picture-ada.png")).unwrap();
    }

    // A preset, and a different body, look different.
    let preset = painter.paint(&Subject::Preset(0));
    assert!(covered(&preset) > 0.6);
    let differ = preset
        .chunks(4)
        .zip(decoded.as_raw().chunks(4))
        .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 8))
        .count();
    assert!(differ > preset.len() / 4 / 100, "Male and Female differ");

    // The file changes: a new key, drawn again, the old picture removed.
    let mut changed = description.clone();
    changed.parameters.insert("frame".into(), 0.0);
    std::fs::write(&ada, changed.to_json()).unwrap();
    let text = std::fs::read_to_string(&ada).unwrap();
    let again = thumbnails::character_picture(&folder, &ada, &text).unwrap();
    assert_ne!(again, out);
    painter
        .paint_to(&thumbnails::Job {
            subject: Subject::Character(changed),
            out: again.clone(),
        })
        .unwrap();
    assert!(again.is_file() && !out.exists());
    drop(painter);

    // The page's thread draws what's missing, and the page loads it.
    let ctx = crate::theme::test_context();
    let mut pictures = Thumbnails::default();
    pictures.set_folder(Some(folder.clone()));
    pictures.set_gpu(&device, &queue);
    pictures.poll(&ctx);
    let preset_out = thumbnails::preset_picture(&folder, 3).unwrap();
    pictures.request(Subject::Preset(3), preset_out.clone());
    assert!(pictures.pending(&preset_out));
    let started = Instant::now();
    while pictures.pending(&preset_out) {
        assert!(
            started.elapsed() < Duration::from_secs(300),
            "the thread draws it"
        );
        std::thread::sleep(Duration::from_millis(50));
        pictures.poll(&ctx);
    }
    assert!(preset_out.is_file());
    let _ = crate::theme::run(&ctx, Default::default(), |_| {
        assert!(pictures.texture(&ctx, &preset_out).is_some());
    });
    // Asking again for a picture that's there draws nothing.
    pictures.request(Subject::Preset(3), preset_out.clone());
    assert!(!pictures.pending(&preset_out));
    let _ = std::fs::remove_dir_all(&folder);
}

/// The packs listed are sorted by name and skip files of a newer format.
#[test]
fn packs_are_listed_by_name_and_newer_ones_skipped() {
    let folder = test_folder("pack-list");
    save(&folder, "Ada", 0.0);
    for name in ["zeta", "Alpha"] {
        packs::save_pack(&folder, None, name, vec!["Ada.character.json".into()]).unwrap();
    }
    std::fs::write(
        folder.join(packs::PACKS_FOLDER).join("Future.pack.json"),
        r#"{"format_version": 7, "name": "Future", "characters": []}"#,
    )
    .unwrap();
    let names: Vec<String> = packs::packs(&folder)
        .into_iter()
        .map(|pack: Pack| pack.name)
        .collect();
    assert_eq!(names, ["Alpha", "zeta"]);
    let _ = std::fs::remove_dir_all(&folder);
}
