//! How long a frame takes from its start to its picture being finished:
//! [`LatencyWatch`], for benchmarks (`docs/benchmarks.md`).
//!
//! A frame starts when its host reads input, and what it draws can't be
//! shown before the GPU has finished drawing it. The time between the two
//! is the part of input latency the engine decides: how long the frame's
//! own work takes, and how many frames it queues behind (frame pacing,
//! ADR-0052). What the display adds after (the wait for its next refresh,
//! the compositor) is the same whatever the engine does.
//!
//! wgpu says a submission is done from inside a poll, so a watch keeps a
//! thread looking at the device five thousand times a second: a frame's
//! time is noted within about a fifth of a millisecond of its work
//! finishing (more when the system is slow to wake the thread). Only made
//! while asked for.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Between two looks at the device. A look doesn't wait: a waiting poll
/// holds wgpu's device lock, which kept the frame's own thread out of
/// `submit` for as long (4 ms a frame, the first way this was written).
const REST: Duration = Duration::from_micros(200);

/// Times each frame from its start to its GPU work finishing.
pub(super) struct LatencyWatch {
    /// The frame being drawn's start, until it's submitted.
    started: Option<Instant>,
    times: Arc<Mutex<Vec<Duration>>>,
    stop: Arc<AtomicBool>,
    poller: Option<std::thread::JoinHandle<()>>,
}

impl LatencyWatch {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let poller = {
            let (device, stop) = (device.clone(), stop.clone());
            std::thread::Builder::new()
                .name("frame latency".to_owned())
                .spawn(move || {
                    while !stop.load(Ordering::Acquire) {
                        let _ = device.poll(wgpu::PollType::Poll);
                        std::thread::sleep(REST);
                    }
                })
                .ok()
        };
        Self {
            started: None,
            times: Arc::default(),
            stop,
            poller,
        }
    }

    /// The frame about to be drawn started at `at` (when its host read
    /// input).
    pub(super) fn frame_started(&mut self, at: Instant) {
        self.started = Some(at);
    }

    /// The frame's work was just submitted to `queue`: its time is noted
    /// when the GPU has finished it. A frame whose start wasn't given
    /// isn't timed.
    pub(super) fn submitted(&mut self, queue: &wgpu::Queue) {
        let Some(started) = self.started.take() else {
            return;
        };
        let times = self.times.clone();
        queue.on_submitted_work_done(move || {
            times
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(started.elapsed());
        });
    }

    /// The times noted since last asked, oldest first.
    pub(super) fn take(&self) -> Vec<Duration> {
        std::mem::take(&mut *self.times.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl Drop for LatencyWatch {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(poller) = self.poller.take() {
            let _ = poller.join();
        }
    }
}
