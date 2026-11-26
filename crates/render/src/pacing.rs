//! When a host starts its next frame: [`FramePacer`].
//!
//! A frame is work on the CPU (the game's steps, the draw list, recording
//! the GPU's commands) and then work on the GPU. A host can start the next
//! frame's CPU work at two moments:
//!
//! - **When the display has shown the last frame.** Under Wayland that's
//!   what winit does once `Window::pre_present_notify` has been called: it
//!   holds `RedrawRequested` back until the compositor's frame callback.
//!   The display can't show a frame before the GPU has finished it, so the
//!   CPU and the GPU take turns. That is the shortest way from input to
//!   picture, and costs nothing while both fit in one refresh of the
//!   display.
//! - **At once**, while the GPU still draws the last frame. A frame is
//!   then as long as the slower of the two, not their sum.
//!
//! Desert Homestead with every ray traced effect on, 15 ms on the CPU and
//! 24 on the GPU, was shown at 24 frames a second the first way and 62 the
//! second, and the GPU's own time fell to 14 ms: idle for the CPU's turn
//! of every frame, it had been running at half its clock
//! (`docs/rendering/performance.md`, "The CPU and the GPU took turns").
//!
//! The pacer picks between them from how the last frames went. The
//! renderer keeps the second way bounded: with
//! `TextureRenderer::set_one_frame_ahead` a frame waits for the frame
//! before the last to finish on the GPU, so never more than one waits
//! behind the one being drawn.

use std::time::Duration;

/// How many frames in a row must miss the display's refresh before frames
/// start early: a hitch or a load isn't a slow scene.
const SLOW_FRAMES: u32 = 8;
/// How many frames in a row must fit in a refresh, CPU and GPU together,
/// before frames wait for the display again (about a second).
const LIGHT_FRAMES: u32 = 90;
/// A frame this many refreshes long or longer missed its refresh.
const MISSED: f64 = 1.5;
/// The refresh assumed for a display that doesn't say its rate.
const DEFAULT_REFRESH: Duration = Duration::from_nanos(16_666_667);

/// One frame as the pacer sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PacedFrame {
    /// From this frame's start to the next one's.
    pub interval: Duration,
    /// The CPU's work in it (not time spent waiting).
    pub cpu: Duration,
    /// Whether it had to wait for an earlier frame to finish on the GPU
    /// (`RenderStats`'s `gpu_wait` wasn't zero): the GPU is the slower
    /// side.
    pub waited_for_gpu: bool,
}

/// Decides whether a host's frames wait for the display to show the last
/// one ([`FramePacer::waits_for_display`]) or start at once.
///
/// It starts waiting for the display. Frames that keep missing the
/// refresh switch it to starting at once; frames light enough to fit in a
/// refresh one after the other (the GPU never waited for, and the CPU's
/// work under half a refresh, so the GPU's is too) switch it back.
#[derive(Debug, Clone)]
pub struct FramePacer {
    refresh: Duration,
    early: bool,
    /// Frames in a row that argue for the other way.
    streak: u32,
    /// Never waits for the display ([`FramePacer::free_running`]).
    always_early: bool,
    /// Always waits for it ([`FramePacer::always_waiting`]).
    always_waits: bool,
}

impl Default for FramePacer {
    fn default() -> Self {
        Self::new(None)
    }
}

impl FramePacer {
    /// A pacer for a display refreshing every `refresh` (`None` where it
    /// doesn't say: 60 times a second is assumed).
    pub fn new(refresh: Option<Duration>) -> Self {
        Self {
            refresh: refresh
                .filter(|refresh| !refresh.is_zero())
                .unwrap_or(DEFAULT_REFRESH),
            early: false,
            streak: 0,
            always_early: false,
            always_waits: false,
        }
    }

    /// A pacer that waits for the display whatever its frames cost, as
    /// hosts did before 2026-11-25: to compare with, and as a way back.
    pub fn always_waiting(refresh: Option<Duration>) -> Self {
        Self {
            always_waits: true,
            ..Self::new(refresh)
        }
    }

    /// A pacer that never waits for the display, for a host drawing as
    /// fast as it can (vertical sync off, a benchmark).
    pub fn free_running() -> Self {
        Self {
            early: true,
            always_early: true,
            ..Self::new(None)
        }
    }

    /// The display's refresh changed (the window moved to another one).
    pub fn set_refresh(&mut self, refresh: Option<Duration>) {
        self.refresh = refresh
            .filter(|refresh| !refresh.is_zero())
            .unwrap_or(DEFAULT_REFRESH);
    }

    /// Whether the next frame waits for the display to show the last one
    /// before it starts (`Window::pre_present_notify`).
    pub fn waits_for_display(&self) -> bool {
        !self.early
    }

    /// [`FramePacer::waits_for_display`] for a window that has the
    /// keyboard's focus or not. One without it always waits (unless the
    /// pacer is free running): a minimized or covered window never has
    /// focus, and under Wayland nothing else says a window is hidden, so
    /// one that didn't wait would go on drawing a heavy scene at full
    /// load for nobody (256 W, measured). Waiting, it is asked for no
    /// frames and its host can stop drawing; a window that is seen without
    /// focus draws at the display's pace, as every window did before.
    pub fn waits_for_display_when(&self, focused: bool) -> bool {
        !self.always_early && (!focused || self.waits_for_display())
    }

    /// Notes a frame. Returns true when the answer of
    /// [`FramePacer::waits_for_display`] changed.
    pub fn frame(&mut self, frame: PacedFrame) -> bool {
        if self.always_early || self.always_waits {
            return false;
        }
        let argues_for_change = if self.early {
            // Light enough to take turns within a refresh: the GPU kept
            // up with the CPU, whose work is under half of one.
            !frame.waited_for_gpu && frame.cpu * 2 <= self.refresh
        } else {
            frame.interval.as_secs_f64() >= self.refresh.as_secs_f64() * MISSED
        };
        if !argues_for_change {
            self.streak = 0;
            return false;
        }
        self.streak += 1;
        let needed = if self.early {
            LIGHT_FRAMES
        } else {
            SLOW_FRAMES
        };
        if self.streak < needed {
            return false;
        }
        self.early = !self.early;
        self.streak = 0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: f64) -> Duration {
        Duration::from_secs_f64(n / 1000.0)
    }

    fn frame(interval: f64, cpu: f64, waited_for_gpu: bool) -> PacedFrame {
        PacedFrame {
            interval: ms(interval),
            cpu: ms(cpu),
            waited_for_gpu,
        }
    }

    /// A 60 Hz display.
    fn pacer() -> FramePacer {
        FramePacer::new(Some(ms(1000.0 / 60.0)))
    }

    #[test]
    fn light_frames_wait_for_the_display() {
        let mut pacer = pacer();
        assert!(pacer.waits_for_display());
        // 3 ms of CPU and a little GPU, shown every refresh.
        for _ in 0..1000 {
            assert!(!pacer.frame(frame(16.7, 3.0, false)));
        }
        assert!(pacer.waits_for_display());
    }

    #[test]
    fn frames_that_keep_missing_the_refresh_start_early() {
        // 15 ms on the CPU, then 14 on the GPU, then the display: every
        // other refresh, where either alone fits in one.
        let mut pacer = pacer();
        let changed: Vec<bool> = (0..SLOW_FRAMES)
            .map(|_| pacer.frame(frame(33.3, 15.0, false)))
            .collect();
        assert_eq!(changed.iter().filter(|changed| **changed).count(), 1);
        assert!(changed[SLOW_FRAMES as usize - 1], "on the last of them");
        assert!(!pacer.waits_for_display());
    }

    #[test]
    fn one_slow_frame_is_not_a_slow_scene() {
        let mut pacer = pacer();
        for _ in 0..100 {
            // A hitch every few frames, never several in a row.
            for _ in 0..SLOW_FRAMES - 1 {
                pacer.frame(frame(50.0, 40.0, false));
            }
            pacer.frame(frame(16.7, 3.0, false));
        }
        assert!(pacer.waits_for_display());
    }

    #[test]
    fn early_frames_stay_early_while_the_scene_is_heavy() {
        let mut pacer = pacer();
        for _ in 0..SLOW_FRAMES {
            pacer.frame(frame(33.3, 15.0, false));
        }
        // Started early they hold the refresh, which waiting couldn't:
        // the CPU's 15 ms is over half of it.
        for _ in 0..1000 {
            assert!(!pacer.frame(frame(16.7, 15.0, false)));
        }
        // A light CPU frame that waits for the GPU is still a heavy frame.
        for _ in 0..1000 {
            assert!(!pacer.frame(frame(20.0, 2.0, true)));
        }
        assert!(!pacer.waits_for_display());
    }

    #[test]
    fn frames_that_became_light_wait_for_the_display_again() {
        let mut pacer = pacer();
        for _ in 0..SLOW_FRAMES {
            pacer.frame(frame(33.3, 15.0, false));
        }
        for _ in 0..LIGHT_FRAMES - 1 {
            assert!(!pacer.frame(frame(16.7, 4.0, false)));
        }
        assert!(pacer.frame(frame(16.7, 4.0, false)));
        assert!(pacer.waits_for_display());
        // And stay there: shown every refresh, nothing argues for early.
        for _ in 0..1000 {
            pacer.frame(frame(16.7, 4.0, false));
        }
        assert!(pacer.waits_for_display());
    }

    #[test]
    fn the_refresh_it_compares_with_is_the_displays() {
        // 10 ms from frame to frame holds a 60 Hz display's refresh and
        // misses a 240 Hz one's more than twice over.
        let mut slow_display = pacer();
        let mut fast_display = FramePacer::new(Some(ms(1000.0 / 240.0)));
        for _ in 0..SLOW_FRAMES {
            slow_display.frame(frame(10.0, 5.0, false));
            fast_display.frame(frame(10.0, 5.0, false));
        }
        assert!(slow_display.waits_for_display());
        assert!(!fast_display.waits_for_display());
        // A display that doesn't say its rate counts as 60 Hz.
        let mut unknown = FramePacer::new(None);
        for _ in 0..SLOW_FRAMES {
            unknown.frame(frame(10.0, 5.0, false));
        }
        assert!(unknown.waits_for_display());
    }

    #[test]
    fn a_window_without_focus_waits_for_the_display_whatever_its_frames_cost() {
        let mut pacer = pacer();
        for _ in 0..SLOW_FRAMES {
            pacer.frame(frame(33.3, 15.0, false));
        }
        // Heavy frames start early in the window someone is using...
        assert!(!pacer.waits_for_display_when(true));
        // ...and not in one nobody is: it may be hidden.
        assert!(pacer.waits_for_display_when(false));
        // Light frames wait either way.
        assert!(FramePacer::default().waits_for_display_when(true));
        assert!(FramePacer::default().waits_for_display_when(false));
        // A benchmark's free-running frames never wait, focus or not: it
        // is timed whatever else is on screen.
        assert!(!FramePacer::free_running().waits_for_display_when(false));
    }

    #[test]
    fn an_always_waiting_pacer_waits_however_slow_its_frames() {
        let mut pacer = FramePacer::always_waiting(Some(ms(1000.0 / 60.0)));
        for _ in 0..1000 {
            assert!(!pacer.frame(frame(50.0, 20.0, true)));
        }
        assert!(pacer.waits_for_display());
    }

    #[test]
    fn a_free_running_pacer_never_waits_for_the_display() {
        let mut pacer = FramePacer::free_running();
        assert!(!pacer.waits_for_display());
        for _ in 0..1000 {
            assert!(!pacer.frame(frame(1.0, 0.5, false)));
        }
        assert!(!pacer.waits_for_display());
    }
}
