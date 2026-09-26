"""Lot FG4: check renders of the fine mounted figures (Eevee, run inside Blender).

Called by ``battle_fine_cavalry.py -- --render DIR``; writes PNGs to DIR, which
``fg4_planche.py`` composes into ``docs/img/fg/fg4_*.png``:

- ``clip_<figure>_<clip>_<k>.png``: frames of every clip of the ``cavalry`` rig;
- ``type_<figure>_<robe>.png``: the three horse builds and the robes of the shader;
- ``lod_<figure>_<level>.png``: the three levels of detail, same view;
- ``head_<figure>.png``: head and bridle close-up.
"""

import json
import os

import battle_fine_cavalry as fc
import battle_fine_proto as fp
import battle_skinned_equipment as eq
import bpy
from mathutils import Vector

# Robes of battle_soldier_skinned.gdshader (linear), for the renders only.
ROBES = {
    "bai": (0.16, 0.07, 0.03),
    "alezan": (0.26, 0.10, 0.03),
    "noir": (0.03, 0.025, 0.022),
    "gris": (0.34, 0.33, 0.31),
    "isabelle": (0.36, 0.26, 0.13),
}
CLIPS = [
    ("c_idle", (0.0, 0.5)),
    ("c_walk", (0.0, 0.25, 0.5, 0.75)),
    ("c_gallop", (0.0, 0.25, 0.5, 0.75)),
    ("c_charge", (0.0, 0.33, 0.66)),
    ("c_thrust", (0.0, 0.35, 0.7)),
    ("c_death", (0.0, 0.35, 0.7, 1.0)),
    ("c_fall", (0.0, 0.3, 0.6, 1.0)),
]
SIDE = ((4.6, -1.2, 1.35), (0.0, 0.1, 0.95), 32)
THREE_Q = ((3.2, -3.4, 2.0), (0.0, -0.1, 1.1), 40)


def _look(robe):
    """Principled look of the coded materials, the coat in `robe`."""
    for m in bpy.data.materials:
        if "code" not in m:
            continue
        fp.look_current(m)
        code = int(m["code"])
        if code != eq.C_COAT:
            continue
        nt, bsdf = fp._principled(m)
        rgb = tuple(m["rgb"])
        col = tuple(c * r for c, r in zip(ROBES[robe], rgb, strict=True))
        if rgb[0] > 0.5:  # coat: per-vertex shade
            attr = nt.nodes.new("ShaderNodeAttribute")
            attr.attribute_name = "fg_shade"
            mix = nt.nodes.new("ShaderNodeMix")
            mix.data_type = "RGBA"
            mix.blend_type = "MULTIPLY"
            mix.inputs[0].default_value = 1.0
            nt.links.new(attr.outputs["Color"], mix.inputs[6])
            mix.inputs[7].default_value = (*col, 1)
            nt.links.new(mix.outputs[2], bsdf.inputs["Base Color"])
        else:
            bsdf.inputs["Base Color"].default_value = (*col, 1)
        bsdf.inputs["Roughness"].default_value = 0.55


def _rig_rider(rider, mount):
    """Deform the rider pieces with the rider armature (groups without ``R:``)."""
    for o in rider:
        for g in o.vertex_groups:
            if g.name.startswith("R:"):
                g.name = g.name[2:]
        if not any(m.type == "ARMATURE" for m in o.modifiers):
            mod = o.modifiers.new("arm", "ARMATURE")
            mod.object = mount.rarm
            mw = o.matrix_world.copy()
            o.parent = mount.rarm
            o.matrix_world = mw


def _shot(out, name, view, res=(640, 480)):
    eye, target, lens = view
    fp.setup_eevee(res)
    fp.look_at(fp.camera(), Vector(eye), Vector(target), lens)
    fp.render(os.path.join(out, name))


def build(fig_name, level):
    """Posable figure (fine rider + fine horse + harness) at `level`."""
    mount, rider, horse, extra = fc.build_figure(fig_name, level)
    fp.PROBE["rig"] = fp.probe_rig(mount.rarm, "R:")
    _rig_rider(rider, mount)
    return mount, rider, horse, extra


LEGS_FIGURE = "cavalry_1"  # unbarded rouncey: the legs are in full view


def _clips(out, fig, mount, meta):
    """Frames of every clip of the rig for `fig` (side view)."""
    meta["clips"][fig] = {}
    for clip, fracs in CLIPS:
        for k, fr in enumerate(fracs):
            fp.pose_cavalry(mount, clip, fr)
            _shot(out, f"clip_{fig}_{clip}_{k}.png", SIDE, (800, 600))
        meta["clips"][fig][clip] = len(fracs)


def render_all(out, names):
    """All check renders for `names` (clips on the first one, builds on all)."""
    os.makedirs(out, exist_ok=True)
    meta = {"clips": {}, "types": [], "lods": {}}
    first = names[0]
    for fig in names:
        mount, rider, horse, extra = build(fig, 0)
        robe = {"destrier": "bai", "rouncey": "alezan", "jennet": "gris"}[
            fc.HORSE_OF[fig]
        ]
        _look(robe)
        fp.pose_cavalry(mount, "c_idle", 0.0)
        _shot(out, f"type_{fig}_{robe}.png", THREE_Q)
        meta["types"].append([fig, robe, fc.HORSE_OF[fig]])
        if fig == LEGS_FIGURE and fig != first:
            _clips(out, fig, mount, meta)
        if fig != first:
            continue
        head = mount.harm.matrix_world @ mount.harm.data.bones["Head"].head_local
        _shot(
            out,
            f"head_{fig}.png",
            (head + Vector((1.5, -1.4, 0.35)), head + Vector((0, -0.25, -0.2)), 40),
        )
        for robe2 in ("noir", "gris", "isabelle"):
            _look(robe2)
            _shot(out, f"type_{fig}_{robe2}.png", THREE_Q)
            meta["types"].append([fig, robe2, fc.HORSE_OF[fig]])
        _look(robe)
        _clips(out, fig, mount, meta)
        # LOD: same view for the three levels (30 m-like framing).
        for level in (0, 1, 2):
            if level:
                mount, rider, horse, extra = build(fig, level)
                _look(robe)
            fp.pose_cavalry(mount, "c_idle", 0.0)
            _shot(out, f"lod_{fig}_{level}.png", THREE_Q)
            meta["lods"][level] = fc._tris(rider + horse + extra)
    meta["figure"] = first
    with open(os.path.join(out, "meta.json"), "w") as f:
        json.dump(meta, f, indent=1)
