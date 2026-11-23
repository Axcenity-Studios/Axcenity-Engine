# Competitive Comparison: Axcenity Engine vs. Roblox Studio

**Reference Baseline:** Roblox Studio (2026: Instance tree, Luau + native `vector` + Parallel Luau, Team Create, Creator Store, hosted cloud servers/DataStores/avatars/marketplace, mobile/desktop/console/VR).  
**Axcenity Baseline:** Commit `8f4c2420` (`0.3.0-alpha.1`, Rust, Luau, wgpu 30, rapier3d, Apache-2.0).

## Executive Verdict

Roblox Studio is Axcenity's **primary workflow inspiration** (Instance tree, Explorer/Properties/Ribbon layout, Luau with Roblox-familiar `Workspace`/`RunService`/`RemoteEvent`/`Humanoid` APIs). Axcenity's compelling differentiation against Roblox is **full creator ownership**: standalone desktop `.axgame` exports with zero platform lock-in or revenue share, text-based Git-friendly project files, Linux support, far richer built-in rendering/modeling/UI (PBR/Skin/Hair/Cloth, Model Studio, CSS/taffy UI, Blocks ↔ Lua), and self-hosted dedicated servers. Conversely, Roblox Studio wins overwhelmingly on **platform distribution, battle-tested 50–100+ player cloud servers, mature Luau IDE/type solver + Parallel Luau, rich avatar/clothing catalog, and security-hardened multi-tenant infrastructure**.

## Domain-by-Domain Comparison

| Domain | Axcenity | Roblox Studio | Evidence & Analysis |
|---|---:|---:|---|
| **Creator Ownership & Export** | **4** | 0 | Axcenity exports standalone Windows/Linux/macOS games under Apache-2.0 with Git-friendly JSON files. Roblox games run only inside the Roblox client with DevEx revenue share. |
| **Workflow Familiarity** | **3** | 4 | Same Instance tree, Explorer/Properties, Play/Stop, Luau API conventions (`Script`/`LocalScript`/`ModuleScript`/`RemoteEvent`), plus Blocks ↔ Lua for beginners. |
| **Luau Tooling & Runtime** | **3** | 4 | Roblox has native `vector`, Parallel Luau actors, per-script memory limits, and an industry-leading built-in Luau type solver & autocomplete. Axcenity has `--!native` JIT and state-preserving hot reload, but userdata `Vector3`, no Parallel Luau, no per-script memory limit, and thin in-editor autocomplete. |
| **Rendering & Modeling** | **3** | 3 | Axcenity's wgpu 30 renderer (Skin/Hair/Cloth, software GI, opt-in HW RT, custom `naga` WGSL surface shaders, volumetric clouds/fog) and full Model Studio exceed Roblox's fixed material system and basic CSG. |
| **Avatars & Character Studio** | **2** | 4 | Axcenity's standalone Character Studio + surface-binding clothing fitter is architecturally strong, but ships only underwear + draft hair/clothes for its realistic default body. Roblox has a massive production avatar/layered-clothing ecosystem. |
| **Team Create / Collaborate** | **2** | 4 | Axcenity's Collaborate (TLS+WS, Yjs live scripts, Tailscale, host handoff, Git sync, Team Board) is genuinely innovative and avoids cloud lock-in, **but** currently has a critical client `Mirror` path-traversal bug and unverified real-world multi-user soak testing. |
| **Multiplayer & Security** | **2** | 4 | Roblox's cloud servers, matchmaking, and anti-exploit boundaries are hardened at massive scale. Axcenity's `net` + `relay` + `server` currently have critical `UserId` spoofing, SSL-strip downgrade, and open-relay defects. |
| **Online Services & Monetization** | **0** | 4 | Roblox provides DataStores, MessagingService, Matchmaking, Marketplace, and built-in player audience. Axcenity Online (`docs/research/online-services.md`) is 0% implemented. |

## What Axcenity Should Adopt, Improve, or Deliberately Reject from Roblox Studio

- **Adopt / Adapt**:
  - **Cryptographic player identity & hardened server trust boundaries** (eliminate `UserId` spoofing and plaintext fallbacks).
  - **Per-script Luau memory caps** and stronger in-editor Luau autocomplete/diagnostics.
  - **Ship a complete, QA-passing starter wardrobe & hair set** for the default realistic character so new games don't spawn characters in underwear.
  - **Plugin trust & permission prompts** (Roblox warns/gates plugin script injection and HTTP permissions; Axcenity must gate project-bundled plugins with `files` access).
- **Deliberately Reject**:
  - Do **not** sacrifice standalone export, Git-friendly text formats, or self-hostable dedicated/collab servers to build a closed walled garden.
