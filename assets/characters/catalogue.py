"""Writes `catalogue.json`, the index of this character library: every base
model manifest (`*.base.json`) and item manifest (`*.item.json`) under this
folder, with what a browser shows without opening each manifest. The same
index as `avatar::Catalogue::of`; `crates/avatar/tests/shipped_library.rs`
fails when this file is stale. Plain Python:

    python -I assets/characters/catalogue.py
"""

import json
import os
import posixpath

ROOT = os.path.dirname(os.path.abspath(__file__))


def manifests(suffix):
    found = []
    for folder, _, files in os.walk(ROOT):
        for name in files:
            if name.endswith(suffix):
                path = os.path.relpath(os.path.join(folder, name), ROOT)
                found.append(path.replace(os.sep, "/"))
    return found


def read(path):
    with open(os.path.join(ROOT, path)) as f:
        return json.load(f)


def resolve(manifest, relative):
    return posixpath.normpath(posixpath.join(posixpath.dirname(manifest), relative))


def main():
    bases, items = [], []
    for path in manifests(".base.json"):
        base = read(path)
        bases.append({"id": base["id"], "manifest": path, "label": base["label"],
                      "family": base["family"]})
    for path in manifests(".item.json"):
        item = read(path)
        entry = {"id": item["id"], "manifest": path, "label": item["label"],
                 "kind": item["kind"], "slot": item["slot"],
                 "tier": item.get("tier", "built_in"), "bases": item["bases"]}
        if item.get("tags"):
            entry["tags"] = item["tags"]
        if item.get("thumbnail"):
            entry["thumbnail"] = resolve(path, item["thumbnail"])
        items.append(entry)
    catalogue = {
        "format_version": 1,
        "bases": sorted(bases, key=lambda entry: entry["id"]),
        "items": sorted(items, key=lambda entry: entry["id"]),
    }
    with open(os.path.join(ROOT, "catalogue.json"), "w", newline="\n") as f:
        json.dump(catalogue, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
