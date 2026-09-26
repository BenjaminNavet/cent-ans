"""Lot FG0: baked material maps for the finer figures (test bake, not wired into the game).

One atlas per figure: the pieces are joined (weights and material slots kept), unwrapped
together at a uniform texel density, then Cycles bakes

* ``normal``: tangent-space normal map of procedural relief per material, in metres of the
  surface (mail rings of 9 mm, cloth weave and vertical folds, leather grain, hammered
  steel, skin pores);
* ``orm``: R = ambient occlusion, G = roughness, B = metallic;
* ``mask``: R = livery (dyed cloth tinted per side), G = heraldry (arms painted from the
  ``heraldry`` UV), B = skin.

The game would keep one such atlas per figure family (shared by its LODs), the livery and
arms still coming from the shader's uniforms (DA1).
"""

import math
import os

import bmesh
import bpy
import numpy as np

# Relief per material: (kind, strength).
RELIEF = {
    "mail": ("mail", 1.0),
    "livery": ("cloth", 1.0),
    "cloth": ("cloth", 0.6),
    "leather": ("leather", 0.5),
    "steel": ("steel", 0.25),
    "brass": ("steel", 0.2),
    "skin": ("skin", 0.15),
    "arms": ("paint", 0.25),
    "wood": ("leather", 0.4),
    "eye": ("none", 0.0),
    "hair": ("cloth", 0.4),
}
MAIL_PITCH = 0.009  # metres between ring centres


def join_for_bake(objs, name):
    """Join copies of `objs` into one mesh (weights, slots, modifiers of the first kept)."""
    copies = []
    for o in objs:
        c = o.copy()
        c.data = o.data.copy()
        bpy.context.scene.collection.objects.link(c)
        copies.append(c)
    base = copies[0]
    with bpy.context.temp_override(
        active_object=base,
        object=base,
        selected_objects=copies,
        selected_editable_objects=copies,
    ):
        bpy.ops.object.join()
    base.name = base.data.name = name
    for o in objs:
        o.hide_render = True
    return base


def unwrap(obj, margin=0.004):
    """Atlas UV (uniform texel density); returns UV units per metre."""
    me = obj.data
    atlas = me.uv_layers.get("atlas") or me.uv_layers.new(name="atlas")
    me.uv_layers.active = atlas
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(
        angle_limit=math.radians(55), island_margin=margin, scale_to_bounds=False
    )
    bpy.ops.uv.pack_islands(rotate=True, margin=margin)
    bpy.ops.object.mode_set(mode="OBJECT")
    bm = bmesh.new()
    bm.from_mesh(me)
    uv = bm.loops.layers.uv["atlas"]
    area3 = sum(f.calc_area() for f in bm.faces)
    area_uv = 0.0
    for f in bm.faces:
        pts = [loop[uv].uv for loop in f.loops]
        for i in range(1, len(pts) - 1):
            a, b, c = pts[0], pts[i], pts[i + 1]
            area_uv += abs((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)) / 2
    bm.free()
    per_m = math.sqrt(area_uv / area3)
    print(
        f"ATLAS {obj.name} surface={area3:.2f} m2 uv_per_m={per_m:.3f} fill={area_uv:.2f}"
    )
    return per_m


# --- Procedural relief nodes ------------------------------------------------------------


def _n(nt, kind, **inputs):
    node = nt.nodes.new(kind)
    for k, v in inputs.items():
        if isinstance(v, bpy.types.NodeSocket):
            nt.links.new(v, node.inputs[k])
        else:
            node.inputs[k].default_value = v
    return node


def _math(nt, op, a, b=None, c=None):
    node = nt.nodes.new("ShaderNodeMath")
    node.operation = op
    for i, x in enumerate((a, b, c)):
        if x is None:
            continue
        if isinstance(x, bpy.types.NodeSocket):
            nt.links.new(x, node.inputs[i])
        else:
            node.inputs[i].default_value = x
    return node.outputs[0]


def relief_height(nt, kind, per_m):
    """Height socket (0-1) of a relief kind; coordinates in metres from the atlas UV."""
    uv = _n(nt, "ShaderNodeUVMap")
    uv.uv_map = "atlas"
    metres = _n(nt, "ShaderNodeVectorMath")
    metres.operation = "SCALE"
    metres.inputs["Scale"].default_value = 1.0 / per_m
    nt.links.new(uv.outputs[0], metres.inputs[0])
    pos = _n(nt, "ShaderNodeNewGeometry").outputs["Position"]
    if kind == "mail":
        # Rings: distance to the nearest lattice centre, staggered rows via a 60 deg lattice.
        vor = _n(nt, "ShaderNodeTexVoronoi", Scale=1.0 / MAIL_PITCH, Randomness=0.0)
        vor.voronoi_dimensions = "2D"
        nt.links.new(metres.outputs[0], vor.inputs["Vector"])
        d = vor.outputs["Distance"]
        ring = _math(
            nt,
            "SUBTRACT",
            1.0,
            _math(
                nt,
                "DIVIDE",
                _math(nt, "ABSOLUTE", _math(nt, "SUBTRACT", d, 0.36)),
                0.14,
            ),
        )
        return _math(nt, "MAXIMUM", ring, 0.0)
    if kind == "cloth":
        # Weave (1 mm) + vertical folds (world position, stretched along Z).
        wave = _n(nt, "ShaderNodeTexWave", Scale=1.0 / 0.0012 / 6.28, Distortion=0.0)
        wave.wave_type = "BANDS"
        wave.bands_direction = "DIAGONAL"
        nt.links.new(metres.outputs[0], wave.inputs["Vector"])
        stretch = _n(nt, "ShaderNodeVectorMath")
        stretch.operation = "MULTIPLY"
        nt.links.new(pos, stretch.inputs[0])
        stretch.inputs[1].default_value = (14.0, 14.0, 1.6)
        folds = _n(nt, "ShaderNodeTexNoise", Scale=1.0, Detail=2.0, Roughness=0.4)
        nt.links.new(stretch.outputs[0], folds.inputs["Vector"])
        return _math(
            nt,
            "MULTIPLY_ADD",
            folds.outputs["Fac"],
            0.85,
            _math(nt, "MULTIPLY", wave.outputs["Fac"], 0.15),
        )
    if kind in ("leather", "skin", "paint"):
        scale = {"leather": 1.0 / 0.004, "skin": 1.0 / 0.0015, "paint": 1.0 / 0.01}[
            kind
        ]
        noise = _n(nt, "ShaderNodeTexNoise", Scale=scale, Detail=6.0, Roughness=0.6)
        nt.links.new(metres.outputs[0], noise.inputs["Vector"])
        return noise.outputs["Fac"]
    if kind == "steel":
        vor = _n(nt, "ShaderNodeTexVoronoi", Scale=1.0 / 0.012, Randomness=1.0)
        vor.feature = "SMOOTH_F1"
        nt.links.new(pos, vor.inputs["Vector"])
        return vor.outputs["Distance"]
    return None


# Relief amplitude in metres per kind (Bump distance).
AMPLITUDE = {
    "mail": 0.0025,
    "cloth": 0.012,
    "leather": 0.0008,
    "steel": 0.0006,
    "skin": 0.0003,
    "paint": 0.0004,
}


def set_bake_shader(obj, per_m, mode):
    """Rebuild every material of `obj` for one bake pass (`normal` or `emit`)."""
    for slot in obj.material_slots:
        m = slot.material
        if m is None:
            continue
        name = m.get("fg", "")
        m.use_nodes = True
        nt = m.node_tree
        nt.nodes.clear()
        out = _n(nt, "ShaderNodeOutputMaterial")
        if mode == "normal":
            bsdf = _n(nt, "ShaderNodeBsdfDiffuse")
            kind, strength = RELIEF.get(name, ("none", 0.0))
            h = relief_height(nt, kind, per_m) if strength else None
            if h is not None:
                bump = _n(
                    nt, "ShaderNodeBump", Strength=strength, Distance=AMPLITUDE[kind]
                )
                nt.links.new(h, bump.inputs["Height"])
                nt.links.new(bump.outputs[0], bsdf.inputs["Normal"])
            nt.links.new(bsdf.outputs[0], out.inputs[0])
        else:
            emit = _n(nt, "ShaderNodeEmission", Strength=1.0)
            emit.inputs[0].default_value = mode_colour(m, mode)
            nt.links.new(emit.outputs[0], out.inputs[0])


def mode_colour(m, mode):
    """Flat colour of a material for an emission bake pass."""
    name = m.get("fg", "")
    if mode == "rm":
        return (0.0, float(m.get("rough", 0.6)), float(m.get("metal", 0.0)), 1.0)
    if mode == "mask":
        return (
            1.0 if name in ("livery",) else 0.0,
            1.0 if name == "arms" else 0.0,
            1.0 if name == "skin" else 0.0,
            1.0,
        )
    raise ValueError(mode)


def bake(obj, kind, image, samples=8):
    """One Cycles bake of `obj` into `image` through the ``atlas`` UV."""
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = samples
    scene.cycles.device = "CPU"
    for slot in obj.material_slots:
        nt = slot.material.node_tree
        tex = nt.nodes.get("fg_bake") or nt.nodes.new("ShaderNodeTexImage")
        tex.name = "fg_bake"
        tex.image = image
        nt.nodes.active = tex
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    obj.data.uv_layers.active = obj.data.uv_layers["atlas"]
    kwargs = {"type": kind, "margin": 4}
    if kind == "NORMAL":
        kwargs["normal_space"] = "TANGENT"
    bpy.ops.object.bake(**kwargs)


def bake_atlas(obj, out_dir, prefix, size=1024):
    """Bake normal, ORM and mask maps of a joined figure; returns {map: path}."""
    per_m = unwrap(obj)
    # Keep the look materials: bake on copies of the slots' materials.
    originals = [s.material for s in obj.material_slots]
    for s in obj.material_slots:
        s.material = s.material.copy()
    paths = {}
    imgs = {}
    for key, colour in (
        ("normal", "Non-Color"),
        ("ao", "Non-Color"),
        ("rm", "Non-Color"),
        ("mask", "Non-Color"),
    ):
        img = bpy.data.images.new(
            f"{prefix}_{key}", size, size, alpha=False, float_buffer=False
        )
        img.colorspace_settings.name = colour
        imgs[key] = img
    set_bake_shader(obj, per_m, "normal")
    bake(obj, "NORMAL", imgs["normal"], samples=4)
    bake(obj, "AO", imgs["ao"], samples=32)
    set_bake_shader(obj, per_m, "rm")
    bake(obj, "EMIT", imgs["rm"], samples=1)
    set_bake_shader(obj, per_m, "mask")
    bake(obj, "EMIT", imgs["mask"], samples=1)
    # ORM = (AO, roughness, metallic).
    ao = np.array(imgs["ao"].pixels[:]).reshape(size, size, 4)
    rm = np.array(imgs["rm"].pixels[:]).reshape(size, size, 4)
    orm = rm.copy()
    orm[..., 0] = ao[..., 0]
    orm_img = bpy.data.images.new(f"{prefix}_orm", size, size, alpha=False)
    orm_img.colorspace_settings.name = "Non-Color"
    orm_img.pixels[:] = orm.ravel()
    imgs["orm"] = orm_img
    os.makedirs(out_dir, exist_ok=True)
    for key in ("normal", "orm", "mask"):
        path = os.path.join(out_dir, f"{prefix}_{key}.png")
        imgs[key].filepath_raw = path
        imgs[key].file_format = "PNG"
        imgs[key].save()
        paths[key] = path
        print("BAKED", path)
    for s, m in zip(obj.material_slots, originals, strict=True):
        bake_copy = s.material
        s.material = m
        bpy.data.materials.remove(bake_copy)
    return imgs


def use_baked(obj, imgs):
    """Plug the baked normal and ORM maps into the Eevee materials of `obj` (copies)."""
    for s in obj.material_slots:
        m = s.material.copy()
        s.material = m
        nt = m.node_tree
        bsdf = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
        uv = nt.nodes.new("ShaderNodeUVMap")
        uv.uv_map = "atlas"
        ntex = nt.nodes.new("ShaderNodeTexImage")
        ntex.image = imgs["normal"]
        nt.links.new(uv.outputs[0], ntex.inputs[0])
        nmap = nt.nodes.new("ShaderNodeNormalMap")
        nmap.uv_map = "atlas"
        nt.links.new(ntex.outputs[0], nmap.inputs["Color"])
        nt.links.new(nmap.outputs[0], bsdf.inputs["Normal"])
        otex = nt.nodes.new("ShaderNodeTexImage")
        otex.image = imgs["orm"]
        nt.links.new(uv.outputs[0], otex.inputs[0])
        sep = nt.nodes.new("ShaderNodeSeparateColor")
        nt.links.new(otex.outputs[0], sep.inputs[0])
        nt.links.new(sep.outputs[1], bsdf.inputs["Roughness"])
        nt.links.new(sep.outputs[2], bsdf.inputs["Metallic"])
        # AO multiplies the base colour (the game would feed it to the ambient term).
        base = bsdf.inputs["Base Color"]
        mix = nt.nodes.new("ShaderNodeMix")
        mix.data_type = "RGBA"
        mix.blend_type = "MULTIPLY"
        mix.inputs[0].default_value = 1.0
        if base.links:
            nt.links.new(base.links[0].from_socket, mix.inputs[6])
        else:
            mix.inputs[6].default_value = base.default_value
        nt.links.new(sep.outputs[0], mix.inputs[7])
        nt.links.new(mix.outputs[2], base)
