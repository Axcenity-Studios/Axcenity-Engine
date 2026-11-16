//! The contract's tests (CO1): files round-trip, format 1 migrates, checks
//! find what is wrong, parameters resolve. The repository's own library is
//! checked against its models in `tests/shipped_library.rs`.

mod formats;
mod migration;
mod models;
mod resolving;
mod validation;

/// The problems' texts, for assertions that name what they expect.
fn texts(problems: &[crate::Problem]) -> Vec<String> {
    problems.iter().map(ToString::to_string).collect()
}

/// Asserts that some problem mentions every one of `words`.
#[track_caller]
fn assert_problem(problems: &[crate::Problem], words: &[&str]) {
    let texts = texts(problems);
    assert!(
        texts
            .iter()
            .any(|text| words.iter().all(|word| text.contains(word))),
        "no problem mentions {words:?}: {texts:#?}"
    );
}
