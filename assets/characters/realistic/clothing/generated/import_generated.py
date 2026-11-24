"""Imports generated garments (text- or image-to-3D models: thick closed
shells in their own size, place and pose) into a character library, through
the engine's intake and fit (`axcenity_avatar::fit::prepare`, `import_garment`).

    cargo build -p axcenity-avatar --release --example import_garment
    python -I assets/characters/realistic/clothing/generated/import_generated.py \
        --source <folder of .glb files> --library <a copy of assets/characters> \
        [--only <text>]... [--textures 2048] [--jobs 4] [--reports <folder>] [--no-dependents]
    python -I .../import_generated.py --library <library> --check

`garments.json` beside this file lists the garments: the start of each
source file's name, the item's ID, label, slot and layer, the hints the
intake can't read from the mesh (`kind`, `ease`, `conform`, `cut_for`,
`triangles`),
the other slots it takes (`also_fills`: a one-piece or a dress takes
`bottom` too, so nothing is worn under it that it would cross), who it is
cut for (`frame`: `masculine`, `feminine` or `unisex`, which become a tag
and, by the list's `frames`, a limit on the body's `frame` for the
browser's "Fits this body"), and at its top the `author` and `licence`
every item is recorded with (an entry's own replace them).
The source files are not in the repository. Each garment is imported in a
copy of the library (so several run at once), then its folder and catalogue
entry are put into `--library`. Every item made is a draft.

**Bottoms first.** A top or a jacket is fitted for the bottoms it will be
worn with (`axcenity_avatar::fit::Hang`): its cloth below the waist hangs
outside theirs, by `worn_most` metres at the most, or, for an entry with
`"worn": "tucked"` (a top that lies on the body: its `layer` is 0, under
the bottoms'), is drawn in under theirs. So the import runs in two rounds:
everything that is fitted to the body alone (the bottoms among them), then
the tops and jackets, each given the bottoms now in the library
(`--worn-with`). The rounds keep themselves whole:

- a top is selected (`--only`) and a bottom it needs isn't in the library:
  the bottom is imported first;
- a bottom is selected: every top and jacket fitted for it is imported
  again after it (`--no-dependents` leaves them, and they are then out of
  date).

`<library>/generated.worn-with.json` records which bottoms (by the SHA-256
of their models) each top was fitted for; `--check` compares it with the
library and the list, and fails when a top is out of date: a bottom was
added, changed or taken out since. The wardrobe pack's `build_pack.py`
runs the same check.
"""

import argparse
import concurrent.futures
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[4]
TOOL = REPO / "target" / "release" / "examples" / "import_garment"
BASE = "axcenity/realistic"
RECORD = "generated.worn-with.json"


def worn(entry):
    """How `entry` is worn with the bottoms: `over` them, `tucked` into
    them, or `None` for what isn't worn with one (a bottom, a dress, a
    one-piece)."""
    if entry["slot"] not in ("top", "outerwear") or "bottom" in entry.get("also_fills", []):
        return None
    return entry.get("worn", "over")


def bottoms_for(entry, listed):
    """The IDs of the bottoms `entry` is fitted for: every bottom worn
    under it by the engine's wearing order (a bottom of its layer or a
    lower one), or, tucked, every bottom."""
    how = worn(entry)
    if how is None:
        return []
    return [b["id"] for b in listed["garments"] if b["slot"] == "bottom" and "bottom" not in b.get("also_fills", [])
            and (how == "tucked" or b["layer"] <= entry["layer"])]


def model_hash(library, item):
    """The SHA-256 of `item`'s model in `library`, or `None` when it has none."""
    catalogue = json.loads((library / "catalogue.json").read_text())
    for listed in catalogue["items"]:
        if listed["id"] == item:
            manifest = library / listed["manifest"]
            model = manifest.parent / json.loads(manifest.read_text())["model"]["file"]
            return hashlib.sha256(model.read_bytes()).hexdigest() if model.exists() else None
    return None


def check(library, listed):
    """What is out of date in `library`: one line for each top or jacket
    that wasn't fitted for the bottoms the library now has."""
    wrong = []
    path = library / RECORD
    record = json.loads(path.read_text())["garments"] if path.exists() else {}
    for entry in listed["garments"]:
        if model_hash(library, entry["id"]) is None:
            continue
        wanted = {b: model_hash(library, b) for b in bottoms_for(entry, listed)}
        wanted = {b: h for b, h in wanted.items() if h}
        if not wanted:
            continue
        have = record.get(entry["id"])
        if have is None:
            wrong.append(f"{entry['id']}: not fitted for any bottom")
            continue
        if have["worn"] != worn(entry):
            wrong.append(f"{entry['id']}: fitted {have['worn']}, listed {worn(entry)}")
        for bottom in sorted(set(wanted) | set(have["with"])):
            if bottom not in have["with"]:
                wrong.append(f"{entry['id']}: not fitted for {bottom}")
            elif bottom not in wanted:
                wrong.append(f"{entry['id']}: fitted for {bottom}, which is gone")
            elif have["with"][bottom] != wanted[bottom]:
                wrong.append(f"{entry['id']}: {bottom} has changed since it was fitted for it")
    return wrong


def command(entry, source, library, listed, textures):
    name = entry["id"].split("/")[-1]
    under = listed["under"]
    cmd = [str(TOOL), str(source), str(library), BASE, entry["id"], entry["label"],
           f"realistic/imports/{name}", "--slot", entry["slot"], "--layer", str(entry["layer"]),
           "--intake", "--author", entry.get("author", listed.get("author", "unknown")),
           "--licence", entry.get("licence", listed.get("licence", "NOASSERTION"))]
    for flag in ("kind", "ease", "conform", "triangles", "scale"):
        if flag in entry:
            cmd += [f"--{flag}", str(entry[flag])]
    for shape, weight in entry.get("cut_for", {}).items():
        cmd += ["--cut-for", f"{shape}={weight}"]
    for slot in entry.get("also_fills", []):
        cmd += ["--also-fills", slot]
    frame = listed.get("frames", {}).get(entry.get("frame", ""), {})
    if "tag" in frame:
        cmd += ["--tag", frame["tag"]]
    if "limit" in frame:
        cmd += ["--limit", frame["limit"]]
    for tag in entry.get("tags", []):
        cmd += ["--tag", tag]
    for item in under["bottom" if entry["slot"] == "bottom" else "other"]:
        cmd += ["--under", item]
    for item in bottoms_for(entry, listed):
        cmd += ["--worn-with", item]
    if worn(entry) == "tucked":
        cmd += ["--tucked"]
    if worn(entry) and "worn_most" in listed:
        cmd += ["--worn-most", str(listed["worn_most"])]
    if textures:
        cmd += ["--textures", str(textures)]
    return cmd


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--source", type=pathlib.Path)
    parser.add_argument("--library", required=True, type=pathlib.Path)
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--textures", type=int, default=2048)
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--reports", type=pathlib.Path,
                        help="a folder for what the tool said of each garment (<name>.txt)")
    parser.add_argument("--no-dependents", action="store_true",
                        help="don't import again the tops fitted for a bottom that is imported")
    parser.add_argument("--check", action="store_true",
                        help="only say which tops are out of date with the library's bottoms")
    args = parser.parse_args()
    listed = json.loads((HERE / "garments.json").read_text())
    if args.check:
        wrong = check(args.library, listed)
        for line in wrong:
            print("OUT OF DATE:", line)
        if not wrong:
            print("every top and jacket is fitted for the library's bottoms")
        sys.exit(1 if wrong else 0)
    if not args.source:
        sys.exit("--source: the folder of .glb files")
    if not TOOL.exists():
        sys.exit(f"build the import tool first: {TOOL} is missing")

    by_id = {e["id"]: e for e in listed["garments"]}
    chosen = {e["id"] for e in listed["garments"] if not args.only or any(o in e["id"] for o in args.only)}
    # A top's bottoms that the library hasn't, first; a bottom's tops, after it.
    for item in sorted(chosen):
        for bottom in bottoms_for(by_id[item], listed):
            if bottom not in chosen and model_hash(args.library, bottom) is None:
                print(f"{bottom}: imported first, for {item}")
                chosen.add(bottom)
    if not args.no_dependents:
        for entry in listed["garments"]:
            needs = [b for b in bottoms_for(entry, listed) if b in chosen]
            if entry["id"] not in chosen and needs and model_hash(args.library, entry["id"]) is not None:
                print(f"{entry['id']}: imported again, fitted for {', '.join(needs)}")
                chosen.add(entry["id"])
    rounds = [[e for e in listed["garments"] if e["id"] in chosen and worn(e) is None],
              [e for e in listed["garments"] if e["id"] in chosen and worn(e) is not None]]

    def one(entry, scratch):
        files = sorted(args.source.glob(entry["file"] + "*.glb"))
        if not files:
            return entry, None, "no source file"
        library = pathlib.Path(scratch) / entry["id"].split("/")[-1]
        subprocess.run(["cp", "-r", "--reflink=auto", str(args.library), str(library)], check=True)
        done = subprocess.run(command(entry, files[0], library, listed, args.textures),
                              capture_output=True, text=True)
        return entry, library, (done.stdout if done.returncode == 0 else None) or done.stderr.strip()

    failed = 0
    record_path = args.library / RECORD
    record = json.loads(record_path.read_text()) if record_path.exists() else {"format": 1, "garments": {}}
    with tempfile.TemporaryDirectory() as scratch, concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        for entries in rounds:
            # Each round's copies are made of the library as the round before left it.
            for entry, library, output in list(pool.map(lambda e: one(e, scratch), entries)):
                name = entry["id"].split("/")[-1]
                if args.reports:
                    args.reports.mkdir(parents=True, exist_ok=True)
                    (args.reports / f"{name}.txt").write_text(output or "")
                folder = library / "realistic" / "imports" / name if library else None
                if folder is None or not folder.is_dir():
                    failed += 1
                    print(f"{entry['id']}: FAILED: {output[-300:]}")
                    continue
                target = args.library / "realistic" / "imports" / name
                shutil.rmtree(target, ignore_errors=True)
                shutil.copytree(folder, target)
                made = [i for i in json.loads((library / "catalogue.json").read_text())["items"]
                        if i["id"] == entry["id"]]
                catalogue = json.loads((args.library / "catalogue.json").read_text())
                catalogue["items"] = [i for i in catalogue["items"] if i["id"] != entry["id"]] + made
                (args.library / "catalogue.json").write_text(json.dumps(catalogue, indent=2) + "\n")
                record["garments"].pop(entry["id"], None)
                if worn(entry):
                    # The bottoms it was fitted for, as its own copy of the library had them.
                    fitted = {b: model_hash(library, b) for b in bottoms_for(entry, listed)}
                    record["garments"][entry["id"]] = {"worn": worn(entry), "with": dict(sorted(fitted.items()))}
                print(f"{entry['id']}: imported")
                shutil.rmtree(library, ignore_errors=True)
            record["garments"] = dict(sorted(record["garments"].items()))
            record_path.write_text(json.dumps(record, indent=2) + "\n")
    wrong = check(args.library, listed)
    for line in wrong:
        print("OUT OF DATE:", line)
    sys.exit(1 if failed or wrong else 0)


if __name__ == "__main__":
    main()
