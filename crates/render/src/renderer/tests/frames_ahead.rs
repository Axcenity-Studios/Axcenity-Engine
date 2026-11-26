//! One frame ahead: a renderer told to keep to it never has more than one
//! frame waiting behind the one the GPU is drawing.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;

fn one_block() -> DrawList {
    list_of(vec![item(Shape::Block, Vec3::new(0.0, 0.0, -5.0), 1.0)])
}

#[test]
fn a_renderer_one_frame_ahead_has_finished_every_frame_but_the_last() {
    let Some((device, queue)) = test_device() else {
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 256, 256);
    renderer.set_one_frame_ahead(true);
    let list = one_block();
    // Counts the frames whose GPU work is done. wgpu says so from inside
    // a poll, and this test never polls: only the renderer's wait can.
    let finished = Arc::new(AtomicU32::new(0));
    for drawn in 0..8u32 {
        renderer.render(&list).unwrap();
        assert!(
            finished.load(Ordering::Acquire) >= drawn,
            "frame {drawn} came back with {} of the {drawn} before it finished",
            finished.load(Ordering::Acquire)
        );
        let finished = finished.clone();
        queue.on_submitted_work_done(move || {
            finished.fetch_add(1, Ordering::AcqRel);
        });
    }
}

#[test]
fn a_renderer_waits_for_no_frame_unless_told_to() {
    let Some((device, queue)) = test_device() else {
        return;
    };
    // Offline renderers (tests, movies, thumbnails, benchmarks that time
    // the CPU's drawing) keep submitting without a wait.
    let mut renderer = TextureRenderer::new(&device, &queue, 256, 256);
    let list = one_block();
    for _ in 0..4 {
        renderer.render(&list).unwrap();
        assert_eq!(renderer.stats().gpu_wait, Duration::ZERO);
    }
}

#[test]
fn a_watched_renderer_times_each_frame_from_its_start_to_its_finished_picture() {
    let Some((device, queue)) = test_device() else {
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 256, 256);
    renderer.watch_latency(true);
    let list = one_block();
    for _ in 0..6 {
        // The host read input 3 ms before it had the frame drawn.
        let started = std::time::Instant::now();
        std::thread::sleep(Duration::from_millis(3));
        renderer.frame_started(started);
        renderer.render(&list).unwrap();
    }
    // The watch's own thread notes them: this test never polls.
    let waited = std::time::Instant::now();
    let mut times = Vec::new();
    while times.len() < 6 && waited.elapsed() < Duration::from_secs(5) {
        times.extend(renderer.take_latencies());
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(times.len(), 6, "{times:?}");
    for time in &times {
        assert!(*time >= Duration::from_millis(3), "{times:?}");
        assert!(*time < Duration::from_secs(1), "{times:?}");
    }
    // Unwatched, nothing is timed and no thread is kept.
    renderer.watch_latency(false);
    renderer.frame_started(std::time::Instant::now());
    renderer.render(&list).unwrap();
    assert!(renderer.take_latencies().is_empty());
}
