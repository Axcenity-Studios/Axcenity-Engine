# Patched dependencies

Copies of crates.io releases with small Axcenity fixes, used through
`[patch.crates-io]` in the workspace `Cargo.toml`. They are left out of the
workspace (`exclude = ["vendor"]`), so they aren't linted or formatted as ours.
Each keeps its own license (rapier: Apache-2.0, https://github.com/dimforge/rapier;
egui-wgpu: MIT OR Apache-2.0, https://github.com/emilk/egui).
Every change is marked `AXCENITY PATCH` in the code.

## rapier3d 0.36.1

The release as published on crates.io (`.cargo_vcs_info.json` names its
upstream commit), with one change:

- `src/geometry/narrow_phase/pair_update.rs`: contact recycling (reusing a
  pair's contact points while the pair has barely moved) skipped every pair
  with an active hook. Pairs whose only hook is `MODIFY_SOLVER_CONTACTS` are
  now recycled too, keeping what the hook last set. Axcenity's hook
  (`physics/src/material.rs`) only sets friction and restitution, which
  depend on the two colliders and not on where they are. Without this, parts
  with friction or elasticity weights other than 1 (Ice, Rubber, custom
  `PhysicalProperties`) never kept their contacts, and a stack of ten of them
  toppled where Plastic stood (`physics` test
  `a_stack_of_ice_blocks_stands_like_plastic`).

### Upgrading rapier

Copy the new release from `~/.cargo/registry/src/*/rapier3d-<version>`,
carry the change over (search for `hooks_involved`), and run the physics
tests. Drop the copy once upstream recycles pairs like these.

## egui-wgpu 0.36.2

The release as published on crates.io (`.cargo_vcs_info.json` names its
upstream commit), with one change:

- `src/winit.rs`: `Painter::paint_and_update_textures` always called winit's
  `Window::pre_present_notify` before presenting. It now does so only while
  `egui_wgpu::winit::set_pre_present_notify` says to (on by default, as
  upstream). Under Wayland that call makes winit hold the next
  `RedrawRequested` back until the compositor says the frame was shown, and
  the compositor can't show a frame before the GPU has finished it: with a
  heavy viewport the editor's CPU and GPU took turns, each idle while the
  other worked (Desert Homestead with ray tracing: 27 frames a second, 41
  without the wait, in a debug build). The editor turns the call off while
  its frames miss the display's refresh and back on when they fit in one
  (`EditorApp::pace_display`, `axcenity_render::FramePacer`, ADR-0052).
  On Windows, macOS and X11 the call does nothing either way.

### Upgrading egui

Copy the new release from `~/.cargo/registry/src/*/egui-wgpu-<version>`
(the same version as `eframe`'s), carry the change over (search for
`pre_present_notify`), and run the editor with `AXCENITY_FRAME_LOG=1` on a
heavy project, with and without `AXCENITY_FRAME_CALLBACKS=on`
(`docs/benchmarks.md`). Drop the copy once upstream lets an app choose.
