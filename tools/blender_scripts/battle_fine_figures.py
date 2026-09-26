"""Lot FG1: fine figure recipes and builder (MakeHuman body on the fine ``human`` rig).

Every Quaternius recipe of ``battle_skinned_figures.FIGURES`` gets a fine counterpart: the
Quaternius body parts (the clothed modular men) are replaced by the fitted MakeHuman body
and garment *shells* (``battle_fine_equipment.shell``: body faces copied, relaxed and pushed
out, keeping the body's weights), chosen by the recipe's Quaternius parts (``OUTFITS``) and
coloured by its ``colors`` table; the scripted equipment (helmets, weapons, shields, jacks)
is rebuilt on the new body by the unchanged builders; the head comes in several faces (one
per variant at LOD0) with period hair and beards; hidden body faces are deleted; the figure
is exported to ``CAM1`` with colours and material codes (``battle_skinned.export_mesh``).

``infantry_0`` (and the knights' rider) wear the FG0 kit of ``battle_fine_equipment``.
"""

import os

import battle_fine_bake as fb
import battle_fine_equipment as fe
import battle_fine_gear as gear
import battle_fine_proto as fp
import battle_skinned as bs
import battle_skinned_equipment as eq
import battle_skinned_figures as figures
import battle_skinned_weapons as weapons
import bmesh
import bpy
from mathutils import Vector

# --- Faces, hair and beards -----------------------------------------------------------

FACE_COUNT = 8  # shape keys face_0..7 of fg_base_male.blend (fg_makehuman_base.FACES)
# Hair (short crop of the 1340s, over the ears) and beard per face.
FACE_BEARD = {1: True, 3: False, 5: True, 7: True}
# Faces picked per figure: rotating through the table so that neighbouring figures differ.
FACE_ORDER = [0, 3, 1, 6, 2, 5, 4, 7]

HELMS_HIDING_HAIR = {"bassinet", "great_helm", "sallet", "fine_bassinet"}

# --- Outfits --------------------------------------------------------------------------
# Quaternius part file of a recipe -> outfit (garments of the fine body).
OUTFIT_OF_PART = {
    "king.glb": "harness",
    "adventurer.glb": "tunic",
    "farmer.glb": "peasant",
    "hooded_adventurer.glb": "hooded",
}

# Triangle budgets per piece role and level of detail (0 = as modelled).
BUDGET = {
    "head": (1300, 320, 56),
    "body": (900, 240, 36),
    "legs_bare": (900, 200, 40),
    "torso": (2200, 440, 70),
    "coat": (900, 180, 30),
    "skirt": (700, 150, 28),
    "legs": (1100, 200, 36),
    "shoes": (320, 60, 16),
    "hair": (260, 70, 0),
    "beard": (160, 40, 0),
    "hood": (600, 140, 24),
    "hat": (0, 80, 30),
    "belt": (0, 60, 0),
    "scabbard": (0, 40, 12),
    "tabard": (500, 110, 16),
    "plates": (1500, 220, 0),
    "jack": (1900, 400, 60),
}
# Pieces dropped at a level of detail (budget 0 there, but kept whole at LOD0).
DROPPED = {"hair": (2,), "beard": (2,), "belt": (2,)}
# FG0 kit pieces (modelled finely): share of their LOD0 triangles kept at LOD1 / LOD2.
KIT_SHARE = (0.18, 0.04)
RIDER_FACTOR = 0.75  # riders are seen from farther and half hidden by the horse


def colour(recipe, key, default):
    """(code, linear rgb) of a Quaternius material key in the recipe (or the default)."""
    return recipe.get("colors", {}).get(key, default)


def outfit_of(recipe):
    """Outfit name of a recipe from its Quaternius body parts."""
    for file_name, part_names in recipe["parts"]:
        if any(
            n.endswith(("_Body", "Body"))
            or n.startswith(("King_", "Farmer_B", "Medieval_B"))
            for n in part_names
        ):
            return OUTFIT_OF_PART[file_name]
    return "tunic"


# --- Body -----------------------------------------------------------------------------


def append_body():
    """Append a fresh copy of the MakeHuman body and rig; returns (rig, body)."""
    with bpy.data.libraries.load(fp.MH_BLEND, link=False) as (src, dst):
        dst.objects = [n for n in src.objects if n in ("MH_Body", "MH_Rig")]
    rig = body = None
    for obj in dst.objects:
        bpy.context.scene.collection.objects.link(obj)
        if obj.type == "ARMATURE":
            rig = obj
        else:
            body = obj
    return rig, body


def set_face(body, face):
    """Give the body the head of shape key ``face_<face>`` (neck and body unchanged)."""
    keys = body.data.shape_keys
    if keys is None:
        return
    key = keys.key_blocks.get(f"face_{face}")
    names = {g.index: g.name for g in body.vertex_groups}
    head_w = []
    for v in body.data.vertices:
        w = 0.0
        for g in v.groups:
            if names[g.group] == "head":
                w = g.weight
        head_w.append(w)
    base = [v.co.copy() for v in body.data.vertices]
    target = [d.co.copy() for d in key.data] if key is not None else base
    # Align the face body on the base at the head-neck seam (age changes the height).
    seam = [i for i, w in enumerate(head_w) if 0.5 < w < 0.95]
    offset = Vector()
    if seam:
        offset = sum((base[i] - target[i] for i in seam), Vector()) / len(seam)
    body.shape_key_clear()
    for i, v in enumerate(body.data.vertices):
        w = eq.smoothstep(0.85, 1.0, head_w[i])
        if w > 0.0:
            v.co = base[i] + (target[i] + offset - base[i]) * w


def fitted_body(arm, face):
    """MakeHuman body with face `face`, fitted and weighted to `arm` (bind pose)."""
    joints = fp.q_joints(arm)
    rig, body = append_body()
    set_face(body, face)
    scale = fp.fit_scale(rig, joints)
    posed = fp.fit_pose(rig, joints, scale)
    fp.apply_pose_to_mesh(rig, body, posed)
    fp.remap_weights(body)
    # MPFB's `body` group weighs every vertex 1.0: it would win every "dominant bone"
    # lookup of the equipment builders (head box, torso rings).
    group = body.vertex_groups.get("body")
    if group is not None:
        body.vertex_groups.remove(group)
    parent_keep(body, arm)
    return body


def parent_keep(obj, arm):
    """Parent `obj` to `arm` keeping its world transform (no deformation modifier)."""
    mw = obj.matrix_world.copy()
    obj.parent = arm
    obj.matrix_world = mw


def mat(code_rgb):
    """Coded material of the export pipeline from a (code, rgb) pair."""
    return bs.material(*code_rgb)


def recolour(obj, code_rgb):
    """Every face of `obj` in one coded material."""
    obj.data.materials.clear()
    obj.data.materials.append(mat(code_rgb))
    for p in obj.data.polygons:
        p.material_index = 0


def region_mesh(body, name, keep_face):
    """Copy of `body` keeping the faces for which `keep_face(centre, bones)` holds."""
    dom = fe.dominant(body)
    obj = body.copy()
    obj.data = body.data.copy()
    obj.name = obj.data.name = name
    bpy.context.scene.collection.objects.link(obj)
    mw = obj.matrix_world
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    doomed = [
        f
        for f in bm.faces
        if not keep_face(mw @ f.calc_center_median(), {dom[v.index] for v in f.verts})
    ]
    bmesh.ops.delete(bm, geom=doomed, context="FACES")
    bmesh.ops.delete(
        bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS"
    )
    bm.to_mesh(obj.data)
    bm.free()
    return obj


def is_head(bones):
    """Face of the head region (every vertex dominated by the head)."""
    return bones == {"Head"}


def skin_materials(obj, hands, eyes=True, feet=None, drop_eyes=False):
    """Skin, eyes, hands and feet of a body piece.

    `hands` / `feet`: (code, rgb) of gloves / shoes, or None for bare skin. `drop_eyes`
    deletes the eyeballs (LOD2: a few pixels, too many triangles).
    """
    skin = (eq.C_SKIN, (0.50, 0.33, 0.24))
    obj.data.materials.clear()
    obj.data.materials.append(mat(skin))
    obj.data.materials.append(mat((eq.C_EXACT, (0.03, 0.025, 0.02))))
    obj.data.materials.append(mat(hands or skin))
    obj.data.materials.append(mat(feet or skin))
    dom = fe.dominant(obj)
    groups = [obj.vertex_groups.get(n) for n in ("helper-l-eye", "helper-r-eye")]
    eye_idx = {g.index for g in groups if g is not None}
    eye_verts = {
        v.index
        for v in obj.data.vertices
        if any(g.group in eye_idx and g.weight > 0.5 for g in v.groups)
    }
    doomed = []
    for p in obj.data.polygons:
        if eye_verts and all(i in eye_verts for i in p.vertices):
            p.material_index = 1
            if drop_eyes or not eyes:
                doomed.append(p.index)
        elif any(dom[i].startswith("Wrist") for i in p.vertices):
            p.material_index = 2
        elif any(dom[i].startswith("Foot") for i in p.vertices):
            p.material_index = 3
        else:
            p.material_index = 0
    if drop_eyes and doomed:
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bm.faces.ensure_lookup_table()
        bmesh.ops.delete(bm, geom=[bm.faces[i] for i in doomed], context="FACES")
        bmesh.ops.delete(
            bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS"
        )
        bm.to_mesh(obj.data)
        bm.free()


def hair_shell(body, lm, name, beard=False):
    """Short cropped hair (or a short full beard) as a shell of the head faces."""
    front_y = lm.head_min.y
    back_y = lm.head_max.y
    brow = lm.eye.z + 0.04
    nape = lm.chin.z + 0.005
    mouth = lm.eye.z - 0.075

    def keep_hair(c, bones):
        if not is_head(bones):
            return False
        t = min(max((c.y - front_y) / max(back_y - front_y, 1e-3), 0.0), 1.0)
        line = brow + (nape - brow) * eq.smoothstep(0.25, 0.9, t)
        # Clear of the ears and the temples' front.
        side = abs(c.x - lm.centre.x) / max((lm.head_max.x - lm.head_min.x) / 2, 1e-3)
        if side > 0.8 and c.z < lm.eye.z + 0.02 and t < 0.75:
            return False
        return c.z > line

    def keep_beard(c, bones):
        if not bones & {"Head"}:
            return False
        if c.y > lm.centre.y + 0.01:
            return False
        return lm.chin.z - 0.03 < c.z < mouth

    keep = keep_beard if beard else keep_hair
    obj = fe.shell(
        body,
        name,
        mat((eq.C_HAIR, (0.06, 0.035, 0.015))),
        keep,
        0.007 if beard else 0.006,
        relax=3,
    )
    fb.hair_mask(obj, lm, beard)  # FG3: density, thin border, open mouth
    return obj


# --- Garments -------------------------------------------------------------------------

LIMBS_OUT = {"Head", "Neck", "Wrist.L", "Wrist.R", "Foot.L", "Foot.R"}
ARMS = {"UpperArm.L", "UpperArm.R", "LowerArm.L", "LowerArm.R"}
LEGS = {"UpperLeg.L", "UpperLeg.R", "LowerLeg.L", "LowerLeg.R"}


def torso_shell(body, lm, name, material, bottom_z, offset, sleeves=True, relax=30):
    """Garment of the trunk (and arms), down to `bottom_z` on the thighs."""

    def keep(c, bones):
        if bones & LIMBS_OUT:
            return False
        if bones & {"LowerLeg.L", "LowerLeg.R"}:
            return False
        if not sleeves and bones & ARMS:
            return False
        return c.z > bottom_z

    return fe.shell(body, name, material, keep, offset, relax=relax)


def legs_shell(body, lm, name, material, top_z, offset=0.008):
    """Hose or chausses from `top_z` to the instep."""

    def keep(c, bones):
        return bool(bones & LEGS) and not bones & {"Foot.L", "Foot.R"} and c.z < top_z

    return fe.shell(body, name, material, keep, offset, relax=4)


def livery_panel(obj, lm, panel):
    """Front of the chest in `panel` (code, rgb): the commoners' livery patch."""
    obj.data.materials.append(mat(panel))
    k = len(obj.data.materials) - 1
    mw = obj.matrix_world
    chest = lm.bone["Chest"]
    for p in obj.data.polygons:
        c = mw @ p.center
        if (
            abs(c.x - chest.x) < 0.11
            and c.y < chest.y - 0.02
            and lm.waist_z - 0.02 < c.z < lm.shoulder_z - 0.03
        ):
            p.material_index = k


def sleeves_in(obj, material):
    """Arms of the garment in another material (shirt sleeves under a tunic)."""
    obj.data.materials.append(material)
    k = len(obj.data.materials) - 1
    dom = fe.dominant(obj)
    for p in obj.data.polygons:
        if any(dom[i] in ARMS for i in p.vertices):
            p.material_index = k


def skirt(lm, bvh, material, name, hem=None):
    """Split skirt from the waist to `hem` (FG0 surcoat skirt, knee) in `material`."""
    obj = fe._skirt(lm, bvh)
    obj.name = obj.data.name = name
    if hem is not None:
        # The FG0 skirt ends at the knee (+5 cm): stretch it to the wanted hem.
        waist = lm.waist_z
        k = (hem - waist) / (lm.knee_z + 0.05 - waist)
        for v in obj.data.vertices:
            v.co.z = waist + (v.co.z - waist) * k
    recolour_mat(obj, material)
    return obj


def recolour_mat(obj, material):
    """Every face of `obj` in `material`."""
    obj.data.materials.clear()
    obj.data.materials.append(material)
    for p in obj.data.polygons:
        p.material_index = 0


def shoes(body, code_rgb):
    """FG0 pointed shoes in the recipe's colour."""
    obj = fe.shoes(body)
    recolour(obj, code_rgb)
    return obj


def hood(body, lm, code_rgb):
    """Chaperon: hood over the head and a short shoulder cape, face left open."""

    def keep(c, bones):
        if bones & {"Wrist.L", "Wrist.R"} or bones & ARMS - {
            "UpperArm.L",
            "UpperArm.R",
        }:
            return False
        if bones & {"Head"}:
            # Face opening: the front of the head below the brow.
            front = c.y < lm.eye.y + 0.035
            return not (front and c.z < lm.eye.z + 0.05 and c.z > lm.chin.z - 0.03)
        if bones & {
            "Neck",
            "Chest",
            "Shoulder.L",
            "Shoulder.R",
            "UpperArm.L",
            "UpperArm.R",
        }:
            return c.z > lm.shoulder_z - 0.12
        return False

    return fe.shell(body, "hood", mat(code_rgb), keep, 0.02, relax=10)


def straw_hat(lm, code_rgb, band):
    """Wide straw hat of the peasants (the Quaternius farmer's headgear)."""
    cx, cy = lm.centre.x, lm.centre.y
    rx = (lm.head_max.x - lm.head_min.x) / 2 + 0.012
    ry = (lm.head_max.y - lm.head_min.y) / 2 + 0.012
    base = lm.eye.z + 0.045
    top = lm.top.z + 0.03
    bm = bmesh.new()
    n = 20
    brim = eq.ring(bm, Vector((cx, cy, base - 0.015)), rx + 0.11, ry + 0.11, n)
    r0 = eq.ring(bm, Vector((cx, cy, base)), rx, ry, n)
    r1 = eq.ring(bm, Vector((cx, cy, base + 0.025)), rx * 0.98, ry * 0.98, n)
    r2 = eq.ring(bm, Vector((cx, cy, top)), rx * 0.8, ry * 0.8, n)
    eq.bridge(bm, brim, r0, 0)
    eq.bridge(bm, r0, r1, 1)
    eq.bridge(bm, r1, r2, 0)
    eq.cap(bm, r2, 0)
    eq.finish(bm)
    obj = eq.to_object("straw_hat", bm, [mat(code_rgb), mat(band)])
    eq.bind_rigid(obj, "Head")
    return obj


def dress(outfit, recipe, body, lm, level, rider):
    """Garment shells of `outfit` (list of (object, budget role)) and the uncovered bones."""
    out = []
    shod = None
    bare = {"Head", "Neck", "Wrist.L", "Wrist.R"}
    thigh = lm.knee_z + (lm.bone["UpperLeg.L"].z - lm.knee_z) * 0.3
    if outfit == "harness":
        coat = colour(recipe, "King_Body:Metal", (eq.C_LIVERY, (0.85, 0.85, 0.85)))
        mail = colour(recipe, "King_Body:Blue", (eq.C_MAIL, eq.MAIL))
        if mail[0] == eq.C_MAIL:
            mail = (eq.C_MAIL, gear.MAIL)  # FG2: lighter mail (the shader darkens it)
        legs = colour(recipe, "King_Legs:Metal", (eq.C_PLATE, eq.STEEL))
        feet = colour(recipe, "King_Feet:Metal", (eq.C_PLATE, eq.STEEL))
        belt_c = colour(recipe, "King_Body:Beige", (eq.C_LEATHER, eq.LEATHER))
        hauberk = torso_shell(body, lm, "hauberk", mat(mail), thigh, 0.022, relax=40)
        out.append((hauberk, "torso"))
        top = lm.knee_z + (lm.bone["UpperLeg.L"].z - lm.knee_z) * 0.7
        # FG2: mail chausses under the leg plates (the plates are separate pieces).
        chausses = (eq.C_MAIL, gear.MAIL) if level < 2 else legs
        out.append((legs_shell(body, lm, "chausses", mat(chausses), top), "legs"))
        shod = feet
        bvh = torso_bvh(hauberk)
        if coat[0] in (eq.C_LIVERY, eq.C_ARMS):
            top_coat, skirt_obj = fe.surcoat(body, lm, hauberk, bvh)
            recolour(top_coat, coat)
            recolour(skirt_obj, coat)
            if level == 0:
                gear.hem(top_coat, 0.004)
                gear.hem(skirt_obj, 0.005)
            out += [(top_coat, "coat"), (skirt_obj, "skirt")]
        else:
            # White harness: breastplate and fauld over the mail, no coat.
            plate = torso_shell(
                hauberk, lm, "cuirass", mat(coat), lm.waist_z - 0.12, 0.012, False, 20
            )
            if level == 0:
                gear.hem(plate, 0.004)
            out.append((plate, "coat"))
        if level == 0:
            gear.hem(hauberk, 0.006)
        for obj in fe.belt(lm, bvh):
            recolour_first(obj, belt_c)
            out.append((obj, "belt"))
        gloves = (eq.C_LEATHER, (0.10, 0.06, 0.03))
    elif outfit in ("tunic", "peasant", "hooded"):
        if outfit == "tunic":
            coat = colour(recipe, "Green", (eq.C_QUILT, figures.GAMBESON))
            panel = colour(recipe, "LightGreen", (eq.C_LIVERY, (0.9, 0.9, 0.9)))
            hose = colour(recipe, "Adventurer_Legs:Brown2", (eq.C_CLOTH, figures.HOSE))
            feet = colour(recipe, "Adventurer_Legs:Brown", (eq.C_LEATHER, eq.LEATHER))
            belt_c = colour(recipe, "Gold", (eq.C_LEATHER, (0.12, 0.08, 0.04)))
            sleeves = None
            hem = 0.14
        elif outfit == "peasant":
            coat = colour(recipe, "Farmer_Body:LightBlue", (eq.C_CLOTH, figures.RUSSET))
            panel = None
            sleeves = colour(recipe, "Farmer_Body:Beige", (eq.C_CLOTH, eq.LINEN))
            hose = colour(recipe, "Farmer_Pants:LightBlue", (eq.C_CLOTH, figures.HOSE))
            feet = colour(recipe, "Farmer_Feet:Brown", (eq.C_LEATHER, eq.LEATHER))
            belt_c = (eq.C_LEATHER, (0.10, 0.06, 0.03))
            hem = 0.03
        else:
            coat = colour(
                recipe, "Medieval_Body:Black", (eq.C_CLOTH, (0.1, 0.08, 0.06))
            )
            panel = None
            sleeves = colour(recipe, "Medieval_Body:Metal", (eq.C_MAIL, eq.MAIL))
            hose = colour(
                recipe, "Medieval_Legs:Black", (eq.C_CLOTH, (0.09, 0.07, 0.05))
            )
            feet = (eq.C_LEATHER, (0.08, 0.05, 0.025))
            belt_c = colour(
                recipe, "Medieval_Body:LightBrown", (eq.C_LEATHER, (0.16, 0.09, 0.04))
            )
            hem = 0.03
        offset = 0.018 if coat[0] == eq.C_QUILT else 0.012
        hip_z = lm.bone["UpperLeg.L"].z - 0.03
        tunic = torso_shell(body, lm, "tunic", mat(coat), hip_z, offset, relax=30)
        if panel is not None and panel[0] in (eq.C_LIVERY, eq.C_ARMS):
            livery_panel(tunic, lm, panel)
        if sleeves is not None:
            sleeves_in(tunic, mat(sleeves))
        out.append((tunic, "torso"))
        bvh = torso_bvh(tunic)
        tunic_skirt = skirt(lm, bvh, mat(coat), "tunic_skirt", lm.knee_z + hem)
        if level == 0:
            gear.hem(tunic, 0.005)
            gear.hem(tunic_skirt, 0.005)
        out.append((tunic_skirt, "skirt"))
        if hose[0] == eq.C_SKIN:
            bare |= LEGS | {"Foot.L", "Foot.R"}
        else:
            top = lm.bone["UpperLeg.L"].z
            out.append((legs_shell(body, lm, "hose", mat(hose), top, 0.005), "legs"))
            shod = feet
        for obj in fe.belt(lm, bvh):
            recolour_first(obj, belt_c)
            out.append((obj, "belt"))
        gloves = None
    else:
        raise KeyError(outfit)
    shoe_colour = None
    if shod is not None and level == 2:
        bare |= {"Foot.L", "Foot.R"}  # LOD2: the body's feet in the shoe colour
        shoe_colour = shod
    elif shod is not None:
        out.append((shoes(body, shod), "shoes"))
    for obj, _role in out:
        parent_keep(obj, body.parent)
    return out, bare, gloves, shoe_colour


def recolour_first(obj, code_rgb):
    """Replace the first material slot of `obj` (leather of the FG0 belt)."""
    obj.data.materials[0] = mat(code_rgb)


def torso_bvh(obj):
    """BVH of a garment without its sleeves (hanging arms reach the belt's height)."""
    from mathutils.bvhtree import BVHTree

    dom = fe.dominant(obj)
    mw = obj.matrix_world
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.transform(mw)
    doomed = [
        f
        for f in bm.faces
        if any(dom[v.index] in ARMS | {"Wrist.L", "Wrist.R"} for v in f.verts)
    ]
    bmesh.ops.delete(bm, geom=doomed, context="FACES")
    tree = BVHTree.FromBMesh(bm)
    bm.free()
    return tree


def trim(body, keep_bones):
    """Delete the body faces entirely under the garments (none of `keep_bones`)."""
    dom = fe.dominant(body)
    bm = bmesh.new()
    bm.from_mesh(body.data)
    doomed = [
        f for f in bm.faces if not any(dom[v.index] in keep_bones for v in f.verts)
    ]
    bmesh.ops.delete(bm, geom=doomed, context="FACES")
    bmesh.ops.delete(
        bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS"
    )
    bm.to_mesh(body.data)
    bm.free()


# --- Variants -------------------------------------------------------------------------

# Figures wearing the FG0 kit (bassinet and draped aventail, sword XVI, heater shield).
FINE_KIT = {"infantry_0", "cavalry_0"}


def covers(mask, variant):
    """True if a piece with variant `mask` is shown for `variant` (0 = always)."""
    return (mask & 0b0011_1111) == 0 or bool(mask & (1 << variant))


def variant_mask(variants, shown):
    """Variant mask of a piece shown for the variants in `shown` (0 if all)."""
    if len(shown) == variants:
        return 0
    return sum(1 << v for v in shown)


def hides_hair(name, kwargs):
    """Headgear that hides the hair."""
    return name in HELMS_HIDING_HAIR


def hides_beard(name, kwargs):
    """Headgear that hides the chin (great helm, aventail, visor, bevor)."""
    if name == "great_helm":
        return True
    if name == "bassinet":
        return kwargs.get("aventail", True) or kwargs.get("visor", False)
    if name == "sallet":
        return kwargs.get("bevor", False)
    return False


def figure_index(name):
    """Stable index of a figure (spreads the faces over the figures)."""
    return sorted(figures.FIGURES).index(name)


def face_of(name, variant):
    """Face (shape key index) of a figure's variant."""
    return FACE_ORDER[(figure_index(name) * 3 + variant) % FACE_COUNT]


def headgear_visibility(recipe):
    """Per variant: (hair shown, beard allowed)."""
    variants = recipe.get("variants", 1)
    hair = [True] * variants
    beard = [True] * variants
    for item in recipe.get("equipment", []):
        name, mask = item[0], item[1]
        kwargs = item[2] if len(item) > 2 else {}
        for v in range(variants):
            if not covers(mask, v):
                continue
            if hides_hair(name, kwargs):
                hair[v] = False
            if hides_beard(name, kwargs):
                beard[v] = False
    hood_mask = recipe.get("masks", {}).get("Medieval_Head")
    if hood_mask is not None:
        for v in range(variants):
            if covers(hood_mask, v):
                hair[v] = False
    return hair, beard


# --- Figure ---------------------------------------------------------------------------


def budget(role, level, mounted):
    """Triangle budget of a piece role at `level` (0 = keep as built)."""
    target = BUDGET.get(role, (0, 0, 0))[level]
    if mounted and target:
        target = int(target * RIDER_FACTOR)
    return target


def decimate(obj, target):
    """Weld and collapse-decimate `obj` to about `target` triangles (0 = weld only).

    ``weld_and_decimate`` never goes below 1 % in one pass (the MakeHuman pieces start at
    ~10 k triangles): repeat until the target is reached or nothing collapses any more.
    """
    if target:
        fb.capture(obj)  # FG3: high-definition source of the baked normal
    tris = bs.weld_and_decimate(obj, target)
    while target and tris > target * 1.1:
        before = tris
        tris = bs.weld_and_decimate(obj, target)
        if tris >= before * 0.97:
            break
    return tris


def fine_kit_item(name, kwargs, ctx, lm, bvhs):
    """FG0 piece replacing a Quaternius builder for `FINE_KIT` figures (else None)."""
    if name == "bassinet" and kwargs.get("aventail", True) and not kwargs.get("visor"):
        helm, frame = fe.bassinet(lm)
        return helm + fe.aventail(lm, frame, bvhs[0], extra=tuple(bvhs[1:]))
    if name == "sword":
        return fe.sword(ctx, fist=fp.FIST.get("R"))
    if name == "heater_shield":
        return fe.heater_shield(ctx)
    return None


def fine_tabard(garments, lm, length=0.34, colour=(1.0, 1.0, 1.0)):
    """Livery tabard draped on the garments: front and back panels, open at the sides.

    Replaces the flat Quaternius panels (``battle_skinned_weapons.tabard``): shells of the
    torso garment and of its skirt within a band of the chest's width, `length` below the
    hips like the original.
    """
    chest = lm.bone["Chest"]
    bottom = lm.bone["Hips"].z - length
    top_z = lm.shoulder_z - 0.04
    out = []
    for obj, role in garments:
        if role not in ("torso", "skirt"):
            continue

        def keep(c, bones, role=role):
            if bones & ARMS or bones & {"Wrist.L", "Wrist.R", "Head", "Neck"}:
                return False
            half = 0.15 + 0.04 * eq.smoothstep(lm.waist_z, bottom, c.z)
            if abs(c.x - chest.x) > half or c.z < bottom:
                return False
            # Over the shoulders the band narrows to a yoke around the neck opening.
            return c.z < top_z or abs(c.x - chest.x) < 0.11

        piece = fe.shell(
            obj,
            f"tabard_{role}",
            mat((eq.C_LIVERY, colour)),
            keep,
            0.008,
            relax=6,
        )
        if len(piece.data.polygons):
            gear.hem(piece, 0.004)  # FG2: cloth thickness at the hems
            parent_keep(piece, obj.parent)
            out.append(piece)
        else:
            bpy.data.objects.remove(piece)
    return out


# FG2: pieces that cover the garments under them (their hidden faces are deleted).
COVERS = (
    "jack",
    "brigandine",
    "tabard",
    "aventail",
    "bevor",
    "limb_plates",
    "surcoat",
    "cuirass",
    "hauberk",
    "tunic",
)


# Garments dropped whole under a piece worn by every variant (the slits show the hose).
REPLACED_BY = {"tunic_skirt": "jack_skirt"}


def hide_covered(garments, equipment, variants):
    """Delete the garment faces covered in every variant by another piece (FG2)."""
    pieces = [(o, 0) for o, _r in garments] + [
        (o, m) for o, m, _k in equipment if o.name.startswith(COVERS)
    ]
    trees = {
        o.name: gear.world_tree(o) for o, _m in pieces if o.name.startswith(COVERS)
    }

    def outers(inner, v, only=COVERS):
        return [
            trees[o.name]
            for o, m in pieces
            if o is not inner
            and o.name in trees
            and o.name.startswith(only)
            and covers(m, v)
        ]

    removed = 0
    for inner, _role in garments:
        if inner.name.startswith("belt"):
            continue
        groups = [outers(inner, v) for v in range(variants)]
        before = len(inner.data.polygons)
        removed += gear.cull_hidden(inner, groups)
        replaced = any(
            o.name.startswith(REPLACED_BY.get(inner.name, "-")) and m & 0b0011_1111 == 0
            for o, m in pieces
        )
        if replaced or len(inner.data.polygons) < before * 0.25:
            # Mostly hidden: the leftovers are slivers showing through slits and hems.
            removed += len(inner.data.polygons)
            inner.data.clear_geometry()
    for obj, mask, _k in equipment:
        if not obj.name.startswith(COVERS):
            continue
        groups = [
            outers(obj, v, ("tabard", "limb_plates"))
            for v in range(variants)
            if covers(mask, v)
        ]
        removed += gear.cull_hidden(obj, groups)
    print(f"HIDDEN faces removed: {removed}")


# Triangle caps of a figure per level of detail (FG2 brief: on foot LOD0 12 k, LOD1
# 2 000, LOD2 350; FG5: LOD1 1 350, LOD2 260, since the LOD0 only draws the soldiers within
# ~12 m and LOD1 takes over from there); riders (the horse is FG4's) get a rider share.
TRI_CAP = (11900, 1350, 260)
RIDER_CAP = (9000, 1000, 180)
# Decimation weight per piece: the faces keep more than the rest.
CUT_WEIGHT = {"head": 0.35, "hair": 0.6, "beard": 0.6}


# FG2: budget roles of the equipment pieces modelled from the body (shells).
GEAR_ROLES = (
    ("jack_skirt", "skirt"),
    ("jack", "jack"),
    ("brigandine", "coat"),
    ("tabard", "tabard"),
)


def gear_role(name):
    """Budget role of an equipment piece (None: kept as built)."""
    for prefix, role in GEAR_ROLES:
        if name.startswith(prefix):
            return role
    return None


def fit_budget(objs, level, mounted):
    """Decimate the pieces proportionally until the figure fits its cap (FG2)."""
    cap = (RIDER_CAP if mounted else TRI_CAP)[level]
    for _round in range(4):
        tris = {o.name: fp._tris(o) for o in objs}
        excess = sum(tris.values()) - cap
        if excess <= 0:
            return
        weights = {
            o.name: CUT_WEIGHT.get(o.name.split("_")[0], 1.0) * tris[o.name]
            for o in objs
            if tris[o.name] > 24
        }
        wsum = sum(weights.values()) or 1.0
        for o in objs:
            if o.name in weights:
                cut = excess * 1.05 * weights[o.name] / wsum
                floor = max(12, int(tris[o.name] * 0.4))
                decimate(o, max(floor, int(tris[o.name] - cut)))


def build_figure(fig_name, level):
    """Fine figure `fig_name` at `level`; returns (armature, objects, recipe)."""
    import battle_fine as bf
    import battle_skinned_cavalry as cav
    import battle_skinned_poses as poses

    recipe = figures.FIGURES[fig_name]
    mounted = recipe["rig"] == "cavalry"
    horse = []
    if mounted:
        import battle_fine_cavalry as fc

        bf.use_fine_mount()
        # FG4: the fine horse, its harness and bards replace the Quaternius horse.
        cav.build_cavalry(
            {
                **recipe,
                "parts": [],
                "equipment": [],
                "horse_equipment": [],
                "horse_budget": [0, 0, 0],
            },
            level,
        )
        horse = fc.horse_and_harness(fig_name, recipe, level, poses.RIDE["mount"])
        arm = poses.RIDE["mount"].rarm
    else:
        arm, _meshes = bf.load_fine_human()
    variants = recipe.get("variants", 1)
    faces = [face_of(fig_name, v) for v in range(variants)] if level == 0 else []
    base_face = face_of(fig_name, 0)
    body = fitted_body(arm, base_face)
    lm = fe.Landmarks(body, arm)
    outfit = outfit_of(recipe)
    garments, bare, gloves, shoe_colour = dress(
        outfit, recipe, body, lm, level, mounted
    )
    fine = fig_name in FINE_KIT
    if fine:
        gloves = (eq.C_LEATHER, (0.10, 0.06, 0.03))
    bvhs = [torso_bvh(o) for o, role in garments if role in ("torso", "coat", "skirt")]
    ctx = eq.Context(arm, level, bs.material, bs.bone_world)
    kit_gear = gear.Gear(ctx, lm, bvhs, fig_name, mounted)
    kit_gear.body = body
    kit_gear.garments = garments
    era = gear.HARNESS_ERA.get(fig_name)
    if outfit == "harness" and era and level < 2:
        # FG2: arm and leg plates over the mail, in the harness colour of the recipe.
        plate_c = colour(recipe, "King_Legs:Metal", (eq.C_PLATE, eq.STEEL))[1]
        # Cuisses only where no coat skirt hides the thighs (white harness).
        bare_thighs = colour(recipe, "King_Body:Metal", (eq.C_LIVERY,))[0] == eq.C_PLATE
        for obj in gear.limb_harness(kit_gear, body, era, plate_c, bare_thighs):
            parent_keep(obj, arm)
            fb.gear_mask("limb_plates", [obj])
            garments.append((obj, "plates"))
        if era == "late":
            gloves = (eq.C_PLATE, plate_c)  # plate gauntlets
    equipment = []
    for item in recipe.get("equipment", []):
        name, mask = item[0], item[1]
        kwargs = item[2] if len(item) > 2 else {}
        objs = gear.build(name, kwargs, kit_gear)
        kit = "gear" if objs is not None else None
        if objs is None and fine:
            objs = fine_kit_item(name, kwargs, ctx, lm, bvhs)
            kit = "kit" if objs is not None else None
        if name == "tabard":
            objs = fine_tabard(garments, lm, **kwargs)
        if objs is None:
            builder = (
                (getattr(cav, name, None) if mounted else None)
                or getattr(weapons, name, None)
                or getattr(eq, name)
            )
            objs = builder(ctx, **kwargs)
            for obj in objs:
                gear.unwrap(obj)  # FG2: per-piece UVs for the baked maps of FG3
        m = mask if mounted else bs.held_mask(name, mask, kwargs)
        fb.gear_mask(name, objs)
        for obj in objs:
            equipment.append((obj, m, kit))
    if fine:
        for obj in fe.scabbard(lm):
            gear.unwrap(obj)
            equipment.append((obj, 0, "kit"))
    # Heads: one face per variant at LOD0, the variant-0 face below.
    hair_ok, beard_ok = headgear_visibility(recipe)
    heads = []
    if level == 0 and len(set(faces)) > 1:
        for v, face in enumerate(faces):
            src = body if face == base_face else fitted_body(arm, face)
            head = region_mesh(src, f"head_{v}", lambda c, b: is_head(b))
            parent_keep(head, arm)
            heads.append((head, 1 << v))
            hlm = fe.Landmarks(src, arm)
            fb.skin_mask(head, hlm)
            if hair_ok[v]:
                heads.append((hair_shell(src, hlm, f"hair_{v}"), 1 << v))
            if beard_ok[v] and FACE_BEARD.get(face, False):
                heads.append((hair_shell(src, hlm, f"beard_{v}", beard=True), 1 << v))
            if src is not body:
                bpy.data.objects.remove(src)
    else:
        head = region_mesh(body, "head_0", lambda c, b: is_head(b))
        parent_keep(head, arm)
        fb.skin_mask(head, lm)
        heads.append((head, 0))
        if level < 2:
            shown = [v for v in range(variants) if hair_ok[v]]
            if shown:
                heads.append(
                    (hair_shell(body, lm, "hair"), variant_mask(variants, shown))
                )
            shown = [
                v
                for v in range(variants)
                if beard_ok[v] and FACE_BEARD.get(base_face, False)
            ]
            if shown:
                heads.append(
                    (
                        hair_shell(body, lm, "beard", beard=True),
                        variant_mask(variants, shown),
                    )
                )
    extra = []
    masks = recipe.get("masks", {})
    if outfit == "hooded" and "Medieval_Head" in masks:
        hood_c = colour(
            recipe, "Medieval_Head:DarkBrown", (eq.C_CLOTH, (0.18, 0.11, 0.05))
        )
        extra.append((hood(body, lm, hood_c), masks["Medieval_Head"], "hood"))
    if outfit == "peasant" and "Farmer_Head" in masks:
        straw = colour(recipe, "Farmer_Head:Beige", (eq.C_CLOTH, figures.STRAW))
        band = colour(recipe, "Farmer_Head:Red", (eq.C_CLOTH, (0.25, 0.05, 0.03)))
        hat = straw_hat(lm, straw, band)
        gear.unwrap(hat)
        extra.append((hat, masks["Farmer_Head"], "hat"))
    # FG2: faces of the garments hidden under other pieces (jack, plates, tabard...).
    hide_covered(garments, equipment, variants)
    # Body without the head (separate) nor the faces under the garments.
    trim(body, bare - {"Head"})
    out = []
    skin_materials(body, gloves, eyes=False, feet=shoe_colour)
    body_role = "legs_bare" if bare >= LEGS else "body"
    if level == 2:
        # One piece with the head: the neck seam welds, the decimation goes further.
        head = heads.pop(0)[0]
        skin_materials(head, None, drop_eyes=True)
        body = fe.join([body, head], "body")
        decimate(body, budget(body_role, level, mounted) + budget("head", 2, mounted))
    else:
        decimate(body, budget(body_role, level, mounted))
    bs.set_face_mask(body, 0)
    out.append(body)
    head_count = sum(1 for o, _m in heads if o.name.startswith("head"))
    for obj, mask in heads:
        role = obj.name.split("_")[0]
        if role == "head":
            skin_materials(obj, None)
        target = budget(role, level, mounted)
        if role == "head" and head_count > 2:
            target = max(700, min(target, int(2800 / head_count)))
        decimate(obj, target)
        bs.set_face_mask(obj, mask)
        out.append(obj)
    for obj, role in garments:
        if level in DROPPED.get(role, ()) or not obj.data.polygons:
            bpy.data.objects.remove(obj)
            continue
        decimate(obj, budget(role, level, mounted))
        bs.set_face_mask(obj, 0)
        out.append(obj)
    for obj, mask, role in extra:
        parent_keep(obj, arm)
        decimate(obj, budget(role, level, mounted))
        bs.set_face_mask(obj, mask)
        out.append(obj)
    for obj, mask, kit in equipment:
        target = 0
        role = gear_role(obj.name)
        if role == "drop":
            bpy.data.objects.remove(obj)
            continue
        if role is not None:
            target = budget(role, level, mounted)
        elif kit == "kit" and level:
            target = max(16, int(fp._tris(obj) * KIT_SHARE[level - 1]))
        decimate(obj, target)
        bs.set_face_mask(obj, mask)
        out.append(obj)
    fit_budget(out, level, mounted)
    if mounted:
        for obj in out:
            cav._rename_groups(obj, "R:")
    print(
        f"FINE {fig_name} lod{level} outfit={outfit} faces={faces or [base_face]} "
        f"tris={sum(fp._tris(o) for o in horse + out)}"
    )
    return arm, horse + out, recipe


def export_figure(fig_name, rigs, bake=False):
    """Every LOD of a fine figure into ``battle_fine``; returns its manifest entry.

    `bake` (lot FG3): LOD0 and LOD1 get their atlas (``battle_fine_bake``), written as a
    layer of the texture strips, and ``CAM2`` meshes carrying the atlas UV.
    """
    import battle_fine as bf
    import battle_skinned_cavalry as cav
    import battle_skinned_poses as poses

    recipe = figures.FIGURES[fig_name]
    rig = rigs[recipe["rig"]]
    alias = cav.cavalry_alias if recipe["rig"] == "cavalry" else bs.human_bone_alias
    files, tris = [], []
    arm = None
    layer = list(figures.FIGURES).index(fig_name)
    for level in range(3):
        if bake and level < 2:
            fb.capture_begin()
        arm, objs, _recipe = build_figure(fig_name, level)
        if bake and level < 2:
            hd = fb.capture_end()
            atlas = fb.prepare_and_bake(objs, hd, level, recipe.get("variants", 1))
            fb.discard_hd(hd)
            fb.store_layer(bf.FINE_DIR, level, layer, len(figures.FIGURES), atlas)
            if recipe["rig"] == "cavalry":
                fb.make_horse(bf.FINE_DIR)
        name = f"{fig_name}_lod{level}.mesh.bin"
        tris.append(
            bs.export_mesh(
                objs,
                rig,
                alias,
                os.path.join(bf.FINE_DIR, name),
                influences=bs.INFLUENCES[level],
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
        "fine": True,
    }
    if bake:
        entry["atlas_layer"] = layer
    if recipe["rig"] == "cavalry":
        arm = poses.RIDE["mount"].rarm
    entry.update(bs.pole_entry(recipe, arm))
    return entry


def build_all(manifest, only=None, bake=False):
    """Build every fine figure (or those in `only`) and record them in `manifest`."""
    rigs = {
        name: bs.rig_stub(name, entry["bones"])
        for name, entry in manifest["rigs"].items()
    }
    for fig_name in figures.FIGURES:
        if only and fig_name not in only:
            continue
        manifest["figures"][fig_name] = export_figure(fig_name, rigs, bake)


# --- Pose check -----------------------------------------------------------------------

CHECKS = [
    # (figure, clip, fractions, camera eye, target)
    ("infantry_0", "idle", (0.0,), (0.0, -3.2, 1.2), (0, 0, 0.95)),
    ("infantry_0", "slash", (0.4,), (1.8, -2.6, 1.5), (0, 0, 1.0)),
    ("archer_0", "bow_shoot", (0.1, 0.5, 0.6), (-2.2, -1.2, 1.6), (0, 0, 1.3)),
    ("archer_1", "xbow_shoot", (0.1, 0.5), (1.8, -2.4, 1.4), (0, 0, 1.0)),
    ("infantry_1", "pike_level", (0.0,), (2.2, -2.4, 1.5), (0, -0.6, 1.0)),
    ("infantry_1", "pike_idle", (0.0,), (1.8, -2.6, 1.5), (0, 0, 1.2)),
]


def _pose_objects(arm, objs):
    """Deform the figure's objects by the armature in Blender (renders only)."""
    for o in objs:
        if o.parent is None or o.parent.type != "ARMATURE":
            parent_keep(o, arm)
        if not any(m.type == "ARMATURE" for m in o.modifiers):
            mod = o.modifiers.new("arm", "ARMATURE")
            mod.object = o.parent


def check_poses(out, only=None):
    """Render the computed poses on the fine figures into `out` (visual check)."""
    os.makedirs(out, exist_ok=True)
    for fig_name, clip, fracs, eye, target in CHECKS:
        if only and fig_name not in only:
            continue
        arm, objs, _recipe = build_figure(fig_name, 0)
        for o in objs:
            attr = o.data.attributes.get("vmask")
            for d in attr.data if attr else ():
                d.value &= (
                    0b0011_1111  # part flags (held arms, pavise) are not variants
                )
        objs = fp.apply_variant(objs, 0)
        _pose_objects(arm, objs)
        fp.PROBE["rig"] = fp.probe_rig(arm)
        fp.setup_workbench((520, 640))
        cam = fp.camera()
        for k, frac in enumerate(fracs):
            fp.pose_clip(arm, clip, frac)
            fp.follow_prop(objs)
            fp.look_at(cam, eye, target, 45)
            fp.render(os.path.join(out, f"{fig_name}_{clip}_{k}.png"))
        bs.rest_pose(arm)
