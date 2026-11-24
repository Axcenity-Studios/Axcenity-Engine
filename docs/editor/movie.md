# Render Movie

Read this when changing how a sequence is rendered to a video file or a PNG sequence: the Render Movie window, the job that draws the frames, the `ffmpeg` side, or `axcenity-editor --render-movie`. The sequence itself (the file, transitions, what tracks can key) is [../project/sequences.md](../project/sequences.md).

## What it is

A movie is the open sequence drawn frame by frame at any size and quality, not a recording of the screen. `crates/editor/src/movie.rs` and `movie/`:

| File | What it holds |
|---|---|
| `movie.rs` | `MovieSettings`, and what they work out to: the size (`Resolution`, `Aspect`), `frame_count`, `frame_time`, the graphics settings, `default_output`. |
| `movie/job.rs` | `MovieJob`: `start`, then `step` draws one frame. |
| `movie/encode.rs` | Finding `ffmpeg`, `Encoders::parse`, `ffmpeg_args`, the writer thread (`Sink`), `write_png`, `blend`. |
| `movie/window.rs` | The Render Movie window (`MovieWindow`). |
| `movie/cli.rs` | `--render-movie`. |
| `app/movie.rs` | The editor's side: `start_movie`, `movie_frame`, the Output panel's line. |

## Using it

1. Open a sequence in the **Sequencer** tab. To render part of it, mark a play range with **In** and **Out**.
2. Press **Render Movie** on the Sequencer's own row (beside In and Out), or run "Render Movie…" from universal search (Ctrl+K).
3. Set what the window offers, then press **Render**.

The window (`widgets::tool_window`, Escape closes it):

- **What to render**: all of the sequence, or the play range.
- **Picture**: **Resolution** (1280 × 720, 1920 × 1080, 2560 × 1440, 3840 × 2160, or Custom with a width and height), **Shape** (16:9, 2.39:1 scope, 4:3, 1:1, 9:16) and **Frame rate** (24, 25, 30, 50, 60). A scope picture keeps the preset's width (3840 × 1608 from 4K); the others keep its height. Sizes are made even, and no bigger than the GPU's largest texture. Picking the scope shape turns the letterbox bars off.
- **Quality**: **Graphics** (Low to Ultra; Ultra unless the adapter draws on the CPU, whatever the editor's own setting), **Ray tracing** (shadows, ambient occlusion, global illumination and reflections at High; only where the editor's device has ray queries, which needs Settings > Graphics > Hardware ray tracing and a restart), **Motion blur** (off: the world's MotionBlurEffect; on: the strength set here) and **Warm-up** frames. **Letterbox bars** leaves the sequence's bars out when unticked. **The game's own UI** hides the world's ScreenGuis when unticked; the sequence's fades, bars and subtitles are always drawn. Depth of field and the other lens effects come from the world (Lighting's effects, GraphicsVolumes).
- **Output**: **Format** and the file. The default file is `<project>/movies/<sequence name>` with the format's extension, until a path is typed or picked with **Browse…** (the system's Save dialog, `rfd`).
- **Render**, then a progress bar (frame n of N), the time elapsed and left, the last frame drawn (small, a few times a second) and **Cancel**. When it ends the Output panel says "Rendered the movie `<path>` (…)" and **Open folder** shows it in the file manager.

The editor stays usable: `movie_frame` runs `MovieJob::step` until 30 ms of the editor's frame are used (`FRAME_BUDGET`), so a slow frame makes the editor draw less often, never freeze for the whole movie. Closing the window doesn't stop a movie. No movie starts during Play. A job has a copy of the world, so the edited world, its undo history and the Sequencer's preview aren't touched.

`<project>/movies/` is local output: Build Game doesn't pack it (it packs `scripts`, `objects`, `assets`, `levels`, `ui` and the project's own files), a Collaborate session doesn't share it (`collab::files::is_synced`), and a newly linked project's `.gitignore` leaves it out of the repository (`repo::repository::GITIGNORE`; add `movies/` to an older project's `.gitignore` by hand). Back Up Project copies the whole folder, movies included.

## Formats and ffmpeg

The engine has no video encoder. A video is made by running the `ffmpeg` program and writing each frame's raw RGBA pixels to its input (`std::process`, no shell, each argument its own string: `ffmpeg_args`).

- `find_ffmpeg` uses the file the `AXCENITY_FFMPEG` variable names, else the first `ffmpeg` (`ffmpeg.exe` on Windows) in the folders of `PATH`.
- `probe` runs `ffmpeg -hide_banner -encoders` on a thread when the window first opens, and `Encoders::parse` reads the video encoders from it. **Check again** runs it again.
- The formats offered (`Encoders::formats`):

| Format | Encoder | Written as |
|---|---|---|
| H.264 video (.mp4) | `libx264` (CRF 16, preset slow), else `libopenh264`, else a hardware encoder (`h264_nvenc`, `h264_qsv`, `h264_amf`, `h264_videotoolbox`, `h264_mf`) at a bit rate by the size (`bit_rate`) | `yuv420p`, BT.709, `+faststart` |
| AV1 video (.mp4) | `libsvtav1` (CRF 24, preset 6) | the same |
| Lossless FFV1 video (.mkv) | `ffv1` (level 3, every frame whole) | `bgr0`, every pixel as drawn |
| PNG image sequence | none: the `png` crate | `<folder>/<name>_0001.png`, RGB |

- A PNG sequence is always offered. With no `ffmpeg`, the window says so and what to install.
- `ffmpeg` lists a graphics card's encoder whether or not the card is there, so a hardware encoder can fail when the movie starts; the error shows what `ffmpeg` said.
- Frames are written on a thread (`Sink`), at most two ahead, so encoding runs beside drawing and a slow encoder slows the renderer. A cancelled or failed video's file is removed; PNGs already written stay.
- A movie has no sound yet.

## The job

`MovieJob::start(device, queue, MovieSource, &MovieSettings, ffmpeg)` takes a world, a `SequenceFile`, `Assets`, the stylesheets and the camera to use while no cut is in charge (the Scene view's). It makes its own `TextureRenderer` at the movie's size on the device it's given (the editor's own, so the viewport keeps drawing), and frees it when the job is dropped. Each `step` returns `Progress`: `Loading`, `Warming`, `Frame` or `Finished`.

Each kept frame, at `frame_time(start, fps, n)`:

1. `SequenceRun::apply` puts the world there: keyed properties, the Environment track's blend of presets (the job reads the project's own with `with_presets`), clips on Animators, the overlay (fades, bars, subtitles), the camera cut. The overlay's subtitle is 28 pixels tall on any screen, which suits 1080 pixels of height; the job scales it by the movie's height once (`size_subtitle`), so it's the same share of a 4K picture as of a small one. The game's own ScreenGuis are laid out at the movie's size as they are in the game (`ScaleMode` Fit scales, Fixed doesn't).
2. The Animators, the particles and the weather move on by `1 / fps` (`Particles::update_against` with `play::particle_surfaces`, so rain and snow stop at roofs). Particles move on the CPU, as in the one-off film tool.
3. `build_draw_list_posed` from the cut's camera, the UI laid out over it, `particles.draw`, `add_weather`, `list.time` the sequence's time. A motion blur strength set in the window replaces `list.lens.motion_blur`.
4. Render, wait for the GPU, `read_pixels`, hand the frame to the `Sink`.

Nothing reads a clock, so two runs give the same frames (tests `a_sequence_renders_to_numbered_pngs_with_its_cut_fade_and_bars`, which renders twice, and `particles_and_weather_move_on_a_fixed_step_whatever_the_machines_pace`).

- **Warm-up** (default 48 frames, `DEFAULT_WARM_UP`): before the first kept frame, frames are drawn and thrown away with the world held at the first frame, the shader clock running up to it and the particles running 5 seconds ahead, so ray traced light, probes, TAA and uploads have settled and rain is already falling.
- **Loading**: while `Assets::texture_progress` says textures are still being read in the background, a step draws nothing and returns `Loading` (at most 3000 times in a row).
- **Camera cuts**: at a cut the renderer forgets what it kept from the frame before (`TextureRenderer::forget_history`: TAA's picture, motion vectors, the fog grid) and `cut_warm_up` frames (a quarter of the warm-up, at most 16) are drawn and thrown away from the new camera. Without it the first frames after a cut ghost and motion blur streaks the whole picture.
- **Dissolves**: while `Applied::dissolve` is set, the world is drawn twice, the outgoing shot with the main renderer and the incoming one with a second renderer made for it, and the two pictures are mixed (`blend`, byte by byte in sRGB as video editors do). When the dissolve ends the second renderer carries on alone, its history unbroken. It needs twice the GPU memory for those frames.

## Command line

```text
axcenity-editor --render-movie <project folder> --sequence <path or name> --out <file or folder>
    [--size WxH] [--fps N] [--quality Low|Medium|High|Ultra] [--format h264|av1|ffv1|png]
    [--range FROM:TO] [--warm-up N] [--motion-blur 0..1]
    [--no-ray-tracing] [--no-letterbox] [--no-ui]
```

No window opens. It loads the project, opens a GPU device of its own (with ray queries where the card has them), runs the same job to its end and prints progress. `--sequence` takes a path from the project folder or a name (`Intro`). Without `--format` the file's name says: `.mp4` is H.264, `.mkv` FFV1, anything else a folder of PNGs. Before the first camera cut the view is a fixed one of the world's middle. It exits 0 when the movie was written.

## Not yet

- Sound: audio sections aren't rendered.
- Level sections aren't loaded (nor in the Sequencer's preview).
- No queue of several sequences, no image formats beyond PNG, no HDR or alpha output.

## Tests

- `movie/tests.rs`: `a_movie_renders_a_sequences_environment_track` (a project's preset keyed on the Environment track darkens the frames from day to night; a missing preset is skipped), frame counts and times, sizes, the range, graphics settings, output paths, `Encoders::parse` and the encoder chosen, `ffmpeg_args`, `find_in`, `blend`, the command line; on a GPU: `a_sequence_renders_to_numbered_pngs_with_its_cut_fade_and_bars` (frame count, size, the cut, the fade, the bars, two identical runs, a range without bars), `a_dissolves_frames_are_between_its_two_shots`, `particles_and_weather_move_on_a_fixed_step_whatever_the_machines_pace`, `subtitles_keep_their_share_of_the_picture`, `a_video_needs_ffmpeg_and_says_so`; with the machine's `ffmpeg` (skipped without one): `ffmpeg_encodes_every_format_it_offers`, `a_sequence_renders_to_a_video_file`.
- `app/movie_tests.rs`: the Sequencer's button and the window's sections, a movie rendered through the app with the Output panel's line, a cut's transition written and undone, the sky keyed from the Sequencer.
- `render` `renderer/tests/temporal.rs`: `forgetting_history_draws_the_frame_after_a_camera_jump_as_a_first_frame`.
