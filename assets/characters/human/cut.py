"""Who each garment and hairstyle of the human base is made for.

Character Studio's Hair and Clothing pages list what is made for the
character's body type with the unisex items, or one body type's, or
everything (`crates/editor/src/character_workspace/browser.rs`, `Cut`). An
item says which with one tag: `masculine`, `feminine` or `unisex`.

MakeHuman's files say it for a third of the clothes (the tags `male` and
`female`) and for none of the hairstyles, so the rest is set here by hand,
from each item on both bodies (Paul, 2026-11-25: "Separate male and female
clothing and hairstyle options"). An item not listed takes MakeHuman's own
tag, and is unisex when it has neither.

`wardrobe.py` and `hair.py` call `tags_with_cut` when they write a manifest.
Run on its own (plain Python, no Blender) it brings the manifests already in
a library up to date, without converting anything again:

    python3 assets/characters/human/cut.py --library assets/characters
    python3 assets/characters/catalogue.py

`--check` changes nothing and fails if a manifest is out of date.
"""

import argparse
import glob
import json
import os
import sys

CUTS = ("masculine", "feminine", "unisex")

# By the item's ID after `makehuman/` (hairstyles: `hair-<name>`).
CUT = {
    # Sleeves, stockings and socks worn with a skirt or a dress.
    "armsleeve-fishnet-large": "feminine",
    "armsleeve-fishnet-medium": "feminine",
    "armsleeve-fishnet-small": "feminine",
    "armsleeve-white-l": "feminine",
    "armsleeve-white-m": "feminine",
    "armsleeve-white-s": "feminine",
    "stockings-fishnet-large": "feminine",
    "stockings-fishnet-medium": "feminine",
    "stockings-fishnet-small": "feminine",
    "stockings-white-l": "feminine",
    "stockings-white-m": "feminine",
    "stockings-white-s": "feminine",
    "sockslacefrill": "feminine",
    "leg-warmer-socks": "feminine",
    # Tops and bottoms cut for the bust and the hips.
    "bodice-style-top": "feminine",
    "camisole-top": "feminine",
    "crudefemaletshirt": "feminine",
    "tank-keyhole-neck": "feminine",
    "turtleneck-halter": "feminine",
    "jean-shorts": "feminine",
    "pants-harem": "feminine",
    # Cut for the male chest: the female one comes through the sweater.
    "crude-male-shirt": "masculine",
    "male-worksuit01": "masculine",
    "sweater-fisherman": "masculine",
    # Either body.
    "cargo-pants": "unisex",
    "t-shirt-basic-tucked": "unisex",
    "crudehighsocks": "unisex",
    "crudelowsocks": "unisex",
    "fedora": "unisex",
    "fedora-cocked": "unisex",
    "shoes01": "unisex",
    "shoes02": "unisex",
    "shoes03": "unisex",
    "shoes04": "unisex",
    "shoes05": "unisex",
    "shoes06": "unisex",
    # Hairstyles.
    "hair-afro01": "unisex",
    "hair-long01": "unisex",
    "hair-short02": "unisex",
    "hair-short03": "unisex",
    "hair-short01": "masculine",
    "hair-short04": "masculine",
    "hair-bob01": "feminine",
    "hair-bob02": "feminine",
    "hair-braid01": "feminine",
    "hair-ponytail01": "feminine",
}

# MakeHuman's own words for it.
THEIRS = {"male": "masculine", "female": "feminine"}


def cut_of(item_id, tags):
    """`masculine`, `feminine` or `unisex` for the item `item_id` (with or
    without `makehuman/`) whose other tags are `tags`."""
    key = item_id.split("/")[-1]
    if key in CUT:
        return CUT[key]
    said = {THEIRS[t] for t in tags if t in THEIRS}
    return said.pop() if len(said) == 1 else "unisex"


def tags_with_cut(item_id, tags):
    """`tags` with the one tag that says who the item is made for."""
    kept = {t for t in tags if t not in CUTS}
    return sorted(kept | {cut_of(item_id, kept)})


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--library", required=True, help="a character library (assets/characters)")
    parser.add_argument("--check", action="store_true", help="change nothing; fail if a manifest is out of date")
    args = parser.parse_args()
    stale = []
    counts = dict.fromkeys(CUTS, 0)
    pattern = os.path.join(args.library, "human", "*", "*", "*.item.json")
    for path in sorted(glob.glob(pattern)):
        with open(path) as f:
            item = json.load(f)
        if not item["id"].startswith("makehuman/"):
            continue
        tags = tags_with_cut(item["id"], item.get("tags", []))
        counts[cut_of(item["id"], tags)] += 1
        if tags == item.get("tags"):
            continue
        stale.append(path)
        if not args.check:
            # The tags keep their place in the file: only they change.
            item["tags"] = tags
            with open(path, "w", newline="\n") as f:
                json.dump(item, f, indent=2)
                f.write("\n")
    print(", ".join(f"{counts[c]} {c}" for c in CUTS))
    for path in stale:
        print(("out of date: " if args.check else "updated: ") + path)
    if args.check and stale:
        sys.exit(1)


if __name__ == "__main__":
    main()
