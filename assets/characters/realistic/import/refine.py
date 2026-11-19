"""Step 3b of the intake (optional): a refinement recipe, the scripted
form of the Blender work decision 11 gives to people (refinement,
topology, fitting), for an asset whose spec names one:

    "refine": {"recipe": "joggers"}

runs `refine_<recipe>.py`'s `step` on `topology.npz` and writes
`refined.npz` (and its maps in `textures/`), which `conform` then reads.
Only the recipes in `RECIPES` run: each is a reviewed script of this
folder, never anything that came with a download.

| Recipe | Script | For |
|---|---|---|
| `joggers` | `refine_joggers.py` | BlenderKit's Grey sweat pants (CC0) to the first licensed joggers (wave 5) |

    python -I refine.py --spec <asset>.intake.json --work <folder>
"""

import argparse
import importlib
import json
import os
import sys

# Run alone with -I, this folder isn't on the path: the recipes are here.
HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

RECIPES = {"joggers": "refine_joggers"}


def step(spec, work):
    recipe = spec.get("refine", {}).get("recipe")
    if recipe not in RECIPES:
        raise SystemExit(f"no refinement recipe `{recipe}` ({', '.join(RECIPES)})")
    return importlib.import_module(RECIPES[recipe]).step(spec, work)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    print(json.dumps(step(spec, args.work), indent=1))


if __name__ == "__main__":
    main()
