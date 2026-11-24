# `samples`

- The engine's own sample projects, in `projects/<name>` (no `hello-` prefix), and the three functions that find them (`folder`, `project`, `names`). Tests, benchmarks, the game test runner's scenes (`include_bytes!`) and the editor's Learn page use them; nothing in the engine reads a project from outside the repo. Topic: `docs/project/samples.md`.
- Keep them small: a sample holds what a test or a lesson needs, never a showcase's media. A model several tests share lives in one sample and the others' tests read it from there.
- A crate's tests reach a sample by path from their own folder: `Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples/projects/<name>")`. Code that isn't a test asks `axcenity_samples::folder`, which is `None` in a packaged build. No dependencies here but the workspace hack.
