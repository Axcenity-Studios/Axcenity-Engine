//! The window's events ([`ApplicationHandler`]): resizing, keys, the mouse,
//! focus and redraws, which run a frame; plus the small helpers they use.

use std::time::Duration;

use super::*;

/// Seconds since `last_frame` (0 for the first frame), and records now as the latest frame.
///
/// This takes only the `last_frame` field rather than all of `App`, so it can
/// be called while other fields of `App` are borrowed.
/// Draws with `settings` from the next frame, logging anything the GPU
/// can't do.
pub(crate) fn set_graphics(renderer: &mut Renderer, settings: &GraphicsSettings) {
    renderer.set_graphics(settings);
    for note in &renderer.graphics().notes {
        log::info!("graphics: {note}");
    }
}

/// The display's refresh is asked for again every this many shown frames.
pub(crate) const REFRESH_ASKED_EVERY: u64 = 30;

/// Whether shown frame number `frame` asks the display for its refresh
/// again. A new window under Wayland isn't on a display yet and has none
/// to say, and a window can be moved to another display with no event
/// that says so.
pub(crate) fn asks_refresh_on(frame: u64) -> bool {
    frame.is_multiple_of(REFRESH_ASKED_EVERY)
}

/// How often `window`'s display refreshes, where it says.
pub(crate) fn refresh_of(window: &Window) -> Option<Duration> {
    let millihertz = window.current_monitor()?.refresh_rate_millihertz()?;
    (millihertz > 0).then(|| Duration::from_secs_f64(1000.0 / f64::from(millihertz)))
}

/// A frame as the pacer sees it (`axcenity_render::FramePacer`): the time
/// since the last frame started, the CPU's work in this one (its steps
/// less the time `blocked` handing the picture over) and whether it
/// waited for an earlier frame's GPU work (`gpu_wait`, part of `blocked`).
pub(crate) fn paced(
    seconds_since_last: f32,
    steps: Duration,
    blocked: Duration,
    gpu_wait: Duration,
) -> axcenity_render::PacedFrame {
    /// Asking whether a finished frame is finished takes a few
    /// microseconds: less than this wasn't waiting.
    const WAITED: Duration = Duration::from_micros(300);
    axcenity_render::PacedFrame {
        interval: Duration::from_secs_f32(seconds_since_last.max(0.0)),
        cpu: steps.saturating_sub(blocked),
        waited_for_gpu: gpu_wait >= WAITED,
    }
}

/// How long a frame that wasn't shown (the window is minimized or covered)
/// lasts: the game keeps stepping, 30 times a second, without a display to
/// pace it. Before, such frames came as fast as the loop turned over.
const HIDDEN_FRAME: Duration = Duration::from_nanos(33_333_333);

/// How long a redraw that was asked for may take to come before the
/// window counts as hidden. Under Wayland a minimized or covered window
/// is never asked to draw, so the game would stop altogether; any display
/// that is showing the window asks far sooner than this.
const REDRAW_OVERDUE: Duration = Duration::from_millis(250);

/// What the event loop knows when it has nothing left to handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Waiting {
    /// When the last frame started, if one has.
    pub(crate) last_started: Option<Instant>,
    /// The last frame wasn't drawn because the window can't show one
    /// (`Renderer::hidden`: minimized, covered).
    pub(crate) hidden: bool,
    /// Frames are being stepped without drawing since a redraw went
    /// unanswered ([`NextFrame::StepUnseen`]), until one comes.
    pub(crate) unseen: bool,
    /// When the redraw now waited for was asked for.
    pub(crate) asked_at: Option<Instant>,
}

/// What the event loop does about the next frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NextFrame {
    /// Ask for a redraw and sleep until it comes as an event, or until
    /// `give_up`, when the window counts as hidden.
    Redraw { give_up: Instant },
    /// Sleep until then, and look again: a hidden window's frames are
    /// spaced out, since showing them doesn't pace them.
    Sleep(Instant),
    /// Run a frame without drawing it, now: nothing is asking for
    /// frames, and the game goes on (a hosted game still serves).
    StepUnseen,
}

/// What to do about the next frame at `now` (see [`NextFrame`]).
pub(crate) fn next_frame(now: Instant, waiting: Waiting) -> NextFrame {
    let spaced = waiting.last_started.map(|started| started + HIDDEN_FRAME);
    if waiting.unseen {
        // Still unasked: a frame every `HIDDEN_FRAME`.
        return match spaced.filter(|due| *due > now) {
            Some(due) => NextFrame::Sleep(due),
            None => NextFrame::StepUnseen,
        };
    }
    if waiting.hidden
        && let Some(due) = spaced.filter(|due| *due > now)
    {
        return NextFrame::Sleep(due);
    }
    let give_up = waiting.asked_at.unwrap_or(now) + REDRAW_OVERDUE;
    if give_up <= now {
        NextFrame::StepUnseen
    } else {
        NextFrame::Redraw { give_up }
    }
}

/// Whether `--bench` keeps vertical sync and the display's pacing:
/// `AXCENITY_VSYNC` says `on` or `1`.
pub(crate) fn bench_as_played(value: Option<&str>) -> bool {
    asks_on(value)
}

/// Whether an environment switch's `value` says `on` or `1`.
pub(crate) fn asks_on(value: Option<&str>) -> bool {
    matches!(
        value.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("on" | "1")
    )
}

pub(crate) fn frame_seconds(last_frame: &mut Option<Instant>) -> f32 {
    let now = Instant::now();
    let seconds = last_frame.map_or(0.0, |last| (now - last).as_secs_f32());
    *last_frame = Some(now);
    seconds.min(MAX_FRAME_SECONDS)
}

/// Records a key press or release. Keys are physical positions, so the same
/// keys work on AZERTY and other layouts.
pub(crate) fn set_move_key(keys: &mut MoveKeys, key: KeyCode, pressed: bool) {
    let held = match key {
        KeyCode::KeyW => &mut keys.forward,
        KeyCode::KeyS => &mut keys.back,
        KeyCode::KeyA => &mut keys.left,
        KeyCode::KeyD => &mut keys.right,
        KeyCode::KeyE => &mut keys.up,
        KeyCode::KeyQ => &mut keys.down,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => &mut keys.slow,
        _ => return,
    };
    *held = pressed;
}

/// The camera as `workspace.CurrentCamera` shows it to LocalScripts.
pub(crate) fn camera_state(camera: &FlyCamera) -> CameraState {
    let (position, orientation) = camera.view();
    CameraState {
        position,
        orientation,
    }
}

/// Mouse wheel notches in a scroll, away from the player (toward the
/// screen) positive. Touchpads report pixels instead of notches.
/// How far the wheel scrolls a ScrollingFrame, in pixels across and down
/// (down moves its content up).
pub(crate) fn wheel_pixels(delta: MouseScrollDelta) -> [f32; 2] {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => [-x * PIXELS_PER_NOTCH, -y * PIXELS_PER_NOTCH],
        MouseScrollDelta::PixelDelta(position) => [-position.x as f32, -position.y as f32],
    }
}

pub(crate) fn wheel_notches(delta: MouseScrollDelta) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(position) => position.y as f32 / PIXELS_PER_NOTCH,
    }
}

/// While looking around, hides the cursor and keeps it from leaving the window.
pub(crate) fn capture_mouse(window: &Window, captured: bool) {
    let result = if captured {
        // `Locked` keeps the cursor still (macOS, Wayland). Windows and X11
        // only offer `Confined`, which keeps it inside the window.
        window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
    } else {
        window.set_cursor_grab(CursorGrabMode::None)
    };
    // Looking around still works without the grab, so this isn't an error.
    if let Err(err) = result {
        log::debug!("could not grab the mouse cursor: {err}");
    }
    window.set_cursor_visible(!captured);
}

impl ApplicationHandler for App {
    /// Called when the app is ready to create windows (at startup on desktop).
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let mut attributes = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(DEFAULT_WINDOW_SIZE)
            .with_window_icon(icon::game_icon(self.icon.as_deref()));
        // `--size`, else `AXCENITY_WINDOW_SIZE=1920x1080` (physical pixels),
        // for benchmarks that need the same window every run, in every engine.
        let size = self.window_size.or_else(|| {
            window_size_override(std::env::var("AXCENITY_WINDOW_SIZE").ok().as_deref())
        });
        if let Some((width, height)) = size {
            attributes = attributes.with_inner_size(winit::dpi::PhysicalSize::new(width, height));
        }
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => return self.fail(event_loop, err.into()),
        };

        // `AXCENITY_FRAME_CALLBACKS=on`: every frame waits for the display,
        // as before frames were paced (to compare, and as a way back).
        self.pacer = if asks_on(std::env::var("AXCENITY_FRAME_CALLBACKS").ok().as_deref()) {
            axcenity_render::FramePacer::always_waiting(refresh_of(&window))
        } else {
            axcenity_render::FramePacer::new(refresh_of(&window))
        };
        // GPU setup is `async`; `pollster::block_on` simply waits for it to finish.
        let size = window.inner_size();
        match pollster::block_on(Renderer::new(window.clone(), size.width, size.height)) {
            Ok(mut renderer) => {
                let folder = self.data_folder.as_deref();
                let capabilities = *renderer.capabilities();
                // A LocalScript may have changed them as it started.
                let settings = match self.sim.scripts.take_graphics_change() {
                    Some(change) => graphics::changed(folder, change, &capabilities),
                    None => graphics::start(folder, &capabilities, self.ray_traced_start),
                };
                // `--quality` or `--graphics`: held for the run, not saved.
                let settings = match &self.fixed_graphics {
                    Some(fixed) => graphics::fit_machine(fixed.clone(), &capabilities),
                    None => settings,
                };
                if self.bench.is_some() {
                    // Frames as fast as they're drawn, each timed on the GPU.
                    // Frames as fast as they're drawn, each timed on the
                    // GPU. `AXCENITY_VSYNC=on` keeps vertical sync and the
                    // display's pacing, to time frames as a player gets them.
                    let as_played =
                        bench_as_played(std::env::var("AXCENITY_VSYNC").ok().as_deref());
                    renderer.set_vsync(as_played);
                    renderer.set_gpu_timing(true);
                    if !as_played {
                        self.pacer = axcenity_render::FramePacer::free_running();
                    }
                    // `AXCENITY_BENCH_LATENCY=on` times each frame from its
                    // start to its finished picture, and
                    // `AXCENITY_FRAMES_AHEAD=unlimited` lets frames queue as
                    // deep as the swapchain, to see what the limit of one
                    // is worth (`docs/benchmarks.md`).
                    renderer.watch_latency(asks_on(
                        std::env::var("AXCENITY_BENCH_LATENCY").ok().as_deref(),
                    ));
                    if std::env::var("AXCENITY_FRAMES_AHEAD").as_deref() == Ok("unlimited") {
                        renderer.set_one_frame_ahead(false);
                    }
                }
                self.sim.scripts.set_graphics(&settings, &capabilities);
                set_graphics(&mut renderer, &settings);
                // What the camera turns or cuts away from stays on the GPU
                // while it fits the budgets, so coming back to it doesn't
                // stall a frame uploading it again.
                renderer.set_keep_undrawn(true);
                // A player has no settings screen: the log says what the
                // memory budgets came to on this card.
                let effective = renderer.graphics();
                log::info!(
                    "memory budgets: {} MB of textures, {} MB of meshes ({})",
                    effective.texture_budget >> 20,
                    effective.mesh_budget >> 20,
                    match capabilities.video_memory {
                        Some(memory) =>
                            format!("this card has {} GB", axcenity_render::whole_gib(memory)),
                        None => "this card doesn't say how much memory it has".to_owned(),
                    },
                );
                self.renderer = Some(renderer);
            }
            Err(err) => return self.fail(event_loop, err),
        }

        window.request_redraw();
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.window.is_none() || self.renderer.is_none() {
            return;
        }

        if self.show_splash && self.splash.covers() {
            match &event {
                WindowEvent::KeyboardInput { event: key, .. }
                    if key.state == ElementState::Pressed && !key.repeat =>
                {
                    self.splash.skip();
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    ..
                } => self.splash.skip(),
                _ => {}
            }
            match &event {
                WindowEvent::CloseRequested
                | WindowEvent::Resized(_)
                | WindowEvent::RedrawRequested
                | WindowEvent::Focused(_) => {}
                _ => return,
            }
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                // Going full screen or moving to another display comes
                // with a new size, and perhaps another refresh rate.
                if let Some(window) = &self.window {
                    self.pacer.set_refresh(refresh_of(window));
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = Some([position.x as f32, position.y as f32]);
            }
            WindowEvent::CursorLeft { .. } => self.pointer = None,
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                let key = match event.physical_key {
                    PhysicalKey::Code(code) => Some(code),
                    PhysicalKey::Unidentified(_) => None,
                };
                // F9, and every key while the Developer Console is open
                // (Phase 71).
                if self.console_key(key, pressed, event.repeat, event.text.as_deref()) {
                    return;
                }
                if self.chat_typing.is_some() {
                    self.type_chat(&event);
                    if let Some(key) = key.and_then(script_key) {
                        self.push_key(key, pressed, true);
                    }
                } else if pressed
                    && key == Some(KeyCode::Slash)
                    && axcenity_ui::chat_enabled(&self.sim.world)
                {
                    // `/` opens the chat (Phase 67).
                    self.chat_typing = Some(String::new());
                } else if self.text_focus.is_some() {
                    self.type_text(&event);
                    // Scripts hear typed keys, as the UI's.
                    if let Some(key) = key.and_then(script_key) {
                        self.push_key(key, pressed, true);
                    }
                } else if let Some(code) = key {
                    // A press that moves focus is the UI's, not the game's.
                    let shift = self.held_keys.contains(&axcenity_core::KeyCode::LeftShift)
                        || self.held_keys.contains(&axcenity_core::KeyCode::RightShift);
                    let script = script_key(code);
                    if pressed
                        && let Some(key) = script
                        && let Some(nav) = axcenity_ui::nav_key(&self.input.map, key, shift)
                        && self.ui.takes(&self.sim.world, nav)
                    {
                        self.nav_keys.push(nav);
                        self.push_key(key, true, true);
                        return;
                    }
                    set_move_key(&mut self.keys, code, pressed);
                    if let Some(key) = script {
                        self.push_key(key, pressed, false);
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.ui_mouse_events.push(state == ElementState::Pressed),
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                // The right button looks around, unless a script holds the cursor.
                if button == MouseButton::Right {
                    self.looking = pressed;
                    if self.mouse.0 == MouseBehavior::Default
                        && let Some(window) = &self.window
                    {
                        capture_mouse(window, self.looking);
                        window.set_cursor_visible(!self.looking && self.mouse.1);
                    }
                }
                if let Some(button) = input::mouse_button(button) {
                    self.input.push(InputEvent::new(InputKind::MouseButton {
                        button,
                        pressed,
                        position: self.pointer.unwrap_or_default(),
                    }));
                }
            }
            // Over a ScrollingFrame the wheel scrolls it; with a Tool in
            // hand it goes through the hotbar; elsewhere it zooms.
            WindowEvent::MouseWheel { delta, .. } => {
                let scrolled = self.pointer.is_some_and(|pointer| {
                    self.ui
                        .wheel(&self.sim.world, pointer, wheel_pixels(delta))
                        .apply(&mut self.sim.world)
                });
                if !scrolled && !tool_in_hand(&self.sim.world) {
                    self.zoom += wheel_notches(delta);
                }
                self.input.push(InputEvent {
                    kind: InputKind::Wheel {
                        position: self.pointer.unwrap_or_default(),
                        notches: wheel_notches(delta),
                    },
                    processed: scrolled,
                });
            }
            WindowEvent::Touch(touch) => self.input.push(InputEvent::new(input::touch(&touch))),
            WindowEvent::Focused(true) => {
                self.focused = true;
                // Locked again, if a script asked for it.
                self.mouse = (MouseBehavior::Default, true);
            }
            // Key releases aren't reported while another window has focus,
            // so let go of everything to avoid the camera drifting on its own.
            WindowEvent::Focused(false) => {
                self.focused = false;
                self.release_game_keys();
                if let Some(window) = &self.window {
                    capture_mouse(window, false);
                }
                self.ui_mouse_events.push(false);
            }
            // One frame: the host's input first, then the shared schedule
            // (`axcenity_sim`, docs/architecture/frame-loop.md), then render.
            WindowEvent::RedrawRequested => {
                // The display asks for a frame: the window is seen.
                self.unseen = false;
                self.frame(event_loop);
            }
            _ => {}
        }
    }

    /// Raw mouse movement, reported even when the cursor is held still.
    /// Scripts hear it (`GetMouseDelta`) while the window has focus; it
    /// turns the camera while the right button is held.
    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        let DeviceEvent::MouseMotion { delta: (dx, dy) } = event else {
            return;
        };
        if !self.focused {
            return;
        }
        let (dx, dy) = (dx as f32, dy as f32);
        self.input.add_motion(dx, dy);
        if self.looking && !self.sim.scripts.camera_scripted() {
            if self.sim.characters.view(&self.sim.world).is_some() {
                self.look_dx += dx;
                self.look_dy += dy;
            } else {
                self.camera.look(dx, dy);
            }
        }
    }

    /// Called once all pending events are handled: ask for the next frame.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(renderer)) = (&self.window, &self.renderer) else {
            return;
        };
        // A frame that was shown is paced by showing it: the next comes
        // as a redraw event (at once, or under Wayland when the display
        // has shown the last, if that's waited for), and the loop sleeps
        // until then. A window that can't show frames paces nothing, so
        // its frames are spaced out here (`next_frame`).
        let now = Instant::now();
        let waiting = Waiting {
            last_started: self.last_frame,
            hidden: renderer.hidden(),
            unseen: self.unseen,
            asked_at: self.redraw_asked_at,
        };
        match next_frame(now, waiting) {
            NextFrame::Redraw { give_up } => {
                self.redraw_asked_at.get_or_insert(now);
                window.request_redraw();
                event_loop.set_control_flow(ControlFlow::WaitUntil(give_up));
            }
            NextFrame::Sleep(until) => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(until));
            }
            NextFrame::StepUnseen => {
                // A redraw stays asked for: when the window is shown
                // again it comes, and frames are drawn as before.
                window.request_redraw();
                self.unseen = true;
                self.frame(event_loop);
                event_loop.set_control_flow(ControlFlow::WaitUntil(now + HIDDEN_FRAME));
            }
        }
    }
}

#[cfg(test)]
mod pacing_tests {
    use super::*;

    #[test]
    fn a_paced_frame_counts_work_not_waiting() {
        let ms = Duration::from_millis;
        // 20 ms of steps, 6 of them blocked handing the picture over, 5 of
        // those waiting for the frame before last on the GPU.
        let frame = paced(0.025, ms(20), ms(6), ms(5));
        assert_eq!(frame.cpu, ms(14));
        assert_eq!(frame.interval, Duration::from_secs_f32(0.025));
        assert!(frame.waited_for_gpu);
        // Asking about a frame that had finished isn't waiting for it.
        assert!(!paced(0.025, ms(20), ms(1), Duration::from_micros(20)).waited_for_gpu);
        // The first frame has no frame before it.
        assert_eq!(paced(0.0, ms(3), ms(5), ms(0)).cpu, Duration::ZERO);
    }

    fn shown(now: Instant) -> Waiting {
        Waiting {
            last_started: Some(now - Duration::from_millis(5)),
            hidden: false,
            unseen: false,
            asked_at: None,
        }
    }

    #[test]
    fn the_displays_refresh_is_asked_for_again_as_frames_go_by() {
        // Under Wayland a new window isn't on a display yet, so the first
        // answer is "unknown" (taken as 60 Hz): frames 17 to 25 ms long on
        // a 240 Hz display then never counted as missing its refresh.
        assert!(asks_refresh_on(0));
        assert_eq!(
            (1..=REFRESH_ASKED_EVERY * 3)
                .filter(|frame| asks_refresh_on(*frame))
                .collect::<Vec<u64>>(),
            [
                REFRESH_ASKED_EVERY,
                REFRESH_ASKED_EVERY * 2,
                REFRESH_ASKED_EVERY * 3
            ]
        );
        // Often enough to know within a second at any frame rate a game
        // is played at, seldom enough to cost nothing.
        assert!((10..=120).contains(&REFRESH_ASKED_EVERY));
        // What knowing it changes: 22 ms from frame to frame is no miss
        // at an assumed 60 Hz, and a miss five times over at 240.
        let mut pacer = axcenity_render::FramePacer::new(None);
        let frame = paced(
            0.022,
            Duration::from_millis(10),
            Duration::ZERO,
            Duration::ZERO,
        );
        for _ in 0..100 {
            pacer.frame(frame);
        }
        assert!(pacer.waits_for_display());
        pacer.set_refresh(Some(Duration::from_secs_f64(1.0 / 240.0)));
        for _ in 0..8 {
            pacer.frame(frame);
        }
        assert!(!pacer.waits_for_display());
    }

    #[test]
    fn a_shown_frame_is_followed_by_a_redraw_waited_for_as_an_event() {
        let now = Instant::now();
        // Nothing turns over meanwhile; a quarter of a second without an
        // answer and the window counts as hidden.
        assert_eq!(
            next_frame(now, shown(now)),
            NextFrame::Redraw {
                give_up: now + REDRAW_OVERDUE
            }
        );
        // Still waiting for the same redraw a little later: same deadline.
        let asked = Waiting {
            asked_at: Some(now),
            ..shown(now)
        };
        assert_eq!(
            next_frame(now + Duration::from_millis(100), asked),
            NextFrame::Redraw {
                give_up: now + REDRAW_OVERDUE
            }
        );
        // The very first frame, before any has started.
        let first = Waiting {
            last_started: None,
            ..shown(now)
        };
        assert!(matches!(next_frame(now, first), NextFrame::Redraw { .. }));
    }

    #[test]
    fn a_redraw_that_never_comes_does_not_stop_the_game() {
        // Under Wayland a minimized window is never asked to draw.
        let now = Instant::now();
        let asked = Waiting {
            asked_at: Some(now),
            ..shown(now)
        };
        assert_eq!(
            next_frame(now + REDRAW_OVERDUE, asked),
            NextFrame::StepUnseen
        );
        // From then on a frame every thirtieth of a second, undrawn,
        // sleeping in between, until a redraw comes.
        let stepped = now + REDRAW_OVERDUE;
        let unseen = Waiting {
            last_started: Some(stepped),
            hidden: false,
            unseen: true,
            asked_at: None,
        };
        assert_eq!(
            next_frame(stepped + Duration::from_millis(1), unseen),
            NextFrame::Sleep(stepped + HIDDEN_FRAME)
        );
        assert_eq!(
            next_frame(stepped + HIDDEN_FRAME, unseen),
            NextFrame::StepUnseen
        );
    }

    #[test]
    fn a_frame_the_window_could_not_show_spaces_out_the_next() {
        // Minimized on Windows, covered on macOS: the redraw comes at
        // once and the frame is skipped, so nothing else would pace it.
        let now = Instant::now();
        let hidden = Waiting {
            hidden: true,
            ..shown(now)
        };
        let started = hidden.last_started.unwrap();
        assert_eq!(
            next_frame(now, hidden),
            NextFrame::Sleep(started + HIDDEN_FRAME)
        );
        // Its turn come, it's asked for as usual (and shown, if the
        // window is back).
        assert!(matches!(
            next_frame(started + HIDDEN_FRAME, hidden),
            NextFrame::Redraw { .. }
        ));
        // Thirty a second: enough for a hosted game to keep serving.
        assert!(HIDDEN_FRAME >= Duration::from_millis(30));
        assert!(HIDDEN_FRAME <= Duration::from_millis(40));
    }

    #[test]
    fn a_bench_keeps_vertical_sync_only_when_asked() {
        assert!(!bench_as_played(None));
        assert!(!bench_as_played(Some("off")));
        assert!(bench_as_played(Some("on")));
        assert!(bench_as_played(Some(" On ")));
        assert!(bench_as_played(Some("1")));
    }
}
