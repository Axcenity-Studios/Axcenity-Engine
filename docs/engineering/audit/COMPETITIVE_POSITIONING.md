# Competitive Positioning & Strategic Recommendations (`8f4c2420`)

## 1. Where Axcenity Actually Stands Today

At commit `8f4c2420` (47 crates, 776k LOC Rust, 10.4k LOC WGSL, 5,579 unit tests), Axcenity Engine is an **unusually broad, cleanly layered, pre-production desktop 3D engine** (`0.3.0-alpha.1`). Its core architecture (generational arena, unified 10-stage frame schedule, strict `player`/`server`/`render` crate boundaries, single Luau runtime with Blocks ↔ Lua, built-in Model/UI/Character/Terrain/Animation studios, self-hosted Collaborate, and native MCP server) is **coherent and real** — not vaporware.

However, Axcenity is **not yet production-ready**. Rapid feature expansion has outpaced hardening, verification discipline, and security review:
- Trunk (`8f4c2420`) currently fails `cargo test --workspace` and `cargo clippy -- -D warnings`.
- Six subsystems contain P0 trust-boundary security vulnerabilities (Collaborate client path traversal, auto-executing project plugins, `UserId` spoofing to remote console, SSL-stripping password downgrade, Git token/option injection, live MCP privilege self-declaration).
- Several flagship features (realistic Character Studio wardrobe/hair, hardware ray tracing, multiplayer at 16–32 real clients) remain partial or unverified at scale.

## 2. Strategic Differentiation Matrix

| Strategy | Capabilities | Rationale |
|---|---|---|
| **Double Down (Core Differentiators)** | 1. **Roblox-familiar workflow + full Apache-2.0 standalone ownership** (Git-friendly text projects, zero royalties, Linux + Windows desktop parity)<br>2. **One Luau runtime + Blocks ↔ Lua round-trip + state-preserving hot reload**<br>3. **Batteries-included creator studios** (Model Studio, Character Studio, UI Designer, Terrain, Animation)<br>4. **Self-hosted Collaborate (Yjs + Git + Team Board)** & **native MCP AI integration** | No other open-source engine combines Roblox's approachable creator ergonomics, standalone royalty-free exports, built-in modeling/character studios, and local-first real-time collaboration. |
| **Adopt & Harden (Table Stakes for Credibility)** | 1. **Trunk green-gate & fixed CI** (`cargo test`, `clippy`, `fmt`)<br>2. **Zero-trust security boundaries** (Collaborate client validation, plugin trust prompts, authenticated multiplayer admin, TLS strict mode, safe zip extraction, hardened MCP)<br>3. **Runtime efficiency fundamentals** (Rapier BVH queries, disk `wgpu::PipelineCache`, in-place `.axgame` reads, bounded asset/audio caches + streaming audio, single-serialize multiplayer broadcast)<br>4. **Complete default character assets** (QA-passing hair/clothes + synchronized LODs + Eye shader) | Without these, security incidents, OOMs, or broken builds will undermine adoption regardless of feature breadth. |
| **Deliberately Defer / Reject** | 1. **ECS rewrite** (ADR-0002)<br>2. **Mobile / Web / Console / XR targets** (ADR-0011)<br>3. **Nanite / Lumen / Niagara parity**<br>4. **Hosted cloud platform / marketplace** (`online-services.md` O1–O9) until core engine & self-hosted servers are hardened | Premature expansion here would split engineering focus and erode desktop quality. |

## 3. Recommended Sequencing

1. **Stabilize & Secure First (Phases 0–2)**: Restore green trunk (`project` compile, `script` tests, `editor` clippy, `ci.yml`), fix all P0 security & data-loss bugs (`collab` path traversal, plugin consent, `net` auth/downgrade, `repo` git security, `mcp` live policy, `Arena::insert_at`, `save` NaN/Inf, `relay` race, `debugger.rs` unsafe checks).
2. **Runtime & Operational Hardening (Phases 3–4)**: Route `physics::query` through Rapier BVH, bound `assets`/`audio` caches + add streaming audio, add `wgpu::PipelineCache` + slim shadow instances, add `axcenity-server` signal handling, fix `net` N× broadcast serialization & `Driven::step` wraparound, mount `.axgame` in place.
3. **Creator Polish & Architecture Scaling (Phases 5–6)**: Continue splitting 257k-LOC `editor` (`Character Studio` extraction), ship QA-passing realistic hair/wardrobe + Eye shader + synchronized character LODs, fix UI `%` padding & `flex-wrap`, refresh stale `wiki/` pages, and execute real multi-machine 16–32 player soak tests.
