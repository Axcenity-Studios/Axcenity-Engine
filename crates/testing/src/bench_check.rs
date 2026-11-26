//! `axcenity-bench-check`: runs the benchmark scenes listed in
//! `benchmarks/baselines.json` and compares each with the numbers recorded
//! for this machine, so an optimization that slips back shows up. How to
//! run it, and why it must be pinned: `docs/benchmarks.md` ("Regression
//! check").
//!
//! It runs the release binaries in `target/release` (build them first), one
//! uncounted warm-up round and then `--rounds` counted ones, and takes the
//! median of the runs' medians. A project scene is copied from the projects
//! folder into `target/bench-projects` once (without its `.axcenity`
//! folder) and cooked there, so the owner's project is never written. A
//! `still:<name>` scene is such a copy with its scripts left out and its
//! Sequences not playing, so the camera stands where the world puts it.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::{Map, Value, json};

const BASELINES: &str = "benchmarks/baselines.json";

const USAGE: &str = "usage: axcenity-bench-check --machine <name> [--rounds <n>] [--only <text>]\n\
                     \x20                           [--projects <folder>] [--fresh] [--update]\n\
                     Run it from the repository root after\n\
                     cargo build --release -p axcenity-player -p axcenity-server -p axcenity-testing -p axcenity-editor";

struct Options {
    machine: String,
    rounds: usize,
    only: Option<String>,
    projects: PathBuf,
    fresh: bool,
    update: bool,
}

/// What one scene measured: the frame's median and, from the player, its
/// startup times, (where the GPU can be timed) the GPU frame's median and
/// the time from one frame's start to the next's (the frame as shown),
/// all in milliseconds, and the 1% low frame rate. Then what isn't
/// judged but explains a change: what limits the frame, the size it was
/// drawn at and what each frame drew.
#[derive(Default)]
struct Measured {
    frame_ms: Vec<f64>,
    first_frame_ms: Vec<f64>,
    assets_ms: Vec<f64>,
    gpu_ms: Vec<f64>,
    shown_ms: Vec<f64>,
    low_1pct_fps: Vec<f64>,
    bound: Option<String>,
    size: Option<(u64, u64)>,
    counts: Vec<(String, u64)>,
}

impl Measured {
    /// Adds one run's `report` (the JSON line a host's `--bench` prints).
    fn read(&mut self, report: &Value, host: &str) {
        let median_of = |stages: &Value| {
            stages
                .as_array()
                .and_then(|stages| stages.iter().find(|stage| stage["name"] == "frame"))
                .and_then(|stage| stage["median_ms"].as_f64())
        };
        self.frame_ms.extend(median_of(&report["stages"]));
        self.gpu_ms.extend(median_of(&report["gpu"]));
        self.shown_ms
            .extend(report["shown"]["interval_ms"].as_f64());
        self.low_1pct_fps
            .extend(report["pacing"]["low_1pct_fps"].as_f64());
        if let Some(bound) = report["bound"].as_str() {
            self.bound = Some(bound.to_owned());
        }
        if let [Some(width), Some(height)] =
            [&report["size"][0], &report["size"][1]].map(Value::as_u64)
        {
            self.size = Some((width, height));
        }
        // The last run's counts: the same scene draws the same each run.
        if let Some(counts) = report["counts"].as_object() {
            self.counts = counts
                .iter()
                .filter_map(|(name, count)| Some((name.clone(), count.as_u64()?)))
                .collect();
        }
        if host == "player" {
            let startup = &report["startup"];
            self.first_frame_ms.extend(startup["first frame"].as_f64());
            self.assets_ms
                .extend(startup["launch to assets loaded"].as_f64());
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let options = parse(std::env::args().skip(1))?;
    let text = std::fs::read_to_string(BASELINES).map_err(|err| {
        format!("can't read {BASELINES} ({err}): run this from the repository root")
    })?;
    let mut file: Value = serde_json::from_str(&text)
        .map_err(|err| format!("{BASELINES} isn't valid JSON: {err}"))?;
    let tolerance = |key: &str, default: f64| file["tolerance"][key].as_f64().unwrap_or(default);
    let (frame_tolerance, startup_tolerance) =
        (tolerance("frame", 0.10), tolerance("startup", 0.30));
    let runs = file["runs"].as_array().cloned().unwrap_or_default();
    let recorded = file["machines"][&options.machine]["results"].clone();
    if recorded.is_null() && !options.update {
        return Err(format!(
            "{BASELINES} has no results for machine `{}`: run once with --update to record them",
            options.machine
        ));
    }

    let mut results = Map::new();
    let mut slower = 0;
    let mut failed = 0;
    println!(
        "{:<34} {:<16} {:>10} {:>10} {:>8}  status",
        "run", "measure", "baseline", "now", "change"
    );
    for run in &runs {
        let name = run["name"].as_str().unwrap_or_default();
        if options
            .only
            .as_deref()
            .is_some_and(|only| !name.contains(only))
        {
            continue;
        }
        let measured = match measure(run, &options) {
            Ok(Some(measured)) => measured,
            Ok(None) => {
                println!(
                    "{name:<34} skipped: its project isn't in {}",
                    options.projects.display()
                );
                continue;
            }
            Err(message) => {
                println!("{name:<34} FAILED: {message}");
                failed += 1;
                continue;
            }
        };
        let mut entry = Map::new();
        for (key, label, samples, allowed) in [
            (
                "frame_ms",
                "frame median",
                &measured.frame_ms,
                frame_tolerance,
            ),
            (
                "first_frame_ms",
                "first frame",
                &measured.first_frame_ms,
                startup_tolerance,
            ),
            (
                "assets_ms",
                "assets in",
                &measured.assets_ms,
                startup_tolerance,
            ),
            ("gpu_ms", "GPU frame", &measured.gpu_ms, frame_tolerance),
            (
                "shown_ms",
                "frame as shown",
                &measured.shown_ms,
                frame_tolerance,
            ),
        ] {
            let Some(now) = median(samples) else { continue };
            entry.insert(key.into(), json!((now * 1000.0).round() / 1000.0));
            let Some(base) = recorded[name][key].as_f64() else {
                println!(
                    "{name:<34} {label:<16} {:>10} {now:>10.3} {:>8}  new",
                    "-", "-"
                );
                continue;
            };
            let change = now / base - 1.0;
            let status = if change > allowed {
                slower += 1;
                "SLOWER"
            } else if change < -allowed {
                "faster"
            } else {
                "ok"
            };
            println!(
                "{name:<34} {label:<16} {base:>10.3} {now:>10.3} {:>+7.1}%  {status}",
                change * 100.0
            );
        }
        // The 1% low and the bound are noted, not judged: the low is one
        // frame in a hundred, too noisy to fail a check on.
        // Only hosts that draw (whose GPU was timed) have a frame rate
        // worth showing.
        let drawn = !measured.gpu_ms.is_empty();
        if let Some(low) = median(&measured.low_1pct_fps).filter(|_| drawn) {
            let base = recorded[name]["low_1pct_fps"]
                .as_f64()
                .map_or("-".to_owned(), |base| format!("{base:.1}"));
            println!("{name:<34} {:<16} {base:>10} {low:>10.1}", "1% low fps");
            entry.insert("low_1pct_fps".into(), json!((low * 10.0).round() / 10.0));
        }
        if let Some(bound) = measured.bound.as_ref().filter(|_| drawn) {
            println!("{name:<34} {bound}");
            entry.insert("bound".into(), json!(bound));
        }
        // The size drawn at and what a frame drew: a frame that moved
        // with these moved because the window or the scene did.
        if let Some((width, height)) = measured.size {
            let size = json!([width, height]);
            let base = &recorded[name]["size"];
            if !base.is_null() && *base != size {
                println!(
                    "{name:<34} DRAWN AT {width} x {height}, not the baseline's {base}: its times don't compare"
                );
            }
            entry.insert("size".into(), size);
        }
        if !measured.counts.is_empty() {
            let mut counts = Map::new();
            for (count, now) in &measured.counts {
                if let Some(base) = recorded[name]["counts"][count].as_u64()
                    && base != *now
                {
                    println!("{name:<34} {count:<16} {base:>10} {now:>10}");
                }
                counts.insert(count.clone(), json!(now));
            }
            entry.insert("counts".into(), Value::Object(counts));
        }
        if let Some(note) = recorded[name]["known"].as_str() {
            println!("{name:<34} known: {note}");
            entry.insert("known".into(), json!(note));
        }
        results.insert(name.into(), Value::Object(entry));
    }

    if options.update {
        let machine = &mut file["machines"][&options.machine];
        if machine.is_null() {
            *machine = json!({ "about": "", "results": {} });
        }
        machine["commit"] = json!(git_commit());
        for (name, entry) in results {
            machine["results"][&name] = entry;
        }
        let text = serde_json::to_string_pretty(&file).map_err(|err| err.to_string())?;
        std::fs::write(BASELINES, text + "\n")
            .map_err(|err| format!("can't write {BASELINES}: {err}"))?;
        println!("Recorded this run as `{}` in {BASELINES}.", options.machine);
    }
    if failed > 0 {
        println!("{failed} scene(s) couldn't be measured.");
        return Ok(ExitCode::from(2));
    }
    if options.update {
        return Ok(ExitCode::SUCCESS);
    }
    if slower > 0 {
        println!(
            "{slower} measure(s) slower than the baseline by more than the tolerance. \
             Rerun before and after builds alternately to confirm (docs/benchmarks.md)."
        );
        return Ok(ExitCode::FAILURE);
    }
    println!("No measure slower than its baseline.");
    Ok(ExitCode::SUCCESS)
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        machine: std::env::var("AXCENITY_BENCH_MACHINE").unwrap_or_default(),
        rounds: 3,
        only: None,
        projects: std::env::var_os("AXCENITY_PROJECTS")
            .map(PathBuf::from)
            .unwrap_or_else(default_projects),
        fresh: false,
        update: false,
    };
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| {
            args.next()
                .ok_or(format!("{flag} needs a value\n\n{USAGE}"))
        };
        match arg.as_str() {
            "--machine" => options.machine = value("--machine")?,
            "--rounds" => {
                options.rounds = value("--rounds")?
                    .parse()
                    .ok()
                    .filter(|&rounds| rounds > 0)
                    .ok_or(format!("--rounds takes a number from 1 up\n\n{USAGE}"))?;
            }
            "--only" => options.only = Some(value("--only")?),
            "--projects" => options.projects = PathBuf::from(value("--projects")?),
            "--fresh" => options.fresh = true,
            "--update" => options.update = true,
            _ => return Err(format!("unknown option `{arg}`\n\n{USAGE}")),
        }
    }
    if options.machine.is_empty() {
        return Err(format!(
            "name this machine with --machine (or AXCENITY_BENCH_MACHINE): numbers are only \
             comparable on one machine\n\n{USAGE}"
        ));
    }
    Ok(options)
}

/// `Documents/Axcenity Projects` in the home folder, where the editor makes
/// projects.
fn default_projects() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .unwrap_or_default();
    PathBuf::from(home)
        .join("Documents")
        .join("Axcenity Projects")
}

/// Runs one entry of `runs` a warm-up round and then `rounds` times, or
/// `None` when its project isn't on this machine.
fn measure(run: &Value, options: &Options) -> Result<Option<Measured>, String> {
    let name = run["name"].as_str().unwrap_or_default();
    let host = run["host"].as_str().unwrap_or_default();
    let frames = run["frames"].as_u64().unwrap_or(300).to_string();
    // A run's own options for its host's binary, such as the player's
    // `--size 2560x1440 --quality Ultra` or the server's `--bench-clients 8`.
    let extra: Vec<String> = run["args"]
        .as_array()
        .map(|args| {
            args.iter()
                .filter_map(|arg| arg.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let Some(scene) = scene_folder(run["scene"].as_str().unwrap_or_default(), options)? else {
        return Ok(None);
    };
    // Absolute, since `cargo test` runs editor Play from the crate's folder.
    let scene = std::path::absolute(&scene).map_err(|err| err.to_string())?;
    let scene_arg = scene.to_string_lossy().into_owned();
    let mut measured = Measured::default();
    for round in 0..=options.rounds {
        let mut command = match host {
            "test" | "server" | "player" => {
                let (name, mut args) = match host {
                    "test" => ("axcenity-test", vec![scene_arg.as_str()]),
                    "server" => ("axcenity-server", vec![scene_arg.as_str(), "--port", "0"]),
                    _ => ("axcenity-player", vec![scene_arg.as_str()]),
                };
                args.extend(["--bench", frames.as_str()]);
                args.extend(extra.iter().map(String::as_str));
                binary(name, &args)?
            }
            "editor-play" => {
                let mut command = Command::new("cargo");
                command
                    .args(["test", "-q", "-p", "axcenity-editor", "--release", "--"])
                    .args(["--ignored", "--exact", "play::tests::bench_project_play"])
                    .args(["--nocapture", "--test-threads", "1"])
                    .env("AXCENITY_BENCH_PROJECT", &scene)
                    .env("AXCENITY_BENCH_FRAMES", &frames);
                command
            }
            other => return Err(format!("{name}: unknown host `{other}`")),
        };
        let output = command
            .output()
            .map_err(|err| format!("{name}: couldn't start it ({err})"))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let report = stdout
            .lines()
            .find(|line| line.starts_with("{\"host\""))
            .and_then(|line| serde_json::from_str::<Value>(line).ok())
            .ok_or_else(|| {
                format!(
                    "{name}: no benchmark report in its output\n{stdout}\n{}",
                    String::from_utf8_lossy(&output.stderr)
                )
            })?;
        if round == 0 {
            continue;
        }
        measured.read(&report, host);
    }
    Ok(Some(measured))
}

/// `s10k` is the scene `axcenity-test --make-s10k` writes (made if
/// missing); `project:<name>` is a cooked copy of that project.
fn scene_folder(scene: &str, options: &Options) -> Result<Option<PathBuf>, String> {
    // Scenes `axcenity-test` writes, made once if missing.
    for (name, flag) in [
        ("s10k", "--make-s10k"),
        ("crowd", "--make-crowd"),
        ("streaming", "--make-streaming"),
    ] {
        if scene != name {
            continue;
        }
        let folder = PathBuf::from("target").join(name);
        if !folder.join("world.json").exists() {
            let status = binary("axcenity-test", &[flag, &folder.to_string_lossy()])?
                .status()
                .map_err(|err| err.to_string())?;
            if !status.success() {
                return Err(format!("couldn't write the {name} scene"));
            }
        }
        return Ok(Some(folder));
    }
    // The showcase is made from the owner's Overgrown Carriage, holding
    // the run's camera view; only the camera changes between views.
    if let Some(view) = scene.strip_prefix("showcase:") {
        let source = options
            .projects
            .join(axcenity_testing::showcase::SOURCE_PROJECT);
        if !source.join("world.json").exists() {
            return Ok(None);
        }
        let copy = PathBuf::from("target/bench-projects").join(axcenity_testing::showcase::NAME);
        if options.fresh {
            let _ = std::fs::remove_dir_all(&copy);
        }
        axcenity_testing::showcase::write(&copy, &source, view)
            .map_err(|err| format!("making the showcase: {err}"))?;
        return cook(&copy, axcenity_testing::showcase::NAME).map(Some);
    }
    // `still:<name>` is the project standing still: its own copy, with
    // no script run and no Sequence played.
    let (project, still) = match (scene.strip_prefix("project:"), scene.strip_prefix("still:")) {
        (Some(project), _) => (project, false),
        (None, Some(project)) => (project, true),
        (None, None) => {
            return Err(format!(
                "unknown scene `{scene}`: use s10k, crowd, streaming, showcase:<view>, \
                 project:<name> or still:<name>"
            ));
        }
    };
    let source = options.projects.join(project);
    if !source.join("world.json").exists() {
        return Ok(None);
    }
    let folder = if still {
        format!("{project} (still)")
    } else {
        project.to_owned()
    };
    let copy = PathBuf::from("target/bench-projects").join(folder);
    if options.fresh || !copy.join("world.json").exists() {
        let _ = std::fs::remove_dir_all(&copy);
        copy_project(&source, &copy).map_err(|err| format!("copying {project}: {err}"))?;
        if still {
            make_still(&copy).map_err(|err| format!("making {project} stand still: {err}"))?;
        }
    }
    cook(&copy, project).map(Some)
}

/// Makes the project copy in `folder` stand still: its `scripts` folder
/// is removed (a script may move the camera) and every Sequence's
/// `AutoPlay` is turned off (a cutscene does), so a host's `--bench`
/// draws the view the world itself starts with, every frame.
fn make_still(folder: &Path) -> Result<(), String> {
    let scripts = folder.join("scripts");
    if scripts.exists() {
        std::fs::remove_dir_all(&scripts).map_err(|err| err.to_string())?;
    }
    let path = folder.join("world.json");
    let text = std::fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let mut world: Value = serde_json::from_str(&text).map_err(|err| err.to_string())?;
    stop_sequences(&mut world);
    let text = serde_json::to_string_pretty(&world).map_err(|err| err.to_string())?;
    std::fs::write(&path, text).map_err(|err| err.to_string())
}

/// Turns `AutoPlay` off on every Sequence under `node`.
fn stop_sequences(node: &mut Value) {
    match node {
        Value::Object(fields) => {
            if fields.get("class").and_then(Value::as_str) == Some("Sequence")
                && let Some(auto_play) = fields
                    .get_mut("properties")
                    .and_then(|properties| properties.get_mut("AutoPlay"))
            {
                *auto_play = json!(false);
            }
            fields.values_mut().for_each(stop_sequences);
        }
        Value::Array(items) => items.iter_mut().for_each(stop_sequences),
        _ => {}
    }
}

/// Cooks what isn't current yet in `copy`, as opening the project in the
/// editor does.
fn cook(copy: &Path, name: &str) -> Result<PathBuf, String> {
    let output = binary("axcenity-editor", &["--cook", &copy.to_string_lossy()])?
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "cooking {name} failed:\n{}",
            String::from_utf8_lossy(&output.stdout)
        ));
    }
    Ok(copy.to_path_buf())
}

/// Copies a project folder, leaving out this computer's `.axcenity` cache.
fn copy_project(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_name() == ".axcenity" {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_project(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// A release binary of this workspace, with `args`.
fn binary(name: &str, args: &[&str]) -> Result<Command, String> {
    let path =
        PathBuf::from("target/release").join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    if !path.exists() {
        return Err(format!("{} isn't built yet\n\n{USAGE}", path.display()));
    }
    let mut command = Command::new(path);
    command.args(args);
    Ok(command)
}

fn median(samples: &[f64]) -> Option<f64> {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    match sorted.len() {
        0 => None,
        n if n % 2 == 1 => Some(sorted[n / 2]),
        n => Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0),
    }
}

fn git_commit() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_median_of_an_even_count_is_the_middle_twos_mean() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), Some(2.5));
    }

    #[test]
    fn a_machine_name_is_needed_and_rounds_must_be_positive() {
        let parse_all = |args: &[&str]| parse(args.iter().map(|arg| arg.to_string()));
        // Only checkable when the machine isn't named in the environment.
        if std::env::var_os("AXCENITY_BENCH_MACHINE").is_none() {
            assert!(parse_all(&[]).is_err());
        }
        let options = parse_all(&["--machine", "desk", "--rounds", "5", "--only", "s10k"]).unwrap();
        assert_eq!((options.machine.as_str(), options.rounds), ("desk", 5));
        assert_eq!(options.only.as_deref(), Some("s10k"));
        assert!(parse_all(&["--machine", "desk", "--rounds", "0"]).is_err());
        assert!(parse_all(&["--machine", "desk", "--fast"]).is_err());
    }

    #[test]
    fn a_still_copy_runs_no_script_and_plays_no_sequence() {
        let root = std::env::temp_dir().join(format!("bench-check-still-{}", std::process::id()));
        std::fs::create_dir_all(root.join("scripts")).unwrap();
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("scripts/tour.client.lua"), "print('hi')").unwrap();
        std::fs::write(root.join("assets/a.png"), "a").unwrap();
        let world = json!({
            "workspace": { "children": [
                { "class": "Sequence", "name": "Tour",
                  "properties": { "SequenceId": "assets/Tour.json", "AutoPlay": true } },
                { "class": "Part", "name": "AutoPlay", "properties": { "Anchored": true } },
                { "class": "Folder", "children": [
                    { "class": "Sequence", "properties": { "AutoPlay": true, "Looped": true } },
                ] },
            ] }
        });
        std::fs::write(root.join("world.json"), world.to_string()).unwrap();
        make_still(&root).unwrap();
        assert!(!root.join("scripts").exists());
        assert!(root.join("assets/a.png").exists());
        let still: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("world.json")).unwrap())
                .unwrap();
        let children = &still["workspace"]["children"];
        assert_eq!(children[0]["properties"]["AutoPlay"], json!(false));
        assert_eq!(
            children[0]["properties"]["SequenceId"],
            json!("assets/Tour.json")
        );
        // Nothing else is touched.
        assert_eq!(children[1], world["workspace"]["children"][1]);
        let nested = &children[2]["children"][0]["properties"];
        assert_eq!(
            (&nested["AutoPlay"], &nested["Looped"]),
            (&json!(false), &json!(true))
        );
        // Making it again changes nothing.
        make_still(&root).unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_players_report_gives_the_frame_as_shown_and_what_it_drew() {
        let report = json!({
            "host": "player",
            "stages": [{ "name": "Render", "median_ms": 9.0 }, { "name": "frame", "median_ms": 10.5 }],
            "gpu": [{ "name": "Scene", "median_ms": 2.0 }, { "name": "frame", "median_ms": 12.25 }],
            "pacing": { "low_1pct_fps": 61.5 },
            "shown": { "average_fps": 70.0, "interval_ms": 14.25 },
            "size": [1996, 998],
            "counts": { "draw calls": 4200, "traced instances": 23000 },
            "bound": "GPU-bound",
            "startup": { "first frame": 900.0, "launch to assets loaded": 6000.0 }
        });
        let mut measured = Measured::default();
        measured.read(&report, "player");
        measured.read(&report, "player");
        assert_eq!(measured.frame_ms, [10.5, 10.5]);
        assert_eq!(measured.gpu_ms, [12.25, 12.25]);
        assert_eq!(measured.shown_ms, [14.25, 14.25]);
        assert_eq!(measured.size, Some((1996, 998)));
        assert_eq!(
            measured.counts,
            [
                ("draw calls".to_owned(), 4200),
                ("traced instances".to_owned(), 23000)
            ]
        );
        assert_eq!(measured.bound.as_deref(), Some("GPU-bound"));
        assert_eq!(measured.assets_ms, [6000.0, 6000.0]);
        // A host that shows nothing has none of these.
        let mut server = Measured::default();
        server.read(
            &json!({ "stages": [{ "name": "frame", "median_ms": 1.0 }] }),
            "server",
        );
        assert_eq!(server.frame_ms, [1.0]);
        assert!(server.shown_ms.is_empty() && server.size.is_none() && server.counts.is_empty());
        assert!(server.assets_ms.is_empty());
    }

    #[test]
    fn a_project_copy_leaves_out_the_local_cache() {
        let root = std::env::temp_dir().join(format!("bench-check-copy-{}", std::process::id()));
        let (from, to) = (root.join("from"), root.join("to"));
        std::fs::create_dir_all(from.join(".axcenity/imported")).unwrap();
        std::fs::create_dir_all(from.join("assets")).unwrap();
        std::fs::write(from.join("world.json"), "{}").unwrap();
        std::fs::write(from.join("assets/a.png"), "a").unwrap();
        std::fs::write(from.join(".axcenity/imported/a.axtex"), "x").unwrap();
        copy_project(&from, &to).unwrap();
        assert!(to.join("world.json").exists() && to.join("assets/a.png").exists());
        assert!(!to.join(".axcenity").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
