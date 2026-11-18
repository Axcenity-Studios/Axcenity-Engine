//! Provenance of imported assets (Paul's decision 11, 2026-11-18): every
//! item made from a third-party asset names a **provenance record**
//! (`*.provenance.json`, beside its manifest) that says where the asset
//! came from, under which licence, what the files were when downloaded,
//! what was changed and the attribution line its licence asks for. The
//! checks here fail a shipped item that is imported without one, under a
//! licence Axcenity refuses, or under an attribution licence without its
//! line; [`character_credits`] gathers the lines a game must carry
//! (`docs/development/character-studio/architecture.md`, "Provenance").

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::manifest::{check_id, check_version, not_empty};
use crate::{FormatError, ItemManifest, Library, Origin, Problem, pretty, read_versioned};

/// The provenance record's format version.
pub const PROVENANCE_VERSION: u32 = 1;
/// A provenance record's file name ends with this.
pub const PROVENANCE_SUFFIX: &str = ".provenance.json";
/// The licence original Axcenity work (class 5) is released under.
pub const ORIGINAL_LICENCE: &str = "CC0-1.0";

/// One imported asset: where it came from and what was done to it. One
/// record may feed several items (a pair of shoes and its variants).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub format_version: u32,
    /// The asset's name at its source.
    pub asset: String,
    /// The page it was downloaded from (the asset's own page, where its
    /// licence is stated).
    pub source_page: String,
    /// Its author, as the source names them.
    pub author: String,
    pub licence: LicenceRecord,
    /// When it was downloaded (`YYYY-MM-DD`).
    pub retrieved: String,
    /// Every file as downloaded, before any change.
    pub files: Vec<SourceFile>,
    /// What was changed, one entry per step (an imported asset is always
    /// changed: fitted, rigged, re-materialed).
    pub changes: Vec<String>,
    /// The attribution line exactly as the licence asks for it (title,
    /// author, source, licence and that it was changed); required for an
    /// attribution licence, a courtesy otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    /// The items it feeds, by ID.
    pub items: Vec<String>,
    /// What the source's own page says about the licence, quoted, so a
    /// reviewer can see why it was accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// A licence by its SPDX ID and version (`CC-BY-4.0`) and the page of its
/// text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LicenceRecord {
    pub id: String,
    pub url: String,
}

/// A downloaded file: its original name and the SHA-256 of its bytes as
/// downloaded (64 lowercase hex digits).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceFile {
    pub name: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
}

/// What a licence Axcenity accepts asks of a game that ships the asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LicenceTerms {
    /// Public domain (CC0, a dedication): no credit required (class 1).
    PublicDomain,
    /// The attribution line must travel with the asset (CC BY 3.0 or 4.0,
    /// MIT, Apache-2.0: class 2).
    Attribution,
}

/// The licences Axcenity accepts for an imported asset, by SPDX ID
/// (`references/README.md`, classes 1 and 2). `LicenseRef-PublicDomain`
/// is a work the source states is in the public domain without a
/// licence. Kept in step with `assets/characters/realistic/import/
/// licences.json` by `tests/shipped_provenance.rs`.
pub const ACCEPTED_LICENCES: [(&str, LicenceTerms); 7] = [
    ("CC0-1.0", LicenceTerms::PublicDomain),
    ("CC-PDDC", LicenceTerms::PublicDomain),
    ("LicenseRef-PublicDomain", LicenceTerms::PublicDomain),
    ("CC-BY-3.0", LicenceTerms::Attribution),
    ("CC-BY-4.0", LicenceTerms::Attribution),
    ("MIT", LicenceTerms::Attribution),
    ("Apache-2.0", LicenceTerms::Attribution),
];

/// What a licence asks, or why Axcenity refuses it: share-alike,
/// non-commercial and no-derivatives licences can't ship in a game made
/// with Axcenity (the game ships under its own terms, and an imported
/// asset is always changed), and a custom or unknown licence can't be
/// checked.
pub fn licence_terms(id: &str) -> Result<LicenceTerms, String> {
    if let Some((_, terms)) = ACCEPTED_LICENCES.iter().find(|(known, _)| *known == id) {
        return Ok(*terms);
    }
    let upper = id.trim().to_ascii_uppercase();
    // SPDX IDs name a licence's elements between dashes (`CC-BY-NC-SA-4.0`).
    let has = |element: &str| upper.split('-').any(|part| part == element);
    let why = if upper.is_empty() || upper == "NOASSERTION" || upper == "NONE" {
        "is unknown: a licence is recorded only after reading it on the source's own page"
            .to_owned()
    } else if has("NC") {
        format!("`{id}` is non-commercial: a game made with Axcenity may be sold")
    } else if has("ND") {
        format!("`{id}` is no-derivatives: an imported asset is always changed (fitted, rigged)")
    } else if has("SA") || upper.contains("GPL") {
        format!("`{id}` is share-alike: a game ships its assets under its own terms")
    } else {
        let accepted: Vec<&str> = ACCEPTED_LICENCES.iter().map(|(id, _)| *id).collect();
        format!(
            "`{id}` isn't one Axcenity accepts ({}); a custom licence can't be checked",
            accepted.join(", ")
        )
    };
    Err(why)
}

/// Whether `text` is a web address.
fn is_url(text: &str) -> bool {
    (text.starts_with("https://") || text.starts_with("http://"))
        && !text.contains(char::is_whitespace)
}

/// Whether `text` is a date written `YYYY-MM-DD`.
fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let digits = |part: &&str| part.bytes().all(|b| b.is_ascii_digit());
    matches!(parts.as_slice(), [y, m, d] if y.len() == 4 && m.len() == 2 && d.len() == 2)
        && parts.iter().all(digits)
}

/// Whether `text` is a SHA-256 as the record writes it.
fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Provenance {
    pub fn from_json(text: &str) -> Result<Self, FormatError> {
        read_versioned(text, "provenance record", PROVENANCE_VERSION)
    }

    pub fn to_json(&self) -> String {
        pretty(self)
    }

    /// Checks the record on its own: every field says something, the
    /// licence is one Axcenity accepts and an attribution licence has its
    /// line.
    pub fn validate(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        let p = &mut problems;
        check_version(p, self.format_version, PROVENANCE_VERSION);
        not_empty(p, "asset", &self.asset);
        not_empty(p, "author", &self.author);
        for (at, url) in [
            ("source_page", &self.source_page),
            ("licence.url", &self.licence.url),
        ] {
            if !is_url(url) {
                p.push(Problem::new(at, format!("`{url}` isn't a web address")));
            }
        }
        let attributed = self
            .attribution
            .as_deref()
            .is_some_and(|line| !line.trim().is_empty());
        match licence_terms(&self.licence.id) {
            Err(why) => p.push(Problem::new("licence.id", why)),
            Ok(LicenceTerms::Attribution) if !attributed => p.push(Problem::new(
                "attribution",
                format!(
                    "`{}` asks for credit: the attribution line is required",
                    self.licence.id
                ),
            )),
            Ok(_) => {}
        }
        if !is_date(&self.retrieved) {
            p.push(Problem::new(
                "retrieved",
                format!("`{}` isn't a date (YYYY-MM-DD)", self.retrieved),
            ));
        }
        if self.files.is_empty() {
            p.push(Problem::new("files", "lists no file as downloaded"));
        }
        for (index, file) in self.files.iter().enumerate() {
            not_empty(p, &format!("files[{index}].name"), &file.name);
            if !is_sha256(&file.sha256) {
                p.push(Problem::new(
                    format!("files[{index}].sha256"),
                    "isn't a SHA-256 (64 lower-case hex digits)",
                ));
            }
        }
        if self.changes.iter().all(|change| change.trim().is_empty()) {
            p.push(Problem::new(
                "changes",
                "says nothing: an imported asset is always changed; say how",
            ));
        }
        if self.items.is_empty() {
            p.push(Problem::new("items", "feeds no item"));
        }
        for (index, id) in self.items.iter().enumerate() {
            check_id(p, &format!("items[{index}]"), id);
        }
        problems
    }
}

/// Whether an item says it is made from an imported asset: its source
/// says `origin: imported` or names a provenance record.
pub fn is_imported(item: &ItemManifest) -> bool {
    item.source.as_ref().is_some_and(|source| {
        source.origin == Some(Origin::Imported) || source.provenance.is_some()
    })
}

/// Checks an item's licence and provenance (`asset-quality.md` §2,
/// Licence), given its record when it names one that was found:
///
/// - every item says who made it and its licence (`source`);
/// - original work (class 5, the default) is released CC0-1.0, so an
///   item under any other licence was imported and needs its record;
/// - an imported item names its record, which is sound
///   ([`Provenance::validate`]: an accepted licence, the attribution
///   line an attribution licence asks for), names the item and has the
///   item's licence.
pub fn check_item_provenance(item: &ItemManifest, record: Option<&Provenance>) -> Vec<Problem> {
    let mut problems = Vec::new();
    let p = &mut problems;
    let Some(source) = &item.source else {
        p.push(Problem::new(
            "source",
            "says neither who made it nor its licence: original work says so \
             (`\"origin\": \"original\"`), an import names its provenance record",
        ));
        return problems;
    };
    if source.origin == Some(Origin::Original) && source.provenance.is_some() {
        p.push(Problem::new(
            "source.origin",
            "says it is original work but names a provenance record",
        ));
        return problems;
    }
    if !is_imported(item) {
        // Original work is released CC0 (class 5): any other licence is a
        // third party's, so the item was imported without its record.
        if source.licence != ORIGINAL_LICENCE {
            p.push(Problem::new(
                "source.licence",
                format!(
                    "is `{}`, which only an imported asset has (original work is released \
                     {ORIGINAL_LICENCE}): say `\"origin\": \"imported\"` and name its provenance \
                     record",
                    source.licence
                ),
            ));
        }
        return problems;
    }
    let Some(path) = &source.provenance else {
        p.push(Problem::new(
            "source.provenance",
            "is imported but names no provenance record",
        ));
        return problems;
    };
    let Some(record) = record else {
        p.push(Problem::new(
            "source.provenance",
            format!("its provenance record `{path}` isn't there"),
        ));
        return problems;
    };
    for problem in record.validate() {
        p.push(Problem::new(
            format!("{path}: {}", problem.at),
            problem.message,
        ));
    }
    if !record.items.contains(&item.id) {
        p.push(Problem::new(
            "source.provenance",
            format!(
                "`{path}` doesn't name `{}` among the items it feeds",
                item.id
            ),
        ));
    }
    if source.licence != record.licence.id {
        p.push(Problem::new(
            "source.licence",
            format!(
                "is `{}`, but its provenance record says `{}`",
                source.licence, record.licence.id
            ),
        ));
    }
    problems
}

/// The provenance record of item `id` in the library at `root`: `None`
/// when it names none or the file isn't there, an error when it can't be
/// read.
pub fn read_provenance(
    root: &Path,
    library: &Library,
    id: &str,
) -> Result<Option<Provenance>, String> {
    let named = library
        .item(id)
        .and_then(|item| item.source.as_ref())
        .and_then(|source| source.provenance.as_deref());
    let Some(named) = named else {
        return Ok(None);
    };
    let path = library
        .path_of(id, named)
        .filter(|path| crate::is_valid_path(path))
        .ok_or_else(|| format!("`{named}` leads out of the library"))?;
    let text = match std::fs::read_to_string(root.join(&path)) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("`{named}` can't be read: {err}")),
    };
    Provenance::from_json(&text)
        .map(Some)
        .map_err(|err| format!("`{named}` isn't a provenance record: {err}"))
}

/// Checks every item of the library at `root` ([`check_item_provenance`],
/// reading each record named); each problem starts with the item's ID.
pub fn check_provenance(root: &Path, library: &Library) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (id, item) in &library.items {
        let found = match read_provenance(root, library, id) {
            Ok(found) => found,
            Err(why) => {
                problems.push(Problem::new(format!("{id}: source.provenance"), why));
                continue;
            }
        };
        for problem in check_item_provenance(item, found.as_ref()) {
            problems.push(Problem::new(
                format!("{id}: {}", problem.at),
                problem.message,
            ));
        }
    }
    problems
}

/// One credit: the line, and the items (labels) it covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credit {
    pub line: String,
    pub items: Vec<String>,
}

/// The credits of a library's imported items: what a game that ships the
/// library must carry, what it carries as thanks, and the items that
/// aren't cleared (no record, a refused licence, no attribution line).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterCredits {
    pub required: Vec<Credit>,
    pub thanks: Vec<Credit>,
    pub uncleared: Vec<String>,
}

impl CharacterCredits {
    /// Whether no item is made from an imported asset.
    pub fn is_empty(&self) -> bool {
        self.required.is_empty() && self.thanks.is_empty() && self.uncleared.is_empty()
    }

    /// The plain text a game carries, headed with its name.
    pub fn text(&self, game: &str) -> String {
        let section = |text: &mut String, title: &str, credits: &[Credit]| {
            if credits.is_empty() {
                return;
            }
            text.push_str(&format!("\n{title}\n\n"));
            for credit in credits {
                text.push_str(&format!(
                    "- {}\n  Used for: {}\n",
                    credit.line,
                    credit.items.join(", ")
                ));
            }
        };
        let mut text = format!(
            "{game}: character asset credits\n\nCharacter items in this game (hair, clothing, \
             accessories) made from third-party assets.\n"
        );
        section(
            &mut text,
            "Credits (required by their licences)",
            &self.required,
        );
        section(
            &mut text,
            "With thanks (public domain, no credit required)",
            &self.thanks,
        );
        if !self.uncleared.is_empty() {
            text.push_str("\nNot cleared (no sound provenance record)\n\n");
            for line in &self.uncleared {
                text.push_str(&format!("- {line}\n"));
            }
        }
        text
    }
}

/// The credits of every item in the library at `root` (a game ships its
/// whole library, so any of them can be worn). Original work isn't
/// listed; an imported item under an attribution licence is credited with
/// its record's line, one under a public-domain licence is thanked (its
/// line, or title, author, source and licence when it has none), and one
/// whose record is missing or unsound is listed as not cleared.
pub fn character_credits(root: &Path, library: &Library) -> CharacterCredits {
    let mut credits = CharacterCredits::default();
    for (id, item) in library.items.iter().filter(|(_, item)| is_imported(item)) {
        let label = format!("{} ({id})", item.label);
        let record = match read_provenance(root, library, id) {
            Ok(Some(record)) => record,
            Ok(None) => {
                credits
                    .uncleared
                    .push(format!("{label}: its provenance record isn't there"));
                continue;
            }
            Err(why) => {
                credits.uncleared.push(format!("{label}: {why}"));
                continue;
            }
        };
        if let Some(problem) = check_item_provenance(item, Some(&record)).first() {
            credits.uncleared.push(format!("{label}: {problem}"));
            continue;
        }
        let required = licence_terms(&record.licence.id) == Ok(LicenceTerms::Attribution);
        let line = record.attribution.clone().unwrap_or_else(|| {
            format!(
                "\"{}\" by {} ({}), {}",
                record.asset, record.author, record.source_page, record.licence.id
            )
        });
        let list = if required {
            &mut credits.required
        } else {
            &mut credits.thanks
        };
        // One line per record, however many items it feeds.
        match list.iter_mut().find(|credit| credit.line == line) {
            Some(credit) => credit.items.push(item.label.clone()),
            None => list.push(Credit {
                line,
                items: vec![item.label.clone()],
            }),
        }
    }
    credits
}
