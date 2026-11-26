#!/usr/bin/env python3
"""Writes a project's terrains for the Unity port, beside the engine dump.

    export_terrain.py <project folder> <dump folder>

In <dump folder>/terrain: terrain.json (each Terrain's place, size and
layers), <name>-heights.r16 (little-endian 16-bit rows, as heights.png),
<name>-paint.rgba8 (eight weights a sample: layers 1 to 4, then 5 to 8),
and each layer's color and normal maps with a mask map packed as Unity's
terrain reads it (red metalness, green occlusion, blue height, alpha
smoothness). Needs numpy and Pillow.
"""
import json
import shutil
import sys
from pathlib import Path

import numpy as np
from PIL import Image

CHANNELS = {"Red": 0, "Green": 1, "Blue": 2, "Alpha": 3}


def main():
    project, dump = Path(sys.argv[1]), Path(sys.argv[2])
    out = dump / "terrain"
    out.mkdir(parents=True, exist_ok=True)
    world = json.loads((project / "world.json").read_text())
    terrains = []
    packed = {}
    for node in world["workspace"]["children"]:
        if node["class"] != "Terrain":
            continue
        props = node["properties"]
        folder = project / props["DataFolder"]
        heights = np.array(Image.open(folder / "heights.png"))
        assert heights.dtype == np.uint16, heights.dtype
        heights.astype("<u2").tofile(out / f"{node['name']}-heights.r16")
        paint = np.concatenate(
            [np.array(Image.open(folder / f"paint-{name}.png").convert("RGBA")) for name in ("1-4", "5-8")],
            axis=2,
        )
        paint.astype(np.uint8).tofile(out / f"{node['name']}-paint.rgba8")
        layers = []
        for layer in node.get("children", []):
            if layer["class"] != "TerrainLayer":
                continue
            p = layer["properties"]
            stem = Path(p["ColorMap"]).stem.removesuffix("_color")
            if stem not in packed:
                shutil.copyfile(project / p["ColorMap"], out / f"{stem}_color.jpg")
                shutil.copyfile(project / p["NormalMap"], out / f"{stem}_normal.jpg")
                occlusion = np.array(Image.open(project / p["AmbientOcclusionMap"]).convert("RGBA"))
                rough = np.array(Image.open(project / p["RoughnessMap"]).convert("RGBA"))
                height = np.array(Image.open(project / p["HeightMap"]).convert("L").resize(rough.shape[1::-1]))
                mask = np.zeros(rough.shape[:2] + (4,), np.uint8)
                mask[..., 1] = occlusion[..., CHANNELS[p["AmbientOcclusionChannel"]]]
                mask[..., 2] = height
                roughness = rough[..., CHANNELS[p["RoughnessChannel"]]].astype(np.float32) / 255 * p.get("Roughness", 1.0)
                mask[..., 3] = np.clip((1 - roughness) * 255 + 0.5, 0, 255).astype(np.uint8)
                Image.fromarray(mask, "RGBA").save(out / f"{stem}_mask.png")
                packed[stem] = True
            layers.append({
                "name": layer["name"],
                "textures": stem,
                "tile_size": p["TileSize"],
                "color": p.get("Color", [1, 1, 1]),
                "normal_strength": p.get("NormalStrength", 1.0),
                "height_blend": p.get("HeightBlend", 0.0),
            })
        terrains.append({
            "name": node["name"],
            "position": props["Position"],
            "size": props["Size"],
            "samples": int(heights.shape[0]),
            "layers": layers,
        })
    (out / "terrain.json").write_text(json.dumps({"terrains": terrains}, indent=1))
    print(f"{len(terrains)} terrains, {len(packed)} layer texture sets written to {out}")


if __name__ == "__main__":
    main()
