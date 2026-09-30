"""Lot NT12: reader of the CMU ASF/AMC motion capture format (no Blender dependency).

The CMU Graphics Lab database (http://mocap.cs.cmu.edu) ships a skeleton (``.asf``) per
subject and one motion (``.amc``) per take, at 120 frames per second. At rest every bone's
global frame is the identity (the ``direction`` of a bone is global), so the posed global
rotation of a bone is ``W(parent) @ C @ M @ C^-1`` with ``C`` the bone's ``axis`` rotation and
``M`` the product of its degrees of freedom (in the listed order, row-vector convention of the
format, hence the reversed product below in column-vector form).

Coordinates stay in the source frame (Y up, units of the ``:UNITS length`` field); the caller
converts. Usage (outside Blender, to inspect a take)::

    python3 tools/blender_scripts/mocap_asf.py <file.asf> <file.amc>
"""

import math
import sys

import numpy as np

SOURCE_FPS = 120


def _rot(axis, degrees):
    """3x3 rotation about the named axis (column vectors)."""
    a = math.radians(degrees)
    c, s = math.cos(a), math.sin(a)
    if axis == "x":
        return np.array([[1, 0, 0], [0, c, -s], [0, s, c]])
    if axis == "y":
        return np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])
    return np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])


def euler(order, values):
    """Rotation of Euler angles applied in `order` (first letter first)."""
    m = np.eye(3)
    for axis, v in zip(order, values, strict=True):
        m = _rot(axis, v) @ m
    return m


class Bone:
    """One ASF bone: rest direction and length, axis frame, degrees of freedom."""

    def __init__(self, name):
        """Empty bone called `name`."""
        self.name = name
        self.direction = np.zeros(3)
        self.length = 0.0
        self.axis = np.eye(3)
        self.dof = []
        self.parent = None
        self.children = []


class Skeleton:
    """ASF skeleton: bones in hierarchy order below ``root``."""

    def __init__(self, path):
        """Parse the ASF file at `path`."""
        self.bones = {"root": Bone("root")}
        self.root_order = ["tx", "ty", "tz", "rx", "ry", "rz"]
        self.root_axis = "xyz"
        self.length_unit = 1.0
        section = None
        current = None
        with open(path) as f:
            lines = [ln.strip() for ln in f if ln.strip() and not ln.startswith("#")]
        for ln in lines:
            if ln.startswith(":"):
                section = ln.split()[0][1:]
                continue
            words = ln.split()
            if section == "units" and words[0] == "length":
                self.length_unit = float(words[1])
            elif section == "root":
                if words[0] == "order":
                    self.root_order = [w.lower() for w in words[1:]]
                elif words[0] == "axis":
                    self.root_axis = words[1].lower()
            elif section == "bonedata":
                if words[0] == "begin":
                    current = None
                elif words[0] == "name":
                    current = Bone(words[1])
                    self.bones[current.name] = current
                elif words[0] == "direction":
                    current.direction = np.array([float(w) for w in words[1:4]])
                elif words[0] == "length":
                    current.length = float(words[1])
                elif words[0] == "axis":
                    order = words[4].lower()
                    current.axis = euler(order, [float(w) for w in words[1:4]])
                elif words[0] == "dof":
                    current.dof = [w.lower() for w in words[1:]]
            elif section == "hierarchy" and words[0] not in ("begin", "end"):
                parent = self.bones[words[0]]
                for child in words[1:]:
                    self.bones[child].parent = parent
                    parent.children.append(self.bones[child])
        self.order = []
        stack = [self.bones["root"]]
        while stack:
            b = stack.pop(0)
            self.order.append(b)
            stack.extend(b.children)


def read_amc(path):
    """Frames of an AMC file: list of {bone name: [values]} in file order."""
    frames = []
    current = None
    with open(path) as f:
        for ln in f:
            ln = ln.strip()
            if not ln or ln.startswith(("#", ":")):
                continue
            words = ln.split()
            if len(words) == 1 and words[0].isdigit():
                current = {}
                frames.append(current)
                continue
            if current is not None:
                current[words[0]] = [float(w) for w in words[1:]]
    return frames


def pose(skel, frame):
    """Global rotation (3x3) and joint start/end positions of every bone for one frame.

    Returns ``{name: (rotation, start, end)}``; the root's start and end are its position.
    """
    out = {}
    root = frame.get("root", [0.0] * 6)
    values = dict(zip(skel.root_order, root, strict=False))
    pos = np.array(
        [values.get("tx", 0.0), values.get("ty", 0.0), values.get("tz", 0.0)]
    )
    order = [a[1] for a in skel.root_order if a.startswith("r")]
    rot = euler(order, [values["r" + a] for a in order])
    out["root"] = (rot, pos, pos)
    for bone in skel.order[1:]:
        prot, _pstart, pend = out[bone.parent.name]
        vals = frame.get(bone.name, [0.0] * len(bone.dof))
        m = euler([d[1] for d in bone.dof], vals) if bone.dof else np.eye(3)
        c = bone.axis
        world = prot @ c @ m @ c.T
        end = pend + world @ (bone.direction * bone.length)
        out[bone.name] = (world, pend, end)
    return out


def rest(skel):
    """Pose of the skeleton with every degree of freedom at zero."""
    return pose(skel, {"root": [0.0] * 6})


def main():
    """Print, per second of the take, the right hand height and speed (to pick segments)."""
    skel = Skeleton(sys.argv[1])
    frames = read_amc(sys.argv[2])
    prev = None
    for i in range(0, len(frames), 12):
        p = pose(skel, frames[i])
        hand = p["rhand"][2]
        root = p["root"][1]
        speed = 0.0 if prev is None else float(np.linalg.norm(hand - prev)) * 10
        prev = hand
        print(
            f"{i:5d} t={i / SOURCE_FPS:5.1f}s root=({root[0]:6.1f},{root[1]:5.1f},{root[2]:6.1f})"
            f" rhand_h={hand[1] - root[1]:5.1f} v={speed:6.1f}"
        )


if __name__ == "__main__":
    main()
