"""Animations from Quaternius's Universal Animation Library (packs 1 and 2,
"Standard", CC0 1.0: https://quaternius.com), brought onto the Axcenity
Humanoid skeleton and keyed as the starter animations are (30 frames a
second, in place, Euler XYZ in each bone's own axes), so the engine plays
them with nothing retargeted at run time.

The library's skeleton is Unreal's mannequin (`pelvis`, `spine_01`,
`upperarm_l`, ...). Each of its bones that has a counterpart (`BONES`)
gives that bone its turn: how far the take turns the library's bone from
its rest, measured in the world, is applied to our bone from its own rest.
A collarbone takes only where the library's points, from where it points
standing at ease (`SWUNG`). The two
skeletons rest in slightly different A-poses, so the limbs are
first lined up (`ALIGNED`: our bone's direction at rest turned onto the
library's, by the shortest way), and the hands by their whole frame (the
palm as well as the fingers' direction): without that the difference
between the rests plays as part of every take. The hips move as the
library's pelvis does, scaled by how high each skeleton's hips rest. Twist
bones, the jaw and the eyes aren't keyed: the engine drives them.

`TAKES` names what is taken and what it is called here. A take with the
name of a keyed one (`animations.CLIPS`) takes its place; the keyed ones
the library has no motion for stay as they are.

Sources aren't in the repository: `build.py --mocap <UAL1.glb> <UAL2.glb>`
(the packs' `Unreal-Godot/UAL*_Standard.glb`, the files without root
motion). `SPEEDS` are the library's own root motion, measured from its
`_RM` files on 2026-11-25 (metres a second).
"""

import bpy
from mathutils import Matrix, Vector

FPS = 30

SIDES = (("Left", "l"), ("Right", "r"))
FINGERS = (("Thumb", "thumb"), ("Index", "index"), ("Middle", "middle"), ("Ring", "ring"), ("Little", "pinky"))
SEGMENTS = (("Proximal", "01"), ("Intermediate", "02"), ("Distal", "03"))


def _bones():
    out = {"Hips": "pelvis", "Spine": "spine_01", "Chest": "spine_02", "UpperChest": "spine_03",
           "Neck": "neck_01", "Head": "Head"}
    for side, s in SIDES:
        for ours, theirs in (("Shoulder", "clavicle"), ("UpperArm", "upperarm"), ("LowerArm", "lowerarm"),
                             ("Hand", "hand"), ("UpperLeg", "thigh"), ("LowerLeg", "calf"), ("Foot", "foot"),
                             ("Toes", "ball")):
            out[side + ours] = f"{theirs}_{s}"
        for finger, f in FINGERS:
            for segment, n in SEGMENTS:
                out[f"{side}{finger}{segment}"] = f"{f}_{n}_{s}"
    return out


#: Our bone -> the library's.
BONES = _bones()


def _aligned():
    """Our bone -> (the bone of ours it points at, the library's bone its
    counterpart points at): the bones lined up before a take turns them."""
    out = {}
    for side, s in SIDES:
        for ours, toward, theirs in (("UpperArm", "LowerArm", "lowerarm"), ("LowerArm", "Hand", "hand"),
                                     ("UpperLeg", "LowerLeg", "calf"), ("LowerLeg", "Foot", "foot"),
                                     ("Foot", "Toes", "ball")):
            out[side + ours] = (side + toward, f"{theirs}_{s}")
        for finger, f in FINGERS:
            for (segment, _), (after, n) in zip(SEGMENTS, SEGMENTS[1:] + (("", "04_leaf"),)):
                out[f"{side}{finger}{segment}"] = (f"{side}{finger}{after}" if after else None, f"{f}_{n}_{s}")
    return out


ALIGNED = _aligned()
#: Our bones that take only where their counterpart points (the library's
#: bone its end is at), not its turn about its own length, and from where
#: it points **standing at ease** (`NEUTRAL`, the first frame of the
#: library's idle), not from where it rests: the collarbones. The library
#: rests with its shoulders drawn back, 19 degrees behind where it stands,
#: and ours rest where a person's hang; taken from its rest, every take
#: rolled our shoulders forward.
SWUNG = {side + "Shoulder": f"upperarm_{s}" for side, s in SIDES}
NEUTRAL = (1, "Idle_Loop")

#: What the library's forward takes cover on the ground, metres a second
#: (its `_RM` files' root, over each take).
SPEEDS = {"Walk": 0.97, "Run": 5.36, "Sprint": 8.25, "CrouchWalk": 0.75, "Swim": 2.17}

# (our name, pack, the library's name, loops). The first block takes the
# place of keyed takes of the same name; the rest are new.
TAKES = (
    ("Walk", 1, "Walk_Loop", True),
    ("Run", 1, "Jog_Fwd_Loop", True),
    ("Sprint", 1, "Sprint_Loop", True),
    ("Jump", 1, "Jump_Start", False),
    ("Fall", 1, "Jump_Loop", True),
    ("Land", 1, "Jump_Land", False),
    ("Crouch", 1, "Crouch_Idle_Loop", True),
    ("CrouchWalk", 1, "Crouch_Fwd_Loop", True),
    ("PickUp", 1, "PickUp_Table", False),
    ("Push", 1, "Push_Loop", True),
    ("Interact", 1, "Interact", False),
    ("Swim", 1, "Swim_Fwd_Loop", True),
    ("Sit", 1, "Sitting_Idle_Loop", True),
    ("Dance", 1, "Dance_Loop", True),
    ("HitReact", 1, "Hit_Chest", False),
    # New. The library's idle is a wide, ready stance (a fighter's, feet
    # apart and one back): it doesn't take the keyed `Idle`'s place, which
    # is a person standing at ease, and what Character Studio shows a
    # character in.
    ("IdleReady", 1, "Idle_Loop", True),
    ("WalkFormal", 1, "Walk_Formal_Loop", True),
    ("WalkCarry", 2, "Walk_Carry_Loop", True),
    ("IdleTalking", 1, "Idle_Talking_Loop", True),
    ("IdleFoldArms", 2, "Idle_FoldArms_Loop", True),
    ("IdlePhone", 2, "Idle_TalkingPhone_Loop", True),
    ("IdleTorch", 1, "Idle_Torch_Loop", True),
    ("IdleLantern", 2, "Idle_Lantern_Loop", True),
    ("IdleRail", 2, "Idle_Rail_Loop", True),
    ("IdleRailCall", 2, "Idle_Rail_Call", False),
    ("Yes", 2, "Yes", False),
    ("No", 2, "Idle_No_Loop", True),
    ("SwimIdle", 1, "Swim_Idle_Loop", True),
    ("SitDown", 1, "Sitting_Enter", False),
    ("StandUp", 1, "Sitting_Exit", False),
    ("SitTalking", 1, "Sitting_Talking_Loop", True),
    ("GetUp", 2, "LayToIdle", False),
    ("Drive", 1, "Driving_Loop", True),
    ("Roll", 1, "Roll", False),
    ("SlideStart", 2, "Slide_Start", False),
    ("Slide", 2, "Slide_Loop", True),
    ("SlideEnd", 2, "Slide_Exit", False),
    ("ClimbUp", 2, "ClimbUp_1m", False),
    ("HighJump", 2, "NinjaJump_Start", False),
    ("HighJumpFall", 2, "NinjaJump_Idle_Loop", True),
    ("HighJumpLand", 2, "NinjaJump_Land", False),
    ("Death", 1, "Death01", False),
    ("HitHead", 1, "Hit_Head", False),
    ("Knockback", 2, "Hit_Knockback", False),
    ("Fix", 1, "Fixing_Kneeling", False),
    ("OpenChest", 2, "Chest_Open", False),
    ("Consume", 2, "Consume", False),
    ("Harvest", 2, "Farm_Harvest", False),
    ("PlantSeed", 2, "Farm_PlantSeed", False),
    ("Water", 2, "Farm_Watering", False),
    ("ChopTree", 2, "TreeChopping_Loop", True),
    ("Throw", 2, "OverhandThrow", False),
    ("PunchJab", 1, "Punch_Jab", False),
    ("PunchCross", 1, "Punch_Cross", False),
    ("Hook", 2, "Melee_Hook", False),
    ("HookRecover", 2, "Melee_Hook_Rec", False),
    ("SwordIdle", 1, "Sword_Idle", True),
    ("SwordAttack", 1, "Sword_Attack", False),
    ("SwordBlock", 2, "Sword_Block", False),
    ("SwordDash", 2, "Sword_Dash", False),
    ("SwordSlashA", 2, "Sword_Regular_A", False),
    ("SwordSlashARecover", 2, "Sword_Regular_A_Rec", False),
    ("SwordSlashB", 2, "Sword_Regular_B", False),
    ("SwordSlashBRecover", 2, "Sword_Regular_B_Rec", False),
    ("SwordSlashC", 2, "Sword_Regular_C", False),
    ("SwordCombo", 2, "Sword_Regular_Combo", False),
    ("SwordHeavyCombo", 2, "Sword_Heavy_Combo", False),
    ("ShieldIdle", 2, "Idle_Shield_Loop", True),
    ("ShieldBreak", 2, "Idle_Shield_Break", False),
    ("ShieldDash", 2, "Shield_Dash", False),
    ("ShieldBash", 2, "Shield_OneShot", False),
    ("PistolIdle", 1, "Pistol_Idle_Loop", True),
    ("PistolAim", 1, "Pistol_Aim_Neutral", False),
    ("PistolAimUp", 1, "Pistol_Aim_Up", False),
    ("PistolAimDown", 1, "Pistol_Aim_Down", False),
    ("PistolShoot", 1, "Pistol_Shoot", False),
    ("PistolReload", 1, "Pistol_Reload", False),
    ("SpellStart", 1, "Spell_Simple_Enter", False),
    ("SpellIdle", 1, "Spell_Simple_Idle_Loop", True),
    ("SpellCast", 1, "Spell_Simple_Shoot", False),
    ("SpellEnd", 1, "Spell_Simple_Exit", False),
    ("ZombieIdle", 2, "Zombie_Idle_Loop", True),
    ("ZombieWalk", 2, "Zombie_Walk_Fwd_Loop", True),
    ("ZombieScratch", 2, "Zombie_Scratch", False),
)
#: Takes the library plays lying at the ground (it swims at the height of
#: its feet): the hips keep their own height on average, as the keyed
#: swim's did, so the body stays in the character's capsule.
LEVEL = ("Swim", "SwimIdle")
#: How many frames each way the hips' correction for the feet is eased over.
EASE = 2
#: The joints that tell where the feet are: ours and the library's.
FEET = ("LeftFoot", "RightFoot", "LeftToes", "RightToes")
THEIR_FEET = ("foot_l", "foot_r", "ball_l", "ball_r")
#: Our joints that come to the ground in takes that kneel, sit or lie on it.
#: None may be lower than the library's is at our size, counted only over
#: the last `LOW` metres above the floor: lining up the feet alone left the
#: hips and the hand of the slide under it.
GROUND = ("Hips", "Spine", "UpperChest", "Head") + tuple(
    side + bone for side, _ in SIDES for bone in ("UpperArm", "LowerArm", "Hand", "LowerLeg"))
LOW = 0.12
#: The takes that also get a version whose `Root` travels, at `SPEEDS`.
ROOT_MOTION = ("Walk", "Run")


def _rotation(matrix):
    """A matrix's turn alone (no scale)."""
    return matrix.to_3x3().normalized()


def _between(a, b):
    """The shortest turn that takes direction `a` onto `b`."""
    return a.normalized().rotation_difference(b.normalized()).to_matrix()


def _frame(forward, across):
    """A right-handed frame from a direction and a second one across it."""
    x = forward.normalized()
    z = x.cross(across).normalized()
    y = z.cross(x)
    return Matrix((x, y, z)).transposed()


class Source:
    """A library file in the scene: its armature and its takes by name."""

    def __init__(self, path):
        before = set(bpy.data.objects)
        actions = set(bpy.data.actions)
        bpy.ops.import_scene.gltf(filepath=path)
        self.objects = [o for o in bpy.data.objects if o not in before]
        self.armature = next(o for o in self.objects if o.type == "ARMATURE")
        # By the library's names, and out of the way of ours: a take of
        # the library's named as one of ours ("Interact", "Roll") is
        # imported as "Interact.001", and would be taken for ours.
        self.actions = {}
        for action in [a for a in bpy.data.actions if a not in actions]:
            name = action.name
            if len(name) > 4 and name[-4] == "." and name[-3:].isdigit():
                name = name[:-4]
            action.name = "library:" + name
            self.actions[name] = action
        data = self.armature.animation_data
        if data is not None:
            for track in list(data.nla_tracks):
                data.nla_tracks.remove(track)
            data.action = None
        self.world = self.armature.matrix_world.copy()
        self.rest = {b.name: self.world @ b.matrix_local for b in self.armature.data.bones}

    def play(self, name):
        action = self.actions[name]
        data = self.armature.animation_data or self.armature.animation_data_create()
        data.action = action
        slots = getattr(action, "slots", None)
        if slots:
            data.action_slot = slots[0]
        return action

    def pose(self):
        """{bone: its matrix in the world} at the scene's frame."""
        return {pb.name: self.world @ pb.matrix for pb in self.armature.pose.bones}

    def remove(self):
        for action in self.actions.values():
            bpy.data.actions.remove(action)
        for obj in self.objects:
            data = obj.data
            bpy.data.objects.remove(obj)
            if data is not None and data.users == 0:
                (bpy.data.armatures if isinstance(data, bpy.types.Armature) else bpy.data.meshes).remove(data)


class Target:
    """Our armature at rest, and what lines it up with a library's."""

    def __init__(self, armature, source, neutral):
        self.armature = armature
        self.neutral = neutral
        bones = armature.data.bones
        self.order = [b.name for b in bones]  # parents first
        self.parent = {b.name: b.parent.name if b.parent else None for b in bones}
        self.rest = {b.name: armature.matrix_world @ b.matrix_local for b in bones}
        self.tail = {b.name: armature.matrix_world @ b.tail_local for b in bones}
        head = lambda rest, name: rest[name].to_translation()
        # What turns each of our bones, at rest, onto its counterpart.
        self.align = {}
        for ours, (toward, theirs_toward) in ALIGNED.items():
            if ours not in self.rest or BONES[ours] not in source.rest or theirs_toward not in source.rest:
                continue
            mine = (head(self.rest, toward) if toward else self.tail[ours]) - head(self.rest, ours)
            theirs = head(source.rest, theirs_toward) - head(source.rest, BONES[ours])
            if mine.length > 1e-6 and theirs.length > 1e-6:
                self.align[ours] = _between(mine, theirs)
        for side, s in SIDES:
            hand = side + "Hand"
            names = (f"middle_01_{s}", f"index_01_{s}", f"pinky_01_{s}")
            if hand not in self.rest or any(n not in source.rest for n in names):
                continue
            mine = _frame(head(self.rest, side + "MiddleProximal") - head(self.rest, hand),
                          head(self.rest, side + "IndexProximal") - head(self.rest, side + "LittleProximal"))
            wrist = head(source.rest, f"hand_{s}")
            theirs = _frame(head(source.rest, names[0]) - wrist,
                            head(source.rest, names[1]) - head(source.rest, names[2]))
            self.align[hand] = theirs @ mine.transposed()
        # How far our hips travel for the library's.
        self.hips_rest = head(self.rest, "Hips")
        self.pelvis_rest = head(source.rest, "pelvis")
        self.reach = self.hips_rest.z / self.pelvis_rest.z
        self.source_rest = {name: _rotation(m) for name, m in source.rest.items()}
        self.source_heads = {name: m.to_translation() for name, m in source.rest.items()}

    def pose(self, theirs):
        """({bone: its turn in its own axes}, the hips' move in the
        world) for the library's pose `theirs`."""
        world, local = {}, {}
        for name in self.order:
            rest = _rotation(self.rest[name])
            parent = self.parent[name]
            # Where the bone would lie with no turn of its own.
            if parent is None:
                carried = rest
            else:
                carried = world[parent] @ _rotation(self.rest[parent]).transposed() @ rest
            counterpart = BONES.get(name)
            if counterpart is None or counterpart not in theirs:
                world[name] = carried
                continue
            if name in SWUNG:
                # Only where it points: which way its end has gone from
                # where it rests, by the shortest turn.
                now = theirs[SWUNG[name]].to_translation() - theirs[counterpart].to_translation()
                turn = _between(self.neutral[name], now)
            else:
                turn = _rotation(theirs[counterpart]) @ self.source_rest[counterpart].transposed()
            wanted = turn @ self.align.get(name, Matrix.Identity(3)) @ rest
            world[name] = wanted
            local[name] = carried.transposed() @ wanted
        travel = (theirs["pelvis"].to_translation() - self.pelvis_rest) * self.reach
        at = self.joints(world, travel)
        under = max(min(theirs[BONES[n]].to_translation().z * self.reach, LOW) - at[n].z for n in GROUND)
        return local, travel, self.feet(at) - self.feet_of(theirs), under

    def joints(self, world, travel):
        """{bone: where its joint is}, with the bones turned as `world`
        and the hips moved."""
        at = {}
        for name in self.order:
            parent = self.parent[name]
            head = self.rest[name].to_translation()
            if parent is None:
                at[name] = head
            elif name == "Hips":
                at[name] = head + travel
            else:
                arm = _rotation(self.rest[parent]).transposed() @ (head - self.rest[parent].to_translation())
                at[name] = at[parent] + world[parent] @ arm
        return at

    def feet(self, at):
        """How far the lowest of our ankles and toes is from where it
        rests, with the joints `at`."""
        return min(at[n].z - self.rest[n].to_translation().z for n in FEET)

    def feet_of(self, theirs):
        """The same of the library's skeleton in its pose, at our size."""
        return min((theirs[n].to_translation().z - self.source_heads[n].z) * self.reach for n in THEIR_FEET)


def _fcurves(action):
    try:
        return list(action.fcurves)
    except AttributeError:
        return [fc for layer in action.layers for strip in layer.strips for bag in strip.channelbags
                for fc in bag.fcurves]


def _key_root(armature, frame, offset):
    root = armature.pose.bones["Root"]
    root.location = armature.data.bones["Root"].matrix_local.to_3x3().inverted() @ offset
    root.keyframe_insert("location", frame=frame)
    root.rotation_mode = "XYZ"
    root.rotation_euler = (0.0, 0.0, 0.0)
    root.keyframe_insert("rotation_euler", frame=frame)


def bake_take(armature, target, source, name, theirs, loops, speed=None):
    """Keys the library's take `theirs` on `armature` as the action
    `name`; with `speed`, the `Root` travels forward at it."""
    scene = bpy.context.scene
    action = source.play(theirs)
    first, last = (int(round(f)) for f in action.frame_range)
    frames = max(last - first, 1)
    poses = []
    for f in range(frames + 1):
        scene.frame_set(first + (0 if loops and f == frames else f))
        poses.append(target.pose(source.pose()))
    # How far our feet are from where the library's are, at worst: our
    # legs aren't the library's, so with the same turns a deep bend leaves
    # our feet above or under the ground its feet stand on.
    made_off = max(abs(off) for _, _, off, _ in poses)
    lifted = 0.0
    if name.split("_")[0] in LEVEL:
        mean = sum(travel.z for _, travel, _, _ in poses) / len(poses)
        poses = [(local, travel - Vector((0.0, 0.0, mean))) for local, travel, _, _ in poses]
    else:
        # The hips ride higher or lower by that much, eased over a few
        # frames (which foot is the lower one changes mid-stride), so the
        # lower foot is at the height the library's is.
        count = len(poses)

        def eased(values, f):
            near = range(f - EASE, f + EASE + 1)
            picked = [values[k % (count - 1)] if loops and count > 1 else values[min(max(k, 0), count - 1)]
                      for k in near]
            return sum(picked) / len(picked)

        offs = [off for _, _, off, _ in poses]
        down = [eased(offs, f) for f in range(count)]
        # Then back up wherever that, or our own proportions, would put a
        # joint that lies on the ground under it (`GROUND`).
        under = [max(0.0, under + down[f]) for f, (_, _, _, under) in enumerate(poses)]
        up = [eased(under, f) for f in range(count)]
        lifted = max(up)
        poses = [(local, travel + Vector((0.0, 0.0, up[f] - down[f])))
                 for f, (local, travel, _, _) in enumerate(poses)]
    if name in bpy.data.actions:
        bpy.data.actions.remove(bpy.data.actions[name])
    made = bpy.data.actions.new(name)
    made.use_fake_user = True
    armature.animation_data_create()
    armature.animation_data.action = made
    hips_axes = _rotation(target.rest["Hips"]).transposed()
    keyed = [b for b in target.order if b in BONES]
    before = {}
    for f, (local, travel) in enumerate(poses):
        for bone in keyed:
            pb = armature.pose.bones[bone]
            pb.rotation_mode = "XYZ"
            turn = local[bone].to_euler("XYZ", before[bone]) if bone in before else local[bone].to_euler("XYZ")
            before[bone] = turn
            pb.rotation_euler = turn
            pb.keyframe_insert("rotation_euler", frame=f)
        hips = armature.pose.bones["Hips"]
        hips.location = hips_axes @ travel
        hips.keyframe_insert("location", frame=f)
        forward = Vector((0.0, -speed * f / FPS, 0.0)) if speed else Vector()
        _key_root(armature, f, forward)
    for fc in _fcurves(made):
        for kp in fc.keyframe_points:
            kp.interpolation = "LINEAR"
    made.frame_range = (0, frames)
    return made, made_off, sum(travel.z for _, travel in poses) / len(poses), lifted


def bake(armature, paths, only=None):
    """Keys every take of `TAKES` on `armature` from the library files
    `paths` ([pack 1, pack 2]) and returns {take: what was made: its
    frames, where it is from, how far our feet come from the library's at
    worst, how high the hips ride on average and how far the take was
    lifted at most to keep its joints out of the ground}. Takes of the
    same names made before (`animations.build_actions`) are replaced."""
    scene = bpy.context.scene
    scene.render.fps = FPS
    report = {}
    neutral = None
    for pack, path in enumerate(paths, start=1):
        source = Source(path)
        if neutral is None:
            # Where the collarbones point standing at ease: the same for
            # both packs, which share a skeleton.
            assert pack == NEUTRAL[0]
            action = source.play(NEUTRAL[1])
            scene.frame_set(int(round(action.frame_range[0])))
            standing = source.pose()
            neutral = {ours: standing[toward].to_translation() - standing[BONES[ours]].to_translation()
                       for ours, toward in SWUNG.items()}
        target = Target(armature, source, neutral)
        for name, of_pack, theirs, loops in TAKES:
            if of_pack != pack or (only and name not in only):
                continue
            if theirs not in source.actions:
                raise KeyError(f"{path} has no take {theirs}")
            made, feet_off, hips, lifted = bake_take(armature, target, source, name, theirs, loops)
            report[name] = {"frames": int(made.frame_range[1]), "from": theirs, "pack": pack,
                            "feet_off_mm": round(feet_off * 1000, 1), "hips_mean_m": round(hips, 3),
                            "lifted_mm": round(lifted * 1000, 1)}
            if name in ROOT_MOTION:
                bake_take(armature, target, source, f"{name}_RootMotion", theirs, False, SPEEDS[name])
        source.remove()
    armature.animation_data.action = None
    for pb in armature.pose.bones:
        pb.location = (0, 0, 0)
        pb.rotation_euler = (0, 0, 0)
    scene.frame_set(0)
    return report


def takes():
    """Every take's name here and where it is from, for the credits."""
    return [(name, f"Universal Animation Library{' 2' if pack == 2 else ''}", theirs) for name, pack, theirs, _ in TAKES]

