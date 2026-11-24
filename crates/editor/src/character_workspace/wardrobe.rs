//! The online wardrobe: the Clothing page lists the garments of the
//! Axcenity Wardrobe (a character pack, `toolbox::character_pack`: a
//! repository with a `pack.json` index) and downloads the ones asked for
//! into the studio's character library, where the Clothing browser then
//! offers them like any other item. Drawing is `panel/wardrobe.rs`; this
//! file is the state, the threads and the library's side.
//!
//! - **Nothing is fetched until it is asked for.** The list is asked for
//!   with a button; the answer is kept (`wardrobe/pack.json` in the
//!   studio's folder, pictures in `wardrobe/previews/`), so the page shows
//!   the kept list at once the next time, and asks again only on Refresh.
//! - **A download** is one garment at a time, in the order asked, on a
//!   thread of its own: its files come into `wardrobe/staging/<item>/`,
//!   each checked against the size and SHA-256 the index lists, and only a
//!   whole, checked item goes into the library ([`install`]). A cancelled
//!   or failed download leaves the library as it was and keeps the files
//!   it finished, so asking again goes on from them.
//! - **In the library** a garment sits where the pack says
//!   (`realistic/imports/<name>/`), listed in `catalogue.json`. An item
//!   never goes over the engine's own or a person's own: an ID the library
//!   has at another path, or a folder that holds something else, refuses
//!   it. Remove takes the folder and the entry out again.
//! - It is all editor state: never in a description, never an undo step.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use axcenity_avatar::{Catalogue, CatalogueItem, ItemManifest, LIBRARY_FOLDER, Library};
use axcenity_toolbox::SourceError;
use axcenity_toolbox::character_pack::{self, Pack, PackItem, Progress};
use axcenity_toolbox::http::{Client, Http};
use eframe::egui;

use super::{Category, CharacterWorkspace, WRITING_LIBRARY};

/// The wardrobe's folder in the studio's folder: the kept index, the
/// pictures and the downloads under way.
pub const FOLDER: &str = "wardrobe";

/// How many pictures are fetched at once.
const PICTURE_THREADS: usize = 4;
/// How many pictures are read from disk in a frame.
const PICTURES_PER_FRAME: usize = 2;
/// A picture's longest side as a texture (the pack's are 512).
const PICTURE_SIZE: u32 = 256;

/// What the wardrobe section asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Asks the wardrobe what it has (the first time, or again).
    List,
    /// Downloads the item with this ID into the library.
    Get(String),
    /// Downloads every item for the character's body that isn't in the
    /// library, or is there in an older version.
    GetAll,
    /// Stops the download under way and forgets the ones waiting.
    Cancel,
    /// Takes a downloaded item out of the library.
    Remove(String),
}

/// Where an item of the wardrobe stands in the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Not in the library.
    Available,
    /// In the library, every file the size the index lists.
    Installed,
    /// In the library, but not the files the index lists now.
    Outdated,
    /// The library has an item of its own with this ID, elsewhere.
    Taken,
}

/// The download under way.
#[derive(Clone, Debug)]
pub struct Job {
    pub id: String,
    pub label: String,
    pub bytes: u64,
    pub progress: Arc<Progress>,
}

impl Job {
    /// How much of it is in hand.
    pub fn done(&self) -> u64 {
        self.progress.done.load(Ordering::Relaxed).min(self.bytes)
    }
}

/// A picture of an item: its file, and its texture once read.
struct Picture {
    file: PathBuf,
    /// Whether the file is there.
    have: bool,
    /// Asked for and not got: not asked for again until the next Refresh.
    failed: bool,
    texture: Option<egui::TextureHandle>,
}

/// What a thread tells the workspace.
enum Message {
    /// The index read, and its text to keep (boxed: far the largest).
    Listed(Result<Box<(Pack, String)>, String>),
    Picture {
        id: String,
        got: bool,
    },
    PicturesDone,
    Installed {
        id: String,
        result: Result<(), Failure>,
    },
}

/// Why a download didn't end in the library.
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Cancelled,
    Failed(String),
}

/// The online wardrobe's state (editor state).
pub struct Wardrobe {
    /// What answers in tests in place of the network.
    http: Option<Arc<dyn Http>>,
    /// Where the pack is, and what its files' addresses must start with.
    address: String,
    allowed: String,
    /// The list: this session's answer, or the kept copy.
    pub pack: Option<Arc<Pack>>,
    /// Whether the list was asked for in this session (else it is the
    /// kept copy, and nothing is fetched for it).
    pub fresh: bool,
    /// The list is being asked for (`wanted`: from the next frame).
    asking: bool,
    wanted: bool,
    /// Why the last ask failed.
    pub problem: Option<String>,
    /// Each listed item's place in the library.
    pub states: BTreeMap<String, State>,
    /// The items waiting to be downloaded, in order.
    pub queue: VecDeque<String>,
    pub job: Option<Job>,
    /// The kept copy was looked for.
    looked: bool,
    pictures: HashMap<String, Picture>,
    picture_threads: usize,
    tx: Sender<Message>,
    rx: Receiver<Message>,
}

impl Default for Wardrobe {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            http: None,
            address: character_pack::WARDROBE.to_owned(),
            allowed: character_pack::WARDROBE_FILES.to_owned(),
            pack: None,
            fresh: false,
            asking: false,
            wanted: false,
            problem: None,
            states: BTreeMap::new(),
            queue: VecDeque::new(),
            job: None,
            looked: false,
            pictures: HashMap::new(),
            picture_threads: 0,
            tx,
            rx,
        }
    }
}

impl Wardrobe {
    /// Answers from `http` at `address` in place of the real wardrobe.
    #[cfg(test)]
    pub fn serve(&mut self, http: Arc<dyn Http>, address: &str) {
        self.http = Some(http);
        self.address = address.to_owned();
        self.allowed = address.to_owned();
    }

    /// A client whose requests give up after `timeout`.
    fn client(&self, timeout: Duration) -> Arc<dyn Http> {
        match &self.http {
            Some(http) => Arc::clone(http),
            None => Arc::new(Client::new(timeout)),
        }
    }

    /// Whether the list is being asked for.
    pub fn asking(&self) -> bool {
        self.asking || self.wanted
    }

    pub fn state(&self, id: &str) -> State {
        self.states.get(id).copied().unwrap_or(State::Available)
    }

    /// Whether `id` waits for its download (not the one under way).
    pub fn waiting(&self, id: &str) -> bool {
        self.queue.iter().any(|queued| queued == id)
    }

    /// Whether anything is being fetched or waits to be.
    #[cfg(test)]
    pub fn busy(&self) -> bool {
        self.asking
            || self.wanted
            || self.job.is_some()
            || !self.queue.is_empty()
            || self.picture_threads > 0
    }

    /// The picture of item `id`, once fetched and read.
    pub fn picture(&self, id: &str) -> Option<&egui::TextureHandle> {
        self.pictures.get(id)?.texture.as_ref()
    }

    /// The pack's items made for base model `base`.
    pub fn items_for<'a>(&'a self, base: &'a str) -> impl Iterator<Item = &'a PackItem> {
        self.pack
            .iter()
            .flat_map(|pack| &pack.items)
            .filter(move |item| item.bases.iter().any(|b| b == base))
    }

    /// The items a "Download all" for `base` would get, and their size.
    pub fn missing(&self, base: &str) -> (usize, u64) {
        let wanted = |item: &&PackItem| {
            matches!(self.state(&item.id), State::Available | State::Outdated)
                && !self.waiting(&item.id)
                && self.job.as_ref().is_none_or(|job| job.id != item.id)
        };
        let items: Vec<&PackItem> = self.items_for(base).filter(wanted).collect();
        (items.len(), items.iter().map(|item| item.bytes()).sum())
    }
}

/// An ID as a file name: `generated/biker-jacket` is `generated-biker-jacket`.
fn file_name(id: &str) -> String {
    id.replace('/', "-")
}

/// The character library's folder in the studio's folder.
fn library_root(folder: &Path) -> PathBuf {
    folder.join(LIBRARY_FOLDER)
}

/// Where an item of `library` (in `root`) stands against the index.
pub fn state_of(root: &Path, library: &Library, item: &PackItem) -> State {
    match library.manifest_path(&item.id) {
        None => State::Available,
        Some(path) if path != item.catalogue.manifest => State::Taken,
        Some(_) => {
            let folder = root.join(item.folder());
            let whole = item.files.iter().all(|file| {
                fs::metadata(folder.join(file.name())).is_ok_and(|meta| meta.len() == file.bytes)
            });
            if whole {
                State::Installed
            } else {
                State::Outdated
            }
        }
    }
}

/// Reads the library's catalogue.
fn read_catalogue(root: &Path) -> Result<Catalogue, String> {
    let text = fs::read_to_string(root.join("catalogue.json"))
        .map_err(|err| format!("the character library's catalogue can't be read: {err}"))?;
    Catalogue::from_json(&text)
        .map_err(|err| format!("the character library's catalogue can't be read: {err}"))
}

/// Writes the library's catalogue, whole or not at all (the workspace
/// reads it again whenever it changes).
fn write_catalogue(root: &Path, catalogue: &Catalogue) -> Result<(), String> {
    let file = root.join("catalogue.json");
    let part = root.join("catalogue.json.part");
    fs::write(&part, catalogue.to_json())
        .and_then(|()| fs::rename(&part, &file))
        .map_err(|err| format!("the character library's catalogue can't be written: {err}"))
}

/// Puts the item downloaded and checked in `staging` into the library in
/// `root`: its folder (replacing an older copy of the same item) and its
/// catalogue entry, made from its manifest as the library's own are.
/// Refused, with the library untouched, when its manifest doesn't read or
/// is another item's, when the library hasn't the body it is made for,
/// when the library has that ID at another path (the engine's own item or
/// a person's), or when its folder holds anything else the library lists.
pub fn install(root: &Path, item: &PackItem, staging: &Path) -> Result<(), String> {
    let _writing = WRITING_LIBRARY
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let mut catalogue = read_catalogue(root)?;
    let text = fs::read_to_string(staging.join(item.manifest_name()))
        .map_err(|err| format!("its manifest can't be read: {err}"))?;
    let manifest = ItemManifest::from_json(&text)
        .map_err(|err| format!("its manifest doesn't read: {err}"))?;
    if manifest.id != item.id {
        return Err(format!(
            "its manifest is `{}`, not the item the wardrobe lists",
            manifest.id
        ));
    }
    let has_base = |id: &String| catalogue.bases.iter().any(|base| base.id == *id);
    if let Some(missing) = manifest.bases.iter().find(|base| !has_base(base)) {
        return Err(format!(
            "it is made for `{missing}`, which this library doesn't have"
        ));
    }
    let path = &item.catalogue.manifest;
    if catalogue
        .items
        .iter()
        .any(|entry| entry.id == item.id && entry.manifest != *path)
        || catalogue.bases.iter().any(|base| base.id == item.id)
    {
        return Err("the library already has something of its own with that ID".to_owned());
    }
    let folder = item.folder();
    let inside = |other: &str| {
        other
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
    };
    if catalogue.bases.iter().any(|base| inside(&base.manifest))
        || catalogue.presets.iter().any(|preset| inside(&preset.file))
        || catalogue
            .items
            .iter()
            .any(|entry| entry.id != item.id && inside(&entry.manifest))
    {
        return Err("its folder holds something else of the library's".to_owned());
    }
    let target = root.join(folder);
    let moved = (|| {
        if target.exists() {
            fs::remove_dir_all(&target)?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(staging, &target)
    })();
    moved.map_err(|err| format!("its files can't be put in the library: {err}"))?;
    catalogue.items.retain(|entry| entry.id != item.id);
    catalogue.items.push(CatalogueItem::of(path, &manifest));
    catalogue.items.sort_by(|a, b| a.id.cmp(&b.id));
    write_catalogue(root, &catalogue)
}

/// Takes a downloaded item out of the library in `root`: its folder and
/// its catalogue entry. Only an item that is where the pack puts it.
pub fn remove(root: &Path, item: &PackItem) -> Result<(), String> {
    let _writing = WRITING_LIBRARY
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let mut catalogue = read_catalogue(root)?;
    let listed =
        |entry: &CatalogueItem| entry.id == item.id && entry.manifest == item.catalogue.manifest;
    if !catalogue.items.iter().any(listed) {
        return Err("it isn't in the library".to_owned());
    }
    catalogue.items.retain(|entry| !listed(entry));
    // The entry first: a library never lists a garment whose files are gone.
    write_catalogue(root, &catalogue)?;
    let folder = root.join(item.folder());
    if folder.exists() {
        fs::remove_dir_all(&folder).map_err(|err| format!("its files can't be removed: {err}"))?;
    }
    Ok(())
}

/// One download, start to finish: the files into the staging folder, then
/// the item into the library.
fn download(
    http: &dyn Http,
    pack: &Pack,
    item: &PackItem,
    folder: &Path,
    progress: &Progress,
) -> Result<(), Failure> {
    let staging = folder
        .join(FOLDER)
        .join("staging")
        .join(file_name(&item.id));
    character_pack::fetch_item(http, pack, item, &staging, progress).map_err(
        |error| match error {
            SourceError::Cancelled => Failure::Cancelled,
            other => Failure::Failed(other.to_string()),
        },
    )?;
    // Only the item's own files go into the library: a file an older
    // version of the pack left in the staging folder doesn't.
    if let Ok(entries) = fs::read_dir(&staging) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !item.files.iter().any(|file| *file.name() == *name) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    install(&library_root(folder), item, &staging).map_err(Failure::Failed)
}

/// A picture's file as a texture, at most [`PICTURE_SIZE`] on a side.
fn read_picture(ctx: &egui::Context, id: &str, file: &Path) -> Option<egui::TextureHandle> {
    let bytes = fs::read(file).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;
    let image = image.thumbnail(PICTURE_SIZE, PICTURE_SIZE).to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Some(ctx.load_texture(
        format!("wardrobe picture {id}"),
        egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        egui::TextureOptions::LINEAR,
    ))
}

impl CharacterWorkspace {
    /// Does what the wardrobe section asked. Threads are started by
    /// [`CharacterWorkspace::wardrobe_step`], each frame.
    pub fn ask_wardrobe(&mut self, ask: Ask) {
        match ask {
            Ask::List => {
                self.wardrobe.wanted = true;
                self.wardrobe.problem = None;
            }
            Ask::Get(id) => self.queue_download(&id),
            Ask::GetAll => {
                let base = self.description.base.clone();
                let ids: Vec<String> = self
                    .wardrobe
                    .items_for(&base)
                    .map(|item| item.id.clone())
                    .collect();
                for id in ids {
                    self.queue_download(&id);
                }
            }
            Ask::Cancel => {
                self.wardrobe.queue.clear();
                if let Some(job) = &self.wardrobe.job {
                    job.progress.cancel.store(true, Ordering::Relaxed);
                }
            }
            Ask::Remove(id) => self.remove_download(&id),
        }
    }

    /// Puts `id` in line for its download, when the wardrobe lists it and
    /// it isn't in the library as listed, in line or under way already.
    fn queue_download(&mut self, id: &str) {
        let wardrobe = &mut self.wardrobe;
        let listed = wardrobe
            .pack
            .as_ref()
            .is_some_and(|pack| pack.item(id).is_some());
        let wanted = matches!(wardrobe.state(id), State::Available | State::Outdated);
        let under_way = wardrobe.job.as_ref().is_some_and(|job| job.id == id);
        if listed && wanted && !under_way && !wardrobe.waiting(id) {
            wardrobe.queue.push_back(id.to_owned());
        }
    }

    /// Takes the downloaded item `id` out of the library. Not while the
    /// character wears it: what is shown would lose it.
    fn remove_download(&mut self, id: &str) {
        let Some(item) = self
            .wardrobe
            .pack
            .as_ref()
            .and_then(|pack| pack.item(id).cloned())
        else {
            return;
        };
        if self.description.items.iter().any(|worn| worn.id == id) {
            self.notice = Some(format!(
                "{} is worn. Take it off before removing it from the library.",
                item.label
            ));
            return;
        }
        // A tile of it tried on goes with it.
        self.end_preview();
        self.notice = Some(match remove(&library_root(&self.folder), &item) {
            Ok(()) => format!(
                "{} is out of the library. It can be downloaded again.",
                item.label
            ),
            Err(error) => format!("Could not remove {}: {error}.", item.label),
        });
        self.update_wardrobe_states();
    }

    /// Works out again where each listed item stands in the library.
    fn update_wardrobe_states(&mut self) {
        let Some(pack) = self.wardrobe.pack.clone() else {
            self.wardrobe.states.clear();
            return;
        };
        let library = self.library();
        let root = library_root(&self.folder);
        self.wardrobe.states = pack
            .items
            .iter()
            .map(|item| (item.id.clone(), state_of(&root, &library, item)))
            .collect();
    }

    /// The wardrobe's frame: takes what its threads finished, starts what
    /// was asked for (the list, the next download, missing pictures) and
    /// reads fetched pictures as textures. With `ctx`, a thread asks for a
    /// repaint when it is done. Called every frame, on any page: a
    /// download goes on while another page is open.
    pub(super) fn wardrobe_step(&mut self, ctx: Option<&egui::Context>) {
        let on_page = self.selection.category == Category::Clothing;
        if on_page && !self.wardrobe.looked {
            self.wardrobe.looked = true;
            self.read_kept_wardrobe();
        }
        while let Ok(message) = self.wardrobe.rx.try_recv() {
            self.take_wardrobe_message(message);
        }
        if self.wardrobe.wanted && !self.wardrobe.asking {
            self.wardrobe.wanted = false;
            self.wardrobe.asking = true;
            let http = self.wardrobe.client(Duration::from_secs(30));
            let (address, allowed) = (self.wardrobe.address.clone(), self.wardrobe.allowed.clone());
            let started = self.wardrobe_thread("wardrobe list", ctx, move |tx| {
                let listed = character_pack::fetch_index(&*http, &address, &allowed)
                    .map(Box::new)
                    .map_err(|error| error.to_string());
                let _ = tx.send(Message::Listed(listed));
            });
            if !started {
                self.wardrobe.asking = false;
                self.wardrobe.problem = Some("Could not ask the wardrobe.".to_owned());
            }
        }
        if self.wardrobe.job.is_none() {
            self.start_next_download(ctx);
        }
        if on_page {
            self.fetch_wardrobe_pictures(ctx);
            if let Some(ctx) = ctx {
                self.read_wardrobe_pictures(ctx);
            }
        }
    }

    /// Runs `work` on a thread of its own, with the channel back and a
    /// repaint when it ends. `false` when the thread couldn't be started.
    fn wardrobe_thread(
        &self,
        name: &str,
        ctx: Option<&egui::Context>,
        work: impl FnOnce(&Sender<Message>) + Send + 'static,
    ) -> bool {
        let tx = self.wardrobe.tx.clone();
        let ctx = ctx.cloned();
        let spawned = std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                work(&tx);
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
        if let Err(error) = &spawned {
            log::warn!("Could not start the {name} thread: {error}");
        }
        spawned.is_ok()
    }

    /// Shows the list kept from the last time it was asked for, if any.
    fn read_kept_wardrobe(&mut self) {
        if self.wardrobe.pack.is_some() {
            return;
        }
        let kept = self.folder.join(FOLDER).join(character_pack::INDEX);
        let Ok(text) = fs::read_to_string(kept) else {
            return;
        };
        if let Ok(pack) = Pack::parse(&text, &self.wardrobe.allowed) {
            self.wardrobe.pack = Some(Arc::new(pack));
            self.wardrobe.fresh = false;
            self.update_wardrobe_states();
        }
    }

    fn take_wardrobe_message(&mut self, message: Message) {
        match message {
            Message::Listed(Ok(listed)) => {
                let (pack, text) = *listed;
                self.wardrobe.asking = false;
                self.wardrobe.problem = None;
                let folder = self.folder.join(FOLDER);
                let kept = fs::create_dir_all(&folder)
                    .and_then(|()| fs::write(folder.join(character_pack::INDEX), text));
                if let Err(error) = kept {
                    log::warn!("Could not keep the wardrobe's list: {error}");
                }
                self.wardrobe.pack = Some(Arc::new(pack));
                self.wardrobe.fresh = true;
                // A picture that failed is asked for again with the list.
                self.wardrobe.pictures.clear();
                self.update_wardrobe_states();
            }
            Message::Listed(Err(error)) => {
                self.wardrobe.asking = false;
                self.wardrobe.problem = Some(format!("Could not ask the wardrobe: {error}."));
            }
            Message::Picture { id, got } => {
                if let Some(picture) = self.wardrobe.pictures.get_mut(&id) {
                    picture.have = got;
                    picture.failed = !got;
                }
            }
            Message::PicturesDone => {
                self.wardrobe.picture_threads = self.wardrobe.picture_threads.saturating_sub(1);
            }
            Message::Installed { id, result } => self.downloaded(&id, result),
        }
    }

    /// A download ended: says so, and shows what came.
    fn downloaded(&mut self, id: &str, result: Result<(), Failure>) {
        let Some(job) = self.wardrobe.job.take() else {
            return;
        };
        match result {
            Ok(()) => {
                self.update_wardrobe_states();
                let draft = self.library().item(id).is_some_and(|item| item.is_draft());
                let mut notice = format!("{} is in your library.", job.label);
                if draft && !self.browser.show_drafts {
                    // What was asked for is shown: it would be hidden.
                    self.browser.show_drafts = true;
                    notice.push_str(" It is a draft, so Show drafts is on.");
                }
                // Its files are read again: an older copy of its model, or
                // the failure to find it, may be remembered.
                let item = self.wardrobe.pack.as_ref().and_then(|pack| pack.item(id));
                for file in item.iter().flat_map(|item| &item.files) {
                    let folder = item.map_or("", |item| item.folder());
                    self.assets
                        .forget(&format!("{LIBRARY_FOLDER}/{folder}/{}", file.name()));
                }
                self.assets.forget_failures();
                // A character that named it already now wears it. (After
                // the apply, whose own notice would hide this one.)
                self.applied = None;
                self.refresh();
                self.notice = Some(notice);
            }
            Err(Failure::Cancelled) => {
                self.notice = Some(format!(
                    "The download of {} was cancelled. What came is kept, so it goes on from \
                     there next time.",
                    job.label
                ));
            }
            Err(Failure::Failed(error)) => {
                // The ones waiting would likely fail the same way.
                self.wardrobe.queue.clear();
                self.notice = Some(format!("Could not download {}: {error}.", job.label));
            }
        }
    }

    /// Starts the download of the next item in line.
    fn start_next_download(&mut self, ctx: Option<&egui::Context>) {
        let Some(pack) = self.wardrobe.pack.clone() else {
            self.wardrobe.queue.clear();
            return;
        };
        let item = loop {
            let Some(id) = self.wardrobe.queue.pop_front() else {
                return;
            };
            if let Some(item) = pack.item(&id) {
                break item.clone();
            }
        };
        let progress = Arc::new(Progress::default());
        self.wardrobe.job = Some(Job {
            id: item.id.clone(),
            label: item.label.clone(),
            bytes: item.bytes(),
            progress: Arc::clone(&progress),
        });
        // A garment is tens of megabytes: a slow line gets its time.
        let http = self.wardrobe.client(Duration::from_secs(30 * 60));
        let folder = self.folder.clone();
        let id = item.id.clone();
        let started = self.wardrobe_thread("wardrobe download", ctx, move |tx| {
            let result = download(&*http, &pack, &item, &folder, &progress);
            let _ = tx.send(Message::Installed {
                id: item.id.clone(),
                result,
            });
        });
        if !started {
            let failed = Failure::Failed("the download couldn't be started".to_owned());
            self.downloaded(&id, Err(failed));
        }
    }

    /// Asks for the pictures the page shows and hasn't, a few at a time.
    /// Only for a list asked for in this session: the kept copy fetches
    /// nothing.
    fn fetch_wardrobe_pictures(&mut self, ctx: Option<&egui::Context>) {
        let Some(pack) = self.wardrobe.pack.clone() else {
            return;
        };
        let folder = self
            .folder
            .join(FOLDER)
            .join("previews")
            .join(file_name(&pack.version));
        let base = self.description.base.clone();
        let ids: Vec<String> = self
            .wardrobe
            .items_for(&base)
            .filter(|item| item.preview.is_some())
            .map(|item| item.id.clone())
            .collect();
        for id in ids {
            self.wardrobe.pictures.entry(id.clone()).or_insert_with(|| {
                let file = folder.join(format!("{}.png", file_name(&id)));
                Picture {
                    have: file.is_file(),
                    file,
                    failed: false,
                    texture: None,
                }
            });
        }
        if !self.wardrobe.fresh || self.wardrobe.picture_threads > 0 {
            return;
        }
        let missing: VecDeque<(PackItem, PathBuf)> = self
            .wardrobe
            .pictures
            .iter()
            .filter(|(_, picture)| !picture.have && !picture.failed)
            .filter_map(|(id, picture)| Some((pack.item(id)?.clone(), picture.file.clone())))
            .collect();
        if missing.is_empty() {
            return;
        }
        let threads = PICTURE_THREADS.min(missing.len());
        let missing = Arc::new(Mutex::new(missing));
        let http = self.wardrobe.client(Duration::from_secs(30));
        for _ in 0..threads {
            let (http, pack, missing) =
                (Arc::clone(&http), Arc::clone(&pack), Arc::clone(&missing));
            let repaint = ctx.cloned();
            let started = self.wardrobe_thread("wardrobe pictures", ctx, move |tx| {
                loop {
                    let next = missing
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .pop_front();
                    let Some((item, file)) = next else {
                        break;
                    };
                    let got = character_pack::fetch_preview(&*http, &pack, &item, &file)
                        .inspect_err(|error| log::warn!("No picture of {}: {error}", item.id))
                        .is_ok();
                    let _ = tx.send(Message::Picture { id: item.id, got });
                    if let Some(ctx) = &repaint {
                        ctx.request_repaint();
                    }
                }
                let _ = tx.send(Message::PicturesDone);
            });
            self.wardrobe.picture_threads += usize::from(started);
        }
    }

    /// Reads a few fetched pictures as textures.
    fn read_wardrobe_pictures(&mut self, ctx: &egui::Context) {
        let mut read = 0;
        for (id, picture) in &mut self.wardrobe.pictures {
            if !picture.have || picture.failed || picture.texture.is_some() {
                continue;
            }
            if read == PICTURES_PER_FRAME {
                ctx.request_repaint();
                break;
            }
            read += 1;
            picture.texture = read_picture(ctx, id, &picture.file);
            // A file that doesn't read isn't read every frame.
            picture.failed = picture.texture.is_none();
        }
    }

    /// Steps the wardrobe until its threads are done (tests have no
    /// frames).
    #[cfg(test)]
    pub fn finish_wardrobe(&mut self) {
        let started = std::time::Instant::now();
        loop {
            self.wardrobe_step(None);
            if !self.wardrobe.busy() {
                return;
            }
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "the wardrobe's threads didn't finish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
