"""Fetches the CC0 fabric tiles the wardrobe's maps are baked from into a
folder of your choice (they are not stored in the repository). Plain
Python:

    python -I assets/characters/realistic/clothing/fabrics.py --out <folder>

Poly Haven (polyhaven.com/license: CC0 1.0): Cotton Jersey, Jogging
Melange, Leather White, at 2K, their colour, normal, roughness, occlusion
and displacement JPEGs. ambientCG (docs.ambientcg.com/license: CC0 1.0):
Rubber004 at 1K (its zip, of which the colour, displacement and roughness
JPEGs are kept). The sources are recorded in
docs/development/character-studio/references/README.md.
"""

import argparse
import io
import os
import urllib.request
import zipfile

POLY_HAVEN = ("cotton_jersey", "jogging_melange", "leather_white")
KINDS = ("diff", "nor_gl", "rough", "ao", "disp")
AMBIENTCG = "Rubber004"
AMBIENTCG_KEEP = ("Color", "Displacement", "Roughness", "NormalGL")


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "garment-build"})
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    for name in POLY_HAVEN:
        folder = os.path.join(args.out, name)
        os.makedirs(folder, exist_ok=True)
        for kind in KINDS:
            file = f"{name}_{kind}_2k.jpg"
            url = f"https://dl.polyhaven.org/file/ph-assets/Textures/jpg/2k/{name}/{file}"
            with open(os.path.join(folder, file), "wb") as f:
                f.write(fetch(url))
            print(file)
    folder = os.path.join(args.out, AMBIENTCG.lower())
    os.makedirs(folder, exist_ok=True)
    data = fetch(f"https://ambientcg.com/get?file={AMBIENTCG}_1K-JPG.zip")
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        for member in z.namelist():
            if member.endswith(".jpg") and any(k in member for k in AMBIENTCG_KEEP):
                with open(os.path.join(folder, os.path.basename(member)), "wb") as f:
                    f.write(z.read(member))
                print(member)


if __name__ == "__main__":
    main()
