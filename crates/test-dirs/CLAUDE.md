# `test-dirs`

- `TestDir`: a test's temporary folder (or file) under the system temp folder, removed when dropped, also when the test panics. `AXCENITY_KEEP_TEST_DIRS=1` keeps them for debugging. How tests use it: `docs/development.md` ("Temporary folders in tests").
- A dev-dependency only, of any crate. Standard library only: never add a dependency or an engine crate here (it would make a cycle).
