"""Lots FG0 and FG3: baked material maps of the fine figures.

FG3 (production, ``battle_fine.py -- bake``): see the second half of this module. FG0 (test
bake of the prototype, below) is kept for ``battle_fine_proto.py``.

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


# ==========================================================================================
# Lot FG3: production bake of the fine figures (``battle_fine.py -- bake``)
# ==========================================================================================
#
# Memory plan (FG0 recommendation): shared tileable detail maps per material
# (``battle_fine_tiles``, one Texture2DArray) + one small atlas per figure and level of detail
# (LOD0 512², LOD1 256², one layer of a Texture2DArray each), + the CC0 horse coat reduced to
# 1024² and shared by every mount. LOD2 carries no atlas (no texture read in the shader).
#
# Atlas texel (RGBA8, BC7 in game): RG = shape normal (tangent space of the ``fg_atlas`` UV,
# baked from the high-definition pieces: the builds before decimation, cloth pieces with
# subdivided folds), B = ambient occlusion (the whole figure, variant by variant, horse
# included), A = mask attribute ``fg_mask`` (per material: hair/beard density, eyebrows on
# the skin, hammering strength of the plates; 1 elsewhere).

import fcntl  # noqa: E402
import heapq  # noqa: E402

import battle_fine_tiles as tiles  # noqa: E402
from mathutils import Vector  # noqa: E402

ATLAS_UV = "fg_atlas"
MASK_ATTR = "fg_mask"
ATLAS_SIZE = (512, 256)  # LOD0, LOD1
ATLAS_SRC = (1, 3)  # packed source code of the shader per level (2 = horse coat)
HORSE_SRC = 2
HORSE_SIZE = 1024
DILATE = (8, 4)
HEAD_TEXEL_SCALE = 1.8  # heads, hair and beards get more texels than the rest
HELMETS = {
    "bassinet",
    "fine_bassinet",
    "kettle_hat",
    "great_helm",
    "sallet",
    "cervelliere",
    "skull_cap",
}
PLATE_MASK = 0.35  # hammering of the plates that are not helmets
CLOTH_CODES = (0, 4, 6, 12)  # livery, cloth, arms, quilt: folds in the HD source
FOLD_DEPTH = 0.007  # metres

TEX_FILES = {
    "lod0": "fine_atlas_lod0.png",
    "lod1": "fine_atlas_lod1.png",
    "detail": "fine_detail.png",
    "horse": "fine_horse.png",
}

_HD = None


# --- High-definition sources --------------------------------------------------------------


def capture_begin():
    """Start keeping a copy of every piece before its first decimation."""
    global _HD
    _HD = {}


def capture_end():
    """Stop capturing; returns {piece name: HD copy}."""
    global _HD
    hd, _HD = _HD, None
    return hd or {}


def capture(obj):
    """Copy `obj` (untouched geometry) if a capture runs and it was not copied yet."""
    if _HD is None or obj.type != "MESH" or obj.name in _HD:
        return
    c = obj.copy()
    c.data = obj.data.copy()
    c.name = f"hd_{obj.name}"
    for coll in obj.users_collection:
        coll.objects.link(c)
    c.hide_render = True
    _HD[obj.name] = c


def _codes(obj):
    """Material code of each face of `obj`."""
    slots = [
        int(s.material["code"])
        if s.material is not None and "code" in s.material
        else 5
        for s in obj.material_slots
    ]
    return [slots[p.material_index] if slots else 5 for p in obj.data.polygons]


def _fold_driver():
    """Empty whose scale stretches the fold noise along the height (vertical folds)."""
    e = bpy.data.objects.get("fg3_folds")
    if e is None:
        e = bpy.data.objects.new("fg3_folds", None)
        bpy.context.scene.collection.objects.link(e)
        e.scale = (1 / 14.0, 1 / 14.0, 1 / 1.6)
    tex = bpy.data.textures.get("fg3_folds") or bpy.data.textures.new(
        "fg3_folds", "CLOUDS"
    )
    tex.noise_scale = 1.0
    tex.noise_depth = 2
    return e, tex


def hd_source(low, hd):
    """High-definition counterpart of `low` for the normal bake (None: identical).

    Decimated pieces use their capture; cloth pieces get subdivided folds (displacement
    along the normal, stretched vertically) so that even an undecimated LOD0 piece gains a
    shape normal.
    """
    src = hd.get(low.name)
    codes = _codes(low)
    cloth = bool(codes) and sum(c in CLOTH_CODES for c in codes) > 0.6 * len(codes)
    if src is None and not cloth:
        return None
    if src is None:
        src = low.copy()
        src.data = low.data.copy()
        src.name = f"hd_{low.name}"
        bpy.context.scene.collection.objects.link(src)
        src.hide_render = True
        hd[low.name] = src
    if cloth and not src.get("fg3_folds"):
        src["fg3_folds"] = True
        sub = src.modifiers.new("fg3_sub", "SUBSURF")
        sub.levels = sub.render_levels = 1
        empty, tex = _fold_driver()
        disp = src.modifiers.new("fg3_folds", "DISPLACE")
        disp.texture = tex
        disp.texture_coords = "OBJECT"
        disp.texture_coords_object = empty
        disp.strength = FOLD_DEPTH
        disp.mid_level = 0.5
    return src


# --- Mask attribute -----------------------------------------------------------------------


def set_mask(obj, values):
    """Per-vertex mask (float or list) in the colour attribute ``fg_mask``."""
    me = obj.data
    attr = me.color_attributes.get(MASK_ATTR) or me.color_attributes.new(
        MASK_ATTR, "FLOAT_COLOR", "POINT"
    )
    if isinstance(values, (int, float)):
        values = [float(values)] * len(me.vertices)
    for i, v in enumerate(values):
        attr.data[i].color = (v, v, v, 1.0)


def has_mask(obj):
    """True if `obj` carries a mask attribute."""
    return obj.type == "MESH" and obj.data.color_attributes.get(MASK_ATTR) is not None


def _smooth(e0, e1, x):
    t = min(max((x - e0) / (e1 - e0), 0.0), 1.0)
    return t * t * (3 - 2 * t)


def boundary_distance(obj):
    """Geodesic distance (metres, along edges) of every vertex to the open border."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.verts.ensure_lookup_table()
    mw = obj.matrix_world
    co = [mw @ v.co for v in bm.verts]
    dist = [math.inf] * len(bm.verts)
    heap = []
    for e in bm.edges:
        if e.is_boundary:
            for v in e.verts:
                if dist[v.index] > 0:
                    dist[v.index] = 0.0
                    heap.append((0.0, v.index))
    heapq.heapify(heap)
    while heap:
        d, i = heapq.heappop(heap)
        if d > dist[i]:
            continue
        for e in bm.verts[i].link_edges:
            j = e.other_vert(bm.verts[i]).index
            nd = d + (co[i] - co[j]).length
            if nd < dist[j]:
                dist[j] = nd
                heapq.heappush(heap, (nd, j))
    bm.free()
    return [d if d < math.inf else 1.0 for d in dist]


def hair_mask(obj, lm, beard):
    """Density of a hair or beard shell: thin at the border, open around the mouth."""
    dist = boundary_distance(obj)
    mw = obj.matrix_world
    mouth_z = lm.eye.z - 0.068
    values = []
    for v, d in zip(obj.data.vertices, dist, strict=True):
        m = _smooth(0.0, 0.016 if beard else 0.012, d)
        if beard:
            p = mw @ v.co
            if p.y < lm.eye.y:
                r = math.hypot((p.x - lm.eye.x) / 1.4, p.z - mouth_z)
                m *= 0.15 + 0.85 * _smooth(0.012, 0.03, r)
        values.append(m)
    set_mask(obj, values)


def skin_mask(obj, lm):
    """Eyebrows (darker towards the hair colour) on a head; 1 = plain skin."""
    mw = obj.matrix_world
    values = []
    for v in obj.data.vertices:
        p = mw @ v.co
        m = 1.0
        if p.y < lm.eye.y - 0.005:
            dz = p.z - (lm.eye.z + 0.02)
            dx = abs(p.x - lm.eye.x)
            if 0.008 < dx < 0.058:
                arch = 0.004 * math.cos((dx - 0.03) / 0.03 * 1.4)
                band = 1.0 - _smooth(0.004, 0.009, abs(dz - arch))
                m = 1.0 - 0.7 * band
        values.append(m)
    set_mask(obj, values)


def gear_mask(name, objs):
    """Hammering strength of the plates of an equipment piece (helmets full)."""
    value = 1.0 if name in HELMETS else PLATE_MASK
    for o in objs:
        if o.type == "MESH" and not has_mask(o):
            set_mask(o, value)


# --- Atlas UV -----------------------------------------------------------------------------


def _is_head(obj):
    return obj.name.split("_")[0] in ("head", "hair", "beard")


def _edit(objs, fn):
    """Run `fn()` in edit mode on `objs` (everything selected, UV selection synced)."""
    view_layer = bpy.context.view_layer
    for o in view_layer.objects:
        o.select_set(False)
    for o in objs:
        o.select_set(True)
    view_layer.objects.active = objs[0]
    bpy.context.scene.tool_settings.use_uv_select_sync = True
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.reveal()
    bpy.ops.mesh.select_all(action="SELECT")
    fn()
    bpy.ops.object.mode_set(mode="OBJECT")
    for o in objs:
        o.select_set(False)


def atlas_unwrap(objs, size):
    """One ``fg_atlas`` UV for all `objs`, packed at a uniform texel density (heads x1.8).

    The islands are seeded from the pieces' own UVs (MakeHuman UVs carried by the body,
    head and garment shells; FG2 per-piece UVs), which survive the decimation as large
    islands; a smart projection of the decimated shells would cut them into ~1 000 slivers.
    Pieces without UVs are smart-projected. The packing separates overlapping islands.
    """
    active = bpy.context.view_layer.objects.active
    if active is not None and active.mode != "OBJECT":
        bpy.ops.object.mode_set(mode="OBJECT")
    previous = {}
    bare = []
    for o in objs:
        me = o.data
        previous[o.name] = me.uv_layers.active.name if me.uv_layers.active else None
        sources = [u for u in me.uv_layers if u.name != ATLAS_UV]
        source = next((u for u in sources if u.name != "heraldry"), None) or (
            sources[0] if sources else None
        )
        layer = me.uv_layers.get(ATLAS_UV) or me.uv_layers.new(name=ATLAS_UV)
        if source is not None:
            uvs = np.empty(len(source.data) * 2, np.float32)
            source.data.foreach_get("uv", uvs)
            layer.data.foreach_set("uv", uvs)
        else:
            bare.append(o)
        me.uv_layers.active = layer
    if bare:
        _edit(
            bare,
            lambda: bpy.ops.uv.smart_project(
                angle_limit=math.radians(66), island_margin=0.0, scale_to_bounds=False
            ),
        )
    margin = 3.0 / size

    def pack():
        bpy.ops.uv.select_all(action="SELECT")
        bpy.ops.uv.average_islands_scale()
        for o in objs:
            if not _is_head(o):
                continue
            bm = bmesh.from_edit_mesh(o.data)
            uv = bm.loops.layers.uv[ATLAS_UV]
            pts = [loop[uv].uv.copy() for f in bm.faces for loop in f.loops]
            if not pts:
                continue
            c = sum(pts, Vector((0.0, 0.0))) / len(pts)
            for f in bm.faces:
                for loop in f.loops:
                    loop[uv].uv = c + (loop[uv].uv - c) * HEAD_TEXEL_SCALE
            bmesh.update_edit_mesh(o.data)
        bpy.ops.uv.select_all(action="SELECT")
        bpy.ops.uv.pack_islands(
            rotate=True,
            scale=True,
            shape_method="CONCAVE",
            margin_method="FRACTION",
            margin=margin,
        )

    _edit(objs, pack)
    for o in objs:
        me = o.data
        name = previous[o.name]
        if name and name != ATLAS_UV and me.uv_layers.get(name):
            me.uv_layers.active = me.uv_layers[name]


# --- Bake passes --------------------------------------------------------------------------


def _bake_material(image, emit_mask=False):
    """Temporary material: an active image node, optionally emitting ``fg_mask``."""
    m = bpy.data.materials.new("fg3_bake")
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    if emit_mask:
        attr = nt.nodes.new("ShaderNodeAttribute")
        attr.attribute_name = MASK_ATTR
        emit = nt.nodes.new("ShaderNodeEmission")
        nt.links.new(attr.outputs["Color"], emit.inputs["Color"])
        nt.links.new(emit.outputs[0], out.inputs[0])
    else:
        bsdf = nt.nodes.new("ShaderNodeBsdfDiffuse")
        nt.links.new(bsdf.outputs[0], out.inputs[0])
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = image
    nt.nodes.active = tex
    return m


def _assign(objs, material):
    for o in objs:
        if not o.material_slots:
            o.data.materials.append(material)
        for s in o.material_slots:
            s.material = material


def _image(name, size, fill):
    img = bpy.data.images.get(name)
    if img is not None:
        bpy.data.images.remove(img)
    img = bpy.data.images.new(name, size, size, alpha=True, float_buffer=True)
    img.colorspace_settings.name = "Non-Color"
    img.pixels.foreach_set(np.tile(np.array(fill, np.float32), size * size))
    return img


def _pixels(img):
    """Pixels of `img` as (h, w, 4) float32, row 0 = top."""
    w, h = img.size
    px = np.empty(w * h * 4, np.float32)
    img.pixels.foreach_get(px)
    return px.reshape(h, w, 4)[::-1]


def _select(active, others=()):
    view_layer = bpy.context.view_layer
    for o in view_layer.objects:
        o.select_set(False)
    for o in others:
        o.select_set(True)
    active.select_set(True)
    view_layer.objects.active = active


def _variant_bits(obj):
    attr = obj.data.attributes.get("vmask") if obj.type == "MESH" else None
    if attr is None or not len(attr.data):
        return 0
    return int(attr.data[0].value) & 63


def _shown(obj, v):
    bits = _variant_bits(obj)
    return bits == 0 or (bits >> v) & 1 == 1


def _cycles(samples):
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    scene.render.bake.margin = 0
    scene.render.bake.use_clear = False
    if scene.world is None:
        scene.world = bpy.data.worlds.new("fg3_world")
    scene.world.light_settings.distance = 0.3


def bake_figure_atlas(objs, others, hd, level, variants):
    """Bake the atlas of `objs` (``fg_atlas`` UV set); returns (h, w, 4) uint8, row 0 top.

    `others`: scene objects that occlude but are not in the atlas (the horse body).
    """
    size = ATLAS_SIZE[level]
    originals = {o.name: [s.material for s in o.material_slots] for o in objs}
    for o in objs:
        if not has_mask(o):
            set_mask(o, 1.0)
    for o in hd.values():
        o.hide_render = True
    # 1. Mask (emission of the attribute), also the coverage of the islands.
    _cycles(1)
    mask_img = _image("fg3_mask", size, (0, 0, 0, 0))
    _assign(objs, _bake_material(mask_img, emit_mask=True))
    _select(objs[0], objs)
    bpy.ops.object.bake(type="EMIT", margin=0, use_clear=False, uv_layer=ATLAS_UV)
    # 2. Ambient occlusion, variant by variant (the variants' pieces overlap).
    _cycles(48 if level == 0 else 32)
    ao_img = _image("fg3_ao", size, (1, 1, 1, 0))
    _assign(objs, _bake_material(ao_img))
    done = set()
    scene_objs = objs + others
    for v in range(max(variants, 1)):
        shown = [o for o in scene_objs if _shown(o, v)]
        targets = [o for o in objs if o in shown and o.name not in done]
        for o in scene_objs:
            o.hide_render = o not in shown
        if targets:
            extra = [
                o.name
                for o in bpy.context.view_layer.objects
                if o.type == "MESH" and not o.hide_render and o not in shown
            ]
            print("FG3 AO variant", v, "targets", len(targets), "other visible", extra)
            _select(targets[0], targets)
            bpy.ops.object.bake(type="AO", margin=0, use_clear=False, uv_layer=ATLAS_UV)
            done.update(o.name for o in targets)
    for o in scene_objs:
        o.hide_render = False
    # 3. Shape normal from the high-definition sources, piece by piece.
    _cycles(1)
    nrm_img = _image("fg3_normal", size, (0.5, 0.5, 1.0, 0.0))
    _assign(objs, _bake_material(nrm_img))
    baked = 0
    for o in objs:
        src = hd_source(o, hd)
        if src is None:
            continue
        src.hide_render = False
        _select(o, [src])
        bpy.ops.object.bake(
            type="NORMAL",
            normal_space="TANGENT",
            use_selected_to_active=True,
            cage_extrusion=0.004,
            max_ray_distance=0.012,
            margin=0,
            use_clear=False,
            uv_layer=ATLAS_UV,
        )
        src.hide_render = True
        baked += 1
    for o in objs:
        for s, m in zip(o.material_slots, originals[o.name], strict=False):
            s.material = m
        if not originals[o.name]:
            o.data.materials.clear()
    mask = _pixels(mask_img)
    ao = _pixels(ao_img)
    nrm = _pixels(nrm_img)
    for img in (mask_img, ao_img, nrm_img):
        bpy.data.images.remove(img)
    covered = mask[..., 3] > 0.5
    rgba = np.zeros((size, size, 4), np.float32)
    rgba[..., 0] = nrm[..., 0]
    rgba[..., 1] = nrm[..., 1]
    rgba[..., 2] = ao[..., 0]
    rgba[..., 3] = mask[..., 0]
    rgba, cov = tiles.dilate(rgba, covered, DILATE[level])
    rgba[~cov] = (0.5, 0.5, 1.0, 1.0)
    print(
        f"FG3 atlas lod{level} {size}px pieces={len(objs)} hd_normals={baked} "
        f"fill={covered.mean():.2f}"
    )
    return np.clip(np.round(rgba * 255.0), 0, 255).astype(np.uint8)


# --- Figure entry point -------------------------------------------------------------------


def prepare_and_bake(objs, hd, level, variants):
    """UV, source codes and atlas of a built figure; returns the atlas image or None.

    The horse body keeps its CC0 UV (source 2, shared coat map); everything else goes to
    the figure's atlas (source 1 at LOD0, 3 at LOD1). LOD2: nothing.
    """
    if level >= 2:
        return None
    meshes = [o for o in objs if o.type == "MESH" and o.data.polygons]
    horse = [o for o in meshes if o.name == "horse_body"]
    atlas = [o for o in meshes if o not in horse]
    for o in horse:
        me = o.data
        src = me.uv_layers.get("UVTex")
        if src is None:
            print("FG3 horse body without UVTex: no coat map")
            continue
        layer = me.uv_layers.get(ATLAS_UV) or me.uv_layers.new(name=ATLAS_UV)
        uvs = np.empty(len(src.data) * 2, np.float32)
        src.data.foreach_get("uv", uvs)
        layer.data.foreach_set("uv", uvs)
        me.uv_layers.active = src
        o["fg_atlas_src"] = HORSE_SRC
    for o in atlas:
        o["fg_atlas_src"] = ATLAS_SRC[level]
    atlas_unwrap(atlas, ATLAS_SIZE[level])
    return bake_figure_atlas(atlas, horse, hd, level, variants)


def discard_hd(hd):
    """Delete the high-definition copies (before the export: they are not exported)."""
    for o in hd.values():
        me = o.data
        bpy.data.objects.remove(o)
        if me.users == 0:
            bpy.data.meshes.remove(me)


# --- Textures on disk ---------------------------------------------------------------------


def tex_dir(fine_dir):
    """Folder of the FG3 maps."""
    path = os.path.join(fine_dir, "textures")
    os.makedirs(path, exist_ok=True)
    return path


def store_layer(fine_dir, level, index, count, rgba):
    """Write `rgba` as slice `index` of the level's atlas strip (file locked)."""
    size = ATLAS_SIZE[level]
    path = os.path.join(tex_dir(fine_dir), TEX_FILES[f"lod{level}"])
    with open(os.path.join(tex_dir(fine_dir), ".lock"), "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        strip = None
        if os.path.exists(path):
            strip = tiles.read_png(path)
            if strip.shape != (size * count, size, 4):
                strip = None
        if strip is None:
            strip = np.zeros((size * count, size, 4), np.uint8)
            strip[..., :] = (128, 128, 255, 255)
        strip[index * size : (index + 1) * size] = rgba
        tiles.write_png(path, strip)
        fcntl.flock(lock, fcntl.LOCK_UN)
    write_import(path, "2d_array_texture", count)


def write_import(path, importer, slices=1):
    """Godot ``.import`` settings (VRAM BC7, mipmaps, linear) for an FG3 map.

    An existing file with the same settings is kept (Godot adds its uid and paths).
    """
    params = [
        "compress/mode=2",
        "compress/high_quality=true",
        "compress/lossy_quality=0.7",
        "compress/hdr_compression=1",
        "compress/channel_pack=1",
        "mipmaps/generate=true",
        "mipmaps/limit=-1",
    ]
    if importer == "2d_array_texture":
        params += ["slices/horizontal=1", f"slices/vertical={slices}"]
    else:
        params += [
            "compress/normal_map=2",
            "roughness/mode=0",
            "process/fix_alpha_border=false",
            "process/premult_alpha=false",
            "detect_3d/compress_to=0",
        ]
    import_path = path + ".import"
    if os.path.exists(import_path):
        with open(import_path) as f:
            text = f.read()
        if f'importer="{importer}"' in text and all(p in text for p in params):
            return
    text = "\n".join(
        ["[remap]", "", f'importer="{importer}"', "", "[params]", "", *params, ""]
    )
    with open(import_path, "w") as f:
        f.write(text)


def make_detail(fine_dir):
    """Shared detail tiles (``battle_fine_tiles``)."""
    path = os.path.join(tex_dir(fine_dir), TEX_FILES["detail"])
    tiles.make_tiles(path)
    write_import(path, "2d_array_texture", len(tiles.LAYERS))


_HORSE_DONE = []


def make_horse(fine_dir):
    """Shared horse coat map from the CC0 images loaded by ``battle_fine_horse``.

    Written once per run (every mount shares it).
    """
    if _HORSE_DONE:
        return True
    path = os.path.join(tex_dir(fine_dir), TEX_FILES["horse"])
    imgs = {i.name.split(".")[0]: i for i in bpy.data.images}
    col = imgs.get("HorseMain4k00")
    nrm = imgs.get("HorseMain4k00Norm00")
    ao = imgs.get("HorseMain4k00AO00")
    if col is None or nrm is None:
        print("FG3 horse images missing: build a mounted figure first")
        return False
    occ = _pixels(ao) if ao is not None else np.ones(_pixels(col).shape, np.float32)
    rgba = tiles.horse_pack(_pixels(col), _pixels(nrm), occ, HORSE_SIZE)
    tiles.write_png(path, rgba)
    write_import(path, "texture")
    print("FG3 horse", path)
    _HORSE_DONE.append(path)
    return True
