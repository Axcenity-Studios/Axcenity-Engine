"""Makes `Humanoid.glb`, the default humanoid's model: what a character is
drawn as before its description is applied, and where a project has no
character library. It is the base's default body out of `HumanParts.glb`
with its face parts and the boxer briefs, on the same skeleton bone for
bone, with only the face's control shapes kept (339 shapes and 30 MB
become 30 shapes and about 2 MB), so the engine can carry it inside the
program.

    python default_model.py            (no Blender needed)
"""

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from glb import Glb  # noqa: E402

PARTS = "HumanParts.glb"
BRIEFS = os.path.join("items", "boxer-briefs", "BoxerBriefs.glb")
OUT = "Humanoid.glb"


def main(out=None):
    with open(os.path.join(HERE, "Human.build.json")) as file:
        controls = set(json.load(file)["face"]["controls"])
    model = Glb(os.path.join(HERE, PARTS))
    briefs = Glb(os.path.join(HERE, BRIEFS))
    briefs.strip_targets()
    model.strip_targets(keep=controls)
    root = model.json["scenes"][0]["nodes"][0]
    model.add_meshes(briefs, under=root)
    size = model.write(out or os.path.join(HERE, OUT))
    kept = {model.json["nodes"][i]["name"]: len((model.json["meshes"][model.json["nodes"][i]["mesh"]].get("extras") or {})
                                                 .get("targetNames", []))
            for i in model.mesh_nodes()}
    print(f"{OUT}: {size / 1e6:.2f} MB; shapes kept: {kept}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else None)
