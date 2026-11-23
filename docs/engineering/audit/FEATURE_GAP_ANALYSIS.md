# Feature Gap Analysis (`8f4c2420`)

This analysis separates **urgent production blockers**, **high-leverage engineering gaps**, and **deliberate non-goals** (per ADRs and `docs/development-policy.md`).

## 1. Critical Production Blockers (Must Close Before Public Alpha/Beta)

1. **Broken Trunk & Verification Hygiene**: `main` at `8f4c2420` fails `cargo test --workspace` (`project`), has 2 failing `script` unit tests, 2 `editor` clippy errors, 1 flaky `editor-toolbox` test, and a broken manual `.github/workflows/ci.yml`. Without a mandatory local pre-push gate (or re-enabled lightweight PR CI), regressions will continuously accumulate across 47 crates.
2. **Trust-Boundary Security Defects Across 6 Subsystems**:
   - Collaborate malicious host → client arbitrary file write/delete (`Mirror::take_theirs`).
   - Editor auto-execution of untrusted project plugins with `files` permission.
   - Multiplayer `UserId` spoofing → `ConsoleService` remote command execution & SSL-stripping plaintext password downgrade.
   - `repo` GitHub OAuth token exfiltration & Git `--upload-pack=` option injection.
   - Live MCP TCP server accepting client-declared `Mode::Full` and `allow_import: ["/"]`.
   - `Arena::insert_at` unbounded allocation DoS & 4 unbounded zip extractors.
3. **Operational & Data-Integrity Gaps**:
   - `save` silently corrupts save files if any float is `NaN`/`Inf`.
   - `axcenity-server` ignores SIGINT/SIGTERM (leaking temp dirs & UPnP leases).
   - `axcenity-relay` has a live concurrent `TcpStream` write race and open-relay abuse vectors.
   - `Git::show` corrupts binary files via `from_utf8_lossy`.

## 2. High-Leverage Engine & Workflow Gaps (vs. Peers)

| Gap | Peers Ahead | Concrete Axcenity Evidence | Strategic Value of Closing |
|---|---|---|---|
| **Physics Spatial Queries (BVH)** | UE5, Unity, Godot, Roblox | `physics/src/query.rs:243-287` linearly scans Workspace bounds | High — immediately speeds up character movement, audio occlusion, and Luau raycasts in large scenes |
| **Disk Shader/Pipeline Cache & Slim Shadow Instances** | UE5, Unity, Godot | `cache: None` at 30+ `render` sites; 172B `InstanceRaw` in shadow passes | High — faster cold startup and lower shadow-pass GPU bandwidth |
| **Bounded Asset/Audio Memory & Streaming Audio** | UE5, Unity, Godot, Roblox | Unbounded `register_identity`, `hidden_models`, `AudioEngine::decoded`; no audio streaming | High — prevents OOM in long Character Studio sessions and music-heavy games |
| **In-Place `.axgame` Mounting** | Godot (PCK), Unreal (Pak), Unity (AssetBundles) | `player/src/main.rs:155-189` unpacks entire archive to temp dir on every launch | High — faster launch, half disk usage, zero temp-dir leaks |
| **Character Content & Synchronized LODs** | Roblox, Unreal MetaHuman | Realistic family ships only underwear + draft hair/clothes; per-part LOD poke-through | High — default new-project characters currently spawn in underwear |
| **Editor Crate Decomposition** | Godot, Unity, Unreal | `crates/editor` is 257k LOC (33% of repo); `EditorApp` has 171 fields | High — faster incremental compiles and lower merge friction |
| **Dynamic Navmesh Obstacles & Pooled Jobs** | UE5, Unity, Godot | `nav/builder.rs:12-15` ignores unanchored parts; ad-hoc OS thread spawns | Medium-High — required for doors, moving crates, and large worlds |
| **Native Extension Seam (Rust/WASM/C ABI)** | Godot (GDExtension), Unity (C#/Burst), Unreal (C++) | Modifying engine behavior requires forking Rust workspace | Medium — needed once third-party ecosystem grows |
| **Real Multi-Machine Multiplayer Proof (16–32+ clients)** | All 4 peers | Only loopback / 16-bot emulated-router runs; `Message::Update` serialized N×/frame | High — required before claiming multiplayer production readiness |

## 3. Deliberate Non-Goals / Deferred Capabilities (Preserve ADRs)

Do **not** treat these as accidental defects unless the project owner explicitly supersedes the governing ADR:
- **No ECS** (ADR-0002): Heterogeneous Roblox-style Instance tree with class/tag indices is intentional.
- **No Mobile / Web / Console / XR Targets Yet** (ADR-0011): Desktop-first (Windows + Linux primary, macOS secondary).
- **No Cross-Machine Simulation Determinism** (ADR-0014): Server-authoritative reconciliation instead of deterministic lockstep.
- **No Render Graph Yet** (ADR-0009): Tracked as L8, deferred until pass count justifies complexity (though `scene.rs` at 3.7k LOC is nearing that inflection point).
- **No Hosted Cloud Platform / Marketplace Yet** (`docs/research/online-services.md` O1–O9): Intentionally deferred until core engine & self-hosted tooling stabilize.
