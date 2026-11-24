//! The engine's own sample projects: small Axcenity projects kept inside the
//! engine's source (`crates/samples/projects/<name>`), one for each thing the
//! engine has to try on a real project. Its tests and benchmarks load them,
//! the game test runner's scenes take their models, and the editor's Learn
//! page opens them. Nothing in the engine reads a project from outside it.
//!
//! They are found in the source tree the program was built from, so they are
//! there for a build from source and for every test, and not for a packaged
//! build: ask [`folder`] and handle `None`.

use std::path::{Path, PathBuf};

/// Where the samples are in the engine's source, relative to its folder.
pub const SOURCE_FOLDER: &str = "crates/samples/projects";

/// The file every project has, by which a sample is known to be there.
const MANIFEST_FILE: &str = "axcenity.json";

/// The folder the samples are in, in the source tree this was built from
/// (whether or not it's still there).
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("projects")
}

/// The sample project `name` (`"part"`, `"touched"`, ...), if the source
/// tree this was built from still has it.
pub fn folder(name: &str) -> Option<PathBuf> {
    let folder = root().join(name);
    let named = !name.is_empty() && !name.contains(['/', '\\']) && name != "..";
    (named && folder.join(MANIFEST_FILE).is_file()).then_some(folder)
}

/// The sample project `name`, for the engine's own tests, which are always
/// run from its source.
///
/// # Panics
///
/// When there is no such sample.
pub fn project(name: &str) -> PathBuf {
    folder(name).unwrap_or_else(|| panic!("no sample project `{name}` in {}", root().display()))
}

/// The names of the samples that are there, sorted.
pub fn names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().join(MANIFEST_FILE).is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_samples_are_in_the_engines_source_by_name() {
        let names = names();
        assert!(names.len() > 20, "the samples: {names:?}");
        assert!(names.iter().all(|name| !name.starts_with("hello-")));
        for name in ["part", "touched", "animation"] {
            assert!(names.iter().any(|found| found == name), "no `{name}`");
            assert_eq!(folder(name), Some(root().join(name)));
            assert_eq!(project(name), root().join(name));
        }
        assert!(root().ends_with(SOURCE_FOLDER));
    }

    #[test]
    fn a_name_that_isnt_a_sample_is_none() {
        assert_eq!(folder("no-such-sample"), None);
        assert_eq!(folder(""), None);
        assert_eq!(folder(".."), None);
        assert_eq!(folder("part/assets"), None);
    }
}
