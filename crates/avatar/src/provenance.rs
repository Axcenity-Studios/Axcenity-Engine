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
    /// The licence's notice as given with the asset, word for word: for
    /// MIT, its copyright and permission notice (the asset's LICENSE
    /// file); for Apache-2.0, its copyright notice and the text of any
    /// NOTICE file. Required for those two ([`needs_notice`]); a game
    /// carries it in full.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
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

/// The page of an accepted licence's own text (its legal code or deed),
/// which a record's `licence.url` must be; `None` for a public-domain
/// statement without a licence.
pub fn licence_page(id: &str) -> Option<&'static str> {
    Some(match id {
        "CC0-1.0" => "https://creativecommons.org/publicdomain/zero/1.0/",
        "CC-PDDC" => "https://creativecommons.org/licenses/publicdomain/",
        "CC-BY-3.0" => "https://creativecommons.org/licenses/by/3.0/",
        "CC-BY-4.0" => "https://creativecommons.org/licenses/by/4.0/",
        "MIT" => "https://opensource.org/license/mit",
        "Apache-2.0" => "https://www.apache.org/licenses/LICENSE-2.0",
        _ => return None,
    })
}

/// Whether `url` is the licence's own text: its page or a page under it
/// (CC's `.../legalcode.en`), over http or https, with or without `www.`.
fn is_licence_page(page: &str, url: &str) -> bool {
    let bare = |url: &str| -> String {
        let url = url.trim();
        let url = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        url.strip_prefix("www.")
            .unwrap_or(url)
            .trim_end_matches('/')
            .to_owned()
    };
    let (page, url) = (bare(page), bare(url));
    // The next character after the page closes a path segment, so
    // `by/4.0` doesn't take `by/4.01`.
    url.strip_prefix(&page)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/') || rest.starts_with('.'))
}

/// Whether a licence asks for its notice to travel with every copy, so a
/// record must carry it word for word: MIT (its copyright and permission
/// notice "shall be included in all copies") and Apache-2.0 (section 4:
/// the copyright notices and any NOTICE text).
pub fn needs_notice(id: &str) -> bool {
    matches!(id, "MIT" | "Apache-2.0")
}

/// The Apache License 2.0's terms, which a game carries once when an item
/// is under it (section 4(a)). The engine is itself Apache-2.0, so this
/// is its own `LICENSE` file, up to the end of the terms (a test checks
/// it still is that licence).
pub fn apache_licence_text() -> &'static str {
    const LICENSE: &str = include_str!("../../../LICENSE");
    const END: &str = "END OF TERMS AND CONDITIONS";
    match LICENSE.find(END) {
        Some(at) => &LICENSE[..at + END.len()],
        None => LICENSE,
    }
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
        // CC BY asks for the licence's address with the credit: the
        // record's is the licence's own text, not a page about it.
        if let Some(page) = licence_page(&self.licence.id)
            && is_url(&self.licence.url)
            && !is_licence_page(page, &self.licence.url)
        {
            p.push(Problem::new(
                "licence.url",
                format!(
                    "`{}` isn't the text of `{}` ({page}); the source's page about the licence \
                     belongs in `evidence`",
                    self.licence.url, self.licence.id
                ),
            ));
        }
        let notice = self.notice.as_deref().unwrap_or("").trim();
        if needs_notice(&self.licence.id) {
            if notice.is_empty() {
                p.push(Problem::new(
                    "notice",
                    format!(
                        "`{}` asks for its notice in every copy: record it word for word as \
                         given with the asset",
                        self.licence.id
                    ),
                ));
            } else if self.licence.id == "MIT"
                && !notice
                    .to_ascii_lowercase()
                    .contains("permission is hereby granted")
            {
                p.push(Problem::new(
                    "notice",
                    "MIT's notice is the copyright and permission notice (\"Permission is \
                     hereby granted...\"), not only a copyright line",
                ));
            }
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

/// One credit, from the record it comes from: the attribution line, the
/// licence and the address of its text, the source, what was changed,
/// the licence's notice (MIT, Apache-2.0) and the items (labels) it
/// covers. CC BY asks for the licence's address and a note that the work
/// was changed (4.0, 3(a)(1)); MIT and Apache-2.0 for their notice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Credit {
    pub line: String,
    /// The licence's SPDX ID.
    pub licence: String,
    /// The address of the licence's text.
    pub licence_url: String,
    pub source_page: String,
    pub changes: Vec<String>,
    pub notice: Option<String>,
    pub items: Vec<String>,
}

impl Credit {
    fn of(record: &Provenance, line: String) -> Credit {
        Credit {
            line,
            licence: record.licence.id.clone(),
            licence_url: record.licence.url.clone(),
            source_page: record.source_page.clone(),
            changes: record
                .changes
                .iter()
                .map(|change| change.trim().to_owned())
                .filter(|change| !change.is_empty())
                .collect(),
            notice: record
                .notice
                .as_deref()
                .map(str::trim)
                .filter(|notice| !notice.is_empty())
                .map(str::to_owned),
            items: Vec::new(),
        }
    }

    /// Whether `other` credits the same record (all but the items).
    fn same_as(&self, other: &Credit) -> bool {
        Credit {
            items: Vec::new(),
            ..self.clone()
        } == Credit {
            items: Vec::new(),
            ..other.clone()
        }
    }

    /// The credit as the file prints it.
    fn write(&self, text: &mut String) {
        text.push_str(&format!("- {}\n", self.line));
        text.push_str(&format!(
            "  Licence: {} ({})\n",
            self.licence, self.licence_url
        ));
        text.push_str(&format!("  Source: {}\n", self.source_page));
        if !self.changes.is_empty() {
            text.push_str(&format!("  Changed: {}\n", self.changes.join("; ")));
        }
        text.push_str(&format!("  Used for: {}\n", self.items.join(", ")));
        if let Some(notice) = &self.notice {
            text.push_str("  Notice, as given with the asset:\n");
            for line in notice.lines() {
                text.push_str(format!("    {line}").trim_end());
                text.push('\n');
            }
        }
    }
}

/// The credits of a library's character items: what a game that ships
/// the library must carry, what it carries as thanks, and the items that
/// aren't cleared for shipping (a third party's licence without a sound
/// record, a refused licence, no attribution line or notice, no source).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterCredits {
    pub required: Vec<Credit>,
    pub thanks: Vec<Credit>,
    /// `label (id): problem`, one per item.
    pub uncleared: Vec<String>,
}

impl CharacterCredits {
    /// Whether every item may ship: none is listed as not cleared.
    pub fn is_cleared(&self) -> bool {
        self.uncleared.is_empty()
    }

    /// Whether there is nothing to list: every item is original work.
    pub fn is_empty(&self) -> bool {
        self.required.is_empty() && self.thanks.is_empty() && self.uncleared.is_empty()
    }

    /// The plain text a game carries, headed with its name: each credit
    /// with its licence, source, changes and notice, then the Apache
    /// License's terms once when an item is under it.
    pub fn text(&self, game: &str) -> String {
        let section = |text: &mut String, title: &str, credits: &[Credit]| {
            if credits.is_empty() {
                return;
            }
            text.push_str(&format!("\n{title}\n\n"));
            for credit in credits {
                credit.write(text);
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
            text.push_str("\nNot cleared for shipping (no sound licence or provenance record)\n\n");
            for line in &self.uncleared {
                text.push_str(&format!("- {line}\n"));
            }
        }
        if self
            .required
            .iter()
            .any(|credit| credit.licence == "Apache-2.0")
        {
            text.push_str(&format!(
                "\nThe Apache License, Version 2.0, under which the items above marked \
                 Apache-2.0 are used:\n\n{}\n",
                apache_licence_text()
            ));
        }
        text
    }
}

/// The credits of every item in the library at `root` (a game ships its
/// whole library, so any of them can be worn). Original work isn't
/// listed; an imported item under an attribution licence is credited from
/// its record, one under a public-domain licence is thanked (its line, or
/// title, author, source and licence when it has none), and any item
/// that can't ship as it is (a record missing or unsound, or a third
/// party's licence on an item that doesn't say it was imported, or no
/// source at all) is listed as not cleared, so nothing is left out.
pub fn character_credits(root: &Path, library: &Library) -> CharacterCredits {
    let mut credits = CharacterCredits::default();
    for (id, item) in &library.items {
        let label = format!("{} ({id})", item.label);
        if !is_imported(item) {
            // Original work under CC0 has nothing to credit; anything
            // else (another licence, no source) is a third party's work
            // without its record.
            if let Some(problem) = check_item_provenance(item, None).first() {
                credits.uncleared.push(format!("{label}: {problem}"));
            }
            continue;
        }
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
        let credit = Credit::of(&record, line);
        let list = if required {
            &mut credits.required
        } else {
            &mut credits.thanks
        };
        // One credit per record, however many items it feeds.
        match list.iter_mut().find(|listed| listed.same_as(&credit)) {
            Some(listed) => listed.items.push(item.label.clone()),
            None => list.push(Credit {
                items: vec![item.label.clone()],
                ..credit
            }),
        }
    }
    credits
}
