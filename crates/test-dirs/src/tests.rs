use super::*;
use std::ffi::OsString;
use std::path::Path;

/// Whether this run keeps test folders (`AXCENITY_KEEP_TEST_DIRS=1`). A test
/// that checks that dropping removes a folder has nothing to check then, so
/// it stops early; `releasing_...` checks both branches without the variable.
fn keeping(test: &str) -> bool {
    let keep = keep_requested(std::env::var_os(KEEP_VARIABLE).as_deref());
    if keep {
        eprintln!("{test}: skipped, {KEEP_VARIABLE} keeps folders on drop");
    }
    keep
}

#[test]
fn new_makes_an_empty_uniquely_named_folder_under_the_temp_folder() {
    let first = TestDir::new("unit");
    let second = TestDir::new("unit");
    assert!(first.is_dir());
    assert_eq!(std::fs::read_dir(&first).unwrap().count(), 0);
    assert_eq!(first.parent(), Some(std::env::temp_dir().as_path()));
    let name = first.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("axcenity-unit-"), "{name}");
    assert!(name.contains(&std::process::id().to_string()), "{name}");
    assert_ne!(first.path(), second.path());
}

#[test]
fn dropping_removes_the_folder_and_everything_in_it() {
    if keeping("dropping_removes_the_folder_and_everything_in_it") {
        return;
    }
    let dir = TestDir::new("drop");
    let path = dir.to_path_buf();
    std::fs::create_dir_all(dir.join("a/b")).unwrap();
    std::fs::write(dir.join("a/b/file.txt"), "x").unwrap();
    // Git's object files are read-only; Windows refuses to delete those.
    let locked = dir.join("a/locked.pack");
    std::fs::write(&locked, "x").unwrap();
    let mut permissions = std::fs::metadata(&locked).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&locked, permissions).unwrap();

    drop(dir);
    assert!(!path.exists(), "{} is still there", path.display());
}

#[test]
fn a_panicking_test_still_removes_its_folder() {
    if keeping("a_panicking_test_still_removes_its_folder") {
        return;
    }
    let mut seen = None;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let dir = TestDir::new("panics");
        std::fs::write(dir.join("half-written.json"), "{").unwrap();
        seen = Some(dir.to_path_buf());
        panic!("the test failed");
    }));
    assert!(result.is_err());
    let path = seen.unwrap();
    assert!(!path.exists(), "{} is still there", path.display());
}

#[test]
fn adopt_removes_a_file_or_folder_the_test_named_itself() {
    if keeping("adopt_removes_a_file_or_folder_the_test_named_itself") {
        return;
    }
    let file = std::env::temp_dir().join(format!("axcenity-adopt-{}.png", std::process::id()));
    std::fs::write(&file, "png").unwrap();
    drop(TestDir::adopt(&file));
    assert!(!file.exists());

    let folder = std::env::temp_dir().join(format!("axcenity-adopt-{}", std::process::id()));
    std::fs::create_dir_all(folder.join("inside")).unwrap();
    drop(TestDir::adopt(&folder));
    assert!(!folder.exists());

    // A path that was never made is fine too.
    drop(TestDir::adopt(folder.join("never-made")));
}

#[test]
fn unmade_leaves_making_the_folder_to_the_code_under_test() {
    if keeping("unmade_leaves_making_the_folder_to_the_code_under_test") {
        return;
    }
    let dir = TestDir::unmade("unmade");
    assert!(!dir.exists());
    std::fs::create_dir_all(dir.join("made-later")).unwrap();
    let path = dir.to_path_buf();
    drop(dir);
    assert!(!path.exists());
}

#[test]
fn at_stands_for_a_path_inside_and_still_removes_the_whole_folder() {
    if keeping("at_stands_for_a_path_inside_and_still_removes_the_whole_folder") {
        return;
    }
    let parent = TestDir::new("at");
    let root = parent.to_path_buf();
    let project = root.join("My Game");
    std::fs::create_dir_all(&project).unwrap();
    let dir = parent.at(&project);
    assert_eq!(dir.path(), project.as_path());
    assert!(dir.join("game.json").starts_with(&root));
    drop(dir);
    assert!(!root.exists(), "{} is still there", root.display());
}

/// What Drop does in each mode, checked directly: kept with the opt-out,
/// removed without it. This runs the same with or without the variable set.
#[test]
fn releasing_keeps_the_folder_when_asked_and_removes_it_otherwise() {
    let dir = TestDir::new("release");
    std::fs::write(dir.join("debug.png"), "png").unwrap();
    dir.release(true);
    assert!(
        dir.join("debug.png").is_file(),
        "{} was removed although keeping was asked for",
        dir.display()
    );
    dir.release(false);
    assert!(!dir.exists(), "{} is still there", dir.display());
}

#[test]
fn the_temp_folder_and_the_folders_above_it_are_never_removed() {
    let temp = Path::new("/tmp/axcenity-runs/temp");
    assert!(is_protected(temp, temp));
    assert!(is_protected(Path::new("/tmp/axcenity-runs"), temp));
    assert!(is_protected(Path::new("/"), temp));
    assert!(!is_protected(&temp.join("axcenity-x-1-0"), temp));
    assert!(!is_protected(&temp.join("axcenity-x-1-0/project"), temp));
    assert!(!is_protected(Path::new("/tmp/elsewhere"), temp));
}

#[test]
fn the_keep_variable_keeps_folders_when_set_to_a_true_value() {
    let keep = |value: &str| keep_requested(Some(OsString::from(value).as_os_str()));
    assert!(!keep_requested(None));
    assert!(!keep(""));
    assert!(!keep("0"));
    assert!(!keep("false"));
    assert!(!keep("FALSE"));
    assert!(keep("1"));
    assert!(keep("true"));
    assert!(keep("yes"));
}

/// A file another process still has open can't be deleted on Windows; the
/// drop waits for it to be let go (here a handle opened with no sharing,
/// closed 300 ms later on another thread).
#[cfg(windows)]
#[test]
fn a_file_still_open_for_a_moment_is_removed_once_it_is_let_go() {
    if keeping("a_file_still_open_for_a_moment_is_removed_once_it_is_let_go") {
        return;
    }
    use std::os::windows::fs::OpenOptionsExt;
    let dir = TestDir::new("held");
    let path = dir.to_path_buf();
    let file = dir.join("held.log");
    std::fs::write(&file, "x").unwrap();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        drop(held);
    });
    drop(dir);
    release.join().unwrap();
    assert!(!path.exists(), "{} is still there", path.display());
}
