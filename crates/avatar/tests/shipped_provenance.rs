//! Paul's decision 11 on the repository's own library: every shipped item
//! says where it comes from, and an imported one names a sound
//! provenance record. The intake scripts' licence list
//! (`assets/characters/realistic/import/licences.json`) is the crate's.

use std::path::{Path, PathBuf};

use axcenity_avatar::{ACCEPTED_LICENCES, Library, LicenceTerms, check_provenance, licence_terms};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

#[test]
fn every_shipped_item_is_original_or_has_its_provenance() {
    let (library, _, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{problems:?}");
    assert!(!library.items.is_empty());
    let problems = check_provenance(&root(), &library);
    let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
    assert!(problems.is_empty(), "{lines:#?}");
}

#[test]
fn the_intake_scripts_accept_and_refuse_the_same_licences() {
    let path = root().join("realistic/import/licences.json");
    let text = std::fs::read_to_string(&path).expect("the intake's licence list");
    let list: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut accepted = Vec::new();
    for (terms, wanted) in [
        ("public_domain", LicenceTerms::PublicDomain),
        ("attribution", LicenceTerms::Attribution),
    ] {
        for id in list["accepted"][terms].as_array().unwrap() {
            let id = id.as_str().unwrap();
            assert_eq!(licence_terms(id), Ok(wanted), "{id}");
            accepted.push(id.to_owned());
        }
    }
    let ours: Vec<&str> = ACCEPTED_LICENCES.iter().map(|(id, _)| *id).collect();
    assert_eq!(accepted.len(), ours.len(), "{accepted:?} against {ours:?}");
    for example in list["refused_examples"].as_array().unwrap() {
        let id = example.as_str().unwrap();
        assert!(licence_terms(id).is_err(), "{id} should be refused");
    }
}

/// A library made outside the repository (an intake's scratch library,
/// `assets/characters/realistic/import/README.md`): its items pass the
/// provenance check, and the credits a game made from it would carry are
/// printed.
///
/// ```text
/// AXCENITY_PROVENANCE_LIBRARY=<folder> cargo test -p axcenity-avatar --test shipped_provenance -- --ignored --nocapture
/// ```
#[test]
#[ignore = "checks the library AXCENITY_PROVENANCE_LIBRARY names"]
fn a_named_library_passes_the_provenance_check() {
    let Some(root) = std::env::var_os("AXCENITY_PROVENANCE_LIBRARY") else {
        eprintln!("skipped: set AXCENITY_PROVENANCE_LIBRARY");
        return;
    };
    let root = PathBuf::from(root);
    let (library, _, problems) = Library::read(&root);
    assert!(problems.is_empty(), "{problems:?}");
    let mut problems = library.validate();
    problems.extend(check_provenance(&root, &library));
    let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
    assert!(problems.is_empty(), "{lines:#?}");
    let credits = axcenity_avatar::character_credits(&root, &library);
    println!(
        "{} items checked\n{}",
        library.items.len(),
        credits.text("Game")
    );
}
