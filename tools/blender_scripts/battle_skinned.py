"""Skinned battle figures with baked bone textures (lot V2, audit A1-18).

Run from the repository root:

    blender --background --python tools/blender_scripts/battle_skinned.py [-- options]

Options (after ``--``):
    --only <figure>[,<figure>...]   build only these figures (the rigs are always baked)
    --no-rigs                       skip the bone textures (mesh work only)
    --preview <png> <figure> <clip> <frame>[,<frame>...] [--wide]
                                    render the figure posed at clip frames (Workbench), no export

Pipeline:
1. Rigs: the Quaternius CC0 characters (62 bones, 24 actions) and horses (50 bones, 13
   actions) are imported, their actions sampled at 24 fps, and every frame's skinning matrix
   (pose * inverse bind, in Godot coordinates) written as three RGBA float texels per bone
   into ``<rig>.bones.bin``. Extra clips (bow, crossbow, riding) are built by overriding
   bones on top of an existing action; mirrored clips swap left and right.
2. Figures: modular Quaternius parts are recoloured with material codes (livery, mail,
   plate, cloth...), dressed with procedural 14th-century equipment (bassinet, kettle hat,
   great helm, tabard, bows, pikes, crossbows, shields, caparison) rigidly bound or weight
   transferred to the bones, decimated per part to three levels of detail, and written to
   ``<figure>_lod<k>.mesh.bin`` (zlib) with four bone indices and weights per vertex.
3. ``manifest.json`` lists rigs (bones, clips) and figures (files, triangles, variants).

Coordinates: everything is built in Blender world space (Z up, figures face -Y, right hand
at -X) and converted to Godot (Y up, facing +Z) on export: (x, y, z) -> (x, z, -y).
"""

import json
import math
import os
import struct
import sys
import zlib

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_skinned_equipment as equip  # noqa: E402
import battle_skinned_weapons as weapons  # noqa: E402

ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
THIRD = os.path.join(ROOT, "game", "assets", "third_party")
CHARS = os.path.join(THIRD, "characters", "quaternius_modular_men")
ANIMALS = os.path.join(THIRD, "animals", "quaternius_animated_animals")
OUT_DIR = os.path.join(ROOT, "game", "assets", "models", "battle_skinned")

FPS = 24
HUMAN_SCALE = 0.97
HORSE_SCALE = 0.44

# Blender world -> Godot: (x, y, z) -> (x, z, -y).
TO_GODOT = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))
FROM_GODOT = TO_GODOT.inverted()

# Human bones kept in the texture (fingers are folded into the wrist).
HUMAN_BONES = [
    "Root",
    "Body",
    "Hips",
    "Abdomen",
    "Torso",
    "Chest",
    "Neck",
    "Head",
    "Shoulder.L",
    "UpperArm.L",
    "LowerArm.L",
    "Wrist.L",
    "Shoulder.R",
    "UpperArm.R",
    "LowerArm.R",
    "Wrist.R",
    "UpperLeg.L",
    "LowerLeg.L",
    "Foot.L",
    "UpperLeg.R",
    "LowerLeg.R",
    "Foot.R",
]
FINGER_PREFIXES = ("Index", "Middle", "Ring", "Pinky", "Thumb")


def human_bone_alias(name):
    """Texture bone for a vertex group of the human rig (fingers -> wrist)."""
    base = name.split(".")[0]
    side = name.rsplit(".", 1)[-1] if "." in name else ""
    if base.rstrip("0123456789") in FINGER_PREFIXES:
        return "Wrist." + side
    return name


def mirror_name(name):
    """Left/right counterpart of a bone name."""
    if name.endswith(".L"):
        return name[:-2] + ".R"
    if name.endswith(".R"):
        return name[:-2] + ".L"
    return name


# --- Scene helpers ----------------------------------------------------------------------


def reset_scene():
    """Empty factory scene at the baking frame rate."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = FPS
    bpy.context.scene.render.fps_base = 1.0


def import_glb(path):
    """Import a glb; return (armature, meshes, root empties) of the new objects."""
    before = set(bpy.data.objects)
    # The glb bind pose is offset from the node rest pose: build the rest from the nodes.
    bpy.ops.import_scene.gltf(filepath=path, guess_original_bind_pose=False)
    new = [o for o in bpy.data.objects if o not in before]
    for o in list(new):
        if o.type == "MESH" and o.name.startswith("Icosphere"):
            bpy.data.objects.remove(o)
            new.remove(o)
    arm = next((o for o in new if o.type == "ARMATURE"), None)
    meshes = [o for o in new if o.type == "MESH"]
    roots = [o for o in new if o.parent is None]
    return arm, meshes, roots


def clean_name(name):
    """Material/action name without Blender's `.001` duplicate suffix."""
    head, _, tail = name.rpartition(".")
    return head if head and tail.isdigit() else name


def rest_pose(arm):
    """Clear the action and put every bone at rest."""
    if arm.animation_data:
        arm.animation_data.action = None
    for pb in arm.pose.bones:
        pb.matrix_basis.identity()
    bpy.context.view_layer.update()


def find_action(name, prefix):
    """Imported action called `prefix|name` (else `name`)."""
    for wanted in (f"{prefix}|{name}", name):
        for act in bpy.data.actions:
            if clean_name(act.name) == wanted:
                return act
    raise KeyError(name)


def set_action(arm, act):
    """Assign an action (Blender 5 slotted actions)."""
    if arm.animation_data is None:
        arm.animation_data_create()
    arm.animation_data.action = act
    if (
        getattr(arm.animation_data, "action_slot", None) is None
        and hasattr(act, "slots")
        and len(act.slots)
    ):
        arm.animation_data.action_slot = act.slots[0]


def bone_world(arm, name, posed=False):
    """World matrix of a bone (rest or current pose)."""
    if posed:
        return arm.matrix_world @ arm.pose.bones[name].matrix
    return arm.matrix_world @ arm.data.bones[name].matrix_local


# --- Rig baking -------------------------------------------------------------------------


class Rig:
    """Bones of one skinning texture.

    Entries are real bones (armature, bone name) or virtual bones (`fn(rest) -> world
    matrix`, e.g. a prop held by both hands or the bow string's nock point).
    """

    def __init__(self, name):
        """Initialise an empty rig with the given texture name."""
        self.name = name
        self.entries = []  # (arm or None, bone name or fn, key)
        self.index = {}  # key -> texture index
        self.rest = {}  # armature name -> rest world matrix; key -> virtual rest matrix
        self.clips = []  # (name, start row, frames, loop)
        self.rows = []  # one bytes row per frame

    def add(self, arm, bone, key):
        """Register a real bone (armature and bone name) under the given key."""
        self.index[key] = len(self.entries)
        self.entries.append((arm, bone, key))

    def add_virtual(self, key, fn):
        """Register a virtual bone computed by `fn(rest) -> world matrix` under the given key."""
        self.index[key] = len(self.entries)
        self.entries.append((None, fn, key))

    def capture_rest(self):
        """Store the rest world matrix of each entry for later skinning matrix computation."""
        for arm, bone, key in self.entries:
            if arm is None:
                self.rest[key] = bone(True)
            else:
                self.rest[arm.name] = arm.matrix_world.copy()

    def frame_matrices(self):
        """Current skinning matrices (Godot space), one per entry."""
        out = []
        for arm, bone, key in self.entries:
            if arm is None:
                posed = bone(False)
                bind = self.rest[key]
            else:
                posed = arm.matrix_world @ arm.pose.bones[bone].matrix
                bind = self.rest[arm.name] @ arm.data.bones[bone].matrix_local
            m = TO_GODOT @ posed @ bind.inverted() @ FROM_GODOT
            out.append(m)
        return out

    def add_frame(self, mats):
        """Pack a list of skinning matrices into one bytes row and append it."""
        row = bytearray()
        for m in mats:
            for r in range(3):
                row += struct.pack("<4f", m[r][0], m[r][1], m[r][2], m[r][3])
        self.rows.append(bytes(row))

    def begin_clip(self):
        """Return the row index where a new animation clip starts."""
        return len(self.rows)

    def end_clip(self, name, start, loop):
        """Record a finished animation clip spanning rows `start` to the current row."""
        self.clips.append((name, start, len(self.rows) - start, loop))
        print(f"CLIP {self.name}/{name} frames={len(self.rows) - start} loop={loop}")

    def mirrored(self, mats):
        """Left/right mirror of a frame (bone swap and X reflection)."""
        s = Matrix.Diagonal((-1, 1, 1, 1))
        out = []
        for _arm, _bone, key in self.entries:
            other = self.index.get(mirror_name(key), self.index[key])
            out.append(s @ mats[other] @ s)
        return out

    def write(self):
        """Write the compressed rig frames to a `.bones.bin` file in `OUT_DIR`."""
        path = os.path.join(OUT_DIR, f"{self.name}.bones.bin")
        data = b"".join(self.rows)
        with open(path, "wb") as f:
            f.write(b"CAB1")
            f.write(struct.pack("<II", len(self.entries), len(self.rows)))
            f.write(zlib.compress(data, 9))
        print(
            f"RIG {self.name} bones={len(self.entries)} frames={len(self.rows)} -> {path}"
        )

    def manifest(self):
        """Build the JSON-serialisable manifest describing this rig's texture, bones and clips."""
        return {
            "texture": f"{self.name}.bones.bin",
            "bones": [key for _a, _b, key in self.entries],
            "fps": FPS,
            "clips": {
                name: {"start": s, "frames": n, "loop": loop}
                for name, s, n, loop in self.clips
            },
        }


def sample_action(
    rig, arm, act, overrides=None, mirror=False, frames=None, before=None
):
    """Append the frames of `act` to the rig; `overrides(arm, t)` poses bones on top.

    `frames` sets the clip length (the source loops); `overrides.source_frame(i, n, first,
    last)` may remap source frames (e.g. play backwards). `before(arm, t)` runs first
    (placement of the armature, e.g. rider on the saddle).
    """
    import battle_skinned_poses as poses

    set_action(arm, act)
    first, last = (int(round(v)) for v in act.frame_range)
    length = last - first + 1
    count = frames if frames else length
    remap = getattr(overrides, "source_frame", None)
    for i in range(count):
        t = i / max(count - 1, 1)
        f = remap(i, count, first, last) if remap else first + (i % max(length, 1))
        # Overrides of the previous frame must not leak into channels the action leaves unkeyed.
        for pb in arm.pose.bones:
            pb.matrix_basis.identity()
        bpy.context.scene.frame_set(f)
        poses.reset_state()
        if before:
            before(arm, t)
        if overrides:
            overrides(arm, t)
        bpy.context.view_layer.update()
        mats = rig.frame_matrices()
        rig.add_frame(rig.mirrored(mats) if mirror else mats)


def human_clip_specs():
    """(clip, source action, loop, overrides, mirror) of the human rig."""
    import battle_skinned_poses as poses

    return [
        ("idle", "Idle", True, None, False),
        ("guard", "Idle_Sword", True, None, False),
        ("walk", "Walk", True, None, False),
        ("run", "Run", True, None, False),
        ("slash", "Sword_Slash", False, None, False),
        ("thrust", "Punch_Right", False, None, False),
        ("hit", "HitRecieve", False, None, False),
        ("death", "Death", False, None, False),
        ("death_m", "Death", False, None, True),
        ("death_knees", "Idle", False, poses.death_knees, False),
        ("death_back", "Idle", False, poses.death_back, False),
        ("knockdown", "Death", False, poses.knockdown, False),
        ("pike_idle", "Idle", True, poses.pike_hold, False),
        ("pike_walk", "Walk", True, poses.pike_hold, False),
        ("pike_level", "Idle", True, poses.pike_level, False),
        ("pike_level_walk", "Walk", True, poses.pike_level, False),
        ("pike_thrust", "Idle", False, poses.pike_thrust, False),
        ("bow_shoot", "Idle", False, poses.bow_shoot, False),
        ("bow_idle", "Idle", True, poses.bow_rest, False),
        ("bow_walk", "Walk", True, poses.bow_rest, False),
        ("xbow_shoot", "Idle", False, poses.crossbow_shoot, False),
        ("xbow_idle", "Idle", True, poses.crossbow_rest, False),
        ("xbow_walk", "Walk", True, poses.crossbow_rest, False),
        ("climb", "Idle", True, poses.climb, False),
        # Lot EP5: standard bearers and musicians (appended: earlier rows keep their place).
        ("std_idle", "Idle", True, poses.std_idle, False),
        ("std_walk", "Walk", True, poses.std_walk, False),
        ("std_run", "Run", True, poses.std_run, False),
        ("std_wave", "Idle", True, poses.std_wave, False),
        ("std_death", "Death", False, poses.std_death, False),
        ("drum_idle", "Idle", True, poses.drum_idle, False),
        ("drum_march", "Walk", True, poses.drum_march, False),
        ("drum_beat", "Idle", True, poses.drum_beat, False),
        ("horn_idle", "Idle", True, poses.horn_idle, False),
        ("horn_walk", "Walk", True, poses.horn_walk, False),
        ("horn_blow", "Idle", True, poses.horn_blow, False),
        # SG3: siege engine crews (windlass, rope, loading, rammer, pushing).
        ("crank", "Idle", True, poses.crank, False),
        ("haul", "Idle", True, poses.haul, False),
        ("load", "Idle", True, poses.load, False),
        ("swab", "Idle", True, poses.swab, False),
        ("push", "Walk", True, poses.push, False),
        # Lot EP12: wounded on the ground (not looped, still at the end) and routers who
        # threw their arms away (the shader hides the `HELD_MASK` faces while they flee).
        ("crawl", "Idle", False, poses.crawl, False),
        ("wounded_sit", "Idle", False, poses.wounded_sit, False),
        ("wounded_kneel", "Idle", False, poses.wounded_kneel, False),
        ("flee", "Run", True, poses.flee, False),
        ("flee_m", "Run", True, poses.flee, True),
    ]


def add_human_virtuals(rig, arm, prefix="", placement=None):
    """Virtual bones of a human rig: right-hand prop, bow nock and nocked arrow."""
    import battle_skinned_poses as poses

    ctx = equip.Context(arm, 0, material, bone_world)
    centre, along, up, out = equip.grip(ctx, "R")
    prop_rest = poses.prop_matrix(centre, along, up)
    wrist_r_rest = bone_world(arm, "Wrist.R")
    wrist_l_rest = bone_world(arm, "Wrist.L")
    nock_rest = wrist_l_rest.copy()
    nock_rest.translation = equip.nock_rest(ctx)
    poses.REST["prop"] = prop_rest
    poses.REST["Wrist.R"] = wrist_r_rest
    poses.REST["Wrist.L"] = wrist_l_rest
    poses.REST["Hips"] = bone_world(arm, "Hips")
    for side in ("L", "R"):
        forearm = (
            bone_world(arm, f"Wrist.{side}").to_translation()
            - bone_world(arm, f"LowerArm.{side}").to_translation()
        )
        poses.REST["forearm." + side] = forearm.normalized()

    def follow(bone, rest_bone, rest_m):
        return bone_world(arm, bone, posed=True) @ rest_bone.inverted() @ rest_m

    def prop(rest):
        if rest:
            return prop_rest
        return (
            poses.STATE["prop"]
            if poses.STATE["prop"] is not None
            else follow("Wrist.R", wrist_r_rest, prop_rest)
        )

    def nock(rest):
        if rest:
            return nock_rest
        m = follow("Wrist.L", wrist_l_rest, nock_rest)
        if poses.STATE["nock"] is not None:
            m.translation = poses.STATE["nock"]
        return m

    def arrow(rest):
        if rest:
            return nock_rest
        m = nock(False)
        if not poses.STATE["arrow"]:
            m = m @ Matrix.Diagonal((1e-4, 1e-4, 1e-4, 1.0))
        return m

    rig.add_virtual(prefix + "Prop", prop)
    rig.add_virtual(prefix + "Nock", nock)
    rig.add_virtual(prefix + "Arrow", arrow)


def bake_human_rig():
    """Bone texture of the foot soldiers."""
    reset_scene()
    arm, _meshes, roots = import_glb(os.path.join(CHARS, "adventurer.glb"))
    for r in roots:
        r.scale = (HUMAN_SCALE,) * 3
    rest_pose(arm)
    rig = Rig("human")
    for b in HUMAN_BONES:
        rig.add(arm, b, b)
    add_human_virtuals(rig, arm)
    rig.capture_rest()
    for clip, source, loop, overrides, mirror in human_clip_specs():
        start = rig.begin_clip()
        act = find_action(source, "CharacterArmature")
        frames = getattr(overrides, "frames", None) if overrides is not None else None
        sample_action(rig, arm, act, overrides=overrides, mirror=mirror, frames=frames)
        rig.end_clip(clip, start, loop)
    rig.write()
    return rig


# --- Mesh export ------------------------------------------------------------------------


def material(code, rgb, name=None):
    """Material carrying a shader code and a base colour (custom properties)."""
    key = name or f"{code}_{rgb[0]:.3f}_{rgb[1]:.3f}_{rgb[2]:.3f}"
    mat = bpy.data.materials.get(key)
    if mat is None:
        mat = bpy.data.materials.new(key)
        mat["code"] = code
        mat["rgb"] = list(rgb)
        mat.diffuse_color = (*equip.srgb_preview(rgb), 1.0)
    return mat


def recolor(obj, mapping, fallback):
    """Replace each Quaternius material by a coded material.

    `mapping["Part:Material"]` or `mapping["Material"]` = (code, linear rgb), else `fallback`.
    """
    part = clean_name(obj.name)
    for slot in obj.material_slots:
        if slot.material is None or "code" in slot.material:
            continue
        name = clean_name(slot.material.name)
        entry = (
            mapping.get(f"{part}:{name}")
            or mapping.get(name)
            or fallback.get(name)
            or (equip.C_EXACT, (0.5, 0.5, 0.5))
        )
        slot.material = material(*entry)


def weld_and_decimate(obj, target_tris):
    """Merge split vertices, then collapse-decimate to about `target_tris` triangles."""
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-6)
    bmesh.ops.triangulate(bm, faces=bm.faces)
    bm.to_mesh(me)
    bm.free()
    tris = len(me.polygons)
    if target_tris and tris > target_tris:
        mod = obj.modifiers.new("dec", "DECIMATE")
        mod.decimate_type = "COLLAPSE"
        mod.ratio = max(target_tris / tris, 0.01)
        mod.use_collapse_triangulate = True
        with bpy.context.temp_override(
            object=obj, active_object=obj, selected_objects=[obj]
        ):
            bpy.ops.object.modifier_apply(modifier=mod.name)
    return len(obj.data.polygons)


def set_face_mask(obj, mask):
    """Variant mask (bit v = shown for variant v; 0 = always) on every face."""
    me = obj.data
    attr = me.attributes.get("vmask") or me.attributes.new("vmask", "INT", "FACE")
    for i in range(len(me.polygons)):
        attr.data[i].value = mask


INFLUENCES = (4, 2, 1)  # bones per vertex at each level of detail

# Lot EP12: face flag (bit 6 of the variant mask) of the weapons and shields a foot soldier
# holds; battle_soldier_skinned.gdshader hides them when his regiment routs (`drop_arms`)
# and on the corpses of routers. Bits 0-5 stay the variant bits, bit 7 the back pavise.
HELD_MASK = 0b0100_0000
HELD_ITEMS = {
    "sword",
    "heater_shield",
    "round_shield",
    "adarga",
    "longbow",
    "crossbow",
    "pike",
    "spear",
    "bill",
    "pitchfork",
    "goedendag",
    "coustille",
    "pollaxe",
    "hand_culverin",
    "javelin",
    "pavise",
}


def held_mask(name, mask, kwargs):
    """Variant mask of an equipment piece, with `HELD_MASK` when it is dropped in a rout."""
    if name in HELD_ITEMS and not kwargs.get("back", False):
        return mask | HELD_MASK
    return mask


ATLAS_UV = "fg_atlas"
ATLAS_BITS = 11  # per coordinate; 2 more bits for the source (exact in a float32)


def pack_atlas_uv(uv, src):
    """FG3: atlas UV (Blender convention, v up) and source packed in one float.

    ``src * 2^22 + v * 2^11 + u`` with u, v quantised to 11 bits in the texture's convention
    (v down); the shader unpacks it per vertex (``CAM2``, UV2.y).
    """
    top = (1 << ATLAS_BITS) - 1
    u = min(max(int(round((uv[0] % 1.0 if uv[0] != 1.0 else 1.0) * top)), 0), top)
    v = min(max(int(round((1.0 - (uv[1] % 1.0 if uv[1] != 1.0 else 1.0)) * top)), 0), top)
    return float((src << (2 * ATLAS_BITS)) | (v << ATLAS_BITS) | u)


def export_mesh(objs, rig, alias, path, smooth_angle=50.0, influences=4):
    """Write the figure (world space, rest pose) with skinning data to `path`."""
    positions, normals, colors, uvs, bones, weights, masks = [], [], [], [], [], [], []
    atlas = []  # FG3: packed atlas UV per vertex (0 = none), written as ``CAM2``
    has_atlas = False
    indices = []
    lookup = {}
    tris = 0
    dg = bpy.context.evaluated_depsgraph_get()
    for obj in objs:
        eval_obj = obj.evaluated_get(dg)
        me = bpy.data.meshes.new_from_object(
            eval_obj, preserve_all_data_layers=True, depsgraph=dg
        )
        me.shade_smooth()
        me.set_sharp_from_angle(angle=math.radians(smooth_angle))
        me.calc_loop_triangles()
        mw = obj.matrix_world
        nm = mw.to_3x3().inverted().transposed()
        groups = {g.index: g.name for g in obj.vertex_groups}
        mask_attr = me.attributes.get("vmask")
        # FG4: optional per-vertex shade multiplying the material colour (fine horse coat).
        shade_attr = me.color_attributes.get("fg_shade")
        if shade_attr is not None and shade_attr.domain != "POINT":
            shade_attr = None
        uv_layer = me.uv_layers.active
        # FG3: optional ``fg_atlas`` UV layer (baked maps of the fine figures), source
        # code in the object property ``fg_atlas_src`` (1 figure atlas, 2 horse coat).
        atlas_layer = me.uv_layers.get(ATLAS_UV)
        atlas_src = int(obj.get("fg_atlas_src", 1)) if atlas_layer else 0
        if atlas_layer is not None and atlas_layer == uv_layer:
            uv_layer = next((u for u in me.uv_layers if u.name != ATLAS_UV), None)
        has_atlas = has_atlas or atlas_layer is not None
        corner_normals = me.corner_normals
        for tri in me.loop_triangles:
            mat = (
                obj.material_slots[tri.material_index].material
                if obj.material_slots
                else None
            )
            code = (
                int(mat["code"]) if mat is not None and "code" in mat else equip.C_EXACT
            )
            rgb = (
                list(mat["rgb"])
                if mat is not None and "rgb" in mat
                else [0.5, 0.5, 0.5]
            )
            mask = mask_attr.data[tri.polygon_index].value if mask_attr else 0
            face = []
            base_rgb = rgb
            for li in tri.loops:
                vi = me.loops[li].vertex_index
                if shade_attr is not None:
                    k = shade_attr.data[vi].color[0]
                    rgb = [c * k for c in base_rgb]
                p = TO_GODOT @ (mw @ me.vertices[vi].co)
                n = (
                    TO_GODOT.to_3x3() @ (nm @ Vector(corner_normals[li].vector))
                ).normalized()
                uv = tuple(uv_layer.data[li].uv) if uv_layer else (0.0, 0.0)
                packed = (
                    pack_atlas_uv(atlas_layer.data[li].uv, atlas_src)
                    if atlas_layer
                    else 0.0
                )
                acc = {}
                for g in me.vertices[vi].groups:
                    name = groups.get(g.group)
                    if name is None or g.weight <= 0.0:
                        continue
                    key = alias(name)
                    if key not in rig.index:
                        continue
                    acc[rig.index[key]] = acc.get(rig.index[key], 0.0) + g.weight
                top = sorted(acc.items(), key=lambda kv: -kv[1])[:influences]
                total = sum(w for _b, w in top)
                if total <= 0.0:
                    top = [(0, 1.0)]
                    total = 1.0
                top = [(b, w / total) for b, w in top] + [(0, 0.0)] * (4 - len(top))
                key = (
                    tuple(round(c, 5) for c in p),
                    tuple(round(c, 3) for c in n),
                    code,
                    tuple(round(c, 3) for c in rgb),
                    tuple(round(c, 4) for c in uv),
                    mask,
                    tuple((b, round(w, 3)) for b, w in top),
                    packed,
                )
                idx = lookup.get(key)
                if idx is None:
                    idx = len(positions)
                    lookup[key] = idx
                    positions.append(p)
                    normals.append(n)
                    colors.append((rgb[0], rgb[1], rgb[2], code))
                    uvs.append(uv)
                    bones.append([float(b) for b, _w in top])
                    weights.append([w for _b, w in top])
                    masks.append(float(mask))
                    atlas.append(packed)
                face.append(idx)
            # Godot front faces are clockwise: swap the winding of Blender's CCW triangles.
            indices.extend((face[0], face[2], face[1]))
            tris += 1
        bpy.data.meshes.remove(me)
    n = len(positions)
    raw = bytearray()
    for arr, width in (
        (positions, 3),
        (normals, 3),
        (colors, 4),
        (uvs, 2),
        (bones, 4),
        (weights, 4),
    ):
        for item in arr:
            raw += struct.pack(f"<{width}f", *item)
    raw += struct.pack(f"<{n}f", *masks)
    if has_atlas:
        raw += struct.pack(f"<{n}f", *atlas)
    raw += struct.pack(f"<{len(indices)}I", *indices)
    with open(path, "wb") as f:
        f.write(b"CAM2" if has_atlas else b"CAM1")
        f.write(struct.pack("<III", n, len(indices), len(raw)))
        f.write(zlib.compress(bytes(raw), 9))
    xs = [p[0] for p in positions]
    ys = [p[1] for p in positions]
    zs = [p[2] for p in positions]
    print(
        f"MESH {os.path.basename(path)} verts={n} tris={tris} bbox=({min(xs):.2f},{min(ys):.2f},{min(zs):.2f})-({max(xs):.2f},{max(ys):.2f},{max(zs):.2f})"
    )
    return tris


# --- Figures ----------------------------------------------------------------------------


_PART_CACHE = {}


def extract_parts(path):
    """Rest geometry of every mesh of a glb (world space, before scaling), cached.

    Importing a second glb into the same scene misplaces its skinned meshes (Blender 5 glTF
    importer), so each file is imported alone and its meshes rebuilt from plain data.
    """
    if path in _PART_CACHE:
        return _PART_CACHE[path]
    reset_scene()
    arm, meshes, _roots = import_glb(path)
    rest_pose(arm)
    parts = {}
    for m in meshes:
        me = m.data
        mw = m.matrix_world
        uv_layer = me.uv_layers.active
        parts[m.name] = {
            "verts": [tuple(mw @ v.co) for v in me.vertices],
            "polys": [tuple(p.vertices) for p in me.polygons],
            "mat_index": [p.material_index for p in me.polygons],
            "materials": [
                clean_name(s.material.name) if s.material else ""
                for s in m.material_slots
            ],
            "colors": {
                clean_name(s.material.name): tuple(
                    next(
                        (
                            n.inputs[0].default_value[:3]
                            for n in s.material.node_tree.nodes
                            if n.type == "BSDF_PRINCIPLED"
                        ),
                        (0.5, 0.5, 0.5),
                    )
                )
                for s in m.material_slots
                if s.material and s.material.use_nodes
            },
            "uvs": [tuple(d.uv) for d in uv_layer.data] if uv_layer else None,
            "groups": {g.index: g.name for g in m.vertex_groups},
            "weights": [[(g.group, g.weight) for g in v.groups] for v in me.vertices],
            "parent_bone": m.parent_bone if m.parent_type == "BONE" else "",
        }
    _PART_CACHE[path] = parts
    return parts


def create_part(name, data, arm):
    """Mesh object rebuilt from `extract_parts` data, parented to `arm` (world kept)."""
    me = bpy.data.meshes.new(name)
    me.from_pydata(data["verts"], [], data["polys"])
    for i, p in enumerate(me.polygons):
        p.material_index = data["mat_index"][i]
    if data["uvs"]:
        layer = me.uv_layers.new(name="UVMap")
        for i, uv in enumerate(data["uvs"]):
            layer.data[i].uv = uv
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    for mat_name in data["materials"]:
        mat = bpy.data.materials.get(mat_name) or bpy.data.materials.new(mat_name)
        me.materials.append(mat)
    groups = {}
    for gi, gname in data["groups"].items():
        groups[gi] = obj.vertex_groups.new(name=gname)
    for vi, ws in enumerate(data["weights"]):
        for gi, w in ws:
            groups[gi].add([vi], w, "REPLACE")
    if data["parent_bone"] and not ws:
        g = obj.vertex_groups.new(name=data["parent_bone"])
        g.add(range(len(me.vertices)), 1.0, "REPLACE")
    # Extracted vertices are in the space of the armature's (unscaled) import: follow the
    # armature's root empty (scale, placement on a saddle).
    base = arm.parent.matrix_world if arm.parent is not None else Matrix.Identity(4)
    obj.parent = arm
    obj.matrix_parent_inverse = arm.matrix_world.inverted() @ base
    return obj


def build_human(recipe, level):
    """Assemble a foot figure at `level`; returns (armature, mesh objects)."""
    sources = {}
    for file_name, part_names in recipe["parts"]:
        parts = extract_parts(os.path.join(CHARS, file_name))
        for pn in part_names:
            sources[pn] = parts[pn]
    reset_scene()
    arm, meshes, roots = import_glb(os.path.join(CHARS, "adventurer.glb"))
    for m in meshes:
        bpy.data.objects.remove(m)
    rest_pose(arm)
    parts = [create_part(name, data, arm) for name, data in sources.items()]
    for r in roots:
        r.scale = (HUMAN_SCALE,) * 3
    rest_pose(arm)
    budget = recipe["budget"][level]
    out = []
    for m in parts:
        recolor(m, recipe.get("colors", {}), equip.DEFAULT_COLORS)
        target = budget.get(clean_name(m.name), budget.get("*", 0))
        weld_and_decimate(m, target)
        set_face_mask(m, recipe.get("masks", {}).get(clean_name(m.name), 0))
        out.append(m)
    ctx = equip.Context(arm, level, material, bone_world)
    for item in recipe.get("equipment", []):
        name, mask = item[0], item[1]
        kwargs = item[2] if len(item) > 2 else {}
        builder = getattr(weapons, name, None) or getattr(equip, name)
        for obj in builder(ctx, **kwargs):
            set_face_mask(obj, held_mask(name, mask, kwargs))
            out.append(obj)
    for fn in recipe.get("post", []):
        fn(ctx, out)
    return arm, out


def export_figure(fig_name, recipe, rigs):
    """All levels of detail of one figure; returns its manifest entry."""
    rig = rigs[recipe["rig"]]
    files, tris = [], []
    for level in range(3):
        _arm, objs = build_human(recipe, level)
        name = f"{fig_name}_lod{level}.mesh.bin"
        tris.append(
            export_mesh(
                objs,
                rig,
                human_bone_alias,
                os.path.join(OUT_DIR, name),
                influences=INFLUENCES[level],
            )
        )
        files.append(name)
    entry = {
        "rig": recipe["rig"],
        "lods": files,
        "tris": tris,
        "variants": recipe.get("variants", 1),
        "style": recipe.get("style", ""),
        "noble": recipe.get("noble", False),
    }
    entry.update(pole_entry(recipe, _arm))
    return entry


def pole_entry(recipe, arm):
    """Lot EP5: `pole_top` / `pole_axis` (Godot rest space) of a standard bearer's pole.

    The tip of the pole on the virtual bone `Prop` at rest: Godot moves it with the baked
    `Prop` matrix of the frame to hang the cloth there. Empty without a `pole` recipe key.
    """
    if "pole" not in recipe:
        return {}
    fr = weapons.prop_frame(equip.Context(arm, 0, material, bone_world))
    top = TO_GODOT @ (fr[0] + fr[2] * recipe["pole"])
    axis = (TO_GODOT.to_3x3() @ fr[2]).normalized()
    return {
        "pole_top": [round(v, 5) for v in top],
        "pole_axis": [round(v, 5) for v in axis],
    }


def rig_stub(name, bones):
    """Rig index only (mesh export without re-baking)."""
    rig = Rig(name)
    for b in bones:
        rig.index[b] = len(rig.index)
    return rig


# --- Preview ----------------------------------------------------------------------------


def preview(png, fig_name, clip_source, frames, wide=False):
    """Render the posed figure (Blender armature deform) for quick checks.

    `wide` (option `--wide`): camera following the body near the ground (EP12 crawl).
    """
    import battle_skinned_figures as figures

    recipe = figures.FIGURES[fig_name]
    arm, objs = build_human(recipe, 0)
    for o in objs:
        if not any(m.type == "ARMATURE" for m in o.modifiers):
            mod = o.modifiers.new("arm", "ARMATURE")
            mod.object = arm
        if o.parent is None:
            mw = o.matrix_world.copy()
            o.parent = arm
            o.matrix_world = mw
    specs = {c[0]: c for c in human_clip_specs()}
    spec = specs.get(clip_source)
    act = find_action(spec[1] if spec else clip_source, "CharacterArmature")
    set_action(arm, act)
    scene = bpy.context.scene
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    scene.collection.objects.link(cam)
    scene.camera = cam
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.rotation_euler = (math.radians(50), 0, math.radians(30))
    scene.collection.objects.link(sun)
    if wide:
        bpy.ops.mesh.primitive_plane_add(size=12.0, location=(0.0, 0.0, 0.0))
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.display.shading.light = "STUDIO"
    scene.display.shading.color_type = "MATERIAL"
    scene.render.resolution_x = 700
    scene.render.resolution_y = 700
    first = int(act.frame_range[0])
    last = int(act.frame_range[1])
    base, ext = os.path.splitext(png)
    import battle_skinned_poses as poses

    count = getattr(spec[3], "frames", None) if spec and spec[3] else None
    count = count or (last - first + 1)
    for k, fr in enumerate(frames):
        # Same sampling as `sample_action` (frame `fr` of the baked clip, overrides reset).
        for pb in arm.pose.bones:
            pb.matrix_basis.identity()
        scene.frame_set(first + fr % max(last - first + 1, 1))
        poses.reset_state()
        if spec and spec[3]:
            spec[3](arm, fr / max(count - 1, 1))
        bpy.context.view_layer.update()
        # The render re-evaluates the action: freeze the overridden pose without it.
        bases = {pb.name: pb.matrix_basis.copy() for pb in arm.pose.bones}
        arm.animation_data.action = None
        for pb in arm.pose.bones:
            pb.matrix_basis = bases[pb.name]
        bpy.context.view_layer.update()
        for view, (loc, rot) in enumerate(
            (
                ((0.0, -4.2, 1.1), (88, 0, 0)),
                ((4.2, 0.0, 1.1), (88, 0, 90)),
            )
        ):
            cam.location = loc
            cam.rotation_euler = tuple(math.radians(a) for a in rot)
            cam.data.lens = 75
            if wide:
                # Follow the body at ground level (clips that travel or lie down).
                body = bone_world(arm, "Body", posed=True).to_translation()
                cam.location = (body.x + loc[0], body.y + loc[1], 0.55)
                cam.rotation_euler = (math.radians(84), 0, math.radians(rot[2]))
                cam.data.lens = 55
            scene.render.filepath = f"{base}_{k}_{view}{ext}"
            bpy.ops.render.render(write_still=True)
        set_action(arm, act)


# --- Main -------------------------------------------------------------------------------


def main():
    """Parse CLI args and drive rig baking, figure export or preview rendering."""
    import battle_skinned_figures as figures

    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    os.makedirs(OUT_DIR, exist_ok=True)
    if "--preview" in args:
        i = args.index("--preview")
        png, fig, clip, frames = args[i + 1 : i + 5]
        preview(png, fig, clip, [int(f) for f in frames.split(",")], "--wide" in args)
        return
    only = None
    if "--only" in args:
        only = set(args[args.index("--only") + 1].split(","))
    manifest_path = os.path.join(OUT_DIR, "manifest.json")
    manifest = {"rigs": {}, "figures": {}}
    if os.path.exists(manifest_path):
        with open(manifest_path) as f:
            manifest = json.load(f)
    rigs = {}
    if "--no-rigs" in args:
        for name, entry in manifest["rigs"].items():
            rigs[name] = rig_stub(name, entry["bones"])
    else:
        import battle_skinned_cavalry as cavalry

        human = bake_human_rig()
        rigs["human"] = human
        manifest["rigs"]["human"] = human.manifest()
        mounted = cavalry.bake_cavalry_rig()
        rigs["cavalry"] = mounted
        manifest["rigs"]["cavalry"] = mounted.manifest()
    for fig_name, recipe in figures.FIGURES.items():
        if only and fig_name not in only:
            continue
        if recipe["rig"] not in rigs:
            continue
        if recipe["rig"] == "cavalry":
            import battle_skinned_cavalry as cavalry

            manifest["figures"][fig_name] = cavalry.export_cavalry(
                fig_name, recipe, rigs["cavalry"]
            )
        else:
            manifest["figures"][fig_name] = export_figure(fig_name, recipe, rigs)
    manifest["source"] = (
        "tools/blender_scripts/battle_skinned.py (Quaternius CC0, see SOURCE.md)"
    )
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True)
    print("OK")


if __name__ == "__main__":
    main()
