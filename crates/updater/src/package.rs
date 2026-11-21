//! Updating the installs `axcenity-updater` doesn't swap: the Linux system
//! packages (`.deb`, `.rpm`, Arch) and the AppImage. The editor downloads
//! the release's own package of that kind ([`crate::download`] checks it
//! against the signed manifest), then:
//!
//! - a system package goes to the package manager as root, through
//!   `pkexec` (the desktop's own password prompt). As root, the package is
//!   first copied into a fresh root-only folder and its SHA-256 checked
//!   again, so what the package manager installs is exactly what the
//!   manifest signed, even if the user's download folder changes meanwhile;
//! - an AppImage is copied beside the running one and renamed over it.
//!
//! The running editor keeps working on the old files until it restarts.
//! ADR-0043 records why system packages may ask for a password.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use axcenity_release::InstallKind;

/// Run by `/bin/sh` as root: `$1` the downloaded package, `$2` its SHA-256
/// from the signed manifest, `$3` the install kind (`rpm`, `deb` or
/// `pacman`). Exit 2: unknown kind; 3: the checksum doesn't match;
/// otherwise the package manager's own status.
pub const SCRIPT: &str = r#"set -eu
case "$3" in
  rpm) ext=rpm ;;
  deb) ext=deb ;;
  pacman) ext=pkg.tar.zst ;;
  *) echo "not a package kind: $3" >&2; exit 2 ;;
esac
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
pkg="$dir/axcenity-editor-update.$ext"
cp -- "$1" "$pkg"
chmod 644 "$pkg"
if ! echo "$2  $pkg" | sha256sum -c --status; then
  echo "the package's checksum doesn't match the signed release" >&2
  exit 3
fi
case "$3" in
  rpm)
    if command -v dnf >/dev/null 2>&1; then dnf install -y "$pkg"
    elif command -v zypper >/dev/null 2>&1; then zypper --non-interactive install --allow-unsigned-rpm "$pkg"
    else rpm -U "$pkg"
    fi ;;
  deb) apt-get install -y "$pkg" ;;
  pacman) pacman -U --noconfirm "$pkg" ;;
esac
"#;

/// The name `SCRIPT` sees as `$0`, in `ps` and error messages.
const SCRIPT_NAME: &str = "axcenity-update";

/// The script's argument for a system package kind; `None` for the others.
pub fn kind_arg(kind: InstallKind) -> Option<&'static str> {
    match kind {
        InstallKind::Rpm => Some("rpm"),
        InstallKind::Deb => Some("deb"),
        InstallKind::Pacman => Some("pacman"),
        _ => None,
    }
}

/// `SCRIPT` for `package` under `shell` (`/bin/sh` behind `pkexec` for
/// real; tests run it directly).
fn script_command(shell: &mut Command, kind: &str, package: &Path, sha256: &str) {
    shell
        .arg("-c")
        .arg(SCRIPT)
        .arg(SCRIPT_NAME)
        .arg(package)
        .arg(sha256)
        .arg(kind);
}

/// Installs the checked `package` with the system's package manager as
/// root, asking for the password with `pkexec`. Blocks until it's done;
/// the error is for the user.
pub fn install_system_package(
    kind: InstallKind,
    package: &Path,
    sha256: &str,
) -> Result<(), String> {
    let kind = kind_arg(kind).ok_or("this install isn't a system package")?;
    let mut command = Command::new("pkexec");
    command.arg("/bin/sh");
    script_command(&mut command, kind, package, sha256);
    let output = command.output().map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => {
            "pkexec (polkit) isn't installed, so the editor can't ask for the password to \
             install the package"
                .to_owned()
        }
        _ => format!("couldn't start pkexec: {error}"),
    })?;
    if output.status.success() {
        return Ok(());
    }
    Err(match output.status.code() {
        // pkexec: the prompt was dismissed or the password refused.
        Some(126) => "the password prompt was cancelled".to_owned(),
        Some(127) => format!(
            "the password couldn't be asked for ({})",
            last_line(&output.stderr).unwrap_or("no polkit authentication agent is running")
        ),
        code => {
            let why = last_line(&output.stderr)
                .or_else(|| last_line(&output.stdout))
                .unwrap_or("no reason given");
            match code {
                Some(code) => format!("the package manager failed (status {code}): {why}"),
                None => format!("the package manager was stopped: {why}"),
            }
        }
    })
}

/// The last non-empty line of a program's output.
fn last_line(output: &[u8]) -> Option<&str> {
    std::str::from_utf8(output)
        .ok()?
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
}

/// The AppImage file this editor runs from: AppImages set `$APPIMAGE`.
pub fn running_appimage() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("APPIMAGE")?);
    path.is_absolute().then_some(path)
}

/// Puts the checked `downloaded` AppImage in place of `appimage`: copied
/// beside it as a hidden `.new` file, made executable, then renamed over it
/// (one step, so a failure leaves the old one whole).
pub fn replace_appimage(downloaded: &Path, appimage: &Path) -> io::Result<()> {
    let name = appimage
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no AppImage file name"))?;
    let new = appimage.with_file_name(format!(".{}.new", name.to_string_lossy()));
    let result = (|| {
        fs::copy(downloaded, &new)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&new, fs::Permissions::from_mode(0o755))?;
        }
        fs::rename(&new, appimage)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&new);
    }
    result
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::download::hash_of;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("axcenity-package-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Runs `SCRIPT` without root, on a `PATH` of only the core tools it
    /// needs and fake package managers that record their arguments.
    fn run(dir: &Path, fakes: &[&str], kind: &str, package: &Path, sha256: &str) -> (i32, String) {
        use std::os::unix::fs::PermissionsExt;
        let bin = dir.join("bin");
        let _ = fs::remove_dir_all(&bin);
        fs::create_dir_all(&bin).unwrap();
        for tool in ["mktemp", "cp", "chmod", "sha256sum", "rm"] {
            let real = ["/usr/bin", "/bin"]
                .iter()
                .map(|d| Path::new(d).join(tool))
                .find(|p| p.is_file())
                .unwrap_or_else(|| panic!("no {tool}"));
            std::os::unix::fs::symlink(real, bin.join(tool)).unwrap();
        }
        let called = dir.join("called");
        let _ = fs::remove_file(&called);
        for fake in fakes {
            let path = bin.join(fake);
            // Records its arguments and checks the package it was given
            // exists (the root-side copy, before the script removes it).
            fs::write(
                &path,
                format!(
                    "#!/bin/sh\necho \"{fake} $*\" > \"{}\"\nfor last; do :; done\n[ -f \"$last\" ] || exit 9\n",
                    called.display()
                ),
            )
            .unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let mut shell = Command::new("/bin/sh");
        script_command(&mut shell, kind, package, sha256);
        let status = shell.env("PATH", &bin).status().unwrap();
        let called = fs::read_to_string(called).unwrap_or_default();
        (status.code().unwrap_or(-1), called.trim().to_owned())
    }

    #[test]
    fn a_checked_package_goes_to_the_systems_package_manager() {
        let dir = scratch("managers");
        let package = dir.join("AxcenityEditor-0.4.0-alpha.1-linux-x86_64.rpm");
        fs::write(&package, b"an rpm").unwrap();
        let hash = hash_of(&package).unwrap();

        let (code, called) = run(&dir, &["dnf", "zypper"], "rpm", &package, &hash);
        assert_eq!(code, 0);
        assert!(called.starts_with("dnf install -y /"), "{called}");
        assert!(called.ends_with("/axcenity-editor-update.rpm"), "{called}");
        // Installed from the root-side copy, not the user's download.
        assert!(!called.contains(&dir.display().to_string()), "{called}");

        let (code, called) = run(&dir, &["zypper"], "rpm", &package, &hash);
        assert_eq!(code, 0);
        assert!(
            called.starts_with("zypper --non-interactive install"),
            "{called}"
        );

        let (_, called) = run(&dir, &["apt-get"], "deb", &package, &hash);
        assert!(called.starts_with("apt-get install -y /"), "{called}");
        assert!(called.ends_with(".deb"), "{called}");

        let (_, called) = run(&dir, &["pacman"], "pacman", &package, &hash);
        assert!(called.starts_with("pacman -U --noconfirm /"), "{called}");
        assert!(called.ends_with(".pkg.tar.zst"), "{called}");
    }

    #[test]
    fn a_package_that_doesnt_match_the_manifest_is_never_installed() {
        let dir = scratch("mismatch");
        let package = dir.join("p.rpm");
        fs::write(&package, b"swapped").unwrap();
        let wrong = "0".repeat(64);
        let (code, called) = run(&dir, &["dnf"], "rpm", &package, &wrong);
        assert_eq!(code, 3);
        assert_eq!(called, "");
        let (code, called) = run(&dir, &["dnf"], "flatpak", &package, &wrong);
        assert_eq!(code, 2);
        assert_eq!(called, "");
    }

    #[test]
    fn only_system_packages_have_a_package_manager() {
        assert_eq!(kind_arg(InstallKind::Rpm), Some("rpm"));
        assert_eq!(kind_arg(InstallKind::Deb), Some("deb"));
        assert_eq!(kind_arg(InstallKind::Pacman), Some("pacman"));
        for kind in [
            InstallKind::AppImage,
            InstallKind::LinuxTarball,
            InstallKind::WindowsSetup,
            InstallKind::MacosDmg,
            InstallKind::Source,
        ] {
            assert_eq!(kind_arg(kind), None);
            assert!(install_system_package(kind, Path::new("/x"), "0").is_err());
        }
    }

    #[test]
    fn an_appimage_is_replaced_in_one_step_and_stays_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("appimage");
        let running = dir.join("AxcenityEditor-0.3.0-alpha.1-linux-x86_64.AppImage");
        fs::write(&running, b"old").unwrap();
        let downloaded = dir.join("download.AppImage");
        fs::write(&downloaded, b"new").unwrap();
        replace_appimage(&downloaded, &running).unwrap();
        assert_eq!(fs::read(&running).unwrap(), b"new");
        let mode = fs::metadata(&running).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111);
        // Nothing left beside it.
        let mut names: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        assert_eq!(names.len(), 2, "{names:?}");
    }
}
