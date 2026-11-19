//! Paul's decision 11: an imported item names a provenance record with an
//! accepted licence and, under an attribution licence, its line; a game's
//! credits list what its library's imported items need.

use std::fs;
use std::path::PathBuf;

use super::{assert_problem, texts};
use crate::fixtures::{TEE, item};
use crate::{
    Catalogue, FormatError, ItemManifest, Library, LicenceTerms, Origin, PROVENANCE_VERSION,
    Provenance, character_credits, check_item_provenance, check_provenance, is_imported,
    licence_page, licence_terms, needs_notice, read_provenance,
};

const HASH: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0";

/// An MIT asset's LICENSE file: its copyright and permission notice.
const MIT_NOTICE: &str = "Copyright (c) 2026 A. Modeller\n\nPermission is hereby granted, free of \
                          charge, to any person obtaining a copy of this software... THE SOFTWARE \
                          IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND.";
/// An Apache-2.0 asset's copyright notice and NOTICE file.
const APACHE_NOTICE: &str = "Bucket Hat\nCopyright 2026 A. Modeller\nThis product includes a \
                             hat made by A. Modeller.";

/// The page of each licence's own text, as a record writes it.
fn page(licence: &str) -> &'static str {
    match licence {
        "CC0-1.0" => "https://creativecommons.org/publicdomain/zero/1.0/",
        "CC-BY-3.0" => "https://creativecommons.org/licenses/by/3.0/",
        "MIT" => "https://opensource.org/license/mit",
        "Apache-2.0" => "https://www.apache.org/licenses/LICENSE-2.0",
        _ => "https://creativecommons.org/licenses/by/4.0/",
    }
}

/// A sound record under `licence` of a hat that feeds `test/hat`.
fn record(licence: &str) -> Provenance {
    Provenance {
        format_version: PROVENANCE_VERSION,
        asset: "Bucket Hat".to_owned(),
        source_page: "https://example.org/assets/bucket-hat".to_owned(),
        author: "A. Modeller".to_owned(),
        licence: crate::LicenceRecord {
            id: licence.to_owned(),
            url: page(licence).to_owned(),
        },
        retrieved: "2026-11-18".to_owned(),
        files: vec![crate::SourceFile {
            name: "bucket_hat.glb".to_owned(),
            sha256: HASH.to_owned(),
            bytes: Some(1234),
        }],
        changes: vec!["Fitted to the realistic body".to_owned()],
        attribution: Some(
            "\"Bucket Hat\" by A. Modeller (https://example.org/assets/bucket-hat), CC BY 4.0, \
             fitted and rigged by Axcenity Engine contributors"
                .to_owned(),
        ),
        notice: match licence {
            "MIT" => Some(MIT_NOTICE.to_owned()),
            "Apache-2.0" => Some(APACHE_NOTICE.to_owned()),
            _ => None,
        },
        items: vec!["test/hat".to_owned()],
        evidence: Some("The page says: License CC BY 4.0".to_owned()),
    }
}

/// An imported hat under `licence`, naming its record.
fn hat(licence: &str) -> ItemManifest {
    let mut hat = item(TEE);
    hat.id = "test/hat".to_owned();
    hat.label = "Bucket hat".to_owned();
    let source = hat.source.as_mut().unwrap();
    source.author = "A. Modeller; fitted by Axcenity Engine contributors".to_owned();
    source.licence = licence.to_owned();
    source.origin = Some(Origin::Imported);
    source.provenance = Some("Hat.provenance.json".to_owned());
    hat
}

#[test]
fn original_work_needs_no_record() {
    let tee = item(TEE);
    assert!(!is_imported(&tee));
    assert!(check_item_provenance(&tee, None).is_empty());
    let mut said = tee.clone();
    said.source.as_mut().unwrap().origin = Some(Origin::Original);
    assert!(check_item_provenance(&said, None).is_empty());
}

#[test]
fn an_item_must_say_where_it_comes_from() {
    let mut tee = item(TEE);
    tee.source = None;
    assert_problem(&check_item_provenance(&tee, None), &["source"]);
}

#[test]
fn an_imported_item_without_a_record_fails() {
    let mut tee = item(TEE);
    tee.source.as_mut().unwrap().origin = Some(Origin::Imported);
    assert!(is_imported(&tee));
    assert_problem(
        &check_item_provenance(&tee, None),
        &["imported", "provenance"],
    );
    // It names one, but the file isn't there.
    assert_problem(
        &check_item_provenance(&hat("CC-BY-4.0"), None),
        &["Hat.provenance.json"],
    );
}

#[test]
fn a_licence_only_an_import_can_have_needs_a_record() {
    // Original work is released CC0-1.0, so an item under CC BY that says
    // nothing else was imported without its record.
    let mut tee = item(TEE);
    tee.source.as_mut().unwrap().licence = "CC-BY-4.0".to_owned();
    assert_problem(
        &check_item_provenance(&tee, None),
        &["CC-BY-4.0", "provenance"],
    );
}

#[test]
fn original_work_names_no_record() {
    let mut said = hat("CC-BY-4.0");
    said.source.as_mut().unwrap().origin = Some(Origin::Original);
    assert_problem(
        &check_item_provenance(&said, Some(&record("CC-BY-4.0"))),
        &["original"],
    );
}

#[test]
fn a_sound_record_passes() {
    for licence in ["CC-BY-4.0", "CC-BY-3.0", "MIT", "Apache-2.0", "CC0-1.0"] {
        let problems = check_item_provenance(&hat(licence), Some(&record(licence)));
        assert!(problems.is_empty(), "{licence}: {:#?}", texts(&problems));
    }
    // A public-domain asset needs no attribution line.
    let mut cc0 = record("CC0-1.0");
    cc0.attribution = None;
    let problems = check_item_provenance(&hat("CC0-1.0"), Some(&cc0));
    assert!(problems.is_empty(), "{:#?}", texts(&problems));
}

#[test]
fn refused_licences_fail_and_say_why() {
    for (licence, why) in [
        ("CC-BY-SA-4.0", "share-alike"),
        ("CC-BY-NC-4.0", "non-commercial"),
        ("CC-BY-NC-SA-3.0", "non-commercial"),
        ("CC-BY-ND-4.0", "no-derivatives"),
        ("GPL-3.0-only", "share-alike"),
        (
            "LicenseRef-Sketchfab-Standard",
            "isn't one Axcenity accepts",
        ),
        ("NOASSERTION", "unknown"),
        ("", "unknown"),
    ] {
        let problems = check_item_provenance(&hat(licence), Some(&record(licence)));
        assert_problem(&problems, &[why]);
    }
}

#[test]
fn an_attribution_licence_needs_its_line() {
    for licence in ["CC-BY-4.0", "MIT"] {
        let mut bare = record(licence);
        bare.attribution = None;
        assert_problem(
            &check_item_provenance(&hat(licence), Some(&bare)),
            &["attribution"],
        );
        bare.attribution = Some("  ".to_owned());
        assert_problem(
            &check_item_provenance(&hat(licence), Some(&bare)),
            &["attribution"],
        );
    }
}

#[test]
fn the_record_names_the_item_and_its_licence() {
    let mut other = record("CC-BY-4.0");
    other.items = vec!["test/cap".to_owned()];
    assert_problem(
        &check_item_provenance(&hat("CC-BY-4.0"), Some(&other)),
        &["test/hat"],
    );
    assert_problem(
        &check_item_provenance(&hat("CC0-1.0"), Some(&record("CC-BY-4.0"))),
        &["CC0-1.0", "CC-BY-4.0"],
    );
}

#[test]
fn a_record_says_where_when_what_and_how_it_changed() {
    let mut bad = record("CC-BY-4.0");
    bad.source_page = "example.org/hat".to_owned();
    bad.author = " ".to_owned();
    bad.licence.url = String::new();
    bad.retrieved = "18 November".to_owned();
    bad.files[0].sha256 = "abc".to_owned();
    bad.changes.clear();
    bad.items.push("Not An ID".to_owned());
    let problems = bad.validate();
    assert_problem(&problems, &["source_page"]);
    assert_problem(&problems, &["author"]);
    assert_problem(&problems, &["licence.url"]);
    assert_problem(&problems, &["retrieved"]);
    assert_problem(&problems, &["files[0].sha256"]);
    assert_problem(&problems, &["changes"]);
    assert_problem(&problems, &["items[1]"]);
    let mut none = record("CC0-1.0");
    none.files.clear();
    assert_problem(&none.validate(), &["files"]);
    assert!(record("CC-BY-4.0").validate().is_empty());
}

#[test]
fn licence_terms_follow_the_classes() {
    for id in ["CC0-1.0", "CC-PDDC", "LicenseRef-PublicDomain"] {
        assert_eq!(licence_terms(id), Ok(LicenceTerms::PublicDomain), "{id}");
    }
    for id in ["CC-BY-3.0", "CC-BY-4.0", "MIT", "Apache-2.0"] {
        assert_eq!(licence_terms(id), Ok(LicenceTerms::Attribution), "{id}");
    }
    for id in [
        "CC-BY-SA-4.0",
        "CC-BY-NC-4.0",
        "CC-BY-ND-3.0",
        "CC-BY-2.0",
        "AGPL-3.0-or-later",
        "custom",
    ] {
        assert!(licence_terms(id).is_err(), "{id}");
    }
}

#[test]
fn records_round_trip_and_a_newer_one_is_refused() {
    let hat = record("CC-BY-4.0");
    assert_eq!(Provenance::from_json(&hat.to_json()).unwrap(), hat);
    let newer = hat.to_json().replace(
        "\"format_version\": 1",
        &format!("\"format_version\": {}", PROVENANCE_VERSION + 1),
    );
    assert!(matches!(
        Provenance::from_json(&newer),
        Err(FormatError::Newer { .. })
    ));
}

#[test]
fn an_item_without_the_new_fields_writes_as_before() {
    // Format compatibility (plan 11.3): the fields are optional, so an
    // older item reads and writes the same, and an older engine ignores
    // them.
    let tee = item(TEE);
    let json = tee.to_json();
    assert!(!json.contains("origin") && !json.contains("provenance"));
    let imported = hat("CC-BY-4.0").to_json();
    assert!(imported.contains("\"origin\": \"imported\""), "{imported}");
    assert!(imported.contains("\"provenance\": \"Hat.provenance.json\""));
    assert_eq!(
        ItemManifest::from_json(&imported).unwrap(),
        hat("CC-BY-4.0")
    );
}

/// A library on disk: the tee (original), the hat (CC BY, with its
/// record) and a cap (CC0, with its record), each in its own folder.
struct OnDisk {
    root: PathBuf,
}

impl OnDisk {
    fn new(test: &str) -> OnDisk {
        let root = std::env::temp_dir().join(format!(
            "axcenity-avatar-provenance-{test}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("hat")).unwrap();
        fs::create_dir_all(root.join("cap")).unwrap();
        OnDisk { root }
    }

    fn library(&self, with_records: bool) -> Library {
        let mut library = Library::new();
        library.add_item("tee/tee.item.json", item(TEE));
        library.add_item("hat/Hat.item.json", hat("CC-BY-4.0"));
        let mut cap = hat("CC0-1.0");
        cap.id = "test/cap".to_owned();
        cap.label = "Cap".to_owned();
        library.add_item("cap/Cap.item.json", cap);
        if with_records {
            fs::write(
                self.root.join("hat/Hat.provenance.json"),
                record("CC-BY-4.0").to_json(),
            )
            .unwrap();
            let mut cc0 = record("CC0-1.0");
            cc0.asset = "Cotton Cap".to_owned();
            cc0.author = "B. Maker".to_owned();
            cc0.attribution = None;
            cc0.items = vec!["test/cap".to_owned()];
            fs::write(self.root.join("cap/Hat.provenance.json"), cc0.to_json()).unwrap();
        }
        fs::write(
            self.root.join("catalogue.json"),
            Catalogue::of(&library).to_json(),
        )
        .unwrap();
        library
    }
}

impl Drop for OnDisk {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn the_library_check_reads_each_record_beside_its_item() {
    let disk = OnDisk::new("check");
    let library = disk.library(true);
    let problems = check_provenance(&disk.root, &library);
    assert!(problems.is_empty(), "{:#?}", texts(&problems));
    let read = read_provenance(&disk.root, &library, "test/hat").unwrap();
    assert_eq!(read, Some(record("CC-BY-4.0")));
    assert_eq!(read_provenance(&disk.root, &library, "test/tee"), Ok(None));

    // A record that isn't there, and one that isn't a record.
    fs::remove_file(disk.root.join("cap/Hat.provenance.json")).unwrap();
    fs::write(disk.root.join("hat/Hat.provenance.json"), "{ not json").unwrap();
    let problems = check_provenance(&disk.root, &library);
    assert_problem(&problems, &["test/cap", "Hat.provenance.json"]);
    assert_problem(&problems, &["test/hat", "Hat.provenance.json"]);
    assert!(read_provenance(&disk.root, &library, "test/hat").is_err());
}

#[test]
fn credits_list_what_a_game_must_carry_and_thank_the_rest() {
    let disk = OnDisk::new("credits");
    let library = disk.library(true);
    let credits = character_credits(&disk.root, &library);
    assert_eq!(credits.required.len(), 1, "{credits:#?}");
    assert_eq!(
        credits.required[0].line,
        record("CC-BY-4.0").attribution.unwrap()
    );
    assert_eq!(credits.required[0].items, ["Bucket hat"]);
    assert_eq!(credits.thanks.len(), 1);
    assert!(credits.thanks[0].line.contains("Cotton Cap"));
    assert!(credits.thanks[0].line.contains("B. Maker"));
    assert!(credits.uncleared.is_empty());

    let text = credits.text("Hat Game");
    assert!(text.starts_with("Hat Game"), "{text}");
    assert!(text.contains(&record("CC-BY-4.0").attribution.unwrap()));
    assert!(text.contains("Cotton Cap"));
    // The tee is original work: nothing to credit.
    assert!(!text.contains("T-shirt"), "{text}");
    let required = text.find("Credits").unwrap();
    let thanks = text.find("With thanks").unwrap();
    assert!(required < thanks);
}

#[test]
fn an_item_that_isnt_cleared_is_listed_as_such() {
    let disk = OnDisk::new("uncleared");
    let library = disk.library(false);
    let credits = character_credits(&disk.root, &library);
    assert!(credits.required.is_empty());
    assert_eq!(credits.uncleared.len(), 2, "{credits:#?}");
    assert!(credits.text("Game").contains("Not cleared"));
}

#[test]
fn a_library_of_original_work_has_no_credits() {
    let disk = OnDisk::new("original");
    let mut library = Library::new();
    library.add_item("tee/tee.item.json", item(TEE));
    let credits = character_credits(&disk.root, &library);
    assert!(credits.is_empty());
}

/// Review fix 1: an item under a third party's licence that doesn't say
/// it was imported (what `import_garment --licence CC-BY-4.0` writes), or
/// that says nothing of its source, can't slip out of a game's credits:
/// it is listed as not cleared.
#[test]
fn a_third_party_item_that_doesnt_say_it_was_imported_is_not_cleared() {
    let disk = OnDisk::new("unmarked");
    let mut library = Library::new();
    library.add_item("tee/tee.item.json", item(TEE));
    let mut unmarked = item(TEE);
    unmarked.id = "test/tee-by".to_owned();
    unmarked.label = "Licensed tee".to_owned();
    unmarked.source.as_mut().unwrap().licence = "CC-BY-4.0".to_owned();
    library.add_item("tee-by/tee.item.json", unmarked);
    let mut unsourced = item(TEE);
    unsourced.id = "test/tee-unsourced".to_owned();
    unsourced.label = "Unsourced tee".to_owned();
    unsourced.source = None;
    library.add_item("tee-unsourced/tee.item.json", unsourced);
    let credits = character_credits(&disk.root, &library);
    assert!(!credits.is_empty());
    assert!(!credits.is_cleared());
    assert!(credits.required.is_empty() && credits.thanks.is_empty());
    assert_eq!(credits.uncleared.len(), 2, "{credits:#?}");
    let unmarked = credits
        .uncleared
        .iter()
        .find(|line| line.contains("test/tee-by"))
        .expect("the CC BY tee");
    assert!(unmarked.contains("Licensed tee") && unmarked.contains("CC-BY-4.0"));
    assert!(
        credits
            .uncleared
            .iter()
            .any(|line| line.contains("test/tee-unsourced"))
    );
    let text = credits.text("Game");
    assert!(text.contains("Not cleared"), "{text}");
    // Original work still isn't listed.
    assert!(!text.contains("(test/tee)"), "{text}");
}

#[test]
fn a_cleared_library_says_so() {
    let disk = OnDisk::new("cleared");
    let library = disk.library(true);
    assert!(character_credits(&disk.root, &library).is_cleared());
    let disk = OnDisk::new("not-cleared");
    let library = disk.library(false);
    assert!(!character_credits(&disk.root, &library).is_cleared());
}

/// Review fix 2: CC BY asks for the licence's address, so a record's
/// `licence.url` must be the licence's own text, not the source's page
/// about it.
#[test]
fn a_record_names_its_licences_own_text() {
    for (id, wanted) in [
        ("CC0-1.0", "creativecommons.org/publicdomain/zero/1.0"),
        ("CC-BY-3.0", "creativecommons.org/licenses/by/3.0"),
        ("CC-BY-4.0", "creativecommons.org/licenses/by/4.0"),
        ("MIT", "opensource.org/license/mit"),
        ("Apache-2.0", "apache.org/licenses/LICENSE-2.0"),
    ] {
        let found = licence_page(id).unwrap_or_else(|| panic!("{id} has no page"));
        assert!(found.contains(wanted), "{id}: {found}");
        let problems = record(id).validate();
        assert!(problems.is_empty(), "{id}: {:#?}", texts(&problems));
    }
    assert_eq!(licence_page("LicenseRef-PublicDomain"), None);
    let mut elsewhere = record("CC0-1.0");
    elsewhere.licence.url = "https://polyhaven.com/license".to_owned();
    assert_problem(
        &elsewhere.validate(),
        &["licence.url", "publicdomain/zero/1.0"],
    );
    let mut wrong = record("CC-BY-4.0");
    wrong.licence.url = page("CC-BY-3.0").to_owned();
    assert_problem(&wrong.validate(), &["licence.url", "by/4.0"]);
    // The legal code's page, over http, is the same text.
    let mut legal = record("CC-BY-4.0");
    legal.licence.url = "http://creativecommons.org/licenses/by/4.0/legalcode.en".to_owned();
    assert!(
        legal.validate().is_empty(),
        "{:#?}",
        texts(&legal.validate())
    );
}

/// MIT asks for its copyright and permission notice in every copy, and
/// Apache-2.0 for the copyright and NOTICE text: the record carries it.
#[test]
fn mit_and_apache_records_carry_their_notice() {
    assert!(needs_notice("MIT") && needs_notice("Apache-2.0"));
    assert!(!needs_notice("CC-BY-4.0") && !needs_notice("CC0-1.0"));
    for licence in ["MIT", "Apache-2.0"] {
        let mut bare = record(licence);
        bare.notice = None;
        assert_problem(&bare.validate(), &["notice", licence]);
        bare.notice = Some(" \n".to_owned());
        assert_problem(&bare.validate(), &["notice", licence]);
        assert_problem(
            &check_item_provenance(&hat(licence), Some(&bare)),
            &["notice"],
        );
    }
    // MIT's notice is the permission notice, not only a copyright line.
    let mut copyright_only = record("MIT");
    copyright_only.notice = Some("Copyright (c) 2026 A. Modeller".to_owned());
    assert_problem(&copyright_only.validate(), &["notice", "permission"]);
}

/// What a game carries for each required credit: the line, the licence
/// and its text's address, the source, that it was changed and how, and
/// (MIT, Apache-2.0) the notice in full, with the Apache License's text
/// once.
#[test]
fn a_required_credit_carries_what_its_licence_asks() {
    let disk = OnDisk::new("credit-terms");
    let mut library = Library::new();
    let mut records = Vec::new();
    for (folder, licence) in [
        ("by", "CC-BY-4.0"),
        ("mit", "MIT"),
        ("apache", "Apache-2.0"),
        ("apache2", "Apache-2.0"),
    ] {
        let mut hat = hat(licence);
        hat.id = format!("test/{folder}");
        hat.label = format!("Hat {folder}");
        library.add_item(format!("{folder}/Hat.item.json"), hat);
        let mut made = record(licence);
        made.asset = format!("Hat {folder} source");
        made.source_page = format!("https://example.org/assets/{folder}");
        made.attribution = Some(format!("\"Hat {folder}\" by A. Modeller"));
        made.changes = vec![
            "Fitted to the realistic body".to_owned(),
            "Rigged".to_owned(),
        ];
        made.items = vec![format!("test/{folder}")];
        fs::create_dir_all(disk.root.join(folder)).unwrap();
        fs::write(
            disk.root.join(folder).join("Hat.provenance.json"),
            made.to_json(),
        )
        .unwrap();
        records.push(made);
    }
    let credits = character_credits(&disk.root, &library);
    assert!(credits.is_cleared(), "{credits:#?}");
    assert_eq!(credits.required.len(), 4, "{credits:#?}");
    let text = credits.text("Hat Game");
    for record in &records {
        let line = record.attribution.as_deref().unwrap();
        let at = text
            .find(&format!("- {line}"))
            .unwrap_or_else(|| panic!("{line}: {text}"));
        // The credit runs to the next one.
        let entry = &text[at..];
        let entry = &entry[..entry[2..].find("\n- ").map_or(entry.len(), |end| end + 2)];
        assert!(entry.contains(&record.licence.id), "{entry}");
        assert!(entry.contains(&record.licence.url), "{entry}");
        assert!(entry.contains(&record.source_page), "{entry}");
        assert!(
            entry.contains("Changed: Fitted to the realistic body; Rigged"),
            "{entry}"
        );
        if let Some(notice) = &record.notice {
            assert!(entry.contains(notice.lines().next().unwrap()), "{entry}");
        }
    }
    assert!(text.contains("Permission is hereby granted"), "{text}");
    assert!(
        text.contains("This product includes a hat made by A. Modeller."),
        "{text}"
    );
    // The Apache License itself, once however many items use it.
    assert_eq!(
        text.matches("TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION")
            .count(),
        1,
        "{text}"
    );
}

#[test]
fn a_library_without_apache_items_carries_no_apache_text() {
    let disk = OnDisk::new("no-apache");
    let library = disk.library(true);
    let text = character_credits(&disk.root, &library).text("Game");
    assert!(!text.contains("TERMS AND CONDITIONS"), "{text}");
    // The CC BY credit names its licence's text and the change.
    assert!(
        text.contains("https://creativecommons.org/licenses/by/4.0/"),
        "{text}"
    );
    assert!(
        text.contains("Changed: Fitted to the realistic body"),
        "{text}"
    );
}

/// The terms a game carries for an Apache-2.0 item are the Apache
/// License 2.0's (the engine's own `LICENSE`, so a relicence would show).
#[test]
fn the_apache_text_is_the_apache_licence() {
    let text = crate::apache_licence_text();
    assert!(text.trim_start().starts_with("Apache License"), "{text}");
    assert!(text.contains("Version 2.0, January 2004"));
    assert!(text.contains("4. Redistribution."));
    assert!(text.trim_end().ends_with("END OF TERMS AND CONDITIONS"));
}
