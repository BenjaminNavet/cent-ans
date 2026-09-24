"""Battle figures (lot B1): soldiers and horses for `battle_soldier.gdshader`.

Run from the repository root:

    blender --background --python tools/blender/battle_figures.py

Writes one `.glb` per figure and level of detail into `game/assets/models/battle/`
(`<kind>_<variant>.glb` and `<kind>_<variant>_lod.glb`) plus `figures.json` (limb pivots).

All geometry is written in Godot coordinates (Y up, the figure faces +Z, its right hand is
at -X) and converted to Blender coordinates on the fly; the glTF exporter converts back.
Shapes are "lofts": elliptical (super-elliptical) sections along a path, smoothed by one
level of Catmull-Clark subdivision on the full level of detail.

Vertex encoding in the glb (read back by `BattleMeshes` in Godot):
- COLOR_0 = (linear colour, material code / 5), codes as in `battle_meshes.gd`;
- TEXCOORD_0 = heraldry UV (outside [0, 1]: plain livery);
- TEXCOORD_1 = (limb id, knee/hock bend weight).
"""

import json
import math
import os
import sys

import bmesh
import bpy
from mathutils import Vector

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT_DIR = os.path.join(ROOT, "game", "assets", "models", "battle")

# Limbs (CUSTOM0.x), shared with `battle_meshes.gd` and the shader.
P_BODY = 0
P_LEG_L = 1
P_LEG_R = 2
P_ARM_R = 3
P_ARM_L = 4
P_HEAD = 5
P_HORSE = 6
P_HLEG_FL = 7
P_HLEG_FR = 8
P_HLEG_BL = 9
P_HLEG_BR = 10
P_HNECK = 11
P_RIDER = 12
P_WEAPON = 13
P_CLOTH = 16
P_BOW = 17

# Material codes (COLOR.a * 5).
C_LIVERY = 0
C_TRIM = 1
C_METAL = 2
C_CLOTH = 3
C_EXACT = 4
C_ARMS = 5

# sRGB colours, as in `battle_meshes.gd`.
WHITE = (1.0, 1.0, 1.0)
SKIN = (0.80, 0.60, 0.47)
STEEL = (0.60, 0.62, 0.66)
MAIL = (0.45, 0.46, 0.48)
WOOD = (0.42, 0.29, 0.17)
WOOD_DARK = (0.26, 0.18, 0.11)
LEATHER = (0.36, 0.24, 0.14)
HOSE = (0.42, 0.36, 0.30)
GAMBESON = (0.70, 0.63, 0.50)
HORSE_COATS = [(0.36, 0.23, 0.14), (0.20, 0.14, 0.10), (0.52, 0.40, 0.28)]
HORSE_DARK = (0.12, 0.09, 0.07)
HOOF = (0.16, 0.15, 0.14)
IRON = (0.22, 0.22, 0.24)
STRING = (0.85, 0.82, 0.72)
HAIR = (0.22, 0.15, 0.09)
FLETCH = (0.9, 0.9, 0.85)
DARK = (0.05, 0.05, 0.05)

# Pivots (Godot y, z) of the animated limbs, standing and mounted (seat offset DY, DZ).
HIP_Y = 0.93
SHOULDER_Y = 1.4
HAND = Vector((0.25, 0.9, 0.12))
DY = 0.78
DZ = -0.05
HORSE_LEGS = {
    P_HLEG_FL: (0.14, 0.52, True),
    P_HLEG_FR: (-0.14, 0.52, True),
    P_HLEG_BL: (0.15, -0.6, False),
    P_HLEG_BR: (-0.15, -0.6, False),
}


def srgb_to_linear(c):
    """Converts one sRGB channel to linear (same formula as Godot's `srgb_to_linear`)."""
    return c / 12.92 if c < 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def to_blender(p):
    """Godot (x, y up, z forward) to Blender (x, y, z up); the glTF exporter undoes it."""
    return Vector((p[0], -p[2], p[1]))


def to_godot(p):
    return Vector((p[0], p[2], -p[1]))


def smoothstep(e0, e1, x):
    t = max(0.0, min(1.0, (x - e0) / (e1 - e0)))
    return t * t * (3.0 - 2.0 * t)


def const(color, code=C_EXACT):
    return lambda _p: (color, code)


class Figure:
    """Collects the Blender objects of one figure, then joins and exports them."""

    def __init__(self, name, level):
        self.name = name
        # Level of detail: 0 full (subdivided), 1 medium, 2 far (also the shadow caster).
        self.level = level
        self.lod = level >= 1
        self.far = level >= 2
        self.objects = []
        self.pivots = {}
        self.knees = {}
        # Transform applied to every piece added (rider seat, crew placement).
        self.offset = Vector((0.0, 0.0, 0.0))
        self.part_override = None

    # --- Building blocks ------------------------------------------------------------------

    def add(self, verts, faces, part, color_fn, *, subdiv=1, smooth=True, bend_fn=None,
            uv_fn=None, solidify=0.0, sharp_angle=None):
        """Adds one piece (Godot coordinates) as a Blender object with its vertex data."""
        if self.part_override is not None and part in (P_BODY, P_HEAD, P_CLOTH):
            part = self.part_override
        mesh = bpy.data.meshes.new(f"{self.name}_piece")
        mesh.from_pydata([to_blender(Vector(v) + self.offset) for v in verts], [], faces)
        mesh.update()
        obj = bpy.data.objects.new(mesh.name, mesh)
        bpy.context.scene.collection.objects.link(obj)
        if solidify > 0.0 and not self.far:
            mod = obj.modifiers.new("solidify", "SOLIDIFY")
            mod.thickness = solidify
            mod.offset = 0.0
        levels = 0 if self.lod else subdiv
        if levels > 0:
            mod = obj.modifiers.new("subsurf", "SUBSURF")
            mod.levels = levels
            mod.render_levels = levels
        if obj.modifiers:
            depsgraph = bpy.context.evaluated_depsgraph_get()
            evaluated = obj.evaluated_get(depsgraph)
            baked = bpy.data.meshes.new_from_object(evaluated)
            obj.modifiers.clear()
            obj.data = baked
            mesh = baked
        bm = bmesh.new()
        bm.from_mesh(mesh)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        bm.to_mesh(mesh)
        bm.free()
        if smooth:
            mesh.shade_smooth()
            if sharp_angle is not None:
                mesh.set_sharp_from_angle(angle=math.radians(sharp_angle))
        else:
            mesh.shade_flat()
        self._paint(mesh, part, color_fn, bend_fn, uv_fn)
        if os.environ.get("FIGURES_DEBUG"):
            import traceback

            frames = traceback.extract_stack()
            caller = next(fr for fr in reversed(frames) if fr.name not in ("add", "loft", "tube", "blob", "box", "plate"))
            tris = sum(len(p.vertices) - 2 for p in mesh.polygons)
            print(f"[piece] {self.name} {caller.name}:{caller.lineno} {tris}")
        self.objects.append(obj)
        return obj

    def _paint(self, mesh, part, color_fn, bend_fn, uv_fn):
        colors = mesh.color_attributes.new("Col", "FLOAT_COLOR", "CORNER")
        uv = mesh.uv_layers.new(name="UVMap")
        meta = mesh.uv_layers.new(name="Meta")
        for loop in mesh.loops:
            p = to_godot(mesh.vertices[loop.vertex_index].co) - self.offset
            rgb, code = color_fn(p)
            colors.data[loop.index].color = (
                srgb_to_linear(rgb[0]), srgb_to_linear(rgb[1]), srgb_to_linear(rgb[2]), code / 5.0)
            u, v = uv_fn(p) if uv_fn else (-1.0, -1.0)
            # glTF stores v flipped; Godot reads the stored value as is.
            uv.data[loop.index].uv = (u, 1.0 - v)
            bend = bend_fn(p) if bend_fn else 0.0
            meta.data[loop.index].uv = (float(part), 1.0 - bend)

    def pivot(self, part, y, z, knee=None):
        self.pivots[part] = [round(y + self.offset.y, 4), round(z + self.offset.z, 4)]
        if knee is not None:
            self.knees[part] = [round(knee[0] + self.offset.y, 4), round(knee[1] + self.offset.z, 4)]

    # --- Shapes --------------------------------------------------------------------------

    def ring_count(self, n):
        if self.far:
            return max(4, (n + 1) // 2)
        return max(4, n - 2) if self.lod else n

    def loft(self, sections, part, color_fn, *, ring=8, caps=(True, True), side=(1.0, 0.0, 0.0),
             power=2.0, closed=True, arc=(0.0, 2.0 * math.pi), **kw):
        """Lofted tube. `sections` = (centre, rx, r_pos, r_neg): `rx` along `side`, `r_pos` /
        `r_neg` on either side along `T x side` (T = path tangent). `closed=False` with `arc`
        gives an open sheet (cloth), to use with `solidify`."""
        n = self.ring_count(ring)
        if self.far and len(sections) > 4:
            sections = [sections[0]] + list(sections[2:-1:2]) + [sections[-1]]
        centres = [Vector(s[0]) for s in sections]
        verts = []
        faces = []
        side_v = Vector(side)
        rings = []
        for i, sec in enumerate(sections):
            c = centres[i]
            prev_c = centres[max(i - 1, 0)]
            next_c = centres[min(i + 1, len(sections) - 1)]
            tangent = (next_c - prev_c).normalized()
            ax = (side_v - tangent * side_v.dot(tangent)).normalized()
            ay = tangent.cross(ax).normalized()
            rx = sec[1]
            rp = sec[2] if len(sec) > 2 else rx
            rn = sec[3] if len(sec) > 3 else rp
            idx = []
            count = n if closed else n + 1
            for j in range(count):
                if closed:
                    t = arc[0] + (arc[1] - arc[0]) * (j + 0.5) / n
                else:
                    t = arc[0] + (arc[1] - arc[0]) * j / n
                ct, st = math.cos(t), math.sin(t)
                ex = math.copysign(abs(ct) ** (2.0 / power), ct)
                ey = math.copysign(abs(st) ** (2.0 / power), st)
                p = c + ax * (ex * rx) + ay * (ey * (rp if ey > 0.0 else rn))
                idx.append(len(verts))
                verts.append(p)
            rings.append(idx)
        count = len(rings[0])
        for k in range(len(rings) - 1):
            a, b = rings[k], rings[k + 1]
            for j in range(count if closed else count - 1):
                j2 = (j + 1) % count
                faces.append((a[j], a[j2], b[j2], b[j]))
        if closed and caps[0]:
            faces.append(tuple(reversed(rings[0])))
        if closed and caps[1]:
            faces.append(tuple(rings[-1]))
        return self.add(verts, faces, part, color_fn, **kw)

    def tube(self, points, radii, part, color_fn, ring=6, **kw):
        """Round tube through `points` with one radius per point."""
        sections = [(p, r) for p, r in zip(points, radii, strict=True)]
        return self.loft(sections, part, color_fn, ring=ring, **kw)

    def blob(self, centre, radii, part, color_fn, ring=8, rows=5, **kw):
        """Ellipsoid (vertical loft of circles)."""
        c = Vector(centre)
        rows = max(3, rows - 2) if self.lod else rows
        if self.far:
            rows = 3
        sections = []
        for i in range(rows + 1):
            a = -math.pi * 0.5 + math.pi * i / rows
            a = max(-math.pi * 0.5 + 0.12, min(math.pi * 0.5 - 0.12, a))
            y = c.y + math.sin(a) * radii[1]
            r = math.cos(a)
            sections.append(((c.x, y, c.z), radii[0] * r, radii[2] * r))
        return self.loft(sections, part, color_fn, ring=ring, side=(1.0, 0.0, 0.0), **kw)

    def box(self, centre, size, part, color_fn, rot_y=0.0, rot_x=0.0, **kw):
        """Box (flat shading), rotated about Y then X."""
        c = Vector(centre)
        h = Vector(size) * 0.5
        verts = []
        cy, sy = math.cos(rot_y), math.sin(rot_y)
        cx, sx = math.cos(rot_x), math.sin(rot_x)
        for i in range(8):
            p = Vector((h.x if i & 1 else -h.x, h.y if i & 2 else -h.y, h.z if i & 4 else -h.z))
            p = Vector((p.x, p.y * cx - p.z * sx, p.y * sx + p.z * cx))
            p = Vector((p.x * cy + p.z * sy, p.y, -p.x * sy + p.z * cy))
            verts.append(c + p)
        faces = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 4, 6, 2), (1, 3, 7, 5), (0, 1, 5, 4), (2, 6, 7, 3)]
        kw.setdefault("subdiv", 0)
        kw.setdefault("smooth", False)
        return self.add(verts, faces, part, color_fn, **kw)

    def plate(self, outline, origin, basis, thickness, part, front_code, back_color, *,
              bend=0.0, cols=6, rows=6, back_code=C_EXACT):
        """Two-sided curved board (shields, pavise, pennon): front in `front_code` with UV over
        the outline's bounding box, back and rim in `back_color`. `outline(x) -> (y_min, y_max)`
        over x in [-1, 1] (scaled by `basis`); `bend` curves the board backwards at its edges."""
        if self.lod:
            cols = max(3, cols // 2)
            rows = max(2, rows // 2)
        if self.far:
            cols, rows = 3, 1
        ex, ey, ez = (Vector(b) for b in basis)
        verts = []
        faces = []
        grid_front = []
        grid_back = []
        for i in range(cols + 1):
            x = -1.0 + 2.0 * i / cols
            y0, y1 = outline(x)
            col_f, col_b = [], []
            for j in range(rows + 1):
                y = y0 + (y1 - y0) * j / rows
                z = -bend * x * x
                p = Vector(origin) + ex * x + ey * y + ez * z
                normal_off = ez * (thickness * 0.5)
                col_f.append(len(verts))
                verts.append(p + normal_off)
                col_b.append(len(verts))
                verts.append(p - normal_off)
            grid_front.append(col_f)
            grid_back.append(col_b)
        for i in range(cols):
            for j in range(rows):
                f = grid_front
                b = grid_back
                faces.append((f[i][j], f[i + 1][j], f[i + 1][j + 1], f[i][j + 1]))
                faces.append((b[i][j], b[i][j + 1], b[i + 1][j + 1], b[i + 1][j]))
        # Rim.
        loop_f = [grid_front[i][0] for i in range(cols + 1)] + [grid_front[cols][j] for j in range(1, rows + 1)]
        loop_f += [grid_front[i][rows] for i in range(cols - 1, -1, -1)] + [grid_front[0][j] for j in range(rows - 1, 0, -1)]
        loop_b = [grid_back[i][0] for i in range(cols + 1)] + [grid_back[cols][j] for j in range(1, rows + 1)]
        loop_b += [grid_back[i][rows] for i in range(cols - 1, -1, -1)] + [grid_back[0][j] for j in range(rows - 1, 0, -1)]
        for k in range(len(loop_f)):
            k2 = (k + 1) % len(loop_f)
            faces.append((loop_f[k], loop_b[k], loop_b[k2], loop_f[k2]))
        origin_v = Vector(origin)
        x_len = ex.length
        y_len = ey.length
        exn = ex.normalized()
        eyn = ey.normalized()
        ezn = ez.normalized()
        y_lo = min(outline(x)[0] for x in [i / 10.0 - 1.0 for i in range(21)])
        y_hi = max(outline(x)[1] for x in [i / 10.0 - 1.0 for i in range(21)])

        def local(p):
            d = p - origin_v
            return d.dot(exn) / x_len, d.dot(eyn) / y_len, d.dot(ezn)

        def uv_fn(p):
            x, y, _z = local(p)
            return (x + 1.0) * 0.5, 1.0 - (y - y_lo) / (y_hi - y_lo)

        obj = self.add(verts, faces, part, lambda p: (WHITE, front_code), subdiv=0, smooth=True,
                       uv_fn=uv_fn, sharp_angle=50.0)
        # Back faces: paint by face normal (front = +ez).
        mesh = obj.data
        colors = mesh.color_attributes["Col"]
        uv = mesh.uv_layers["UVMap"]
        back = tuple(srgb_to_linear(c) for c in back_color) + (back_code / 5.0,)
        for poly in mesh.polygons:
            n = to_godot(poly.normal)
            if n.dot(ezn) < 0.5:
                for li in poly.loop_indices:
                    colors.data[li].color = back
                    uv.data[li].uv = (-1.0, 2.0)
        return obj


# --- Human figures -----------------------------------------------------------------------


def knee_weight(knee_y, span=0.05):
    return lambda p: 1.0 - smoothstep(knee_y - span, knee_y + span, p.y)


def legs(f, armored, hose=HOSE):
    """Legs from the hips to the shoes; knee bend weight on the shin and foot."""
    for side in (1.0, -1.0):
        part = P_LEG_L if side > 0.0 else P_LEG_R
        x = 0.1 * side
        f.pivot(part, HIP_Y, 0.0, knee=(0.5, 0.02))

        def colour(p, armored=armored):
            if p.y < 0.1:
                return LEATHER, C_EXACT
            if armored:
                return (STEEL, C_METAL) if p.y < 0.56 else (MAIL, C_METAL)
            return hose, C_CLOTH

        sections = [
            ((x * 0.9, 0.99, 0.0), 0.095, 0.1),
            ((x, 0.86, 0.01), 0.092, 0.098),
            ((x * 1.04, 0.7, 0.015), 0.078, 0.082),
            ((x * 1.06, 0.53, 0.025), 0.056, 0.06),
            ((x * 1.07, 0.42, 0.0), 0.058, 0.066, 0.058),
            ((x * 1.08, 0.27, -0.005), 0.05, 0.05),
            ((x * 1.08, 0.12, 0.0), 0.036, 0.038),
            ((x * 1.08, 0.06, 0.005), 0.042, 0.05),
        ]
        f.loft(sections, part, colour, ring=6, side=(1.0, 0.0, 0.0), bend_fn=knee_weight(0.5))
        # Foot (shoe, pointed toe).
        foot = [
            ((x * 1.08, 0.06, -0.06), 0.035, 0.035),
            ((x * 1.08, 0.055, 0.02), 0.045, 0.045),
            ((x * 1.08, 0.04, 0.12), 0.04, 0.035),
            ((x * 1.08, 0.025, 0.2), 0.015, 0.015),
        ]
        if not f.far:
            f.loft(foot, part, const(LEATHER), ring=6, side=(1.0, 0.0, 0.0), bend_fn=lambda _p: 1.0, subdiv=0)
        if armored and not f.lod:
            # Poleyn (knee cop).
            f.blob((x * 1.06, 0.53, 0.055), (0.05, 0.05, 0.035), part, const(STEEL, C_METAL), ring=6,
                   rows=3, bend_fn=knee_weight(0.5))


def torso(f, under, under_code, surcoat, skirt_to=0.55, belt=True, skirt=None):
    """Torso (mail or gambeson; livery surcoat over it), skirt, belt, neck and head."""
    # Vertical lofts: r_pos is towards the back (-Z), r_neg towards the chest (+Z).
    body = [
        ((0.0, 0.86, 0.0), 0.165, 0.105, 0.105),
        ((0.0, 0.95, 0.0), 0.162, 0.105, 0.11),
        ((0.0, 1.05, 0.0), 0.155, 0.105, 0.115),
        ((0.0, 1.18, 0.005), 0.17, 0.115, 0.13),
        ((0.0, 1.3, 0.01), 0.195, 0.12, 0.14),
        ((0.0, 1.37, 0.005), 0.205, 0.115, 0.13),
        ((0.0, 1.43, 0.0), 0.175, 0.1, 0.105),
        ((0.0, 1.48, -0.005), 0.115, 0.078, 0.078),
        ((0.0, 1.52, -0.005), 0.06, 0.055, 0.055),
    ]

    def body_fn(p):
        # Sleeveless surcoat from the belt to the base of the neck, mail or cloth elsewhere.
        if surcoat and 0.9 < p.y < 1.46:
            return WHITE, C_LIVERY
        return under, under_code

    f.loft(body, P_BODY, body_fn, ring=8, power=2.4)
    # Skirt of the surcoat (or of the gambeson), flared, following the legs a little.
    skirt_colour = const(WHITE, C_LIVERY) if surcoat else const(under, under_code)
    skirt = skirt or [
        ((0.0, 1.0, 0.0), 0.165, 0.115, 0.12),
        ((0.0, 0.85, 0.0), 0.2, 0.14, 0.15),
        ((0.0, skirt_to, 0.01), 0.25, 0.18, 0.2),
    ]
    f.loft(skirt, P_CLOTH, skirt_colour, ring=10, caps=(False, False), solidify=0.01)
    if belt and not f.far:
        f.loft([((0.0, 0.93, 0.0), 0.175, 0.12, 0.125), ((0.0, 0.98, 0.0), 0.172, 0.118, 0.122)], P_BODY,
               const(LEATHER), ring=10, caps=(False, False), subdiv=0)
        if not f.lod:
            f.box((0.0, 0.955, 0.135), (0.05, 0.045, 0.012), P_BODY, const(WHITE, C_TRIM))
    # Neck and head (face, nose, ears hidden under helmets).
    if not f.far:
        f.tube([(0.0, 1.44, 0.0), (0.0, 1.5, 0.005), (0.0, 1.57, 0.01)], [0.056, 0.054, 0.05], P_HEAD,
               const(SKIN), ring=6)
    head = [
        ((0.0, 1.515, 0.03), 0.045, 0.04),
        ((0.0, 1.55, 0.02), 0.07, 0.075, 0.08),
        ((0.0, 1.61, 0.01), 0.085, 0.095, 0.1),
        ((0.0, 1.68, 0.0), 0.09, 0.1, 0.105),
        ((0.0, 1.74, -0.005), 0.08, 0.09, 0.095),
        ((0.0, 1.78, -0.01), 0.05, 0.055, 0.06),
    ]
    f.loft(head, P_HEAD, const(SKIN), ring=8)
    if not f.lod:
        f.loft([((0.0, 1.66, 0.085), 0.012, 0.01), ((0.0, 1.62, 0.115), 0.018, 0.02), ((0.0, 1.6, 0.1), 0.014, 0.012)],
               P_HEAD, const(SKIN), ring=5, subdiv=0)


def arms(f, sleeve, sleeve_code, glove, glove_code=C_EXACT):
    """Right (-X) and left (+X) arms from the shoulder to the fist, forearm bent forward."""
    for side in (-1.0, 1.0):
        part = P_ARM_R if side < 0.0 else P_ARM_L
        f.pivot(part, SHOULDER_Y, 0.0)
        s = side
        sections = [
            ((0.13 * s, 1.44, 0.0), 0.06, 0.06),
            ((0.19 * s, 1.41, 0.0), 0.07, 0.072),
            ((0.225 * s, 1.3, -0.01), 0.058, 0.06),
            ((0.24 * s, 1.13, -0.02), 0.047, 0.05),
            ((0.245 * s, 1.04, 0.03), 0.048, 0.046),
            ((0.25 * s, 0.96, 0.085), 0.034, 0.03),
        ]
        f.loft(sections, part, const(sleeve, sleeve_code), ring=7, side=(0.0, 0.0, 1.0))
        # Fist.
        f.blob((HAND.x * s, HAND.y - 0.01, HAND.z + 0.005), (0.042, 0.05, 0.05), part,
               const(glove, glove_code), ring=6, rows=4, subdiv=0)


def helmet(f, style):
    """Helmets: open bassinet with aventail, visored (hounskull) bassinet, kettle hat, hood."""
    if style in ("bassinet", "hounskull"):
        dome = [
            ((0.0, 1.585, -0.01), 0.105, 0.112, 0.115),
            ((0.0, 1.66, -0.012), 0.112, 0.118, 0.125),
            ((0.0, 1.74, -0.02), 0.1, 0.11, 0.115),
            ((0.0, 1.8, -0.035), 0.07, 0.08, 0.085),
            ((0.0, 1.85, -0.055), 0.035, 0.04, 0.04),
            ((0.0, 1.885, -0.075), 0.004, 0.004),
        ]
        f.loft(dome, P_HEAD, const(STEEL, C_METAL), ring=10)
        # Aventail (mail cape): pushed back at face height so that the face shows.
        aventail = [
            ((0.0, 1.63, -0.06), 0.112, 0.1, 0.09),
            ((0.0, 1.56, -0.05), 0.118, 0.1, 0.1),
            ((0.0, 1.5, -0.01), 0.14, 0.12, 0.13),
            ((0.0, 1.44, 0.0), 0.2, 0.13, 0.15),
            ((0.0, 1.4, 0.0), 0.225, 0.13, 0.155),
        ]
        f.loft(aventail, P_HEAD, const(MAIL, C_METAL), ring=10, caps=(False, False))
        if style == "hounskull":
            visor = [
                ((0.0, 1.69, 0.06), 0.1, 0.07, 0.09),
                ((0.0, 1.66, 0.12), 0.075, 0.055, 0.07),
                ((0.0, 1.635, 0.19), 0.035, 0.028, 0.035),
                ((0.0, 1.625, 0.225), 0.006, 0.006),
            ]
            f.loft(visor, P_HEAD, const(STEEL, C_METAL), ring=8, side=(1.0, 0.0, 0.0))
            if not f.lod:
                for sx in (-1.0, 1.0):
                    f.box((0.045 * sx, 1.7, 0.135), (0.05, 0.012, 0.012), P_HEAD, const(DARK), rot_y=0.5 * sx)
    elif style == "kettle":
        dome = [
            ((0.0, 1.64, 0.0), 0.12, 0.12),
            ((0.0, 1.72, 0.0), 0.112, 0.112),
            ((0.0, 1.78, 0.0), 0.08, 0.08),
            ((0.0, 1.81, 0.0), 0.02, 0.02),
        ]
        f.loft(dome, P_HEAD, const(STEEL, C_METAL), ring=10)
        brim = [
            ((0.0, 1.655, 0.0), 0.12, 0.12),
            ((0.0, 1.64, 0.0), 0.2, 0.2),
            ((0.0, 1.61, 0.0), 0.225, 0.225),
        ]
        f.loft(brim, P_HEAD, const(STEEL, C_METAL), ring=12, caps=(False, False), solidify=0.008,
               subdiv=0)
        # Mail/cloth coif under the hat.
        coif = [
            ((0.0, 1.64, -0.035), 0.108, 0.1, 0.1),
            ((0.0, 1.55, -0.02), 0.115, 0.1, 0.11),
            ((0.0, 1.46, 0.0), 0.17, 0.12, 0.13),
            ((0.0, 1.42, 0.0), 0.2, 0.12, 0.14),
        ]
        f.loft(coif, P_HEAD, const(MAIL, C_METAL), ring=8, caps=(False, False))
    elif style == "hood":
        hood = [
            ((0.0, 1.58, -0.06), 0.11, 0.1, 0.1),
            ((0.0, 1.66, -0.05), 0.108, 0.1, 0.12),
            ((0.0, 1.74, -0.03), 0.1, 0.11, 0.1),
            ((0.0, 1.8, -0.04), 0.06, 0.07, 0.06),
            ((0.0, 1.82, -0.05), 0.01, 0.01),
        ]
        f.loft(hood, P_HEAD, const(WHITE, C_LIVERY), ring=8)
        cape = [
            ((0.0, 1.56, -0.05), 0.115, 0.1, 0.1),
            ((0.0, 1.49, -0.01), 0.15, 0.12, 0.13),
            ((0.0, 1.44, 0.0), 0.19, 0.13, 0.14),
            ((0.0, 1.38, 0.0), 0.235, 0.15, 0.16),
            ((0.0, 1.3, 0.0), 0.25, 0.165, 0.17),
        ]
        f.loft(cape, P_HEAD, const(WHITE, C_LIVERY), ring=10, caps=(False, False))
        # Liripipe (tail of the hood).
        f.tube([(0.0, 1.76, -0.1), (0.0, 1.66, -0.16), (0.0, 1.5, -0.2), (0.0, 1.38, -0.2)],
               [0.035, 0.03, 0.022, 0.012], P_HEAD, const(WHITE, C_LIVERY), ring=5)


def heater(f, centre, width, height, yaw, part, lean=0.0):
    """Heater shield (curved, painted with the arms), strapped to the left arm."""
    cy, sy = math.cos(yaw), math.sin(yaw)
    ex = Vector((cy, 0.0, -sy)) * (width * 0.5)
    ez = Vector((sy, 0.0, cy))

    def outline(x):
        # Straight top, sides straight for 35 % then curving to the point.
        top = height * 0.5
        ax = abs(x)
        bottom = -height * 0.5 + height * 0.65 * (1.0 - math.sqrt(max(0.0, 1.0 - ax * ax)))
        bottom = max(bottom, -height * 0.5)
        return bottom, top

    f.plate(outline, centre, (ex, Vector((0.0, 1.0, 0.0)), ez), 0.025, part, C_ARMS, WOOD_DARK,
            bend=0.04, cols=8, rows=6)


def sword(f, hand, part=P_WEAPON):
    h = Vector(hand)
    f.tube([h + Vector((0, -0.07, 0)), h + Vector((0, 0.07, 0))], [0.018, 0.018], part, const(LEATHER), ring=5)
    f.blob(h + Vector((0, -0.09, 0)), (0.025, 0.022, 0.025), part, const(WHITE, C_TRIM), ring=5, rows=3)
    f.box(h + Vector((0, 0.085, 0)), (0.2, 0.025, 0.03), part, const(STEEL, C_METAL))
    blade = [
        (h + Vector((0, 0.1, 0)), 0.026, 0.008),
        (h + Vector((0, 0.5, 0)), 0.022, 0.006),
        (h + Vector((0, 0.82, 0)), 0.012, 0.005),
        (h + Vector((0, 0.9, 0)), 0.002, 0.002),
    ]
    f.loft(blade, part, const(STEEL, C_METAL), ring=4, subdiv=0, sharp_angle=40.0)


def polearm(f, foot, length, head_len, part=P_WEAPON, shaft_r=0.02, leaf=False):
    base = Vector(foot)
    top = base + Vector((0, length, 0))
    f.tube([base, base + Vector((0, length * 0.5, 0)), top], [shaft_r, shaft_r * 0.95, shaft_r * 0.85],
           part, const(WOOD), ring=5, subdiv=0)
    if leaf:
        head = [
            (top, shaft_r * 1.1, shaft_r * 1.1),
            (top + Vector((0, head_len * 0.3, 0)), 0.035, 0.01),
            (top + Vector((0, head_len, 0)), 0.002, 0.002),
        ]
        f.loft(head, part, const(STEEL, C_METAL), ring=4, subdiv=0)
    else:
        f.tube([top, top + Vector((0, head_len, 0))], [shaft_r * 1.2, 0.002], part, const(STEEL, C_METAL),
               ring=4, subdiv=0)


def longbow(f, grip, half, depth, part=P_BOW, string=True):
    g = Vector(grip)
    f.pivot(part, g.y, g.z)
    points = []
    radii = []
    steps = 10
    for i in range(steps + 1):
        t = -1.0 + 2.0 * i / steps
        points.append(g + Vector((0.0, t * half, depth * (1.0 - t * t))))
        radii.append(0.02 * (1.0 - 0.6 * abs(t)))
    f.tube(points, radii, part, const(WOOD), ring=5, subdiv=0)
    if string and not f.lod:
        f.tube([g + Vector((0, -half, 0)), g + Vector((0, half, 0))], [0.004, 0.004], part, const(STRING),
               ring=3, subdiv=0, caps=(False, False))


def quiver(f, base, top, part=P_BODY):
    b, t = Vector(base), Vector(top)
    f.tube([b, (b + t) * 0.5, t], [0.055, 0.06, 0.065], part, const(LEATHER), ring=7, subdiv=0)
    if not f.lod:
        for k in range(5):
            a = k * 1.3
            off = Vector((math.cos(a) * 0.03, 0.0, math.sin(a) * 0.03))
            f.tube([t + off, t + off + Vector((0, 0.12, -0.01))], [0.006, 0.006], part, const(WOOD), ring=3,
                   subdiv=0)
            f.box(t + off + Vector((0, 0.1, -0.01)), (0.004, 0.07, 0.035), part, const(FLETCH), rot_y=a)


def crossbow(f, hand, part=P_WEAPON):
    h = Vector(hand)
    # Tiller (stock) forwards from the hand.
    f.box(h + Vector((0, 0.02, 0.2)), (0.055, 0.06, 0.62), part, const(WOOD))
    f.box(h + Vector((0, -0.04, -0.02)), (0.05, 0.1, 0.14), part, const(WOOD_DARK), rot_x=0.3)
    # Prod (steel bow) across the front, bent forwards.
    prod = []
    radii = []
    for i in range(9):
        t = -1.0 + 2.0 * i / 8
        prod.append(h + Vector((t * 0.34, 0.035, 0.52 + 0.06 * (1.0 - t * t))))
        radii.append(0.017 * (1.0 - 0.4 * abs(t)))
    f.tube(prod, radii, part, const(IRON, C_METAL), ring=4, subdiv=0)
    # Stirrup.
    f.tube([h + Vector((-0.05, 0.035, 0.55)), h + Vector((-0.04, 0.035, 0.65)), h + Vector((0.04, 0.035, 0.65)),
            h + Vector((0.05, 0.035, 0.55))], [0.008] * 4, part, const(IRON, C_METAL), ring=3, subdiv=0)
    if not f.lod:
        for sx in (-1.0, 1.0):
            f.tube([h + Vector((sx * 0.34, 0.035, 0.52)), h + Vector((0, 0.05, 0.3))], [0.004, 0.004], part,
                   const(STRING), ring=3, subdiv=0, caps=(False, False))


def lance(f, hand, part=P_WEAPON):
    h = Vector(hand)
    shaft = [h + Vector((0, -0.6, 0)), h + Vector((0, 0.1, 0)), h + Vector((0, 1.5, 0)), h + Vector((0, 3.0, 0))]
    f.tube(shaft, [0.028, 0.034, 0.026, 0.018], part, const(WOOD), ring=6, subdiv=0)
    # Vamplate (hand guard).
    f.tube([h + Vector((0, 0.08, 0)), h + Vector((0, 0.2, 0))], [0.08, 0.03], part, const(STEEL, C_METAL),
           ring=8, subdiv=0)
    f.tube([h + Vector((0, 3.0, 0)), h + Vector((0, 3.25, 0))], [0.022, 0.002], part, const(STEEL, C_METAL),
           ring=4, subdiv=0)
    # Pennon (livery, swallow-tailed), in the plane of the lance, pointing backwards.
    top = h + Vector((0, 2.95, 0))

    def outline(x):
        u = (x + 1.0) * 0.5
        y_top = 0.12 - 0.06 * u
        y_bot = -0.12 + 0.06 * u
        return y_bot, y_top

    ex = Vector((0.0, 0.0, -0.25))
    f.plate(outline, top + Vector((0, -0.1, -0.25)), (ex, Vector((0, 1, 0)), Vector((1, 0, 0))), 0.008,
            part, C_LIVERY, WHITE, bend=0.02, cols=4, rows=2, back_code=C_LIVERY)


def pavise(f):
    """Pavise slung on the back (Genoese), painted with the arms."""

    def outline(x):
        return -0.6, 0.6 - 0.05 * x * x

    f.plate(outline, (0.0, 1.05, -0.24), (Vector((-0.3, 0, 0)), Vector((0, 1, 0)), Vector((0, 0.12, -1)).normalized()),
            0.035, P_BODY, C_ARMS, WOOD_DARK, bend=0.05, cols=6, rows=6)


# --- Figures -----------------------------------------------------------------------------


def infantry(f, variant):
    f.pivot(P_WEAPON, 0.9, 0.12)
    f.pivot(P_CLOTH, HIP_Y, 0.0)
    if variant == 1:  # Flemish pikemen: gambeson, kettle hat, 4.5 m pike.
        legs(f, False)
        torso(f, GAMBESON, C_CLOTH, True, 0.64)
        arms(f, GAMBESON, C_CLOTH, LEATHER)
        helmet(f, "kettle")
        polearm(f, (-0.25, 0.05, 0.12), 4.45, 0.3)
    elif variant == 2:  # Urban militia: cloth coat, kettle hat, spear and shield.
        legs(f, False)
        torso(f, HOSE, C_CLOTH, True, 0.6)
        arms(f, HOSE, C_CLOTH, SKIN)
        helmet(f, "kettle")
        heater(f, (0.34, 1.02, 0.15), 0.44, 0.56, 0.45, P_ARM_L)
        polearm(f, (-0.25, 0.1, 0.12), 2.4, 0.25, leaf=True)
    else:  # Men-at-arms on foot: mail, bassinet, surcoat, sword, shield.
        legs(f, True)
        torso(f, MAIL, C_METAL, True, 0.58)
        arms(f, MAIL, C_METAL, STEEL, C_METAL)
        helmet(f, "bassinet")
        heater(f, (0.35, 1.05, 0.14), 0.46, 0.6, 0.5, P_ARM_L)
        sword(f, (-0.25, 0.9, 0.12))


def archer(f, variant):
    f.pivot(P_WEAPON, 0.9, 0.12)
    f.pivot(P_CLOTH, HIP_Y, 0.0)
    legs(f, False)
    if variant == 0:  # Longbowmen: livery jack, hood, 1.9 m bow, quiver.
        torso(f, GAMBESON, C_CLOTH, True, 0.68)
        arms(f, HOSE, C_CLOTH, SKIN)
        helmet(f, "hood")
        longbow(f, (0.25, 0.9, 0.14), 0.95, 0.16)
        quiver(f, (-0.2, 0.72, -0.13), (-0.21, 1.2, -0.17))
    else:  # Crossbowmen: gambeson, kettle hat, crossbow; pavise on the back (Genoese).
        torso(f, GAMBESON, C_CLOTH, True, 0.62)
        arms(f, GAMBESON, C_CLOTH, LEATHER)
        helmet(f, "kettle")
        crossbow(f, (-0.25, 0.93, 0.12))
        f.box((0.2, 0.86, -0.05), (0.08, 0.3, 0.14), P_BODY, const(LEATHER))
        if variant == 2:
            pavise(f)


def horse(f, coat, caparison, dark_points):
    """Horse: body (breast, barrel, croup), neck, head, ears, mane, tail, four legs with
    knee/hock bend weights, saddle and bridle; livery caparison with the arms on the flanks."""
    f.pivot(P_HNECK, 1.5, 0.6)

    def coat_fn(p):
        return coat, C_EXACT

    # Along +Z: r_pos = top line (croup, back, withers), r_neg = belly.
    body = [
        ((0.0, 1.3, -1.0), 0.09, 0.1, 0.12),
        ((0.0, 1.33, -0.93), 0.21, 0.22, 0.3),
        ((0.0, 1.35, -0.76), 0.3, 0.28, 0.38),
        ((0.0, 1.33, -0.48), 0.32, 0.26, 0.41),
        ((0.0, 1.3, -0.18), 0.32, 0.29, 0.43),
        ((0.0, 1.3, 0.14), 0.32, 0.31, 0.44),
        ((0.0, 1.32, 0.4), 0.29, 0.33, 0.43),
        ((0.0, 1.32, 0.58), 0.25, 0.33, 0.37),
        ((0.0, 1.27, 0.72), 0.19, 0.27, 0.29),
        ((0.0, 1.24, 0.8), 0.09, 0.12, 0.13),
    ]
    f.loft(body, P_HORSE, coat_fn, ring=10, side=(1.0, 0.0, 0.0), power=2.2, subdiv=1)
    neck = [
        ((0.0, 1.42, 0.5), 0.2, 0.33, 0.33),
        ((0.0, 1.58, 0.69), 0.16, 0.25, 0.25),
        ((0.0, 1.76, 0.83), 0.12, 0.19, 0.18),
        ((0.0, 1.92, 0.93), 0.09, 0.14, 0.13),
        ((0.0, 2.03, 0.99), 0.078, 0.1, 0.1),
    ]
    f.loft(neck, P_HNECK, coat_fn, ring=8, side=(1.0, 0.0, 0.0))
    head = [
        ((0.0, 2.06, 0.95), 0.065, 0.075),
        ((0.0, 2.03, 1.02), 0.092, 0.1, 0.14),
        ((0.0, 1.95, 1.13), 0.086, 0.09, 0.12),
        ((0.0, 1.84, 1.26), 0.066, 0.075, 0.075),
        ((0.0, 1.74, 1.38), 0.06, 0.07, 0.065),
        ((0.0, 1.68, 1.45), 0.056, 0.062, 0.06),
        ((0.0, 1.65, 1.48), 0.03, 0.03),
    ]

    def head_fn(p):
        # Darker muzzle.
        if p.z > 1.38:
            return tuple(c * 0.55 for c in coat), C_EXACT
        return coat, C_EXACT

    f.loft(head, P_HNECK, head_fn, ring=8, side=(1.0, 0.0, 0.0))
    for sx in (-1.0, 1.0):
        f.loft([((0.045 * sx, 2.08, 0.99), 0.025, 0.015), ((0.055 * sx, 2.15, 0.985), 0.022, 0.012),
                ((0.06 * sx, 2.21, 0.975), 0.003, 0.003)], P_HNECK, coat_fn, ring=5, subdiv=0)
        if not f.lod:
            f.blob((0.075 * sx, 1.98, 1.1), (0.016, 0.016, 0.016), P_HNECK, const(DARK), ring=5, rows=3, subdiv=0)
    # Mane along the crest, forelock.
    mane = [
        ((0.0, 2.08, 0.95), 0.02, 0.03, 0.02),
        ((0.0, 1.97, 0.87), 0.028, 0.06, 0.03),
        ((0.0, 1.82, 0.76), 0.03, 0.07, 0.04),
        ((0.0, 1.66, 0.62), 0.03, 0.065, 0.05),
        ((0.0, 1.6, 0.52), 0.02, 0.03, 0.03),
    ]
    if not f.far:
        f.loft(mane, P_HNECK, const(HORSE_DARK), ring=6, side=(1.0, 0.0, 0.0))
    # Tail.
    f.tube([(0.0, 1.55, -0.95), (0.0, 1.46, -1.06), (0.0, 1.2, -1.14), (0.0, 0.95, -1.14), (0.0, 0.72, -1.1)],
           [0.05, 0.07, 0.075, 0.06, 0.02], P_HORSE, const(HORSE_DARK), ring=6)
    # Legs.
    for part, (x, z, front) in HORSE_LEGS.items():
        if front:
            knee = (0.6, z + 0.01)
            sections = [
                ((x, 1.2, z - 0.02), 0.1, 0.16),
                ((x, 0.95, z), 0.09, 0.14, 0.1),
                ((x * 1.02, 0.78, z + 0.01), 0.065, 0.085),
                ((x * 1.02, 0.6, z + 0.01), 0.05, 0.062),
                ((x * 1.02, 0.48, z + 0.005), 0.036, 0.045),
                ((x * 1.02, 0.27, z), 0.036, 0.045),
                ((x * 1.02, 0.19, z + 0.005), 0.044, 0.05),
                ((x * 1.02, 0.11, z + 0.04), 0.034, 0.036),
                ((x * 1.02, 0.065, z + 0.06), 0.045, 0.05),
                ((x * 1.02, 0.0, z + 0.075), 0.05, 0.058),
            ]
        else:
            knee = (0.6, z - 0.17)
            sections = [
                ((x * 0.9, 1.3, z + 0.04), 0.12, 0.2),
                ((x, 1.02, z + 0.02), 0.1, 0.17, 0.14),
                ((x * 1.02, 0.84, z - 0.08), 0.06, 0.09),
                ((x * 1.02, 0.62, z - 0.17), 0.045, 0.07, 0.045),
                ((x * 1.02, 0.48, z - 0.155), 0.037, 0.05),
                ((x * 1.02, 0.26, z - 0.13), 0.037, 0.046),
                ((x * 1.02, 0.19, z - 0.125), 0.044, 0.05),
                ((x * 1.02, 0.11, z - 0.09), 0.034, 0.036),
                ((x * 1.02, 0.065, z - 0.07), 0.045, 0.05),
                ((x * 1.02, 0.0, z - 0.055), 0.05, 0.058),
            ]
        f.pivot(part, 1.2, z, knee=knee)

        def leg_fn(p, front=front):
            if p.y < 0.085:
                return HOOF, C_EXACT
            if dark_points and p.y < 0.5:
                return HORSE_DARK, C_EXACT
            return coat, C_EXACT

        f.loft(sections, part, leg_fn, ring=5, side=(1.0, 0.0, 0.0), bend_fn=knee_weight(knee[0], 0.06))
    # Saddle: seat, high cantle and pommel, girth, stirrups.
    saddle = [
        ((0.0, 1.62, -0.36), 0.18, 0.06, 0.05),
        ((0.0, 1.64, -0.1), 0.2, 0.05, 0.05),
        ((0.0, 1.66, 0.14), 0.19, 0.05, 0.05),
    ]
    f.loft(saddle, P_HORSE, const(LEATHER), ring=8, side=(1.0, 0.0, 0.0), subdiv=0)
    # Cantle and pommel: arches across the saddle.
    for z, h, w in ((-0.36, 0.2, 0.17), (0.15, 0.13, 0.14)):
        arch = [(-w, 1.64, z), (-w * 0.75, 1.64 + h * 0.8, z), (0.0, 1.64 + h, z), (w * 0.75, 1.64 + h * 0.8, z),
                (w, 1.64, z)]
        f.loft([(p, 0.035, 0.025) for p in arch], P_HORSE, const(WOOD_DARK), ring=6, side=(0.0, 0.0, 1.0),
               subdiv=0)
    for sx in (-1.0, 1.0):
        f.box((0.3 * sx, 1.3, 0.1), (0.012, 0.6, 0.04), P_HORSE, const(LEATHER), rot_x=0.1)
        if not f.lod:
            f.tube([(0.31 * sx, 1.02, 0.13), (0.31 * sx, 0.97, 0.13)], [0.035, 0.035], P_HORSE, const(IRON, C_METAL),
                   ring=5, subdiv=0)
    # Bridle: headstall, noseband, reins to the withers.
    if not f.lod:
        f.loft([((0.0, 1.8, 1.29), 0.068, 0.078), ((0.0, 1.78, 1.315), 0.066, 0.075)], P_HNECK, const(LEATHER),
               ring=8, caps=(False, False), solidify=0.01, side=(1.0, 0.0, 0.0), subdiv=0)
        f.loft([((0.0, 2.04, 1.0), 0.09, 0.1), ((0.0, 2.02, 1.035), 0.088, 0.098)], P_HNECK, const(LEATHER),
               ring=8, caps=(False, False), solidify=0.01, side=(1.0, 0.0, 0.0), subdiv=0)
        for sx in (-1.0, 1.0):
            f.tube([(0.06 * sx, 1.74, 1.34), (0.1 * sx, 1.7, 0.95), (0.14 * sx, 1.78, 0.3)], [0.008] * 3, P_HNECK,
                   const(LEATHER), ring=3, subdiv=0)
    if caparison:
        _caparison(f, body, neck, head)


def _caparison(f, body, neck, head):
    """Livery trapper: a cloth over the body wrapping the chest and the hindquarters, falling
    to mid-cannon with a wavy hem, arms on the flanks; crinet over the neck, cloth chanfron."""
    n = 10 if not f.lod else 6
    drop = 4 if not f.lod else 2
    rings = []
    verts = []
    for c, rx, rp, _rn in (body[::2] + body[-1:] if f.far else body):
        x_half = rx + 0.05
        top = c[1] + rp + 0.03
        z = c[2] + (0.03 if c[2] > 0.7 else -0.03 if c[2] < -0.9 else 0.0)
        hem = 0.74 + 0.03 * math.sin(z * 13.0)
        pts = [(-x_half, hem + (c[1] - hem) * k / drop) for k in range(drop)]
        for j in range(n + 1):
            a = math.pi * j / n
            pts.append((-math.cos(a) * x_half, c[1] + math.sin(a) * (top - c[1])))
        pts += [(x_half, c[1] - (c[1] - hem) * k / drop) for k in range(1, drop + 1)]
        idx = []
        for px, py in pts:
            idx.append(len(verts))
            verts.append((px, py, z))
        rings.append(idx)
    faces = []
    count = len(rings[0])
    for k in range(len(rings) - 1):
        ra, rb = rings[k], rings[k + 1]
        for j in range(count - 1):
            faces.append((ra[j], ra[j + 1], rb[j + 1], rb[j]))

    def uv_fn(p):
        # Arms on each flank behind the rider's leg, mirrored on the right side (facing forwards).
        size = 0.56
        u = (p.z + 0.36) / size + 0.5
        if p.x < 0.0:
            u = 1.0 - u
        v = (1.12 - p.y) / size + 0.5
        return (u, v) if abs(p.x) > 0.15 else (-1.0, -1.0)

    f.add(verts, faces, P_HORSE, const(WHITE, C_ARMS), subdiv=1, uv_fn=uv_fn)
    crinet = [(c, rx + 0.025, rp + 0.025, rn + 0.02) for c, rx, rp, rn in neck[:-1]]
    f.loft(crinet, P_HNECK, const(WHITE, C_LIVERY), ring=8, side=(1.0, 0.0, 0.0), caps=(False, False))
    chanfron = [(c, rx + 0.012, rp + 0.012, rn + 0.012) for c, rx, rp, rn in head[1:-2]]
    f.loft(chanfron, P_HNECK, const(WHITE, C_LIVERY), ring=8, side=(1.0, 0.0, 0.0), caps=(False, False))


def rider(f, under, under_code, surcoat, helmet_style, armored):
    """Seated rider: same torso, arms and helmet as on foot, raised to the saddle; thighs
    along the saddle, shins down the flanks, feet in the stirrups."""
    f.offset = Vector((0.0, DY, DZ))
    f.part_override = P_RIDER
    # Skirt spread over the saddle.
    skirt = [
        ((0.0, 0.98, 0.0), 0.18, 0.13, 0.13),
        ((0.0, 0.86, 0.02), 0.3, 0.2, 0.18),
        ((0.0, 0.72, 0.02), 0.36, 0.26, 0.22),
    ]
    torso(f, under, under_code, surcoat, skirt=skirt)
    helmet(f, helmet_style)
    f.part_override = None
    # Arms pivot at the rider's shoulders.
    saved = f.offset
    arms(f, under, under_code, STEEL if armored else LEATHER, C_METAL if armored else C_EXACT)
    f.offset = saved
    # Legs (static relative to the horse).
    leg_colour = (MAIL, C_METAL) if armored else (HOSE, C_CLOTH)
    f.offset = Vector((0.0, 0.0, 0.0))
    for sx in (-1.0, 1.0):
        thigh = [
            ((0.1 * sx, 1.72, -0.06), 0.09, 0.09),
            ((0.22 * sx, 1.64, 0.08), 0.085, 0.08),
            ((0.3 * sx, 1.52, 0.2), 0.06, 0.06),
            ((0.32 * sx, 1.32, 0.14), 0.058, 0.062),
            ((0.325 * sx, 1.1, 0.08), 0.04, 0.042),
        ]

        def leg_fn(p, sx=sx):
            if armored and p.y < 1.5:
                return STEEL, C_METAL
            return leg_colour

        f.loft(thigh, P_RIDER, leg_fn, ring=7, side=(0.0, 0.0, 1.0))
        foot = [
            ((0.325 * sx, 1.07, 0.04), 0.04, 0.04),
            ((0.325 * sx, 1.04, 0.12), 0.045, 0.035),
            ((0.325 * sx, 1.03, 0.2), 0.015, 0.015),
        ]
        f.loft(foot, P_RIDER, const(LEATHER), ring=6, side=(1.0, 0.0, 0.0))


def cavalry(f, variant):
    hand = Vector((-HAND.x, HAND.y + DY, HAND.z + DZ))
    f.pivot(P_WEAPON, hand.y, hand.z)
    if variant == 1:  # Mounted sergeants: bare horse, gambeson, kettle hat, lance.
        horse(f, HORSE_COATS[2], False, False)
        rider(f, GAMBESON, C_CLOTH, True, "kettle", False)
        lance(f, hand)
    elif variant == 2:  # Mounted archers: jack, hood, bow.
        horse(f, HORSE_COATS[0], False, True)
        rider(f, HOSE, C_CLOTH, True, "hood", False)
        longbow(f, (HAND.x, HAND.y + DY, HAND.z + DZ), 0.85, 0.14)
        f.offset = Vector((0.0, DY, DZ))
        quiver(f, (-0.2, 0.8, -0.13), (-0.21, 1.2, -0.17), P_RIDER)
        f.offset = Vector((0.0, 0.0, 0.0))
    else:  # Knights: caparison, visored bassinet, surcoat, lance, shield.
        horse(f, HORSE_COATS[1], True, True)
        rider(f, MAIL, C_METAL, True, "hounskull", True)
        f.offset = Vector((0.0, DY, DZ))
        heater(f, (0.36, 1.12, 0.15), 0.44, 0.56, 0.35, P_ARM_L)
        f.offset = Vector((0.0, 0.0, 0.0))
        lance(f, hand)


# --- Export ------------------------------------------------------------------------------


LEVEL_SUFFIX = ["", "_lod", "_far"]


def build(kind, variant, level):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    name = f"{kind}_{variant}" + LEVEL_SUFFIX[level]
    f = Figure(name, level)
    {"infantry": infantry, "archer": archer, "cavalry": cavalry}[kind](f, variant)
    objs = f.objects
    with bpy.context.temp_override(active_object=objs[0], object=objs[0], selected_objects=objs,
                                   selected_editable_objects=objs):
        bpy.ops.object.join()
    joined = objs[0]
    joined.name = name
    mesh = joined.data
    mesh.color_attributes.active_color = mesh.color_attributes["Col"]
    mesh.uv_layers.active = mesh.uv_layers["UVMap"]
    # Remove anything the join may have left over (sharp_face etc. are kept).
    for obj in list(bpy.context.scene.objects):
        obj.select_set(obj == joined)
    bpy.context.view_layer.objects.active = joined
    path = os.path.join(OUT_DIR, name + ".glb")
    bpy.ops.export_scene.gltf(
        filepath=path,
        export_format="GLB",
        use_selection=True,
        export_materials="NONE",
        export_normals=True,
        export_texcoords=True,
        export_yup=True,
        export_apply=True,
    )
    tris = sum(len(p.vertices) - 2 for p in mesh.polygons)
    print(f"[battle_figures] {name}: {tris} triangles -> {path}")
    return f, tris


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    only = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    meta = {"figures": {}}
    for kind in ("infantry", "archer", "cavalry"):
        for variant in range(3):
            name = f"{kind}_{variant}"
            if only and name not in only:
                continue
            f, tris = build(kind, variant, 0)
            _f, tris_lod = build(kind, variant, 1)
            _f, tris_far = build(kind, variant, 2)
            meta["figures"][name] = {
                "pivots": {str(k): v for k, v in sorted(f.pivots.items())},
                "knees": {str(k): v for k, v in sorted(f.knees.items())},
                "triangles": [tris, tris_lod, tris_far],
            }
    if not only:
        with open(os.path.join(OUT_DIR, "figures.json"), "w", encoding="utf-8") as out:
            json.dump(meta, out, indent=1, sort_keys=True)
            out.write("\n")


if __name__ == "__main__":
    main()
