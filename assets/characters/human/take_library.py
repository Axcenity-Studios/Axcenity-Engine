"""Takes the converted garments and hairstyles of a library built by
`wardrobe.py` and `hair.py` (the human base's own, kept outside the
repository with every licence the converters accept) into this package:
only those that are **CC0 1.0** by all three of the converter's report,
the item's own manifest (`source.licence`) and its place in the report
(nothing left out for its licence is looked at again). CC BY garments
stay outside. Writes each folder's `CREDITS.md` and report for what was
taken. Plain Python:

    python -I take_library.py <library>/human [--dry-run]

Rebuild the catalogue afterwards (`../catalogue.py`).
"""

import argparse
import glob
import json
import os
import shutil

HERE = os.path.dirname(os.path.abspath(__file__))
CC0 = ("CC0", "CC0-1.0")
SOURCE_URL = "https://static.makehumancommunity.org/assets/assetpacks.html"
KINDS = (("wardrobe", "wardrobe.report.json", "garment", "wardrobe.py", "# Wardrobe credits"),
         ("hair", "hair.report.json", "hairstyle", "hair.py", "# Hair credits"))


def folders(root):
    """{item id: (folder, manifest)} of the items under `root`."""
    out = {}
    for path in glob.glob(os.path.join(root, "*", "*.item.json")):
        with open(path, encoding="utf-8") as file:
            out[json.load(file)["id"]] = (os.path.dirname(path), path)
    return out


def credits(path, taken, what, script, title):
    lines = [
        title, "",
        f"Every {what} here is a MakeHuman community asset, converted for Axcenity's human base by",
        f"`assets/characters/human/{script}` with the MPFB add-on (https://static.makehumancommunity.org/mpfb.html).",
        "The meshes were re-posed, re-skinned to the Axcenity Humanoid skeleton and given the body's shapes;",
        "textures are as their authors made them. Source: " + SOURCE_URL, "",
        "All are CC0 1.0 (https://creativecommons.org/publicdomain/zero/1.0/): credit isn't asked for, and is",
        "given all the same.", "",
        "| Item | Author | Licence | Pack |", "|---|---|---|---|",
    ]
    for t in sorted(taken, key=lambda t: t["id"]):
        lines.append(f"| {t['label']} (`{t['id']}`) | {t['author']} | CC0 1.0 | {t['pack']} |")
    with open(path, "w", encoding="utf-8") as file:
        file.write("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("library", help="the built library's `human` folder")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    for kind, report_name, what, script, title in KINDS:
        source = os.path.join(args.library, kind)
        with open(os.path.join(source, report_name), encoding="utf-8") as file:
            report = json.load(file)
        found = folders(source)
        taken, left = [], list(report.get("left_out", []))
        for entry in report["taken"]:
            folder, manifest = found[entry["id"]]
            with open(manifest, encoding="utf-8") as file:
                item = json.load(file)
            own = (item.get("source") or {}).get("licence")
            if entry["licence"] not in CC0 or own not in CC0:
                left.append({"file": os.path.basename(folder),
                             "reason": f"its licence is {entry['licence']} (the item's own: {own}), not CC0"})
                continue
            taken.append(entry)
            target = os.path.join(HERE, kind, os.path.basename(folder))
            # One already here with the same manifest is the same item
            # (the converter's files differ in a few bytes from run to
            # run): it stays, so a second run changes nothing.
            kept = os.path.join(target, os.path.basename(manifest))
            if os.path.isfile(kept) and open(kept, "rb").read() == open(manifest, "rb").read():
                continue
            if not args.dry_run:
                if os.path.isdir(target):
                    shutil.rmtree(target)
                shutil.copytree(folder, target)
        size = sum(os.path.getsize(os.path.join(root, name))
                   for entry in taken for root, _, names in os.walk(found[entry["id"]][0]) for name in names)
        print(f"{kind}: {len(taken)} taken ({size / 1e6:.0f} MB), {len(left)} left out")
        if args.dry_run:
            continue
        report = dict(report, taken=taken, left_out=left)
        with open(os.path.join(HERE, kind, report_name), "w", encoding="utf-8") as file:
            json.dump(report, file, indent=1)
            file.write("\n")
        credits(os.path.join(HERE, kind, "CREDITS.md"), taken, what, script, title)


if __name__ == "__main__":
    main()
