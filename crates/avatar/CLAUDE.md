# `avatar`

- Character Data (Character Studio overhaul CO1): the `*.character.json` description (format 2; format 1, the workspace's `Look`, migrates in `migrate.rs`), the base model manifest (`*.base.json`: parameters, regions, slots, sockets, materials, skeleton, face), the item manifest (`*.item.json`: garments, accessories, grooms), the catalogue (`catalogue.json`), material values per shading model (`material.rs`), the checks (`Problem`) and what parameters do to shapes (`resolve.rs`).
- Every schema, rule and version is in `docs/development/character-studio/architecture.md`; change it there first. Only the lead changes a contract (plan section 5).
- Depends on `assets` only (and may on `core`): no rendering, UI, Instances or editor state, so `player`, `server` and `editor` can all use it.
- A new optional field needs no version bump (unknown fields are kept); a changed meaning bumps `format_version` and adds a migration. Never panic on a bad file.
- Tests: unit tests per module; `tests/shipped_library.rs` checks the repository's library (`assets/characters/`) against its models. After changing the humanoid's manifests run `python -I assets/characters/humanoid/manifests.py` and `python -I assets/characters/catalogue.py`.
