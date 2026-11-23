# Weather

Rain and snow, wet and snow-covered surfaces, sky-visibility occlusion, manual interior/exterior masking, and dynamic snow footprints (graphics Phase 27 + Section 4). One state holds it all: Lighting's `Weather`. `scene/src/weather.rs` reads it, eases toward it, updates a 64×64 world-space `WeatherField` around the camera, and `render::Weather` (`DrawList::weather`) + `DrawList::weather_field` drive the shaders (`Frame::weather`, `Frame::weather_bounds`, `@binding(19) weather_field`, `weathered` in `lighting.wgsl`, and snow lift in `terrain.wgsl`).

## The world's weather

A `Weather` inside Lighting (the Lighting ribbon's **Weather**, the Explorer's Insert menu, or the Properties panel's **Weather & Surface Detection** section; it always goes into Lighting). The first one is used. Without one, surfaces are dry and nothing falls.

- `Enabled` (bool, default `true`): quick toggle for the weather system. When `false`, precipitation stops and surfaces dry and clear over `TransitionTime`.
- `Precipitation` (`Enum.Precipitation`: None, Rain or Snow; default Rain): what falls around the camera.
- `Intensity` (0 to 1, default 0.5): how hard it falls. Rain wets surfaces at least this much, and falling snow covers them at least this much.
- `Wetness` and `SnowCover` (0 to 1, default 0): how wet surfaces are and how much snow lies on them, whatever is falling, such as puddles after the rain or old snow.
- `SnowThickness` (0 to 2 studs, default 0.35): customizable snow layer thickness controlling terrain snow lift, normal smoothing, and footprint depression depth.
- `TransitionTime` (seconds, default 10): how long a change takes to ease in. Rain starting and stopping, surfaces wetting and drying, and snow settling and melting each move at most one whole change per `TransitionTime`. 0 changes at once.
- `OcclusionEnabled` (bool, default `true`): enables precipitation collision occlusion and automatic top-down sky-visibility surface detection.
- `FootprintsEnabled` (bool, default `true`) and `FootprintFadeTime` (seconds, default 30): stamps dynamic footprints and trails into snow where characters or unanchored parts move, fading back in over `FootprintFadeTime` (faster during snowfall).
- `SurfaceMask` (string, default `""`): manual interior/exterior cell overrides painted in the editor (`"x,z:I;x,z:E"`).
- The Workspace's `GlobalWind` blows the rain and snow.

## Easing & Surface Detection

`settled` is the weather once eased in. `build_draw_list` puts it into the list, so a world drawn on its own (a still, a test, a probe bake) shows it settled. The host's `Particles` (player and editor viewport) owns a `WeatherState`. `Particles::update_against` eases it toward the world's weather, incrementally casts up to 512 top-down sky-visibility rays per frame via `Surfaces` (`physics::surface_hits`) across a 64×64 stud grid around the camera, tracks moving characters and unanchored parts for snow footprints, and builds `WeatherField`. `Particles::add_weather` puts the eased weather, `WeatherField`, and occluded rain/snow sprites into the draw list.

## Surfaces & Exterior Masking

`light_surface` (and `toon_surface`, `cloth_surface`, `skin_surface`) runs `weathered` first, so parts, models, surface shaders and terrain all get it. See-through items get it too. Emission isn't changed.

- **Exterior Masking**: `weather_sample` combines `Interior` `GraphicsVolume` boxes (`FogEffect.Interior == true`), automatic sky-visibility roof heights (`weather_field.x`), and manual `SurfaceMask` overrides (`weather_field.y`: `-1` Interior, `0` Auto, `+1` Exterior). Surfaces under roofs, inside `Interior` volumes, or painted Interior stay dry and bare.
- **Wet**: a non-metal's color is darkened toward half (water fills its pores), and the roughness is lowered toward 0.08 (a film of water), so the sun's reflection is sharper. Undersides stay drier (by the surface's own normal).
- **Snow & Footprints**: covers exposed surfaces by their own normal's height, `clamp(1.5 × cover − 2.5 × (1 − up))`. `SnowThickness` lifts exposed upward-facing terrain vertices (`terrain.wgsl`) and scales footprint normal perturbation. Stamped footprints (`weather_field.z`) depress terrain snow lift, perturb the snow normal for concave 3D rims, and compact/darken the snow inside the trail.

## Rain and snow

They fall in a 64 × 48 × 64 stud box around the camera (`BOX`). Each drop wraps from one side to the other, so the camera never runs out of them. There are up to 2500 raindrops and 2000 snowflakes. As many are drawn as the eased intensity says, as sprites that fade toward the box's sides. When `OcclusionEnabled` is on, drops below the cell's sky-visibility `roof_y`, inside painted `Interior` cells, or inside `Interior` `GraphicsVolume` boxes are culled so precipitation does not pass through roofs. Raindrops fall at 60 studs a second (±20% each) and show a thin streak (`rain_streak`). Snowflakes fall at 5 studs a second, sway 1.5 studs a second, and show the soft Circle picture. Both are carried by `GlobalWind`.

## Editor Inspector & Paint Tool

`editor/src/weather_inspector.rs` shows a **Weather & Surface Detection** section in Properties (when nothing, `Lighting`, or `Weather` is selected) with:
- Quick `Weather Enabled` toggle, particle preset buttons (`None`, `Rain`, `Snow`), `Intensity` / `Snow Depth` sliders, and one-click presets (`Clear`, `Drizzle`, `Rain`, `Storm`, `Light Snow`, `Heavy Snow`, `Blizzard`, `Wet After Rain`).
- Custom saved profiles stored per project in `.axcenity/weather_profiles.json`.
- **Surface Mask Paint Tool** (`Paint Interior`, `Paint Exterior`, `Erase (Auto)` + brush radius + live 3D overlay) to manually override automatic sky-visibility detection.

## Cost

Weather adds no render pass, render target, or draw call apart from the precipitation sprites while something falls. Every lit pixel skips `weathered` on one uniform test while surfaces are dry and bare.

## Tests

- `render/src/renderer/tests/weather.rs` (GPU): wet surfaces are darker and glossier; snow whitens tops and leaves walls bare; clear weather draws identically with zero cost; `WeatherField` roof occlusion and manual interior/exterior overrides shelter or expose surfaces; snow footprints visibly compact and darken snow.
- `scene/src/weather.rs`: easing, `Enabled = false` clearing, wind drift, roof occlusion of raindrops, `SurfaceMask` parse/format/paint round-trip, and moving character snow footprints fading over `FootprintFadeTime`.
- `player/src/weather_tests.rs`: host frame wetting/drying, Lua `Enum.Precipitation`, save/reload parity.
- `editor/src/weather_inspector.rs`: built-in profile application round-trip and custom profile disk persistence.
