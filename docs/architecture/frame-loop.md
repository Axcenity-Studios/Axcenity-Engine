# Frame order

The fixed order of one frame. Do not reorder it casually.

The order is one table, `STANDARD` in `crates/sim/src/steps.rs` (roadmap M1): every step, its stage and the hosts that run it. `crates/sim/src/golden.rs` writes out each host's order, so moving a step fails a test. The player, editor Play, the dedicated server and the test runner all run that table, each through its own `HostIo` (input, sound, network, drawing). Each step is named and timed in a profiler span. The steps by stage, and which hosts run each (`the_frame_loop_doc_lists_the_schedule` checks this table against the schedule and prints the new one when a step changes):

| Step | Stage | Player | Editor Play | Server | Test runner |
|---|---|:-:|:-:|:-:|:-:|
| `net.receive` | NetReceive | ✓ | ✓ | ✓ |  |
| `characters.step` | Input | ✓ | ✓ |  | ✓ |
| `characters.landed` | Input | ✓ | ✓ |  | ✓ |
| `humanoids` | Input | ✓ | ✓ |  | ✓ |
| `players` | Input | ✓ | ✓ | ✓ | ✓ |
| `agents.step` | Input | ✓ | ✓ |  | ✓ |
| `agents.sensed` | Input | ✓ | ✓ |  | ✓ |
| `input.events` | Input | ✓ | ✓ |  | ✓ |
| `prompts` | Input | ✓ | ✓ |  | ✓ |
| `tools` | Input | ✓ | ✓ |  | ✓ |
| `abilities` | Input | ✓ | ✓ |  | ✓ |
| `scripts.step` | PreSimulation | ✓ | ✓ | ✓ | ✓ |
| `scripts.stepped` | PreSimulation | ✓ | ✓ | ✓ | ✓ |
| `physics.hold` | PreSimulation | ✓ | ✓ | ✓ |  |
| `physics.kinematic` | PreSimulation | ✓ | ✓ |  | ✓ |
| `physics.shatters` | PreSimulation | ✓ | ✓ | ✓ | ✓ |
| `physics.step` | Simulation | ✓ | ✓ | ✓ | ✓ |
| `touches.began` | PostSimulation | ✓ | ✓ | ✓ | ✓ |
| `touches.ended` | PostSimulation | ✓ | ✓ | ✓ | ✓ |
| `breaks` | PostSimulation | ✓ | ✓ | ✓ | ✓ |
| `health` | PostSimulation | ✓ | ✓ | ✓ | ✓ |
| `heartbeat` | Heartbeat | ✓ | ✓ | ✓ | ✓ |
| `audio` | PreAnimation | ✓ | ✓ |  | ✓ |
| `particles` | PreAnimation | ✓ | ✓ |  | ✓ |
| `animators` | Animation | ✓ | ✓ |  | ✓ |
| `tools.place` | Animation | ✓ | ✓ |  | ✓ |
| `levels.loaded` | PreRender | ✓ | ✓ |  | ✓ |
| `loading.screen` | PreRender | ✓ | ✓ |  | ✓ |
| `render_step` | PreRender | ✓ | ✓ |  | ✓ |
| `camera.move` | PreRender | ✓ | ✓ |  | ✓ |
| `camera.follow` | PreRender | ✓ | ✓ |  | ✓ |
| `graphics.changed` | PreRender | ✓ | ✓ |  |  |
| `streaming` | PreRender | ✓ | ✓ |  |  |
| `draw.scene` | PreRender | ✓ |  |  |  |
| `draw.ui` | PreRender | ✓ |  |  |  |
| `net.send` | NetSend | ✓ | ✓ | ✓ |  |

The server runs `physics.shatters` before `physics.hold` (its one `DRIFT` line). The numbered steps below are the same frame, described by what happens in it.

0. Network receive (Phase 63): `net::Network::receive` does the `NetworkService` requests scripts made last frame (`ScriptRuntime::take_network_requests`), then takes in what arrived: players who joined or left go in or out of `Players` (`PlayerAdded`, `PlayerRemoving`), a failed or refused connection fires `ConnectionFailed`, and a finished LAN search resumes `FindLanGames`.. When `Players.CharacterAuthority` is `"Server"` (MP6a), the server then moves each remote player's character by the moves their device sent (`Network::step_characters`), and a joined device takes the server's answers back into its prediction. See [networking.md](networking.md).
1. Read input. Keys for the UI's focus (Tab, the arrows, Enter, Escape while something is focused) and game controllers' buttons move focus first, through the host's `UiInteraction` (`docs/ui.md`, Keyboard and controller focus); a confirmed button fires `Activated` here, as a click does. When a `ThirdPersonController` or `FirstPersonController` in the Workspace has a body, `character` steps it here (walk or run, jump and fall, crouch, slopes and steps, and its camera: following and wheel zoom, or at the eyes) before scripts run. Only the first such controller receives the keys; the others walk their `MoveDirection` (Phase 46). Every controller with a body sits on a Seat it touches, or that `seat:Sit` gave it, and stands up when asked (Phase 39); hosts then fire `Seated` (`ScriptRuntime::fire_seated`, from `Controllers::take_seat_changes`), then fall damage and `Landed` (`axcenity_gameplay::fire_landed`, from `Controllers::take_landings`, Phase 43). Then `axcenity_gameplay::step_humanoids` (Phase 46) makes Humanoids' missing pieces, walks each `MoveTo` on by its controller's `MoveDirection` (used on the next controller step), works out each Humanoid's state and fires `MoveToFinished`, `StateChanged` and `Died`; its Animator gets the matching state. A Health that dies at any step kills its Humanoid at once (from `fire_changes`). Then `ScriptRuntime::step_players` (Phase 47) joins the player and gives them their first character, removes spawn ForceFields whose time is up, and respawns players whose `RespawnTime` has passed; `PlayerAdded`, `CharacterRemoving` and `CharacterAdded` fire (from `fire_changes`). The host draws the leaderboard (`ui::leaderboard_items`, while `leaderboard_shown`) over the UI with the hotbar. After the frame's input reaches scripts (`ScriptRuntime::input_events`), `ScriptRuntime::step_prompts` shows the ProximityPrompts in reach and follows their actions, and follows the mouse for ClickDetectors, firing their events (Phase 44); the host draws `shown_prompts` over the UI and shows `click_cursor`. Then `axcenity_gameplay::step_tools` picks up the dropped Tools characters touch and follows the player's hotbar keys, Drop and Primary (Phase 45); the host draws the hotbar (`player_inventory`, `ui::hotbar_items`) under the prompts. Then `nav` steps every `NavigationAgent` with a body toward its `Destination`, or at its `Velocity`, and has it look and listen for the player character; hosts fire `PlayerSensed` for the senses that just started (`docs/architecture/navigation.md`).
2. Move tweens on (`TweenService`, setting their properties), then resume coroutines whose wait has expired, and fire queued events
3. Hold that character's and the agents' parts kinematic (`Controllers::keep_kinematic`, `Agents::keep_kinematic`), hand scripts' `fracture:Shatter` calls to physics (`ScriptRuntime::apply_shatters`, Phase 42), then step physics at a fixed 60 Hz (accumulator; may run 0–N steps per frame). Other bodies keep the same gravity and friction.
4. Fire `Touched`, then `TouchEnded` (Phase 38), then `Broken` and `Shattered` (`ScriptRuntime::fire_breaks`, Phase 42), then DamageZones, regeneration, their Health events and health bars following their Health (`axcenity_gameplay::step_health`, Phase 43), then `RunService.Heartbeat`; then particles and Animators move on (Animators pose their MeshParts and fire `StateChanged`), and Tools in hand move to the posed right hands (`axcenity_gameplay::place_tools`, Phase 45)
5. Move the loading screen on (a LocalScript's `RemoveDefaultLoadingScreen`, the player's asset preload, then `LoadingGui::advance`, which may turn its ScreenGui off), then update UI layout (`taffy`) and shape text
6. Build the draw list (UI layout included), with moving parts drawn between physics steps (`Sim::blends`, `Physics::blend_alpha`, roadmap G3; `docs/architecture/physics.md`), then network send (`Network::send`, Phase 63; a joined device first records its character's step as a move with `Network::record_move`, MP6a), then render

The player runs the schedule's steps from `App::frame`, then renders (`crates/player/src/main.rs`; its `HostIo`, in `host_io.rs`, is the input, sound, streaming and drawing). The editor runs steps 0-4 only during Play (`Session::frame` in `crates/editor/src/play.rs`, through its `PlayHost`), then network send at the end of the frame; its viewport lays out UI at step 5 using its current pixel size, then builds and draws step 6. Play's network step talks to its other Play devices over the in-memory loopback (multi-client Play, Phase 64); it can't host or join a real game, so scripts' `Host` and `Join` fire `ConnectionFailed` and a LAN search finds nothing. Editor Play runs the player's order without drawing (roadmap M1): sound actions right after Heartbeat, then particles, Animators and Tools, then the render step, and world streaming around the camera drawn this frame. Its particles and streaming belong to the `Session`, so they move on whether or not a Scene tab is open; the Scene viewport borrows them to draw (`Session::swap_effects`), and moves its own particles only while editing.

**When the next frame starts** (ADR-0052). A frame is this work on the CPU and then the GPU's drawing. While both fit in one refresh of the display, the next frame waits for the display to show this one (under Wayland, winit's `Window::pre_present_notify`). When frames keep missing the refresh, it starts at once, while the GPU still draws this one, and the renderer waits only for the frame before that (`axcenity_render::FramePacer`, `TextureRenderer::set_one_frame_ahead`; the player's `App::pacer`, the editor's `EditorApp::pace_display`). Measurements: [rendering/performance.md](../rendering/performance.md#the-cpu-and-the-gpu-took-turns-frame-pacing-desert-homestead).

**What a window's state does to its frames** (ADR-0052; the player's `window::next_frame` and `App::frame`, the editor's `EditorApp::pace_display`). The player asks the display for its refresh again every 30 shown frames (`window::asks_refresh_on`): under Wayland a new window isn't on a display yet and has none to say.

| The window | The game's steps | Drawing |
|---|---|---|
| Has the keyboard's focus | Every frame | Every frame, paced as above |
| Seen, another window has the focus | Every frame | Every frame, always waiting for the display: under Wayland that is the slower pace for a heavy scene (30 frames a second where 90 are drawn with the focus); on Windows, macOS and X11 nothing changes |
| Covered by other windows | 30 times a second | None, from a quarter of a second after the display stops asking for frames (Wayland) or wgpu says `Occluded` (macOS). Windows doesn't say a window is covered, so it draws as if seen |
| Minimized | 30 times a second | None |
| The editor without focus | Play as before | Its frames wait for the display; it draws when something asks, as always |

A game keeps stepping in every state, because a host has other players to answer and sounds keep playing. Paul's choice of 2026-11-25 for the window seen without focus: keep the slower pace and the lower power, since nothing under Wayland tells a hidden window from an unfocused one.

**A policy that can be set (considered 2026-11-25, not built).** What each state does is fixed in code. Paul asked for a background rendering policy to be considered for later. What it would take: one value for each of three states (focused, without focus, hidden: minimized or covered) out of *full pace*, *the display's pace*, *a frame rate cap*, *steps only* and, for a game alone on its device, *paused*; a place for a game to say what it needs (a multiplayer host must not pause; a film render must not slow down) and a place for the person at the machine to say what they want (`UserGameSettings`, the editor's Settings), the stricter of the two winning for power and the game's need winning for stepping. It belongs with a frame rate cap, which doesn't exist either: both are "draw less than you could, on purpose". Nothing above needs it to work; it is noted in ADR-0052 under what to revisit.

`axcenity-server` runs the schedule's server steps (step 0, steps 2-4 without characters, agents or animation, then network send), with its own network, printing and far-physics sleep as its `HostIo` (`crates/server/src/main.rs`). The game test runner's `TestGame::frame` (`crates/testing/src/game.rs`, Phase 53) runs the schedule's test-runner steps at a fixed 1/60 second: steps 1-4, then the render step's scripts and the game camera, with no audio or drawing. Visual-debug overlays (colliders, raycasts, bounds, emitters, the navigation mesh, agent paths) are painted in the editor viewport after the scene texture; they do not change this order, and the player never draws them.

Since Phase 16, change events (`Changed`, `ChildAdded`, ...) for whatever changed fire at every step that runs scripts, before and after it, so physics' changes fire at the start of step 4 (see `docs/scripting/execution.md`).

Since Phase 32, `RunService.Stepped` fires on both sides between steps 2 and 3 (`ScriptRuntime::stepped`), and just before step 6 builds the frame `ScriptRuntime::render_step` runs LocalScripts' `BindToRenderStep` functions in priority order and then `RenderStepped`, before `take_camera_move` and `step_camera`, so a camera they move is drawn this frame. The player calls it right before `end_camera`; the editor's Play at the same place, after Animators, so `RenderStepped` sees this frame's poses in both.

Since Phase 14, step 1 also hands LocalScripts the camera (`set_camera`) and the key presses since the last frame (`ScriptRuntime::input`), and step 2 first delivers RemoteEvent messages to each side (server, then client) before resuming its tasks. Where a LocalScript moved the camera (`take_camera_move`) is applied just before step 6, unless a character controller owns the view; then that move is dropped and the controller's camera is drawn. Since Phase 31, step 1 also passes the view's size (`set_viewport_size`), and just before step 6 `step_camera` moves a Follow or Track camera after its subject and moves shakes on; its `GameCamera::view` turns the host's camera into the one drawn (a CameraType other than Custom draws the script's camera instead, and the fly camera waits). See `docs/scripting/client-server.md` and `docs/scripting/api-character.md`.

Since Phase 79, step 4's Animators end with each skinned MeshPart's foot placement and `IKControl`s, over its pose and before drawing ([../rendering/animation.md](../rendering/animation.md)).

Since Phase 77, `step_prompts` in step 1 runs a dialogue's keys instead of the prompts while one runs, starts a triggered prompt's `Dialogue`, and checks quests' Reach objectives; hosts give the character controller no movement while `movement_locked` (a dialogue runs, or a Sequence locks input) and draw the dialogue box and quest tracker over the game UI ([../scripting/api-dialogue.md](../scripting/api-dialogue.md)).

Since Phase 76, step 1 ends with `axcenity_gameplay::step_abilities` after `step_tools`, on every device (a joined one too): the player's Abilities whose `ActivationKey` was just pressed ask the server to activate, and a host serves the requests waiting. Step 4's `axcenity_gameplay::step_health` then counts cooldowns, active abilities and Effects down on the server; the host draws the ability bar (`ability_bar`, `ui::ability_bar_items`) with the hotbar ([../scripting/api-abilities.md](../scripting/api-abilities.md)).

Since Phase 74, step 2 starts by making again what SplineMeshes and SplineScatters make where their spline or settings changed (all of them the first frame), on every device, before scripts run ([splines.md](splines.md)).

Since Phase 73, step 2 moves SplineFollowers on after both sides' scripts and before Sequences, on the server only, and fires their `Reached`; in step 1 a grounded character first rides the anchored part under it as far as it moved since the last step ([splines.md](splines.md)).

Since Phase 51, step 2 ends by moving Sequences on (after both sides' scripts, so the Animators in step 4 sample this frame's time), and step 1 gives the character controllers no movement while a Sequence with `LockPlayerInput` plays or is paused (`ScriptRuntime::input_locked`); `input_events` lets only releases through then, and a Menu press skips a skippable Sequence. See [../project/sequences.md](../project/sequences.md).
