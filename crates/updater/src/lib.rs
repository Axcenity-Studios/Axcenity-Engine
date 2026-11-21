//! Updating an installed Axcenity editor (release system Phase 6;
//! `docs/research/release-system/architecture.md` §7).
//!
//! - [`download`]: the payload, resumable and checked against the signed
//!   manifest's size and SHA-256.
//! - [`extract`]: unpacking it without letting anything out of its folder.
//! - [`stage`]: the editor's side: the payload this install needs, staging
//!   it, and handing over to the new release's `axcenity-updater`.
//! - [`apply`]: what `axcenity-updater` does after the editor exits: swap
//!   the install folder, check the new editor, roll back on failure.
//! - [`package`]: the Linux system packages and the AppImage, which update
//!   in place instead: the package manager through `pkexec`, or a rename.
//!
//! The Windows setup and the Linux `.tar.gz` go through `axcenity-updater`;
//! `.deb`, `.rpm`, Arch packages and the AppImage through [`package`]. Only
//! the macOS disk image and source builds point to the release page.

pub mod apply;
pub mod download;
pub mod extract;
pub mod package;
pub mod stage;

#[cfg(test)]
mod tests;
