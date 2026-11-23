# Master Feature Matrix (`8f4c2420`)

**Scale:** `0` Missing · `1` Experimental · `2` Partial · `3` Implemented · `4` Production-ready.  
Axcenity scores are **verified directly from source code and tests** at `8f4c2420`. Peer scores reflect current production baselines (Unreal Engine 5.7, Unity 6.3 LTS, Godot 4.6, Roblox Studio 2026) and are marked **Claimed / Reference**.

| Capability Domain | Axcenity (`8f4c2420`) | Unreal 5 | Unity 6 | Godot 4 | Roblox Studio | Evidence / Key Axcenity Limitations | Confidence |
|---|---:|---:|---:|---:|---:|---|---|
| Core Scene / Entity Model | **3** | 4 | 4 | 4 | 4 | Generational arena + Instance tree (`core`), class/tag index, 10-stage schedule (`sim`). Deliberately no ECS (ADR-0002). | High |
| Scripting & Language Ecosystem | **3** | 4 | 4 | 4 | 4 | Sandboxed Luau + `--!native` JIT + hot reload (`script`). No Parallel Luau, no per-script RAM cap, no native C/Rust extension API, thin in-editor autocomplete. | High |
| Visual Scripting | **3** | 4 | 3 | 1 | 0 | Blocks ↔ Lua round-trip, Script/UI/BT/Dialogue node graphs (`blocks`, `blocks-runtime`, `editor-node-graph`). Shallower than Blueprints. | High |
| 3D PBR & Tiled Lighting | **3** | 4 | 4 | 4 | 3 | `wgpu` 30 tiled forward+ (1,024 lights), 47 WGSL shaders, Skin/Hair/Cloth models, `naga` surface shaders. No render graph, no pipeline cache, no Eye shader. | High |
| Shadows & Global Illumination | **3** | 4 | 4 | 3 | 3 | 4-cascade CSM, spot/point/contact shadows, baked & realtime GI probes, CPU-BVH software GI, opt-in HW RT (disabled by default due to DX12 RX 9070 XT bug). | High |
| Geometry Scale (LOD / GPU-Driven) | **2** | 4 | 4 | 3 | 3 | Offline `meshopt` LODs, frustum/distance culling, CPU-planned instancing. No Nanite/GPU Resident Drawer/occlusion culling. | High |
| Post-Processing & Upscaling | **3** | 4 | 4 | 3 | 3 | Bloom, LUT grading, DoF, motion blur, TAA/TAAU, FSR1-style EASU/RCAS. No DLSS/XeSS/FSR3/TSR. | High |
| Physics & Destruction | **3** | 4 | 4 | 4 | 4 | Patched `rapier3d` 0.36, joints, vehicles, ragdolls, Voronoi fracture. O(N) non-BVH `raycast`/queries; sync model load on fracture. | High |
| Animation & Retargeting | **3** | 4 | 4 | 3 | 3 | State machines, blend spaces, layers, 2-bone/FABRIK IK, retargeting, facial/viseme lip-sync, GPU compute posing. No motion matching yet (planned AP7). | High |
| Character Creation & Wardrobe | **2** | 4 | 2 | 1 | 4 | Standalone Character Studio + v2 manifests + surface-binding fitter (`avatar`). Realistic family ships only underwear + draft hair/clothes with fit defects. | High |
| Terrain & Procedural Worlds | **3** | 4 | 3 | 2 | 3 | Heightfield sculpt/erosion/biomes/stitched LODs, scatter, spline paths, procedural buildings. Voxel SDF (`terrain/volume.rs`) is an unintegrated spike. | High |
| Audio & Spatial DSP | **2** | 4 | 4 | 3 | 3 | Kira spatial audio, occlusion, buses, reverb zones, effect chains. No streaming audio (all `StaticSoundData`), unbounded decode cache. | High |
| In-Game UI & Accessibility | **2** | 4 | 4 | 4 | 4 | CSS + `taffy` flexbox, UI Designer, gamepad focus, localization, accessibility. No `display: grid`/`flex-wrap`/`calc`, % padding zeroed in 6 widgets. | High |
| VFX & Particles | **3** | 4 | 4 | 3 | 3 | CPU + GPU compute particles (262k/emitter), curves, flipbooks, screen-space collisions. No node-based VFX graph yet. | High |
| AI & Navigation | **2** | 4 | 4 | 3 | 3 | 2.5D column navmesh, A*, steering agents, perception, behavior trees. Ignores dynamic unanchored obstacles; thread-per-rebuild. | High |
| Multiplayer & Netcode | **2** | 4 | 4 | 3 | 4 | `renet` 2, TLS tokens, self-hosted relay, prediction, lag comp, replays. Critical security defects (UserId spoofing, SSL-strip downgrade, open relay, unverified at scale). | High |
| Dedicated Server Operations | **2** | 4 | 4 | 3 | 4 | Headless `axcenity-server` + `axcenity-bots`. No SIGINT/SIGTERM cleanup, no `/health`, no structured logs, no container/orchestration. | High |
| World Streaming & Large Worlds | **2** | 4 | 3 | 2 | 4 | Distance streaming (`StreamingEnabled`), DataLayers, regions. Off by default; partition rebuilds entire Workspace on tree edit (~12ms at 51k parts). | High |
| In-Editor 3D Modeling & CSG | **3** | 3 | 2 | 1 | 2 | Full Model Studio (edit mesh, modifiers, UV, sculpt, paint, bake, rig) + BSP CSG. Ahead of Godot/Unity/Roblox in-box; BSP recursion stack-overflow risk. | High |
| Real-Time Team Collaboration | **2** | 3 | 2 | 1 | 4 | Live Yjs scripts, TLS+WS+Ed25519 ops, host handoff, Git sync, Team Board (`collab`, `repo`, `teamwork`). Critical client `Mirror` path traversal & Git token/arg bugs. | High |
| Editor Extensibility / Plugins | **2** | 4 | 4 | 4 | 4 | Sandboxed Luau Plugin API v3. Auto-executes untrusted project plugins with `files` access; no native Rust/C ABI extension mechanism. | High |
| AI Assistant / MCP Integration | **3** | 2 | 2 | 1 | 3 | Native `rmcp` MCP server (30+ tools, stdio + live TCP). Live TCP privilege self-declaration defect; `game_run` blocks UI thread. | High |
| Asset Pipeline & Formats | **3** | 4 | 4 | 4 | 3 | glTF/FBX/OBJ + USD→GLB + BC7/BC5 cooking + Toolbox (ambientCG/Poly Haven/Openverse). No Alembic/animated USD; unbounded RAM caches. | High |
| Platform Reach (Mobile/Web/XR/Console) | **0** | 4 | 4 | 4 | 4 | Windows + Linux primary, macOS secondary (unverified per change). Mobile, Web, XR, Consoles deliberately deferred (ADR-0011). | High |
| Online Services & Marketplace | **0** | 4 | 4 | 2 | 4 | Design-only (`docs/research/online-services.md` O1–O9). No accounts, cloud saves, matchmaking, hosted relays, or marketplace. | High |
| CI/CD, QA & Production Proof | **1** | 4 | 4 | 4 | 4 | 5,579 unit tests + Luau game tests + bench-check, **but** CI disabled (ADR-0018), manual `ci.yml` broken, `main` has compile/test/clippy failures, no fuzzing, no shipped commercial titles. | High |
