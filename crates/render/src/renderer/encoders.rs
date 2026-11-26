//! A frame's commands, recorded in parts that are encoded at the same
//! time: [`FrameEncoders`].
//!
//! Recording a pass with wgpu only notes its commands. The work is done
//! by `CommandEncoder::finish`: every draw call is checked and turned into
//! the backend's commands there, on the thread that calls it, about a
//! microsecond a draw. Desert Homestead's frame has 4,300 draw calls over
//! its shadow maps, its depth prepass and its scene, and its ray traced
//! scene's acceleration structures to build: finishing them in one
//! encoder took 4.0 ms of the frame's thread, and in parts takes 1.6
//! (`docs/rendering/performance.md`, "Encoding on several threads").
//!
//! So the frame is recorded into several encoders, one after another, and
//! they are finished on the renderer's threads together. The command
//! buffers are submitted in the order they were recorded, in one call, so
//! the GPU does the same work in the same order as from one encoder.
//!
//! A part ends where [`FrameEncoders::part`] is called, and only between
//! passes: a pass belongs to one encoder.

use std::ops::{Deref, DerefMut};

use rayon::prelude::*;

use super::parallel;

/// A pass with fewer draw calls than this isn't worth a part of its own:
/// it is encoded with what comes before it.
pub(super) const WORTH_A_PART: usize = 256;

/// The encoder being recorded into, and the ones before it.
///
/// It derefs to the current `wgpu::CommandEncoder`, so the frame's passes
/// record into it as into one encoder.
pub(super) struct FrameEncoders {
    device: wgpu::Device,
    /// Whether [`FrameEncoders::part`] starts a new encoder.
    in_parts: bool,
    done: Vec<wgpu::CommandEncoder>,
    current: wgpu::CommandEncoder,
}

impl FrameEncoders {
    /// Starts a frame. Not `in_parts`, it is one encoder as before
    /// (`AXCENITY_FRAME_ENCODERS=one`, and to compare with).
    pub(super) fn new(device: &wgpu::Device, in_parts: bool) -> Self {
        Self {
            device: device.clone(),
            in_parts,
            done: Vec::new(),
            current: encoder(device, "frame"),
        }
    }

    /// What is recorded from here on is a part of its own, encoded beside
    /// the others. Call it between passes that hold many draw calls.
    pub(super) fn part(&mut self, label: &'static str) {
        if self.in_parts {
            let before = std::mem::replace(&mut self.current, encoder(&self.device, label));
            self.done.push(before);
        }
    }

    /// Encodes every part, the parts shared among the renderer's threads,
    /// and returns their command buffers in the order to submit them.
    pub(super) fn finish(mut self) -> Vec<wgpu::CommandBuffer> {
        self.done.push(self.current);
        if self.done.len() == 1 {
            return self.done.into_iter().map(|part| part.finish()).collect();
        }
        // `collect` on an indexed parallel iterator keeps the order.
        parallel::shared(|| {
            self.done
                .into_par_iter()
                .with_min_len(1)
                .map(|part| part.finish())
                .collect()
        })
    }
}

fn encoder(device: &wgpu::Device, label: &str) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
}

impl Deref for FrameEncoders {
    type Target = wgpu::CommandEncoder;

    fn deref(&self) -> &wgpu::CommandEncoder {
        &self.current
    }
}

impl DerefMut for FrameEncoders {
    fn deref_mut(&mut self) -> &mut wgpu::CommandEncoder {
        &mut self.current
    }
}

/// Whether frames are recorded in parts: yes unless
/// `AXCENITY_FRAME_ENCODERS` is `one`.
pub(super) fn in_parts_by_default() -> bool {
    in_parts_when(std::env::var("AXCENITY_FRAME_ENCODERS").ok().as_deref())
}

fn in_parts_when(asked: Option<&str>) -> bool {
    !asked.is_some_and(|value| value.trim().eq_ignore_ascii_case("one"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_recorded_in_parts_unless_one_encoder_is_asked_for() {
        assert!(in_parts_when(None));
        assert!(in_parts_when(Some("")));
        assert!(in_parts_when(Some("parts")));
        assert!(!in_parts_when(Some("one")));
        assert!(!in_parts_when(Some(" One ")));
    }
}
