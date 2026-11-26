---
title: "ADR-0052: A frame starts while the GPU draws the last when frames miss the display's refresh"
type: adr
adr: 52
adr_status: Proposed
date: 2026-11-25
supersedes:
superseded_by:
affects: [render, player, editor, vendor]
source: [crates/render/src/pacing.rs, crates/render/src/renderer/scene.rs, crates/render/src/renderer.rs, crates/player/src/window.rs, crates/player/src/main.rs, crates/editor/src/app.rs, crates/editor/src/app/viewport_bar.rs, crates/editor/src/viewport.rs, crates/core/src/bench.rs, vendor/egui-wgpu/src/winit.rs]
docs: [docs/rendering/performance.md, docs/benchmarks.md, vendor/README.md]
tags: [adr, rendering, performance, frame-pacing, wayland]
---

# ADR-0052: A frame starts while the GPU draws the last when frames miss the display's refresh

- **Status:** Proposed (written with the change on 2026-11-25). Paul reviewed it twice that day: he agrees with the approach, chose to keep a window without focus waiting for the display, and will accept once the platform and window-state validation is written down. It now is (`docs/benchmarks.md`, "Checking frame pacing on a platform": every check run under Wayland; Windows, macOS and X11 not run, with what to run there). His to accept
- **Date:** 2026-11-25
- **Affected systems:** render, the player's window, the editor's frame, the vendored egui-wgpu

## Context and problem

Paul's report of 2026-11-25: Desert Homestead with every ray traced effect on "is still significantly underutilizing my hardware". Measured on the Desktop under Fedora (GNOME on Wayland, radv, a 240 Hz display), release player at the editor's settings and viewport size (1996 × 998):

- A frame was shown every 40.8 ms (24.5 a second), where the CPU's work was 15.4 ms and the GPU's 24.7.
- The card was **59% busy**, drawing 70 W of the 304 W it may, with its core clock at about 1,510 MHz. The frame's thread was at 96% of a core, though its steps took 15 ms of each 41 (what it did with the rest wasn't sampled).
- The cause is in two libraries' sources. Under Wayland, winit's `Window::pre_present_notify` asks the compositor for a frame callback and holds `RedrawRequested` back until it comes (`winit/src/platform_impl/linux/wayland/event_loop/mod.rs`). The compositor can't show a frame, or say it did, before the GPU has finished drawing it. The player called `pre_present_notify` each frame, and egui-wgpu calls it unconditionally for the editor. So frame N+1's CPU work started only after frame N's GPU work was done and shown: the two took turns, each idle while the other worked, and the GPU's clock fell with its load, which stretched its own work to nearly twice what it takes under load.
- With the call left out, the same player on the same view: **61.8 frames a second**, the card 85% busy at 2,950 MHz and 221 W, and the GPU's frame **13.7 ms**.
- Starting early has a cost of its own. With the GPU the slower side (the same view in a 2560 × 1405 window, asked for as 3840 × 2160, which GNOME doesn't give), frames queued in the swapchain: each `get_current_texture` waited 3 ms, which wgpu does for the submission that last used the picture it hands out, as many frames back as the swapchain has pictures. That is several frames of GPU work between input and picture.
- On Windows, macOS and X11 `pre_present_notify` does nothing: there the engine has always started the next frame at once.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep waiting for the display (as it was) | Shortest path from input to picture; nothing drawn that isn't shown; no work | 24 frames a second where 62 are possible; the GPU at half its clock; each side idle for the other's turn |
| Never wait for the display | One rule; what Windows already does | A light game on a 60 Hz display under Wayland would queue frames in the swapchain (two or three refreshes more from input to picture) for nothing: its frames fit in a refresh either way |
| Wait while frames fit in a refresh, start early when they don't (**chosen**) | Light scenes keep the display's pacing; heavy ones get both sides working at once | A policy to keep right (`FramePacer`); a second of the slower pace before it switches; needs the editor's painter patched |
| Draw the editor's viewport one frame late (submit its work after the frame is presented) | No patched dependency | Every viewport frame shown a frame late on every platform, an idle gap of up to a refresh each frame, and a stale picture whenever the editor stops drawing |
| Start the CPU's work just in time for the GPU to be free (as latency reducers do) | Frame rate of starting early with the latency of waiting | Needs each side's time predicted; a refinement of the chosen option, not instead of it |

## Decision

A host that shows frames asks `axcenity_render::FramePacer` each frame whether the next one waits for the display. It does at first. After 8 frames in a row at least 1.5 refreshes long it stops waiting, and after 90 in a row light enough to take turns within a refresh (the GPU never waited for, the CPU's work at most half a refresh) it waits again. A renderer whose frames are shown is **one frame ahead** (`TextureRenderer::set_one_frame_ahead`, always on for a window's `Renderer`): after submitting a frame it waits for the frame before the last to finish on the GPU, so the GPU always has a frame to go on with and never a third behind it.

The player calls `pre_present_notify` only while the pacer says to wait. The editor does the same through a switch added to a vendored copy of egui-wgpu (`egui_wgpu::winit::set_pre_present_notify`, `vendor/README.md`).

Added the same day, after Paul's review asked for hidden and unfocused windows and latency to be looked at:

- **A window without focus always waits for the display** (`FramePacer::waits_for_display_when`). A minimized or covered window never has focus, and under Wayland nothing else says a window is hidden: started early, a minimized heavy scene went on drawing at full load (256 W).
- **A hidden game keeps stepping, undrawn.** A redraw unanswered for a quarter of a second means nothing is showing the window: the player then runs its frames 30 times a second without drawing them until a redraw comes (`player/src/window.rs`, `next_frame`). A frame the window couldn't show (minimized on Windows, covered on macOS) spaces out the next the same way. Before, such a game stopped altogether under Wayland, or ran as fast as its loop turned.
- **The display's refresh is asked for again as frames go by** (every 30 shown frames in the player, `window::asks_refresh_on`; the editor already did). Under Wayland a new window isn't on a display yet, so the first answer was "unknown", taken as 60 Hz: on a 240 Hz display frames 17 to 25 ms long then never counted as missing the refresh, and a heavy game in a 1000 × 600 window was shown at 43.8 frames a second where it draws 97.9 (found and fixed in the third round, 2026-11-25).
- **Paul's choice for the window seen without focus** (2026-11-25): keep it waiting for the display. "I prefer reduced GPU consumption over running minimized heavy scenes at full speed."
- **The event loop sleeps until the redraw comes** (`ControlFlow::Wait` with a deadline) where it used to turn over (`Poll`): the frame's thread was at 96% of a core doing 15 ms of work in each 41.

## Reasons

- Measured, the same view and picture: 24.5 → 62 frames a second in the player (66.6 once the memory read was fixed too), 20.7 → 53.0 in a 2560 × 1405 window, 26 → 41 in the debug editor. No setting, shader or resolution changed.
- Waiting costs nothing while the CPU's and the GPU's work fit in one refresh together, and is then the lowest latency there is, so that case is left as it was. Most small games are that case.
- One frame ahead bounds what starting early can add to latency at one frame of GPU work, whatever the swapchain's depth. It applies on every platform, which makes Windows' queue shallower too when the GPU is the slower side.
- The repository already keeps a patched dependency this way (`vendor/rapier3d`), and the patch is three lines that default to upstream's behavior.

## Consequences

- **Latency**, measured (`AXCENITY_BENCH_LATENCY`, a frame's start to its picture finished on the GPU; the tour as played, medians): 38.7 ms waiting for the display against **27.6** paced at 1996 × 998, and 47.1 against **35.6** in a 2560 × 1405 window, where the GPU is the slower side and frames queue. There the limit of one frame ahead is worth 6.5 ms at the same frame rate (42.1 without it). A light scene is 3.0 ms either way. So pacing shortens the way from input to picture as well as raising the frame rate; what the display adds after (up to a refresh) wasn't measured and is the same either way.
- **Power.** A heavy scene now runs the card near its limit (221 W against 70 here). That is the hardware being used; a frame rate cap is the tool for anyone who wants less.
- **A dependency to carry.** `vendor/egui-wgpu` is copied again with every egui upgrade until upstream lets an app choose.
- **Not run on Windows or macOS.** From their sources (winit 0.30.13 and wgpu-hal 30.0.1, the versions in `Cargo.lock`, read 2026-11-25): `pre_present_notify` is empty on both (`winit/src/platform_impl/windows/window.rs:161`, `macos/window_delegate.rs:921`), so the pacer's choice changes nothing there. One frame ahead does apply: `device.poll` for a submission waits on the fence with an event on DirectX 12 (`wgpu-hal/src/dx12/device.rs:2361`) and a condition variable on Metal (`metal/device.rs:2017`), with its timeout on both. A minimized Windows window sends `Resized(0, 0)` (`windows/event_loop.rs:1397`) and its frames are skipped; a covered or minimized macOS window's `get_current_texture` returns `Occluded` (`metal/surface.rs:368`); both are now spaced out. Nothing tells a covered Windows window it is covered. `docs/benchmarks.md`, "Checking frame pacing on a platform", is the list to run on each.
- **A hidden window**, run under Wayland (GNOME 51, Mesa 26.2). Minimized (`AXCENITY_BENCH_MINIMIZE_AFTER`): a light scene and a heavy one both keep stepping (0.3% and 2% of a core, the GPU idle). Wholly covered by another window (run in the third round, a second game opened over the first): 0.4 s after it was covered the game stopped drawing and stepped 30.0 times a second with no GPU use of its own, and drew at full pace again within 0.1 s of being uncovered. `AXCENITY_FRAME_CALLBACKS=on` makes the player or the editor wait every frame, as a way back.
- **A window seen without focus** draws a heavy scene at the waiting pace: run the same way, 30.5 frames a second where 86 to 94 were drawn with the focus, the card at 82 W, and the full pace back within 0.1 s of the focus. That is the price of knowing nothing better about a hidden window under Wayland, and Paul chose to keep paying it; a platform that reports occlusion could do without it.
- **Validation, platform by platform**: `docs/benchmarks.md`, "Checking frame pacing on a platform", has each check with what it should show and a table of what has been run. Under Wayland all of them have. On Windows, macOS and X11 none has, by anyone: what is known of those is the sources' reading above.
- **Revisit** if upstream egui makes the call optional (drop the copy), if a just-in-time start is wanted for latency, or if a compositor is found where starting early stalls.
- **To consider later: a background rendering policy that can be set** (Paul, 2026-11-25). What a focused, an unfocused and a hidden window do is fixed in code today. A setting for each (full pace, the display's pace, a frame rate cap, steps only, paused), with a game's needs and the person's wishes both heard, is sketched in `docs/architecture/frame-loop.md`. It would replace the fixed rule for a window without focus, and wants a frame rate cap beside it. Not built, and nothing here depends on it.

## Implementation references

- `crates/render/src/pacing.rs`: `FramePacer`, `PacedFrame`, and their tests (light frames wait; frames that keep missing the refresh start early; one slow frame doesn't; light again waits again; the display's own refresh)
- `crates/render/src/renderer/scene.rs`: `ScenePipeline::wait_for_frame_before`; `crates/render/src/renderer/tests/frames_ahead.rs`
- `crates/player/src/window.rs` (`paced`, `refresh_of`, `asks_refresh_on`, `bench_as_played`, `next_frame` and its tests), `crates/player/src/main.rs` (`App::frame`)
- `crates/render/src/renderer/latency.rs` (`LatencyWatch`), the measurement behind the latency figures
- `crates/editor/src/app.rs`: `EditorApp::pace_display`, `DisplayPace`, `paced_refresh`
- `vendor/egui-wgpu/src/winit.rs`: `set_pre_present_notify` (marked `AXCENITY PATCH`)
- `crates/core/src/bench.rs`: `BenchRun::time_between_frames`, the measurement that shows either way's frame rate
- `docs/rendering/performance.md`, "The CPU and the GPU took turns"
