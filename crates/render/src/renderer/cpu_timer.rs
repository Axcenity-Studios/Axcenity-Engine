//! Where a frame's time goes on the thread that draws it: [`CpuTimer`]
//! times `ScenePipeline::draw` section by section, for
//! [`RenderStats::cpu_times`](crate::RenderStats). The GPU's side is
//! `gpu_timer.rs`.

use std::time::{Duration, Instant};

/// The sections of drawing a frame on the calling thread, in the order
/// they run. Together they are the whole of `ScenePipeline::draw` but the
/// wait for an earlier frame (`RenderStats::gpu_wait`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuSection {
    /// Lights, the sky, probes and the frame's uniforms.
    Setup,
    /// Each item's level of detail and whether any view can see it.
    Lods,
    /// Each draw's material binding (`FrameUses::of`).
    Materials,
    /// Meshes and textures seen for the first time.
    Uploads,
    /// Culling, batches and instances (`plan_draws`).
    Plan,
    /// Each shadow view's casters.
    Casters,
    /// Animated items' joints and shapes.
    Posing,
    /// What moved since last frame, for motion vectors.
    Motion,
    /// The instances written to the GPU.
    Instances,
    /// UI text and quads, particles and the lamps' shadow maps.
    Interface,
    /// The ray traced scene's tables and builds.
    RayScene,
    /// Recording every pass's draws.
    Passes,
    /// wgpu turning the recorded passes into the GPU's commands.
    Encode,
    /// Handing the commands to the GPU.
    Submit,
    /// Next frame's history and freeing what wasn't drawn.
    After,
}

impl CpuSection {
    pub const COUNT: usize = 15;
    pub const ALL: [CpuSection; Self::COUNT] = [
        Self::Setup,
        Self::Lods,
        Self::Materials,
        Self::Uploads,
        Self::Plan,
        Self::Casters,
        Self::Posing,
        Self::Motion,
        Self::Instances,
        Self::Interface,
        Self::RayScene,
        Self::Passes,
        Self::Encode,
        Self::Submit,
        Self::After,
    ];

    /// The section's name, as benchmarks and the frame log print it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Setup => "Lights and sky",
            Self::Lods => "Levels of detail",
            Self::Materials => "Materials",
            Self::Uploads => "Uploads",
            Self::Plan => "Culling and batches",
            Self::Casters => "Shadow casters",
            Self::Posing => "Posing",
            Self::Motion => "Motion history",
            Self::Instances => "Instances written",
            Self::Interface => "UI and particles",
            Self::RayScene => "Ray traced scene",
            Self::Passes => "Recording passes",
            Self::Encode => "Encoding (wgpu finish)",
            Self::Submit => "Submit",
            Self::After => "After the frame",
        }
    }
}

/// Times a frame's sections one after another.
pub(super) struct CpuTimer {
    last: Instant,
    times: [Duration; CpuSection::COUNT],
}

impl CpuTimer {
    pub(super) fn start() -> Self {
        Self {
            last: Instant::now(),
            times: [Duration::ZERO; CpuSection::COUNT],
        }
    }

    /// Ends `section`: the time since the last lap (or the start) is its.
    /// A section ended twice adds up.
    pub(super) fn lap(&mut self, section: CpuSection) {
        self.lap_at(section, Instant::now());
    }

    fn lap_at(&mut self, section: CpuSection, now: Instant) {
        self.times[section as usize] += now.saturating_duration_since(self.last);
        self.last = now;
    }

    /// Leaves the time since the last lap out of every section (a wait).
    pub(super) fn skip(&mut self) {
        self.last = Instant::now();
    }

    pub(super) fn times(&self) -> [Duration; CpuSection::COUNT] {
        self.times
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_section_is_listed_once_in_order_with_a_name_of_its_own() {
        for (index, section) in CpuSection::ALL.into_iter().enumerate() {
            assert_eq!(section as usize, index);
        }
        let mut names: Vec<_> = CpuSection::ALL.iter().map(|s| s.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), CpuSection::COUNT);
    }

    #[test]
    fn laps_give_each_section_its_time_and_add_up() {
        let ms = Duration::from_millis;
        let mut timer = CpuTimer::start();
        let start = timer.last;
        timer.lap_at(CpuSection::Setup, start + ms(2));
        timer.lap_at(CpuSection::Plan, start + ms(5));
        // Twice in a frame adds up.
        timer.lap_at(CpuSection::Setup, start + ms(6));
        let times = timer.times();
        assert_eq!(times[CpuSection::Setup as usize], ms(3));
        assert_eq!(times[CpuSection::Plan as usize], ms(3));
        assert_eq!(times.iter().sum::<Duration>(), ms(6));
        assert_eq!(times[CpuSection::Encode as usize], Duration::ZERO);
    }

    #[test]
    fn a_skipped_wait_is_in_no_section() {
        let mut timer = CpuTimer::start();
        std::thread::sleep(Duration::from_millis(3));
        timer.skip();
        timer.lap(CpuSection::Submit);
        assert!(timer.times().iter().sum::<Duration>() < Duration::from_millis(3));
    }
}
