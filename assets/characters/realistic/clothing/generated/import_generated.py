"""Imports canonical A-pose single-shell garments (prepped by
`prep_meshy_garments.py`) into a character library using ADR-0046's
non-destructive proxy fitting and biharmonic inpainting (`import_garment`).

    cargo build -p axcenity-avatar --release --example import_garment
    python3 -I assets/characters/realistic/clothing/generated/import_generated.py \
        --source <folder of prepped .glb files> --library <a copy of assets/characters> \
        [--only <text>]... [--textures 2048] [--jobs 4] [--reports <folder>] \
        [--no-strict-validate]
    python3 -I .../import_generated.py --library <library> --check

`garments.json` beside this file lists the 33 source garments, each tagged with:
- `frame`: `masculine`, `feminine`, or `unisex` (unisex items generate both a
  masculine home-frame primary item and a `-feminine` feminine home-frame
  variant from `<stem>_feminine.glb`).
- `proxy`: `body` or `skirt` (for dresses, skirts, and long coats).
- `slot`, `layer`, `also_fills`, `author`, `licence`.

Under ADR-0046, garments are fitted independently without rest-mesh mutation:
layering uses ADR-0045 UV cover maps plus a live `Shrink_Under` morph target
activated by `avatar::resolve` whenever an overlapping higher layer is worn.
"""

import argparse
import concurrent.futures
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


def expand_entries(listed):
    """Expand `garments.json` into concrete import jobs. Unisex garments produce
    both a masculine home-frame primary item and a `-feminine` home-frame variant."""
    jobs = []
    for entry in listed["garments"]:
        frame = entry.get("frame", "masculine")
        if frame == "unisex":
            masc = dict(entry)
            masc["_home_frame"] = "masculine"
            masc["_suffix"] = ""
            jobs.append(masc)

            fem = dict(entry)
            fem["id"] = entry["id"] + "-feminine"
            fem["label"] = entry["label"] + " (Feminine)"
            fem["frame"] = "feminine"
            fem["_home_frame"] = "feminine"
            fem["_suffix"] = "_feminine"
            jobs.append(fem)
        else:
            job = dict(entry)
            job["_home_frame"] = frame
            job["_suffix"] = ""
            jobs.append(job)
    return jobs


def resolve_source(source_dir: pathlib.Path, entry: dict):
    """Find the matching prepped `.glb` file for `entry` (accounting for `_feminine` suffix)."""
    suffix = entry.get("_suffix", "")
    matches = sorted(source_dir.glob(entry["file"] + "*.glb"))
    if suffix:
        fem_matches = [p for p in matches if p.stem.endswith(suffix)]
        return fem_matches[0] if fem_matches else None
    non_fem = [p for p in matches if not p.stem.endswith("_feminine")]
    return non_fem[0] if non_fem else None


def command(entry, source, library, listed, textures, strict_validate):
    name = entry["id"].split("/")[-1]
    under = listed["under"]
    cmd = [
        str(TOOL),
        str(source),
        str(library),
        BASE,
        entry["id"],
        entry["label"],
        f"realistic/imports/{name}",
        "--slot",
        entry["slot"],
        "--layer",
        str(entry["layer"]),
        "--frame",
        entry["_home_frame"],
        "--proxy",
        entry.get("proxy", "body"),
        "--canonical",
        "--author",
        entry.get("author", listed.get("author", "unknown")),
        "--licence",
        entry.get("licence", listed.get("licence", "NOASSERTION")),
    ]
    if strict_validate:
        cmd += ["--strict-validate"]
    if not entry.get("has_sleeves", False):
        cmd += ["--sleeveless"]
    for slot in entry.get("also_fills", []):
        cmd += ["--also-fills", slot]
    frame_meta = listed.get("frames", {}).get(entry.get("frame", ""), {})
    if "tag" in frame_meta:
        cmd += ["--tag", frame_meta["tag"]]
    if "limit" in frame_meta:
        cmd += ["--limit", frame_meta["limit"]]
    for tag in entry.get("tags", []):
        cmd += ["--tag", tag]
    for item in under["bottom" if entry["slot"] == "bottom" else "other"]:
        cmd += ["--under", item]
    if textures:
        cmd += ["--textures", str(textures)]
    return cmd


def check_library(library: pathlib.Path, listed: dict):
    """Verify every imported `generated/*` item in `library` has an existing model and manifest."""
    catalogue_path = library / "catalogue.json"
    if not catalogue_path.exists():
        return [f"missing catalogue: {catalogue_path}"]
    catalogue = json.loads(catalogue_path.read_text())
    wrong = []
    for item in catalogue.get("items", []):
        if not item["id"].startswith("generated/"):
            continue
        manifest_path = library / item["manifest"]
        if not manifest_path.exists():
            wrong.append(f"{item['id']}: missing manifest {item['manifest']}")
            continue
        manifest = json.loads(manifest_path.read_text())
        model_path = manifest_path.parent / manifest["model"]["file"]
        if not model_path.exists():
            wrong.append(f"{item['id']}: missing model {model_path}")
    return wrong


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--source", type=pathlib.Path)
    parser.add_argument("--library", required=True, type=pathlib.Path)
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--textures", type=int, default=2048)
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument(
        "--reports",
        type=pathlib.Path,
        help="a folder for what the tool said of each garment (<name>.txt)",
    )
    parser.add_argument(
        "--no-strict-validate",
        action="store_true",
        help="allow importing garments with validation warnings",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify all imported generated garments in --library are intact",
    )
    args = parser.parse_args()
    listed = json.loads((HERE / "garments.json").read_text())
    if args.check:
        wrong = check_library(args.library, listed)
        for line in wrong:
            print("BROKEN:", line)
        if not wrong:
            print("all imported generated garments in library are intact (ADR-0046)")
        sys.exit(1 if wrong else 0)
    if not args.source:
        sys.exit("--source: the folder of prepped .glb files")
    if not TOOL.exists():
        sys.exit(f"build the import tool first: {TOOL} is missing")

    all_jobs = expand_entries(listed)
    chosen = [
        e
        for e in all_jobs
        if not args.only or any(o in e["id"] or o in e["file"] for o in args.only)
    ]
    strict_validate = not args.no_strict_validate

    def one(entry, scratch):
        src = resolve_source(args.source, entry)
        if src is None:
            return entry, None, f"no source file for {entry['file']}{entry.get('_suffix', '')}"
        library = pathlib.Path(scratch) / entry["id"].split("/")[-1]
        subprocess.run(["cp", "-r", "--reflink=auto", str(args.library), str(library)], check=True)
        done = subprocess.run(
            command(entry, src, library, listed, args.textures, strict_validate),
            capture_output=True,
            text=True,
        )
        out = (done.stdout or "") + ("\n" + done.stderr if done.stderr else "")
        return entry, library if done.returncode == 0 else None, out.strip()

    failed = 0
    with tempfile.TemporaryDirectory() as scratch, concurrent.futures.ThreadPoolExecutor(
        args.jobs
    ) as pool:
        for entry, library, output in list(pool.map(lambda e: one(e, scratch), chosen)):
            name = entry["id"].split("/")[-1]
            if args.reports:
                args.reports.mkdir(parents=True, exist_ok=True)
                (args.reports / f"{name}.txt").write_text((output or "") + "\n")
            folder = library / "realistic" / "imports" / name if library else None
            if folder is None or not folder.is_dir():
                failed += 1
                print(f"{entry['id']}: FAILED: {output[-300:]}")
                continue
            target = args.library / "realistic" / "imports" / name
            shutil.rmtree(target, ignore_errors=True)
            shutil.copytree(folder, target)
            made = [
                i
                for i in json.loads((library / "catalogue.json").read_text())["items"]
                if i["id"] == entry["id"]
            ]
            catalogue = json.loads((args.library / "catalogue.json").read_text())
            catalogue["items"] = [i for i in catalogue["items"] if i["id"] != entry["id"]] + made
            (args.library / "catalogue.json").write_text(json.dumps(catalogue, indent=2) + "\n")
            first_line = output.splitlines()[0] if output else "imported"
            print(f"{entry['id']}: {first_line}")
            shutil.rmtree(library, ignore_errors=True)

    wrong = check_library(args.library, listed)
    for line in wrong:
        print("BROKEN:", line)
    sys.exit(1 if failed or wrong else 0)


if __name__ == "__main__":
    main()
