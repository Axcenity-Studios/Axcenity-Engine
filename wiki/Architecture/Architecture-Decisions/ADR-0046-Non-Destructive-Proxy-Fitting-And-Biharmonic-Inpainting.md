---
title: "ADR-0046: Offline garment fitting is non-destructive, using Fit/Skirt proxies, biharmonic inpainting, home-frame cuts, and Shrink_Under"
type: adr
adr: 46
adr_status: Accepted
date: 2026-11-24
supersedes: ADR-0044
superseded_by:
affects: [avatar, assets]
source: [crates/avatar/src/fit/proxy.rs, crates/avatar/src/fit/inpaint.rs, crates/avatar/src/fit/bind.rs, crates/avatar/src/fit/transfer.rs, crates/avatar/src/fit/validate.rs, crates/avatar/src/fit/import.rs, crates/avatar/src/resolve.rs, assets/characters/realistic/clothing/generated/prep_meshy_garments.py]
docs: [docs/development/character-studio/architecture.md, docs/research/character-clothing-workflow.md, crates/avatar/CLAUDE.md]
tags: [adr, characters, clothing, assets]
---

# ADR-0046: Offline garment fitting is non-destructive, using Fit/Skirt proxies, biharmonic inpainting, home-frame cuts, and Shrink_Under

- **Status:** Accepted (2026-11-24).
- **Date:** 2026-11-24
- **Affected systems:** Character Data (`crates/avatar/src/fit/`, `crates/avatar/src/resolve.rs`), upstream garment prep (`assets/characters/realistic/clothing/generated/prep_meshy_garments.py`).
- **Supersedes:** [ADR-0044](ADR-0044-Garments-Not-Made-On-The-Body-Come-Through-An-Intake-Before-The-Fit.md) (the destructive 11-step in-engine intake). Amends [ADR-0035](ADR-0035-Clothing-Fits-By-Surface-Binding-With-Region-Hiding.md) (replaces raw-skin binding & rest push-out with FitProxy binding & biharmonic inpainting) and [ADR-0045](ADR-0045-What-Is-Worn-Over-Skin-Or-Cloth-Hides-It-By-A-Cover-Map.md) (keeps UV cover-map culling; replaces `hang.rs` & `conform.rs` with `Shrink_Under`).

## Context and problem

The 2026-11-24 Character Studio audit found that while Axcenity's **runtime** character architecture (Format 2 descriptions, [ADR-0041](ADR-0041-Shared-Base-Mesh-With-Live-Identity-Morphs-Baking-Is-Optional.md) live identity morphs, CO8 GPU compute posing, and [ADR-0045](ADR-0045-What-Is-Worn-Over-Skin-Or-Cloth-Hides-It-By-A-Cover-Map.md) UV cover-map triangle culling) is sound, the **offline fitting & intake pipeline** caused severe mesh warping, self-intersection, and permanent rest-geometry disfigurement:

1. **Stacked destructive rest-mesh mutations:** `intake/shell.rs`, hanging-arm unbending + web-cutting, `cut_for` feminine-to-masculine warping, `centre.rs`, `conform.rs`, `hang.rs`, and `transfer.rs` Step 5 extreme rest-lifting mutated canonical garment rest vertices in place across 6+ sequential passes.
2. **Raw nearest-skin tangent-frame lever-arm blowups:** Binding loose cloth directly to concave body skin (armpits, cleavage, crotch, buttock cleft) caused single-morph displacements up to 18.9 cm (`AthleticLeggings.glb`).
3. **Post-conform decimation and ear-clipping:** Decimating after conforming without locked boundaries and ear-clipping arbitrary 3D slits (`mend.rs`) produced 109–417 ragged boundary edges and sliver triangles.

## Decision

1. **Immutable canonical rest geometry:** `fit_garment` never mutates rest vertex positions or topology. `fit/hang.rs`, `fit/mend.rs`, `fit/parts.rs`, and `fit/intake/*` are removed.
2. **Upstream canonical source contract + read-only gate:** AI-generated `.glb` garments undergo a one-time headless Blender prep pass (`assets/characters/realistic/clothing/generated/prep_meshy_garments.py`: outer-shell extraction, 45° A-pose arm placement, boundary-locked remesh enforcing $L < \sqrt{8rm}$ at openings). `avatar::fit::validate_garment` validates manifoldness, boundary loops, and opening chord-sag read-only before fitting.
3. **Authored Fit & Skirt Proxies + Layer Envelopes:** Garments bind to smooth, fixed-topology `FitProxy` (`ProxyKind::Body` bridging concavities, or `ProxyKind::Skirt` hip-to-ankle tube wrapping both legs) driven by fixed barycentric mixes of `BodyMesh` vertices, with deterministic radial `LAYER_ENVELOPE_OFFSETS` (Layers 0–3).
4. **Part-restricted proxy binding + `faer` biharmonic inpainting:** `bind_proxy` partitions vertices into 5 semantic `BodyPart`s and accepts confident matches only within `0.05 * bbox_diag` and `<= 35 deg` normal agreement (Abdrashitov et al., SIGGRAPH Asia 2023, MIT). Unbound vertices (armpits, collars, lapels, wide hems) are solved via a single `faer` (MIT) sparse Cholesky (`LLT`) factorization of $\mathbf{Q} = \mathbf{L} + \mathbf{L}\mathbf{M}^{-1}\mathbf{L}$ for:
   - **Skin weights** (clamped $\ge 0$, lightly smoothed on solved vertices, normalized, top-4 joints, cuff/hem joint holding);
   - **Frame-relative morph deltas** (solving the biharmonic residual relative to each vertex's smooth local proxy displacement, so free-hanging hems expand naturally with hip/thigh/chest morphs without flattening or lever-arm spikes).
5. **Home-frame authoring:** Cross-frame `cut_for` rest warping is removed; garments are fitted on their `HomeFrame` (`Masculine` or `Feminine`, dual-source variants for unisex items).
6. **Non-destructive layering (`Shrink_Under`):** ADR-0045 UV cover maps cull covered skin and inner cloth; `fit_garment` generates one non-destructive `Shrink_Under` morph target toward Layer Envelope `layer - 1`, activated live (`1.0`) by `avatar::resolve` only when an overlapping higher-layer garment is equipped.

## Consequences

- Rest-mesh drift during fitting is strictly `0.0 mm`.
- Single-morph lever-arm blowups are eliminated; sub-0.5 mm morph targets are pruned to reduce fitted `.glb` sizes.
- `faer` (MIT) is pinned in workspace `Cargo.toml`, and both `faer` and `RobustSkinWeightsTransferCode` (MIT) are attributed in `NOTICE`.
- Runtime crates (`anim`, `assets`, `gameplay`, `render`) require zero changes because `gameplay::description` already applies `ResolvedPart::shapes` live through CO8 compute posing.
