---
title: "ADR-0051: A renderer that shows a world keeps undrawn meshes and textures within memory budgets"
type: adr
adr: 51
adr_status: Accepted
date: 2026-11-25
supersedes:
superseded_by:
affects: [render, render-types, player, editor]
source: [crates/render/src/renderer/undrawn.rs, crates/render/src/renderer/texture_budget.rs, crates/render/src/renderer/types.rs, crates/render-types/src/graphics.rs, crates/render/src/video_memory.rs, crates/render/src/adapters.rs, crates/player/src/window.rs, crates/editor/src/viewport.rs, crates/editor/src/app/settings/graphics.rs]
docs: [docs/rendering/models.md, docs/rendering/performance.md, docs/rendering/settings.md, docs/architecture/streaming.md]
tags: [adr, rendering, memory, performance]
---

# ADR-0051: A renderer that shows a world keeps undrawn meshes and textures within memory budgets

- **Status:** Accepted (Paul, 2026-11-25)
- **Date:** 2026-11-25
- **Affected systems:** render, the player's window, the editor's viewport

## Context and problem

The renderer freed a caller's mesh or texture once it had gone undrawn for 600 frames and 10 seconds, and uploaded it again when it was next drawn, in that frame. In Desert Homestead's tour (shots of about 10 seconds) every camera cut stalled one frame: 30 to 105 ms at 2560 × 1440 on the Desktop under Fedora. Two cuts go back to a view already seen, and those cost 105 and 75 ms only because what they show had been freed (`docs/rendering/performance.md`, "Kept within a budget"). The scene holds 1,579 MiB of textures and 174 MiB of meshes at full size, on a card with 16 GB.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep the 10-second rule | Memory in use follows what is on screen | A stall whenever something comes back after 10 seconds, however much memory is free |
| Upload on a thread in the player too (as while streaming, and in the editor) | No stall, on first sights either | A mesh is missing and a texture white for a few frames after a cut: Paul judged that worse than a stall |
| A longer time (a minute, ten) | One number | The stall comes later, not never; memory still isn't what decides |
| **Keep what fits a budget; past it, free the least recently drawn of what isn't being drawn** | No upload on coming back while there is room; memory is bounded by a number the settings already have | Memory in use grows toward the budget and stays there; a first sight still uploads in its frame |

## Decision

A renderer told to (`Renderer::set_keep_undrawn`, `TextureRenderer::set_keep_undrawn`; the player's window and the editor's viewport) keeps meshes while they fit `EffectiveGraphics::mesh_budget` and color textures while they fit `texture_budget`. Past a budget, what has gone undrawn for 600 frames and 10 seconds is freed, the least recently drawn first and no more than it takes; what is being drawn is never freed, and textures still over their budget lose their largest levels as before. Every other renderer keeps the 10-second rule.

The budgets follow the card's memory (Paul, the same day: "2 GB seems too low for really any high fidelity application"): with no texture memory budget set it is a quarter of the card's memory, as a power of two between 256 MiB and 8 GiB, wherever that is more than the preset's, and the settings say when a world needs more than the budgets.

## Reasons

- Paul's choice of 2026-11-25: a white or missing model after a cut breaks the picture more than one stall on a first view, and once a tour's views have been seen its cuts should be free.
- Measured: the two cuts back to a seen view went from 105 and 75 ms to 12–14 and 21–23 ms; the first sights are unchanged (30–62 ms).
- The texture memory budget already existed, with its own setting and its own way to make room (dropping levels). Meshes get a budget derived from it (`MESH_BUDGET_SHARE`: half), so there is no new setting.
- It is opt-in for a renderer because the editor has several (thumbnails, Character Studio's preview, the probe and movie renderers), each with its own caches: each filling a budget would multiply the memory held.

## Consequences

- GPU memory in use no longer falls when the camera turns away: it grows toward the budgets as a world is seen, and stays. The budgets are the bound, so they have to suit the card: a quarter of its memory for textures and an eighth for meshes (4,096 and 2,048 MiB on a 16 GB card), and never less than the presets' own (textures Low 256, Medium 512, High 1024, Ultra 2048 MiB; meshes half that), which a card that doesn't say its memory gets. A fixed higher number was not taken: Automatic is High on every card, a 4 GB one included.
- The card's memory is read from the system, not from wgpu (which doesn't report it): the registry on Windows (two `reg` queries, once per card, when the first renderer is made; not run on Windows in this change) and amdgpu's sysfs file on Linux. NVIDIA and Intel cards on Linux don't say, and keep the presets' budgets until someone sets one.
- A mesh or texture whose key will never be drawn again (a terrain chunk's mesh from before an edit, a texture that was replaced) stays until its budget is full, then leaves first, being the oldest.
- Materials, UI images and terrain weights still leave after 10 seconds: small, quick to make again, and some are new every frame.
- Ray tracing's acceleration structures and the software BVHs keep their own timers: with ray tracing on, a mesh that was kept still has its structure built again on coming back.
- Revisit if memory held by long sessions becomes a problem in the editor, or if a quarter proves wrong for cards that share their memory with the system.

## Implementation references

- `crates/render/src/renderer/undrawn.rs` (`to_free`, `free_undrawn_textures`, `forget_undrawn`), called from `ScenePipeline::prepare` before `fit_texture_budget` and at the end of a frame
- `crates/render-types/src/graphics.rs` (`EffectiveGraphics::mesh_budget`, `MESH_BUDGET_SHARE`, `texture_budget_for`, `GraphicsSettings::texture_budget_on`, `Capabilities::video_memory`), `crates/render/src/video_memory.rs`, `crates/render/src/adapters.rs` (`opened_card_memory`)
- `crates/editor/src/app/settings/graphics.rs` (the Texture memory row and the warning), `RenderStats::memory_warnings`
- `crates/player/src/window.rs`, `crates/editor/src/viewport.rs` (turned on)
- Tests: `renderer::undrawn::tests`; `renderer/tests/streaming.rs` (`a_kept_mesh_stays_however_long_it_goes_undrawn_while_meshes_fit_their_budget`, `past_the_mesh_budget_the_least_recently_drawn_undrawn_meshes_leave_first`, and the 10-second rule's `what_stops_being_drawn_leaves_the_gpu_and_uploads_again_on_the_way_back`); `renderer/tests/texture_budget.rs` (`a_kept_texture_undrawn_a_long_time_stays_within_the_budget_and_leaves_whole_past_it`); `render-types` (`the_texture_budget_follows_the_cards_memory_where_that_is_more_than_the_presets`); the editor's `the_graphics_page_says_when_the_world_needs_more_than_the_memory_budgets`
