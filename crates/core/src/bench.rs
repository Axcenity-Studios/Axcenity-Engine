//! Frame-stage timing for benchmarks (roadmap item I1, "measure first").
//!
//! A host times each frame's stages with [`Laps`], then hands them to
//! [`StageSamples`], which keeps every frame and reports the mean, median,
//! 95th percentile and worst time of each stage. Every host (player,
//! editor Play, dedicated server, test runner) reports in the same shape,
//! so their numbers can be put side by side. How to run them:
//! `docs/development/benchmarks.md`.
//!
//! This only uses `std::time`, so it costs nothing to keep in headless
//! builds. Profiler spans (the `profiling` crate) are separate: they show
//! the same stages in Tracy with the `profile-tracy` feature.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

/// The time of each stage of one frame, timed one after another.
#[derive(Debug, Clone)]
pub struct Laps {
    started: Instant,
    last: Instant,
    stages: Vec<(&'static str, Duration)>,
}

impl Default for Laps {
    fn default() -> Self {
        Self::start()
    }
}

impl Laps {
    /// Starts timing a frame.
    pub fn start() -> Self {
        let now = Instant::now();
        Self {
            started: now,
            last: now,
            stages: Vec::new(),
        }
    }

    /// A frame already timed: it started at `started` and its stages
    /// took `stages`, one after another.
    pub fn timed(started: Instant, stages: &[(&'static str, Duration)]) -> Self {
        Self {
            started,
            last: started + stages.iter().map(|(_, time)| *time).sum::<Duration>(),
            stages: stages.to_vec(),
        }
    }

    /// When the frame started.
    pub fn started(&self) -> Instant {
        self.started
    }

    /// Ends the stage `name`: the time since the last lap (or the start).
    pub fn lap(&mut self, name: &'static str) {
        let now = Instant::now();
        self.stages.push((name, now - self.last));
        self.last = now;
    }

    /// Ends the stage `name` like [`Laps::lap`], but counts `wait` of it
    /// (time spent blocked, such as waiting for the display to take a
    /// frame) as a stage of its own called `wait_name`, after it. The
    /// frame's total is unchanged.
    pub fn lap_with_wait(&mut self, name: &'static str, wait_name: &'static str, wait: Duration) {
        let now = Instant::now();
        let whole = now - self.last;
        let wait = wait.min(whole);
        self.stages.push((name, whole - wait));
        self.stages.push((wait_name, wait));
        self.last = now;
    }

    /// Each stage's time, in the order they ran.
    pub fn stages(&self) -> &[(&'static str, Duration)] {
        &self.stages
    }

    /// The time from the start to the last lap.
    pub fn total(&self) -> Duration {
        self.last - self.started
    }
}

/// One stage's times over a run, summarized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageSummary {
    pub name: &'static str,
    pub mean: Duration,
    pub median: Duration,
    pub p95: Duration,
    pub max: Duration,
}

/// Every frame's stage times over a run.
#[derive(Debug, Default, Clone)]
pub struct StageSamples {
    frames: Vec<Duration>,
    /// Stage names in the order first seen, with every frame's time.
    stages: Vec<(&'static str, Vec<Duration>)>,
}

impl StageSamples {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a frame `total` long whose stages took `stages`. A stage a
    /// frame skipped counts as zero for it, so means stay per frame.
    pub fn record(&mut self, total: Duration, stages: &[(&'static str, Duration)]) {
        let frame = self.frames.len();
        self.frames.push(total);
        for &(name, time) in stages {
            let index = match self.stages.iter().position(|(seen, _)| *seen == name) {
                Some(index) => index,
                None => {
                    self.stages.push((name, vec![Duration::ZERO; frame]));
                    self.stages.len() - 1
                }
            };
            let times = &mut self.stages[index].1;
            // The same stage twice in a frame adds up.
            if times.len() > frame {
                times[frame] += time;
            } else {
                times.push(time);
            }
        }
        for (_, times) in &mut self.stages {
            times.resize(frame + 1, Duration::ZERO);
        }
    }

    /// Adds a frame timed with `laps`.
    pub fn record_laps(&mut self, laps: &Laps) {
        self.record(laps.total(), laps.stages());
    }

    /// Each recorded frame's length, in order.
    pub fn frame_times(&self) -> &[Duration] {
        &self.frames
    }

    /// One line per frame, `frame,<stage>…` in milliseconds, for a
    /// spreadsheet: where uneven frames come from.
    pub fn csv(&self) -> String {
        let mut text = String::from("frame");
        for (name, _) in &self.stages {
            let _ = write!(text, ",{name}");
        }
        text.push('\n');
        for (index, frame) in self.frames.iter().enumerate() {
            let _ = write!(text, "{:.4}", millis(*frame));
            for (_, times) in &self.stages {
                let _ = write!(text, ",{:.4}", millis(times[index]));
            }
            text.push('\n');
        }
        text
    }

    /// How many frames were recorded.
    pub fn frames(&self) -> usize {
        self.frames.len()
    }

    /// The whole frame's times, summarized (named `frame`).
    pub fn frame_summary(&self) -> StageSummary {
        summarize("frame", &self.frames)
    }

    /// Each stage's times, summarized, in the order the stages ran.
    pub fn summaries(&self) -> Vec<StageSummary> {
        self.stages
            .iter()
            .map(|(name, times)| summarize(name, times))
            .collect()
    }

    /// A table for people: one row per stage, then the whole frame.
    pub fn report(&self, title: &str) -> String {
        let mut text = format!("{title} ({} frames)\n", self.frames());
        let _ = writeln!(
            text,
            "  {:<24} {:>10} {:>10} {:>10} {:>10}",
            "stage", "mean ms", "median ms", "p95 ms", "max ms"
        );
        for summary in self
            .summaries()
            .into_iter()
            .chain(std::iter::once(self.frame_summary()))
        {
            let _ = writeln!(
                text,
                "  {:<24} {:>10.3} {:>10.3} {:>10.3} {:>10.3}",
                summary.name,
                millis(summary.mean),
                millis(summary.median),
                millis(summary.p95),
                millis(summary.max)
            );
        }
        if let Some(pacing) = self.pacing() {
            let _ = writeln!(text, "  {}", pacing.line());
        }
        text
    }

    /// The table without the pacing line (for the GPU's passes, whose
    /// "frame rate" would mislead).
    fn table(&self, title: &str) -> String {
        let text = self.report(title);
        match self.pacing() {
            Some(pacing) => text.replace(
                &format!(
                    "  {}
",
                    pacing.line()
                ),
                "",
            ),
            None => text,
        }
    }

    /// How evenly the frames came ([`FramePacing`]), if any were recorded.
    pub fn pacing(&self) -> Option<FramePacing> {
        FramePacing::of(&self.frames)
    }

    /// One line of JSON for tools and nightly trends:
    /// `{"host":…,"scene":…,"frames":…,"stages":[{"name":…,"mean_ms":…},…]}`.
    pub fn json(&self, host: &str, scene: &str) -> String {
        let mut text = format!(
            "{{\"host\":{},\"scene\":{},\"frames\":{},\"stages\":{}",
            json_string(host),
            json_string(scene),
            self.frames(),
            self.stages_json()
        );
        if let Some(pacing) = self.pacing() {
            let _ = write!(
                text,
                ",\"pacing\":{{\"average_fps\":{:.2},\"low_1pct_fps\":{:.2},\"low_01pct_fps\":{:.2},\"stddev_ms\":{:.4},\"p99_ms\":{:.4}}}",
                pacing.average_fps,
                pacing.low_1pct_fps,
                pacing.low_01pct_fps,
                pacing.stddev_ms,
                pacing.p99_ms
            );
        }
        text.push('}');
        text
    }

    /// Each stage, then the whole frame, as a JSON array.
    fn stages_json(&self) -> String {
        let mut text = String::from("[");
        let all = self
            .summaries()
            .into_iter()
            .chain(std::iter::once(self.frame_summary()));
        for (index, summary) in all.enumerate() {
            if index > 0 {
                text.push(',');
            }
            let _ = write!(
                text,
                "{{\"name\":{},\"mean_ms\":{:.4},\"median_ms\":{:.4},\"p95_ms\":{:.4},\"max_ms\":{:.4}}}",
                json_string(summary.name),
                millis(summary.mean),
                millis(summary.median),
                millis(summary.p95),
                millis(summary.max)
            );
        }
        text.push(']');
        text
    }
}

/// How evenly frames came over a run: the numbers frame-rate tools
/// report, worked out from each frame's length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePacing {
    /// Frames over the time they took.
    pub average_fps: f64,
    /// The average frame rate of the slowest 1% of frames (at least one).
    pub low_1pct_fps: f64,
    /// The same for the slowest 0.1%.
    pub low_01pct_fps: f64,
    /// How far frame times stray from their mean (standard deviation).
    pub stddev_ms: f64,
    /// 99 frames in 100 are this long or shorter.
    pub p99_ms: f64,
}

impl FramePacing {
    /// The pacing of frames `frames` long, or `None` for no frames.
    pub fn of(frames: &[Duration]) -> Option<Self> {
        Self::of_millis(frames.iter().map(|&frame| millis(frame)))
    }

    /// The same from frame lengths in milliseconds.
    pub fn of_millis(frames: impl IntoIterator<Item = f64>) -> Option<Self> {
        let mut sorted: Vec<f64> = frames.into_iter().collect();
        if sorted.is_empty() {
            return None;
        }
        // Slowest first.
        sorted.sort_by(|a, b| b.total_cmp(a));
        let count = sorted.len() as f64;
        let mean = sorted.iter().sum::<f64>() / count;
        let fps = |ms: f64| if ms > 0.0 { 1000.0 / ms } else { 0.0 };
        let slowest = |share: f64| {
            let take = ((count * share).ceil() as usize).clamp(1, sorted.len());
            fps(sorted[..take].iter().sum::<f64>() / take as f64)
        };
        let variance = sorted.iter().map(|ms| (ms - mean).powi(2)).sum::<f64>() / count;
        let p99_index = ((count - 1.0) * 0.01).round() as usize;
        Some(Self {
            average_fps: fps(mean),
            low_1pct_fps: slowest(0.01),
            low_01pct_fps: slowest(0.001),
            stddev_ms: variance.sqrt(),
            p99_ms: sorted[p99_index],
        })
    }

    /// One line for people.
    pub fn line(&self) -> String {
        format!(
            "FPS average {:.1}, 1% low {:.1}, 0.1% low {:.1}; frame time std dev {:.3} ms, p99 {:.3} ms",
            self.average_fps, self.low_1pct_fps, self.low_01pct_fps, self.stddev_ms, self.p99_ms
        )
    }
}

/// What limits a frame, from where its time went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// The GPU's work takes about the whole frame.
    Gpu,
    /// The CPU's work (the frame less the time it was blocked waiting for
    /// the display or the GPU) takes about the whole frame.
    Cpu,
    /// Neither is busy for most of the frame: it waits on the display
    /// (vertical sync) or the two take turns.
    Sync,
    /// Without a GPU time it can't be told.
    Unknown,
}

impl Bound {
    /// Which of these a frame `frame_ms` long is, given its CPU work and
    /// its GPU time (`None` where the GPU can't be timed). A side is the
    /// limit when it's busy for at least 85% of the frame; the GPU wins a
    /// tie, since a CPU waiting on a busy GPU can look busy too.
    pub fn of(frame_ms: f64, cpu_work_ms: f64, gpu_ms: Option<f64>) -> Self {
        const BUSY: f64 = 0.85;
        let Some(gpu_ms) = gpu_ms else {
            return Self::Unknown;
        };
        if frame_ms <= 0.0 {
            return Self::Unknown;
        }
        if gpu_ms >= frame_ms * BUSY {
            Self::Gpu
        } else if cpu_work_ms >= frame_ms * BUSY {
            Self::Cpu
        } else {
            Self::Sync
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Gpu => "GPU-bound",
            Self::Cpu => "CPU-bound",
            Self::Sync => "sync-bound",
            Self::Unknown => "bound unknown",
        }
    }
}

/// A host's `--bench <frames>` run: frames to skip while caches fill and
/// physics settles, then frames to keep.
#[derive(Debug, Clone)]
pub struct BenchRun {
    warm_up: usize,
    frames: usize,
    seen: usize,
    pub samples: StageSamples,
    /// The very first frame, which reads whatever wasn't loaded ahead.
    first_frame: Option<Duration>,
    /// The slowest frame of the warm-up, where first-sight loads land.
    warm_up_worst: Duration,
    /// One-off times a host notes, such as launch to first frame.
    startup: Vec<(&'static str, Duration)>,
    /// The GPU's frame and passes, for frames the GPU was timed on (a
    /// host whose device can't take timestamps leaves this empty).
    pub gpu: StageSamples,
    /// The depth prepass's and the scene pass's GPU time by what was drawn
    /// (terrain, scattered items, other items, the sky), where the GPU can
    /// be stamped between draws. Its frame is the parts' sum.
    pub gpu_parts: StageSamples,
    /// The drawing's work on the frame's thread, section by section
    /// (the renderer's `CpuSection`s), for a host that draws.
    pub drawing: StageSamples,
    /// Each kept frame's time from its start to its picture being
    /// finished on the GPU, for a host that measures it
    /// ([`BenchRun::record_latency`]).
    latency: Vec<Duration>,
    /// Stages that are time blocked rather than work, left out of the
    /// CPU's work when the bound is worked out.
    waits: Vec<&'static str>,
    /// Whether the time between frames is counted
    /// ([`BenchRun::time_between_frames`]).
    between: bool,
    /// When the last kept frame started.
    last_started: Option<Instant>,
    /// From each kept frame's start to the next one's.
    intervals: Vec<Duration>,
    /// What each kept frame drew, by name, for a host that draws
    /// ([`BenchRun::record_counts`]).
    counts: Vec<(&'static str, Vec<u64>)>,
    /// The size frames were drawn at, in pixels ([`BenchRun::note_size`]).
    size: Option<(u32, u32)>,
}

impl BenchRun {
    /// Frames skipped before measuring unless a host says otherwise.
    pub const DEFAULT_WARM_UP: usize = 60;

    /// Keeps `frames` frames after `warm_up` ones.
    pub fn new(warm_up: usize, frames: usize) -> Self {
        Self {
            warm_up,
            frames,
            seen: 0,
            samples: StageSamples::new(),
            first_frame: None,
            warm_up_worst: Duration::ZERO,
            startup: Vec::new(),
            gpu: StageSamples::new(),
            gpu_parts: StageSamples::new(),
            drawing: StageSamples::new(),
            latency: Vec::new(),
            waits: Vec::new(),
            between: false,
            last_started: None,
            intervals: Vec::new(),
            counts: Vec::new(),
            size: None,
        }
    }

    /// Counts the time from each frame's start to the next one's, for a
    /// host that shows its frames: the rate they are shown at
    /// ([`BenchRun::shown`]), which the steps' sum misses whenever the
    /// host waits between frames (for the display, or for the GPU to
    /// finish the last one). The bound is then worked out against that
    /// time. A host that steps as fast as it can has no use for it.
    pub fn time_between_frames(&mut self) {
        self.between = true;
    }

    /// How the kept frames came, start to start: the frame rate as shown.
    /// `None` unless [`BenchRun::time_between_frames`] was asked for and
    /// two frames were kept.
    pub fn shown(&self) -> Option<FramePacing> {
        if !self.between {
            return None;
        }
        FramePacing::of(&self.intervals)
    }

    /// The median time from one kept frame's start to the next one's.
    fn interval(&self) -> Option<Duration> {
        self.shown()?;
        Some(summarize("interval", &self.intervals).median)
    }

    /// Counts the stage `name` as waiting, not work, in the bound.
    pub fn wait_stage(&mut self, name: &'static str) {
        if !self.waits.contains(&name) {
            self.waits.push(name);
        }
    }

    /// Notes the GPU's time for a frame and each of its passes (passes it
    /// didn't draw left out). Only kept while measuring, not warming up:
    /// call it before [`BenchRun::record`] for the same frame.
    pub fn record_gpu(&mut self, frame: Duration, passes: &[(&'static str, Duration)]) {
        if self.seen >= self.warm_up && !self.done() {
            self.gpu.record(frame, passes);
        }
    }

    /// Notes the GPU's time within the depth prepass and the scene pass by
    /// what was drawn (nothing where `parts` is empty: a GPU that can't be
    /// stamped between draws). Like [`BenchRun::record_gpu`], only kept
    /// while measuring: call it before [`BenchRun::record`].
    pub fn record_gpu_parts(&mut self, parts: &[(&'static str, Duration)]) {
        if self.seen >= self.warm_up && !self.done() && !parts.is_empty() {
            let total = parts.iter().map(|(_, time)| *time).sum();
            self.gpu_parts.record(total, parts);
        }
    }

    /// Notes where the drawing's time went on the frame's thread this
    /// frame, section by section. Like [`BenchRun::record_gpu`], only
    /// kept while measuring: call it before [`BenchRun::record`].
    pub fn record_drawing(&mut self, sections: &[(&'static str, Duration)]) {
        if self.seen >= self.warm_up && !self.done() {
            let total = sections.iter().map(|(_, time)| *time).sum();
            self.drawing.record(total, sections);
        }
    }

    /// Notes what this frame drew, by name (draw calls, items, the ray
    /// traced scene's instances): the report gives each one's median.
    /// A frame rate that moved with these moved because the scene did.
    /// Like [`BenchRun::record_gpu`], only kept while measuring: call it
    /// before [`BenchRun::record`].
    pub fn record_counts(&mut self, counts: &[(&'static str, u64)]) {
        if self.seen < self.warm_up || self.done() {
            return;
        }
        for &(name, count) in counts {
            match self.counts.iter_mut().find(|(known, _)| *known == name) {
                Some((_, kept)) => kept.push(count),
                None => self.counts.push((name, vec![count])),
            }
        }
    }

    /// Each count's median over the kept frames, in the order first
    /// noted.
    pub fn counts(&self) -> Vec<(&'static str, u64)> {
        self.counts
            .iter()
            .map(|(name, kept)| {
                let mut sorted = kept.clone();
                sorted.sort_unstable();
                (*name, sorted[sorted.len() / 2])
            })
            .collect()
    }

    /// Notes the size frames are drawn at, in pixels: what the window
    /// really is, which a window system may make smaller than was asked
    /// for.
    pub fn note_size(&mut self, width: u32, height: u32) {
        self.size = Some((width, height));
    }

    /// Notes frames' times from their start (when the host read input)
    /// to their picture being finished on the GPU: the part of input
    /// latency the engine decides. They arrive a frame or two after the
    /// frames they're of, so they're kept from the end of the warm-up on.
    pub fn record_latency(&mut self, times: &[Duration]) {
        if self.seen >= self.warm_up {
            self.latency.extend_from_slice(times);
        }
    }

    /// The frames' times to a finished picture, summarized (named
    /// `latency`), if any were noted.
    pub fn latency(&self) -> Option<StageSummary> {
        (!self.latency.is_empty()).then(|| summarize("latency", &self.latency))
    }

    /// What limits the measured frames, by their medians.
    pub fn bound(&self) -> Bound {
        let steps = millis(self.samples.frame_summary().median);
        let waited: f64 = self
            .samples
            .summaries()
            .iter()
            .filter(|summary| self.waits.contains(&summary.name))
            .map(|summary| millis(summary.median))
            .sum();
        let gpu = (self.gpu.frames() > 0).then(|| millis(self.gpu.frame_summary().median));
        // A frame lasts until the next one starts, where that was timed:
        // a host that waits between frames isn't busy for that wait.
        let frame = self.interval().map_or(steps, millis);
        Bound::of(frame, steps - waited, gpu)
    }

    /// Notes one frame. True once every frame to keep has been kept.
    pub fn record(&mut self, laps: &Laps) -> bool {
        self.seen += 1;
        let total = laps.total();
        self.first_frame.get_or_insert(total);
        if self.seen > self.warm_up {
            if let Some(last) = self.last_started.replace(laps.started()) {
                self.intervals
                    .push(laps.started().saturating_duration_since(last));
            }
            self.samples.record_laps(laps);
        } else {
            self.warm_up_worst = self.warm_up_worst.max(total);
        }
        self.done()
    }

    /// Notes a one-off time, such as launch to first frame, once: a second
    /// note of the same name is ignored.
    pub fn note(&mut self, name: &'static str, time: Duration) {
        if !self.startup.iter().any(|(seen, _)| *seen == name) {
            self.startup.push((name, time));
        }
    }

    /// Whether a one-off time called `name` was noted.
    pub fn noted(&self, name: &str) -> bool {
        self.startup.iter().any(|(seen, _)| *seen == name)
    }

    /// The first frame, the warm-up's worst frame, then each noted time,
    /// in milliseconds, in the order they're reported.
    pub fn startup(&self) -> Vec<(&'static str, Duration)> {
        let mut times = Vec::new();
        if let Some(first) = self.first_frame {
            times.push(("first frame", first));
            times.push(("worst warm-up frame", self.warm_up_worst));
        }
        times.extend(self.startup.iter().copied());
        times
    }

    /// True once every frame to keep has been kept.
    pub fn done(&self) -> bool {
        self.samples.frames() >= self.frames
    }

    /// The table, then the JSON line, as hosts print them when done.
    pub fn finish_text(&self, host: &str, scene: &str) -> String {
        let mut text = self.samples.report(&format!("{host}: {scene}"));
        if self.drawing.frames() > 0 {
            text.push_str(
                &self
                    .drawing
                    .table("Drawing on the frame's thread, by section"),
            );
        }
        if self.gpu.frames() > 0 {
            text.push_str(&self.gpu.table("GPU, from timestamps"));
        }
        if self.gpu_parts.frames() > 0 {
            text.push_str(
                &self
                    .gpu_parts
                    .table("GPU within the prepass and the scene pass, by what is drawn"),
            );
        }
        let shown = self.shown().zip(self.interval());
        if let Some((shown, _)) = &shown {
            let _ = writeln!(
                text,
                "  as shown: {:.1} FPS ({:.3} ms between frames, 1% low {:.1} FPS)",
                shown.average_fps,
                1000.0 / shown.average_fps.max(1e-9),
                shown.low_1pct_fps
            );
        }
        let latency = self.latency();
        if let Some(latency) = &latency {
            let _ = writeln!(
                text,
                "  input to finished picture: median {:.3} ms, p95 {:.3} ms, worst {:.3} ms ({} frames)",
                millis(latency.median),
                millis(latency.p95),
                millis(latency.max),
                self.latency.len()
            );
        }
        if let Some((width, height)) = self.size {
            let _ = writeln!(text, "  drawn at {width} x {height}");
        }
        let counts = self.counts();
        if !counts.is_empty() {
            let each: Vec<String> = counts
                .iter()
                .map(|(name, count)| format!("{name} {count}"))
                .collect();
            let _ = writeln!(text, "  each frame (median): {}", each.join(", "));
        }
        let bound = self.bound();
        let _ = writeln!(text, "  {}", bound.label());
        let startup = self.startup();
        for (name, time) in &startup {
            let _ = writeln!(text, "  {:<24} {:>10.3}", name, millis(*time));
        }
        let mut json = self.samples.json(host, scene);
        // `"startup":{…}` goes in before the closing brace.
        json.pop();
        if self.drawing.frames() > 0 {
            let _ = write!(json, ",\"drawing\":{}", self.drawing.stages_json());
        }
        if self.gpu.frames() > 0 {
            let _ = write!(json, ",\"gpu\":{}", self.gpu.stages_json());
        }
        if self.gpu_parts.frames() > 0 {
            let _ = write!(json, ",\"gpu_parts\":{}", self.gpu_parts.stages_json());
        }
        if let Some((shown, interval)) = &shown {
            let _ = write!(
                json,
                ",\"shown\":{{\"average_fps\":{:.2},\"interval_ms\":{:.4},\"low_1pct_fps\":{:.2},\"p99_ms\":{:.4}}}",
                shown.average_fps,
                millis(*interval),
                shown.low_1pct_fps,
                shown.p99_ms
            );
        }
        if let Some(latency) = &latency {
            let _ = write!(
                json,
                ",\"latency\":{{\"median_ms\":{:.4},\"p95_ms\":{:.4},\"max_ms\":{:.4},\"frames\":{}}}",
                millis(latency.median),
                millis(latency.p95),
                millis(latency.max),
                self.latency.len()
            );
        }
        if let Some((width, height)) = self.size {
            let _ = write!(json, ",\"size\":[{width},{height}]");
        }
        if !counts.is_empty() {
            json.push_str(",\"counts\":{");
            for (index, (name, count)) in counts.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                let _ = write!(json, "{}:{count}", json_string(name));
            }
            json.push('}');
        }
        let _ = write!(json, ",\"bound\":{}", json_string(bound.label()));
        json.push_str(",\"startup\":{");
        for (index, (name, time)) in startup.iter().enumerate() {
            if index > 0 {
                json.push(',');
            }
            let _ = write!(json, "{}:{:.4}", json_string(name), millis(*time));
        }
        json.push_str("}}");
        format!("{text}{json}\n")
    }
}

/// The scene a benchmark ran: a project folder's last name.
pub fn scene_name(folder: &std::path::Path) -> String {
    folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder.display().to_string())
}

fn summarize(name: &'static str, times: &[Duration]) -> StageSummary {
    if times.is_empty() {
        return StageSummary {
            name,
            mean: Duration::ZERO,
            median: Duration::ZERO,
            p95: Duration::ZERO,
            max: Duration::ZERO,
        };
    }
    let mut sorted = times.to_vec();
    sorted.sort();
    let at = |fraction: f64| {
        let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
        sorted[index]
    };
    let sum: Duration = sorted.iter().sum();
    StageSummary {
        name,
        mean: sum / sorted.len() as u32,
        median: at(0.5),
        p95: at(0.95),
        max: *sorted.last().expect("not empty"),
    }
}

fn millis(time: Duration) -> f64 {
    time.as_secs_f64() * 1000.0
}

fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn stages_are_summarized_per_frame_in_the_order_they_ran() {
        let mut samples = StageSamples::new();
        for n in 1..=100 {
            samples.record(ms(n + 1), &[("Scripts", ms(n)), ("Physics", ms(1))]);
        }
        let summaries = samples.summaries();
        assert_eq!(summaries[0].name, "Scripts");
        assert_eq!(summaries[1].name, "Physics");
        assert_eq!(summaries[0].max, ms(100));
        assert_eq!(summaries[0].median, ms(51));
        assert_eq!(summaries[0].p95, ms(95));
        assert_eq!(summaries[1].mean, ms(1));
        assert_eq!(samples.frame_summary().max, ms(101));
    }

    #[test]
    fn a_stage_a_frame_skips_counts_as_zero_and_repeats_add_up() {
        let mut samples = StageSamples::new();
        samples.record(ms(4), &[("Physics", ms(4))]);
        samples.record(ms(6), &[("Physics", ms(2)), ("Nav", ms(3)), ("Nav", ms(1))]);
        let summaries = samples.summaries();
        let nav = summaries.iter().find(|s| s.name == "Nav").unwrap();
        assert_eq!(nav.max, ms(4));
        assert_eq!(nav.mean, ms(2));
    }

    #[test]
    fn the_json_line_names_the_host_scene_and_every_stage() {
        let mut samples = StageSamples::new();
        samples.record(ms(2), &[("Scripts", ms(2))]);
        let json = samples.json("server", "S10k \"v1\"");
        assert!(json.starts_with("{\"host\":\"server\",\"scene\":\"S10k \\\"v1\\\"\""));
        assert!(json.contains("\"name\":\"Scripts\""));
        assert!(json.contains("\"name\":\"frame\""));
        assert!(json.ends_with('}'));
        assert!(json.contains("\"pacing\""));
    }

    #[test]
    fn a_bench_run_skips_the_warm_up_then_keeps_its_frames() {
        let mut run = BenchRun::new(2, 3);
        let laps = Laps::start();
        assert!(!run.record(&laps));
        assert!(!run.record(&laps));
        assert_eq!(run.samples.frames(), 0);
        assert!(!run.record(&laps));
        assert!(!run.record(&laps));
        assert!(run.record(&laps));
        assert_eq!(run.samples.frames(), 3);
        assert!(
            run.finish_text("server", "S10k")
                .contains("\"host\":\"server\"")
        );
    }

    #[test]
    fn a_bench_run_reports_its_first_frame_worst_warm_up_and_notes() {
        let mut run = BenchRun::new(2, 1);
        let mut slow = Laps::start();
        std::thread::sleep(ms(5));
        slow.lap("Draw list");
        run.record(&Laps::start());
        run.record(&slow);
        run.record(&Laps::start());
        run.note("launch to first frame", ms(40));
        run.note("launch to first frame", ms(99));
        let startup = run.startup();
        let names: Vec<_> = startup.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            [
                "first frame",
                "worst warm-up frame",
                "launch to first frame"
            ]
        );
        assert!(startup[0].1 < ms(5), "the first frame was the fast one");
        assert!(startup[1].1 >= ms(5), "the slow frame was in the warm-up");
        assert_eq!(startup[2].1, ms(40), "a second note is ignored");
        let text = run.finish_text("player", "S10k");
        let json = text.lines().last().unwrap();
        assert!(
            json.ends_with("\"launch to first frame\":40.0000}}"),
            "{json}"
        );
        assert!(json.contains(",\"startup\":{\"first frame\":"), "{json}");
    }

    #[test]
    fn pacing_reports_the_slowest_frames_and_their_spread() {
        // 99 frames of 10 ms and one of 50 ms.
        let mut frames = vec![ms(10); 99];
        frames.push(ms(50));
        let pacing = FramePacing::of(&frames).unwrap();
        assert!((pacing.average_fps - 1000.0 / 10.4).abs() < 1e-9);
        assert!(
            (pacing.low_1pct_fps - 20.0).abs() < 1e-9,
            "the one slow frame"
        );
        assert!(
            (pacing.low_01pct_fps - 20.0).abs() < 1e-9,
            "at least one frame"
        );
        assert!(
            (pacing.p99_ms - 10.0).abs() < 1e-9,
            "99 of 100 frames are 10 ms"
        );
        assert!((pacing.stddev_ms - (0.99f64 * 0.16 + 0.01 * 1568.16).sqrt()).abs() < 1e-6);
        let even = FramePacing::of(&[ms(4); 10]).unwrap();
        assert_eq!(even.stddev_ms, 0.0);
        assert_eq!(even.low_1pct_fps, even.average_fps);
        assert!(FramePacing::of(&[]).is_none());
    }

    #[test]
    fn the_bound_is_whichever_side_is_busy_for_most_of_the_frame() {
        assert_eq!(Bound::of(10.0, 9.9, Some(9.5)), Bound::Gpu);
        assert_eq!(Bound::of(10.0, 9.5, Some(4.0)), Bound::Cpu);
        assert_eq!(Bound::of(16.7, 3.0, Some(4.0)), Bound::Sync);
        assert_eq!(Bound::of(10.0, 9.5, None), Bound::Unknown);
    }

    #[test]
    fn a_wait_lap_splits_off_without_changing_the_total() {
        let mut laps = Laps::start();
        std::thread::sleep(ms(3));
        laps.lap_with_wait("Render", "Present wait", ms(2));
        let stages = laps.stages();
        assert_eq!(stages[0].0, "Render");
        assert_eq!((stages[1].0, stages[1].1), ("Present wait", ms(2)));
        assert_eq!(stages[0].1 + stages[1].1, laps.total());
        // A wait longer than the lap is cut to it.
        let mut laps = Laps::start();
        laps.lap_with_wait("Render", "Present wait", Duration::from_secs(5));
        assert_eq!(laps.stages()[0].1, Duration::ZERO);
    }

    #[test]
    fn a_bench_run_reports_gpu_times_pacing_and_the_bound() {
        let mut run = BenchRun::new(1, 2);
        run.wait_stage("Present wait");
        for _ in 0..3 {
            let mut laps = Laps::start();
            std::thread::sleep(ms(4));
            laps.lap_with_wait("Render", "Present wait", ms(3));
            run.record_gpu(ms(1), &[("Scene", ms(1))]);
            run.record(&laps);
        }
        // The warm-up frame's GPU time isn't kept.
        assert_eq!(run.gpu.frames(), 2);
        assert_eq!(run.bound(), Bound::Sync, "mostly waiting, little GPU work");
        let text = run.finish_text("player", "Patch");
        assert!(text.contains("GPU, from timestamps"), "{text}");
        assert!(text.contains("1% low"), "{text}");
        let json = text.lines().last().unwrap();
        assert!(json.contains("\"pacing\":{\"average_fps\":"), "{json}");
        assert!(json.contains("\"gpu\":[{\"name\":\"Scene\""), "{json}");
        assert!(json.contains("\"bound\":\"sync-bound\""), "{json}");
        let parsed: Vec<&str> = json.matches("\"startup\"").collect();
        assert_eq!(parsed.len(), 1);
    }

    /// A frame that started `at` after `origin` and whose steps took `work`.
    fn frame_at(origin: Instant, at: Duration, work: Duration) -> Laps {
        Laps::timed(origin + at, &[("Render", work)])
    }

    #[test]
    fn a_run_that_counts_the_time_between_frames_reports_the_rate_as_shown() {
        // Frames whose steps take 15 ms but which start 40 ms apart: the
        // rest is waiting for the display, which no step counts.
        let mut run = BenchRun::new(1, 4);
        run.time_between_frames();
        let origin = Instant::now();
        for frame in 0..5 {
            run.record(&frame_at(origin, ms(40 * frame), ms(15)));
        }
        let shown = run.shown().expect("frames were counted");
        assert!((shown.average_fps - 25.0).abs() < 1e-9, "{shown:?}");
        let text = run.finish_text("player", "Patch");
        assert!(
            text.contains("as shown: 25.0 FPS (40.000 ms between frames"),
            "{text}"
        );
        let json = text.lines().last().unwrap();
        assert!(
            json.contains("\"shown\":{\"average_fps\":25.00,\"interval_ms\":40.0000,"),
            "{json}"
        );
    }

    #[test]
    fn a_run_reports_what_its_frames_drew_and_at_what_size() {
        let mut run = BenchRun::new(1, 3);
        let origin = Instant::now();
        // The warm-up frame's counts aren't kept.
        run.record_counts(&[("draw calls", 9), ("items drawn", 9)]);
        run.record(&frame_at(origin, ms(0), ms(1)));
        for (frame, draws) in [(1, 4000), (2, 4400), (3, 4200)] {
            run.record_counts(&[("draw calls", draws), ("items drawn", 15_000)]);
            run.record(&frame_at(origin, ms(10 * frame), ms(1)));
        }
        run.note_size(1996, 998);
        let text = run.finish_text("player", "Homestead");
        assert!(text.contains("  drawn at 1996 x 998\n"), "{text}");
        assert!(
            text.contains("  each frame (median): draw calls 4200, items drawn 15000\n"),
            "{text}"
        );
        let json = text.lines().last().unwrap();
        assert!(json.contains("\"size\":[1996,998]"), "{json}");
        assert!(
            json.contains("\"counts\":{\"draw calls\":4200,\"items drawn\":15000}"),
            "{json}"
        );
        // A host that draws nothing says nothing of either.
        let mut quiet = BenchRun::new(0, 1);
        quiet.record(&frame_at(origin, ms(0), ms(1)));
        let text = quiet.finish_text("server", "S10k");
        assert!(
            !text.contains("drawn at") && !text.contains("each frame"),
            "{text}"
        );
        assert!(
            !text.contains("\"size\"") && !text.contains("\"counts\""),
            "{text}"
        );
    }

    #[test]
    fn the_bound_counts_the_time_between_frames() {
        // 15 ms of steps and 22 ms of GPU work in frames 40 ms apart: the
        // two take turns, so neither limits the frame.
        let frames = |between: bool| {
            let mut run = BenchRun::new(0, 4);
            if between {
                run.time_between_frames();
            }
            let origin = Instant::now();
            for frame in 0..4 {
                run.record_gpu(ms(22), &[("Scene", ms(22))]);
                run.record(&frame_at(origin, ms(40 * frame), ms(15)));
            }
            run
        };
        assert_eq!(frames(true).bound(), Bound::Sync);
        // Without it the steps' sum stands for the frame, as before.
        assert_eq!(frames(false).bound(), Bound::Gpu);
        assert!(frames(false).shown().is_none());
        assert!(
            !frames(false)
                .finish_text("server", "S10k")
                .contains("as shown")
        );
    }

    #[test]
    fn the_time_between_frames_leaves_the_warm_up_out() {
        let mut run = BenchRun::new(2, 2);
        run.time_between_frames();
        let origin = Instant::now();
        // A slow warm-up, then even frames.
        for at in [0, 500, 1000, 1010, 1020] {
            run.record(&frame_at(origin, ms(at), ms(1)));
        }
        let shown = run.shown().unwrap();
        assert!((shown.average_fps - 100.0).abs() < 1e-9, "{shown:?}");
    }

    #[test]
    fn a_run_reports_where_the_drawing_went_section_by_section() {
        let mut run = BenchRun::new(1, 2);
        for _ in 0..3 {
            run.record_drawing(&[("Culling and batches", ms(2)), ("Ray traced scene", ms(3))]);
            run.record(&Laps::start());
        }
        // The warm-up frame's isn't kept.
        assert_eq!(run.drawing.frames(), 2);
        assert_eq!(run.drawing.frame_summary().median, ms(5));
        let text = run.finish_text("player", "Patch");
        assert!(
            text.contains("Drawing on the frame's thread, by section (2 frames)"),
            "{text}"
        );
        assert!(text.contains("Ray traced scene"), "{text}");
        let json = text.lines().last().unwrap();
        assert!(
            json.contains("\"drawing\":[{\"name\":\"Culling and batches\""),
            "{json}"
        );
        // A host that draws nothing prints no such table.
        let mut headless = BenchRun::new(0, 1);
        headless.record(&Laps::start());
        assert!(
            !headless
                .finish_text("server", "S10k")
                .contains("Drawing on")
        );
    }

    #[test]
    fn a_run_reports_the_time_from_a_frames_start_to_its_finished_picture() {
        let mut run = BenchRun::new(1, 3);
        // Noted during the warm-up: not kept.
        run.record_latency(&[ms(500)]);
        run.record(&Laps::start());
        run.record_latency(&[ms(20), ms(30)]);
        run.record(&Laps::start());
        run.record_latency(&[ms(40)]);
        let latency = run.latency().expect("three were kept");
        assert_eq!((latency.median, latency.max), (ms(30), ms(40)));
        let text = run.finish_text("player", "Patch");
        assert!(
            text.contains("input to finished picture: median 30.000 ms, p95 40.000 ms, worst 40.000 ms (3 frames)"),
            "{text}"
        );
        let json = text.lines().last().unwrap();
        assert!(
            json.contains("\"latency\":{\"median_ms\":30.0000,\"p95_ms\":40.0000,\"max_ms\":40.0000,\"frames\":3}"),
            "{json}"
        );
        assert!(BenchRun::new(0, 1).latency().is_none());
    }

    #[test]
    fn the_csv_has_a_line_per_frame_with_each_stage() {
        let mut samples = StageSamples::new();
        samples.record(ms(3), &[("A", ms(1)), ("B", ms(2))]);
        samples.record(ms(4), &[("A", ms(4))]);
        assert_eq!(
            samples.csv(),
            "frame,A,B\n3.0000,1.0000,2.0000\n4.0000,4.0000,0.0000\n"
        );
    }

    #[test]
    fn laps_time_one_stage_after_another() {
        let mut laps = Laps::start();
        laps.lap("A");
        laps.lap("B");
        let names: Vec<_> = laps.stages().iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["A", "B"]);
        let sum: Duration = laps.stages().iter().map(|(_, t)| *t).sum();
        assert_eq!(sum, laps.total());
    }
}
