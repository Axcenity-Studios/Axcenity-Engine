#!/usr/bin/env python3
"""Times Desert Homestead in Axcenity and in its Unity HDRP port: the same
views, window size, frame count and CPU cores, one run at a time.

    run.py [--engine axcenity|unity|both] [--what stills|tour|all]
           [--size 1920x1080] [--runs 3] [--frames 1800] [--only hero,truck]
           [--graphics benchmarks/features/owner-ultra.json] [--label ultra]
           [--shots]   (Unity also saves a picture of each still view)
           [--unity-set clouds=0,...]   (one feature off in Unity, for ablate.sh)
           [--unity-size 2560x1405]     (GNOME gives a window asked to be 2560x1440
                                         2560 x 1405; Unity draws the size it is told)

Each still view is the tour's camera at one moment of a shot (VIEWS). The
tour is the sequence's first 61 seconds, the daytime shots, at a sixtieth
of a second a frame. Every run writes one JSON file in the output folder
(target/cross-engine/homestead/runs), with the frame times the engine
measured itself and what the machine's sensors read while it measured:
the card's load, power and memory, the process's CPU time and memory.

Read README.md beside this file before changing what is compared.
"""
import argparse
import csv
import json
import math
import os
import subprocess
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ENGINE = HERE.parents[2]
WORK = ENGINE / "target/cross-engine/homestead"
UNITY_PROJECT = Path(os.environ.get("HOMESTEAD_UNITY", Path.home() / "Projects/Unity Projects/Desert Homestead HDRP"))
UNITY_PLAYER = UNITY_PROJECT / "Build/DesertHomesteadHDRP.x86_64"
AXCENITY_PLAYER = ENGINE / "target/release/axcenity-player"
# HOMESTEAD_STILL names another copy of the standing-still project, to time
# a change to the world (no clouds, no wind) the same way.
STILL_PROJECT = Path(os.environ.get("HOMESTEAD_STILL", ENGINE / "target/bench-projects/Desert Homestead (still)"))
TOUR_PROJECT = ENGINE / "target/bench-projects/Desert Homestead"
# The 9950X3D's V-cache cores under Linux (docs/benchmarks.md).
CORES = os.environ.get("HOMESTEAD_CORES", "0-7,16-23")
CARD = Path("/sys/class/drm/card1/device")

# Each still view: its name and the tour's second it is taken from.
VIEWS = [("hero", 5.5), ("mailbox", 15.5), ("porch", 25.0), ("truck", 35.0), ("windmill", 45.0), ("crane", 58.0)]
TOUR_SECONDS = 61.0
# Axcenity's --bench warms up for 60 frames while the tour plays.
AXCENITY_WARMUP = 60


def track():
    """The tour's camera, a row a frame (the engine's own, from the dump)."""
    with open(UNITY_PROJECT / "Dump/tour.csv") as file:
        return [{key: float(value) for key, value in row.items()} for row in csv.DictReader(file)]


def look_of(row):
    """The engine's look vector: ahead is -Z, pitch up, yaw toward -X."""
    pitch, yaw = math.radians(max(-89.0, min(89.0, row["pitch"]))), math.radians(row["yaw"])
    return (-math.sin(yaw) * math.cos(pitch), math.sin(pitch), -math.cos(yaw) * math.cos(pitch))


class Sensors(threading.Thread):
    """Reads the card and the process five times a second."""

    def __init__(self, pid):
        super().__init__(daemon=True)
        self.pid = pid
        self.samples = []
        self.stop = threading.Event()
        hwmon = sorted((CARD / "hwmon").glob("hwmon*"))
        self.hwmon = hwmon[0] if hwmon else None
        self.ticks = os.sysconf("SC_CLK_TCK")
        self.page = os.sysconf("SC_PAGE_SIZE")

    @staticmethod
    def number(path):
        try:
            return float(Path(path).read_text().split()[0])
        except (OSError, ValueError, IndexError):
            return None

    def process_vram(self):
        """The process's own memory on the card, from its DRM clients."""
        seen, total = set(), 0.0
        try:
            for entry in Path(f"/proc/{self.pid}/fdinfo").iterdir():
                fields = dict(line.split(":", 1) for line in entry.read_text().splitlines() if ":" in line)
                client = fields.get("drm-client-id")
                if client is None or client in seen or "drm-memory-vram" not in fields:
                    continue
                seen.add(client)
                total += float(fields["drm-memory-vram"].split()[0]) / 1024.0
        except OSError:
            return None
        return total if seen else None

    def run(self):
        while not self.stop.is_set():
            sample = {"t": time.time()}
            sample["gpu_busy"] = self.number(CARD / "gpu_busy_percent")
            vram = self.number(CARD / "mem_info_vram_used")
            sample["card_vram_mb"] = vram / 2**20 if vram is not None else None
            if self.hwmon:
                power = self.number(self.hwmon / "power1_average")
                sample["gpu_watts"] = power / 1e6 if power is not None else None
                clock = self.number(self.hwmon / "freq1_input")
                sample["gpu_mhz"] = clock / 1e6 if clock is not None else None
            try:
                stat = Path(f"/proc/{self.pid}/stat").read_text().rsplit(")", 1)[1].split()
                sample["cpu_seconds"] = (int(stat[11]) + int(stat[12])) / self.ticks
                sample["rss_mb"] = int(Path(f"/proc/{self.pid}/statm").read_text().split()[1]) * self.page / 2**20
            except (OSError, IndexError, ValueError):
                pass
            sample["process_vram_mb"] = self.process_vram()
            self.samples.append(sample)
            self.stop.wait(0.2)

    def summary(self, start, end):
        """Averages over the measured part of the run."""
        inside = [s for s in self.samples if start <= s["t"] <= end]
        if len(inside) < 2:
            return {"samples": len(inside)}
        out = {"samples": len(inside)}
        for key in ("gpu_busy", "gpu_watts", "gpu_mhz", "card_vram_mb", "process_vram_mb", "rss_mb"):
            values = [s[key] for s in inside if s.get(key) is not None]
            if values:
                out[key] = round(sum(values) / len(values), 1)
                out[key + "_max"] = round(max(values), 1)
        first, last = inside[0], inside[-1]
        if "cpu_seconds" in first and "cpu_seconds" in last and last["t"] > first["t"]:
            # Cores' worth of CPU time the process used, all its threads.
            out["cpu_cores_used"] = round((last["cpu_seconds"] - first["cpu_seconds"]) / (last["t"] - first["t"]), 2)
        return out


def launch(command, env, log):
    """Runs one benchmark pinned to CORES with the sensors read beside it."""
    full_env = dict(os.environ, **env)
    with open(log, "w") as out:
        started = time.time()
        process = subprocess.Popen(["taskset", "-c", CORES, *map(str, command)], stdout=out, stderr=subprocess.STDOUT, env=full_env)
        sensors = Sensors(process.pid)
        sensors.start()
        try:
            process.wait(timeout=900)
        except subprocess.TimeoutExpired:
            process.kill()
            raise RuntimeError(f"timed out: see {log}")
        finally:
            sensors.stop.set()
            sensors.join()
    if process.returncode != 0:
        raise RuntimeError(f"exit {process.returncode}: see {log}")
    return started, sensors


def percentile(sorted_values, share):
    return sorted_values[min(len(sorted_values) - 1, int(len(sorted_values) * share))]


def frame_stats(times):
    """What both engines are compared by, from frame-to-frame times in ms."""
    ordered = sorted(times)
    slowest = ordered[-max(1, round(len(ordered) * 0.01)):]
    return {
        "frames": len(times),
        "fps": round(1000.0 * len(times) / sum(times), 2),
        "mean_ms": round(sum(times) / len(times), 3),
        "median_ms": round(ordered[len(ordered) // 2], 3),
        "p95_ms": round(percentile(ordered, 0.95), 3),
        "p99_ms": round(percentile(ordered, 0.99), 3),
        "worst_ms": round(ordered[-1], 3),
        "low_1pct_fps": round(1000.0 * len(slowest) / sum(slowest), 2),
    }


def axcenity_run(name, project, frames, size, graphics, out):
    timeline = out / f"{name}.csv"
    log = out / f"{name}.log"
    started, sensors = launch(
        [AXCENITY_PLAYER, project, "--bench", frames, "--size", size, "--graphics", graphics],
        {"AXCENITY_BENCH_TIMELINE": str(timeline)},
        log,
    )
    result = json.loads(log.read_text().strip().splitlines()[-1])
    with open(f"{timeline.with_suffix('')}.counts.csv") as file:
        counts = list(csv.DictReader(file))
    with open(timeline) as file:
        steps = list(csv.DictReader(file))
    # The timeline has the warm-up's frames too: the measured ones are the last.
    warmup = max(0, len(counts) - int(frames) - 1)
    all_counts, counts = counts, counts[warmup:]
    # Frame to frame, as shown: the steps' times with the wait for the display.
    ends = [float(row["seconds"]) for row in counts]
    times = [(b - a) * 1000.0 for a, b in zip(ends, ends[1:])]
    stages = {stage["name"]: stage["mean_ms"] for stage in result["stages"]}
    record = {
        "engine": "Axcenity",
        "drawn_at": result["size"],
        "bound": result.get("bound"),
        **frame_stats(times),
        "reported_fps": result["shown"]["average_fps"],
        "reported_low_1pct_fps": result["shown"]["low_1pct_fps"],
        "cpu_frame_thread_ms": round(stages["frame"] - stages.get("Present wait", 0.0), 3),
        "cpu_render_step_ms": round(stages.get("Render", 0.0), 3),
        "gpu_ms": round(next((g["mean_ms"] for g in result.get("gpu", []) if g["name"] == "frame"), 0.0), 3),
        "gpu_passes_ms": {g["name"]: round(g["mean_ms"], 3) for g in result.get("gpu", []) if g["name"] != "frame"},
        "gpu_parts_ms": {g["name"]: round(g["mean_ms"], 3) for g in result.get("gpu_parts", []) if g["name"] != "frame"},
        "cpu_sections_ms": {s["name"]: round(s["mean_ms"], 3) for s in result.get("drawing", []) if s["name"] != "frame"},
        "counts": result.get("counts"),
        "triangles": round(sum(float(row["triangles"]) for row in counts) / len(counts)),
        "startup": result.get("startup"),
        "sensors": sensors.summary(started + ends[0], started + ends[-1]),
        "focused_share": round(sum(int(row["focused"]) for row in counts) / len(counts), 3),
    }
    # Every frame of the run, the warm-up's too, each with whether it was measured.
    all_ends = [float(row["seconds"]) for row in all_counts]
    per_frame = [
        {"x": float(row["camera x"]), "y": float(row["camera y"]), "z": float(row["camera z"]),
         "ms": round((end - start) * 1000.0, 4), "measured": index >= warmup,
         "items": int(row["items drawn"]), "triangles": int(row["triangles"])}
        for index, (row, start, end) in enumerate(zip(all_counts[1:], all_ends, all_ends[1:]))
    ]
    return record, per_frame


def unity_run(name, view_args, frames, size, out, shot=None, settings=""):
    result_file = out / f"{name}.unity.json"
    log = out / f"{name}.log"
    command = [UNITY_PLAYER, "-logFile", log.with_suffix(".player.log"), "-hs-out", result_file, "-hs-size", size,
               "-hs-frames", frames, *view_args]
    if shot:
        command += ["-hs-shot", shot]
    if settings:
        command += ["-hs-set", settings]
    # A Wayland window like the engine's own; Unity opens an X11 one otherwise.
    _, sensors = launch(command, {"SDL_VIDEODRIVER": "wayland"}, log)
    result = json.loads(result_file.read_text())
    times = result.pop("frame_ms")
    start = result["measure_start_unix"]
    record = {
        "engine": "Unity HDRP",
        "version": result["version"],
        "graphics_api": result["graphics_api"],
        "ray_tracing_supported": result["ray_tracing_supported"],
        "drawn_at": result["drawn_at"],
        **frame_stats(times),
        "cpu_main_thread_ms": result.get("cpu_main_thread_ms"),
        "cpu_render_thread_ms": result.get("cpu_render_thread_ms"),
        "cpu_frame_ms": result.get("cpu_frame_ms"),
        "gpu_ms": result.get("gpu_ms"),
        "counts": {key: result[key] for key in ("draw_calls", "batches", "set_pass_calls", "shadow_casters") if key in result},
        "triangles": result.get("triangles"),
        "load_ms": result.get("load_ms"),
        "sensors": sensors.summary(start, start + result["seconds"]),
    }
    return record, [{"ms": ms} for ms in times]


def still_project(name, row):
    """A copy of the standing-still project with its camera set by a script."""
    folder = WORK / "projects" / (name if "HOMESTEAD_STILL" not in os.environ else f"{STILL_PROJECT.name}-{name}")
    if not folder.exists():
        folder.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["cp", "-a", "--reflink=auto", str(STILL_PROJECT), str(folder)], check=True)
    look = look_of(row)
    scripts = folder / "scripts"
    scripts.mkdir(exist_ok=True)
    (scripts / "view.client.lua").write_text(
        "-- Written by benchmarks/cross-engine/homestead/run.py: one still view of the tour.\n"
        "local camera = workspace.CurrentCamera\n"
        "camera.CameraType = Enum.CameraType.Scriptable\n"
        f"camera.FieldOfView = {row['fov']}\n"
        f"camera.CFrame = CFrame.lookAt(Vector3.new({row['x']}, {row['y']}, {row['z']}), "
        f"Vector3.new({row['x'] + look[0]}, {row['y'] + look[1]}, {row['z'] + look[2]}))\n"
    )
    return folder


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", default="both", choices=["axcenity", "unity", "both"])
    parser.add_argument("--what", default="all", choices=["stills", "tour", "all"])
    parser.add_argument("--size", default="1920x1080")
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--frames", type=int, default=1800)
    parser.add_argument("--only", default="")
    parser.add_argument("--graphics", default=str(ENGINE / "benchmarks/features/owner-ultra.json"))
    parser.add_argument("--label", default="ultra")
    parser.add_argument("--shots", action="store_true")
    parser.add_argument("--unity-set", default="", help="look changes for Unity's runs (HomesteadBench.Set)")
    parser.add_argument("--unity-size", default="", help="Unity's size, where the desktop gives Axcenity's window another")
    args = parser.parse_args()

    out = WORK / "runs" / f"{args.label}-{args.size}"
    out.mkdir(parents=True, exist_ok=True)
    rows = track()
    only = set(filter(None, args.only.split(",")))
    engines = ["axcenity", "unity"] if args.engine == "both" else [args.engine]
    jobs = []
    if args.what in ("stills", "all"):
        jobs += [(name, rows[round(seconds * 60)]) for name, seconds in VIEWS if not only or name in only]
    if args.what in ("tour", "all") and (not only or "tour" in only):
        jobs.append(("tour", None))

    for run in range(1, args.runs + 1):
        # The engines take turns, so neither has the warmer or the busier machine.
        for name, row in jobs:
            for engine in engines:
                label = f"{name}-{engine}-{run}"
                target = out / f"{label}.json"
                if target.exists():
                    continue
                print(f"{label} …", flush=True)
                if engine == "axcenity":
                    if row is None:
                        frames = round(TOUR_SECONDS * 60) - AXCENITY_WARMUP
                        record, per_frame = axcenity_run(label, TOUR_PROJECT, frames, args.size, args.graphics, out)
                    else:
                        record, per_frame = axcenity_run(label, still_project(name, row), args.frames, args.size, args.graphics, out)
                else:
                    if row is None:
                        record, per_frame = unity_run(label, ["-hs-tour", TOUR_SECONDS], 0, args.unity_size or args.size, out, settings=args.unity_set)
                    else:
                        view = ",".join(str(row[key]) for key in ("x", "y", "z", "pitch", "yaw", "fov"))
                        shot = out / f"{name}-unity.png" if args.shots and run == 1 else None
                        record, per_frame = unity_run(label, ["-hs-view", view], args.frames, args.unity_size or args.size, out, shot, args.unity_set)
                record.update({"view": name, "run": run, "size_asked": args.size, "cores": CORES, "label": args.label})
                target.write_text(json.dumps(record, indent=1))
                (out / f"{label}.frames.json").write_text(json.dumps(per_frame))
                print(f"  {record['fps']} FPS, 1% low {record['low_1pct_fps']}, drawn at {record['drawn_at']}, "
                      f"GPU {record['sensors'].get('gpu_busy')}%, {record['sensors'].get('gpu_watts')} W", flush=True)
                time.sleep(3)


if __name__ == "__main__":
    main()
