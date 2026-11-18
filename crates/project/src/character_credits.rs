//! The credits of a game's character items (Paul's decision 11,
//! 2026-11-18): a game carries the attribution of every item in its
//! character library made from an imported asset, without its author
//! having to find them. A game ships its whole library (`assets/
//! characters/`), so every item there can be worn and every one is
//! credited; items of the engine's library the project hasn't copied are
//! not. The lines come from each item's provenance record
//! (`axcenity_avatar::Provenance`).

use std::path::Path;

use axcenity_avatar::{CharacterCredits, Library};

use crate::CHARACTER_LIBRARY_PATH;

/// The plain-text credits file a game carries, at the top of the game
/// file and beside a built game.
pub const CHARACTER_CREDITS_FILE: &str = "CREDITS-CHARACTERS.txt";

/// The character credits of the project at `folder` and their text, or
/// `None` when no item of its library is made from an imported asset.
pub fn character_credits(folder: &Path) -> Option<(CharacterCredits, String)> {
    let root = folder.join(CHARACTER_LIBRARY_PATH);
    if !root.join("catalogue.json").is_file() {
        return None;
    }
    // What can't be read can't be worn either: only the items read count.
    let (library, _, _) = Library::read(&root);
    let credits = axcenity_avatar::character_credits(&root, &library);
    if credits.is_empty() {
        return None;
    }
    let game = crate::project_name(folder)
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "This game".to_owned());
    let text = credits.text(&game);
    Some((credits, text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    const HASH: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0";
    const LINE: &str = "\"Bucket Hat\" by A. Modeller (https://example.org/hat), CC BY 4.0 \
                        (https://creativecommons.org/licenses/by/4.0/); fitted and rigged by \
                        Axcenity Engine contributors";

    /// A project folder with a manifest and a character library of the
    /// items given as (folder, ID, label, licence, record).
    struct Project {
        folder: PathBuf,
    }

    impl Project {
        fn new(test: &str) -> Project {
            let folder = std::env::temp_dir().join(format!(
                "axcenity-character-credits-{test}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&folder);
            fs::create_dir_all(folder.join(CHARACTER_LIBRARY_PATH)).unwrap();
            fs::write(
                folder.join(crate::MANIFEST_FILE),
                r#"{ "name": "Hat Game", "format_version": 1 }"#,
            )
            .unwrap();
            Project { folder }
        }

        fn library(&self) -> PathBuf {
            self.folder.join(CHARACTER_LIBRARY_PATH)
        }

        /// Writes an item (and its record, for an import) and lists it.
        fn item(&self, folder: &str, id: &str, label: &str, licence: &str, imported: bool) {
            let dir = self.library().join(folder);
            fs::create_dir_all(&dir).unwrap();
            let source = if imported {
                format!(
                    r#"{{ "author": "A. Modeller", "licence": "{licence}", "origin": "imported", "provenance": "Item.provenance.json" }}"#
                )
            } else {
                format!(r#"{{ "author": "Axcenity Engine contributors", "licence": "{licence}" }}"#)
            };
            fs::write(
                dir.join("Item.item.json"),
                format!(
                    r#"{{ "format_version": 1, "id": "{id}", "label": "{label}", "kind": "accessory",
                         "slot": "hair", "bases": ["axcenity/realistic"],
                         "model": {{ "file": "Item.glb" }},
                         "fit": {{ "method": "binding" }}, "source": {source} }}"#
                ),
            )
            .unwrap();
            if imported {
                let attribution = if licence == "CC-BY-4.0" {
                    format!(r#""attribution": "{}","#, LINE.replace('"', "\\\""))
                } else {
                    String::new()
                };
                fs::write(
                    dir.join("Item.provenance.json"),
                    format!(
                        r#"{{ "format_version": 1, "asset": "{label} source", "source_page": "https://example.org/hat",
                             "author": "A. Modeller",
                             "licence": {{ "id": "{licence}", "url": "https://creativecommons.org/" }},
                             "retrieved": "2026-11-18",
                             "files": [{{ "name": "hat.glb", "sha256": "{HASH}" }}],
                             "changes": ["Fitted to the realistic body"], {attribution}
                             "items": ["{id}"] }}"#
                    ),
                )
                .unwrap();
            }
            let catalogue = self.library().join("catalogue.json");
            let mut entries: serde_json::Value = fs::read_to_string(&catalogue)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_else(|| serde_json::json!({ "format_version": 1, "items": [] }));
            entries["items"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "id": id, "manifest": format!("{folder}/Item.item.json"), "label": label,
                    "kind": "accessory", "slot": "hair", "tier": "built_in",
                    "bases": ["axcenity/realistic"]
                }));
            fs::write(&catalogue, entries.to_string()).unwrap();
        }

        /// The game packed and unpacked: its credits file, if any.
        fn packed_credits(&self) -> Option<String> {
            let game = crate::pack_game(&self.folder).unwrap();
            let unpacked = self.folder.with_extension("unpacked");
            let _ = fs::remove_dir_all(&unpacked);
            crate::unpack_game(&game, &unpacked).unwrap();
            let text = fs::read_to_string(unpacked.join(CHARACTER_CREDITS_FILE)).ok();
            let _ = fs::remove_dir_all(&unpacked);
            text
        }
    }

    impl Drop for Project {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.folder);
        }
    }

    #[test]
    fn a_game_whose_character_wears_a_cc_by_item_carries_its_line() {
        let project = Project::new("cc-by");
        project.item("tee", "axcenity/tee", "Tee", "CC0-1.0", false);
        project.item("hat", "imported/hat", "Bucket hat", "CC-BY-4.0", true);
        let (credits, text) = character_credits(&project.folder).expect("credits");
        assert_eq!(credits.required.len(), 1);
        assert!(text.starts_with("Hat Game"), "{text}");
        assert!(text.contains(LINE), "{text}");
        assert!(
            !text.contains("Tee"),
            "original work isn't credited: {text}"
        );
        let packed = project
            .packed_credits()
            .expect("the game carries its credits");
        assert_eq!(packed, text);
    }

    #[test]
    fn a_game_with_only_cc0_items_thanks_them_and_requires_nothing() {
        let project = Project::new("cc0");
        project.item("hat", "imported/hat", "Bucket hat", "CC0-1.0", true);
        let (credits, text) = character_credits(&project.folder).expect("thanks");
        assert!(credits.required.is_empty());
        assert_eq!(credits.thanks.len(), 1);
        assert!(text.contains("With thanks"), "{text}");
        assert!(!text.contains("Credits (required"), "{text}");
        assert_eq!(project.packed_credits(), Some(text));
    }

    #[test]
    fn a_game_of_original_items_or_without_a_library_has_no_credits() {
        let project = Project::new("original");
        assert!(character_credits(&project.folder).is_none());
        assert_eq!(project.packed_credits(), None);
        project.item("tee", "axcenity/tee", "Tee", "CC0-1.0", false);
        assert!(character_credits(&project.folder).is_none());
        assert_eq!(project.packed_credits(), None);
    }

    #[test]
    fn only_the_games_own_items_are_credited() {
        let project = Project::new("only-listed");
        project.item("hat", "imported/hat", "Bucket hat", "CC-BY-4.0", true);
        // A record of an item the catalogue doesn't list: no item of the
        // game wears it.
        let stray = project.library().join("stray");
        fs::create_dir_all(&stray).unwrap();
        fs::copy(
            project.library().join("hat/Item.provenance.json"),
            stray.join("Item.provenance.json"),
        )
        .unwrap();
        fs::write(
            stray.join("Item.provenance.json"),
            fs::read_to_string(stray.join("Item.provenance.json"))
                .unwrap()
                .replace("Bucket hat source", "Stray source"),
        )
        .unwrap();
        let (_, text) = character_credits(&project.folder).unwrap();
        assert!(!text.contains("Stray"), "{text}");
        assert_eq!(text.matches(LINE).count(), 1, "{text}");
    }
}
