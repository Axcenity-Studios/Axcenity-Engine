//! Temporary folders for tests that clean up after themselves.
//!
//! A test that writes files makes a [`TestDir`]: a uniquely named folder
//! under the system temp folder that is removed when the `TestDir` is
//! dropped. Drop also runs while a failing test unwinds, so a panicking test
//! cleans up too. Set `AXCENITY_KEEP_TEST_DIRS=1` to keep the folders for
//! debugging; each kept folder's path is printed to standard error.
//!
//! ```
//! let project = axcenity_test_dirs::TestDir::new("my-test");
//! std::fs::write(project.join("game.json"), "{}").unwrap();
//! assert!(project.join("game.json").is_file());
//! // `project` derefs to `PathBuf`, so `project.join(..)` and `&project`
//! // work wherever a path is expected. The folder goes when it drops.
//! ```
//!
//! See `docs/development.md` ("Temporary folders in tests").

use std::ffi::OsStr;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The environment variable that keeps test folders instead of removing them.
pub const KEEP_VARIABLE: &str = "AXCENITY_KEEP_TEST_DIRS";

/// A temporary folder (or file) that is removed when this value is dropped.
///
/// It derefs to [`PathBuf`] (and so on to [`Path`]), so `&dir` can be passed to
/// anything that takes a `&Path`, `&PathBuf` or `impl AsRef<Path>`, and
/// `dir.clone()` or `dir.to_path_buf()` gives an owned `PathBuf` (a path only:
/// the copy doesn't remove anything). Deref to `PathBuf` rather than `Path` lets
/// a helper that returned a `PathBuf` return a `TestDir` with its callers
/// unchanged. Keep the `TestDir` alive for as long as the test uses
/// the folder (a test that checks something survives a restart keeps it for the
/// whole test).
#[derive(Debug)]
pub struct TestDir {
    /// What is removed on drop.
    root: PathBuf,
    /// What the value derefs to: `root`, or a path inside it ([`TestDir::at`]).
    path: PathBuf,
}

impl TestDir {
    /// Makes a new, empty folder named `axcenity-{name}-{process id}-{n}` under
    /// the system temp folder. The process id and a per-process counter make it
    /// unique across parallel test processes (nextest) and threads (`cargo test`).
    ///
    /// # Panics
    ///
    /// If the folder can't be made.
    pub fn new(name: &str) -> TestDir {
        let dir = TestDir::unmade(name);
        // A folder left with the same name (a reused process id from a run
        // killed earlier) would not be empty: start clean.
        remove(&dir.root);
        std::fs::create_dir_all(&dir.root)
            .unwrap_or_else(|error| panic!("can't make {}: {error}", dir.root.display()));
        dir
    }

    /// Like [`TestDir::new`] but doesn't make the folder, for a test whose code
    /// under test must create it (or a file of that name) itself.
    pub fn unmade(name: &str) -> TestDir {
        TestDir::adopt(unique_path(name))
    }

    /// Takes charge of a path a test chose itself (a folder or a file, made or
    /// not yet): it is removed when the returned value drops.
    pub fn adopt(path: impl Into<PathBuf>) -> TestDir {
        let root = path.into();
        TestDir {
            path: root.clone(),
            root,
        }
    }

    /// The same temporary folder, still removed whole on drop, but standing
    /// for `path`, a folder or file inside it: for a helper that makes a
    /// project inside its temporary folder and returns the project's path.
    pub fn at(mut self, path: impl Into<PathBuf>) -> TestDir {
        self.path = path.into();
        self
    }

    /// The folder's path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Deref for TestDir {
    type Target = PathBuf;

    fn deref(&self) -> &PathBuf {
        &self.path
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<OsStr> for TestDir {
    fn as_ref(&self) -> &OsStr {
        self.path.as_os_str()
    }
}

impl TestDir {
    /// What dropping does: removes the folder, or with `keep` leaves it and
    /// prints its path. Drop passes whether [`KEEP_VARIABLE`] is set; a test
    /// can call this directly to check either branch without setting it.
    pub fn release(&self, keep: bool) {
        if keep {
            if self.root.exists() {
                eprintln!("{KEEP_VARIABLE}: kept {}", self.root.display());
            }
            return;
        }
        remove(&self.root);
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        self.release(keep_requested(std::env::var_os(KEEP_VARIABLE).as_deref()));
    }
}

/// Whether the value of [`KEEP_VARIABLE`] asks to keep test folders: set and
/// not empty, `0` or `false`.
pub fn keep_requested(value: Option<&OsStr>) -> bool {
    match value.and_then(OsStr::to_str) {
        Some(value) => {
            let value = value.trim();
            !(value.is_empty() || value == "0" || value.eq_ignore_ascii_case("false"))
        }
        // A value that isn't Unicode is still a value someone set.
        None => value.is_some(),
    }
}

/// `axcenity-{name}-{pid}-{n}` under the system temp folder.
fn unique_path(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("axcenity-{name}-{}-{n}", std::process::id()))
}

/// Removes a folder with everything in it, or a file. Never panics: a drop
/// that panics while a failing test unwinds would abort the process and hide
/// the real failure, so a folder that can't be removed is reported instead.
fn remove(path: &Path) {
    if is_protected(path, &std::env::temp_dir()) {
        eprintln!(
            "axcenity-test-dirs: refused to remove {}: it is the temp folder or holds it",
            path.display()
        );
        return;
    }
    // On Windows a file still open in another process (a luau-lsp or
    // updater the test started, which is just exiting) can't be deleted
    // yet: try again for up to about three seconds.
    let mut last_error = None;
    for attempt in 0..REMOVE_ATTEMPTS {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        match remove_once(path) {
            Ok(()) => return,
            Err(error) => last_error = Some(error),
        }
        if std::fs::symlink_metadata(path).is_err() {
            return; // Gone after all.
        }
    }
    if let Some(error) = last_error {
        eprintln!(
            "axcenity-test-dirs: couldn't remove {}: {error}",
            path.display()
        );
    }
}

/// How many times [`remove`] tries, 100 ms apart.
const REMOVE_ATTEMPTS: u32 = 30;

/// One try at removing a folder (with everything in it) or a file.
fn remove_once(path: &Path) -> std::io::Result<()> {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return Ok(()); // Already gone, or never made.
    };
    if metadata.is_dir() {
        std::fs::remove_dir_all(path).or_else(|_| {
            // Git marks its object files read-only, and on Windows a
            // read-only file can't be deleted: clear the flag and retry.
            clear_read_only(path);
            std::fs::remove_dir_all(path)
        })
    } else {
        std::fs::remove_file(path).or_else(|_| {
            clear_read_only(path);
            std::fs::remove_file(path)
        })
    }
}

/// Whether removing `path` would take the temp folder `temp` with it: `path`
/// is the temp folder itself, one of the folders above it, or a root. A test
/// that adopts `std::env::temp_dir()` by mistake must never empty it.
pub fn is_protected(path: &Path, temp: &Path) -> bool {
    path.parent().is_none() || temp.starts_with(path)
}

/// Clears the read-only flag on a path and everything under it.
fn clear_read_only(path: &Path) {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return;
    };
    if metadata.is_dir()
        && let Ok(entries) = std::fs::read_dir(path)
    {
        for entry in entries.flatten() {
            clear_read_only(&entry.path());
        }
    }
    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        // Clippy warns that on Unix this makes the file writable by everyone;
        // that's fine for a test folder about to be deleted.
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        let _ = std::fs::set_permissions(path, permissions);
    }
}

#[cfg(test)]
mod tests;
