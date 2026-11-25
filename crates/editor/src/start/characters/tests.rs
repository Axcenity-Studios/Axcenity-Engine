//! The Character Studio page: Recent with Show more, the cards and their
//! overviews, Delete asking first, the pack editor, and (with
//! `AXCENITY_SNAPSHOTS`) the page drawn with real pictures.

use super::*;
use crate::character_workspace::thumbnails::{Job, Painter};
use crate::character_workspace::{CHARACTERS_FOLDER, DEFAULT_BASE};
use crate::start::tests::drawn_text;
use axcenity_avatar::Description;

/// Saves `description` in `folder`'s characters.
fn save(folder: &Path, description: &Description) -> PathBuf {
    let path = folder
        .join(CHARACTERS_FOLDER)
        .join(format!("{}.character.json", description.name));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, description.to_json()).unwrap();
    path
}

fn realistic(name: &str, parameters: &[(&str, f32)]) -> Description {
    let mut description = Description::new(name, DEFAULT_BASE);
    for (id, value) in parameters {
        description.parameters.insert((*id).to_owned(), *value);
    }
    description
}

/// Six characters, saved oldest first and opened in that order, so the
/// newest is first in Recent.
fn six_characters(folder: &Path) -> Vec<PathBuf> {
    let cast = [
        realistic("Ada", &[("frame", 1.0)]),
        realistic("Bo", &[]),
        realistic("Cy", &[("muscle", 0.7), ("fat", -0.4)]),
        realistic("Dee", &[("frame", 1.0), ("fat", 0.8), ("muscle", 0.2)]),
        realistic("Eli", &[("fat", 0.8)]),
        realistic("Fay", &[("frame", 1.0), ("muscle", 0.6), ("fat", -0.4)]),
    ];
    cast.iter()
        .map(|description| {
            let path = save(folder, description);
            recent::remember(folder, &path);
            path
        })
        .collect()
}

fn page(folder: &Path) -> StartScreen {
    let mut start = StartScreen {
        page: Page::Characters,
        ..Default::default()
    };
    start.set_characters_folder(Some(folder.to_owned()));
    start
}

fn draw_page(start: &mut StartScreen, ui: &mut egui::Ui) -> Option<StartAction> {
    let ctx = &ui.ctx().clone();
    theme::apply(ctx, &theme::Appearance::default());
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::palette(ctx).panel))
        .show(ui, |ui| {
            let brand = Brand::load(ui.ctx());
            start.show(ui, &mut Vec::new(), &brand)
        })
        .inner
}

/// The page drawn `size` big, after it settles: what it says.
fn texts(start: &mut StartScreen, size: egui::Vec2) -> Vec<String> {
    let ctx = theme::test_context();
    ctx.enable_accesskit();
    let mut output = None;
    for frame in 0..4 {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), size)),
            time: Some(frame as f64 * 10.0),
            ..Default::default()
        };
        output = Some(crate::theme::run(&ctx, input, |ui| {
            assert!(draw_page(start, ui).is_none());
        }));
    }
    drawn_text(&output.unwrap())
        .into_iter()
        .map(|(text, ..)| text)
        .collect()
}

fn count(texts: &[String], text: &str) -> usize {
    texts.iter().filter(|t| *t == text).count()
}

#[test]
fn recent_shows_four_then_show_more_lists_the_rest() {
    let folder = crate::character_studio::test_folder("page-recent");
    six_characters(&folder);
    let mut start = page(&folder);
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    // The four newest are in Recent and Your characters; the two oldest
    // only in Your characters.
    for name in ["Fay", "Eli", "Dee", "Cy"] {
        assert_eq!(
            count(&drawn, &format!("label: {name}")),
            2,
            "{name}: {drawn:?}"
        );
    }
    for name in ["Bo", "Ada"] {
        assert_eq!(
            count(&drawn, &format!("label: {name}")),
            1,
            "{name}: {drawn:?}"
        );
    }
    assert!(drawn.iter().any(|t| t == "Show 2 more"), "{drawn:?}");
    // Each card has its overview, Edit and Delete; presets start a new one.
    assert!(
        drawn.iter().any(|t| t == "label: Female · Realistic"),
        "{drawn:?}"
    );
    assert!(
        drawn
            .iter()
            .any(|t| t == "label: Male · Realistic · 2 values set")
    );
    assert_eq!(count(&drawn, "Edit"), 10);
    assert_eq!(count(&drawn, "Delete…"), 10);
    // The realistic family's six whole-character presets, the stylized
    // family's three (plan section 0, decision 7; the stylized ones are
    // data since wave 3) and the human base's two (ADR-0047).
    assert_eq!(count(&drawn, "Start from this"), 11);
    for name in ["Masculine", "Feminine"] {
        assert!(
            drawn
                .iter()
                .any(|t| t.starts_with(&format!("label: {name}"))),
            "{name} in {drawn:?}"
        );
    }
    assert_eq!(count(&drawn, "label: Stylized character"), 0, "{drawn:?}");

    start.shelf.more_recent = true;
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    for name in ["Fay", "Ada"] {
        assert_eq!(count(&drawn, &format!("label: {name}")), 2, "{name}");
    }
    assert!(drawn.iter().any(|t| t == "Show less"), "{drawn:?}");

    // A file that's gone drops out of Recent.
    std::fs::remove_file(folder.join(CHARACTERS_FOLDER).join("Fay.character.json")).unwrap();
    start.shelf.stale();
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    assert_eq!(count(&drawn, "label: Fay"), 0);
    assert_eq!(count(&drawn, "label: Ada"), 2, "five, all shown: {drawn:?}");
    assert!(drawn.iter().any(|t| t == "Show less"), "{drawn:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn delete_asks_first_then_removes_the_character_everywhere() {
    let folder = crate::character_studio::test_folder("page-delete");
    let paths = six_characters(&folder);
    let mut start = page(&folder);
    texts(&mut start, vec2(1400.0, 3000.0));
    start.follow_pick(Pick::Delete(paths[0].clone()));
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    assert!(
        drawn.iter().any(|t| t == "label: Delete character?"),
        "{drawn:?}"
    );
    assert!(paths[0].exists(), "asked, not deleted yet");
    // What the dialog's Delete Character does.
    start.shelf.dialog = None;
    start.delete_character(&folder, &paths[0]);
    assert!(!paths[0].exists());
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    assert_eq!(count(&drawn, "label: Ada"), 0, "{drawn:?}");
    assert!(
        drawn.iter().any(|t| t == "label: Deleted \"Ada\"."),
        "{drawn:?}"
    );
    assert!(!recent::recent(&folder).contains(&paths[0]));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn edit_opens_a_character_and_a_preset_starts_one() {
    let folder = crate::character_studio::test_folder("page-edit");
    let paths = six_characters(&folder);
    let mut start = page(&folder);
    texts(&mut start, vec2(1400.0, 3000.0));
    assert!(matches!(
        start.follow_pick(Pick::Edit(paths[2].clone())),
        Some(StartAction::OpenCharacter(path)) if path == paths[2]
    ));
    let athletic = Start::Preset("axcenity/realistic-athletic-female".into());
    assert!(matches!(
        start.follow_pick(Pick::Preset(athletic.clone())),
        Some(StartAction::NewCharacter(picked)) if picked == athletic
    ));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_pack_editor_makes_and_renames_a_pack_shown_on_the_page() {
    let folder = crate::character_studio::test_folder("page-packs");
    six_characters(&folder);
    let mut start = page(&folder);
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    assert!(
        drawn.iter().any(|t| t == "label: No packs yet"),
        "{drawn:?}"
    );
    start.follow_pick(Pick::NewPack);
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    assert!(drawn.iter().any(|t| t == "label: New pack"), "{drawn:?}");
    // The dialog's fields, filled; Make Pack saves them.
    let Some(ShelfDialog::EditPack { name, picked, .. }) = &mut start.shelf.dialog else {
        panic!("the pack editor is open");
    };
    *name = "Villagers".into();
    picked.extend([
        "Ada.character.json".to_owned(),
        "Bo.character.json".to_owned(),
    ]);
    let pack = packs::save_pack(&folder, None, name, picked.clone()).unwrap();
    start.shelf.dialog = None;
    start.shelf.stale();
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    assert!(drawn.iter().any(|t| t.contains("Villagers")), "{drawn:?}");
    assert!(
        drawn.iter().any(|t| t == "label: 2 characters"),
        "{drawn:?}"
    );
    for button in ["Edit…", "Export…", "Add to Project…", "Delete pack…"] {
        assert!(drawn.iter().any(|t| t == button), "{button}: {drawn:?}");
    }
    // Edit opens the editor on it; Delete asks.
    start.follow_pick(Pick::Pack(0, PackButton::Edit));
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    assert!(drawn.iter().any(|t| t == "label: Edit pack"), "{drawn:?}");
    assert!(drawn.iter().any(|t| t == "label: 2 picked"), "{drawn:?}");
    start.shelf.dialog = None;
    start.follow_pick(Pick::Pack(0, PackButton::Delete));
    let drawn = texts(&mut start, vec2(1400.0, 3000.0));
    assert!(
        drawn.iter().any(|t| t == "label: Delete pack?"),
        "{drawn:?}"
    );
    assert!(pack.path.exists());
    let _ = std::fs::remove_dir_all(&folder);
}

/// Narrow windows: the cards go down to one column and stay inside it.
#[test]
fn the_page_fits_a_narrow_window() {
    assert_eq!(columns(300.0, MIN_CHARACTER_CARD_WIDTH), (1, 300.0));
    // A narrow page (a 640-point window with the sidebar) has two columns.
    assert_eq!(columns(340.0, MIN_CHARACTER_CARD_WIDTH).0, 2);
    let (n, width) = columns(1160.0, MIN_CHARACTER_CARD_WIDTH);
    assert!(n as f32 * width + (n - 1) as f32 * CARD_SPACING <= 1160.0);
    let folder = crate::character_studio::test_folder("page-narrow");
    six_characters(&folder);
    let mut start = page(&folder);
    let drawn = texts(&mut start, vec2(560.0, 3000.0));
    assert_eq!(count(&drawn, "label: Ada"), 1, "{drawn:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// With `AXCENITY_SNAPSHOTS`, the page with real pictures (drawn with the
/// workspace's preview), Recent and a pack, wide and narrow, and the pack
/// editor. Without a GPU the pictures are placeholders.
#[test]
fn the_character_page_draws_pictures_recent_and_a_pack() {
    let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) else {
        return;
    };
    let folder = crate::character_studio::test_folder("page-snapshot");
    let paths = six_characters(&folder);
    packs::save_pack(
        &folder,
        None,
        "Villagers",
        ["Ada", "Bo", "Cy", "Dee"]
            .map(|name| format!("{name}.character.json"))
            .to_vec(),
    )
    .unwrap();
    packs::save_pack(
        &folder,
        None,
        "Athletes",
        ["Cy", "Fay"]
            .map(|name| format!("{name}.character.json"))
            .to_vec(),
    )
    .unwrap();
    // The pictures, drawn as the page's thread draws them.
    let instance = axcenity_render::adapters::instance();
    let gpu = pollster::block_on(axcenity_render::adapters::adapter(&instance, None))
        .ok()
        .and_then(|adapter| pollster::block_on(adapter.request_device(&Default::default())).ok());
    if let Some((device, queue)) = gpu {
        let mut painter = Painter::new(&folder, &device, &queue).unwrap();
        for path in &paths {
            let text = std::fs::read_to_string(path).unwrap();
            painter
                .paint_to(&Job {
                    subject: Subject::Character(Description::from_json(&text).unwrap()),
                    out: thumbnails::character_picture(&folder, path, &text).unwrap(),
                })
                .unwrap();
        }
        let library = shelf::library_for(&folder);
        for listed in presets::all(&library, &Default::default()) {
            if listed.info.kind != axcenity_avatar::PresetKind::Character {
                continue;
            }
            let subject = Subject::Character(listed.pictured(&library));
            painter
                .paint_to(&Job {
                    out: thumbnails::preset_picture(&folder, listed.id, &subject),
                    subject,
                })
                .unwrap();
        }
    }
    std::fs::create_dir_all(&snapshots).unwrap();
    for (name, size) in [
        ("start-characters", vec2(1400.0, 2200.0)),
        ("start-characters-narrow", vec2(640.0, 3400.0)),
    ] {
        let mut start = page(&folder);
        crate::snapshot::snapshot(
            &snapshots.join(format!("{name}.png")),
            size,
            vec![Vec::new(); 6],
            |ui| {
                draw_page(&mut start, ui);
            },
        );
    }
    let mut start = page(&folder);
    crate::snapshot::snapshot(
        &snapshots.join("start-characters-pack-editor.png"),
        vec2(1400.0, 900.0),
        vec![Vec::new(); 6],
        |ui| {
            if start.shelf.dialog.is_none() && start.shelf.listed.is_some() {
                let pack = start.shelf.packs.last().cloned();
                start.shelf.dialog = Some(ShelfDialog::edit_pack(pack.as_ref()));
            }
            draw_page(&mut start, ui);
        },
    );
    let _ = std::fs::remove_dir_all(&folder);
}
