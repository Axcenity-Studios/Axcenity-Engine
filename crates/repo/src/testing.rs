//! Test helpers: the real `git`, kept away from the machine's own Git
//! settings, and temporary folders.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::git::{Author, Git, Invocation, Output, Runner, SystemRunner};
use crate::secret::Secret;
use axcenity_test_dirs::TestDir;

/// The real `git` with no global or system settings, plus `config`.
pub struct ConfigRunner {
    inner: SystemRunner,
    config: Vec<(String, String)>,
    global: PathBuf,
    /// The folder `global` is in, removed with the runner.
    _global_dir: TestDir,
}

impl ConfigRunner {
    pub fn new(config: Vec<(String, String)>) -> ConfigRunner {
        let global_dir = temp("gitconfig");
        let global = global_dir.join("config");
        std::fs::write(&global, "").unwrap();
        ConfigRunner {
            inner: SystemRunner::default(),
            config,
            global,
            _global_dir: global_dir,
        }
    }
}

impl Runner for ConfigRunner {
    fn run(&self, call: &Invocation) -> std::io::Result<Output> {
        let mut args = Vec::new();
        for (key, value) in &self.config {
            args.push("-c".to_owned());
            args.push(format!("{key}={value}"));
        }
        args.extend(call.args.iter().cloned());
        let mut env = call.env.clone();
        env.push((
            "GIT_CONFIG_GLOBAL".into(),
            Secret::new(self.global.to_string_lossy()),
        ));
        env.push(("GIT_CONFIG_NOSYSTEM".into(), Secret::new("1")));
        self.inner.run(&Invocation {
            dir: call.dir,
            args,
            env,
            stdin: call.stdin.clone(),
            timeout: call.timeout,
        })
    }
}

/// `git` for tests: no machine settings, `main` as the first branch.
pub fn git() -> Git {
    Git::with_runner(Arc::new(ConfigRunner::new(vec![
        ("init.defaultBranch".into(), "main".into()),
        ("protocol.file.allow".into(), "always".into()),
    ])))
}

/// A new empty folder of its own under the temp folder
/// (`axcenity-repo-<name>-<process>-<n>`), removed when dropped. Folders
/// `axcenity-repo-<process>` that runs before `TestDir` left are removed the
/// first time.
pub fn temp(name: &str) -> TestDir {
    static SWEEP: std::sync::Once = std::sync::Once::new();
    SWEEP.call_once(|| sweep_old_runs(&std::env::temp_dir()));
    TestDir::new(&format!("repo-{name}"))
}

/// Removes `axcenity-repo-<process>` folders of test runs that ended (on
/// Linux by their process; elsewhere once a day old).
fn sweep_old_runs(base: &Path) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|n| n.strip_prefix("axcenity-repo-"))
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == std::process::id() {
            continue;
        }
        let ended = if cfg!(target_os = "linux") {
            !Path::new("/proc").join(pid.to_string()).exists()
        } else {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|t| t.elapsed().unwrap_or_default().as_secs() > 24 * 60 * 60)
        };
        if ended {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// A bare repository to push to, and its `file://` URL.
pub fn bare_remote(git: &Git) -> (TestDir, String) {
    let dir = temp("remote");
    git.run(
        &dir,
        &["init", "--quiet", "--bare", "--initial-branch", "main"],
    )
    .unwrap();
    let url = format!("file://{}", dir.to_string_lossy().replace('\\', "/"));
    (dir, url)
}

pub fn author(name: &str) -> Author {
    Author {
        name: name.into(),
        email: format!("{}@example.com", name.to_lowercase()),
    }
}

/// Writes `text` to `dir/path`, making folders.
pub fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}
