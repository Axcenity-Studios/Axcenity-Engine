# Character Studio overhaul: dependencies and licences

What the overhaul depends on, code and data, and the rules for adding more. The engine is Apache-2.0: every dependency must be compatible, pinned once in the workspace `Cargo.toml` with a comment, followed by `cargo hakari generate`, and kept out of the player when it is editor-only ([wiki: Dependencies](../../../wiki/Reference/Dependencies.md)). Only the lead adds a dependency ([agent-assignments.md](agent-assignments.md)).

## Code

| Dependency | Version | Licence | Used for | Status |
|---|---|---|---|---|
| `serde`, `serde_json` | 1 | MIT or Apache-2.0 | Character Data files (`avatar`) | Already pinned; used by `avatar` since CO1 |
| `thiserror` | 2 | MIT or Apache-2.0 | `avatar::FormatError` | Already pinned |
| `axcenity-assets` | workspace | Apache-2.0 | `avatar::ModelContents` reads loaded models | Engine crate |
| `meshopt` | 0.7 | MIT | MikkTSpace tangents (`generate_tangents`), constrained LODs with locked region borders | Already pinned; CO4, CO7 |
| `gltf` | 1.4 (feature `extensions`) | MIT or Apache-2.0 | Reading `KHR_materials_sheen`, `specular`, `clearcoat`, `anisotropy`, `variants` | Already pinned; turning the feature on is a CO7/CO9 change for the lead |
| `openusd`, `openusd-schemas` | 0.8 | TOST-1.0 (Apache-2.0 terms, different trademark clause) | USD `BasisCurves` → groom file, UsdSkel → glTF (editor-only `convert`, ADR-0027) | Already pinned, editor-only; CO11 |
| Vendored `rapier3d` | 0.36.1 | Apache-2.0 | Soft bodies (`cloth`, `rope`) for coarse cloth proxies | In `vendor/`; CO15 |
| `intel_tex_2` | 0.5 | MIT (ISPC kernels) | BC7/BC5 cooking of the new texture sets; BC4 for one-channel maps | Already pinned, editor-only |

**CO1 added no dependency.** The new crate `axcenity-avatar` (Apache-2.0, `crates/avatar`) uses only the rows above; `cargo hakari generate` also dropped a stale `zeroize` line from `crates/workspace-hack`.

**None planned.** Every later milestone is covered by what is pinned or vendored. A specialist who finds otherwise asks the lead with: the crate, version, licence (checked in the registry source, not only the README), maintenance state, size, whether the player would link it, and the alternative without it.

**Rejected or deferred:** TressFX and HairWorks (frozen or closed); NvCloth (NVIDIA licence); `havok-cloth` (GPL-3); MB-Lab and MPFB2 code (AGPL, GPL); OpenSubdiv and MaterialX crates (reference only); Alembic (no usable crate; hair comes through USD).

## Data and content

| Source | Licence | Use | Rule |
|---|---|---|---|
| Blender Studio Human Base Meshes v1.4.1 (realistic and stylized bodies, heads, hands, feet, teeth) | CC0 1.0 | Base bodies (ADR-0032) | Vendored with provenance (bundle URL and SHA-256 in `source/extract.py`) |
| MakeHuman system assets (targets, skins, proxies) | CC0 (assets) | Body shape targets transferred onto our topology (CO5) | **Data only**: never MakeHuman or MPFB2 code (AGPL, GPL) |
| Anny (NAVER) | Apache-2.0 code; MakeHuman-derived CC0 body data | Reference and data source | Never its optional SMPL-X files (non-commercial) |
| Google GNM Head v3 | Apache-2.0 (code and weights) | Spike: a face identity space (CO6) | Only if its topology wraps onto ours |
| ICT FaceKit Light | MIT | Spike, as GNM | |
| Separable SSS reference code (Jimenez et al.) | BSD-2-style | Screen-space skin scattering (CO17) | **Requires the credit line** "Uses Separable SSS. Copyright (C) 2011 by Jorge Jimenez and Diego Gutierrez." in the credits |
| Artist-made or imported items | Per item | Premium tier (ADR-0034) | `source.licence` (SPDX) is required in the item manifest; the import flow refuses items without one |

**Cannot bundle:** SMPL, SMPL-X, STAR, SUPR, older FLAME (non-commercial); FLAME 2023 Open (use restrictions that bind users); MetaHuman assets and DNA Calibration; 3D Scan Store, Texturing.xyz and Digital Emily samples. Users may import their own.

## Patents to check before shipping

| Area | What | Before |
|---|---|---|
| Strand hair | Hair rasterizer patents (compute rasterization of strands) | CO14 ships strands |
| Cloth | Long-range attachments (tethers): NVIDIA US 9,070,220; expiry unchecked | CO15 implements tethers |
| Skinning | Optimized centres of rotation, Delta Mush | Either is adopted (neither is planned) |
