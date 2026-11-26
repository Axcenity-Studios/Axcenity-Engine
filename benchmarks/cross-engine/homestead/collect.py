#!/usr/bin/env python3
"""Turns run.py's files into tables: collect.py [<runs folder> ...]

Each folder is one of target/cross-engine/homestead/runs (ultra-1920x1080
unless given). For every view the run with the median frame rate of each
engine is shown. The tour is cut into its shots by where the camera is,
leaving out the first second (Axcenity's warm-up) and the half second
after each cut (both engines load what the new view shows).
"""
import csv
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ENGINE = HERE.parents[2]
RUNS = ENGINE / "target/cross-engine/homestead/runs"
TRACK = Path.home() / "Projects/Unity Projects/Desert Homestead HDRP/Dump/tour.csv"
SHOTS = [("hero", 0, 11), ("mailbox", 11, 20), ("porch", 20, 30), ("truck", 30, 40), ("windmill", 40, 50), ("crane", 50, 61)]
ORDER = ["hero", "mailbox", "porch", "truck", "windmill", "crane", "tour"]


def median_run(records):
    return sorted(records, key=lambda r: r["fps"])[len(records) // 2]


def spread(records):
    rates = [r["fps"] for r in records]
    return f"{min(rates):.0f}–{max(rates):.0f}" if len(rates) > 1 else "one run"


def tour_seconds(frames):
    """Each of Axcenity's frames' place in the tour, by its camera."""
    with open(TRACK) as file:
        track = [(float(r["x"]), float(r["y"]), float(r["z"])) for r in csv.DictReader(file)]
    at, out = 0, []
    for frame in frames:
        best, best_distance = at, float("inf")
        for index in range(at, min(at + 8, len(track))):
            x, y, z = track[index]
            distance = (x - frame["x"]) ** 2 + (y - frame["y"]) ** 2 + (z - frame["z"]) ** 2
            if distance < best_distance:
                best, best_distance = index, distance
        at = best
        out.append(best / 60.0)
    return out


def shot_rates(folder, engine, run):
    frames = json.loads((folder / f"tour-{engine}-{run}.frames.json").read_text())
    if engine == "axcenity":
        seconds = tour_seconds(frames)
    else:
        seconds = [index / 60.0 for index in range(len(frames))]
    rates = {}
    for name, start, end in SHOTS:
        times = [f["ms"] for f, t in zip(frames, seconds) if max(start + 0.5, 1.0) <= t < end]
        if times:
            ordered = sorted(times)
            slowest = ordered[-max(1, round(len(ordered) * 0.01)):]
            rates[name] = (1000.0 * len(times) / sum(times), 1000.0 * len(slowest) / sum(slowest), len(times))
    return rates


def table(folder):
    records = {}
    for path in sorted(folder.glob("*.json")):
        if path.name.endswith((".frames.json", ".unity.json")):
            continue
        record = json.loads(path.read_text())
        key = "axcenity" if record["engine"] == "Axcenity" else "unity"
        records.setdefault((record["view"], key), []).append(record)
    views = [view for view in ORDER if (view, "axcenity") in records or (view, "unity") in records]
    print(f"\n### {folder.name}\n")
    print("| View | Axcenity FPS (1% low) | Unity FPS (1% low) | Unity ÷ Axcenity | Axcenity runs | Unity runs | Drawn at |")
    print("|---|---|---|---|---|---|---|")
    for view in views:
        a, u = records.get((view, "axcenity")), records.get((view, "unity"))
        am, um = (median_run(a) if a else None), (median_run(u) if u else None)
        ratio = f"{um['fps'] / am['fps']:.2f}×" if am and um else ""
        cell = lambda m: f"{m['fps']:.1f} ({m['low_1pct_fps']:.0f})" if m else "not run"
        size = "×".join(str(v) for v in (am or um)["drawn_at"])
        print(f"| {view} | {cell(am)} | {cell(um)} | {ratio} | {spread(a) if a else ''} | {spread(u) if u else ''} | {size} |")

    print("\n| View | Engine | Frame ms | GPU ms | CPU ms (frame thread; Unity main + render) | GPU load | GPU W | CPU cores used | Process VRAM MB | Process RAM MB | Triangles | Draw calls |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for view in views:
        for key in ("axcenity", "unity"):
            if (view, key) not in records:
                continue
            m = median_run(records[(view, key)])
            s = m["sensors"]
            if key == "axcenity":
                cpu = f"{m['cpu_frame_thread_ms']:.2f}"
                draws = m["counts"]["draw calls"]
            else:
                cpu = f"{m['cpu_main_thread_ms']:.2f} + {m['cpu_render_thread_ms']:.2f}"
                draws = int(m["counts"]["draw_calls"])
            print(f"| {view} | {m['engine']} | {m['mean_ms']:.2f} | {m['gpu_ms']:.2f} | {cpu} | {s.get('gpu_busy', 0):.0f}% | "
                  f"{s.get('gpu_watts', 0):.0f} | {s.get('cpu_cores_used', 0):.1f} | {s.get('process_vram_mb', 0):.0f} | "
                  f"{s.get('rss_mb', 0):.0f} | {m['triangles'] / 1e6:.1f} M | {draws} |")

    passes = [(view, median_run(records[(view, "axcenity")])) for view in views if (view, "axcenity") in records]
    if passes:
        names = []
        for _, m in passes:
            names += [name for name in m["gpu_passes_ms"] if name not in names]
        print("\nAxcenity's GPU time by pass, ms:\n")
        print("| View | " + " | ".join(names) + " |")
        print("|---|" + "---|" * len(names))
        for view, m in passes:
            print(f"| {view} | " + " | ".join(f"{m['gpu_passes_ms'].get(name, 0):.2f}" for name in names) + " |")

    parts = [(view, m) for view, m in passes if m.get("gpu_parts_ms")]
    if parts:
        names = []
        for _, m in parts:
            names += [name for name in m["gpu_parts_ms"] if name not in names]
        print("\nAxcenity's GPU time within the prepass, the scene pass and the shadows, by what is drawn, ms:\n")
        print("| View | " + " | ".join(names) + " |")
        print("|---|" + "---|" * len(names))
        for view, m in parts:
            print(f"| {view} | " + " | ".join(f"{m['gpu_parts_ms'].get(name, 0):.2f}" for name in names) + " |")

    if (folder / "tour-axcenity-1.frames.json").exists() and (folder / "tour-unity-1.frames.json").exists():
        runs = len(list(folder.glob("tour-axcenity-*.frames.json")))
        print("\nThe tour, shot by shot (median of the runs; FPS and 1% low):\n")
        print("| Shot | Axcenity | Unity | Unity ÷ Axcenity |")
        print("|---|---|---|---|")
        per = {engine: [shot_rates(folder, engine, run) for run in range(1, runs + 1)] for engine in ("axcenity", "unity")}
        for name, _, _ in SHOTS:
            cells, medians = [], []
            for engine in ("axcenity", "unity"):
                rates = sorted(r[name] for r in per[engine] if name in r)
                if not rates:
                    cells.append("")
                    continue
                mid = rates[len(rates) // 2]
                medians.append(mid[0])
                cells.append(f"{mid[0]:.1f} ({mid[1]:.0f})")
            ratio = f"{medians[1] / medians[0]:.2f}×" if len(medians) == 2 else ""
            print(f"| {name} | {cells[0]} | {cells[1]} | {ratio} |")


def main():
    folders = [Path(arg) for arg in sys.argv[1:]] or [RUNS / "ultra-1920x1080"]
    for folder in folders:
        table(folder if folder.is_absolute() or folder.exists() else RUNS / folder)


if __name__ == "__main__":
    main()
