//! IDs and paths in Character Data files.
//!
//! An ID names a base model or an item in a library: lower-case segments
//! of `a`–`z`, `0`–`9`, `-`, `_` and `.`, joined by `/`, at least two of
//! them (a namespace, then a name): `axcenity/tee`, `studio-x/coats/parka`.
//! First-party content uses the `axcenity` namespace.
//!
//! A path in a manifest is relative to the manifest's own folder, with `/`
//! between folders (`../HumanoidParts.glb`, `cards/swept.glb`): never
//! absolute, never a drive or a backslash. `..` is allowed; a library check
//! makes sure every path stays inside the library.

/// Whether `id` is a well-formed ID.
pub fn is_valid_id(id: &str) -> bool {
    let segments: Vec<&str> = id.split('/').collect();
    segments.len() >= 2
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment.chars().all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.')
                })
                && *segment != "."
                && *segment != ".."
        })
}

/// Whether `path` is a well-formed relative path.
pub fn is_valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path.split('/').all(|part| !part.is_empty())
}

/// The path of `relative` (written in the manifest at `manifest`, both
/// relative to the library's folder) relative to the library's folder,
/// with `.` and `..` worked out; `None` when it leaves the library.
pub fn resolve_path(manifest: &str, relative: &str) -> Option<String> {
    let mut parts: Vec<&str> = manifest.split('/').collect();
    parts.pop();
    for part in relative.split('/') {
        match part {
            "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_have_a_namespace_and_plain_lower_case_segments() {
        for good in ["axcenity/tee", "studio-x/coats/parka", "a/b.c_d-1"] {
            assert!(is_valid_id(good), "{good}");
        }
        for bad in [
            "tee",
            "",
            "Axcenity/Tee",
            "axcenity//tee",
            "axcenity/../x",
            "a b/c",
        ] {
            assert!(!is_valid_id(bad), "{bad}");
        }
    }

    #[test]
    fn paths_are_relative_and_resolve_inside_the_library() {
        for good in [
            "HumanoidParts.glb",
            "../HumanoidParts.glb",
            "cards/swept.glb",
        ] {
            assert!(is_valid_path(good), "{good}");
        }
        for bad in ["", "/abs.glb", "C:/x.glb", "a\\b.glb", "a//b.glb"] {
            assert!(!is_valid_path(bad), "{bad}");
        }
        assert_eq!(
            resolve_path("humanoid/items/tee.item.json", "../HumanoidParts.glb").as_deref(),
            Some("humanoid/HumanoidParts.glb")
        );
        assert_eq!(
            resolve_path("humanoid/Humanoid.base.json", "./HumanoidParts.glb").as_deref(),
            Some("humanoid/HumanoidParts.glb")
        );
        assert_eq!(resolve_path("tee.item.json", "../../x.glb"), None);
    }
}
