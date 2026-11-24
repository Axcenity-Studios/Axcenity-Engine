# Clouds

A layer of cloud high in the sky, lit by the sun, whose shadows fall on the world (graphics Phase 26). `DrawList::clouds` (`render::Clouds`, `render/src/draw_list.rs`) comes from the world (`scene/src/clouds.rs`); the renderer packs it into the frame (`renderer/clouds.rs`, `CloudsRaw`, `Clouds` in `clouds.wgsl`, compiled after `fog.wgsl` into every shader that includes `lighting.wgsl`).

## The world's clouds

A `Clouds` inside Lighting (the Lighting ribbon's **Clouds**, beside Atmosphere, or the Explorer's Insert menu; it always goes into Lighting, like an Atmosphere). The first one is drawn. Without one there are none.

- `Cover` (0 to 1, default 0.5): how much of the sky is cloud. 0 is a clear sky, drawn exactly as without clouds; 1 is overcast.
- `Density` (0 to 1, default 0.7): how much light the cloud takes away (0.05 per stud at 1, `EXTINCTION`), so thicker clouds are whiter on top, darker below, and cast deeper shadows. 0 is none.
- `Color` (default white): the clouds' own color, lit by the sun and the sky.
- `Speed` (studs a second, default 20; 0 holds them still, below 0 counts as 0) and `Direction` (degrees from +X, east, toward +Z, south; default 0): how they drift on their own, so they move out of the box. The Workspace's `GlobalWind` (studs a second; its level part) adds to that drift, and their shadows move with them. The renderer keeps how far they've gone (`CloudDrift`), moving them on by the wind times the time since the last frame (`DrawList::time`), so a new wind changes their speed, not where they are; when the time counts back (a host starting again) they stay put. Each renderer has its own drift (a probe bake starts from none). The editor keeps drawing frames while clouds are in the wind (`viewport::shader_time`).

An environment preset sets `Cover`, `Density`, `Color` and `Speed` with the sky and the weather, and a sequence's Environment track blends them ([../project/sequences.md](../project/sequences.md#environment-presets-and-the-environment-track)).

## How they're drawn

The layer lies between `Clouds::BOTTOM` (800) and `Clouds::TOP` (1200) studs up. Above each level point, five octaves of value noise (shapes about 1600 studs across, `CLOUD_SCALE`), cut at a level set by `Cover`, say how thick the cloud is (`cloud_cover`, 0 to 1). The shapes are made to look torn by the air, not round (before 2026-11-23 they were smooth lens-shaped puffs):

- They're drawn out one way, half again as long as wide (`CLOUD_STRETCH`), and pulled about by a slower noise before they're cut (`CLOUD_PULL`), so no edge runs straight or round.
- The three finest octaves are ridged (`1 − |2n − 1|`), which gives strands and tufts along the edges.
- The cut is soft and wide (`CLOUD_EDGE`, 0.42 of the noise's range) and eased, so a cloud is mostly thin veil round a thicker middle.
- `Cover` is about the share of the sky that's cloud: 0.3 is scattered wisps, 0.5 to 0.6 broken cloud, from 0.85 an overcast, and at 1 there are no holes (its thin places are thin cloud, lighter from below).

A cloud `cover` thick lies about a third of the way up the layer and grows both ways until it fills it (`cloud_span`), thickest through its middle; before, it filled the layer from the bottom up with a rounded top. With fine detail (High and Ultra), two octaves of 3D noise drawn out the same way eat the thin parts away to wisps and leave the thick middle whole.

- **In the sky** (`clouded_sky`, from `fs_sky`): the sky behind is dimmed by the cloud in front of it and the cloud's light is added, step by step through the layer (integrated exactly over each step), each step lit by the sun through a dual-lobe phase (a silver lining toward the sun) and by the sky's colors. Deep in cloud, where none of the sun's light arrives straight, a share still arrives scattered many times from every side (`cloud_glow`: one part in 1 + 0.075 × the cloud's depth, the two-stream answer for droplets, times `CLOUD_GLOW`), so the underside of an overcast or storm sky is grey, lighter where the cloud is thinner, and not the dimmed blue of the sky behind it. Where the noise stands well above the cut the cloud more than fills its layer (`cloud_fill` above 1; `cloud_cover` is `cloud_thick` of it), and its underside shows less of that scattered light and of the sky's, down to half where it's heaviest (`cloud_belly`, `CLOUD_BELLY`): every part of an overcast is too deep to see through, so without it the sky was one flat grey from horizon to horizon. Broken and fair-weather cloud never fills that far and is drawn as before. Far clouds fade into the sky from 5000 to 20000 studs (`CLOUD_FAR`), and a ray is marched through at most 4000 studs of the layer (`CLOUD_MARCH`). The fog then fades the clouded sky like the sky. Clouds are part of the sky: they're drawn behind everything, so from above them the ground isn't hidden, and they aren't in the environment maps' sky.
- **Shadows** (`cloud_shadow`, in `light_surface`, so on parts, models and terrain, and in the volumetric fog's sun): the sun's light at a point is dimmed by the cloud where its path crosses a quarter of the way up the layer, its thickness there over the sun's height. A share of the light gets through even thick cloud (`cloud_light_through`: light scattered many times inside), so the ground under a cloud is dim, not black. Every cloud quality casts the same shadows.

- **The sky's light under them** (`under_clouds` in `lighting.wgsl`, inside `sky_diffuse` and `sky_specular`, so on every surface, in ray traced GI's and reflections' misses, in the fog's own light and the Path Traced view): the environment maps and the sky's harmonics hold the clear sky, since clouds aren't baked into them, so until 2026-11-23 an overcast world was still lit, and its puddles still mirrored, a clear blue sky the clouds hid. From broken cloud on (`OVERCAST_FROM`: none at a `Cover` of 0.4, all of it from 0.9) the sky's light turns the grey of its own brightness, tinted half-way to the clouds' `Color`, and a thick layer keeps 0.8 of it (`OVERCAST_LIGHT`). How thick is `1 − exp(−¼ × Density's extinction × the layer's depth)`, so a veil of little `Density` changes nothing however much sky it covers. It's by the world's `Cover`, not by the cloud over each point: the sky's light comes from all of it. Fair-weather cloud (under 0.4) draws exactly as before. Still not there: the clouds' own shapes in reflections.

The **cloud quality** graphics setting (`CloudQuality`, [settings.md](settings.md); `UserGameSettings.CloudQuality`) sets the sky's cost: Low a flat sheet a quarter of the way up (one look-up per pixel), Medium 16 steps lit by the cloud above each step, High 32 steps with fine 3D detail eroding the edges and 3 steps toward the sun, Ultra 64 and 6. Steps start at a per-pixel offset (`pixel_noise`) so they don't band.

## Cost

Clouds add no pass, target or draw call: they're worked out in the sky pass's pixels, and every lit pixel skips them on one uniform test while there are none (`Clouds.color.w`). The sky pass is drawn after solid items and before see-through ones, with a depth test that passes only where no solid item was drawn, so the sky (and its clouds) is worked out only for the pixels that show it.

## Tests

- `render/src/renderer/tests/clouds.rs` (GPU): the clouds overhead and their shadows on a floor move by the wind's distance (the middle row's best-matching shift is the wind's 100 studs in pixels), and stay still without wind; Cover 0 draws exactly the clear frame, clouds add no draw calls or target bytes, and taking them out restores the frame; every cloud quality draws clouds and shades the ground the same.
- `render/src/renderer/tests/clouds.rs` (GPU), `an_overcast_turns_the_skys_light_grey`: a floor lit by a blue sky alone stays as blue under fair-weather cloud and under a veil of no density, and turns grey and a little dimmer under a thick overcast.
- `render/src/renderer/clouds.rs`: none by default or without density, each quality's steps and detail, the drift following the wind (level part only), keeping its place when the wind changes or time counts back.
- `scene/src/clouds.rs`: a new world has none; Lighting's Clouds take Cover (held to 0 to 1), Density and Color, drift east at 20 studs a second by default, turn with Direction and scale with Speed (not below 0), add the GlobalWind, and Cover 0 or removing it is none.
- `player/src/clouds_tests.rs` (GPU): a world's Clouds shade part of the floor, its GlobalWind moves the shadows, still air at Speed 0 keeps them, their own Speed moves them, and destroying the Clouds gives the frame without them.
- `render-types/src/graphics.rs`: each preset's cloud quality.
