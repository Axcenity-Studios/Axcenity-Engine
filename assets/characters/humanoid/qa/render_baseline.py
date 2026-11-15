"""Renders the humanoid QA sheets: the same cameras, lights, backdrop,
resolution and Cycles settings every run, so a later "after" set compares
one to one with this "before" set.

    python -I render_baseline.py --out DIR [--package stylized|realistic]
                                 [--parts FILE] [--default FILE] [--only a,b] [--samples 32]

Sheets: full_body_turnaround_masculine (the shipped Humanoid.glb),
full_body_turnaround_feminine, face_closeups, face_topology, body_clay,
build_range, hair_styles, deformation_poses, expressions, details. Every
look other than the masculine turnaround is built from the parts file:
the default outfit with the masks the customization file lists, the
default hair, Frame_Feminine with its bone offsets for the feminine frame.

The realistic package (`--package realistic`) has no wardrobe, hair or
expressions yet, so its set is the sheets a bare body has, with the same
cameras: both turnarounds (from its parts file), face_closeups,
face_topology, body_clay, deformation_poses, and its own details,
twist_poses (the limbs turned about their length), clips (the starter
animations retargeted onto it, as the engine plays them), lods and
skin_maps.
"""

import argparse
import os
import shutil
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

import qa_scene as Q  # noqa: E402

FACE = ["Body", "Eyes", "Teeth", "Brows", "Lashes"]
FRAMES = (("masculine", 0.0), ("feminine", 1.0))
VIEWS = (("front", 0), ("three-quarter", 35), ("side", 90), ("back", 180))

# Cameras (Blender space, the character faces -Y), all 85 mm unless noted.
CAM_FULL = ((0.0, -4.72, 0.95), (0.0, 0.0, 0.92), 85)
CAM_FACE = ((0.0, -1.13, 1.62), (0.0, -0.03, 1.60), 85)
CAM_HAIR = ((0.0, -1.25, 1.64), (0.0, -0.01, 1.60), 85)
CAM_POSE = {
    "rest": ((0.0, -4.72, 0.95), (0.0, 0.0, 0.92), 85),
    "elbow_135": ((1.40, -0.90, 1.30), (0.32, -0.01, 1.19), 85),
    "knee_135": ((1.95, -0.30, 0.66), (0.09, 0.10, 0.60), 85),
    "shoulder_abd_90": ((0.35, -1.95, 1.40), (0.35, 0.0, 1.30), 85),
    "shoulder_abd_150": ((0.10, -2.05, 1.45), (0.10, 0.0, 1.38), 85),
    "hip_flex_100": ((2.6, -0.50, 0.95), (0.08, -0.32, 0.85), 85),
    "wrist_70": ((0.95, -0.95, 1.05), (0.45, 0.0, 0.92), 85),
    "spine_twist_45": ((0.0, -3.2, 1.15), (0.0, 0.0, 1.10), 85),
    "fist": ((0.95, -0.80, 1.00), (0.55, 0.0, 0.90), 85),
}
FACE_EXPRESSIONS = [
    ("neutral", {}, None),
    ("smile", {"Mouth_Smile_Left": 1, "Mouth_Smile_Right": 1}, None),
    ("frown", {"Mouth_Frown_Left": 1, "Mouth_Frown_Right": 1}, None),
    ("jaw open", {"Jaw_Open": 1}, Q.jaw_open),
    ("jaw open + smile", {"Jaw_Open": 1, "Mouth_Smile_Left": 1, "Mouth_Smile_Right": 1}, Q.jaw_open),
    ("blink", {"Blink_Left": 1, "Blink_Right": 1}, None),
    ("brow raise", {"Brow_Raise_Left": 1, "Brow_Raise_Right": 1}, None),
    ("sneer", {"Nose_Sneer_Left": 1, "Nose_Sneer_Right": 1}, None),
]
HAIRS = ["Hair_Swept", "Hair_Bob", "Hair_Long", "Hair_Crop", "Hair_Buzz", "Hair_Ponytail"]


class Run:
    def __init__(self, args):
        self.a = args
        self.tmp = tempfile.mkdtemp(prefix="humanoid_qa_")
        parts, default, custom = Q.package_paths(args.package)
        args.parts = args.parts or parts
        args.default = args.default or default
        self.custom = Q.load_custom(args.custom or custom)
        self.outfit = self.custom["default"]["outfit"]
        self.hair = self.custom["default"]["hair"]
        self.name = os.path.basename(args.parts)

    def scene(self, path, w, h, merge=False):
        Q.reset()
        # The realistic body's vertices are split along its region borders
        # as well as its texture seams, and Blender shades a split edge as
        # a crease once a shape moves it: merged, it's shaded as the engine
        # shades it (the file carries the shape's normals).
        merge = merge or self.a.package != "stylized"
        ch = Q.Character(path, merge_vertices=merge, custom=self.custom)
        cam = Q.setup_studio(w, h, self.a.samples)
        return ch, cam

    def tile(self, cam, name, cam_def):
        pos, target, lens = cam_def
        return Q.shot(cam, os.path.join(self.tmp, name + ".png"), pos, target, lens)

    def dressed(self, ch, frame=0.0, build=None, hair=None, outfit=None, expr=None, face=None):
        outfit = self.outfit if outfit is None else outfit
        hair = self.hair if hair is None else hair
        ch.show_only(FACE + list(outfit) + ([hair] if hair else []))
        ch.set_look(frame=frame, build=build, expr=expr, face=face, masks=ch.masks_for(outfit))

    def save(self, name, tiles, cols, labels, title, grey=False):
        out = os.path.join(self.a.out, name + ".png")
        Q.contact_sheet(out, tiles, cols, labels, title, grey=grey)
        print("wrote", out, os.path.getsize(out) // 1024, "KB")

    # -- sheets --------------------------------------------------------------------

    def turnaround(self, which):
        frame = 1.0 if which == "feminine" else 0.0
        if which == "masculine" and self.a.default:
            ch, cam = self.scene(self.a.default, 320, 480)
            ch.set_look(masks=ch.masks_for(self.outfit), keep_defaults=True)
            src = "Humanoid.glb (shipped default)"
        else:
            ch, cam = self.scene(self.a.parts, 320, 480)
            self.dressed(ch, frame)
            src = f"{self.name}, Frame_Feminine {frame:g}"
        tiles, labels = [], []
        for label, deg in VIEWS:
            Q.turn_character(ch, deg)
            tiles.append(self.tile(cam, f"turn_{which}_{deg}", CAM_FULL))
            labels.append(label)
        what = "Default outfit" if self.outfit else "Base body"
        self.save(f"full_body_turnaround_{which}", tiles, 4, labels, f"{what}, {which} - {src}")

    def face_closeups(self):
        ch, cam = self.scene(self.a.parts, 320, 400)
        tiles, labels = [], []
        for fname, frame in FRAMES:
            self.dressed(ch, frame)
            for label, deg in VIEWS[:2]:
                Q.turn_character(ch, deg)
                tiles.append(self.tile(cam, f"face_{fname}_{deg}", CAM_FACE))
                labels.append(f"{fname} {label}")
        self.save("face_closeups", tiles, 4, labels,
                  "Face, default hair and outfit, real materials" if self.outfit else "Face, real materials")

    def face_topology(self):
        ch, cam = self.scene(self.a.parts, 640, 640)
        Q.clay_parts(ch.parts.values(), Q.clay_kind)
        tiles, labels = [], []
        for fname, frame in FRAMES:
            ch.show_only(["Body", "Eyes"])
            ch.set_look(frame=frame)
            Q.turn_character(ch, 0)
            for label, deg in VIEWS[:2]:
                Q.turn_character(ch, deg)
                wire = Q.wire_overlay(ch.parts["Body"], 0.00035)
                tiles.append(self.tile(cam, f"topo_{fname}_{deg}", ((0.0, -0.97, 1.61), (0.0, -0.03, 1.60), 85)))
                bpy.data.objects.remove(wire)
                labels.append(f"{fname} {label}")
        self.save("face_topology", tiles, 2, labels, "Head topology (glTF triangles re-joined to quads), clay", grey=True)

    def body_clay(self):
        ch, cam = self.scene(self.a.parts, 320, 480)
        Q.clay_parts(ch.parts.values(), Q.clay_kind)
        tiles, labels = [], []
        for fname, frame in FRAMES:
            under = ["Underwear_Briefs"] + (["Underwear_Top"] if frame else [])
            under = [n for n in under if n in ch.parts]
            ch.show_only(FACE + under)
            ch.set_look(frame=frame)
            for label, deg in (VIEWS[0], VIEWS[2]):
                Q.turn_character(ch, deg)
                wires = [Q.wire_overlay(ch.parts[n], 0.0018) for n in ["Body"] + under]
                tiles.append(self.tile(cam, f"body_{fname}_{deg}", CAM_FULL))
                for w in wires:
                    bpy.data.objects.remove(w)
                labels.append(f"{fname} {label}")
        self.save("body_clay", tiles, 4, labels,
                  "Base body, underwear only, clay with wireframe" if self.outfit
                  else "Base body, clay with wireframe", grey=True)

    def build_range(self):
        ch, cam = self.scene(self.a.parts, 320, 480)
        tiles, labels = [], []
        Q.turn_character(ch, 35)
        for fname, frame in FRAMES:
            for b in (None, "Build_Heavy", "Build_Slim", "Build_Muscular"):
                self.dressed(ch, frame, build={b: 1.0} if b else None)
                tiles.append(self.tile(cam, f"build_{fname}_{b}", CAM_FULL))
                labels.append(f"{fname} {(b or 'neutral').replace('Build_', '')}")
        self.save("build_range", tiles, 4, labels, "Builds at 1, default outfit, three-quarter")

    def hair_styles(self):
        ch, cam = self.scene(self.a.parts, 426, 400)
        tiles, labels = [], []
        Q.turn_character(ch, 35)
        for h in HAIRS:
            self.dressed(ch, 0.0, hair=h)
            tiles.append(self.tile(cam, f"hair_{h}", CAM_HAIR))
            labels.append(h)
        self.save("hair_styles", tiles, 3, labels, "Hair styles on the default (masculine) head, three-quarter")

    def deformation_poses(self):
        ch, cam = self.scene(self.a.parts, 426, 426)
        Q.clay_parts(ch.parts.values(), Q.clay_kind)
        tiles, labels = [], []
        self.dressed(ch, 0.0)
        for pose in Q.POSES:
            Q.apply_pose(ch, pose)
            tiles.append(self.tile(cam, f"pose_{pose}", CAM_POSE[pose]))
            labels.append(pose)
        ch.pose_reset()
        self.save("deformation_poses", tiles, 3, labels,
                  "Test poses (left side), default outfit, clay" if self.outfit
                  else "Test poses (left side), base body, clay", grey=True)

    def expressions(self):
        ch, cam = self.scene(self.a.parts, 320, 320)
        tiles, labels = [], []
        for fname, frame in FRAMES:
            for label, expr, pose in FACE_EXPRESSIONS:
                self.dressed(ch, frame, expr=expr)
                ch.pose_reset()
                if pose:
                    pose(ch)
                    Q.update()
                tiles.append(self.tile(cam, f"expr_{fname}_{label}", ((0.0, -0.98, 1.60), (0.0, -0.03, 1.585), 85)))
                labels.append(f"{fname[:4]} {label}")
        ch.pose_reset()
        self.save("expressions", tiles, 4, labels, "Expressions at 1 (jaw open = Jaw bone 25 deg + Jaw_Open), front")

    def details(self):
        """Close-ups of defects the measurements point at."""
        if self.a.package != "stylized":
            return self.details_bare()
        tiles, labels = [], []
        ch, cam = self.scene(self.a.default, 320, 320)
        ch.set_look(masks=ch.masks_for(self.outfit), keep_defaults=True)
        tiles.append(self.tile(cam, "d_waist", ((0.55, -1.1, 1.02), (0.0, -0.02, 0.98), 85)))
        labels.append("Humanoid.glb waist")
        tiles.append(self.tile(cam, "d_shoe", ((0.45, -0.75, 0.55), (0.10, -0.06, 0.05), 85)))
        labels.append("Humanoid.glb shoe")
        ch, cam = self.scene(self.a.parts, 320, 320)
        self.dressed(ch, 0.0, outfit=["Shirt_Tee", "Pants_Cargo", "Shoes_Sneakers"])
        tiles.append(self.tile(cam, "d_tee", ((0.9, -1.6, 1.15), (0.0, 0.0, 1.05), 85)))
        labels.append("tee without jacket")
        self.dressed(ch, 0.0, hair="Hair_Long")
        Q.turn_character(ch, 90)
        tiles.append(self.tile(cam, "d_long", ((0.0, -1.3, 1.55), (0.0, -0.01, 1.52), 85)))
        labels.append("Hair_Long side")
        Q.turn_character(ch, 0)
        self.dressed(ch, 0.0, expr={"Blink_Left": 1, "Blink_Right": 1})
        tiles.append(self.tile(cam, "d_blink", ((0.10, -0.40, 1.60), (0.035, -0.08, 1.59), 85)))
        labels.append("blink, left eye")
        self.dressed(ch, 0.0, expr={"Jaw_Open": 1})
        Q.jaw_open(ch)
        Q.update()
        Q.turn_character(ch, 30)
        tiles.append(self.tile(cam, "d_jaw", ((0.0, -0.45, 1.50), (0.0, -0.03, 1.49), 85)))
        labels.append("jaw open, three-quarter")
        ch.pose_reset()
        Q.turn_character(ch, 0)
        self.dressed(ch, 0.0, build={"Build_Heavy": 1.0}, hair="Hair_Buzz")
        Q.turn_character(ch, 35)
        tiles.append(self.tile(cam, "d_buzz", ((0.0, -1.2, 1.64), (0.0, -0.01, 1.61), 85)))
        labels.append("Hair_Buzz + Build_Heavy")
        Q.turn_character(ch, 0)
        self.dressed(ch, 0.0, hair="Hair_Bob", face={"Face_Ears_Out": 1, "Face_Ears_Size": 1})
        Q.turn_character(ch, 60)
        tiles.append(self.tile(cam, "d_bob_ears", ((0.0, -1.2, 1.62), (0.0, -0.01, 1.59), 85)))
        labels.append("Hair_Bob + Ears_Out/Size 1")
        Q.turn_character(ch, 0)
        self.dressed(ch, 0.0, hair="Hair_Long")
        tiles.append(self.tile(cam, "d_long_front", ((0.0, -1.3, 1.57), (0.0, -0.01, 1.54), 85)))
        labels.append("Hair_Long front")
        self.dressed(ch, 0.0)
        tiles.append(self.tile(cam, "d_eye", ((0.10, -0.40, 1.60), (0.035, -0.08, 1.59), 85)))
        labels.append("masculine left eye")
        self.dressed(ch, 1.0)
        tiles.append(self.tile(cam, "d_eye_f", ((0.12, -0.40, 1.60), (0.047, -0.085, 1.588), 85)))
        labels.append("feminine left eye")
        self.dressed(ch, 0.0)
        Q.apply_pose(ch, "hip_flex_100")
        tiles.append(self.tile(cam, "d_hip", ((1.0, 1.3, 0.75), (0.09, 0.05, 0.80), 85)))
        labels.append("hip_flex_100 from behind")
        ch.pose_reset()
        self.save("details", tiles, 4, labels, "Details the measurements point at")


    def fit(self):
        """Clothing fit at the extremes the measurements test: the default
        outfit, the tee without the jacket and the underwear, on both
        frames with the heavy build (the hardest case), plus the shoes on
        the slim build. The body is whole here, with the garments' masks."""
        ch, cam = self.scene(self.a.parts, 320, 320)
        tiles, labels = [], []
        under = ["Underwear_Top", "Underwear_Briefs"]
        shots = [
            ("masculine heavy, outfit", 0.0, "Build_Heavy", None, 35, ((0.0, -2.5, 1.05), (0.0, 0.0, 1.0), 85)),
            ("feminine heavy, outfit", 1.0, "Build_Heavy", None, 35, ((0.0, -2.5, 1.05), (0.0, 0.0, 1.0), 85)),
            ("feminine heavy, tee side", 1.0, "Build_Heavy", ["Shirt_Tee", "Pants_Cargo", "Shoes_Sneakers"], 90,
             ((0.0, -1.5, 1.08), (0.0, 0.0, 1.02), 85)),
            ("feminine heavy, outfit back", 1.0, "Build_Heavy", None, 180, ((0.0, -1.7, 1.12), (0.0, 0.0, 1.05), 85)),
            ("masculine heavy, briefs front", 0.0, "Build_Heavy", under, 0, ((0.0, -1.2, 0.86), (0.0, 0.0, 0.84), 85)),
            ("masculine heavy, briefs back", 0.0, "Build_Heavy", under, 180, ((0.0, -1.2, 0.88), (0.0, 0.0, 0.86), 85)),
            ("masculine heavy, sports top", 0.0, "Build_Heavy", under, 35, ((0.0, -1.3, 1.30), (0.0, 0.0, 1.27), 85)),
            ("feminine heavy, briefs back", 1.0, "Build_Heavy", under, 180, ((0.0, -1.2, 0.93), (0.0, 0.0, 0.91), 85)),
            ("masculine heavy, shoes", 0.0, "Build_Heavy", None, 0, ((0.45, -0.75, 0.55), (0.10, -0.06, 0.05), 85)),
            ("masculine slim, shoes", 0.0, "Build_Slim", None, 0, ((0.45, -0.75, 0.55), (0.10, -0.06, 0.05), 85)),
            ("feminine heavy, shoes", 1.0, "Build_Heavy", None, 0, ((0.45, -0.75, 0.55), (0.10, -0.06, 0.05), 85)),
            ("feminine heavy, shoes front", 1.0, "Build_Heavy", None, 0, ((0.05, -0.85, 0.18), (0.05, -0.08, 0.06), 85)),
        ]
        for label, frame, build, outfit, deg, cam_def in shots:
            self.dressed(ch, frame, build={build: 1.0}, outfit=outfit)
            Q.turn_character(ch, deg)
            tiles.append(self.tile(cam, "fit_" + label.replace(",", "").replace(" ", "_"), cam_def))
            labels.append(label)
        Q.turn_character(ch, 0)
        self.save("fit", tiles, 4, labels, "Clothing fit at the extremes (parts file, heavy and slim builds)")


    # -- a bare body's sheets (the realistic package) ----------------------------------

    def details_bare(self):
        """Close-ups of a bare body: the eyes, the mouth at rest and open,
        the hands and feet, and the joints from the sides the pose sheet
        doesn't show."""
        ch, cam = self.scene(self.a.parts, 320, 320)
        tiles, labels = [], []

        def take(label, cam_def, frame=0.0, pose=None, deg=0, also=None):
            self.dressed(ch, frame)
            ch.pose_reset()
            if pose:
                Q.apply_pose(ch, pose)
            if also:
                also(ch)
                ch.drive()
                Q.update()
            Q.turn_character(ch, deg)
            tiles.append(self.tile(cam, "d_" + label.replace(",", "").replace(" ", "_"), cam_def))
            labels.append(label)
            Q.turn_character(ch, 0)
            ch.pose_reset()

        eye_m = ch.eye_centre("Left")
        take("masculine left eye", ((eye_m.x + 0.04, eye_m.y - 0.17, eye_m.z + 0.003), (eye_m.x, eye_m.y, eye_m.z), 85))
        ch.set_look(frame=1.0)
        eye_f = ch.eye_centre("Left")
        take("feminine left eye", ((eye_f.x + 0.04, eye_f.y - 0.17, eye_f.z + 0.003), (eye_f.x, eye_f.y, eye_f.z), 85),
             frame=1.0)
        ch.set_look()
        teeth = Q.Geo(ch.parts["Teeth"]).co
        mouth = (0.0, float(teeth[:, 1].min()) - 0.008, float(teeth[:, 2].mean()))
        take("mouth at rest", ((0.0, mouth[1] - 0.26, mouth[2]), mouth, 85))
        take("jaw open, three-quarter", ((0.0, mouth[1] - 0.30, mouth[2] - 0.01), (mouth[0], mouth[1], mouth[2] - 0.01), 85), deg=30, also=Q.jaw_open)
        take("left hand, back", ((0.95, -0.80, 1.00), (0.55, 0.0, 0.90), 85))
        take("left hand, palm", ((0.30, -0.55, 0.70), (0.58, -0.10, 0.93), 85))
        take("feet, front", ((0.0, -1.05, 0.30), (0.0, -0.02, 0.07), 85))
        take("left foot, side", ((1.05, -0.05, 0.22), (0.13, -0.02, 0.07), 85))
        take("elbow_135 from behind", ((1.25, 1.00, 1.32), (0.33, 0.0, 1.20), 85), pose="elbow_135")
        take("knee_135 from the front", ((0.75, -1.75, 0.62), (0.11, 0.05, 0.52), 85), pose="knee_135")
        take("shoulder_abd_150 from behind", ((0.10, 2.05, 1.45), (0.10, 0.0, 1.38), 85), pose="shoulder_abd_150")
        take("hip_flex_100 from behind", ((1.0, 1.3, 0.75), (0.09, 0.05, 0.80), 85), pose="hip_flex_100")
        self.save("details", tiles, 4, labels, "Details: eyes, mouth, hands, feet and the joints' other sides")

    def build_grid(self, frame):
        """The body over muscle (across) by fat (down), clay: the two
        whole-body sliders at their ends and their middle."""
        ch, cam = self.scene(self.a.parts, 300, 400)
        Q.clay_parts(ch.parts.values(), Q.clay_kind)
        ch.show_only(["Body", "Eyes", "Teeth"])
        value = 0.0 if frame == "masculine" else 1.0
        Q.turn_character(ch, 25)
        tiles, labels = [], []
        for fat in (1.0, 0.0, -1.0):
            for muscle in (-1.0, 0.0, 1.0):
                ch.set_sliders({"frame": value, "muscle": muscle, "fat": fat})
                tiles.append(self.tile(cam, f"build_{frame}_{muscle}_{fat}", ((0.0, -4.72, 0.95), (0.0, 0.0, 0.92), 85)))
                labels.append(f"muscle {muscle:+g}, fat {fat:+g}")
        Q.turn_character(ch, 0)
        self.save(f"build_grid_{frame}", tiles, 3, labels, f"Muscle by fat, {frame}, clay", grey=True)

    def build_details(self, frame):
        """The trunk close up, front and back, at the sliders' corners:
        the sheet to judge the anatomy by."""
        ch, cam = self.scene(self.a.parts, 360, 440)
        Q.clay_parts(ch.parts.values(), Q.clay_kind)
        ch.show_only(["Body", "Eyes", "Teeth"])
        value = 0.0 if frame == "masculine" else 1.0
        looks = [("neutral", {}), ("heavy and soft", {"muscle": -1.0, "fat": 1.0}),
                 ("heavy and muscular", {"muscle": 1.0, "fat": 1.0}), ("heavy", {"fat": 1.0}),
                 ("slim", {"fat": -1.0}), ("slight", {"muscle": -1.0, "fat": -1.0}),
                 ("muscular", {"muscle": 1.0}), ("lean and muscular", {"muscle": 1.0, "fat": -1.0})]
        tiles, labels = [], []
        for turn, side in ((15, "front"), (195, "back")):
            Q.turn_character(ch, turn)
            for name, values in looks:
                ch.set_sliders(dict(values, frame=value))
                tiles.append(self.tile(cam, f"detail_{frame}_{side}_{name}", ((0.0, -2.6, 1.18), (0.0, 0.0, 1.14), 85)))
                labels.append(f"{name}, {side}")
        Q.turn_character(ch, 0)
        self.save(f"build_details_{frame}", tiles, 4, labels, f"The trunk at the sliders' corners, {frame}, clay",
                  grey=True)

    def build_regions(self, frame):
        """Each region's fat and muscle slider alone, at 1 (and the
        whole-body sliders at 0), clay."""
        ch, cam = self.scene(self.a.parts, 260, 400)
        Q.clay_parts(ch.parts.values(), Q.clay_kind)
        ch.show_only(["Body", "Eyes", "Teeth"])
        value = 0.0 if frame == "masculine" else 1.0
        regions = [p["id"][len("fat_"):] for p in ch.manifest["parameters"] if p["id"].startswith("fat_")]
        Q.turn_character(ch, 25)
        tiles, labels = [], []
        for kind in ("fat", "muscle"):
            for region in regions:
                ch.set_sliders({"frame": value, f"{kind}_{region}": 1.0})
                tiles.append(self.tile(cam, f"region_{frame}_{kind}_{region}", ((0.0, -4.72, 0.95), (0.0, 0.0, 0.92), 85)))
                labels.append(f"{kind}_{region} +1")
        Q.turn_character(ch, 0)
        self.save(f"build_regions_{frame}", tiles, len(regions), labels, f"One region's slider at a time, {frame}, clay",
                  grey=True)

    def twist_poses(self):
        """The limbs turned about their length, clay with wireframe: a
        forearm, an upper arm and a thigh (measure.py's twist poses), and
        the head turned on the neck."""
        import math
        ch, cam = self.scene(self.a.parts, 426, 426)
        Q.clay_parts(ch.parts.values(), Q.clay_kind)
        self.dressed(ch, 0.0)
        tiles, labels = [], []
        shots = [
            ("forearm_twist_90", "LeftHand", "LeftLowerArm", 90, ((1.30, -0.95, 1.20), (0.45, -0.03, 1.10), 85)),
            ("forearm_twist_-90", "LeftHand", "LeftLowerArm", -90, ((1.30, -0.95, 1.20), (0.45, -0.03, 1.10), 85)),
            ("upper_arm_twist_70", "LeftUpperArm", "LeftUpperArm", 70, ((0.75, -1.45, 1.42), (0.27, 0.0, 1.28), 85)),
            ("upper_arm_twist_-70", "LeftUpperArm", "LeftUpperArm", -70, ((0.75, -1.45, 1.42), (0.27, 0.0, 1.28), 85)),
            ("thigh_twist_45", "LeftUpperLeg", "LeftUpperLeg", 45, ((0.9, -1.9, 0.80), (0.10, 0.0, 0.66), 85)),
            ("head_turn_60", "Head", None, 60, ((0.0, -1.13, 1.62), (0.0, -0.03, 1.56), 85)),
        ]
        for label, bone, along, deg, cam_def in shots:
            ch.pose_reset()
            if along:
                axis = (ch.tail(along) - ch.head(along)).normalized()
                ch.bend(bone, axis, deg)
            else:
                ch.bend("Neck", Q.UP, deg / 2.0)
                ch.bend("Head", Q.UP, deg / 2.0)
            ch.drive()
            Q.update()
            wire = Q.wire_overlay(ch.parts["Body"], 0.0007)
            tiles.append(self.tile(cam, "twist_" + label, cam_def))
            bpy.data.objects.remove(wire)
            labels.append(label)
        ch.pose_reset()
        del math
        self.save("twist_poses", tiles, 3, labels, "Limbs turned about their length, clay with wireframe", grey=True)


    def clips(self):
        """The stylized humanoid's starter animations on this body, as the
        engine plays a take on a skeleton with other proportions: each
        bone turned by the take's turn from its own rest, in the model's
        space, the hips moved by the take's travel scaled by leg length
        (`anim` `retarget.rs`), then the twist and helper bones driven.
        The last tile is the same take bound by bone name instead (bone
        offsets and all), which is what the engine does today when the
        names match and the clip has no retarget profile."""
        ch, cam = self.scene(self.a.parts, 320, 400)
        self.dressed(ch, 0.0)
        take_arm, actions = Q.load_takes(self.a.clips)
        shots = [("Idle", 0.0), ("Idle", 0.5), ("Walk", 0.0), ("Walk", 0.25), ("Walk", 0.5), ("Walk", 0.75),
                 ("Run", 0.0), ("Run", 0.25), ("Run", 0.5), ("Run", 0.75), ("Jump", 0.35), ("Jump", 0.7),
                 ("Crouch", 0.5), ("Wave", 0.5), ("Sit", 0.9)]
        tiles, labels = [], []
        Q.turn_character(ch, 35)
        full = ((0.0, -4.0, 1.0), (0.0, 0.0, 0.92), 85)
        for name, at in shots:
            Q.pose_take(ch, take_arm, actions[name], at, retarget=True)
            tiles.append(self.tile(cam, f"clip_{name}_{at}", full))
            labels.append(f"{name} {at:g}")
        Q.pose_take(ch, take_arm, actions["Walk"], 0.25, retarget=False)
        tiles.append(self.tile(cam, "clip_by_name", full))
        labels.append("Walk 0.25, bound by name")
        ch.pose_reset()
        Q.turn_character(ch, 0)
        self.save("clips", tiles, 4, labels,
                  "Starter animations (HumanoidAnimations.glb) retargeted as the engine does; last: bound by name")

    def lods(self):
        """The body and its simpler copies, clay with wireframe: whole,
        the head, and an arm bent (they share the skeleton and weights)."""
        ch, cam = self.scene(self.a.parts, 320, 400)
        Q.clay_parts(ch.parts.values(), Q.clay_kind)
        levels = [n for n in ("Body", "Body_LOD1", "Body_LOD2") if n in ch.parts]
        rows = [("whole", None, ((0.0, -4.72, 0.95), (0.0, 0.0, 0.92), 85), 0.0016),
                ("head", None, ((0.35, -0.95, 1.64), (0.0, -0.03, 1.60), 85), 0.0005),
                ("elbow_135", "elbow_135", ((1.40, -0.90, 1.30), (0.32, -0.01, 1.19), 85), 0.0007)]
        tiles, labels = [], []
        for row, pose, cam_def, thick in rows:
            for level in levels:
                ch.show_only([level, "Eyes", "Teeth"])
                ch.set_look()
                ch.pose_reset()
                if pose:
                    Q.apply_pose(ch, pose)
                wire = Q.wire_overlay(ch.parts[level], thick, quads=level == "Body")
                tiles.append(self.tile(cam, f"lod_{row}_{level}", cam_def))
                bpy.data.objects.remove(wire)
                tris = sum(len(poly.vertices) - 2 for poly in ch.parts[level].data.polygons)
                labels.append(f"{level} {tris} tris, {row}")
        ch.pose_reset()
        self.save("lods", tiles, len(levels), labels, "Levels of detail, clay with wireframe", grey=True)

    def skin_maps(self):
        """The skin's texture set as it is in the file (albedo, normal,
        occlusion, roughness) and the body regions (the map, and the body
        coloured by each triangle's region)."""
        import colorsys
        from PIL import Image
        images = Q.glb_images(self.a.parts)
        size = 640
        tiles, labels = [], []

        def add(img, label):
            path = os.path.join(self.tmp, f"map_{len(tiles)}.png")
            img.convert("RGB").resize((size, size), Image.LANCZOS).save(path)
            tiles.append(path)
            labels.append(label)

        for name, img in images.items():
            if "Albedo" in name:
                add(img, f"albedo {img.size[0]} px")
            elif "Normal" in name:
                add(img, f"normal {img.size[0]} px")
            elif "ORM" in name:
                r, g, _ = img.convert("RGB").split()
                add(r, "occlusion (red)")
                add(g, "roughness (green)")
        regions_path = os.path.join(os.path.dirname(self.a.parts), "RealisticRegions.png")
        palette = [(24, 24, 26)] + [tuple(int(255 * c) for c in colorsys.hsv_to_rgb((k * 0.381) % 1.0, 0.55, 0.9))
                                    for k in range(1, 16)]
        if os.path.exists(regions_path):
            index = Image.open(regions_path).convert("L")
            colour = Image.new("RGB", index.size)
            colour.putdata([palette[min(value, 15)] for value in index.getdata()])
            add(colour.resize((size, size), Image.NEAREST), "regions map (index as colour)")
        # The body coloured by its second texture coordinates.
        ch, cam = self.scene(self.a.parts, size, size)
        ch.show_only(["Body"])
        ch.set_look()
        body = ch.parts["Body"]
        layers = body.data.uv_layers
        if len(layers) > 1:
            attr = body.data.color_attributes.new(name="Region", type="BYTE_COLOR", domain="CORNER")
            second = layers[1].data
            for i in range(len(second)):
                rgb = palette[min(15, int(second[i].uv[0] * 16))]
                # The attribute takes linear values: the palette is display colours.
                attr.data[i].color = tuple((c / 255) ** 2.2 for c in rgb) + (1.0,)
            m = Q.flat_material("Regions", (1, 1, 1), 0.7)
            node = m.node_tree.nodes.new("ShaderNodeVertexColor")
            node.layer_name = "Region"
            m.node_tree.links.new(node.outputs["Color"], m.node_tree.nodes["Principled BSDF"].inputs["Base Color"])
            for slot in body.material_slots:
                slot.material = m
            tiles.append(self.tile(cam, "regions_front", ((0.0, -4.4, 0.92), (0.0, 0.0, 0.90), 85)))
            labels.append("regions on the body (TEXCOORD_1)")
        self.save("skin_maps", tiles, 3, labels, "Skin maps as shipped in the file, and the body regions")


SHEETS = ["turnaround_masculine", "turnaround_feminine", "face_closeups", "face_topology", "body_clay",
          "build_range", "hair_styles", "deformation_poses", "expressions", "details", "fit"]
# A package without a wardrobe, hair or expressions.
BARE_SHEETS = ["turnaround_masculine", "turnaround_feminine", "face_closeups", "face_topology", "body_clay",
               "deformation_poses", "details", "twist_poses", "clips", "lods", "skin_maps"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--package", default="stylized", choices=sorted(Q.PACKAGES))
    ap.add_argument("--parts", default="", help="the parts file (the package's, unless given)")
    ap.add_argument("--default", default="", help="the default character file (the package's, if it has one)")
    ap.add_argument("--custom", default="", help="the customization or build data file (the package's, unless given)")
    ap.add_argument("--clips", default=os.path.join(Q.ASSET_DIR, "HumanoidAnimations.glb"),
                    help="the animations file the clips sheet plays")
    ap.add_argument("--out", required=True)
    ap.add_argument("--only", default="")
    ap.add_argument("--samples", type=int, default=Q.STUDIO["samples"])
    a = ap.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
    os.makedirs(a.out, exist_ok=True)
    r = Run(a)
    only = [s for s in a.only.split(",") if s] or (SHEETS if a.package == "stylized" else BARE_SHEETS)
    for s in only:
        if s.startswith("build_"):
            kind, frame = s.rsplit("_", 1)
            getattr(r, kind)(frame)
            continue
        if s.startswith("turnaround_"):
            r.turnaround(s.split("_", 1)[1])
        else:
            getattr(r, s)()
    shutil.rmtree(r.tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
