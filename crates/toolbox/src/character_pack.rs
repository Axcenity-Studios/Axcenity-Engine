//! Character packs: clothing and other character items published as a
//! repository with a `pack.json` index (format `axcenity-character-pack`),
//! such as the Axcenity Wardrobe. Character Studio lists a pack and
//! downloads its items into a character library (`docs/editor/
//! character-studio.md`, the online wardrobe); this is the part that reads
//! the index and fetches files, with no knowledge of the library.
//!
//! The index and every file come from the internet, so nothing in it is
//! trusted: files are fetched only from under the address the caller
//! allows, each path is checked before it becomes a file name, an item is
//! kept only when all its files sit in its own folder and are of a kind an
//! item is made of, and every file is checked against the size and SHA-256
//! the index lists before it is put in place.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::http::{self, Http};
use crate::{SourceError, guard};

/// The `format` of a pack's index.
pub const FORMAT: &str = "axcenity-character-pack";
/// The newest index version this reads.
pub const FORMAT_VERSION: u32 = 1;
/// The index's file name, at the top of a pack.
pub const INDEX: &str = "pack.json";

/// Where the Axcenity Wardrobe is read from: its repository's files.
pub const WARDROBE: &str =
    "https://raw.githubusercontent.com/Axcenity-Studios/Axcenity-Wardrobe/main/";
/// What every address of the Axcenity Wardrobe starts with (a later
/// version may name a tag in place of `main`).
pub const WARDROBE_FILES: &str =
    "https://raw.githubusercontent.com/Axcenity-Studios/Axcenity-Wardrobe/";

/// The most an index may be.
pub const INDEX_LIMIT: u64 = 8 * 1024 * 1024;
/// The most an item's picture may be.
pub const PREVIEW_LIMIT: u64 = 8 * 1024 * 1024;
/// The most one file of an item may be (a garment's model carries every
/// body shape: tens of megabytes).
pub const FILE_LIMIT: u64 = 512 * 1024 * 1024;

/// The folder of a pack that holds its items, at their library paths.
const LIBRARY: &str = "library/";
/// What an item is made of: its model, its manifest and fit data, and its
/// thumbnail.
const EXTENSIONS: [&str; 3] = ["glb", "json", "png"];

/// A pack's index.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Pack {
    pub format: String,
    pub format_version: u32,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    /// An SPDX code (`CC-BY-4.0`).
    #[serde(default)]
    pub licence: String,
    /// The line to give credit with.
    #[serde(default)]
    pub credit: String,
    #[serde(default)]
    pub repository: String,
    /// What a file's address starts with: `files_at` then its `path`.
    pub files_at: String,
    #[serde(default)]
    pub items: Vec<PackItem>,
    /// How many items [`Pack::parse`] left out because they broke a rule
    /// (a newer pack may hold kinds of file this editor doesn't take).
    #[serde(skip)]
    pub refused: usize,
}

/// One item of a pack.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PackItem {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub slot: String,
    /// `draft` or `production`; none is a draft, as in an item manifest.
    #[serde(default)]
    pub quality: Option<String>,
    /// The base models it is made for.
    #[serde(default)]
    pub bases: Vec<String>,
    /// Its picture in the pack (`previews/biker-jacket.png`).
    #[serde(default)]
    pub preview: Option<String>,
    /// Its entry in a library's catalogue: where its manifest goes.
    pub catalogue: PackEntry,
    pub files: Vec<PackFile>,
}

/// The part of an item's catalogue entry a download needs.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PackEntry {
    /// The manifest's path in a library (`human/imports/x/X.item.json`).
    pub manifest: String,
    /// The item's tags (who a garment is cut for: `masculine`,
    /// `feminine`, `unisex`).
    #[serde(default)]
    pub tags: Vec<String>,
}

/// One file of an item.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PackFile {
    /// Its path in the pack (`library/human/imports/x/X.glb`).
    pub path: String,
    pub bytes: u64,
    /// Lowercase hex.
    pub sha256: String,
}

impl PackFile {
    /// Its file name.
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

impl PackItem {
    /// Its folder in a library (`human/imports/biker-jacket`): where
    /// every one of its files goes, by name.
    pub fn folder(&self) -> &str {
        self.catalogue
            .manifest
            .rsplit_once('/')
            .map_or("", |(folder, _)| folder)
    }

    /// Its manifest's file name.
    pub fn manifest_name(&self) -> &str {
        let manifest = &self.catalogue.manifest;
        manifest.rsplit('/').next().unwrap_or(manifest)
    }

    /// Its files' sizes together.
    pub fn bytes(&self) -> u64 {
        self.files.iter().map(|file| file.bytes).sum()
    }

    /// Whether it hasn't passed the quality checks (no `quality`, or
    /// `draft`).
    pub fn is_draft(&self) -> bool {
        self.quality.as_deref().is_none_or(|q| q == "draft")
    }

    /// Why this item can't be taken, if it can't: an ID or a path that
    /// isn't plain, a file outside its own folder or of another kind, no
    /// manifest among its files, a size or checksum that can't be one.
    fn problem(&self) -> Option<&'static str> {
        if !plain_path(&self.id) {
            return Some("an ID that isn't a plain name");
        }
        let manifest = &self.catalogue.manifest;
        if !plain_path(manifest) || !manifest.ends_with(".item.json") {
            return Some("a manifest path that isn't one");
        }
        // Two levels at least: an item never sits at the top of a
        // library or of a package, where the engine's own files are.
        let folder = self.folder();
        if folder.split('/').count() < 2 {
            return Some("an item outside a folder of its own");
        }
        if self.files.is_empty() {
            return Some("an item with no files");
        }
        let mut names: Vec<&str> = Vec::new();
        for file in &self.files {
            let inside = file
                .path
                .strip_prefix(LIBRARY)
                .and_then(|path| path.strip_prefix(folder))
                .and_then(|rest| rest.strip_prefix('/'));
            let Some(name) = inside.filter(|name| !name.contains('/')) else {
                return Some("a file outside its item's folder");
            };
            if !plain_path(&file.path) {
                return Some("a file name that isn't plain");
            }
            let extension = name.rsplit_once('.').map(|(_, extension)| extension);
            if !extension.is_some_and(|e| EXTENSIONS.contains(&e)) {
                return Some("a kind of file an item isn't made of");
            }
            if names.contains(&name) {
                return Some("the same file twice");
            }
            names.push(name);
            if file.bytes == 0 || file.bytes > FILE_LIMIT {
                return Some("a file size that can't be right");
            }
            let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
            if file.sha256.len() != 64 || !file.sha256.chars().all(hex) {
                return Some("a checksum that isn't SHA-256");
            }
        }
        if !names.contains(&self.manifest_name()) {
            return Some("no manifest among its files");
        }
        if let Some(preview) = &self.preview
            && (!plain_path(preview) || !preview.ends_with(".png"))
        {
            return Some("a picture that isn't a PNG in the pack");
        }
        None
    }
}

/// Whether `path` is a relative path of plain names: parts of ASCII
/// letters, digits, `-`, `_` and `.` between single `/`s, none of them `.`
/// or `..`. Such a path needs no escaping in an address and can't leave
/// the folder it is joined to.
fn plain_path(path: &str) -> bool {
    guard::safe_relative_path(path).is_some_and(|safe| safe == path)
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
}

impl Pack {
    /// Reads an index. `allowed` is what every file's address must start
    /// with (the place the index itself came from): a pack that sends its
    /// files' downloads elsewhere is refused. Items that break a rule are
    /// left out and counted in [`Pack::refused`].
    pub fn parse(text: &str, allowed: &str) -> Result<Pack, SourceError> {
        // The format and version first, so a newer pack says so instead of
        // failing on a field that changed.
        #[derive(Deserialize)]
        struct Head {
            #[serde(default)]
            format: String,
            #[serde(default)]
            format_version: u32,
        }
        let head: Head = serde_json::from_str(text)
            .map_err(|_| SourceError::BadResponse("the pack's index isn't JSON"))?;
        if head.format != FORMAT {
            return Err(SourceError::BadResponse("that isn't a character pack"));
        }
        if head.format_version > FORMAT_VERSION {
            return Err(SourceError::BadResponse(
                "the pack was made for a newer editor; update the editor to read it",
            ));
        }
        let mut pack: Pack = serde_json::from_str(text)
            .map_err(|_| SourceError::BadResponse("the pack's index is missing something"))?;
        if !pack.files_at.starts_with("https://") {
            return Err(SourceError::NotHttps);
        }
        if !pack.files_at.starts_with(allowed) {
            return Err(SourceError::Refused(
                "the pack's files aren't where the pack is",
            ));
        }
        if !pack.files_at.ends_with('/') {
            pack.files_at.push('/');
        }
        let listed = pack.items.len();
        pack.items.retain(|item| match item.problem() {
            None => true,
            Some(problem) => {
                log::warn!("Character pack item {:?} left out: {problem}", item.id);
                false
            }
        });
        // An ID or a folder named twice would have one item write over
        // another: the first is kept.
        let mut seen: Vec<(String, String)> = Vec::new();
        pack.items.retain(|item| {
            let twice = seen
                .iter()
                .any(|(id, folder)| *id == item.id || folder == item.folder());
            if !twice {
                seen.push((item.id.clone(), item.folder().to_owned()));
            }
            !twice
        });
        pack.refused = listed - pack.items.len();
        Ok(pack)
    }

    /// The address of `path` in the pack.
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.files_at)
    }

    pub fn item(&self, id: &str) -> Option<&PackItem> {
        self.items.iter().find(|item| item.id == id)
    }
}

/// Fetches the index of the pack at `address` (which ends with `/`) and
/// reads it. Returns the index's text too, to keep a copy of.
pub fn fetch_index(
    http: &dyn Http,
    address: &str,
    allowed: &str,
) -> Result<(Pack, String), SourceError> {
    let text = http::get_text(http, &format!("{address}{INDEX}"), INDEX_LIMIT)?;
    let pack = Pack::parse(&text, allowed)?;
    Ok((pack, text))
}

/// How a download is going, shared with the thread that shows it.
#[derive(Debug, Default)]
pub struct Progress {
    /// The bytes of the item in hand so far.
    pub done: AtomicU64,
    /// Set to stop the download at the next block.
    pub cancel: AtomicBool,
}

/// A file's SHA-256 as lowercase hex.
pub fn sha256_of(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Whether `path` is the file the index lists: its size, then its SHA-256.
pub fn is_listed(path: &Path, file: &PackFile) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() == file.bytes)
        && sha256_of(path).is_ok_and(|sum| sum == file.sha256)
}

/// Writes to a file while hashing and counting what goes by, and stops
/// when the download is cancelled.
struct Checked<'a> {
    file: fs::File,
    hasher: Sha256,
    written: u64,
    progress: &'a Progress,
}

impl Write for Checked<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.progress.cancel.load(Ordering::Relaxed) {
            return Err(io::Error::other("cancelled"));
        }
        let written = self.file.write(buffer)?;
        self.hasher.update(&buffer[..written]);
        self.written += written as u64;
        self.progress
            .done
            .fetch_add(written as u64, Ordering::Relaxed);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// Downloads every file of `item` into the folder `staging`, by file
/// name, each through a `.part` file checked against the size and SHA-256
/// the index lists. A file already there and right isn't fetched again, so
/// a download that stopped part way goes on from the files it finished.
/// When this returns `Ok`, `staging` holds the whole item, checked.
pub fn fetch_item(
    http: &dyn Http,
    pack: &Pack,
    item: &PackItem,
    staging: &Path,
    progress: &Progress,
) -> Result<(), SourceError> {
    fs::create_dir_all(staging)?;
    for file in &item.files {
        if progress.cancel.load(Ordering::Relaxed) {
            return Err(SourceError::Cancelled);
        }
        let dest = staging.join(file.name());
        if is_listed(&dest, file) {
            progress.done.fetch_add(file.bytes, Ordering::Relaxed);
            continue;
        }
        let part = staging.join(format!("{}.part", file.name()));
        let before = progress.done.load(Ordering::Relaxed);
        let fetched = (|| {
            let mut out = Checked {
                file: fs::File::create(&part)?,
                hasher: Sha256::new(),
                written: 0,
                progress,
            };
            http.fetch(&pack.url(&file.path), file.bytes, &mut out)?;
            out.file.flush()?;
            if out.written != file.bytes || hex(&out.hasher.finalize()) != file.sha256 {
                return Err(SourceError::BadResponse(
                    "a file didn't match the size or checksum the pack lists",
                ));
            }
            drop(out.file);
            fs::rename(&part, &dest)?;
            Ok(())
        })();
        if let Err(error) = fetched {
            let _ = fs::remove_file(&part);
            // What was counted of this file isn't in hand after all.
            progress.done.store(before, Ordering::Relaxed);
            return Err(if progress.cancel.load(Ordering::Relaxed) {
                SourceError::Cancelled
            } else {
                error
            });
        }
    }
    Ok(())
}

/// Downloads `item`'s picture to `dest` (a PNG, through a `.part` file).
pub fn fetch_preview(
    http: &dyn Http,
    pack: &Pack,
    item: &PackItem,
    dest: &Path,
) -> Result<(), SourceError> {
    let preview = item.preview.as_deref().ok_or(SourceError::NotFound)?;
    let bytes = http::get_bytes(http, &pack.url(preview), PREVIEW_LIMIT)?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(SourceError::BadResponse("the picture isn't a PNG"));
    }
    if let Some(folder) = dest.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut name = dest.file_name().unwrap_or_default().to_owned();
    name.push(".part");
    let part = dest.with_file_name(name);
    fs::write(&part, bytes)?;
    fs::rename(&part, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::http::Fixture;

    const AT: &str = "https://example.org/wardrobe/";

    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "axcenity-character-pack-{name}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sum(bytes: &[u8]) -> String {
        hex(&Sha256::digest(bytes))
    }

    /// One item's JSON, its files `(path, body)`.
    fn item_json(id: &str, manifest: &str, files: &[(&str, &[u8])]) -> String {
        let files: Vec<String> = files
            .iter()
            .map(|(path, body)| {
                format!(
                    r#"{{ "path": "{path}", "bytes": {}, "sha256": "{}" }}"#,
                    body.len(),
                    sum(body)
                )
            })
            .collect();
        format!(
            r#"{{ "id": "{id}", "label": "Jacket", "slot": "outerwear", "quality": "draft",
                 "bases": ["axcenity/human"], "preview": "previews/jacket.png",
                 "catalogue": {{ "manifest": "{manifest}" }}, "files": [{}] }}"#,
            files.join(", ")
        )
    }

    fn pack_json(files_at: &str, items: &[String]) -> String {
        format!(
            r#"{{ "format": "axcenity-character-pack", "format_version": 1,
                 "name": "Test Wardrobe", "version": "0.1.0", "licence": "CC-BY-4.0",
                 "credit": "Test Wardrobe by Someone", "files_at": "{files_at}",
                 "items": [{}] }}"#,
            items.join(", ")
        )
    }

    const MANIFEST: &str = "human/imports/jacket/Jacket.item.json";

    fn jacket_files() -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("library/human/imports/jacket/Jacket.glb", b"model"),
            ("library/human/imports/jacket/Jacket.item.json", b"manifest"),
        ]
    }

    fn jacket_pack() -> String {
        pack_json(AT, &[item_json("test/jacket", MANIFEST, &jacket_files())])
    }

    fn served() -> Fixture {
        let mut http = Fixture::default().with(&format!("{AT}pack.json"), jacket_pack());
        for (path, body) in jacket_files() {
            http = http.with(&format!("{AT}{path}"), body);
        }
        http
    }

    #[test]
    fn an_index_is_read_with_its_items_folders_and_sizes() {
        let (pack, text) = fetch_index(&served(), AT, AT).unwrap();
        assert_eq!(text, jacket_pack());
        assert_eq!(pack.name, "Test Wardrobe");
        assert_eq!(pack.refused, 0);
        let item = pack.item("test/jacket").unwrap();
        assert_eq!(item.folder(), "human/imports/jacket");
        assert_eq!(item.manifest_name(), "Jacket.item.json");
        assert_eq!(item.bytes(), 13);
        assert!(item.is_draft());
        assert_eq!(
            pack.url(&item.files[0].path),
            "https://example.org/wardrobe/library/human/imports/jacket/Jacket.glb"
        );
    }

    #[test]
    fn the_real_wardrobe_address_is_allowed_for_itself() {
        assert!(WARDROBE.starts_with(WARDROBE_FILES));
        let text = pack_json(
            WARDROBE,
            &[item_json("test/jacket", MANIFEST, &jacket_files())],
        );
        assert_eq!(Pack::parse(&text, WARDROBE_FILES).unwrap().items.len(), 1);
    }

    #[test]
    fn a_pack_that_isnt_one_or_is_newer_is_refused() {
        assert!(matches!(
            Pack::parse("not json", AT),
            Err(SourceError::BadResponse(_))
        ));
        let other = jacket_pack().replace("axcenity-character-pack", "something-else");
        assert!(matches!(
            Pack::parse(&other, AT),
            Err(SourceError::BadResponse("that isn't a character pack"))
        ));
        let newer = jacket_pack().replace("\"format_version\": 1", "\"format_version\": 2");
        let error = Pack::parse(&newer, AT).unwrap_err();
        assert!(error.to_string().contains("newer editor"), "{error}");
    }

    #[test]
    fn files_are_only_taken_from_where_the_pack_is() {
        let elsewhere = pack_json("https://elsewhere.example/files/", &[]);
        assert!(matches!(
            Pack::parse(&elsewhere, AT),
            Err(SourceError::Refused(_))
        ));
        let plain = pack_json("http://example.org/wardrobe/", &[]);
        assert!(matches!(
            Pack::parse(&plain, AT),
            Err(SourceError::NotHttps)
        ));
    }

    #[test]
    fn items_that_break_a_rule_are_left_out_and_counted() {
        let good = item_json("test/jacket", MANIFEST, &jacket_files());
        let bad = [
            // A file that climbs out of the pack.
            item_json(
                "test/a",
                "human/imports/a/A.item.json",
                &[
                    ("library/human/imports/a/A.item.json", b"m"),
                    ("library/human/imports/a/../../Human.glb", b"x"),
                ],
            ),
            // A file in another item's folder.
            item_json(
                "test/b",
                "human/imports/b/B.item.json",
                &[
                    ("library/human/imports/b/B.item.json", b"m"),
                    ("library/human/imports/jacket/Jacket.glb", b"x"),
                ],
            ),
            // A file in a folder below its own.
            item_json(
                "test/c",
                "human/imports/c/C.item.json",
                &[
                    ("library/human/imports/c/C.item.json", b"m"),
                    ("library/human/imports/c/deep/C.glb", b"x"),
                ],
            ),
            // A kind of file an item isn't made of.
            item_json(
                "test/d",
                "human/imports/d/D.item.json",
                &[
                    ("library/human/imports/d/D.item.json", b"m"),
                    ("library/human/imports/d/run.sh", b"x"),
                ],
            ),
            // No manifest among its files.
            item_json(
                "test/e",
                "human/imports/e/E.item.json",
                &[("library/human/imports/e/E.glb", b"x")],
            ),
            // At the top of a package, where the body's own files are.
            item_json(
                "test/f",
                "Human.item.json",
                &[("library/Human.item.json", b"m")],
            ),
            // An ID that would be a path out of the library.
            item_json(
                "../f",
                "human/imports/g/G.item.json",
                &[("library/human/imports/g/G.item.json", b"m")],
            ),
            // The same ID as the good one.
            item_json(
                "test/jacket",
                "human/imports/h/H.item.json",
                &[("library/human/imports/h/H.item.json", b"m")],
            ),
            // The same folder as the good one.
            item_json(
                "test/i",
                "human/imports/jacket/I.item.json",
                &[("library/human/imports/jacket/I.item.json", b"m")],
            ),
            // An empty file, and a checksum that isn't one.
            item_json(
                "test/j",
                "human/imports/j/J.item.json",
                &[("library/human/imports/j/J.item.json", b"")],
            ),
            item_json(
                "test/k",
                "human/imports/k/K.item.json",
                &[("library/human/imports/k/K.item.json", b"m")],
            )
            .replace(&sum(b"m"), "nothex"),
        ];
        let mut items = vec![good];
        items.extend(bad.iter().cloned());
        let pack = Pack::parse(&pack_json(AT, &items), AT).unwrap();
        let ids: Vec<&str> = pack.items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["test/jacket"]);
        assert_eq!(pack.refused, bad.len());
        // The one kept is the first with its ID.
        assert_eq!(pack.items[0].folder(), "human/imports/jacket");
    }

    #[test]
    fn an_item_is_downloaded_checked_and_counted() {
        let temp = Temp::new("fetch");
        let http = served();
        let (pack, _) = fetch_index(&http, AT, AT).unwrap();
        let item = pack.item("test/jacket").unwrap();
        let progress = Progress::default();
        fetch_item(&http, &pack, item, &temp.0, &progress).unwrap();
        assert_eq!(fs::read(temp.0.join("Jacket.glb")).unwrap(), b"model");
        assert_eq!(
            fs::read(temp.0.join("Jacket.item.json")).unwrap(),
            b"manifest"
        );
        assert_eq!(progress.done.load(Ordering::Relaxed), item.bytes());
        // Nothing half-made is left.
        let parts = fs::read_dir(&temp.0)
            .unwrap()
            .filter(|entry| {
                let name = entry.as_ref().unwrap().file_name();
                name.to_string_lossy().ends_with(".part")
            })
            .count();
        assert_eq!(parts, 0);
        // Again with nothing served: every file is there already, so
        // nothing is asked for (an empty fixture answers 404).
        let again = Progress::default();
        fetch_item(&Fixture::default(), &pack, item, &temp.0, &again).unwrap();
        assert_eq!(again.done.load(Ordering::Relaxed), item.bytes());
    }

    #[test]
    fn a_file_that_isnt_the_one_listed_is_never_put_in_place() {
        let temp = Temp::new("wrong");
        // The model served isn't the one the index lists (same size).
        let http = served().with(
            &format!("{AT}library/human/imports/jacket/Jacket.glb"),
            "MODEL",
        );
        let (pack, _) = fetch_index(&http, AT, AT).unwrap();
        let item = pack.item("test/jacket").unwrap();
        let progress = Progress::default();
        let error = fetch_item(&http, &pack, item, &temp.0, &progress).unwrap_err();
        assert!(matches!(error, SourceError::BadResponse(_)), "{error}");
        assert!(!temp.0.join("Jacket.glb").exists());
        assert!(!temp.0.join("Jacket.glb.part").exists());
        assert_eq!(progress.done.load(Ordering::Relaxed), 0);
        // A longer answer than listed is cut off, not kept.
        let http = served().with(
            &format!("{AT}library/human/imports/jacket/Jacket.glb"),
            "model and more",
        );
        let error = fetch_item(&http, &pack, item, &temp.0, &Progress::default()).unwrap_err();
        assert!(matches!(error, SourceError::TooLarge), "{error}");
        assert!(!temp.0.join("Jacket.glb").exists());
    }

    #[test]
    fn a_cancelled_download_stops_and_goes_on_later_from_what_it_had() {
        let temp = Temp::new("cancel");
        let http = served();
        let (pack, _) = fetch_index(&http, AT, AT).unwrap();
        let item = pack.item("test/jacket").unwrap();
        let progress = Progress::default();
        progress.cancel.store(true, Ordering::Relaxed);
        let error = fetch_item(&http, &pack, item, &temp.0, &progress).unwrap_err();
        assert!(matches!(error, SourceError::Cancelled), "{error}");
        assert!(!temp.0.join("Jacket.glb").exists());
        // With the model in hand, only the manifest is asked for.
        fs::write(temp.0.join("Jacket.glb"), b"model").unwrap();
        let rest = Fixture::default().with(
            &format!("{AT}library/human/imports/jacket/Jacket.item.json"),
            "manifest",
        );
        fetch_item(&rest, &pack, item, &temp.0, &Progress::default()).unwrap();
        assert!(temp.0.join("Jacket.item.json").is_file());
    }

    #[test]
    fn a_picture_must_be_a_png() {
        let temp = Temp::new("preview");
        let png = b"\x89PNG\r\n\x1a\nrest".to_vec();
        let http = served().with(&format!("{AT}previews/jacket.png"), png.clone());
        let (pack, _) = fetch_index(&http, AT, AT).unwrap();
        let item = pack.item("test/jacket").unwrap();
        let dest = temp.0.join("previews").join("jacket.png");
        fetch_preview(&http, &pack, item, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), png);
        let http = served().with(&format!("{AT}previews/jacket.png"), "<html>");
        let other = temp.0.join("previews").join("other.png");
        assert!(matches!(
            fetch_preview(&http, &pack, item, &other),
            Err(SourceError::BadResponse(_))
        ));
        assert!(!other.exists());
    }
}
