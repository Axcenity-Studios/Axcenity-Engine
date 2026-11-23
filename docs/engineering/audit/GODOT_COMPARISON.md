# Competitive Comparison: Axcenity Engine vs. Godot 4

**Reference Baseline:** Godot 4.6 (C++, GDScript/C#/GDExtension, Forward+/Mobile/Compatibility, Jolt physics, open source MIT, desktop/mobile/web/consoles).  
**Axcenity Baseline:** Commit `8f4c2420` (`0.3.0-alpha.1`, Rust, Luau, wgpu 30, rapier3d, Apache-2.0).

## Executive Verdict

Godot 4 is Axcenity's **closest technical and philosophical peer** (open-source, single scene/instance tree, lightweight desktop editor, built-in scripting + multiplayer + import pipeline). On **raw 3D simulation speed (XB10k: 5.14 ms vs Godot's 10.27 ms)**, **in-editor 3D/character/terrain studios**, **visual scripting (Blocks ↔ Lua)**, **real-time Collaborate**, and **native MCP AI tooling**, Axcenity already matches or exceeds Godot 4 in scope. However, Godot 4 decisively leads on **stability/QA, platform reach (mobile, web, 2D), native extensibility (GDExtension), audio streaming, in-place `.pck` mounting, documentation freshness, and community ecosystem**.

## Domain-by-Domain Comparison

| Domain | Axcenity | Godot 4 | Evidence & Analysis |
|---|---:|---:|---|
| **3D Performance (XB10k)** | **3** | 3 | Measured XB10k (2026-11-11, Desktop 9950X3D): Axcenity **5.14 ms** vs Godot **10.27 ms**. Axcenity's Rust core + retained draw list + parallel Rapier gives a strong 3D baseline, though Godot has occlusion culling and Jolt physics. |
| **3D Rendering & Lighting** | **3** | 3–4 | Both offer Forward+-class PBR, CSM, probes, volumetric fog, and TAA/FSR1. Axcenity adds Skin/Hair/Cloth shading models, GPU compute posing, software BVH GI, and opt-in wgpu ray queries; Godot adds SDFGI/VoxelGI, clustered lighting, and baked lightmaps. |
| **2D & Platform Reach** | **0** | 4 | Godot has a world-class dedicated 2D engine plus Mobile, Web, and partner Console exports. Axcenity is 3D desktop-only (ADR-0011). |
| **Scripting & Extensibility** | **3** | 4 | Godot offers GDScript (typed, full built-in editor autocomplete), C#, and **GDExtension** (native C/C++/Rust plugins without forking the engine). Axcenity's Luau + Blocks ↔ Lua + hot reload is great for beginners, but lacks a native extension ABI and has weaker built-in editor autocomplete. |
| **In-Box Studios & Collaboration** | **3** | 1–2 | Axcenity ships Model Studio, Character Studio, Terrain erosion/biomes, Behavior Trees, Dialogue/Quests, Collaborate (Yjs + Git + Team Board), and MCP built in. Godot relies on community addons for nearly all of these. |
| **Packaging & Audio** | **2** | 4 | Godot mounts `.pck` archives directly in place (including delta patches) and streams Ogg/MP3 audio. Axcenity unpacks `.axgame` to a temp directory on every launch and decodes all audio into RAM (`StaticSoundData`). |
| **Stability, CI & Ecosystem** | **1** | 4 | Godot has automated cross-OS CI, thousands of contributors, mature docs, and shipped commercial games. Axcenity's `main` has compile/test/clippy regressions and unpatched security bugs. |

## What Axcenity Should Adopt, Improve, or Deliberately Reject from Godot 4

- **Adopt / Adapt**:
  - **In-place archive reading** (Godot `.pck` model) for `.axgame` instead of temp-dir unpacking.
  - **Built-in script editor autocomplete & inline diagnostics** (tighter `luau-lsp` integration across all project files, not only open tabs).
  - **Automated PR/pre-push verification** so trunk stays green.
  - **Native extension ABI (future P3)** inspired by GDExtension once core APIs stabilize.
- **Deliberately Reject**:
  - Do **not** build a separate 2D engine or chase mobile/web exports prematurely (preserve ADR-0011 desktop-first focus).
