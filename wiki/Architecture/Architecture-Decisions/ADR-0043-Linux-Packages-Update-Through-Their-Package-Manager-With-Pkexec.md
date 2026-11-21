---
title: "ADR-0043: Linux system packages update from the editor through their package manager with pkexec; the AppImage replaces its own file"
type: adr
adr: 43
adr_status: Accepted
date: 2026-11-21
supersedes:
superseded_by:
affects: [updater, release, editor]
source: [crates/updater/src/package.rs, crates/updater/src/stage.rs, crates/release/src/install.rs, crates/editor/src/app/update_install.rs]
docs: [docs/research/release-system/architecture.md, docs/project/install.md, docs/editor/shell.md, docs/project/releases.md]
tags: [adr, release, updater, linux]
---

# ADR-0043: Linux system packages update from the editor through their package manager with pkexec; the AppImage replaces its own file

- **Status:** Accepted by Paul (2026-11-21), after 0.3.0-alpha.1: an `.rpm` install was told to download the new package by hand, and Paul asked that every new release update from the editor.
- **Date:** 2026-11-21
- **Affected systems:** `updater` (`package.rs`, `stage::payload`), `release` (`InstallKind::updates_in_place`), the editor's Settings > Updates (`app/update_install.rs`).
- **Revises:** release-system decision D8 and the "Privilege escalation" row of `docs/research/release-system/architecture.md` §9, which left `.deb`/`.rpm`/pacman and the AppImage to a notification and asked for no admin rights at all.

## Context and problem

`axcenity-updater` swaps a per-user install folder (Windows setup, Linux tar.gz) and keeps the old one for Roll back. A `.deb`, `.rpm` or Arch package installs into `/opt/axcenity` as root and belongs to the package manager: swapping that folder would leave the package database wrong (`docs/research/release-system/audit.md`). So those installs, and the AppImage, only got a "download the new package" line. Most Linux users install the distribution package, so most Linux users had no in-app update.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep the notification | No root, nothing new | The complaint: every update is a manual download and install |
| Host an apt/dnf/pacman repository | The distribution's own updater does it, signed by the repository | Infrastructure to host and sign per distribution (against the development policy until needed); still needs the user to add the repository |
| **Download the release's own package, check it against the signed manifest, and run the package manager as root through `pkexec` (chosen)** | No new infrastructure (the packages are already in each release's signed manifest); the package database stays right; the system's own password prompt | Asks for a password; needs polkit and an authentication agent; no Roll back |
| Swap `/opt/axcenity` with a root helper | Roll back like the tar.gz | Corrupts the package database |

## Decision

1. `InstallKind::updates_in_place` is true for `deb`, `rpm`, `pacman` and `appimage`; `updates_itself` (the updater swap, with Roll back) stays the Windows setup and the tar.gz only. `stage::payload` picks the manifest artifact of the install's own kind (roles `deb`, `rpm`, `pacman`, `appimage`).
2. The editor downloads it with the same resumable, size- and SHA-256-checked download as the archives.
3. A system package goes to `pkexec /bin/sh -c <package::SCRIPT> <file> <sha256> <kind>`. As root, the script copies the file into a fresh `mktemp -d` folder, checks the SHA-256 again, and runs `dnf install -y` (else `zypper --non-interactive install --allow-unsigned-rpm`, else `rpm -U`), `apt-get install -y` or `pacman -U --noconfirm` on that copy. The arguments are passed as positional parameters, never put into the script text.
4. An AppImage's new file is copied beside `$APPIMAGE` as a hidden `.new` file, made executable and renamed over it.
5. Both happen while the editor runs on the old files (Linux keeps a running program's replaced files open); **Restart now** writes the updater's `result.json` and opens the new editor with the open project.

## Consequences

- Every Linux release must keep its `.deb`, `.rpm`, `.pkg.tar.zst` and `.AppImage` in the signed manifest under those roles; dropping one stops that kind updating.
- The editor asks for root only when the user pressed Download and install, only for the package manager, and only for a package whose hash the signed manifest gives, checked again on the root side. A same-user program could already call `pkexec` itself, so this adds no new way in.
- No Roll back for these installs; the older package can be installed by hand.
- Copies from 0.3.0-alpha.1 and older don't have this code: their first update is still by hand.
- macOS still gets the notification (unsigned app; Gatekeeper).

## Implementation references

- `crates/updater/src/package.rs`: the root script, `install_system_package`, `replace_appimage`; tests run the script with fake package managers and a wrong hash.
- `crates/editor/src/app/update_install.rs`: `Install::Installed`, `install_in_place`, `restart_into`.
