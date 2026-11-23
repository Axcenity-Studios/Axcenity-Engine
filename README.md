<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/branding/wordmark-white.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/branding/wordmark.png">
    <img alt="Axcenity" src="assets/branding/wordmark.png" width="360">
  </picture><sup>™</sup>
</p>

<p align="center">
  <strong>A free, open-source 3D game engine and editor written in Rust.</strong><br>
  <em>Axcenity</em> is pronounced <strong>ax-SEN-ih-tee</strong>.
</p>

<p align="center">
  <a href="https://axcenity.com">Website</a> ·
  <a href="https://axcenity.com/docs">Documentation</a> ·
  <a href="https://axcenity.com/discussions">Forums</a> ·
  <a href="https://github.com/MrBearson321/Axcenity-Engine/releases">Releases</a>
</p>

---

Axcenity™ Engine is a cross-platform 3D game engine and editor inspired by Roblox Studio. Build worlds, script them in **Luau** or with **visual blocks**, test directly in the editor, and export standalone games for **Windows, macOS, and Linux**.

> **Status:** Early development (alpha). Expect bugs and breaking changes between releases. For full feature overviews, guides, and API documentation, visit [axcenity.com](https://axcenity.com) and [axcenity.com/docs](https://axcenity.com/docs). Join the community on the [forums](https://axcenity.com/discussions).

## Supported hardware

Axcenity renders with **wgpu** using **DirectX 12** on Windows (optional Vulkan), **Vulkan** on Linux, and **Metal** on macOS.

| | Minimum | Recommended | High end |
|---|---|---|---|
| **Operating system** | Windows 10 64-bit, Linux 64-bit (glibc 2.35+, e.g. Ubuntu 22.04), macOS 11 Big Sur | Windows 11 or a current Linux distribution | Windows 11 or a current Linux distribution |
| **Processor** | 64-bit, 4 cores, AVX (or Apple silicon) | 6–8 cores (Ryzen 5 5600 / Core i5-12400 class) | 8+ fast cores (Ryzen 7 7800X3D / Core i7-13700K class) |
| **Memory** | 8 GB RAM | 16 GB RAM | 32 GB+ RAM |
| **Graphics** | DirectX 12 (FL 11_0+), Vulkan, or Metal GPU with 2 GB VRAM (integrated graphics supported) | Dedicated GPU with 8 GB VRAM (RX 6600 / RTX 3060 / Arc A750 class) | 12–16 GB VRAM (RX 9070 XT / RTX 4070 Ti Super or better) |
| **Storage** | 1 GB free | SSD with 20 GB free | NVMe SSD with 50 GB free |
| **Display** | 1280 × 720 | 1920 × 1080 | 2560 × 1440 or higher |

- **Platforms:** Windows 10/11 (x86-64) and Linux (x86-64, X11 or Wayland) are the primary platforms; macOS 11+ (universal Apple silicon and Intel) ships with every release. Dedicated servers run headless on Windows and Linux.
- **Not currently supported:** 32-bit systems, Windows/Linux on Arm, Windows 7/8, mobile (Android, iOS), web browsers, and game consoles.
- **Ray tracing (optional):** Groundwork is in place for DXR 1.1 (Windows), `VK_KHR_ray_query` (Linux), and macOS 15+ (Apple M3+), and is never required to run the engine.

For full details, graphics preset budgets, and network requirements, see [System requirements](documentation/reference/system-requirements.md).

## Getting the engine

Prebuilt alpha releases are available from [axcenity.com](https://axcenity.com) and [GitHub Releases](https://github.com/MrBearson321/Axcenity-Engine/releases):

- **Windows (x86-64):** `.exe` setup wizard
- **macOS (Apple silicon & Intel):** universal `.dmg`
- **Linux (x86-64):** `.deb`, `.rpm`, `.pkg.tar.zst`, portable `.tar.gz`, and `AppImage`

Each editor download bundles the game player for Windows, macOS, and Linux so you can export standalone games for any supported desktop platform.

> Alpha builds are not code-signed. See [docs/project/install.md](docs/project/install.md) for platform-specific installation notes, checksum verification, and updating instructions.

## Building from source

You only need to build from source if you want to modify or contribute to the engine itself. Plan for **16 GB RAM** (8 GB works with `cargo build -j 4`) and roughly **30 GB of free disk space** for build artifacts.

### Prerequisites

1. **Rust:** Install [rustup](https://rustup.rs). The repository's [`rust-toolchain.toml`](rust-toolchain.toml) pins the exact Rust version and `rustup` installs it automatically on the first build.
2. **C/C++ compiler & system libraries** (needed for Luau and native dependencies):

   - **Windows 10 / 11:** [Visual Studio 2022](https://visualstudio.microsoft.com/downloads/) (17.11 or later) or its Build Tools with:
     - the **Desktop development with C++** workload
     - the individual component **C++ ATL for latest v143 build tools (x86 & x64)**
     - `curl` and `tar` (included with Windows 10 and 11) to download the DXC shader compiler during the build.
   - **macOS:** Xcode Command Line Tools:
     ```bash
     xcode-select --install
     ```
   - **Linux:** A C/C++ compiler, `pkg-config`, development headers for ALSA, libudev, D-Bus, libxkbcommon, and Wayland, plus a Vulkan driver:

     | Distribution | Command |
     |---|---|
     | Fedora | `sudo dnf install gcc gcc-c++ pkgconf-pkg-config alsa-lib-devel systemd-devel dbus-devel libxkbcommon-devel wayland-devel mesa-vulkan-drivers vulkan-loader` |
     | Ubuntu / Debian | `sudo apt install build-essential pkg-config libasound2-dev libudev-dev libdbus-1-dev libxkbcommon-dev libwayland-dev mesa-vulkan-drivers` |
     | Arch | `sudo pacman -S --needed base-devel alsa-lib systemd-libs dbus libxkbcommon wayland vulkan-icd-loader` plus your GPU driver (`vulkan-radeon`, `vulkan-intel`, or `nvidia-utils`) |

### Build and run

```bash
git clone https://github.com/MrBearson321/Axcenity-Engine.git
cd Axcenity-Engine

# Run the editor (opens the start screen, or pass a project folder)
cargo run -p axcenity-editor
cargo run -p axcenity-editor -- examples/hello-touched

# Run the standalone player with an example project
cargo run -p axcenity-player -- examples/hello-part
```

Common workspace commands:

| Task | Command |
|---|---|
| Build everything | `cargo build --workspace` |
| Release build | `cargo build --workspace --release` |
| Run tests | `cargo nextest run --workspace` (or `cargo test --workspace`) |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format | `cargo fmt --all` |
| Update workspace-hack after dependency changes | `cargo hakari generate` |

See [docs/development.md](docs/development.md) for development setup details and tips for faster local builds.

## License and trademark

### License

Axcenity Engine is licensed under the [Apache License 2.0](LICENSE).

- **Your games are yours.** Games you create with Axcenity belong entirely to you—no royalties and no fees. Sell or distribute them however you choose. Exported games automatically include a credits file containing the engine's [NOTICE](NOTICE).
- **Forks are welcome.** You may fork and modify the engine freely under the Apache License 2.0. Keep the `LICENSE` and `NOTICE` files, mark any files you modify, and distribute your fork under a different name.

### Pending trademark

**Axcenity™** and **Axcenity Engine™** (the word marks) are pending trademarks of Paul Janota. Trademark protection is claimed **only for the words**, not for the Axcenity geometric logo mark. The Apache License 2.0 does not grant permission to use the Axcenity name or trademarks, except as required for reasonable and customary use in describing the origin of the work and reproducing the content of the `NOTICE` file.
