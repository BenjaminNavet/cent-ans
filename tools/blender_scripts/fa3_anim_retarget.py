"""Lot FA3: CC0 combat animations retargeted onto the fine battle figures.

Run from the repository root (source GLB files in ``$CENT_ANS_FA_ANIM_SRC``, default
``~/dev/cent-ans-raw/fa/anim``, outside the repository)::

    blender -b --factory-startup --python tools/blender_scripts/fa3_anim_retarget.py
    blender -b --factory-startup --python tools/blender_scripts/fa3_anim_retarget.py -- render DIR
    uv run --project tools python tools/blender_scripts/fa3_anim_board.py DIR BOARD_DIR

Sources and clip table: ``data/fx/fa3_anim_sources.json`` (Mesh2Motion and KayKit, CC0). Only
the baked skinning frames leave this script (never the source files).

Retargeting reuses the NT12 chain (``nt12_mocap_trial``: target rest data, rotation formula,
basis conversion, ``CAB1`` bake). A glTF source bone has a rest matrix: its world rotation
since rest ``S = G(t) · R⁻¹`` drives the mapped target bone, ``target = P · S · P⁻¹ · A · T``
(``P`` source world -> armature space, ``A`` rest limb alignment, ``T`` target rest rotation).
On top of NT12:

- the hips move by the source pelvis displacement since rest, scaled by the ratio of hip
  heights, and each ankle is pinned by two-bone IK onto the source ankle displacement, scaled
  alike: a foot planted in the source stays planted on the figure;
- loops lose the root drift, drop the closing key that repeats the first frame and are
  blended only when the source loop does not close;
- options of the table: shield arm kept from a guard (``left_arm``), no joint under the
  ground (``ground``), weapon aimed forwards and left hand on the shaft (``prop``), bow held
  upright with the string drawn by the right hand and the aim raised (``bow``).

Output: ``game/assets/models/battle_fine/fa3_anim/`` — ``human.bones.bin`` and
``manifest.json`` (clip table, source and licence of each clip, measures, the same measures
on the clip it replaces). ``BattleSkinned`` substitutes the clips flagged ``default`` in the
table, all of them with ``--fa-anim`` after ``--``, none with ``--no-fa-anim``.
"""

import json
import math
import os
import struct
import sys
import zlib

import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_fine as bf  # noqa: E402
import battle_skinned as bs  # noqa: E402
import nt12_mocap_trial as nt12  # noqa: E402
import video_mocap_clean as vc  # noqa: E402

RAW_DIR = os.environ.get(
    "CENT_ANS_FA_ANIM_SRC", os.path.expanduser("~/dev/cent-ans-raw/fa/anim")
)
TABLE = os.environ.get(  # another table: trials of candidate clips (renders only)
    "CENT_ANS_FA3_TABLE", os.path.join(bs.ROOT, "data", "fx", "fa3_anim_sources.json")
)
OUT_DIR = os.path.join(bf.FINE_DIR, "fa3_anim")
MELEE_DIR = os.path.join(bf.FINE_DIR, "melee")  # NT14 default melee clips
ORDER = list(nt12.MAP.keys())
PARENT = {b: spec[2] for b, spec in nt12.MAP.items()}
LEGS = ("UpperLeg", "LowerLeg", "Foot")
UPPER_BODY = [b for b in ORDER if b not in ("Body", "Hips") and not b.startswith(LEGS)]
CHEST_UP = [b for b in UPPER_BODY if b not in ("Abdomen", "Torso")]
PROP = "_prop"  # key of the weapon's world matrix in posed frames
DUPLICATE_M = 0.005  # a closing frame this close to the first one repeats it
SHIELD_CENTRE = 0.77  # shield centre along the forearm (as NT13)
FACE_ABOVE_M = 0.10  # face centre above the head joint
FACE_NEAR_M = 0.22  # shield centre this close to the line of sight hides the face
BOW_CANT_DEG = -10.0  # as the keyframed archer
SHIELD_FAMILIES = ("melee", "cheer")
FIGURES = {
    "melee": "infantry_0",
    "cheer": "infantry_0",
    "polearm": "infantry_1",
    "bow": "archer_0",
}


def load_table():
    """Return the clip source table (``data/fx/fa3_anim_sources.json``)."""
    with open(TABLE, encoding="utf-8") as f:
        return json.load(f)


# --- Sources ------------------------------------------------------------------------------


class Source:
    """Rest data of one source file and the sampled frames of its clip parts."""

    def __init__(self, bones, root, scale_by="hips"):
        """`bones`: target bone -> (source bone, source child or None); `root`: root bone.

        `scale_by`: joint whose height above the ankles scales the displacements (``hips``
        for a human build, ``head`` for a stylised one with short legs).
        """
        self.bones = bones
        self.root = root
        self.scale_by = scale_by
        self.rest = {}
        self.parts = {}

    def measure(self):
        """Axes, hip height and joint positions at rest (once `rest` is filled)."""
        head = {b: m.to_translation() for b, m in self.rest.items()}
        self.head = head
        self.up = Vector((0.0, 0.0, 1.0))
        self.left = Vector((1.0, 0.0, 0.0))
        self.frame = nt12.frame_of(self.left, self.up)
        hips = head[self.bones["UpperLeg.L"][0]] - head[self.bones["UpperLeg.R"][0]]
        if hips.x <= 0.0:
            raise ValueError("source skeleton: left and right are swapped")
        ankle = 0.5 * (
            head[self.bones["Foot.L"][0]].z + head[self.bones["Foot.R"][0]].z
        )
        self.hip_height = head[self.bones["Body"][0]].z - ankle
        self.head_height = head[self.bones["Head"][0]].z - ankle

    def scale(self, tgt):
        """Factor from source displacements to armature units of the target."""
        if self.scale_by == "head":
            feet = 0.5 * (tgt.head["Foot.L"] + tgt.head["Foot.R"])
            return (tgt.head["Head"] - feet).dot(tgt.up) / self.head_height
        return tgt.body_height / self.hip_height

    def direction(self, bone):
        """Rest direction of the source limb mapped to target `bone` (None if unknown)."""
        sbone, child = self.bones[bone]
        if child is None:
            return None
        return (self.head[child] - self.head[sbone]).normalized()


def part_key(part):
    """Hashable identity of a clip part of the table."""
    return (
        part["source"],
        part["action"],
        part.get("first", 0),
        part.get("last", -1),
        part.get("frames"),
    )


def read_sources(table):
    """Import every source file used by the table and sample the parts (24 fps).

    Must run before the target figure is loaded (each import starts from an empty scene).
    Returns ``{source name: Source}``.
    """
    wanted = {}
    for clip in table["clips"]:
        for part in clip["parts"]:
            wanted.setdefault(part["source"], []).append(part)
    out = {}
    for name, parts in wanted.items():
        spec = table["sources"][name]
        skeleton = table["skeletons"][spec["skeleton"]]
        src = Source(
            {b: tuple(pair) for b, pair in skeleton["bones"].items()},
            skeleton.get("root"),
            skeleton.get("scale_by", "hips"),
        )
        bs.reset_scene()
        bpy.ops.import_scene.gltf(filepath=os.path.join(RAW_DIR, spec["file"]))
        arm = max(
            (o for o in bpy.data.objects if o.type == "ARMATURE"),
            key=lambda o: len(o.data.bones),
        )
        names = {n for pair in src.bones.values() for n in pair if n}
        if src.root:
            names.add(src.root)
        world = arm.matrix_world.copy()
        src.rest = {n: world @ arm.data.bones[n].matrix_local for n in names}
        src.measure()
        actions = {a.name: a for a in bpy.data.actions}
        for part in parts:
            key = part_key(part)
            if key in src.parts:
                continue
            act = actions[part["action"]]
            bs.set_action(arm, act)
            f0, f1 = (int(round(v)) for v in act.frame_range)
            count = f1 - f0 + 1
            first = part.get("first", 0)
            last = part.get("last", -1)
            last = count + last if last < 0 else min(last, count - 1)
            n = part.get("frames") or (last - first + 1)
            frames = []
            for t in np.linspace(first, last, n):
                # Some actions leave channels unkeyed: nothing may leak from the last one.
                for pb in arm.pose.bones:
                    pb.matrix_basis.identity()
                whole = int(math.floor(t))
                bpy.context.scene.frame_set(f0 + whole, subframe=float(t - whole))
                bpy.context.view_layer.update()
                frames.append({b: world @ arm.pose.bones[b].matrix for b in names})
            src.parts[key] = frames
            print(f"SOURCE {name}/{part['action']} {first}-{last} -> {n} frames")
        out[name] = src
    return out


# --- Retargeting --------------------------------------------------------------------------


def target(arm):
    """NT12 target data of `arm` with the true axes of the figure.

    ``nt12.Target`` reads its axes from the rest joints (head over the hips, left hip to
    right hip), and the fine rest pose stands in a staggered stance: that frame is turned by
    27 degrees and tilted by 5. Hand-made clips are authored against the world axes, which the
    sources share with the figure after import (left +X, forwards -Y, up +Z).
    """
    tgt = nt12.Target(arm)
    to_arm = arm.matrix_world.to_3x3().normalized().inverted()
    tgt.left = to_arm @ Vector((1.0, 0.0, 0.0))
    tgt.up = to_arm @ Vector((0.0, 0.0, 1.0))
    tgt.frame = nt12.frame_of(tgt.left, tgt.up)
    feet = (tgt.head["Foot.L"] + tgt.head["Foot.R"]) * 0.5
    tgt.foot_rest = min(tgt.head[f].dot(tgt.up) for f in ("Foot.L", "Foot.R"))
    tgt.body_height = (tgt.head["Body"] - feet).dot(tgt.up)
    return tgt


def flat(tgt, v):
    """Horizontal part of an armature-space vector."""
    return v - tgt.up * v.dot(tgt.up)


def two_axes(a, b):
    """Orthonormal frame whose first axis is `a`, the second as close to `b` as possible."""
    a = a.normalized()
    b = (b - a * b.dot(a)).normalized()
    return Matrix((a, b, a.cross(b))).transposed()


def squaring(tgt, line):
    """Rotation about the vertical taking the horizontal `line` onto the figure's left."""
    return flat(tgt, line).normalized().rotation_difference(tgt.left).to_matrix()


def alignments(tgt, src, p):
    """``A`` of every mapped bone: the figure's rest pose brought onto the source's.

    NT12 aligns each limb by the smallest rotation. The fine rest pose is a contrapposto
    (hips turned by 27 degrees, one foot back and turned out, knees bent), so twists matter
    here: the hips and the chest are squared on the hip and shoulder lines, each leg bone is
    aligned with its knee hinge across the body, each foot is turned to point forwards.
    """
    out = {}
    left = tgt.left
    hips = squaring(tgt, tgt.head["UpperLeg.L"] - tgt.head["UpperLeg.R"])
    chest = squaring(tgt, tgt.head["UpperArm.L"] - tgt.head["UpperArm.R"])
    for bone in ORDER:
        limb = tgt.limb(bone)
        want = src.direction(bone)
        want = None if want is None else (p @ want).normalized()
        base = bone.split(".")[0]
        if bone in ("Body", "Hips"):
            out[bone] = hips
        elif base == "Foot":
            out[bone] = squaring(tgt, tgt.rest[bone].to_3x3().col[0])
        elif base in ("UpperLeg", "LowerLeg") and want is not None:
            side = bone[-1]
            thigh = tgt.limb(f"UpperLeg.{side}")
            shin = tgt.limb(f"LowerLeg.{side}")
            hinge = thigh.cross(shin)
            if hinge.length > math.sin(math.radians(8.0)):
                out[bone] = two_axes(want, left) @ two_axes(limb, hinge).transposed()
            else:
                out[bone] = limb.rotation_difference(want).to_matrix()
        elif limb is None or want is None:
            out[bone] = out.get(PARENT[bone], Matrix.Identity(3))
        elif bone == "Chest":
            out[bone] = (chest @ limb).rotation_difference(want).to_matrix() @ chest
        else:
            out[bone] = limb.rotation_difference(want).to_matrix()
    return out


def pin_foot(tgt, rot, pos, side, target):
    """Two-bone IK of one leg so that the ankle reaches `target` (armature space)."""
    upper, lower, foot = f"UpperLeg.{side}", f"LowerLeg.{side}", f"Foot.{side}"
    a, b, c = pos[upper], pos[lower], pos[foot]
    # A straight leg has no bend plane: the knee goes forwards.
    line = (c - a).normalized()
    off = (b - a) - line * (b - a).dot(line)
    if off.length < 0.01 / tgt.unit:
        delta = rot[upper] @ tgt.rest[upper].to_3x3().transposed()
        b = b + (delta @ tgt.frame.col[1]) * (0.02 / tgt.unit)
    b2, c2 = (Vector(v) for v in vc.two_bone_ik(a, b, c, target))
    rot[upper] = (pos[lower] - a).rotation_difference(b2 - a).to_matrix() @ rot[upper]
    rot[lower] = (c - pos[lower]).rotation_difference(c2 - b2).to_matrix() @ rot[lower]
    pos[lower] = b2
    pos[foot] = c2


def place(tgt, src, sbone, at, p, scale, bone):
    """Armature-space position of target `bone` for its source joint at `at` (source space).

    Both figures stand on their origin: the ground position is the source's, scaled; the
    height is the target joint's rest height plus the scaled rise of the source joint. (The
    rest pose of the figure is a staggered stance: displacements from rest would carry it.)
    """
    ground = Vector((at.x, at.y, 0.0))
    height = tgt.head[bone].dot(tgt.up) + (at.z - src.head[sbone].z) * scale
    return (p @ ground) * scale + tgt.up * height


def solve_frame(tgt, src, frame, p, align, scale, shift):
    """Armature-space pose matrix of every mapped bone for one source frame.

    `shift`: source-space offset removed from the pelvis and the ankles (root motion, loop
    drift). Returns (matrices, largest miss of an ankle target in armature units).
    """
    pinv = p.transposed()
    rot = {}
    pos = {}
    for bone in ORDER:
        sbone = src.bones[bone][0]
        s = (
            frame[sbone].to_3x3().normalized()
            @ src.rest[sbone].to_3x3().normalized().transposed()
        )
        rot[bone] = p @ s @ pinv @ align[bone] @ tgt.rest[bone].to_3x3()
        parent = PARENT[bone]
        if parent is None:
            pos[bone] = place(
                tgt, src, sbone, frame[sbone].to_translation() - shift, p, scale, bone
            )
        else:
            offset = tgt.rest[parent].to_3x3().transposed() @ (
                tgt.head[bone] - tgt.head[parent]
            )
            pos[bone] = pos[parent] + rot[parent] @ offset
    miss = 0.0
    for side in ("L", "R"):
        foot = f"Foot.{side}"
        sfoot = src.bones[foot][0]
        at = frame[sfoot].to_translation() - shift
        target = place(tgt, src, sfoot, at, p, scale, foot)
        pin_foot(tgt, rot, pos, side, target)
        miss = max(miss, (pos[foot] - target).length)
    return {b: Matrix.Translation(pos[b]) @ rot[b].to_4x4() for b in ORDER}, miss


def planted_slide(points, rest_height, near=0.03):
    """Largest ground travel (metres) of a Z-up ankle track while it is planted.

    Planted: within `near` of its rest height, the criterion of ``nt12.quality``, applied
    here to the source itself.
    """
    slide = 0.0
    planted = None
    for point in points:
        if point.z - rest_height < near:
            flat = Vector((point.x, point.y, 0.0))
            if planted is None:
                planted = flat
            slide = max(slide, (flat - planted).length)
        else:
            planted = None
    return slide


def rotate_about(mats, bones, pivot, rotation):
    """Rotate the matrices of `bones` rigidly about `pivot` (armature space)."""
    m = (
        Matrix.Translation(pivot)
        @ rotation.to_4x4()
        @ Matrix.Translation(-Vector(pivot))
    )
    for b in bones:
        mats[b] = m @ mats[b]


def smoothstep(a, b, x):
    """Hermite step from 0 at `a` to 1 at `b`."""
    t = min(max((x - a) / (b - a), 0.0), 1.0)
    return t * t * (3.0 - 2.0 * t)


def pose_gap(tgt, a, b):
    """Largest joint distance between two solved frames (metres)."""
    return (
        max((a[k].to_translation() - b[k].to_translation()).length for k in ORDER)
        * tgt.unit
    )


def prop_axis(tgt, mats):
    """Armature-space axis of the right-hand prop carried by the wrist of a solved frame."""
    import battle_skinned_poses as poses

    world = tgt.arm.matrix_world
    wrist = world @ mats["Wrist.R"]
    prop = wrist @ poses.REST["Wrist.R"].inverted() @ poses.REST["prop"]
    axis = world.to_3x3().inverted() @ (prop.to_3x3() @ Vector((0.0, 1.0, 0.0)))
    return axis.normalized()


def guard_arm(solved):
    """Left arm of a solved frame relative to its chest (``nt12.SHIELD_ARM`` bones)."""
    chest = solved["Chest"].inverted()
    return {b: chest @ solved[b] for b in nt12.SHIELD_ARM}


class Clip:
    """One retargeted clip: solved frames, bases and measures."""

    def __init__(self, tgt, sources, table, spec, guard=None):
        """Retarget the clip `spec` of the table (`guard`: left arm of the FA3 guard)."""
        self.spec = spec
        self.name = spec["clip"]
        self.loop = spec["loop"]
        frames = []
        src = None
        for part in spec["parts"]:
            part_src = sources[part["source"]]
            if src is not None and part_src.bones != src.bones:
                raise ValueError(f"{self.name}: parts with different skeletons")
            src = part_src
            frames += src.parts[part_key(part)]
        p = tgt.frame @ src.frame.transposed()
        align = alignments(tgt, src, p)
        scale = src.scale(tgt)
        pelvis = src.bones["Body"][0]
        keep = float(spec.get("root_motion", 1.0))
        shifts = []
        for f in frames:
            shift = Vector((0.0, 0.0, 0.0))
            if src.root and keep < 1.0:
                moved = f[src.root].to_translation() - src.head[src.root]
                shift = moved * (1.0 - keep)
                shift.z = 0.0
            shifts.append(shift)
        if self.loop and len(frames) > 2:
            # No drift over a loop: the pelvis ends where it started.
            drift = (
                frames[-1][pelvis].to_translation()
                - shifts[-1]
                - frames[0][pelvis].to_translation()
                + shifts[0]
            )
            drift.z = 0.0
            for k in range(len(frames)):
                shifts[k] = shifts[k] + drift * (k / (len(frames) - 1))
        # Feet hovering over the whole clip (motion capture offsets) come down to the ground.
        ankles = [src.bones[f"Foot.{side}"][0] for side in ("L", "R")]
        hover = min(
            f[a].to_translation().z - src.head[a].z for f in frames for a in ankles
        )
        if hover > 0.0:
            shifts = [shift + Vector((0.0, 0.0, hover)) for shift in shifts]
        self.source_slide = max(
            planted_slide(
                [
                    (f[a].to_translation() - shift) * scale * tgt.unit
                    for f, shift in zip(frames, shifts, strict=True)
                ],
                src.head[a].z * scale * tgt.unit,
            )
            for a in ankles
        )
        solved = []
        self.miss = 0.0
        for f, shift in zip(frames, shifts, strict=True):
            mats, miss = solve_frame(tgt, src, f, p, align, scale, shift)
            solved.append(mats)
            self.miss = max(self.miss, miss)
        rel = None
        if spec.get("left_arm") == "keyframed_guard":
            rel = tgt.shield_rel
        elif spec.get("left_arm") == "clip_guard":
            rel = guard
        if rel is not None:
            for s in solved:
                for b in nt12.SHIELD_ARM:
                    s[b] = s["Chest"] @ rel[b]
        bow = spec.get("bow")
        if bow and bow.get("elevation_deg"):
            self._raise_aim(tgt, solved, bow)
        self.yaw = 0.0
        aim = spec.get("prop", {}).get("aim")
        if aim:
            self._aim_forward(tgt, solved, aim)
        if spec.get("ground"):
            self._keep_above_ground(tgt, solved)
        self.source_gap = None
        if self.loop and len(solved) > 2:
            self.source_gap = pose_gap(tgt, solved[-1], solved[0])
            if self.source_gap < DUPLICATE_M:
                solved.pop()  # the closing key repeats the first frame
        bases = [nt12.to_basis(tgt, s) for s in solved]
        blend = int(table.get("loop_blend_frames", 6))
        self.blended = False
        if (
            self.loop
            and self.source_gap is not None
            and self.source_gap >= DUPLICATE_M
            and len(bases) > blend + 1
        ):
            n = len(bases)
            for j in range(blend):
                k = n - blend + j
                bases[k] = nt12.blend_basis(bases[k], bases[0], (j + 1) / (blend + 1))
            self.blended = True
        back = int(spec.get("return_frames", 0))
        last = bases[-1]
        for j in range(back):
            w = smoothstep(0.0, 1.0, (j + 1) / back)
            bases.append(nt12.blend_basis(last, bases[0], w))
        self.solved = solved
        self.bases = bases

    def _raise_aim(self, tgt, solved, bow):
        """Raise the aim so that the arrow leaves at the game's elevation.

        The upper body leans back by the elevation (60 % at the waist, 40 % at the chest);
        what the bow arm still lacks at full draw (a source aiming below the horizontal) is
        made up by turning both arms about their shoulders.
        """
        angle = math.radians(float(bow["elevation_deg"]))
        left = tgt.frame.col[0]
        a, b = bow["draw_from"], bow["release"]

        def weight(i):
            return smoothstep(a - 8, a + 8, i) * (1.0 - smoothstep(b + 4, b + 12, i))

        for i, s in enumerate(solved):
            w = weight(i)
            if w <= 0.0:
                continue
            for bones, pivot, share in (
                (UPPER_BODY, "Abdomen", 0.6),
                (CHEST_UP, "Chest", 0.4),
            ):
                rot = Matrix.Rotation(-angle * share * w, 3, left)
                rotate_about(s, bones, s[pivot].to_translation(), rot)
        full = solved[min(b - 1, len(solved) - 1)]
        aim = full["Wrist.L"].to_translation() - full["UpperArm.L"].to_translation()
        lacking = angle - math.asin(aim.normalized().dot(tgt.up))
        for i, s in enumerate(solved):
            w = weight(i)
            if w <= 0.0:
                continue
            rot = Matrix.Rotation(-lacking * w, 3, left)
            for side in ("L", "R"):
                arm = [f"{b}.{side}" for b in ("UpperArm", "LowerArm", "Wrist")]
                rotate_about(s, arm, s[arm[0]].to_translation(), rot)

    def _aim_forward(self, tgt, solved, aim):
        """Turn the figure about the vertical so that its weapon points forwards at first.

        `aim`: ``weapon`` (the prop carried by the right wrist) or ``hands`` (the line from
        the right fist to the left one: a shaft held across the body by both hands).
        """
        if aim == "hands":
            first = solved[0]
            axis = first["Wrist.L"].to_translation() - first["Wrist.R"].to_translation()
        else:
            axis = prop_axis(tgt, solved[0])
        up, fwd, left = tgt.up, tgt.frame.col[1], tgt.frame.col[0]
        self.yaw = math.atan2(axis.dot(left), axis.dot(fwd))
        rot = Matrix.Rotation(self.yaw, 3, up)
        if (rot @ axis).dot(fwd) < (rot.transposed() @ axis).dot(fwd):
            rot = rot.transposed()
        pivot = 0.5 * (tgt.head["Foot.L"] + tgt.head["Foot.R"])
        for s in solved:
            rotate_about(s, ORDER, pivot, rot)

    def _keep_above_ground(self, tgt, solved):
        """Lift the frames whose joints sink under the ground (falls), smoothly."""
        margin = nt12.GROUND_MARGIN / tgt.unit
        lifts = []
        for s in solved:
            lift = 0.0
            for bone in ORDER:
                low = tgt.foot_rest + (0.0 if bone.startswith("Foot") else margin)
                lift = max(lift, low - s[bone].to_translation().dot(tgt.up))
            lifts.append(lift)
        n = len(lifts)
        peak = [max(lifts[max(i - 2, 0) : min(i + 3, n)]) for i in range(n)]
        for i, s in enumerate(solved):
            window = peak[max(i - 2, 0) : min(i + 3, n)]
            lift = sum(window) / len(window)
            if lift > 0.0:
                up = Matrix.Translation(tgt.up * lift)
                for b in ORDER:
                    s[b] = up @ s[b]

    def prepare(self, arm):
        """Hook posing what the solved bones do not carry (prop, left hand, bow, string)."""
        import battle_skinned_poses as poses

        prop_opts = self.spec.get("prop", {})
        bow = self.spec.get("bow")
        if not prop_opts.get("left_hand") and "axis" not in prop_opts and not bow:
            return None

        def hook(i):
            if "axis" in prop_opts or prop_opts.get("left_hand"):
                prop = poses._held_prop(arm)
                if "axis" in prop_opts:
                    # The weapon keeps one direction (a pike in a formation): only the
                    # right fist of the source carries it.
                    prop = poses.prop_matrix(
                        prop.translation,
                        Vector(prop_opts["axis"]),
                        Vector((0.0, 0.0, 1.0)),
                    )
                    poses.fist_on_prop(arm, prop)
                poses.STATE["prop"] = prop
            if prop_opts.get("left_hand"):
                axis = (prop.to_3x3() @ Vector((0.0, 1.0, 0.0))).normalized()
                along = (poses.pos(arm, "Wrist.L") - prop.translation).dot(axis)
                along = min(max(along, 0.3), 0.8)
                pole = poses.pos(arm, "UpperArm.L") + Vector((0.5, 0.1, -0.4))
                poses.hand_to_prop(arm, "L", prop, along, pole)
            if bow:
                fore = poses.pos(arm, "Wrist.L") - poses.pos(arm, "LowerArm.L")
                fore.normalize()
                poses.bow_upright(arm, fore)
                poses.rotate_about(arm, "Wrist.L", fore, math.radians(BOW_CANT_DEG))
                if bow["draw_from"] <= i < bow["release"]:
                    poses.STATE["nock"] = poses._held_prop(arm).translation.copy()
                poses.STATE["arrow"] = 2 <= i < bow["release"]

        return hook


# --- Measures -----------------------------------------------------------------------------


def shield_face(tgt, solved):
    """Frames where the shield hides the face, and its closest pass to the line of sight (m)."""
    fwd_rest = tgt.frame.col[1]
    count = 0
    nearest = None
    for s in solved:
        head = s["Head"]
        face = head.to_translation() + tgt.up * (FACE_ABOVE_M / tgt.unit)
        look = (
            head.to_3x3().normalized() @ tgt.rest["Head"].to_3x3().transposed()
        ) @ fwd_rest
        elbow = s["LowerArm.L"].to_translation()
        centre = elbow.lerp(s["Wrist.L"].to_translation(), SHIELD_CENTRE)
        d = centre - face
        ahead = d.dot(look)
        if ahead <= 0.05 / tgt.unit:
            continue
        side = (d - look * ahead).length * tgt.unit
        nearest = side if nearest is None else min(nearest, side)
        if side < FACE_NEAR_M:
            count += 1
    return count, nearest


def measure(tgt, solved, loop, shield=True):
    """Measures of a solved clip, in centimetres and degrees (`shield`: one is carried)."""
    q = nt12.quality(tgt, solved)
    face_frames, face_near = shield_face(tgt, solved) if shield else (None, None)
    jerk = []
    for a, b, c in zip(solved, solved[1:], solved[2:], strict=False):
        for bone in ORDER:
            qa, qb, qc = (m[bone].to_quaternion() for m in (a, b, c))
            jerk.append(
                abs(qb.rotation_difference(qc).angle - qa.rotation_difference(qb).angle)
            )
    steps = [pose_gap(tgt, a, b) for a, b in zip(solved, solved[1:], strict=False)]
    gap = round(pose_gap(tgt, solved[-1], solved[0]) * 100.0, 1)
    return {
        "frames": len(solved),
        "foot_slide_cm": round(q["foot_slide_m"] * 100.0, 1),
        "foot_below_rest_cm": round(q["foot_below_rest_m"] * 100.0, 1),
        "wrist_r_max_deg_per_frame": q["wrist_r_max_deg_per_frame"],
        "jitter_deg_per_frame2": round(
            math.degrees(float(np.mean(jerk))) if jerk else 0.0, 2
        ),
        "largest_step_cm": round(max(steps) * 100.0, 1) if steps else 0.0,
        "loop_gap_cm": gap if loop else None,
        "end_to_start_cm": gap,
        "shield_face_frames": face_frames,
        "shield_nearest_to_sight_cm": None
        if face_near is None
        else round(face_near * 100.0, 1),
    }


def prop_measures(tgt, solved):
    """Direction ranges (degrees) and forward travel (cm) of the weapon (``Prop`` bone)."""
    yaws, elevations, reach = [], [], []
    to_arm = tgt.arm.matrix_world.inverted()
    up, fwd, left = tgt.up, tgt.frame.col[1], tgt.frame.col[0]
    for s in solved:
        prop = to_arm @ s[PROP]
        axis = (prop.to_3x3() @ Vector((0.0, 1.0, 0.0))).normalized()
        yaws.append(math.degrees(math.atan2(axis.dot(left), axis.dot(fwd))))
        elevations.append(math.degrees(math.asin(max(-1.0, min(1.0, axis.dot(up))))))
        reach.append(prop.translation.dot(fwd) * tgt.unit * 100.0)
    return {
        "prop_yaw_deg": [round(min(yaws), 1), round(max(yaws), 1)],
        "prop_elevation_deg": [round(min(elevations), 1), round(max(elevations), 1)],
        "prop_thrust_cm": round(max(reach) - min(reach), 1),
    }


def prop_world(arm):
    """World matrix of the virtual ``Prop`` bone in the current pose."""
    import battle_fine_proto as fp

    if fp.PROBE.get("arm") is not arm:
        fp.PROBE["arm"] = arm
        fp.PROBE["fa3"] = fp.probe_rig(arm)
    rig = fp.PROBE["fa3"]
    return rig.entries[rig.index["Prop"]][1](False).copy()


def posed_frames(arm, bases, prepare=None):
    """Armature-space matrices of the mapped bones once `bases` (and the hook) are posed."""
    import battle_skinned_poses as poses

    out = []
    for i, basis in enumerate(bases):
        for pb in arm.pose.bones:
            pb.matrix_basis = basis.get(pb.name, Matrix.Identity(4))
        poses.reset_state()
        bpy.context.view_layer.update()
        if prepare is not None:
            prepare(i)
            bpy.context.view_layer.update()
        out.append({b: arm.pose.bones[b].matrix.copy() for b in ORDER})
        out[-1][PROP] = prop_world(arm)
    return out


def keyframed_frames(arm, name):
    """Armature-space matrices of every frame of the keyframed ``human`` clip `name`."""
    import battle_fine_proto as fp

    specs = {c[0]: c for c in bs.human_clip_specs()}
    _n, source, _loop, overrides, _mirror = specs[name]
    act = bs.find_action(source, "CharacterArmature")
    first, last = (int(round(v)) for v in act.frame_range)
    count = getattr(overrides, "frames", None) or (last - first + 1)
    out = []
    for i in range(count):
        fp.pose_clip(arm, name, i / max(count - 1, 1))
        out.append({b: arm.pose.bones[b].matrix.copy() for b in ORDER})
        out[-1][PROP] = prop_world(arm)
    bs.rest_pose(arm)
    return out


def baked_frames(arm, directory, name):
    """Armature-space matrices of a clip baked in `directory` (None if it is not there)."""
    path = os.path.join(directory, "manifest.json")
    if not os.path.exists(path):
        return None
    with open(path) as f:
        manifest = json.load(f)
    clip = manifest["clips"].get(name)
    if clip is None:
        return None
    with open(os.path.join(directory, manifest["texture"]), "rb") as f:
        raw = f.read()
    bones, _frames = struct.unpack("<II", raw[4:12])
    data = np.frombuffer(zlib.decompress(raw[12:]), dtype="<f4")
    data = data.reshape(-1, bones, 3, 4)
    index = {b: i for i, b in enumerate(manifest["bones"])}
    world_inv = arm.matrix_world.inverted()
    out = []
    for row in data[clip["start"] : clip["start"] + clip["frames"]]:
        mats = {}
        for b in ORDER:
            m = Matrix([list(map(float, r)) for r in row[index[b]]] + [[0, 0, 0, 1]])
            bind = arm.matrix_world @ arm.data.bones[b].matrix_local
            mats[b] = world_inv @ (bs.FROM_GODOT @ m @ bs.TO_GODOT @ bind)
        out.append(mats)
    return out


def current_clip(arm, name):
    """(origin, frames) of the clip the game plays today: NT14 default, else keyframed."""
    frames = baked_frames(arm, MELEE_DIR, name)
    if frames is not None:
        return "nt14_default", frames
    return "keyframed", keyframed_frames(arm, name)


# --- Bake ---------------------------------------------------------------------------------


def retarget_all(tgt, sources, table, only=None):
    """``{clip name: Clip}`` of the table (the guard first: other clips may use its arm)."""
    specs = sorted(table["clips"], key=lambda c: c["clip"] != "guard")
    clips = {}
    guard = None
    for spec in specs:
        if only and spec["clip"] not in only and spec["clip"] != "guard":
            continue
        clip = Clip(tgt, sources, table, spec, guard)
        if spec["clip"] == "guard":
            guard = guard_arm(clip.solved[0])
        clips[spec["clip"]] = clip
    return {c["clip"]: clips[c["clip"]] for c in table["clips"] if c["clip"] in clips}


def bake():
    """Bake the clips of the table into ``OUT_DIR`` and write its manifest."""
    import battle_fine_proto as fp

    table = load_table()
    sources = read_sources(table)
    arm, _meshes = bf.load_fine_human()
    rig = bs.Rig("human")
    for b in bs.HUMAN_BONES:
        rig.add(arm, b, b)
    bs.add_human_virtuals(rig, arm)
    rig.capture_rest()
    tgt = target(arm)
    fp.prime_virtuals(arm)
    clips = retarget_all(tgt, sources, table)
    report = {}
    for name, clip in clips.items():
        spec = clip.spec
        hook = clip.prepare(arm)
        nt12.bake_clip(
            arm, rig, name, clip.bases, clip.loop, hook, bool(spec.get("mirror"))
        )
        posed = posed_frames(arm, clip.bases, hook)
        shield = spec["family"] in SHIELD_FAMILIES
        q = measure(tgt, posed, clip.loop, shield)
        q["foot_slide_source_cm"] = round(clip.source_slide * 100.0, 1)
        q["loop_gap_source_cm"] = (
            None if clip.source_gap is None else round(clip.source_gap * 100.0, 1)
        )
        q["loop_blended"] = clip.blended
        q["foot_target_miss_cm"] = round(clip.miss * tgt.unit * 100.0, 1)
        if "prop" in spec:
            q.update(prop_measures(tgt, posed))
            q["yaw_applied_deg"] = round(math.degrees(clip.yaw), 1)
        origin, frames = current_clip(arm, name)
        base = measure(tgt, frames, clip.loop, shield)
        if "prop" in spec:
            base.update(prop_measures(tgt, frames))
        parts = spec["parts"]
        first = table["sources"][parts[0]["source"]]
        report[name] = {
            "family": spec["family"],
            "source": first["title"],
            "author": first["author"],
            "license": first["license"],
            "url": first["url"],
            "actions": [part["action"] for part in parts],
            "note": spec.get("note", ""),
            "quality": q,
            "replaces": origin,
            "replaced_quality": base,
        }
        print("QUALITY", name, json.dumps(q))
        print("CURRENT", name, origin, json.dumps(base))
    bs.rest_pose(arm)
    nt12.write_bake(
        rig,
        OUT_DIR,
        report,
        "tools/blender_scripts/fa3_anim_retarget.py - Mesh2Motion and KayKit animations "
        "(CC0), table data/fx/fa3_anim_sources.json, see docs/wip/fa3-anim.md",
    )
    # `default`: the clips ``BattleSkinned`` substitutes without an option (from the table).
    path = os.path.join(OUT_DIR, "manifest.json")
    with open(path) as f:
        manifest = json.load(f)
    for name, clip in clips.items():
        manifest["clips"][name]["default"] = bool(clip.spec["default"])
        manifest["clip_sources"][name]["default"] = bool(clip.spec["default"])
    with open(path, "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True)


# --- Check renders ------------------------------------------------------------------------

RENDER_FRACS = (0.08, 0.36, 0.64, 0.92)
RENDER_EYE = ((2.4, -4.2, 1.5), (0.0, -0.2, 0.85))
RENDER_LENS = 68  # standing clips; falls are framed wider
RENDER_LENS_GROUND = 40


def render_fractions(spec, count):
    """Fractions of the clip shown on the board (a bow clip: nocked, drawn, loosed, after)."""
    bow = spec.get("bow")
    if not bow:
        return RENDER_FRACS
    last = max(count - 1, 1)
    frames = (
        bow["draw_from"] - 6,
        (bow["draw_from"] + bow["release"]) // 2,
        bow["release"] - 1,
        bow["release"] + 5,
    )
    return tuple(min(max(f / last, 0.0), 1.0) for f in frames)


def render(out, only=None):
    """Workbench renders of each clip: keyframed ``k``, NT14 default ``d``, FA3 ``f``.

    Files ``<family>__<clip>__<k|d|f>__<i>.png`` in `out` (fractions ``RENDER_FRACS``),
    composed into boards by ``fa3_anim_board.py``.
    """
    import battle_fine_figures as ff
    import battle_fine_proto as fp
    import battle_skinned_poses as poses

    os.makedirs(out, exist_ok=True)
    table = load_table()
    sources = read_sources(table)
    families = {}
    for spec in table["clips"]:
        if only and spec["clip"] not in only:
            continue
        families.setdefault(spec["family"], []).append(spec["clip"])
    for family, names in families.items():
        arm, objs, _recipe = ff.build_figure(FIGURES[family], 0)
        for o in objs:
            attr = o.data.attributes.get("vmask")
            for d in attr.data if attr else ():
                d.value &= 0b0011_1111
        objs = fp.apply_variant(objs, 0)
        ff._pose_objects(arm, objs)
        fp.PROBE["rig"] = fp.probe_rig(arm)
        fp.setup_workbench((300, 400))
        cam = fp.camera()
        tgt = target(arm)
        clips = retarget_all(tgt, sources, table, names)

        def show(basis, hook=None, i=0, arm=arm, objs=objs):
            for pb in arm.pose.bones:
                pb.matrix_basis = basis.get(pb.name, Matrix.Identity(4))
            poses.reset_state()
            bpy.context.view_layer.update()
            if hook is not None:
                hook(i)
                bpy.context.view_layer.update()
            fp.follow_prop(objs)

        for name in names:
            clip = clips[name]
            hook = clip.prepare(arm)
            default = baked_frames(arm, MELEE_DIR, name)
            default = default and [nt12.to_basis(tgt, s) for s in default]
            lens = RENDER_LENS_GROUND if clip.spec.get("ground") else RENDER_LENS
            fp.look_at(cam, RENDER_EYE[0], RENDER_EYE[1], lens)
            for i, frac in enumerate(render_fractions(clip.spec, len(clip.bases))):
                stem = os.path.join(out, f"{family}__{name}__")
                fp.pose_clip(arm, name, frac)
                fp.follow_prop(objs)
                fp.render(f"{stem}k__{i}.png")
                if default:
                    show(default[int(round(frac * (len(default) - 1)))])
                    fp.render(f"{stem}d__{i}.png")
                k = int(round(frac * (len(clip.bases) - 1)))
                show(clip.bases[k], hook, k)
                fp.render(f"{stem}f__{i}.png")


def main():
    """``bake`` (default) or ``render DIR [clips]`` after ``--``."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if args and args[0] == "render":
        render(args[1], args[2:] or None)
    else:
        bake()


if __name__ == "__main__":
    main()
