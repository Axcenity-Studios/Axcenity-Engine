# Dependency Register (`8f4c2420`)

All third-party dependencies are centrally pinned in root [`Cargo.toml`](../../../Cargo.toml) (`[workspace.dependencies]`) and unified across crates via `crates/workspace-hack` (`cargo-hakari`). All licenses are Apache-2.0 compatible.

## Core Runtime & Engine Dependencies

| Crate | Version | Scope | License | Purpose & Risk Notes |
|---|---:|---|---|---|
| `wgpu` | `30` (`static-dxc`) | Render / Player / Editor | MIT / Apache-2.0 | GPU backend (DX12, Vulkan, Metal). Locked in version step with `eframe`, `glyphon`, `egui_dock`. Uses experimental ray queries via `unsafe`. |
| `winit` | `0.30` | Player / Editor | Apache-2.0 | Cross-platform windowing & input. |
| `rapier3d` | `0.36` (**patched**) | Physics | Apache-2.0 | Patched in `vendor/rapier3d` (preserves contact persistence when contact hooks only modify friction/restitution). Maintenance burden on upstream upgrades. |
| `mlua` + `luau0-src` | `0.12` / `=0.22.0` | Script / Plugins | MIT | Embedded Luau VM + optional JIT (`--!native`). **Strict pin**: `luau0-src 0.22.1` emits bytecode v13 which `mlua-sys 0.13.0` rejects (guarded by `luau_pin_tests.rs`). Requires C++ compiler. |
| `kira` | `0.12` | Audio | MIT / Apache-2.0 | Spatial audio, buses, DSP effects. Requires ALSA + D-Bus dev headers on Linux. |
| `glam` | `0.34` | Workspace | MIT / Apache-2.0 | Linear algebra math. |
| `taffy` | `0.7` | UI | MIT | Flexbox/grid layout engine (currently only flex used). |
| `glyphon` | `0.12` | Render / UI | MIT / Apache-2.0 | GPU text rendering (locked to `wgpu` 30). |
| `gilrs` | `0.11` | Player / Editor | Apache-2.0 / MIT | Gamepad input (requires `libudev-dev` on Linux). |
| `rayon` | `1.10` | Physics / Anim / Render / Cook | MIT / Apache-2.0 | Data-parallel worker pools. |
| `profiling` | `1` | Workspace | MIT / Apache-2.0 | Zero-cost span macros (`profile-tracy` opt-in). |

## Asset, Compression & Serialization Dependencies

| Crate | Version | Scope | License | Purpose & Risk Notes |
|---|---:|---|---|---|
| `gltf` | `1.4` | Assets | MIT / Apache-2.0 | glTF 2.0 / GLB loader (`KHR_texture_transform`, `extras`). |
| `ufbx` | `0.11` | Assets | MIT / Unlicense | C library compiled via `cc`; loads FBX and OBJ+MTL. |
| `meshopt` | `0.7` | Assets / Mesh Edit | MIT / Apache-2.0 | C++ `meshoptimizer` via `cc`; offline LOD simplification. |
| `openusd` + `openusd-schemas` | `=0.8.0` | Convert (**editor-only**) | MIT | Pure-Rust USD reader (exact pin; early `0.8` API). |
| `intel_tex_2` | `0.5` | Cook (**editor-only**) | MIT / Apache-2.0 | Prebuilt ISPC BC7/BC5 encoder (x86_64/arm64 desktop). |
| `texture2ddecoder` | `0.1.2` | Assets / Player | MIT / Apache-2.0 | Pure-Rust CPU BC7/BC5 decoder for cooked-only games. |
| `image` / `png` | `0.25` / `0.18` | Assets / Editor / Player | MIT / Apache-2.0 | Image decoding (BMP, EXR, HDR, JPEG, PNG, TGA). |
| `serde` / `serde_json` / `toml` | `1` / `1` / `1` | Workspace | MIT / Apache-2.0 | Text project, save, and release serialization. |
| `postcard` | `1.1` | Net | MIT / Apache-2.0 | Binary wire framing for multiplayer snapshots/RPCs. |
| `zstd` | `0.13` | Project / Collab | MIT / BSD | C `zstd` via `cc`; `.axgame` compression and collab snapshots. |
| `zip` / `tar` / `flate2` | `2` / `0.4` / `1` | Project / Toolbox / Updater | MIT / Apache-2.0 | Archive handling (note: zip-bomb bounds missing in 4 unpackers). |

## Networking, Collaboration & Security Dependencies

| Crate | Version | Scope | License | Purpose & Risk Notes |
|---|---:|---|---|---|
| `renet` / `renet_netcode` / `renetcode` | `2` | Net | MIT / Apache-2.0 | UDP reliable/unreliable channels + ChaCha20-Poly1305 netcode tokens. |
| `rustls` / `ring` / `rcgen` | `0.23` / `0.17` / `0.14` | Net / Collab / Toolbox | Apache-2.0 / MIT / ISC | TLS 1.3 transport, Ed25519 keys, self-signed pinned certs. |
| `tungstenite` | `0.28` | Collab | MIT / Apache-2.0 | Sync WebSocket framing over `rustls`. |
| `yrs` | `0.28` | Collab | MIT | Rust Yjs CRDT for live collaborative Lua/CSS editing. |
| `ureq` | `3` (`rustls`) | Toolbox / Updater / Repo | MIT / Apache-2.0 | Sync HTTPS client (no OpenSSL dependency). |
| `minisign-verify` / `sha2` / `md-5` | `0.2` / `0.11` / `0.11` | Release / Updater / Toolbox | MIT / Apache-2.0 | Release manifest signature verification, SHA-256, Poly Haven MD5 integrity. |

## Editor, Tooling & Platform Dependencies

| Crate | Version | Scope | License | Purpose & Risk Notes |
|---|---:|---|---|---|
| `eframe` / `egui_dock` / `egui-phosphor` | `0.36` / `0.21` / `0.14` | Editor-only | MIT / Apache-2.0 | Editor UI shell & docking. **Ecosystem lockstep** with `wgpu` 30. |
| `rmcp` + `tokio` | `3.9` / `1.53` | MCP (**editor-only**) | Apache-2.0 / MIT | Official MCP SDK; `tokio` runs only on MCP thread. |
| `rusqlite` | `0.41` (`bundled`) | Toolbox (**editor-only**) | MIT / Public Domain | Bundled SQLite C build for central asset metadata DB. |
| `notify` / `rfd` / `memory-stats` | `8.2` / `0.17` / `1.2` | Project / Editor | CC0 / MIT / Apache-2.0 | File watching, native dialogs, RSS memory stats. |
| `muda` / `objc2*` | `0.21` / `0.6` / `0.3` | Editor (macOS only) | MIT / Apache-2.0 | Native macOS menu bar & titlebar behavior. |

## Build & Supply-Chain Risk Summary

1. **C/C++ Native Toolchain Requirement**: Building from source requires a C and C++ compiler (`luau0-src`, `ufbx`, `meshopt`, `zstd`, `rusqlite`) plus `libasound2-dev`, `libdbus-1-dev`, `libudev-dev` on Linux and Visual Studio 17.11+ ATL (`atls.lib`) for `static-dxc` on Windows.
2. **Vendored Fork**: `vendor/rapier3d` must be rebased carefully on every Rapier upgrade.
3. **CI Packaging Supply-Chain Gap**: `.github/workflows/release.yml:391-395` downloads `nfpm` and `appimagetool` (mutable `continuous` tag) via `curl -sSL` with **no SHA-256 verification**, and `pip install --upgrade pillow` / `dmgbuild` are unpinned.
4. **No `cargo-deny` / `cargo-audit` Automation**: Dependency security advisories and license checks are manual due to ADR-0018.
