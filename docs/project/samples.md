# Samples

The engine's own sample projects: small Axcenity projects inside the engine's source, in `crates/samples/projects/<name>` (the `samples` crate). They are what the engine itself opens: its tests and benchmarks load them, the game test runner's scenes take their models, and the editor's Learn page opens them. Nothing in the engine reads a project from outside the repository.

Until 2026-11-23 they were `examples/hello-<name>` at the top of the repository, with everything a showcase would hold (4K texture sets, several copies of the same character). The complete projects are no longer in the repository; the engine keeps what its tests and lessons need.

## Finding one

- **A test** reaches a sample by path from its own crate's folder: `Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples/projects/<name>")`. Tests are always run from the source, so it is there. A source file that takes a sample's file at build time (`testing/src/s10k.rs`, `crowd.rs`) names it from its own folder: `include_bytes!("../../samples/projects/<name>/...")`.
- **Code that isn't a test** asks the crate: `axcenity_samples::folder(name)` is the sample's folder, or `None` when the source tree the program was built from isn't there any more, as in a packaged build. `project(name)` is the same for tests and panics instead; `names()` lists the samples that are there; `SOURCE_FOLDER` is `crates/samples/projects`.
- **The editor's Learn page** (`editor/src/start/learn.rs`, `find_sample`) looks for `crates/samples/projects/<name>` under the folder it was started from, then under each folder above its own program, then in the source tree a debug build came from, then asks `axcenity_samples::folder`. A packaged editor has none beside it, and **Open sample** says so.
- **By hand**: `cargo run -p axcenity-player -- crates/samples/projects/part`.

A documentation page names the samples it follows in its front matter (`samples: [part, touched]`), and a link to `.../crates/samples/projects/<name>` on GitHub opens that sample in the editor ([../editor/learn-and-docs.md](../editor/learn-and-docs.md)).

## Rules

- **Small.** A sample holds what a test or a lesson needs. Textures are 512 pixels across at most (the `roads` and `character` ground sets keep their `4K` file names from the sets they were made from, so the worlds that name them didn't change). No showcase media.
- **One copy of a shared model.** `character` has the Walking and Fast Run characters, `events` its own Walking (its world and the posing tests use it), `pbr` the Damaged Helmet, `primitives` the AWP (the cook tests' big model with embedded maps), `humanoid` the default humanoid, `animation` the Noodle.
- **No prefix.** A sample's folder is the thing it shows: `part`, `touched`, `ui-graph`. Its display name is in its `axcenity.json`.
- A new sample is loaded, drawn and checked by the tests that walk the whole folder (`project` `every_example_loads_without_problems`, `player` `every_example_renders`, the editor's properties and Luau checks), so it must load without problems.

## Tests

`samples/src/lib.rs`: the samples are found by name without a prefix; a name that isn't one (empty, a path, `..`) is `None`.
