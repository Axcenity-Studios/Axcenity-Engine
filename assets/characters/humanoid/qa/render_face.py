"""Renders the face sheets (Character Studio overhaul CO6): every facial
control, viseme and built-in expression, the blink close up, and the face
identity parameters' two ends, on both frames, in the QA studio
(`qa_scene.STUDIO`: Cycles, three lights, grey backdrop; never changed).

    python -I render_face.py --package realistic --out DIR [--only a,b]

(`python` has the `bpy` module.) The poses are `face_checks.py`'s: shapes
on every part that has them, the jaw and eye bones turned by the facial
profile's limits. Sheets: `face_rest`, `face_blink`, `face_controls_eyes`,
`face_controls_mouth`, `face_visemes`, `face_expressions`,
`face_identity`, `face_clay` (controls without materials, where the forms
read), `face_sidelit` (lids and lips under one low side light) and
`face_identity_sidelit_<id>` (one parameter's ends under that light;
`--params a,b` names which).
"""

import argparse
import os
import sys
import tempfile

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import face_checks as FC  # noqa: E402
import qa_scene as Q  # noqa: E402

FRAMES = (("masculine", 0.0), ("feminine", 1.0))
PARTS = ["Body", "Eyes", "Teeth", "EyeShell", "Lashes", "Brows"]


def both(name, w=1.0):
    return {f"{name}_Left": w, f"{name}_Right": w}


EYES = [
    ("rest", {}), ("blink", both("Blink")), ("blink left", {"Blink_Left": 1}), ("eye closed", both("Eye_Closed")),
    ("squint", both("Eye_Squint")), ("wide", both("Eye_Wide")), ("look up", {"Eye_Look_Up": 1}),
    ("look down", {"Eye_Look_Down": 1}), ("look left", {"Eye_Look_Left": 1}), ("look right", {"Eye_Look_Right": 1}),
    ("brow raise", both("Brow_Raise")), ("brow raise left", {"Brow_Raise_Left": 1}), ("brow lower", both("Brow_Lower")),
    ("brow inward", both("Brow_Inward")), ("cheek raise", both("Cheek_Raise")), ("nose sneer", both("Nose_Sneer")),
]
MOUTH = [
    ("rest", {}), ("smile", both("Mouth_Smile")), ("smile left", {"Mouth_Smile_Left": 1}), ("frown", both("Mouth_Frown")),
    ("mouth open", {"Mouth_Open": 1}), ("wide", {"Mouth_Wide": 1}), ("narrow", {"Mouth_Narrow": 1}),
    ("pucker", {"Mouth_Pucker": 1}), ("mouth left", {"Mouth_Left": 1}), ("mouth right", {"Mouth_Right": 1}),
    ("upper lip raise", {"Upper_Lip_Raise": 1}), ("upper lip raise left", {"Upper_Lip_Raise_Left": 1}),
    ("lower lip lower", {"Lower_Lip_Lower": 1}), ("lower lip lower right", {"Lower_Lip_Lower_Right": 1}),
    ("corner pull", both("Lip_Corner_Pull")), ("cheek puff", {"Cheek_Puff": 1}), ("jaw open", {"Jaw_Open": 1}),
    ("jaw forward", {"Jaw_Forward": 1}), ("jaw left", {"Jaw_Left": 1}), ("jaw right", {"Jaw_Right": 1}),
    ("tongue out", {"Tongue_Out": 1, "Jaw_Open": 0.5}), ("tongue up", {"Tongue_Up": 1, "Jaw_Open": 0.5}),
    ("tongue down", {"Tongue_Down": 1, "Jaw_Open": 0.5}), ("jaw open + smile", dict(both("Mouth_Smile"), Jaw_Open=1)),
]


class Run:
    def __init__(self, a):
        self.a = a
        self.tmp = tempfile.mkdtemp(prefix="face_qa_")
        parts, _, custom = Q.package_paths(a.package)
        self.parts = a.parts or parts
        self.custom_path = a.custom or custom
        self.manifest = os.path.join(os.path.dirname(self.parts), "Realistic.base.json")

    def scene(self, w, h, clay=False):
        Q.reset()
        import json
        custom = Q.load_custom(self.custom_path)
        with open(self.custom_path, encoding="utf-8") as f:
            custom["face"] = json.load(f).get("face", {}).get("face_identity", {})
        ch = Q.Character(self.parts, merge_vertices=True, custom=custom)
        cam = Q.setup_studio(w, h, self.a.samples)
        ch.show_only([p for p in PARTS if p in ch.parts])
        if clay:
            shown = [p for p in ("Body", "Eyes", "Teeth") if p in ch.parts]
            Q.clay_parts([ch.parts[p] for p in shown], Q.clay_kind)
            ch.show_only(shown)
        return ch, cam

    def eye_z(self, ch):
        return ch.eye_centre("Left").z

    def shot(self, cam, ch, name, view, deg=0):
        """`view`: 'face', 'eyes', 'eye', 'mouth' from the front, turned `deg`."""
        z = self.eye_z(ch)
        x = ch.eye_centre("Left").x
        cams = {
            "face": ((0.0, -0.98, z - 0.035), (0.0, -0.03, z - 0.05), 85),
            "eyes": ((0.0, -0.75, z), (0.0, -0.10, z), 150),
            "eye": ((x, -0.55, z), (x, -0.12, z - 0.001), 200),
            "mouth": ((0.0, -0.75, z - 0.075), (0.0, -0.12, z - 0.078), 150),
        }
        Q.turn_character(ch, deg)
        pos, target, lens = cams[view]
        if deg and view == "eye":
            # The eye is off the turning axis: aim where it went.
            import math
            a = math.radians(deg)
            c = ch.eye_centre("Left")
            at = Vector((c.x * math.cos(a) - c.y * math.sin(a), c.x * math.sin(a) + c.y * math.cos(a), c.z))
            pos, target = (at.x, at.y - 0.45, at.z), (at.x, at.y, at.z)
        path = Q.shot(cam, os.path.join(self.tmp, name + ".png"), pos, target, lens)
        Q.turn_character(ch, 0)
        return path

    def save(self, name, tiles, cols, labels, title):
        out = os.path.join(self.a.out, name + ".png")
        Q.contact_sheet(out, tiles, cols, labels, title, max_kb=1400)
        print("wrote", out, os.path.getsize(out) // 1024, "KB")

    def grid(self, name, poses, view, title, size=300, cols=8, clay=False, frames=FRAMES):
        ch, cam = self.scene(size, size, clay)
        tiles, labels = [], []
        for fname, frame in frames:
            for label, weights in poses:
                FC.pose(ch, frame, weights)
                tiles.append(self.shot(cam, ch, f"{name}_{fname}_{label}".replace(" ", "_"), view))
                labels.append(f"{fname[:4]} {label}")
        self.save(name, tiles, cols, labels, title)

    # -- sheets ------------------------------------------------------------

    def face_rest(self):
        ch, cam = self.scene(420, 420)
        tiles, labels = [], []
        for fname, frame in FRAMES:
            FC.pose(ch, frame, {})
            for view, deg in (("face", 0), ("face", 35), ("eyes", 0), ("eye", 0), ("eye", 55), ("mouth", 0)):
                tiles.append(self.shot(cam, ch, f"rest_{fname}_{view}_{deg}", view, deg))
                labels.append(f"{fname} {view} {deg or ''}")
        self.save("face_rest", tiles, 6, labels, "The face at rest: both frames, real materials")

    def face_blink(self):
        ch, cam = self.scene(420, 300)
        tiles, labels = [], []
        for fname, frame in FRAMES:
            for share in (0.0, 0.5, 1.0):
                FC.pose(ch, frame, both("Blink", share))
                for deg in (0, 55):
                    tiles.append(self.shot(cam, ch, f"blink_{fname}_{share}_{deg}", "eye", deg))
                    labels.append(f"{fname} blink {share:g}{' three-quarter' if deg else ''}")
        self.save("face_blink", tiles, 6, labels, "Blink at 0, 0.5 and 1: the left eye, front and three-quarter")

    def face_controls_eyes(self):
        self.grid("face_controls_eyes", EYES, "eyes", "Eye, brow, cheek and nose controls at 1 (both sides unless said)")

    def face_controls_mouth(self):
        self.grid("face_controls_mouth", MOUTH, "mouth",
                  "Mouth, jaw and tongue controls at 1 (tongue with the jaw half open)")

    def face_clay(self):
        poses = [EYES[1], EYES[4], EYES[10], EYES[12], EYES[13], EYES[14], EYES[15], MOUTH[1], MOUTH[3], MOUTH[4],
                 MOUTH[6], MOUTH[7], MOUTH[10], MOUTH[12], MOUTH[15], MOUTH[16]]
        self.grid("face_clay", poses, "face", "Controls at 1, clay (forms without materials)", clay=True)

    def face_sidelit(self):
        """The lids and lips under one low light from the side (the QA
        studio's lights turned down, a key added at the character's
        left): what flat light hides."""
        poses = [("rest", {}), ("blink 0.5", both("Blink", 0.5)), ("blink", both("Blink")), ("squint", both("Eye_Squint")),
                 ("smile", both("Mouth_Smile")), ("pucker", {"Mouth_Pucker": 1}), ("jaw open", {"Jaw_Open": 1}),
                 ("AI", FC.visemes()["AI"])]
        ch, cam = self.scene(420, 420)
        for o in bpy.data.objects:
            if o.type == "LIGHT":
                o.data.energy *= 0.12
        key = bpy.data.lights.new("SideKey", "AREA")
        key.energy, key.size = 60.0, 0.25
        lamp = bpy.data.objects.new("SideKey", key)
        bpy.context.scene.collection.objects.link(lamp)
        tiles, labels = [], []
        for fname, frame in FRAMES:
            z = None
            for label, weights in poses:
                FC.pose(ch, frame, weights)
                z = self.eye_z(ch)
                lamp.location = (0.9, -0.55, z + 0.12)
                lamp.rotation_euler = (Vector((0.0, -0.1, z - 0.03)) - lamp.location).to_track_quat("-Z", "Y").to_euler()
                view = "eye" if label in ("rest", "blink 0.5", "blink", "squint") else "mouth"
                tiles.append(self.shot(cam, ch, f"side_{fname}_{label}".replace(" ", "_"), view, 30 if view == "eye" else 0))
                labels.append(f"{fname[:4]} {label}")
        self.save("face_sidelit", tiles, 8, labels, "Lids and lips under one low side light")

    def face_lods(self):
        """Blink, jaw open and smile on the body and on its two simpler
        copies (each drawn with the eyes and teeth)."""
        ch, cam = self.scene(300, 300)
        poses = [("blink", both("Blink")), ("jaw open", {"Jaw_Open": 1}), ("smile", both("Mouth_Smile"))]
        tiles, labels = [], []
        for fname, frame in FRAMES:
            for level in ("Body", "Body_LOD1", "Body_LOD2"):
                if level not in ch.parts:
                    continue
                ch.show_only([level, "Eyes", "Teeth"])
                for label, weights in poses:
                    FC.pose(ch, frame, weights)
                    tiles.append(self.shot(cam, ch, f"lod_{fname}_{level}_{label}".replace(" ", "_"), "face"))
                    labels.append(f"{fname[:4]} {level} {label}")
        self.save("face_lods", tiles, 9, labels, "Blink, jaw open and smile on the body and its two simpler copies")

    def face_under_chin(self):
        """The jaw closed and open from below: nothing of the mouth's
        inside may come through the skin under the chin."""
        ch, cam = self.scene(480, 480)
        tiles, labels = [], []
        for fname, frame in FRAMES:
            for label, weights in (("rest", {}), ("jaw open", {"Jaw_Open": 1}), ("jaw open + smile", dict(both("Mouth_Smile"), Jaw_Open=1))):
                FC.pose(ch, frame, weights)
                z = self.eye_z(ch)
                for side in (0.28, -0.28):
                    path = Q.shot(cam, os.path.join(self.tmp, f"chin_{fname}_{label}_{side}.png".replace(" ", "_")),
                                  (side, -0.62, z - 0.50), (0.0, -0.08, z - 0.14), 85)
                    tiles.append(path)
                    labels.append(f"{fname[:4]} {label}")
        self.save("face_under_chin", tiles, 6, labels, "From under the chin: jaw closed and open")

    def face_visemes(self):
        self.grid("face_visemes", list(FC.visemes().items()), "mouth", "Visemes (the built-in map, jaw share 0.4)", cols=6)

    def face_expressions(self):
        self.grid("face_expressions", list(FC.expressions().items()), "face", "The 15 built-in expressions", cols=5)

    def face_identity(self):
        ch, cam = self.scene(240, 240)
        shapes = {kb.name for kb in ch.parts["Body"].data.shape_keys.key_blocks}
        tiles, labels = [], []
        for fname, frame in FRAMES:
            for pid, drivers in FC.face_parameters(self.manifest):
                for end in (-1.0, 1.0):
                    identity = {k: w for k, w in drivers(end).items() if k in shapes}
                    FC.pose(ch, frame, {}, identity)
                    tiles.append(self.shot(cam, ch, f"id_{fname}_{pid}_{end}", "face", 25 if "ear" in pid else 0))
                    labels.append(f"{fname[0]} {pid} {end:+.0f}")
        self.save("face_identity", tiles, 12, labels, "Face identity parameters at -1 and +1")


    # Where each group's ends are seen from, side-lit (R4): (a point of the
    # head to look at, as offsets from between the eyes in metres, x to the
    # character's left; the character's turn, negative showing its left;
    # the lens). The eye groups use the `eye` view.
    SIDELIT_VIEWS = {
        "nose": (((0.0, -0.03, -0.035), 0, 110), ((0.0, -0.03, -0.035), -80, 110)),
        "lips": (((0.0, -0.03, -0.065), 0, 110), ((0.0, -0.03, -0.065), -80, 110)),
        "chin": (((0.0, -0.02, -0.085), 0, 100), ((0.0, -0.02, -0.085), -80, 100)),
        "jaw": (((0.0, 0.0, -0.06), 0, 75), ((0.045, 0.03, -0.075), -60, 100)),
        "ears": (((0.075, 0.09, -0.02), -60, 120), ((0.075, 0.09, -0.02), -90, 120)),
    }

    def close_shot(self, cam, ch, name, at, deg, lens):
        """A close camera 0.45 m in front of `at` (offsets from between the
        eyes), with the character turned `deg` about its feet."""
        import math
        c = 0.5 * (ch.eye_centre("Left") + ch.eye_centre("Right"))
        p = Vector((c.x + at[0], c.y + at[1], c.z + at[2]))
        a = math.radians(deg)
        q = Vector((p.x * math.cos(a) - p.y * math.sin(a), p.x * math.sin(a) + p.y * math.cos(a), p.z))
        Q.turn_character(ch, deg)
        path = Q.shot(cam, os.path.join(self.tmp, name + ".png"), (q.x, q.y - 0.45, q.z), (q.x, q.y, q.z), lens)
        Q.turn_character(ch, 0)
        return path

    def face_identity_sidelit(self):
        """Each parameter's -1, 0 and +1 under one low side light, close
        up in two views chosen by its group, both frames (R4: one sheet a
        parameter, `--params` choosing which)."""
        import json
        with open(self.manifest, encoding="utf-8") as f:
            groups = {p["id"]: p["group"] for p in json.load(f)["parameters"]}
        wanted = [p for p in filter(None, self.a.params.split(","))]
        ch, cam = self.scene(400, 400)
        shapes = {kb.name for kb in ch.parts["Body"].data.shape_keys.key_blocks}
        for o in bpy.data.objects:
            if o.type == "LIGHT":
                o.data.energy *= 0.12
        key = bpy.data.lights.new("SideKey", "AREA")
        key.energy, key.size = 60.0, 0.25
        lamp = bpy.data.objects.new("SideKey", key)
        bpy.context.scene.collection.objects.link(lamp)
        for pid, drivers in FC.face_parameters(self.manifest):
            if wanted and pid not in wanted:
                continue
            group = groups.get(pid)
            tiles, labels = [], []
            for fname, frame in FRAMES:
                for k in range(2):
                    for end in (-1.0, 0.0, 1.0):
                        identity = {n: w for n, w in drivers(end).items() if n in shapes} if end else {}
                        FC.pose(ch, frame, {}, identity)
                        z = self.eye_z(ch)
                        lamp.location = (0.9, -0.55, z + 0.12)
                        lamp.rotation_euler = (Vector((0.0, -0.1, z - 0.03)) - lamp.location).to_track_quat("-Z", "Y").to_euler()
                        name = f"sl_{pid}_{fname}_{k}_{end}"
                        if group == "eyes" or group not in self.SIDELIT_VIEWS:
                            deg = (0, -40)[k]
                            tiles.append(self.shot(cam, ch, name, "eye", deg))
                        else:
                            at, deg, lens = self.SIDELIT_VIEWS[group][k]
                            tiles.append(self.close_shot(cam, ch, name, at, deg, lens))
                        labels.append(f"{fname[:4]} {end:+.0f} {deg}")
            self.save(f"face_identity_sidelit_{pid}", tiles, 6, labels, f"{pid} at -1, 0, +1 under one low side light")


SHEETS = ["face_rest", "face_blink", "face_sidelit", "face_lods", "face_under_chin", "face_controls_eyes", "face_controls_mouth", "face_clay", "face_visemes",
          "face_expressions", "face_identity", "face_identity_sidelit"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--package", default="realistic", choices=sorted(Q.PACKAGES))
    ap.add_argument("--parts", default="")
    ap.add_argument("--custom", default="")
    ap.add_argument("--out", default=".")
    ap.add_argument("--only", default="")
    ap.add_argument("--samples", type=int, default=None)
    # `face_identity_sidelit`: which parameters (ids, `,`-separated; all
    # when empty).
    ap.add_argument("--params", default="")
    a = ap.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
    os.makedirs(a.out, exist_ok=True)
    run = Run(a)
    only = set(filter(None, a.only.split(",")))
    for name in SHEETS:
        if not only or name in only:
            getattr(run, name)()


if __name__ == "__main__":
    main()
