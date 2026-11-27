//! `AXCENITY_WAKE_LOG=<seconds>`: a measuring switch for how the editor
//! leaves its idle pace (`docs/benchmarks.md`). Every that many seconds a
//! timer wakes the editor as an input event would, by the route a window
//! event takes (a repaint asked of the event loop while a later one is
//! pending), and the frame it wakes counts as input, so frames go steady
//! for a second and fall idle again. Each frame's line on standard error
//! says how long after the last frame it started and which lines of code
//! had asked for it; the woken frame's also says how long after the timer
//! asked.

use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(super) struct WakeLog {
    /// When the timer last asked for a frame, until a frame takes it.
    asked: Arc<Mutex<Option<Instant>>>,
    /// Whether the timer's thread runs yet: it needs the egui context.
    started: bool,
    every: Duration,
    last_frame: Option<Instant>,
}

impl WakeLog {
    /// The log `AXCENITY_WAKE_LOG` asks for, if it names a time.
    pub(super) fn from_env() -> Option<Self> {
        let seconds: f64 = std::env::var("AXCENITY_WAKE_LOG")
            .ok()?
            .trim()
            .parse()
            .ok()?;
        (seconds.is_finite() && seconds > 0.0).then(|| Self {
            asked: Arc::default(),
            started: false,
            every: Duration::from_secs_f64(seconds),
            last_frame: None,
        })
    }

    /// Before a frame's input is read: starts the timer, and makes a frame
    /// the timer woke count as input.
    pub(super) fn before_frame(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if !self.started {
            self.started = true;
            let (asked, ctx, every) = (self.asked.clone(), ctx.clone(), self.every);
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(every);
                    *asked
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Instant::now());
                    ctx.request_repaint();
                }
            });
        }
        let now = Instant::now();
        let asked = self
            .asked
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let since_last = self
            .last_frame
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f64() * 1000.0);
        // Who asked for this frame (egui keeps each request's place in
        // the code): what keeps an editor nobody touches from resting.
        let mut askers: Vec<String> = ctx
            .repaint_causes()
            .iter()
            .map(|cause| {
                let file = cause.file.rsplit('/').next().unwrap_or(cause.file);
                format!("{file}:{}", cause.line)
            })
            .collect();
        askers.sort();
        askers.dedup();
        let askers = askers.join(", ");
        match asked {
            Some(asked) => {
                // A mouse that reports no movement: an event, and nothing
                // for the editor to act on.
                raw_input
                    .events
                    .push(egui::Event::MouseMoved(egui::Vec2::ZERO));
                eprintln!(
                    "wake: frame {since_last:.1} ms after the last, {:.1} ms after it was asked for; asked by {askers}",
                    now.duration_since(asked).as_secs_f64() * 1000.0
                );
            }
            None => eprintln!("wake: frame {since_last:.1} ms after the last; asked by {askers}"),
        }
    }
}
