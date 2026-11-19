"""The licensed-asset intake in one command (Paul's decision 11): a
downloaded CC0 or CC BY mesh to a draft item on the realistic body.
Blender's Python (bpy 5.x with NumPy):

    python -I intake.py --spec <asset>.intake.json --downloads <folder> --work <folder> --out <library>
        [--only record,load,...] [--from <step>]

The steps, in order (each also runs alone: `python -I <step>.py --help`):

| Step | Script | Writes in `--work` |
|---|---|---|
| record | `provenance.py` | `provenance.json` (licence checked, files hashed) |
| load | `load.py` | `loaded.npz`, `textures/` |
| topology | `topology.py` | `topology.npz` (report, fixes, retopology help) |
| conform | `conform.py` | `conformed.npz` (placed, posed to the rest pose) |
| weights | `skinning.py` | `weighted.npz` |
| levels | `levels.py` | `lod0.npz`, `lod1.npz`, ... |
| checks | `checks.py` | `checks.json` (asset-quality.md section 2) |
| export | `fit.py` | the item in `--out` (model, binding, thumbnail, manifest, provenance record), `export.json` |

A step's report is its `.json` beside its `.npz`; this command also
gathers them all into `intake.json`. A refused licence stops the chain at
the first step, before anything is read.
"""

import argparse
import json
import os
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

STEPS = ["record", "load", "topology", "conform", "weights", "levels", "checks", "export"]


def run(name, spec, args):
    if name == "record":
        import provenance
        try:
            return provenance.step(spec, args.downloads, args.work)
        except provenance.Refused as why:
            raise SystemExit(f"refused: {why}")
    if name == "load":
        import load
        return load.step(spec, args.downloads, args.work)
    if name == "topology":
        import topology
        return topology.step(spec, args.work)
    if name == "conform":
        import conform
        return conform.step(spec, args.work)
    if name == "weights":
        import skinning
        return skinning.step(spec, args.work)
    if name == "levels":
        import levels
        return levels.step(spec, args.work)
    if name == "checks":
        import checks
        return checks.step(spec, args.work)
    if name == "export":
        import fit
        return fit.step(spec, args.work, args.out)
    raise SystemExit(f"no step `{name}`: {', '.join(STEPS)}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--downloads", required=True)
    parser.add_argument("--work", required=True)
    parser.add_argument("--out", required=True, help="the character library folder the item goes into")
    parser.add_argument("--only", help="steps to run, by name, `,` between")
    parser.add_argument("--from", dest="start", help="run from this step on")
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    steps = STEPS
    if args.only:
        steps = [s for s in STEPS if s in args.only.split(",")]
    elif args.start:
        steps = STEPS[STEPS.index(args.start):]
    os.makedirs(args.work, exist_ok=True)
    gathered_path = os.path.join(args.work, "intake.json")
    gathered = json.load(open(gathered_path, encoding="utf-8")) if os.path.exists(gathered_path) else {}
    for name in steps:
        started = time.time()
        report = run(name, spec, args)
        gathered[name] = {"seconds": round(time.time() - started, 1), "report": report}
        print(f"{name}: {gathered[name]['seconds']} s")
        with open(gathered_path, "w", newline="\n") as f:
            json.dump(gathered, f, indent=1)
            f.write("\n")
    print(json.dumps({k: v["seconds"] for k, v in gathered.items()}))


if __name__ == "__main__":
    main()
