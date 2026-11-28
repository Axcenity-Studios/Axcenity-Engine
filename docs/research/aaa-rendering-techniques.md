# AAA rendering techniques for Axcenity: research notes

Researched 2026-11-27 for the Axcenity renderer (wgpu, forward+ PBR with a depth prepass, 4 sun cascades with PCSS and contact shadows, SSAO, TAA and temporal upscaling, froxel fog, volumetric clouds, terrain with 7-8 splat layers, painted foliage of alpha-tested cards, optional RT through wgpu ray queries). Target GPU: RX 9070 XT (RDNA4) on Linux Vulkan (RADV).

There are two goals. The first is performance close to Unity HDRP and Unreal Engine 5. The second is a look like The Last of Us Part II (TLOU2) and Rockstar's RAGE (RDR2, GTA VI).

**How to read this.** "Primary" means I read the slides, paper or vendor documentation myself. "Secondary" means press or third-party analysis. "Unverified" means I could not read the source, so the claim comes from an abstract, a summary, or my recall. The proxy blocked downloads from advances.realtimerendering.com, so I could not open the RDR2 slides (232 MB PPTX), and the RDR2 section relies on the abstract and secondary sources. I downloaded and read Naughty Dog's TLOU2 lighting, low-level optimization and tech-art decks in full.

**Where Axcenity stands** (from [performance.md](../rendering/performance.md), Overgrown Street, 1996 x 703, Ultra, RX 9070 XT):
- Without RT the GPU frame is 25.9 ms. Its biggest parts are the scattered plants in the prepass (4.8 ms, bound by vertex work), their pixels in the scene pass (3.0 ms), cut-out depth (2.0 ms) and cascade 4 (5.1 ms averaged, 10.4 ms on the frames it is drawn).
- With RT the frame is 40.3 ms. RT shadows take 12.8 ms at Full resolution (5.5 at Half) and RT AO takes 6.0 ms.
- A perfect occlusion test would save only about 1.45 ms.

The ranking at the end uses these numbers.

**What became of it (2026-11-28).** Recommendations taken up so far, each behind a switch for comparing and timed against the 2026-11-27 baseline (Overgrown Street at 2560 × 1405 on the PC):
- 10, card LODs for foliage: simpler copies of cut-out foliage with fewer, bigger leaf cards (`model/cards.rs`, PR #436, awaiting the owner's look check). Impostors are not done.
- 1, cheaper far cascades: the farthest of four drawn one frame in four (PR #438). Caching static casters across frames and opaque far proxies (2) are not done.
- 3, the RT ray budget: one solid-blocker query a pixel instead of one per ray stopped by leaves (PR #437). Near-field-only RT shadows, checkerboarding and a shadow denoiser are not done.
- Also from the profile, not ranked here: cut-outs lit over the depth started from the prepass, with no separate depth fill (PR #435).

---

## A. The Last of Us Part II (Naughty Dog, PS4, 30 fps)

Naughty Dog gave five talks at SIGGRAPH 2020 (index: https://www.naughtydog.com/blog/naughty_dog_at_siggraph_2020). The core point of the lighting talk: TLOU2 is "mainly overcast", so ambient light is the image, and it was made entirely from **baked** data with clever runtime interpolation. There was no runtime GI.

### A1. Baked ambient: lightmaps, vertex lighting and SH probes (primary)
Source: Doghramachi, *Lighting Technology of The Last of Us Part II*, https://s3.amazonaws.com/nd.images/research/2020_siggraph/LightingTechTlou2.pptx

- **What it is.** Static geometry uses lightmaps or per-vertex lighting. Dynamic objects, and more and more static ones, use artist-placed **L2 (9-coefficient) SH probes**. There are up to 64k probes per playable area, stored in kd-trees, at 40 bytes each: 27 quantized SH bytes, a 4-byte scale, 1 byte of baked sun shadow and 8 bytes of occlusion.
- **Per-pixel probe interpolation.** The old system interpolated the 10 nearest probes once per object on the CPU. The new one interpolates at each of the 8 corners of the object's OBB on async compute, writes the results into a 3D texture atlas (7 x RGBA8_SNORM plus 1 x R16F), and does a hardware trilinear lookup per pixel. Atlas entries are cached and rebuilt only when an object moves. Grass clusters drawn in one instanced call use each instance's own OBB (the object ID offset by the instance ID). Ropes are lit per node during skinning.
  - Benefit: characters stop being lit uniformly, with an over-bright top and a dark bottom.
  - Cost: small, because of the caching.
- **Dominant light extraction with SH masking.** At runtime, per pixel, they take the SH product of the radiance and a clamped cosine lobe around the normal, then extract an "optimal linear" dominant direction and colour from the masked result. The remainder is flat ambient. The dominant part gets a specular response and its own shadowing. This removes the "flat, game-like look" (slide 7). HLSL is in Appendix A.
- **SH deringing.**
  - Static: at bake time, apply the least aggressive window that removes negative lobes (Sloan 2017).
  - Dynamic: reduce how much light is extracted as the normal turns away from the unmasked dominant direction. This fixes "burned out spots / metallic look" on faces and shoulders.
- **Probe occlusion against light leaks.** At bake time, several hundred rays per probe. The hit distances are fitted with an NNLS solver to the 6 faces of a cube oriented along the most occluded direction. That takes 8 bytes: six 1-byte depths up to 6 m, the normal as a 1-byte spherical Fibonacci index, and the tangent angle in 1 byte. At runtime, weight the 6 depths by the squared direction components and apply a smoothstep test (Appendix C). The talk admits it is approximate for vegetation, where artists moved probes or tuned the bias.
- **Hybrid probe lighting.** Under overcast skies, chrominance varies much less than luminance. So they store only **luminance per vertex** (4 bytes instead of 16) and take chrominance and direction from the probe volume. This needs the per-pixel masking above.

**Why it matters for the look.** This is the recipe for "soft overcast that still has shape": directional ambient (the dominant direction), no leaks (probe occlusion), and no SH artifacts.

### A2. Shadowing the ambient: specular occlusion and SSCT (primary)
- **Screen-space cone tracing (SSCT).** The dominant ambient light is shadowed by tracing a visibility cone through the depth buffer, treated as a height field. It uses 4 samples with growing steps, jittered per 4x4 tile per frame, and relies on TAA to remove the noise. Measured on base PS4 at 1080p, the SSAO pass costs 1.15 ms without SSCT and **1.27 ms with it** (about +0.12 ms). The final dominant-light term is min(SSAO, SSCT). It is used for self-shadowing on dynamic objects only. Narrower cones are reused for sun and local-light screen-space shadows. Code is in Appendix B.
- **Ambient capsules and volumes** (from TLOU1, Iwanicki 2013) cast soft ambient shadows from dynamic objects onto the world and, new in TLOU2, onto other dynamic objects. An object-ID buffer from the depth pass prevents self-shadowing (https://history.siggraph.org/learning/lighting-technology-of-the-last-of-us-by-iwanicki).
- **IBL occlusion.** The tech-art talk (https://s3.amazonaws.com/nd.images/research/2020_siggraph/NaughtyDog_TechArt_TLOU2-final.pptx, slide 51) runs the same screen-space shadow function along the **reflection vector** to occlude reflections, plainly a form of specular occlusion. SSAO and screen-space shadows also read a **depth+heightmap** buffer, so material relief such as cracks and bricks occludes "almost for free".
- For a published specular-occlusion formulation with costs, see Jimenez et al. 2016, *Practical Realtime Strategies for Accurate Indirect Occlusion* (https://www.iryoku.com/downloads/Practical-Realtime-Strategies-for-Accurate-Indirect-Occlusion.pdf).
  - GTAO costs **0.5 ms on PS4 at 1080p**, computed at half resolution with one direction per pixel. A 4x4 bilateral filter plus 6 temporal rotations gives 96 effective directions.
  - **GTSO** specular occlusion is the overlap of a bent-normal visibility cone (cos a = sqrt(1 - AO)) and a roughness-derived specular cone, baked into a 4D or 3D LUT.
  - The multi-bounce fit is G(A, rho) = a(rho)A^3 - b(rho)A^2 + c(rho)A, with coefficients linear in albedo. Bright foliage and concrete then don't go grey under AO.

### A3. Wet surfaces (primary)
Source: the tech-art deck above, slides 29-42.
- **One wetness mask drives everything.** Artists author only a wetness mask. It is animated by a scrolling cloud texture crossfaded with the "Flow" technique to make rivulets. Height comes from the mask (heightmaps are in cm, +/-10 cm), and normals are derived from height with screen derivatives (code on slide 27).
- **Separate specular and diffuse normals.** A deferred permutation carries two normals. Subsurface and diffuse read the diffuse normal, IBL and specular read the specular (water) normal. Without this, "the drips would look like veins". Puddles use it the same way: a diffuse normal for the ground under the water and a specular normal for the water surface.
- **Fake caustics and refractive shadowing.** A cheap analytic term (slide 39) brightens the far side of a droplet and adds a Fresnel-occlusion darkening. It is tuned against photographed reference.
- **Cost.** It is a shader permutation plus one extra normal. Axcenity is forward+, so the second normal never touches a G-buffer and costs only a few ALU per wet pixel.

### A4. Fog and atmosphere (abstract only; unverified detail)
Kovalovs, *Volumetric Effects of TLOU2* (deck is 1.2 GB; abstract at https://history.siggraph.org/learning/volumetric-effects-of-the-last-of-us-part-two-by-kovalovs). It describes a new **view-space froxel grid** fog that composites correctly with glass and particles, and work to "reduce grid artifacts" within budget. It builds on Wronski 2014, Hillaire 2015 and Hoobler 2016. Axcenity already has froxel fog, so the TLOU2 lesson is integration (transparents fogged per vertex or per pixel from the same grid) rather than a new technique.

### A5. Foliage (no rendering talk; secondary)
Naughty Dog published no foliage rendering talk for TLOU2. What is known:
- Per-instance probe lighting inside instanced grass clusters (A1).
- Stealth grass "went through dozens of iterations" balancing density against frame rate.
- Simplygon made the automatic LODs.

Source for the last two: 80.lv interviews, https://80.lv/articles/how-naughty-dog-created-the-immersive-world-of-the-last-of-us-part-ii (secondary). The low-level optimization talk (https://s3.amazonaws.com/nd.images/research/2020_siggraph/Low_Level_Optimizations_In_TLOU2.pptx) lists "alpha geometry" among the most expensive costs. Its fixes are GCN-specific but still instructive:
- Forward water and alpha-tested depth-only geometry were bound by the depth block when **EarlyZ plus LateZ** were both active. Forcing **LateZ** raised pixel-wave throughput.
- Low-occupancy compute was moved to pixel shaders for better wave distribution.
- Cache-thrashing shaders ran at deliberately reduced occupancy.
- Total saving: **2.1 ms of a 33 ms frame** with no visual change.

### A6. How they got overcast to look real (secondary, plus primary on philosophy)
- Lighting lead Mark Shoaf (80.lv, above): "The majority of our lighting ... is achieved through baked lighting", with skydome and sun bounce baked. Physically based overcast "tends to look flat", so artists **painted ambient shadows and cloud shadows** into levels. In interiors they **blocked the sky** and placed small area lights outside windows and doors instead of letting flat skylight in. Cinematics used runtime lights without GI, so bounce on characters was hand-placed.
- Brinck (tech art): shoot your own reference, measure energy against ground truth, and avoid spatial and temporal discontinuities such as SSR-to-cubemap seams, because "hard edges and localized areas of contrast create mini focal points". Post and LUTs "contribute much of the final image" (80.lv).

**Takeaway for Axcenity.** The look needs (1) directional, leak-free ambient, (2) art tools for negative or blocked sky light (ambient-shadow volumes, sky-occlusion painting), and (3) seamless reflection fallbacks. Physical accuracy alone gives a flat overcast.

---

## B. RDR2 / RAGE and GTA VI

### B1. Bauer et al., *Creating the Atmospheric World of RDR2* (SIGGRAPH 2019 Advances)
Slides: https://advances.realtimerendering.com/s2019/slides_public_release.pptx (I could not download them here). Course page: https://advances.realtimerendering.com/s2019/index.htm. Video: ACM recording, about 1:58:00 into the course (per https://habr.com/ru/post/508934).

Confirmed by the abstract (primary):
- One **unified light-scattering system** covers clouds, fog and volumetrics, plus an **ambient model for indirect sky light**.
- The scattering and transmittance approach combines **voxelization and raymarching**. It serves the **main view, the reflection maps and a sky irradiance probe grid**. The systems "share data".
- It covers artist workflow and console optimizations.

From conference notes (https://carlos-lopez-garces.github.io/blog/misc/2019-08-02-siggraph-2019, secondary):
- "Voxelization is used to render at the near distance and ray marching at the far distance". In other words, a froxel volume near the camera and raymarched clouds and fog far away.
- Keywords listed: extinction volumes, Perlin-Worley noise, **checkerboard min/max depth downsampling** (low-res volumetrics reconstructed against depth), raymarch reconstruction, plus lightning and (non-physical) rainbows.

Unverified, from my recall of the talk (check the slides before relying on it): clouds raymarched at reduced resolution with temporal reprojection; a cloud shadow map shared by terrain, fog and the sky probes; and the sky irradiance grid baked per time-of-day keyframe and blended, so the ambient follows weather and cloud cover.

**Why it helps.** One medium and one set of lighting inputs (sun transmittance through clouds, sky irradiance) for fog, clouds and ambient means that a storm rolling in darkens the fog, the ground ambient and the reflections together. That coherence is the "RDR2 look".

### B2. RDR2 frame structure (RenderDoc analysis, PC; secondary)
Source: https://imgeself.github.io/posts/2020-06-19-graphics-study-rdr2.
- **Relit cubemap every frame.** Each frame renders a low-LOD G-buffer cubemap of static opaques at the camera (albedo, normal, depth, baked AO), lights it with tiled deferred compute, and convolves mips for split-sum IBL. This makes reflections and specular ambient **follow time of day and weather** without re-baking. Interiors use baked cubemaps reloaded nearby and recomputed on time-of-day change.
- **Baked large-scale AO.** A top-down world lightmap (like Assassin's Creed III) plus baked AO in the cubemaps and a G-buffer channel. Screen-space AO handles only small-scale occlusion.
- **Shadows.** 4 sun cascades in a 1024 x 4096 R16 atlas (medium settings). Static point lights use **baked shadow cubemaps**, and dynamic maps are drawn only when the player is near.
- **Bloom** follows CoD:AW: non-thresholded, a 7-mip R11G11B10 chain, a 13-tap downsample and a 3x3 tent upsample.

### B3. GTA VI (released November 2026; secondary, Rockstar has published no technical talk)
Digital Foundry analysis, as reported by TechPowerUp (https://www.techpowerup.com/355004/gta-6-reportedly-plays-best-on-ps5-pro-xbox-series-s-struggles), TweakTown (https://www.tweaktown.com/news/114763/gta-6-deemed-the-single-most-visually-striking-game-of-the-generation-by-tech-experts/index.html) and Vandal on the August 2026 footage (https://vandal.elespanol.com/noticia/2026240028/digital-foundry-analiza-los-graficos-de-gta-6-quiza-lo-mas-impresionante-que-hemos-visto-en-tiempo-real/):
- **RT GI and RT reflections on every console except possibly Series S.** Reflections use "hybrid" techniques on some surfaces to save cost. **RT shadows only on PS5 Pro.**
- About **1440p internal at 30 fps**. PSSR on Pro, a TAA-based spatial upscale on base PS5 and Series X, and FSR 4 at about 720p on Series S. RT reflections are noted as noisy.
- **Unverified:** the RT GI algorithm (probe or screen-space caching versus per-pixel ReSTIR), the denoiser, and any reuse of RDR2's atmosphere stack.

**Takeaway.** Rockstar's current-gen budget is RT GI and reflections at 1440p30 on a PS5, which is roughly a 6700-class GPU. A 9070 XT has about 2-3x that throughput (estimate). So RT GI plus hybrid reflections at 1440p60 is realistic **only with a caching GI design** (see F3), not per-pixel path tracing.

---

## C. Unreal Engine 5 (5.4 to 5.8; 5.8 is the last 5.x before UE6)

### C1. Lumen
Sources: Lumen technical details, https://dev.epicgames.com/documentation/en-us/unreal-engine/lumen-technical-details-in-unreal-engine, and Wright et al., SIGGRAPH 2022, https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Lumen-Wright%20et%20al.pdf (both primary).

- **Budget.** The Epic scalability level costs "around **8 ms** on next-generation consoles for GI and reflections at 1080p internal". The High level targets 60 fps open worlds.
- **Final gather.**
  - A screen-space radiance cache at **1/16 resolution in each dimension** (adaptively subdivided where geometry is detailed), with 8x8 directional resolution per probe. Product importance sampling gives "about 4x more rays" of quality.
  - A world-space radiance cache (clipmaps, persistent) handles distant lighting at higher angular and lower spatial resolution.
  - Translucency and fog GI use froxel probes at 1/8 of the opaque budget.
  - Reusing screen probes for rough reflections (roughness 0.4-1.0) cut reflection cost by **50-70%**.
- **Trace order.** Screen traces first (HZB, half-res), then mesh SDF to about 2 m, then global SDF clipmaps, then far field, then the skylight.
- **Surface cache.** Mesh "cards" (12 per mesh by default), a 4096^2 atlas, and per-frame lighting budgets of 1024^2 texels direct and 512^2 indirect.
- **Software vs hardware RT, measured on PS5 at 1080p (final-gather tracing).**
  - Lyra: global SDF 1.21 ms, HWRT 1.41 ms.
  - Matrix Awakens: SWRT 1.83 ms, HWRT 1.72 ms.
  - "Lumen in the Land of Nanite" with heavy instance overlap: SDF **0.94 ms vs HWRT 10.03 ms**.
  - Reflections with hit lighting: 11.54 ms, against 2.44 ms with the surface cache.
  - Lesson: **HWRT cost explodes with many overlapping instances**, which is exactly the foliage case. Epic's docs say software RT "is the only performant option with many overlapping instances".
- **Foliage in SDF tracing.** A coverage channel gives stochastic transparency plus a surface bias. World Position Offset (WPO, vertex animation) isn't captured.
- **Lumen Lite (5.8, experimental).** "Irradiance fields with probe occlusion", about **2x faster than Lumen High**, and enough for 60 fps GI on Switch 2 (https://www.unrealengine.com/news/unreal-engine-5-8-is-now-available, primary marketing). Epic moving toward a DDGI-like probe field for its cheap tier supports Axcenity's choice of a DDGI-style design.

### C2. Virtual Shadow Maps: caching is the lever
Source: https://dev.epicgames.com/documentation/en-us/unreal-engine/virtual-shadow-maps-in-unreal-engine (primary).
- A 16k^2 virtual map per clipmap level in 128^2 pages. Pages are allocated only where on-screen pixels need them.
- **Caching is on by default**, and **separate static caching** keeps static and dynamic depth apart, so a moving character doesn't re-render static pages ("static page invalidations should be near zero").
- **What invalidates pages:** a moving light, moving casters, and **WPO or pixel depth offset (PDO)**. Swaying foliage therefore invalidates every frame.
- **Epic's mitigations:** a WPO disable distance; non-WPO materials at distance through LODs; replacing WPO with skinning; contact shadows for grass; tight bounds. Shadow-projection cost scales with SMRT rays x samples (8 rays and 4-8 samples at Epic), not with page count.
- Non-Nanite casters are "much more expensive". A UE 5.8 thread confirms the receiver-mask default and the "Shadow Casters" debug view (https://forums.unrealengine.com/t/vsm-cached-page-mode-in-5-8/2843727).

**Why it matters.** With caching, the sun's shadow cost is roughly proportional to *what changed*, not to what is visible.

### C3. Nanite foliage, assemblies and voxels (5.7, experimental)
Source: https://dev.epicgames.com/documentation/en-us/unreal-engine/nanite-foliage (primary).
- **Assemblies:** up to 65k part instances (branches, fronds) encoded into one hierarchy and transformed on the fly during cluster culling. Big memory savings at a small culling cost.
- **Voxels:** at distance, clusters switch to near-pixel-sized voxel bricks (4x4x4, up to 128 bricks per cluster) when the voxel error is lower than the triangle error. They are rasterized front to back and store a normal *distribution* sampled stochastically. This is Epic's answer to card-heavy canopies at distance.
- **Skinned wind instead of WPO:** bounds come from the bones, so culling and VSM caching stay tight. About **0.1 ms per 100k updating bones**. Animation stops below 0.1 screen size by default.
- Epic avoids WPO because worst-case bound padding inflates culling and overdraw, and each WPO material gets its own dispatch.

### C4. MegaLights (beta in 5.7, production in 5.8)
Source: https://dev.epicgames.com/documentation/en-us/unreal-engine/megalights-in-unreal-engine (primary).
- Stochastic direct lighting with a fixed number of rays per (downsampled) pixel, ray guiding toward unoccluded lights, a screen trace followed by HWRT, and a denoiser. The cost is roughly constant in light count.
- **Alpha-masked surfaces are handled only by screen-space traces by default**, because HWRT alpha support "adds notable overhead". Epic avoids any-hit for alpha in production RT, which matters for F1.
- Directional lights are off by default, since "strong directional lights are better handled with deferred lighting and VSM".

### C5. Substrate (production-ready in 5.7)
A layered material framework (slabs, coats and so on) integrated across platforms (https://forums.unrealengine.com/t/unreal-engine-5-7-released/2673913, primary release notes). It is a look and authoring feature, not a performance one. It is relevant to Axcenity only as a model for wet layers and clear coat.

### C6. TSR
Source: https://dev.epicgames.com/documentation/en-us/unreal-engine/temporal-super-resolution-in-unreal-engine (primary).
- At 4K output the doc's example is 57.5 ms native against **33.4 ms from 1080p** (about half the frame).
- The TSR pass costs 0.79 ms at 100% and 0.43 ms at 50% (Valley of the Ancient). In Fortnite on PS5 the effective cost is 1.5 ms.
- The history update runs at display resolution. A 200% history (Epic and Cinematic levels) costs about 4x.
- Thin and sub-pixel geometry flickers. The fix is a shading-rejection flicker heuristic, plus correct previous-frame WPO for motion vectors.

### C7. Scalability settings that matter most for performance
Inferred from the docs above (the docs give the mechanisms; the ordering is mine).
1. **Screen percentage / TSR**: the largest single lever, about 2x.
2. **sg.GlobalIlluminationQuality and sg.ReflectionQuality**: Lumen Epic versus High is roughly 30 fps versus 60 fps by design.
3. **sg.ShadowQuality**: VSM resolution bias and SMRT rays.
4. **Foliage**: WPO disable distance and cull distances.
5. **sg.AntiAliasingQuality** (the TSR history).

### C8. Linux Vulkan RT status
- UE supports SM6 and HWRT on Vulkan.
- In 2026 forum reports, UE 5.8 on Linux with RADV hit `VK_ERROR_DEVICE_LOST` on RDNA4 and RDNA2 in Lumen, Nanite and VSM compute paths. The workaround was disabling async compute. The issue was closed as a duplicate, and a user said it "seems fixed" in 5.8.1 (https://forums.unrealengine.com/t/ue-5-8-vulkan-on-linux-triggers-vk-error-device-lost-gpuvm-read-fault-on-amd-rx-9070-xt-with-radv/2730749).
- Phoronix reported that Mesa 26.0 "merges the big ray-tracing improvement for UE5 Lumen" (headline only; I could not read the article).
- So UE5 HWRT on this exact platform is usable but still rough, and Axcenity isn't at a competitive disadvantage there.

---

## D. Unity HDRP (Unity 6)

- **GPU Resident Drawer** (https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/gpu-resident-drawer.html, plus the Unity 6.6 performance page).
  - What it is: groups GameObjects with the same mesh and shader variant into instanced draws (BatchRendererGroup-based).
  - Effect: it cuts **CPU** cost, "slightly increases GPU workload", and can raise overdraw. Unity recommends depth priming (a prepass) to counter that.
  - No published ms figures.
  - Axcenity already instances its scattered plants by mesh, material and distance band.
- **GPU occlusion culling** (https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/gpu-culling.html). It uses current and previous-frame depth and culls what is "occluded in both frames". Unity warns that "if occlusion culling has little effect, rendering time can increase" because of the setup cost. That matches Axcenity's measured ceiling of 1.45 ms.
- **Adaptive Probe Volumes** (https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/probevolumes-concept.html).
  - Bricks of 4x4x4 probes, whose spacing (1/3/9/27 m by default) follows geometry density. **Per-pixel sampling of the 8 nearest probes**.
  - Leak tools (validity, virtual offset, a dilation pass). **Sky occlusion** is updated at runtime, so time of day works with baked probes. **Lighting scenarios** can be blended, and streaming is supported.
  - This is the closest engine equivalent of TLOU2's system, without dominant-light extraction.
- **STP**: a compute-shader spatial-temporal upscaler (SM5), with heavier filtering and deringing on PC and console. No published cost (https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/stp/stp-upscaler).
- **Shadow caching** (https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@16.0/manual/shadow-update-mode.html and https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.1/manual/Shadows-in-HDRP).
  - Update modes: Every Frame, On Enable and On Demand. **Per-cascade on-demand updates** go through `RequestSubShadowMapRendering(index)`, and lights can update on a movement threshold.
  - **Mixed cached shadows**: static casters live in a cached atlas, and dynamic casters are drawn every frame on top after a blit. The docs say this "greatly improves performance both on the CPU and GPU" in mostly static scenes.
  - Caveats: cascades are view-dependent, so the cached ones must be refreshed as the camera moves, and the blit costs extra.
- **Volumetric clouds**, about 2 ms on base PS4 and "**well under 0.5 ms** on a recent high-end GPU" (https://blog.unity.com/es/technology/experience-true-volumetric-clouds-with-hdrp-unity-20212). They are cheap because of:
  - **quarter-resolution** raymarching;
  - temporal reprojection and accumulation with ghosting reduction;
  - a 2D coverage (cloud) map plus an altitude-profile LUT for density, erosion and AO, with only two 3D noises;
  - bounded steps (primary steps scale linearly, light steps "exponentially");
  - reuse in the sky reflection and ambient probe.

  Axcenity's clouds already cost 0.14-0.30 ms at 1080p Ultra, which is on par.

---

## E. Performance techniques for foliage-heavy scenes

### E1. GPU-driven culling and two-phase Hi-Z
Source: Haar and Aaltonen, *GPU-Driven Rendering Pipelines* (AC Unity), https://advances.realtimerendering.com/s2015/aaltonenhaar_siggraph2015_combined_final_footer_220dpi.pdf (primary).
- **Phase 1:** cull objects and 64-triangle clusters against the previous frame's depth pyramid, then draw. **Phase 2:** rebuild the pyramid, retest what was culled, and draw the false negatives. Measured on Xbox One at 1080p:
  - object cull and LOD: 0.28 + 0.26 ms;
  - cluster cull: 0.09 + 0.04 ms;
  - pyramid: 0.06 ms;
  - CPU: 0.2 ms on one Jaguar core;
  - geometry rendering gain: "under 10%".
- **Per-cluster backface culling** removed 10-30% of triangles (20-40% with bounds culling). This doesn't apply to double-sided leaf cards.
- **The shadow win.** Reproject camera depth into light space (about 70 µs) and cull shadow casters that can't affect visible receivers: **30-80% of shadow triangles culled**. For Axcenity this matters more than camera occlusion, because shadows are its dominant pass.
- Nanite's two-pass HZB is the same idea at cluster granularity (Karis 2021, https://advances.realtimerendering.com/s2021/Karis_Nanite_SIGGRAPH_Advances_2021_final.pdf; I did not re-read it here).

### E2. Alpha-tested foliage on RDNA
- AMD's RDNA guide (https://gpuopen.com/performance/, primary): "Avoid `discard` in long running shaders"; "Use depth test equal to skip pixels that were discarded in a Z pre-pass, instead of calling `discard`". 24-bit and 32-bit depth cost the same.
- Horizon Zero Dawn vegetation (Sanders, GDC 2018; notes at https://wiki.kwonsh.duckdns.org/Between-Tech-and-Art--The-Vegetation-of-/Between-Tech-and-Art--The-Vegetation-of-.html, secondary transcription; GDC Vault https://www.gdcvault.com/play/1025530):
  - Measured on PS4: **depth prime 10% plus geometry 11.5% of the frame** in a forest.
  - Alpha test runs only in the cheapest depth pass, with an EQUAL test in the colour pass.
  - Alpha coverage is preserved by a **custom offline alpha mip chain**, so it costs one sample. Alpha textures are small (a 256x128 BC4 is 21 KB with mips).
  - Grass is reduced to about 32 triangles per clump with **no alpha planes**.
  - **Shadow casters are separate LODs that are neither alpha-tested nor animated.**
  - Vegetation is drawn only into cascades 0-1, and **cascade 2 is a heightfield system**.
  - The last LOD has no vertex animation.
- Axcenity status: it already does a cut-out depth fill and then lights with a "nearer than one step" test, plus hashed far cut-outs under TAA. Equal-test variants were measured faster but changed 0.1-1.2% of pixels. Remaining wins come from **fewer vertices and fewer alpha-tested shadow casters**, not from early-Z tricks.
- Alpha-to-coverage: under MSAA, RADV produces a fixed 2x2 dither pattern (Axcenity's own finding). Prefer TAA with hashed alpha (Wyman and McGuire 2017).

### E3. Impostors and card clusters for distant trees
- **Octahedral impostors** (Brucks, https://shaderbits.com/blog/octahedral-impostors): bake N x N views on an octahedral hemisphere or full sphere (albedo, normal, depth) and blend the 3 nearest frames per pixel. A tree becomes 2 triangles. There are no published ms figures, but the vertex cost goes from thousands of triangles to 2.
- **God of War** used **card clusters for LODs and shadow proxies** (Feeley, SIGGRAPH 2019, https://advances.realtimerendering.com/s2019/Feeley-Advances_in_RTR-Interactive_Wind_in_God_of_War-web.pptx; abstract only).
- Horizon put cross billboards in LOD 4-5 (above).
- **The Axcenity-specific finding.** "Leaf cards don't simplify": edge collapse can't merge disjoint quads, so shrubs of 840-4,000 triangles get **no LODs**, and big oaks keep 195k-260k triangles at LOD2 (performance.md). That is why the prepass is vertex-bound at 4.8 ms.

### E4. Visibility buffer
Burns and Hunt 2013 (http://jcgt.org/published/0002/02/04/). Rasterize triangle and instance IDs once, then shade per pixel in compute. It removes "draw twice" (prepass plus scene) and quad-overshading on tiny triangles. It is the basis of Nanite. For Axcenity it would fold prepass vertex work and scene vertex work into one rasterization. That is a big rewrite of a forward+ material system.

### E5. Cached and scrolling shadow cascades
- **Insomniac CSM scrolling** (Acton, SIGGRAPH 2012, https://advances.realtimerendering.com/s2012/insomniac/Acton-CSM_Scrolling(Siggraph2012).pdf).
  - How it works: keep a static-only cache per cascade. Scroll it with the camera, offsetting stored depth for movement along the light. Render only the newly exposed edge slabs, and composite dynamic casters on top.
  - Measured: **about 70% of static shadow rendering skipped**.
- **Cyberpunk 2077** (https://history.siggraph.org/wp-content/uploads/2022/06/2021-Talks-Dybisz_Shadows-Optimizations-in-Cyberpunk-2077.pdf): distant cascades (each about 4x the previous, the last covering the whole map) are **refreshed periodically**; there is offline per-mesh shadow visibility for 12 times of day; and local shadow maps are split into static and dynamic slices.
- UE VSM static caching and HDRP mixed cached shadows (C2, D) are the same principle.

### E6. Shadow proxies for foliage
- Horizon: opaque, unanimated shadow LODs about as cheap as visual LOD3.
- UE: contact shadows for grass and WPO disable distance.
- Axcenity has already turned `CastShadow` off on five ground-cover rules (3.3 ms off RT shadows, casters halved).
- The next step is **opaque crown proxies**, built from the card cloud (for example a coarse voxel shell or convex shell), used beyond a near radius in cascades 3-4 *and* in the RT scene.

### E7. Terrain splat cost
- **Adaptive virtual texturing** (Far Cry 4; Chen, GDC 2015, https://gdcvault.com/play/1021761/Adaptive-Virtual-Texture-Rendering-in): blend all layers once into cached VT pages, so the per-pixel cost becomes one or two VT fetches. I did not read the slides. AC Unity also used a 256k^2 virtual atlas with 128^2 pages and an 8k^2 cache (E1 deck).
- A cheaper alternative: store the **top-2 or top-3 layer indices plus weights** per texel and sample only those. The benefit is unmeasured for Axcenity. Terrain isn't in its current hot list, so this ranks low.

### E8. Variable rate shading
- Gears 5 and Tactics, Tier 2 (https://devblogs.microsoft.com/directx/gears-vrs-tier2/, primary): a Sobel luminance edge mask from the previous frame, reprojected, in 8x8 or 16x16 tiles.
  - RX 6900 XT at 4K: **8-20% (1.0-4.5 ms) saved**.
  - Mask generation: under 0.1 ms.
  - "Diminishing returns at 1080p or lower".
- Software VRS for any hardware: Drobot, CoD MW, SIGGRAPH 2020 (https://advances.realtimerendering.com/s2020/Drobot-SIGGRAPH_2020_VRS_Final.pptx).
- **wgpu does not expose fragment shading rate** (to my knowledge; verify on the current wgpu release). Not actionable now.

### E9. Half-resolution effects and checkerboarding
- Half resolution plus bilateral or depth-aware upsampling is the norm: GTAO at half-res (A2), HDRP clouds at quarter-res (D), Lumen screen probes at 1/16 (C1), and NRD checkerboard and quarter-res modes (F2).
- Horizon used checkerboard 2160p on PS4 Pro (secondary: https://www.dualshockers.com/guerrilla-lighting-tech-horizon-zero-dawn-2160-checkerboard/). TAA-based upscaling has replaced it in modern engines.

### E10. wgpu-specific constraints
- One queue, so no async compute overlap. AMD's guide suggests overlapping the prepass and shadows with compute, which wgpu can't do today (verify).
- No fragment shading rate.
- Ray queries are experimental, and **acceleration-structure updates (refit) are unsupported**: "Updating ... is currently unsupported" (https://wgpu.rs/doc/wgpu/documentation/extensions/ray_tracing/index.html). Every bent BLAS is therefore a full rebuild.
- BLAS **compaction is supported** (`prepare_compaction_async` and `compact_blas`).

---

## F. RT performance on AMD RDNA

### F1. Inline ray-query best practices
- **AMD's RDNA guide** (https://gpuopen.com/performance/):
  - "Trace as few rays as possible."
  - "Skip procedural and non-opaque primitives when you can with explicit ray flags."
  - "Stop traversing after first hit for opaque shadows."
  - "Smaller BVHs are always better", with high-quality build flags.
  - Refit or rebuild only the dynamic LODs you need.
  - Rebuild the TLAS each frame, on the compute queue if possible.
  - Keep one active RayQuery in scope; use 8x4 thread groups; avoid heavy groupshared memory, since traversal uses LDS.
- **RDNA4 hardware** (Chips and Cheese, https://old.chipsandcheese.com/?p=35892, secondary but detailed): 8-wide BVH nodes in 128 B, oriented bounding boxes (one of 104 preset rotations per node), **2x box and triangle intersection rate**, an LDS stack with push8/pop instructions, compressed triangle-pair primitive nodes, and a dedicated instance-transform block. In practice, deep TLAS-to-BLAS transitions are cheaper than on RDNA3, but **non-opaque (any-hit-style) candidates still return to the shader**. In wgpu each one goes through `rayQueryProceed` and a confirm, which is costly on alpha-card forests.
- **RADV** (the only AMD Vulkan driver now that AMDVLK has ended): Phoronix's 2025 retrospective (https://www.phoronix.com/review/radeon-radv-rt-2025) calls RT "the biggest area of performance gains this year". Igor's Lab measured Mesa 26.0 on a 9070 XT: **+25-35% average RT FPS, up to +75%** (https://www.igorslab.de/en/mesa-26-0-radv-catapults-radeon-raytracing-forward-under-linux/). **Benchmark on Mesa 26.x or later.**
- **Instance count.** Axcenity measured this itself: 113k TLAS instances cost 6.3 ms of wgpu submit and 3.8 ms of encoding. Grouping small statics into per-ground-square BLASes cut that to about 20k instances. That is the industry pattern (Lumen's note on overlapping instances, C1).
- **Opaque proxies.** Epic keeps alpha-masked surfaces out of HWRT in MegaLights by default (C4) and traces simplified Nanite fallback meshes. Horizon uses non-alpha shadow LODs. For Axcenity: mark far foliage BLASes **opaque** (FORCE_OPAQUE plus TERMINATE_ON_FIRST_HIT for shadow and AO rays), use **opaque crown proxies**, and keep true alpha only in the near field. It already limits holes to 60 studs, which saved 2.4 ms.
- **Opacity micromaps** would fix alpha in hardware, but they aren't in wgpu or exposed on RDNA4 Vulkan to my knowledge (unverified).

### F2. Denoisers and ray budgets
- **NRD** (https://github.com/NVIDIAGameWorks/RayTracingDenoiser, primary). On an RTX 4080 at 1440p:
  - REBLUR diffuse+specular: **2.55 ms**;
  - RELAX: 3.25 ms;
  - **SIGMA shadow: 0.40 ms**.

  Recommended budgets: specular 1 rpp, or **0.5 rpp checkerboard** ("stable"); diffuse 0.5 rpp checkerboard or **0.25 rpp** with half- or quarter-res denoising. NRD is HLSL, compilable to SPIR-V; porting it to WGSL or naga is a real effort.
- **AMD FidelityFX Denoiser** (https://gpuopen.com/manuals/fidelityfx_sdk/techniques/denoiser/, primary):
  - Shadow denoiser for **at most 1 jittered shadow ray per pixel**. It uses tile classification to skip uniform tiles, a mean and variance neighborhood (17x17, separable), and history clamping.
  - Reflection denoiser keyed on roughness.
  - No ms numbers published.
- **FSR Ray Regeneration** (ML denoiser, paired with FSR 4) is documented with a DX12 backend only (https://gpuopen.com/manuals/fsr_sdk/techniques/denoising/). Vulkan support is unverified, so it's not usable from wgpu.

### F3. GI cost: ReSTIR GI against caching
- **ReSTIR GI** (Ouyang et al. 2021, https://research.nvidia.com/publication/2021-06_restir-gi-path-resampling-real-time-path-tracing): **9.3-166x lower MSE** than path tracing at 1 spp. It still traces 1 path per pixel plus a denoiser, so the cost scales with resolution.
- **AMD GI-1.0** (https://gpuopen.com/download/GPUOpen2022_GI1_0.pdf, primary). Screen probes (1 per 8x8 tile, 8x8 octahedral) with temporal reuse, plus a world-space **hash-grid radiance cache**, at about **1/4 sample per pixel**. Measured on an RX 6900 XT at 1080p:
  - total **1.9-3.1 ms** (99k to 9M triangles);
  - raytracing alone 0.24-0.68 ms;
  - caching and sampling 1.0-1.7 ms;
  - interpolation and denoising about 0.7 ms.

  This is the design class Lumen, Lumen Lite and (probably) GTA VI use: trace sparsely, cache heavily.
- For Axcenity's "DDGI-style probe GI", the published guidance is: keep rays per probe per frame low (amortize over frames), use probe-occlusion visibility (DDGI's Chebyshev depth, or TLOU2's 6-face cube), and gather per pixel from probes rather than tracing per pixel.

### F4. Temporal accumulation and ray budgets (summary)
| Effect | Industry budget | Source |
|---|---|---|
| RT sun shadow | ≤1 ray/px, often half-res; SIGMA/FFX denoise ~0.4 ms | NRD, FFX |
| RT AO | 0.25-0.5 rpp, short rays, half-res | NRD guidance |
| Diffuse GI | ~0.25 spp + radiance cache | GI-1.0, Lumen |
| Glossy reflections | 0.5-1 rpp; reuse GI probes for roughness > 0.4 | Lumen (50-70% saved), NRD |

---

## Ranked recommendations for Axcenity

Ranked by (expected ms saved or look gained) / implementation effort. The ms figures come from Axcenity's Overgrown Street measurements at 1996 x 703 Ultra on the 9070 XT (performance.md); "estimate" marks my projections.

1. **Cache the far cascades (static cache plus scrolling, Acton 2012 / VSM / HDRP mixed).** Re-render cascades 3-4 only on a sun-angle threshold and on scrolled-in edge strips, with dynamic casters composited on top. Cascade 4 costs 5.1 ms on average (10.4 when drawn); expect under 1 ms on average (estimate). Effort: medium-low.
2. **Far shadow casters: opaque, unswayed proxies (Horizon / UE WPO disable distance).** In cascades 3-4, draw trees as opaque crown proxies without wind and without alpha. This is what makes item 1 cacheable and cuts its refresh cost. Expect 2-4 ms when combined with item 1 (estimate). Effort: medium, because it needs a proxy baker.
3. **RT ray budget: near-field RT shadows plus checkerboard plus a SIGMA/FFX-class denoiser.** Trace sun shadows only within about 40-60 studs, at 0.5 rpp checkerboard at Half resolution, and use the cached cascades beyond. Set TERMINATE_ON_FIRST_HIT and cull masks so grass is excluded. RT shadows cost 12.8 ms (Full) or 5.5 ms (Half); expect about 2-3 ms (estimate). Effort: low-medium.
4. **RT AO at 0.25-0.5 rpp, half resolution, short rays, with GTAO for the small scale.** RT AO costs 6.0 ms; expect about 1.5 ms (estimate). Effort: low.
5. **Opaque foliage BLASes beyond the hole radius.** Use crown proxies (from item 2) as FORCE_OPAQUE instances for shadow, AO and GI rays. This removes `rayQueryProceed` candidate loops, as Epic does for MegaLights alpha. Expect several ms of RT time (estimate; the holes radius alone saved 2.4 ms). Effort: medium, shared with item 2.
6. **GTAO with bent normals, GTSO specular occlusion and the multi-bounce fit (Jimenez 2016).** A big look gain under overcast: it stops canopy-shaded wet ground from mirroring the bright sky, and foliage stops going grey. Expected cost about 0.2-0.5 ms (estimate; 0.5 ms on PS4). Effort: low-medium.
7. **TLOU2 probe upgrades.** Add 8-byte probe occlusion (a 6-face depth cube) against leaks, per-pixel masked dominant-light extraction with dynamic deringing, and OBB-corner interpolation for characters. The biggest look gain for "real overcast" at near-zero frame cost. Effort: medium (bake plus shader).
8. **Temporal upscaling as the Ultra default instead of MSAA 4x.** MSAA 4x costs 0.7-1.5 ms at 1440p, and render scale costs 1.09 ms per megapixel. At 67-77% internal resolution, expect 3-6 ms at 1440p (estimate); the TSR example halves the frame. Effort: low, since it already exists, but it needs the owner's sign-off on the look.
9. **Canopy and sky-occlusion height map (RDR2 top-down world AO / APV sky occlusion).** Render foliage density from above into a world-space texture, then darken sky ambient, fog in-scatter and reflections under trees. A large "overgrown" look gain for about 0.1 ms (estimate). Effort: low-medium.
10. **Card-cluster LODs and octahedral impostors for shrubs and trees.** Leaf cards don't simplify today. This targets prepass plants (4.8 ms), scene pixels (3.0 ms) and cut-out depth (2.0 ms); expect 3-5 ms (estimate). Effort: high, because it needs a baker, atlases and transitions.
11. **Shadow-caster culling from reprojected camera depth (AC Unity).** Cull casters that can't shade visible receivers: 30-80% of shadow triangles culled for about 0.1 ms. Do it before camera Hi-Z, which Axcenity measured at only 1.45 ms. Effort: medium.
12. **Wet surfaces with separate specular and diffuse normals (TLOU2).** The water film or puddle gets its own specular normal while the ground keeps its diffuse normal, plus mask-driven rivulets. A look gain for rain scenes at a few ALU per pixel in forward+. Effort: low-medium.
13. **Relit G-buffer reflection probes (RDR2).** Store albedo, normal and depth per probe and relight a few faces per frame, so reflections and specular ambient follow time of day and weather without re-baking (Axcenity's bake takes 30 minutes). A look gain at about 0.2-0.5 ms amortized (estimate). Effort: medium.
14. **Character ambient shadowing: SSCT for dominant-light self-shadowing plus ambient capsules (TLOU2).** About +0.12 ms on PS4 for SSCT, and characters gain cinematic shape under overcast. Effort: medium.
15. **Terrain: top-N layer indices per texel, or an adaptive virtual texture (Far Cry 4).** Gain unmeasured, since terrain isn't in the current hot list. Profile the 7-8 layer cost first. Effort: medium-high.

Not ranked:
- **Camera GPU Hi-Z occlusion and a visibility buffer**: the measured ceiling of 1.45 ms doesn't justify them yet.
- **VRS and async compute**: not exposed by wgpu.
- **Clouds and fog**: already 0.14-0.38 ms, so the work is coherence (sharing a cloud shadow map and sky irradiance), not speed.
