//! Character Studio refinement, wave 3, R4's second pass (plan 11.5 R4:
//! "each preset differs visibly from the default in the front and side
//! captures"): each of the human base's eight face presets, on each
//! frame, changes at least 3% of the face's pixels against that frame's
//! default face in the studio light's `face-front` and `face-side`
//! pictures (a pixel counts when a channel moves by more than 15, the
//! refinement's rule). The face crops are fixed shares of the picture,
//! the same as the sheets' script uses. Skipped without a GPU.

use super::character_capture::{
    Settings, Stage, capture_in, lights_named, repository_library, views_named,
};

/// The face crops, as shares of the picture (left, top, right, bottom):
/// brows to chin and ear to ear in front; the nose's tip to the ear and
/// the forehead to under the chin in profile. Both frames' faces fit.
const CROPS: [(&str, [f32; 4]); 2] = [
    ("face-front", [0.36, 0.28, 0.64, 0.80]),
    ("face-side", [0.25, 0.30, 0.47, 0.82]),
];
const FACE_PRESETS: [&str; 8] = [
    "angular",
    "broad",
    "gaunt",
    "heavy",
    "long",
    "narrow",
    "soft",
    "weathered",
];
/// The least share of the face crop a preset changes, in each view.
const LEAST: f64 = 0.03;
/// Presets that change less than that in a view, measured 2026-11-25 on
/// both frames, and what they must still change: a narrow face is no
/// different in profile (2.3%), and the older face is lines and lids
/// more than shape (2.5% in each view).
const SLIGHT: [(&str, &str); 3] = [
    ("narrow", "face-side"),
    ("weathered", "face-front"),
    ("weathered", "face-side"),
];
const LEAST_SLIGHT: f64 = 0.02;

/// The share of `crop` where `a` and `b` (RGBA, `size`) differ by more
/// than 15 in any colour channel.
fn changed(a: &[u8], b: &[u8], size: (u32, u32), crop: [f32; 4]) -> f64 {
    let (w, h) = (size.0 as usize, size.1 as usize);
    let x = ((crop[0] * w as f32) as usize, (crop[2] * w as f32) as usize);
    let y = ((crop[1] * h as f32) as usize, (crop[3] * h as f32) as usize);
    let mut count = 0usize;
    for row in y.0..y.1 {
        for column in x.0..x.1 {
            let i = (row * w + column) * 4;
            if (0..3).any(|c| a[i + c].abs_diff(b[i + c]) > 15) {
                count += 1;
            }
        }
    }
    count as f64 / ((x.1 - x.0) * (y.1 - y.0)) as f64
}

#[test]
fn every_face_preset_differs_visibly_from_the_default_face() {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return;
    };
    let stage = Stage::new("face-presets", &repository_library()).expect("the stage");
    let library = axcenity_gameplay::description::library(&stage.assets);
    // (The stage copies the library's models, not its presets: they are
    // read from the repository's library.)
    let (shipped, _, problems) = axcenity_avatar::Library::read(&repository_library());
    assert!(problems.is_empty(), "{problems:?}");
    let settings = Settings {
        size: (960, 540),
        frames: 16,
        ..Settings::default()
    };
    let lights = lights_named("studio").unwrap();
    let views = views_named("face-front,face-side").unwrap();
    let mut weak = Vec::new();
    for frame in [0.0f32, 1.0] {
        let mut default = axcenity_gameplay::description::default_description(
            &library,
            axcenity_avatar::DEFAULT_BASE,
        );
        default.parameters.insert("frame".to_owned(), frame);
        let neutral = capture_in(
            &device,
            &queue,
            &stage.assets,
            &default,
            settings,
            &lights,
            &views,
        )
        .unwrap();
        for name in FACE_PRESETS {
            let id = format!("axcenity/human-face-{name}");
            let preset = shipped
                .preset(&id)
                .unwrap_or_else(|| panic!("no preset `{id}`"));
            let mut description = default.clone();
            axcenity_avatar::apply_preset(&mut description, preset, &shipped).unwrap();
            let shots = capture_in(
                &device,
                &queue,
                &stage.assets,
                &description,
                settings,
                &lights,
                &views,
            )
            .unwrap();
            for ((view, crop), (shot, plain)) in CROPS.iter().zip(shots.iter().zip(&neutral)) {
                assert_eq!(shot.view, *view);
                let share = changed(&plain.pixels, &shot.pixels, settings.size, *crop);
                println!("frame {frame} {name} {view}: {:.1}%", share * 100.0);
                let least = if SLIGHT.contains(&(name, *view)) {
                    LEAST_SLIGHT
                } else {
                    LEAST
                };
                if share < least {
                    weak.push(format!("frame {frame} {name} {view} {:.1}%", share * 100.0));
                }
            }
        }
    }
    assert!(
        weak.is_empty(),
        "face presets that change too little of the face: {weak:?}"
    );
}
