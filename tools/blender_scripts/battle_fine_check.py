"""Lot FG1: render the *exported* figures skinned by the *baked* bone textures.

The check skins the ``CAM1`` mesh with the ``CAB1`` matrices on the CPU, exactly like
``battle_soldier_skinned.gdshader`` (four bones, virtual bones ``Prop`` / ``Nock`` /
``Arrow`` included), so what it shows is what the game draws: hands on the weapons, arrow
at the cheek, feet in the stirrups. Run from the repository root:

    blender -b --factory-startup --python tools/blender_scripts/battle_fine_check.py -- \
        <out dir> [--dir game/assets/models/battle_fine] [--only fig,fig]
"""

import json
import os
import struct
import sys
import zlib

import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_skinned as bs  # noqa: E402

# (figure, clip, fractions of the clip, camera eye, target), Blender world space.
SHOTS = [
    ("infantry_0", "guard", (0.0,), (0.0, -3.4, 1.3), (0, 0, 1.0)),
    ("infantry_0", "slash", (0.45,), (1.9, -2.6, 1.5), (0, 0, 1.0)),
    ("archer_0", "bow_shoot", (0.1, 0.5), (-2.4, -1.4, 1.6), (0, -0.2, 1.35)),
    ("archer_0", "bow_shoot", (0.5,), (0.9, -1.0, 1.65), (0, -0.1, 1.5)),
    ("archer_1", "xbow_shoot", (0.05, 0.5), (1.9, -2.4, 1.4), (0, 0, 1.0)),
    ("infantry_1", "pike_level", (0.0,), (2.3, -2.4, 1.5), (0, -0.6, 1.0)),
    ("infantry_1", "pike_idle", (0.0,), (1.9, -2.6, 1.5), (0, 0, 1.2)),
    ("infantry_2", "pike_thrust", (0.4,), (2.3, -2.4, 1.5), (0, -0.4, 1.0)),
    ("cavalry_0", "c_idle", (0.0,), (-3.6, -3.4, 1.9), (0, 0, 1.3)),
    ("cavalry_0", "c_charge", (0.3,), (3.8, -3.0, 2.0), (0, -0.3, 1.3)),
    ("cavalry_2", "c_bow_shoot", (0.5,), (-3.6, -2.4, 2.2), (0, 0, 1.6)),
    ("standard_0", "std_idle", (0.0,), (1.8, -3.0, 1.6), (0, 0, 1.3)),
    ("musician_0", "drum_beat", (0.3,), (1.4, -2.6, 1.4), (0, 0, 1.1)),
    ("crew_1", "swab", (0.3,), (1.9, -2.6, 1.4), (0, 0, 1.0)),
]


def load_mesh(path):
    """(positions, colours rgba, bones, weights, masks, indices) of a ``CAM1`` file."""
    with open(path, "rb") as f:
        data = f.read()
    n, m, size = struct.unpack("<III", data[4:16])
    raw = zlib.decompress(data[16:])
    assert len(raw) == size
    floats = struct.unpack(f"<{n * 21}f", raw[: n * 84])
    idx = struct.unpack(f"<{m}I", raw[n * 84 : n * 84 + m * 4])

    def cols(offset, width):
        return [floats[offset + i * width : offset + (i + 1) * width] for i in range(n)]

    return (
        cols(0, 3),
        cols(n * 6, 4),
        cols(n * 12, 4),
        cols(n * 16, 4),
        floats[n * 20 : n * 21],
        idx,
    )


def load_bones(path):
    """Per frame, per bone: 3x4 skinning matrix (Godot space) of a ``CAB1`` file."""
    with open(path, "rb") as f:
        data = f.read()
    bones, frames = struct.unpack("<II", data[4:12])
    raw = zlib.decompress(data[12:])
    vals = struct.unpack(f"<{bones * frames * 12}f", raw)
    out = []
    for fr in range(frames):
        row = []
        for b in range(bones):
            o = (fr * bones + b) * 12
            m = Matrix(
                (
                    vals[o : o + 4],
                    vals[o + 4 : o + 8],
                    vals[o + 8 : o + 12],
                    (0, 0, 0, 1),
                )
            )
            row.append(m)
        out.append(row)
    return out


def skinned_object(name, mesh, mats, variant):
    """Blender object of the mesh skinned by `mats`, faces of other variants dropped."""
    pos, col, bones, weights, masks, idx = mesh
    verts = []
    for i, p in enumerate(pos):
        acc = Vector((0.0, 0.0, 0.0))
        for b, w in zip(bones[i], weights[i], strict=True):
            if w > 0:
                acc += (mats[int(b)] @ Vector((*p, 1.0))).to_3d() * w
        verts.append((acc.x, -acc.z, acc.y))  # Godot -> Blender
    faces = []
    for t in range(0, len(idx), 3):
        a, b, c = idx[t : t + 3]
        mk = int(masks[a]) & 0b0011_1111
        if mk and not mk & (1 << variant):
            continue
        faces.append((a, c, b))  # back to counter-clockwise
    me = bpy.data.meshes.new(name)
    me.from_pydata(verts, [], faces)
    attr = me.color_attributes.new("col", "FLOAT_COLOR", "POINT")
    for i, c in enumerate(col):
        attr.data[i].color = (c[0], c[1], c[2], 1.0)
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def render_shots(out, mesh_dir, only=None):
    """Render `SHOTS` from the exported data in `mesh_dir` into `out`."""
    import battle_fine_proto as fp

    with open(os.path.join(mesh_dir, "manifest.json")) as f:
        manifest = json.load(f)
    textures = {}
    os.makedirs(out, exist_ok=True)
    for k, (fig, clip, fracs, eye, target) in enumerate(SHOTS):
        if only and fig not in only:
            continue
        entry = manifest["figures"].get(fig)
        if entry is None:
            continue
        rig = manifest["rigs"][entry["rig"]]
        if entry["rig"] not in textures:
            textures[entry["rig"]] = load_bones(os.path.join(mesh_dir, rig["texture"]))
        frames = textures[entry["rig"]]
        mesh = load_mesh(os.path.join(mesh_dir, entry["lods"][0]))
        c = rig["clips"][clip]
        for j, frac in enumerate(fracs):
            bs.reset_scene()
            fr = c["start"] + int(round(frac * (c["frames"] - 1)))
            skinned_object(fig, mesh, frames[fr], 0)
            scene = fp.setup_workbench((560, 700))
            scene.display.shading.color_type = "VERTEX"
            cam = fp.camera()
            fp.look_at(cam, Vector(eye), Vector(target), 45)
            fp.render(os.path.join(out, f"{k:02d}_{fig}_{clip}_{j}.png"))


def main():
    """Parse the output directory and options, render the shots."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out = args[0] if args else "/tmp/fg1_baked"
    mesh_dir = os.path.join(bs.ROOT, "game", "assets", "models", "battle_fine")
    if "--dir" in args:
        mesh_dir = os.path.join(bs.ROOT, args[args.index("--dir") + 1])
    only = None
    if "--only" in args:
        only = set(args[args.index("--only") + 1].split(","))
    render_shots(out, mesh_dir, only)
    print("OK")


if __name__ == "__main__":
    main()
