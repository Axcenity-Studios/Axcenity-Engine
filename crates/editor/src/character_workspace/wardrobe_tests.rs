//! The online wardrobe (`wardrobe.rs`), driven through the studio's
//! intents against a pack answered from memory (never the network): the
//! list only when asked and kept for the next time, a download checked and
//! put in the library and the Clothing browser, worn and removed, a file
//! that isn't the one listed, a cancelled download, and the library's own
//! items never written over. The pack's garments are the shipped crew tee
//! and joggers under IDs of their own, so a downloaded one is worn for
//! real. With `AXCENITY_SNAPSHOTS` the section is drawn
//! (`the_wardrobe_section_is_drawn`).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use axcenity_avatar::{Catalogue, LIBRARY_FOLDER, Library};
use axcenity_toolbox::SourceError;
use axcenity_toolbox::character_pack::{self, Pack};
use axcenity_toolbox::http::{Fixture, Http};
use eframe::egui;

use super::browser::{self, Browse, Shelf};
use super::intent::Intent;
use super::shell_tests::{drawn_texts, input};
use super::wardrobe::{self, Ask, State};
use super::{Category, CharacterWorkspace, Start, find_character_library};
use crate::character_studio::test_folder;
use crate::theme;

const AT: &str = "https://wardrobe.test/pack/";
const TEE: &str = "wardrobe-test/tee";
const JOGGERS: &str = "wardrobe-test/joggers";

/// A garment of the test pack: a shipped garment's files under another
/// ID, in a folder of its own.
struct Garment {
    id: &'static str,
    label: &'static str,
    slot: &'static str,
    /// Its folder in a library.
    folder: &'static str,
    /// The shipped garment it is made from: its folder in the repository's
    /// library, and its files' name.
    from: &'static str,
    name: &'static str,
}

const GARMENTS: [Garment; 2] = [
    Garment {
        id: TEE,
        label: "Wardrobe tee",
        slot: "top",
        folder: "human/imports/wardrobe-tee",
        from: "human/wardrobe/crude_male_shirt",
        name: "CrudeMaleShirt",
    },
    Garment {
        id: JOGGERS,
        label: "Wardrobe joggers",
        slot: "bottom",
        folder: "human/imports/wardrobe-joggers",
        from: "human/wardrobe/cargo_pants",
        name: "CargoPants",
    },
];

impl Garment {
    /// Its files in the pack: path and bytes. The manifest is the shipped
    /// one with this garment's ID and label, as a draft.
    fn files(&self) -> Vec<(String, Vec<u8>)> {
        let shipped = find_character_library().expect("the repository's library");
        let read = |ending: &str| {
            let file = shipped
                .join(self.from)
                .join(format!("{}.{ending}", self.name));
            std::fs::read(&file).unwrap_or_else(|err| panic!("{}: {err}", file.display()))
        };
        let mut manifest: serde_json::Value = serde_json::from_slice(&read("item.json")).unwrap();
        manifest["id"] = self.id.into();
        manifest["label"] = self.label.into();
        manifest["quality"] = "draft".into();
        // (Its cover map isn't one of the pack's files.)
        if let Some(fields) = manifest.as_object_mut() {
            fields.remove("cover_map");
        }
        let path = |ending: &str| format!("library/{}/{}.{ending}", self.folder, self.name);
        vec![
            (
                path("item.json"),
                serde_json::to_vec_pretty(&manifest).unwrap(),
            ),
            (path("glb"), read("glb")),
            (path("png"), read("png")),
        ]
    }

    fn manifest(&self) -> String {
        format!("{}/{}.item.json", self.folder, self.name)
    }

    fn picture(&self) -> String {
        format!("previews/{}.png", self.name)
    }
}

/// The test pack: its index and every file it serves, by address.
struct Served {
    index: String,
    files: Vec<(String, Vec<u8>)>,
}

/// The pack of [`GARMENTS`], each item's ID, manifest path and files as
/// `item` gives them (so a test can list an item that breaks a rule).
fn served_with(item: impl Fn(&Garment) -> (String, String)) -> Served {
    let mut files = Vec::new();
    let mut items = Vec::new();
    for garment in &GARMENTS {
        let (id, manifest) = item(garment);
        let listed: Vec<serde_json::Value> = garment
            .files()
            .into_iter()
            .map(|(path, bytes)| {
                // Tests run side by side: a scratch file each.
                static NEXT: AtomicUsize = AtomicUsize::new(0);
                let scratch = std::env::temp_dir().join(format!(
                    "axcenity-wardrobe-sum-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::write(&scratch, &bytes).unwrap();
                let sha256 = character_pack::sha256_of(&scratch).unwrap();
                let _ = std::fs::remove_file(&scratch);
                let entry = serde_json::json!({
                    "path": path, "bytes": bytes.len(), "sha256": sha256,
                });
                files.push((format!("{AT}{path}"), bytes));
                entry
            })
            .collect();
        // The picture is the garment's own thumbnail.
        let picture = garment.files().pop().unwrap().1;
        files.push((format!("{AT}{}", garment.picture()), picture));
        items.push(serde_json::json!({
            "id": id, "label": garment.label, "kind": "garment", "slot": garment.slot,
            "quality": "draft", "bases": ["axcenity/human"],
            "preview": garment.picture(),
            "catalogue": { "manifest": manifest },
            "files": listed,
        }));
    }
    let index = serde_json::json!({
        "format": "axcenity-character-pack", "format_version": 1,
        "name": "Test Wardrobe", "version": "0.1.0", "author": "Axcenity",
        "licence": "CC-BY-4.0", "credit": "Test Wardrobe by Axcenity, CC BY 4.0",
        "files_at": AT, "items": items,
    });
    Served {
        index: serde_json::to_string_pretty(&index).unwrap(),
        files,
    }
}

fn served() -> Served {
    served_with(|garment| (garment.id.to_owned(), garment.manifest()))
}

impl Served {
    fn http(&self) -> Fixture {
        let mut http = Fixture::default().with(&format!("{AT}pack.json"), self.index.clone());
        for (address, bytes) in &self.files {
            http = http.with(address, bytes.clone());
        }
        http
    }

    fn pack(&self) -> Pack {
        Pack::parse(&self.index, AT).unwrap()
    }
}

/// A workspace on the default character, on the Clothing page,
/// whose wardrobe is answered by `http`.
fn workspace(name: &str, http: impl Http + 'static) -> (PathBuf, CharacterWorkspace) {
    let folder = test_folder(name);
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.wardrobe.serve(Arc::new(http), AT);
    workspace.apply(Intent::PickCategory(Category::Clothing));
    workspace.apply(Intent::Browse(Browse::Online(true)));
    (folder, workspace)
}

fn ask(workspace: &mut CharacterWorkspace, ask: Ask) {
    workspace.apply(Intent::Wardrobe(ask));
    workspace.finish_wardrobe();
}

fn draw(workspace: &mut CharacterWorkspace) -> Vec<String> {
    let ctx = theme::test_context();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let output = theme::run(&ctx, input(Vec::new()), |ui| {
            assert_eq!(workspace.ui(ui, None), None);
        });
        texts = drawn_texts(&output);
    }
    texts
}

fn has(texts: &[String], text: &str) -> bool {
    texts.iter().any(|drawn| drawn.contains(text))
}

/// The IDs the Clothing page lists.
fn listed(workspace: &CharacterWorkspace) -> Vec<String> {
    let library = workspace.library();
    browser::list(
        &library,
        workspace.description(),
        Shelf::Clothing,
        workspace.browser.filters(Shelf::Clothing),
        workspace.browser.show_drafts,
        &workspace.item_favourites,
        &workspace.recent_items,
    )
    .entries
    .iter()
    .map(|entry| entry.item.id.clone())
    .collect()
}

fn wears(workspace: &CharacterWorkspace, id: &str) -> bool {
    workspace
        .description()
        .items
        .iter()
        .any(|worn| worn.id == id)
}

fn catalogue_text(folder: &Path) -> String {
    std::fs::read_to_string(folder.join(LIBRARY_FOLDER).join("catalogue.json")).unwrap()
}

#[test]
fn the_wardrobe_is_listed_only_when_asked_and_kept_for_next_time() {
    let (folder, mut workspace) = workspace("wardrobe-list", served().http());
    // Nothing is asked for by opening the page.
    workspace.finish_wardrobe();
    assert!(workspace.wardrobe.pack.is_none());
    let texts = draw(&mut workspace);
    assert!(has(&texts, "Your library") && has(&texts, "Online wardrobe"));
    assert!(has(&texts, "Show the online wardrobe"), "{texts:?}");
    assert!(workspace.wardrobe.pack.is_none());

    ask(&mut workspace, Ask::List);
    let pack = workspace.wardrobe.pack.clone().expect("the list");
    assert_eq!(pack.name, "Test Wardrobe");
    assert!(workspace.wardrobe.fresh);
    assert_eq!(workspace.wardrobe.state(TEE), State::Available);
    assert_eq!(workspace.wardrobe.state(JOGGERS), State::Available);
    assert_eq!(workspace.wardrobe.missing("axcenity/human").0, 2);
    // Its pictures came with it, into the wardrobe's own folder.
    let pictures = folder.join(wardrobe::FOLDER).join("previews").join("0.1.0");
    assert!(pictures.join("wardrobe-test-tee.png").is_file());
    assert!(pictures.join("wardrobe-test-joggers.png").is_file());
    let texts = draw(&mut workspace);
    for text in [
        "Test Wardrobe 0.1.0",
        "2 garments for this body",
        "Free to use if you credit the author.",
        "Copy credit line",
        "Wardrobe tee",
        "Wardrobe joggers",
        "Download all 2",
    ] {
        assert!(has(&texts, text), "{text:?} isn't drawn: {texts:?}");
    }
    // The page's search narrows the wardrobe too.
    let mut filters = workspace.browser.filters(Shelf::Clothing).clone();
    filters.search = "joggers".to_owned();
    workspace.apply(Intent::Browse(Browse::Filters(Shelf::Clothing, filters)));
    let texts = draw(&mut workspace);
    assert!(has(&texts, "Wardrobe joggers") && !has(&texts, "Wardrobe tee"));
    // Nothing of it is in the library, the character or its undo.
    assert!(workspace.library().item(TEE).is_none());
    assert!(!workspace.can_undo());

    // Another session, with nothing answering: the kept list shows, and
    // nothing is fetched for it.
    drop(workspace);
    let mut again = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    again.wardrobe.serve(Arc::new(Fixture::default()), AT);
    again.apply(Intent::PickCategory(Category::Clothing));
    again.apply(Intent::Browse(Browse::Online(true)));
    again.finish_wardrobe();
    assert_eq!(again.wardrobe.pack.as_deref(), Some(&*pack));
    assert!(!again.wardrobe.fresh);
    assert!(again.wardrobe.problem.is_none());
    assert!(has(&draw(&mut again), "As listed the last time you asked."));
    // Asking again, offline, says why and keeps the list.
    ask(&mut again, Ask::List);
    assert!(again.wardrobe.pack.is_some());
    let problem = again.wardrobe.problem.clone().expect("a problem");
    assert!(
        problem.starts_with("Could not ask the wardrobe"),
        "{problem}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_downloaded_garment_is_in_the_library_worn_and_removed_again() {
    let (folder, mut workspace) = workspace("wardrobe-download", served().http());
    ask(&mut workspace, Ask::List);
    assert!(!workspace.browser.show_drafts);
    ask(&mut workspace, Ask::Get(TEE.to_owned()));

    // In the library, where the pack puts it, every file checked.
    let library = workspace.library();
    let item = library.item(TEE).expect("the tee in the library");
    assert_eq!(item.label, "Wardrobe tee");
    assert_eq!(
        library.manifest_path(TEE),
        Some("human/imports/wardrobe-tee/CrudeMaleShirt.item.json")
    );
    let root = folder.join(LIBRARY_FOLDER);
    for (path, bytes) in GARMENTS[0].files() {
        let file = root.join(path.strip_prefix("library/").unwrap());
        assert_eq!(std::fs::read(&file).unwrap(), bytes, "{}", file.display());
    }
    // The library reads whole, and its catalogue is the one its manifests
    // make: the entry is the same as any other item's.
    let (read, catalogue, problems) = Library::read(&root);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(catalogue, Some(Catalogue::of(&read)));
    // Nothing is left of the download.
    let staging = folder.join(wardrobe::FOLDER).join("staging");
    assert!(!staging.join("wardrobe-test-tee").exists());
    assert_eq!(workspace.wardrobe.state(TEE), State::Installed);
    assert_eq!(workspace.wardrobe.state(JOGGERS), State::Available);
    assert!(workspace.wardrobe.job.is_none());

    // What was asked for is shown: it is a draft, so drafts are listed.
    assert!(workspace.browser.show_drafts);
    let notice = workspace.notice.clone().unwrap_or_default();
    assert!(
        notice.contains("Wardrobe tee is in your library"),
        "{notice}"
    );
    assert!(notice.contains("Show drafts is on"), "{notice}");
    assert!(listed(&workspace).contains(&TEE.to_owned()));
    // Getting it changed neither the character nor its undo.
    assert!(!wears(&workspace, TEE));
    assert!(!workspace.can_undo());
    let texts = draw(&mut workspace);
    assert!(has(&texts, "In library"), "{texts:?}");
    assert!(has(&texts, "Remove"), "{texts:?}");

    // It is worn like any other garment: one undo step, a part on the body.
    workspace.notice = None;
    workspace.apply(Intent::Wear {
        slot: "top".to_owned(),
        item: Some(TEE.to_owned()),
    });
    assert!(wears(&workspace, TEE));
    assert!(workspace.can_undo());
    assert_eq!(workspace.notice, None, "the garment couldn't be worn");
    let worn = workspace.worn();
    assert!(
        worn.iter().any(|part| part.contains("CrudeMaleShirt")),
        "{worn:?}"
    );

    // Worn, it stays in the library.
    ask(&mut workspace, Ask::Remove(TEE.to_owned()));
    assert!(workspace.library().item(TEE).is_some());
    let notice = workspace.notice.clone().unwrap_or_default();
    assert!(notice.contains("Take it off"), "{notice}");
    // Taken off, it goes: its folder and its entry, nothing else.
    workspace.apply(Intent::Wear {
        slot: "top".to_owned(),
        item: None,
    });
    ask(&mut workspace, Ask::Remove(TEE.to_owned()));
    assert!(workspace.library().item(TEE).is_none());
    assert!(!root.join("human/imports/wardrobe-tee").exists());
    assert_eq!(workspace.wardrobe.state(TEE), State::Available);
    let (read, catalogue, problems) = Library::read(&root);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(catalogue, Some(Catalogue::of(&read)));
    assert!(read.item("makehuman/crude-male-shirt").is_some());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn download_all_gets_every_garment_one_after_another() {
    let (folder, mut workspace) = workspace("wardrobe-all", served().http());
    ask(&mut workspace, Ask::List);
    ask(&mut workspace, Ask::GetAll);
    let library = workspace.library();
    assert!(library.item(TEE).is_some() && library.item(JOGGERS).is_some());
    assert_eq!(workspace.wardrobe.missing("axcenity/human"), (0, 0));
    assert!(workspace.wardrobe.queue.is_empty());
    // Asked for again, there is nothing to get.
    workspace.apply(Intent::Wardrobe(Ask::GetAll));
    workspace.apply(Intent::Wardrobe(Ask::Get(TEE.to_owned())));
    assert!(workspace.wardrobe.queue.is_empty());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_file_that_isnt_the_one_listed_never_reaches_the_library() {
    let served = served();
    // The tee's model answers with other bytes of the same length.
    let model = format!("{AT}library/human/imports/wardrobe-tee/CrudeMaleShirt.glb");
    let length = served
        .files
        .iter()
        .find(|(address, _)| *address == model)
        .map(|(_, bytes)| bytes.len())
        .unwrap();
    let http = served.http().with(&model, vec![7u8; length]);
    let (folder, mut workspace) = workspace("wardrobe-wrong", http);
    ask(&mut workspace, Ask::List);
    let before = catalogue_text(&folder);
    // The joggers wait behind it: a failure stops the line.
    workspace.apply(Intent::Wardrobe(Ask::Get(TEE.to_owned())));
    workspace.apply(Intent::Wardrobe(Ask::Get(JOGGERS.to_owned())));
    workspace.finish_wardrobe();
    let notice = workspace.notice.clone().unwrap_or_default();
    assert!(
        notice.starts_with("Could not download Wardrobe tee"),
        "{notice}"
    );
    assert!(notice.contains("checksum"), "{notice}");
    assert!(workspace.library().item(TEE).is_none());
    assert!(workspace.library().item(JOGGERS).is_none());
    assert_eq!(catalogue_text(&folder), before);
    assert!(!folder.join(LIBRARY_FOLDER).join("human/imports").exists());
    assert_eq!(workspace.wardrobe.state(TEE), State::Available);
    assert!(!workspace.browser.show_drafts);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_character_that_names_a_garment_wears_it_once_it_is_downloaded() {
    let (folder, mut workspace) = workspace("wardrobe-named", served().http());
    // A character saved where the tee was in the library, opened where it
    // isn't: the tee is named and can't be shown.
    workspace.edit(|description| {
        description
            .items
            .push(axcenity_avatar::WornItem::new(TEE.to_owned()));
    });
    assert!(wears(&workspace, TEE));
    let shown = |workspace: &CharacterWorkspace| {
        workspace
            .worn()
            .iter()
            .any(|part| part.contains("CrudeMaleShirt"))
    };
    assert!(!shown(&workspace));
    ask(&mut workspace, Ask::List);
    ask(&mut workspace, Ask::Get(TEE.to_owned()));
    assert!(shown(&workspace), "{:?}", workspace.worn());
    let notice = workspace.notice.clone().unwrap_or_default();
    assert!(notice.contains("is in your library"), "{notice}");
    // Worn, it can't be removed.
    ask(&mut workspace, Ask::Remove(TEE.to_owned()));
    assert!(workspace.library().item(TEE).is_some());
    let _ = std::fs::remove_dir_all(&folder);
}

/// Answers as a [`Fixture`], but a model (`.glb`) only once it's let go:
/// a download that is still under way when the test looks.
struct Gated {
    fixture: Fixture,
    gate: Mutex<Receiver<()>>,
}

impl Gated {
    fn new(fixture: Fixture) -> (Self, Sender<()>) {
        let (open, gate) = mpsc::channel();
        let gated = Self {
            fixture,
            gate: Mutex::new(gate),
        };
        (gated, open)
    }
}

impl Http for Gated {
    fn fetch(&self, url: &str, limit: u64, out: &mut dyn Write) -> Result<u64, SourceError> {
        if url.ends_with(".glb") {
            let _ = self.gate.lock().unwrap().recv();
        }
        self.fixture.fetch(url, limit, out)
    }
}

#[test]
fn cancel_stops_the_download_and_forgets_the_ones_waiting() {
    let (http, open) = Gated::new(served().http());
    let (folder, mut workspace) = workspace("wardrobe-cancel", http);
    ask(&mut workspace, Ask::List);
    let before = catalogue_text(&folder);
    workspace.apply(Intent::Wardrobe(Ask::GetAll));
    // The first is under way (its model held back), the other waits.
    workspace.wardrobe_step(None);
    let job = workspace
        .wardrobe
        .job
        .clone()
        .expect("a download under way");
    assert_eq!(workspace.wardrobe.queue.len(), 1);
    assert!(job.done() <= job.bytes);
    let texts = draw(&mut workspace);
    assert!(has(&texts, "1 waiting"), "{texts:?}");
    assert!(has(&texts, "Cancel"), "{texts:?}");
    assert!(has(&texts, "Waiting"), "{texts:?}");

    workspace.apply(Intent::Wardrobe(Ask::Cancel));
    assert!(workspace.wardrobe.queue.is_empty());
    // Let go, the model's first block is refused: the download stops.
    drop(open);
    workspace.finish_wardrobe();
    assert!(workspace.wardrobe.job.is_none());
    let notice = workspace.notice.clone().unwrap_or_default();
    assert!(notice.contains("was cancelled"), "{notice}");
    assert!(workspace.library().item(TEE).is_none());
    assert!(workspace.library().item(JOGGERS).is_none());
    assert_eq!(catalogue_text(&folder), before);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_librarys_own_items_are_never_written_over() {
    // A pack whose tee has the shipped crew tee's ID, and whose joggers
    // go into the shipped joggers' folder under an ID of their own.
    let odd = served_with(|garment| match garment.id {
        TEE => ("makehuman/crude-male-shirt".to_owned(), garment.manifest()),
        _ => (
            garment.id.to_owned(),
            "human/wardrobe/cargo_pants/CargoPants.item.json".to_owned(),
        ),
    });
    // The joggers' files are listed in their own folder, so the pack's
    // reader already leaves that item out; the tee it can't tell.
    let pack = odd.pack();
    assert_eq!(pack.items.len(), 1);
    assert_eq!(pack.refused, 1);
    let (folder, mut workspace) = workspace("wardrobe-own", odd.http());
    ask(&mut workspace, Ask::List);
    let before = catalogue_text(&folder);
    assert_eq!(
        workspace.wardrobe.state("makehuman/crude-male-shirt"),
        State::Taken
    );
    // Asked for anyway, it isn't downloaded.
    ask(
        &mut workspace,
        Ask::Get("makehuman/crude-male-shirt".to_owned()),
    );
    assert_eq!(catalogue_text(&folder), before);
    assert!(has(&draw(&mut workspace), "Yours"));

    // Past the list, the install itself refuses both: an ID the library
    // has elsewhere, and a folder that holds another of its items.
    let root = folder.join(LIBRARY_FOLDER);
    let stage = |name: &str, garment: &Garment, id: &str| {
        let staging = folder.join(name);
        std::fs::create_dir_all(&staging).unwrap();
        for (path, bytes) in garment.files() {
            let bytes = if path.ends_with(".item.json") {
                let mut manifest: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                manifest["id"] = id.into();
                serde_json::to_vec(&manifest).unwrap()
            } else {
                bytes
            };
            std::fs::write(staging.join(path.rsplit('/').next().unwrap()), bytes).unwrap();
        }
        staging
    };
    let tee = &pack.items[0];
    let staging = stage("staged-tee", &GARMENTS[0], "makehuman/crude-male-shirt");
    let refused = wardrobe::install(&root, tee, &staging).unwrap_err();
    assert!(refused.contains("of its own with that ID"), "{refused}");

    let mut joggers = served().pack().items[1].clone();
    joggers.catalogue.manifest = "human/wardrobe/cargo_pants/CargoPants.item.json".to_owned();
    let staging = stage("staged-joggers", &GARMENTS[1], JOGGERS);
    let shipped = root.join("human/wardrobe/cargo_pants/CargoPants.item.json");
    let shipped_before = std::fs::read(&shipped).unwrap();
    let refused = wardrobe::install(&root, &joggers, &staging).unwrap_err();
    assert!(refused.contains("holds something else"), "{refused}");
    assert_eq!(std::fs::read(&shipped).unwrap(), shipped_before);

    // And a manifest that is another item's than the one listed.
    let staging = stage("staged-other", &GARMENTS[1], "wardrobe-test/other");
    let real = served().pack().items[1].clone();
    let refused = wardrobe::install(&root, &real, &staging).unwrap_err();
    assert!(
        refused.contains("not the item the wardrobe lists"),
        "{refused}"
    );
    assert_eq!(catalogue_text(&folder), before);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_newer_version_in_the_wardrobe_is_offered_as_an_update() {
    let (folder, mut workspace) = workspace("wardrobe-update", served().http());
    ask(&mut workspace, Ask::List);
    ask(&mut workspace, Ask::Get(TEE.to_owned()));
    assert_eq!(workspace.wardrobe.state(TEE), State::Installed);
    // The library's copy is no longer the file the index lists.
    let thumbnail = folder
        .join(LIBRARY_FOLDER)
        .join("human/imports/wardrobe-tee/CrudeMaleShirt.png");
    std::fs::write(&thumbnail, b"an older picture").unwrap();
    ask(&mut workspace, Ask::List);
    assert_eq!(workspace.wardrobe.state(TEE), State::Outdated);
    assert!(has(&draw(&mut workspace), "Update"));
    ask(&mut workspace, Ask::Get(TEE.to_owned()));
    assert_eq!(workspace.wardrobe.state(TEE), State::Installed);
    assert_ne!(std::fs::read(&thumbnail).unwrap(), b"an older picture");
    let _ = std::fs::remove_dir_all(&folder);
}

/// With `AXCENITY_SNAPSHOTS`: the Clothing page with the wardrobe listed,
/// one garment downloaded.
#[test]
fn the_wardrobe_section_is_drawn() {
    let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) else {
        return;
    };
    let (folder, mut workspace) = workspace("wardrobe-snapshot", served().http());
    ask(&mut workspace, Ask::List);
    ask(&mut workspace, Ask::Get(TEE.to_owned()));
    snapshot(&snapshots, "character-wardrobe.png", &mut workspace);
    let _ = std::fs::remove_dir_all(&folder);
}

/// The real Axcenity Wardrobe, end to end: its list reads whole, its
/// smallest garment downloads, checks and goes into the library, and the
/// character wears it. With `AXCENITY_SNAPSHOTS` the wardrobe and the
/// character wearing the garment are drawn. Ignored: it reaches the
/// network (the list, its pictures and one garment: about 15 MB).
#[test]
#[ignore = "reaches the network"]
fn the_real_wardrobe_is_listed_downloaded_and_worn() {
    let folder = test_folder("wardrobe-live");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::PickCategory(Category::Clothing));
    workspace.apply(Intent::Browse(Browse::Online(true)));
    ask(&mut workspace, Ask::List);
    assert_eq!(workspace.wardrobe.problem, None);
    let pack = workspace
        .wardrobe
        .pack
        .clone()
        .expect("the wardrobe's list");
    assert_eq!(pack.refused, 0);
    let snapshots = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from);
    if let Some(snapshots) = &snapshots {
        snapshot(snapshots, "character-wardrobe-live.png", &mut workspace);
    }
    let smallest = pack
        .items
        .iter()
        .min_by_key(|item| item.bytes())
        .expect("a garment");
    ask(&mut workspace, Ask::Get(smallest.id.clone()));
    let notice = workspace.notice.clone().unwrap_or_default();
    assert!(notice.contains("is in your library"), "{notice}");
    assert_eq!(workspace.wardrobe.state(&smallest.id), State::Installed);
    let (_, _, problems) = Library::read(&folder.join(LIBRARY_FOLDER));
    assert!(problems.is_empty(), "{problems:?}");
    workspace.notice = None;
    workspace.apply(Intent::Browse(Browse::Online(false)));
    workspace.apply(Intent::Wear {
        slot: smallest.slot.clone(),
        item: Some(smallest.id.clone()),
    });
    assert!(wears(&workspace, &smallest.id));
    assert_eq!(workspace.notice, None, "the garment couldn't be worn");
    eprintln!(
        "downloaded and wore {} ({} bytes): {:?}",
        smallest.id,
        smallest.bytes(),
        workspace.worn()
    );
    if let Some(snapshots) = &snapshots {
        snapshot(
            snapshots,
            "character-wardrobe-live-worn.png",
            &mut workspace,
        );
    }
    let _ = std::fs::remove_dir_all(&folder);
}

fn snapshot(snapshots: &Path, name: &str, workspace: &mut CharacterWorkspace) {
    workspace.canvas = Default::default();
    crate::snapshot::snapshot_rendered(
        &snapshots.join(name),
        egui::vec2(1440.0, 900.0),
        vec![Vec::new(); 24],
        |ui, render_state| {
            workspace.ui(ui, Some(render_state));
        },
    );
}
