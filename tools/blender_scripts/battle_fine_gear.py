"""Lot FG2: fine equipment of the battle figures (helmets, armour pieces, weapons, shields).

Every builder of ``GEAR`` replaces the V2 builder of the same name
(``battle_skinned_equipment`` / ``battle_skinned_weapons`` / ``battle_skinned_cavalry``) for
the fine figures of ``battle_fine_figures``. A builder takes a `Gear` (the V2 builder
context, the fitted body's landmarks, the garment BVH trees, the level of detail) and the
recipe's keyword arguments, and returns mesh objects in world space (bind pose) bound to the
rig's bones, with the coded materials of the export pipeline (``battle_skinned.material``).

Conventions shared with V2: two-handed weapons are built in the ``Prop`` frame
(``battle_skinned_weapons.prop_frame``: origin in the right fist, Y towards the tip), shields
on the left forearm, helmets rigid on ``Head``. The figures face -Y, their right is -X.
Painted faces (``C_ARMS``) keep the heraldry UV (0-1 over the face); every other face gets a
per-piece unwrap (``unwrap``) for the baked maps of FG3.

Helmets are surfaces of revolution around the head (``revolve``): at each angle a profile
function gives the points from the inner rim, over the rolled edge, up to the crown; the
face is at ``sin(a) = -1``.
"""

import math

import battle_fine_equipment as fe
import battle_skinned_equipment as eq
import bmesh
import bpy
from mathutils import Vector

GEAR = {}

# Colours (linear) of the fine pieces. Mail is lighter than V2's: the ring pattern of the
# shader darkens it by a third and the metallic look loses the rest without reflections
# (« camail trop sombre », FG0).
STEEL = (0.52, 0.53, 0.55)
STEEL_DARK = (0.36, 0.36, 0.38)
MAIL = (0.44, 0.44, 0.45)
BRASS = (0.55, 0.40, 0.14)
IRON = (0.22, 0.22, 0.23)
SLIT = (0.012, 0.011, 0.01)  # sights and breaths: the dark inside of the helmet
LINING = (0.30, 0.22, 0.14)  # quilted lining seen at the helmet's rim


class Gear:
    """What a fine builder needs: V2 context, body landmarks, garment BVHs, figure."""

    def __init__(self, ctx, lm, bvhs, fig_name, mounted):
        """Store the builder inputs (`ctx.level` is the level of detail)."""
        self.ctx = ctx
        self.lm = lm
        self.bvhs = bvhs
        self.fig = fig_name
        self.mounted = mounted
        self.body = None  # fitted body (untrimmed), set by the figure builder
        self.garments = []  # (object, budget role) of the outfit

    @property
    def level(self):
        """Level of detail (0 full, 1 medium, 2 far)."""
        return self.ctx.level

    def seg(self, full, medium, far):
        """Value for the current level of detail."""
        return (full, medium, far)[self.ctx.level]

    def mat(self, code, rgb):
        """Coded material of the export pipeline."""
        return self.ctx.material(code, rgb)


def gear(name):
    """Register a fine builder under the V2 builder `name`."""

    def wrap(fn):
        GEAR[name] = fn
        return fn

    return wrap


def build(name, kwargs, g):
    """Fine pieces of the V2 item `name` (None when there is no fine builder)."""
    fn = GEAR.get(name)
    if fn is None:
        return None
    objs = fn(g, **kwargs)
    if objs is not None:
        for obj in objs:
            unwrap(obj)
    return objs


# --- Mesh helpers -------------------------------------------------------------------------


def revolve(bm, n, profile, apex=None, mat_of=None, closed=True):
    """Surface through the profiles `profile(a)` (same point count) at `n` angles.

    Consecutive profile points are bridged across neighbouring angles (outward normals when
    the profile runs from the rim upwards and the angles turn counter-clockwise seen from
    above); `apex` closes the last row with a fan. `mat_of(a, row)` picks the material of
    the face between profile points `row` and `row + 1` around angle `a` (mid-face).
    Returns the grid (list of rows, each a list of `n` vertices).
    """
    count = n if closed else n + 1
    step = 2 * math.pi / n if closed else 1.0 / n
    cols = []
    for i in range(count):
        a = -math.pi / 2 + i * step if closed else i * step
        cols.append([bm.verts.new(p) for p in profile(a)])
    rows = len(cols[0])
    last = n if closed else n
    for i in range(last):
        j = (i + 1) % count
        a_mid = -math.pi / 2 + (i + 0.5) * step if closed else (i + 0.5) * step
        for r in range(rows - 1):
            quad = (cols[i][r], cols[j][r], cols[j][r + 1], cols[i][r + 1])
            if len(set(quad)) < 4:
                continue
            f = bm.faces.new(quad)
            f.material_index = mat_of(a_mid, r) if mat_of else 0
        if apex is not None:
            f = bm.faces.new((cols[i][-1], cols[j][-1], apex))
            f.material_index = mat_of(a_mid, rows - 1) if mat_of else 0
    return [[cols[i][r] for i in range(count)] for r in range(rows)]


def sweep(bm, path, radius, sides, mat=0, closed=False, caps=True):
    """Tube of `sides` sides along the polyline `path` (radius: number or list)."""
    m = len(path)
    radii = radius if isinstance(radius, (list, tuple)) else [radius] * m
    rings = []
    prev_u = None
    for k, p in enumerate(path):
        if closed:
            t = path[(k + 1) % m] - path[k - 1]
        else:
            t = path[min(k + 1, m - 1)] - path[max(k - 1, 0)]
        t.normalize()
        if prev_u is None:
            helper = Vector((0, 0, 1)) if abs(t.z) < 0.9 else Vector((1, 0, 0))
            u = t.cross(helper).normalized()
        else:
            u = (prev_u - t * prev_u.dot(t)).normalized()
        v = t.cross(u).normalized()
        prev_u = u
        rings.append(
            [
                bm.verts.new(
                    p
                    + (
                        u * math.cos(2 * math.pi * s / sides)
                        + v * math.sin(2 * math.pi * s / sides)
                    )
                    * radii[k]
                )
                for s in range(sides)
            ]
        )
    last = m if closed else m - 1
    for k in range(last):
        eq.bridge(bm, rings[k], rings[(k + 1) % m], mat)
    if caps and not closed:
        eq.cap(bm, rings[0], mat, flip=False)
        eq.cap(bm, rings[-1], mat, flip=True)
    return rings


def rivet(bm, p, normal, r=0.005, mat=0, sides=4):
    """Round-headed rivet: a low pyramid of `sides` faces standing on `p` along `normal`."""
    n = normal.normalized()
    helper = Vector((0, 0, 1)) if abs(n.z) < 0.9 else Vector((1, 0, 0))
    u = n.cross(helper).normalized()
    v = n.cross(u)
    base = [
        bm.verts.new(
            p
            - n * 0.001
            + (
                u * math.cos(2 * math.pi * s / sides)
                + v * math.sin(2 * math.pi * s / sides)
            )
            * r
        )
        for s in range(sides)
    ]
    top = bm.verts.new(p + n * r * 0.7)
    for s in range(sides):
        f = bm.faces.new((base[s], base[(s + 1) % sides], top))
        f.material_index = mat
        f.normal_update()
        if f.normal.dot(n) < 0:
            f.normal_flip()


def orient_out(bm, centre):
    """Flip the faces of `bm` that look towards `centre` (convex shells around the head)."""
    for f in bm.faces:
        f.normal_update()
        if f.normal.dot(f.calc_center_median() - centre) < 0:
            f.normal_flip()


def unwrap(obj):
    """Per-piece UV unwrap of the faces that are not painted (``C_ARMS`` keep theirs).

    Smart projection into the 0-1 square of the piece (one island set per piece, same
    orientation rules everywhere): FG3 packs the pieces into its atlases.
    """
    me = obj.data
    if not me.polygons:
        return
    painted = {
        i
        for i, m in enumerate(me.materials)
        if m is not None and int(m.get("code", -1)) == eq.C_ARMS
    }
    keep = None
    if me.uv_layers:
        layer = me.uv_layers.active
        keep = {
            li: tuple(layer.data[li].uv)
            for p in me.polygons
            if p.material_index in painted
            for li in p.loop_indices
        }
    else:
        me.uv_layers.new(name="UVMap")
    view_layer = bpy.context.view_layer
    for o in view_layer.objects:
        o.select_set(False)
    view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bm = bmesh.from_edit_mesh(me)
    for f in bm.faces:
        f.select = f.material_index not in painted
    bmesh.update_edit_mesh(me)
    bpy.ops.uv.smart_project(angle_limit=math.radians(55), island_margin=0.01)
    bpy.ops.object.mode_set(mode="OBJECT")
    obj.select_set(False)
    if keep:
        layer = me.uv_layers.active
        for li, uv in keep.items():
            layer.data[li].uv = uv


def finish_object(name, bm, materials, bone=None, weigh=None):
    """Mesh object from `bm`, bound rigidly to `bone` or by `weigh`."""
    obj = eq.to_object(name, bm, materials)
    if bone is not None:
        eq.bind_rigid(obj, bone)
    elif weigh is not None:
        eq.bind_by(obj, weigh)
    return obj


# --- Head frame ---------------------------------------------------------------------------


class HeadFrame:
    """Ellipse of the skull (centre, half-axes) and the face's heights, from the landmarks."""

    def __init__(self, lm, pad=0.0):
        """Measure the head of `lm`; `pad` widens the ellipse (padding under the helmet)."""
        self.lm = lm
        self.cx = (lm.head_min.x + lm.head_max.x) / 2
        self.cy = (lm.head_min.y + lm.head_max.y) / 2 + 0.005
        self.rx = (lm.head_max.x - lm.head_min.x) / 2 + pad
        self.ry = (lm.head_max.y - lm.head_min.y) / 2 + pad * 1.4
        self.top = lm.top.z
        self.eye = lm.eye.z
        self.chin = lm.chin.z
        self.centre = Vector((self.cx, self.cy, (self.top + self.chin) / 2))

    def at(self, a, k, z, out=0.0, dy=0.0):
        """Point at angle `a`, `k` times the ellipse radius plus `out` metres, height `z`."""
        c, s = math.cos(a), math.sin(a)
        d = Vector((self.rx * c, self.ry * s, 0.0))
        n = Vector((c / max(self.rx, 1e-3), s / max(self.ry, 1e-3), 0.0)).normalized()
        return Vector((self.cx, self.cy + dy, z)) + d * k + n * out


def front(a):
    """How much angle `a` faces forwards (1 at the face, 0 on the sides and back)."""
    return max(0.0, -math.sin(a))


def back(a):
    """How much angle `a` faces backwards."""
    return max(0.0, math.sin(a))


def dome(t):
    """Height and radius factors of a round skull at parameter `t` (0 rim, 1 top)."""
    ang = t * math.pi / 2
    return math.sin(ang), math.cos(ang)


def rolled_rim(hf, a, k, z, roll=0.0035, detail=2):
    """Profile points of a rolled edge at the rim: inner lining, roll, outer surface start."""
    if detail == 0:
        return [hf.at(a, k, z)]
    pts = [hf.at(a, k, z + 0.008, out=-0.004)]
    if detail > 1:
        pts.append(hf.at(a, k, z - roll * 0.6, out=-0.002))
    pts.append(hf.at(a, k, z - roll, out=roll * 0.6))
    pts.append(hf.at(a, k, z, out=roll * 1.1))
    return pts


# --- Helmets ------------------------------------------------------------------------------

# Figures whose open bassinet (without aventail) is replaced by a cervelière (skull cap):
# the Flemish and routier infantry of the 1340s-1370s.
CERVELIERE = {"infantry_5", "infantry_8"}


def _bassinet_rim(hf, a, open_face=True):
    """Lower edge of the bassinet: at the brow in front (open face), the jaw elsewhere."""
    f = eq.smoothstep(0.5, 0.88, front(a))
    brow = hf.eye + 0.035
    jaw = hf.chin + 0.045
    if not open_face:
        return jaw
    return brow * f + jaw * (1 - f)


def bassinet_shell(g, name="bassinet", open_face=True, point=0.05, low=True):
    """Pointed bassinet of the 1340s-1380s: tall skull drawn back to a point, rolled edge.

    `low`: the sides come down to the jaw (with aventail); otherwise to the ear's top.
    Returns (object, head frame).
    """
    hf = HeadFrame(g.lm, pad=0.012)
    n = g.seg(28, 12, 6)
    rows = g.seg(9, 4, 2)
    top = hf.top + 0.016
    brow = hf.eye + 0.035
    detail = g.seg(2, 0, 0)

    def rim_z(a):
        if low:
            return _bassinet_rim(hf, a, open_face)
        return brow + 0.01 * (1 - front(a))

    apex_z = top + point * 0.9
    zc = brow + 0.02

    def profile(a):
        z0 = rim_z(a)
        pts = rolled_rim(hf, a, 1.0, z0, detail=detail)
        side = 0.3 if z0 < zc else 0.0
        z_arch = max(z0, zc) if side else z0
        for r in range(1, rows + 1):
            t = r / rows
            if t <= side:
                # Straight sides from the jaw up to the brow.
                u = t / side
                pts.append(hf.at(a, 1.0 + 0.02 * u, z0 + (zc - z0) * u))
                continue
            # Gothic arch drawn back to the point: radius falls linearly at the apex.
            u = (t - side) / (1.0 - side)
            k = 1.02 * (1.0 - u) * (1.0 + 1.1 * u)
            z = z_arch + (apex_z - z_arch) * u
            pts.append(hf.at(a, max(k, 0.03), z, dy=point * u**1.6))
        return pts

    bm = bmesh.new()
    apex = bm.verts.new(Vector((hf.cx, hf.cy + point * 1.02, apex_z + 0.004)))
    rim_rows = len(rolled_rim(hf, 0.0, 1.0, 0.0, detail=detail))

    def mat_of(a, r):
        return 1 if r < 1 and rim_rows > 1 else 0

    revolve(bm, n, profile, apex=apex, mat_of=mat_of)
    if g.level == 0:
        # Vervelles (staples of the aventail) along the sides and back, rivets at the brow.
        for i in range(0, 28, 2):
            a = -math.pi / 2 + 2 * math.pi * i / 28
            if front(a) > 0.5 or not low:
                continue
            z = rim_z(a) + 0.022
            p = hf.at(a, 1.0, z, out=0.002)
            rivet(bm, p, p - Vector((hf.cx, hf.cy, z)), 0.0045, 2)
    obj = finish_object(
        name,
        bm,
        [
            g.mat(eq.C_PLATE, STEEL),
            g.mat(eq.C_LEATHER, LINING),
            g.mat(eq.C_TRIM, BRASS),
        ],
        bone="Head",
    )
    return obj, hf


def aventail(g, hf):
    """Mail aventail laced to the bassinet, draped on the shoulders (FG0) with a hem."""
    frame = (hf.cx, hf.cy, hf.rx + 0.004, hf.ry + 0.004)
    n = g.seg(32, 12, 6)
    rows = g.seg(10, 4, 2)
    objs = fe.aventail(g.lm, frame, g.bvhs[0], extra=tuple(g.bvhs[1:]), n=n, rows=rows)
    for obj in objs:
        obj.data.materials[0] = g.mat(eq.C_MAIL, MAIL)
        if g.level == 0:
            hem(obj, 0.006, g.mat(eq.C_MAIL, MAIL))
    return objs


def hem(obj, depth, material=None):
    """Give an open shell a visible thickness: extrude its boundary `depth` inwards.

    The rim faces take `material` (default: the shell's first material). Normals of the
    shell point outwards; the extrusion goes along the vertex normal reversed.
    """
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.normal_update()
    edges = [e for e in bm.edges if e.is_boundary]
    if not edges:
        bm.free()
        return
    normals = {v: v.normal.copy() for e in edges for v in e.verts}
    res = bmesh.ops.extrude_edge_only(bm, edges=edges)
    new_verts = [x for x in res["geom"] if isinstance(x, bmesh.types.BMVert)]
    # Map new verts back to their source vertex (same position right after extrusion).
    by_pos = {tuple(round(c, 6) for c in v.co): v for v in normals}
    for v in new_verts:
        src = by_pos.get(tuple(round(c, 6) for c in v.co))
        if src is not None:
            v.co = v.co - normals[src] * depth
    k = 0
    if material is not None:
        if material.name not in [m.name for m in me.materials if m]:
            me.materials.append(material)
        k = [m.name if m else "" for m in me.materials].index(material.name)
    new_faces = [x for x in res["geom"] if isinstance(x, bmesh.types.BMFace)]
    for f in new_faces:
        f.material_index = k
    bmesh.ops.recalc_face_normals(bm, faces=new_faces)
    bm.to_mesh(me)
    bm.free()
    _copy_weights_to_new(obj)


def _copy_weights_to_new(obj):
    """Vertices without weights (made by an extrusion) take their nearest neighbour's."""
    from mathutils.kdtree import KDTree

    me = obj.data
    weighted = [v for v in me.vertices if v.groups]
    bare = [v for v in me.vertices if not v.groups]
    if not bare or not weighted:
        return
    kd = KDTree(len(weighted))
    for i, v in enumerate(weighted):
        kd.insert(v.co, i)
    kd.balance()
    for v in bare:
        _co, i, _d = kd.find(v.co)
        src = weighted[i]
        for grp in src.groups:
            obj.vertex_groups[grp.group].add([v.index], grp.weight, "REPLACE")


def visor(g, hf):
    """Pointed « museau de chien » visor (hounskull) closing the bassinet's face.

    Hinged at the temples; the snout reaches ~11 cm before the mouth; two sights at eye
    level, breaths on the right cheek of the snout (dark faces), brass pivot rivets.
    """
    n = g.seg(22, 10, 6)
    rows = g.seg(8, 3, 2)
    brow = hf.eye + 0.035
    low = hf.chin - 0.005
    snout = Vector((hf.cx, hf.lm.head_min.y - 0.115, hf.eye - 0.045))
    base_y = hf.lm.head_min.y + 0.035  # plane of the opening (temples)

    # Opening: an oval in the face plane, following the bassinet's brow and jaw.
    def opening(i):
        u = 2 * math.pi * i / n
        x = hf.cx + (hf.rx + 0.018) * math.cos(u)
        z = (brow + low) / 2 + (brow - low) / 2 * math.sin(u)
        y = base_y - 0.02 * abs(math.sin(u)) - 0.03 * max(0.0, -math.sin(u))
        return Vector((x, y, z))

    bm = bmesh.new()
    rings = []
    for r in range(rows):
        t = r / rows
        ring = []
        for i in range(n):
            p = opening(i)
            # Towards the snout: shrink and pull forwards (bulging cone).
            q = p.lerp(snout, t**1.15)
            bulge = math.sin(t * math.pi) * 0.008
            d = q - Vector((hf.cx, q.y, (brow + low) / 2))
            if d.length > 1e-5:
                q += d.normalized() * bulge
            ring.append(bm.verts.new(q))
        rings.append(ring)
    tip = bm.verts.new(snout)
    for r in range(rows - 1):
        for i in range(n):
            j = (i + 1) % n
            f = bm.faces.new(
                (rings[r][j], rings[r][i], rings[r + 1][i], rings[r + 1][j])
            )
            c = f.calc_center_median()
            f.material_index = 0
            del c
    for i in range(n):
        bm.faces.new((rings[-1][(i + 1) % n], rings[-1][i], tip))
    if g.level < 2:
        # Sights: two dark slits at eye level, cut into the visor's faces.
        cut_slit(
            bm,
            [(hf.cx - 0.068, hf.cx - 0.014), (hf.cx + 0.014, hf.cx + 0.068)],
            hf.eye - 0.003,
            hf.eye + 0.008,
            1,
            hf.cy,
        )
    if g.level == 0:
        # Breaths: small dark discs on the snout's right side (the left is the lance side).
        for bz in (-0.03, -0.045, -0.06):
            for bx in (-0.035, -0.05):
                p = Vector((hf.cx + bx, 0.0, hf.eye + bz))
                hit = _closest_on(bm, p)
                if hit is not None:
                    co, nrm = hit
                    rivet(bm, co + nrm * 0.0005, nrm, 0.0035, 1, sides=4)
        for a_pivot in (0.0, math.pi):
            p = hf.at(a_pivot, 1.0, hf.eye + 0.02, out=0.02, dy=-0.02)
            rivet(bm, p, Vector((math.cos(a_pivot), 0, 0)), 0.008, 2, sides=6)
    orient_out(bm, Vector((hf.cx, hf.cy, hf.eye - 0.02)))
    return finish_object(
        "visor",
        bm,
        [
            g.mat(eq.C_PLATE, STEEL),
            g.mat(eq.C_EXACT, SLIT),
            g.mat(eq.C_TRIM, BRASS),
        ],
        bone="Head",
    )


def cut_slit(bm, x_ranges, z0, z1, mat, y_max):
    """Cut the faces of `bm` along z0, z1 and the x bounds; dark faces in between.

    Only faces in front of `y_max` (the face side, -Y) are painted: sights and slots.
    """
    planes = [(Vector((0, 0, z)), Vector((0, 0, 1))) for z in (z0, z1)]
    for xa, xb in x_ranges:
        planes += [(Vector((x, 0, 0)), Vector((1, 0, 0))) for x in (xa, xb)]
    for co, no in planes:
        geom = list(bm.verts) + list(bm.edges) + list(bm.faces)
        bmesh.ops.bisect_plane(bm, geom=geom, plane_co=co, plane_no=no, dist=1e-5)
    for f in bm.faces:
        c = f.calc_center_median()
        if z0 < c.z < z1 and c.y < y_max and any(xa < c.x < xb for xa, xb in x_ranges):
            f.material_index = mat


def _closest_on(bm, p):
    """Closest point and normal of the faces of `bm` from `p` (projected along -Y)."""
    from mathutils.bvhtree import BVHTree

    bm.faces.ensure_lookup_table()
    tree = BVHTree.FromBMesh(bm)
    hit = tree.ray_cast(Vector((p.x, p.y - 1.0, p.z)), Vector((0, 1, 0)), 2.0)
    if hit[0] is None:
        return None
    return hit[0], hit[1]


@gear("bassinet")
def fine_bassinet(g, aventail=True, visor=False):
    """Bassinet: with aventail (camail), with a hounskull visor, or open (commoners).

    Figures of ``CERVELIERE`` wear a cervelière instead of the open bassinet.
    """
    if not aventail and not visor and g.fig in CERVELIERE:
        return cerveliere(g)
    helm, hf = bassinet_shell(g, low=aventail or visor)
    out = [helm]
    if visor:
        out.append(globals()["visor"](g, hf))
    if aventail:
        out += globals()["aventail"](g, hf)
    return out


def cerveliere(g):
    """Cervelière: close round skull cap over the crown, rolled edge, riveted lining band."""
    hf = HeadFrame(g.lm, pad=0.01)
    n = g.seg(24, 10, 6)
    rows = g.seg(6, 3, 2)
    z0 = hf.eye + 0.045
    top = hf.top + 0.014
    detail = g.seg(2, 0, 0)

    def profile(a):
        z_rim = z0 + 0.008 * back(a) - 0.004 * front(a)
        pts = rolled_rim(hf, a, 1.0, z_rim, detail=detail)
        for r in range(1, rows + 1):
            h, k = dome(r / rows)
            pts.append(hf.at(a, max(k * 1.01, 0.05), z_rim + (top - z_rim) * h))
        return pts

    bm = bmesh.new()
    apex = bm.verts.new(Vector((hf.cx, hf.cy, top + 0.002)))
    revolve(bm, n, profile, apex=apex, mat_of=lambda a, r: 1 if r < 1 and detail else 0)
    if g.level == 0:
        for i in range(12):
            a = 2 * math.pi * i / 12
            p = hf.at(a, 1.0, z0 + 0.018, out=0.002)
            rivet(bm, p, p - Vector((hf.cx, hf.cy, p.z)), 0.004, 2)
    return [
        finish_object(
            "cerveliere",
            bm,
            [
                g.mat(eq.C_PLATE, STEEL_DARK),
                g.mat(eq.C_LEATHER, LINING),
                g.mat(eq.C_TRIM, IRON),
            ],
            bone="Head",
        )
    ]


@gear("kettle_hat")
def kettle_hat(g):
    """Chapel de fer: raised skull with a low comb, wide sloping brim with a rolled edge.

    Skull and brim riveted together (a row of rivets at the crown's base).
    """
    hf = HeadFrame(g.lm, pad=0.014)
    n = g.seg(28, 12, 6)
    rows = g.seg(7, 3, 2)
    base = hf.eye + 0.04
    top = hf.top + 0.035
    brim = g.seg(0.075, 0.075, 0.07)
    drop = 0.04
    detail = g.seg(2, 1, 0)

    def profile(a):
        pts = []
        # Brim: under side, rolled outer edge, upper side.
        pts.append(hf.at(a, 1.0, base - 0.004, out=0.0))
        pts.append(hf.at(a, 1.0, base - drop - 0.006, out=brim - 0.004))
        if detail:
            pts.append(hf.at(a, 1.0, base - drop - 0.009, out=brim + 0.002))
            if detail > 1:
                pts.append(hf.at(a, 1.0, base - drop - 0.003, out=brim + 0.007))
            pts.append(hf.at(a, 1.0, base - drop + 0.004, out=brim + 0.002))
        pts.append(hf.at(a, 1.0, base + 0.002, out=0.004))
        # Skull: comb ridge along the front-back axis (x near the centre).
        for r in range(1, rows + 1):
            h, k = dome(r / rows)
            ridge = 0.006 * (1 - abs(math.cos(a))) ** 6 * h
            pts.append(hf.at(a, max(k * 1.02, 0.04), base + (top - base) * h + ridge))
        return pts

    bm = bmesh.new()
    apex = bm.verts.new(Vector((hf.cx, hf.cy, top + 0.006)))
    revolve(bm, n, profile, apex=apex)
    if g.level == 0:
        for i in range(16):
            a = 2 * math.pi * i / 16
            p = hf.at(a, 1.02, base + 0.012, out=0.002)
            rivet(bm, p, p - Vector((hf.cx, hf.cy, p.z)), 0.0045, 1)
    return [
        finish_object(
            "kettle_hat",
            bm,
            [g.mat(eq.C_PLATE, (0.40, 0.40, 0.41)), g.mat(eq.C_TRIM, IRON)],
            bone="Head",
        )
    ]


@gear("great_helm")
def great_helm(g):
    """Great helm of the 1340s: sugarloaf crown, sights under a brow flange, breaths.

    A riveted cross (brow band and nasal strip) reinforces the face; worn over a padded
    coif (the head does not show).
    """
    hf = HeadFrame(g.lm, pad=0.03)
    n = g.seg(28, 12, 6)
    bottom = hf.chin - 0.05
    top = hf.top + 0.05
    sight = hf.eye + 0.005
    slit = 0.007  # half-height of the sights
    zs = g.seg(
        [
            bottom,
            bottom + 0.07,
            sight - 0.06,
            sight - slit,
            sight + slit,
            sight + 0.02,
            sight + 0.03,
            sight + 0.06,
            top - 0.035,
            top - 0.01,
        ],
        [bottom, sight - slit, sight + slit, sight + 0.03, top - 0.02],
        [bottom, sight, top - 0.03],
    )

    def profile(a):
        pts = [hf.at(a, 1.02, bottom + 0.012, out=-0.006)]
        for z in zs:
            # Slight flare at the bottom, sugarloaf taper over the brow.
            if z < sight:
                k = 1.03 - 0.03 * (z - bottom) / max(sight - bottom, 1e-3)
            else:
                u = (z - sight) / max(top - sight, 1e-3)
                k = math.cos(u * math.pi / 2 * 0.92) ** 0.8
            out = 0.0
            if abs(z - sight) <= slit + 1e-4 and front(a) > 0.3:
                out = -0.005  # the sights are sunk under the brow flange
            if abs(z - (sight + 0.02)) < 0.004 and front(a) > 0.3:
                out = 0.006
            pts.append(hf.at(a, k, z, out=out))
        return pts

    slit_row = zs.index(sight - slit) + 1 if g.level < 2 else -1

    def mat_of(a, r):
        c = math.cos(a)
        if r == slit_row and front(a) > 0.6 and abs(c) > 0.04:
            return 1
        return 0

    bm = bmesh.new()
    apex = bm.verts.new(Vector((hf.cx, hf.cy, top + 0.004)))
    revolve(bm, n, profile, apex=apex, mat_of=mat_of)
    if g.level == 0:
        # Nasal strip and brow band with rivets.
        for dz in (-0.1, -0.075, -0.05, -0.025, 0.035):
            p = hf.at(-math.pi / 2, 1.0, sight + dz, out=0.003)
            rivet(bm, p, Vector((0, -1, 0)), 0.005, 2)
        # Breaths on the right cheek (cross pattern of small dark holes).
        for dz in (-0.05, -0.07, -0.09):
            for da in (0.35, 0.5, 0.65):
                a = -math.pi / 2 - da
                p = hf.at(a, 1.03, sight + dz, out=0.001)
                rivet(bm, p, p - Vector((hf.cx, hf.cy, p.z)), 0.0035, 1)
        # Chain staple (to the breast) and the cross's arms at the brow.
        for da in (-0.5, -0.3, 0.3, 0.5):
            a = -math.pi / 2 + da
            p = hf.at(a, 1.0, sight + 0.02, out=0.008)
            rivet(bm, p, p - Vector((hf.cx, hf.cy, p.z)), 0.005, 2)
    return [
        finish_object(
            "great_helm",
            bm,
            [
                g.mat(eq.C_PLATE, STEEL),
                g.mat(eq.C_EXACT, SLIT),
                g.mat(eq.C_TRIM, BRASS),
            ],
            bone="Head",
        )
    ]


@gear("sallet")
def sallet(g, bevor=False, colour=eq.STEEL):
    """Sallet of the 1430s-1450s: round skull with a low ridge, long flared tail.

    With `bevor` (men-at-arms): a close sallet down to the nose with a sight, and a bevor
    (bavière) covering chin and throat, two gorget lames. Without: the open archer's sallet.
    """
    hf = HeadFrame(g.lm, pad=0.014)
    colour = tuple(colour)
    if colour == tuple(eq.STEEL):
        colour = STEEL
    n = g.seg(28, 12, 6)
    rows = g.seg(8, 3, 2)
    top = hf.top + 0.022
    brow = hf.eye + 0.03
    nose = hf.eye - 0.035
    detail = g.seg(2, 1, 0)

    def rim_z(a):
        f = eq.smoothstep(0.3, 0.8, front(a))
        b = back(a)
        face = nose if bevor else brow
        side = hf.eye - 0.02
        return face * f + side * (1 - f) - 0.1 * b**1.5

    def rim_out(a):
        return 0.012 * back(a) ** 1.2 + 0.05 * back(a) ** 3

    sight = (hf.eye - 0.004, hf.eye + 0.01)

    def profile(a):
        z0 = rim_z(a)
        o = rim_out(a)
        pts = []
        if detail:
            pts.append(hf.at(a, 1.0, z0 + 0.008, out=o - 0.003))
            pts.append(hf.at(a, 1.0, z0 - 0.002, out=o + 0.001))
        pts.append(hf.at(a, 1.0, z0, out=o + 0.003))
        zc = hf.eye + 0.015
        for r in range(1, rows + 1):
            t = r / rows
            h, k = dome(t)
            z_base = max(z0, zc - 0.03) if t > 0 else z0
            z = z_base + (top - z_base) * h
            ridge = 0.005 * (1 - abs(math.cos(a))) ** 8 * h
            oo = o * (1 - eq.smoothstep(0.0, 0.5, t))
            pts.append(hf.at(a, max(k * 1.02, 0.04), z + ridge, out=oo))
        return pts

    def mat_of(a, r):
        return 0

    bm = bmesh.new()
    apex = bm.verts.new(Vector((hf.cx, hf.cy + 0.005, top + 0.005)))
    grid = revolve(bm, n, profile, apex=apex, mat_of=mat_of)
    mats = [g.mat(eq.C_PLATE, colour), g.mat(eq.C_EXACT, SLIT), g.mat(eq.C_TRIM, BRASS)]
    if bevor and g.level < 2:
        # Sight: a dark slit across the front at eye level.
        cut_slit(
            bm, [(hf.cx - 0.075, hf.cx + 0.075)], sight[0], sight[1], 1, hf.cy - 0.03
        )
    if g.level == 0:
        for i in range(10):
            a = -math.pi / 2 + 2 * math.pi * (i + 0.5) / 10
            p = hf.at(a, 1.0, hf.eye + 0.05, out=0.003)
            rivet(bm, p, p - Vector((hf.cx, hf.cy, p.z)), 0.0042, 2)
    del grid
    out = [finish_object("sallet", bm, mats, bone="Head")]
    if bevor:
        out.append(bevor_plate(g, hf, colour))
    return out


def bevor_plate(g, hf, colour):
    """Bevor (bavière): chin cup up to the sallet's lower edge, two gorget lames."""
    n = g.seg(16, 8, 4)
    arc = math.pi * 1.25
    top = hf.eye - 0.02  # tucked under the sallet's lower edge (at the nose)
    chin = hf.chin
    neck = hf.lm.shoulder_z + 0.035
    # (height, radius factor, offset, forward push) from the mouth down to the collar.
    table = [
        (top, 0.96, 0.006, 0.0),
        (top - 0.04, 0.94, 0.008, 0.004),
        (chin + 0.005, 0.88, 0.01, 0.01),
        (chin - 0.025, 0.78, 0.01, 0.006),
        (neck + 0.03, 0.7, 0.01, 0.002),
        (neck + 0.012, 0.8, 0.02, 0.012),
        (neck - 0.01, 0.92, 0.03, 0.03),
    ]
    keep = g.seg(range(7), (0, 2, 4, 6), (0, 3, 6))
    bm = bmesh.new()
    rows = []
    for idx in keep:
        z, k, out, push = table[idx]
        row = []
        for i in range(n + 1):
            a = -math.pi / 2 - arc / 2 + arc * i / n
            p = hf.at(a, k, z, out=out)
            p.y -= push * front(a)
            p.z -= 0.02 * front(a) if z < neck else 0.0
            row.append(bm.verts.new(p))
        rows.append(row)
    for a_, b_ in zip(rows, rows[1:], strict=False):
        eq.bridge(bm, b_, a_, 0, closed=False)
    if g.level == 0:
        for i in (2, 5, n - 5, n - 2):
            p = rows[1][i].co
            rivet(bm, p, p - Vector((hf.cx, hf.cy, p.z)), 0.0045, 1)
    orient_out(bm, Vector((hf.cx, hf.cy, hf.chin)))
    obj = finish_object(
        "bevor",
        bm,
        [g.mat(eq.C_PLATE, colour), g.mat(eq.C_TRIM, BRASS)],
        weigh=lambda p: (
            {"Head": 0.45, "Neck": 0.35, "Chest": 0.2}
            if p.z > chin - 0.02
            else {"Neck": 0.5, "Chest": 0.5}
        ),
    )
    if g.level == 0:
        hem(obj, 0.004)
    return obj


@gear("cloth_cap")
def cloth_cap(g, colour=(0.25, 0.10, 0.05)):
    """Felt hat: soft rounded crown pinched at the top, brim turned up (split at the back)."""
    hf = HeadFrame(g.lm, pad=0.008)
    n = g.seg(24, 10, 6)
    rows = g.seg(6, 3, 2)
    base = hf.eye + 0.045
    top = hf.top + 0.04

    def profile(a):
        pts = [hf.at(a, 1.0, base - 0.004, out=0.0)]
        # Brim turned up, wider at the front, cut away at the back.
        w = 0.035 + 0.02 * front(a) - 0.03 * back(a) ** 2
        pts.append(hf.at(a, 1.0, base - 0.006, out=max(w, 0.006)))
        pts.append(hf.at(a, 1.0, base + 0.03, out=max(w, 0.006) + 0.006))
        pts.append(hf.at(a, 1.0, base + 0.026, out=0.004))
        for r in range(1, rows + 1):
            h, k = dome(r / rows)
            k = k * (1.04 + 0.05 * math.sin(r / rows * math.pi))
            pts.append(
                hf.at(a, max(k, 0.04), base + 0.02 + (top - base) * h, dy=0.03 * h)
            )
        return pts

    bm = bmesh.new()
    apex = bm.verts.new(Vector((hf.cx, hf.cy + 0.035, top + 0.01)))
    revolve(bm, n, profile, apex=apex)
    return [
        finish_object("cloth_cap", bm, [g.mat(eq.C_CLOTH, tuple(colour))], bone="Head")
    ]


# --- Armour -------------------------------------------------------------------------------

# Harness of the men-at-arms per figure: "early" (1340s-1360s: spaudlers, couters and
# vambraces over the mail sleeves, poleyns and greaves over mail chausses) or "late"
# (1400s-1440s: full arm and leg harness, cuisses, plate gauntlets).
HARNESS_ERA = {
    "infantry_0": "early",
    "cavalry_0": "early",
    "standard_0": "early",
    "standard_1": "early",
    "infantry_7": "late",
    "cavalry_3": "late",
}
LIMBS_OUT = {"Head", "Neck", "Wrist.L", "Wrist.R", "Foot.L", "Foot.R"}
ARM_BONES = {"UpperArm.L", "UpperArm.R", "LowerArm.L", "LowerArm.R"}


def plate_shell(body, name, material, keep, offset, relax=8, depth=0.004):
    """Shell of the body faces selected by `keep`, with a rolled thickness (or None)."""
    obj = fe.shell(body, name, material, keep, offset, relax=relax)
    if not obj.data.polygons:
        bpy.data.objects.remove(obj)
        return None
    if depth:
        hem(obj, depth)
    return obj


def limb_harness(g, body, era, colour):
    """Plates of the arms and legs over the mail (one object, the body's weights).

    Early: spaudler of three lames, couter, tubular vambrace, poleyn, greave. Late adds the
    rerebrace, a fourth lame and the cuisse.
    """
    lm = g.lm
    material = g.mat(eq.C_PLATE, colour)
    late = era == "late"
    pieces = []
    for s in "LR":
        sign = 1.0 if s == "L" else -1.0
        knee = lm.bone[f"LowerLeg.{s}"]
        ankle = lm.bone[f"Foot.{s}"]
        hip = lm.bone[f"UpperLeg.{s}"]
        shoulder = lm.bone[f"UpperArm.{s}"]
        elbow = lm.bone[f"LowerArm.{s}"]
        wrist = lm.bone[f"Wrist.{s}"]
        lower_leg, upper_leg = f"LowerLeg.{s}", f"UpperLeg.{s}"
        lower_arm, upper_arm = f"LowerArm.{s}", f"UpperArm.{s}"

        def greave(c, bones, lower_leg=lower_leg, knee=knee, ankle=ankle):
            return bones <= {lower_leg} and ankle.z + 0.045 < c.z < knee.z - 0.075

        def poleyn(c, bones, knee=knee, lower_leg=lower_leg, upper_leg=upper_leg):
            if not bones & {lower_leg, upper_leg} or bones & LIMBS_OUT:
                return False
            return (c - knee).length < 0.085 and c.y < knee.y + 0.01

        def cuisse(c, bones, knee=knee, hip=hip, upper_leg=upper_leg):
            return (
                bones <= {upper_leg}
                and knee.z + 0.075 < c.z < hip.z - 0.1
                and c.y < knee.y + 0.03
            )

        def couter(c, bones, elbow=elbow, lower_arm=lower_arm, upper_arm=upper_arm):
            if not bones & {lower_arm, upper_arm} or bones & LIMBS_OUT:
                return False
            return (c - elbow).length < 0.07

        def vambrace(c, bones, elbow=elbow, wrist=wrist, lower_arm=lower_arm):
            return (
                bones <= {lower_arm}
                and (c - elbow).length > 0.07
                and (c - wrist).length > 0.035
            )

        def rerebrace(c, bones, elbow=elbow, shoulder=shoulder, upper_arm=upper_arm):
            return (
                bones <= {upper_arm}
                and (c - elbow).length > 0.07
                and (c - shoulder).length > 0.1
            )

        legs = [("greave", greave, 0.02), ("poleyn", poleyn, 0.03)]
        arms = [("couter", couter, 0.034), ("vambrace", vambrace, 0.03)]
        if late:
            legs.append(("cuisse", cuisse, 0.022))
            arms.append(("rerebrace", rerebrace, 0.03))
        for name, keep, off in legs + arms:
            obj = plate_shell(body, f"{name}_{s}", material, keep, off)
            if obj is not None:
                pieces.append(obj)
        # Spaudler: lames overlapping downwards from the top of the shoulder.
        lames = 4 if late else 3
        top = shoulder.z + 0.09
        step = 0.045
        for k in range(lames):
            z_hi = top - k * step
            z_lo = z_hi - step - 0.012

            def lame(
                c,
                bones,
                z_hi=z_hi,
                z_lo=z_lo,
                shoulder=shoulder,
                sign=sign,
                s=s,
            ):
                if not bones & {f"Shoulder.{s}", f"UpperArm.{s}"}:
                    return False
                if bones & LIMBS_OUT:
                    return False
                return z_lo < c.z < z_hi and (c.x - shoulder.x) * sign > -0.05

            obj = plate_shell(
                body, f"spaudler_{s}{k}", material, lame, 0.036 + 0.005 * k, relax=4
            )
            if obj is not None:
                pieces.append(obj)
    if not pieces:
        return []
    return [fe.join(pieces, "limb_plates")]


def quilt(obj, lm, amp=0.005, spacing=0.05):
    """Model the quilting of a padded coat: vertical channels, rings on the sleeves."""
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.normal_update()
    dom = fe.dominant(obj)
    axis = lm.bone["Hips"]
    n = max(8, int(2 * math.pi * 0.16 / spacing))
    mw = obj.matrix_world
    inv = mw.inverted()
    bm.verts.ensure_lookup_table()
    for v in bm.verts:
        p = mw @ v.co
        bone = dom[v.index]
        if bone in ARM_BONES:
            side = bone[-1]
            d = (p - lm.bone[f"UpperArm.{side}"]).length
            ridge = 0.5 + 0.5 * math.cos(2 * math.pi * d / spacing)
        else:
            phi = math.atan2(p.y - axis.y, p.x - axis.x)
            ridge = 0.5 + 0.5 * math.cos(phi * n)
        v.co = inv @ (p + (mw.to_3x3() @ v.normal).normalized() * amp * ridge)
    bm.to_mesh(me)
    bm.free()


def skirt_piece(lm, bvh, material, name, hem_z, push=0.0, folds=1.0):
    """Split skirt from the waist to `hem_z` (FG0 skirt), pushed out by `push`."""
    obj = fe._skirt(lm, bvh, folds=folds)
    obj.name = obj.data.name = name
    waist = lm.waist_z
    k = (hem_z - waist) / (lm.knee_z + 0.05 - waist)
    centre = (lm.bone["UpperLeg.L"] + lm.bone["UpperLeg.R"]) / 2
    for v in obj.data.vertices:
        v.co.z = waist + (v.co.z - waist) * k
        if push:
            d = Vector((v.co.x - centre.x, v.co.y - centre.y, 0.0))
            if d.length > 1e-4:
                v.co += d.normalized() * push
    obj.data.materials.clear()
    obj.data.materials.append(material)
    for p in obj.data.polygons:
        p.material_index = 0
    return obj


def _torso_tree(obj):
    """BVH of `obj` without its sleeves."""
    from mathutils.bvhtree import BVHTree

    dom = fe.dominant(obj)
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.transform(obj.matrix_world)
    doomed = [
        f
        for f in bm.faces
        if any(dom[v.index] in ARM_BONES | {"Wrist.L", "Wrist.R"} for v in f.verts)
    ]
    bmesh.ops.delete(bm, geom=doomed, context="FACES")
    tree = BVHTree.FromBMesh(bm)
    bm.free()
    return tree


@gear("jack")
def jack(g, colour=(0.55, 0.47, 0.32), skirt=0.3, livery=False):
    """Jaque: padded jacket to mid-thigh, sleeves to the wrist, modelled quilting.

    `livery=True` dyes it in the side's colours (francs-archers' hoquetons).
    """
    lm = g.lm
    code = eq.C_LIVERY if livery else eq.C_QUILT
    material = g.mat(code, tuple(colour))
    hip_z = lm.bone["UpperLeg.L"].z - 0.03

    def keep(c, bones):
        if bones & LIMBS_OUT or bones & {"LowerLeg.L", "LowerLeg.R"}:
            return False
        return c.z > hip_z

    top = fe.shell(g.body, "jack", material, keep, 0.036, relax=30)
    if g.level == 0:
        quilt(top, lm, 0.006)
    hem(top, 0.008)
    hem_z = lm.bone["Hips"].z - skirt
    low = skirt_piece(lm, _torso_tree(top), material, "jack_skirt", hem_z, push=0.024)
    if g.level == 0:
        quilt(low, lm, 0.005)
    hem(low, 0.008)
    return [top, low]


@gear("brigandine")
def brigandine(g, colour=(0.35, 0.05, 0.04), studs=True):
    """Brigandine: cloth-covered plates to the hips, rows of gilt rivets front and back."""
    from mathutils.bvhtree import BVHTree

    lm = g.lm
    hips = lm.bone["Hips"]
    bottom = hips.z - 0.1
    material = g.mat(eq.C_CLOTH, tuple(colour))

    def keep(c, bones):
        if bones & LIMBS_OUT or bones & ARM_BONES:
            return False
        if bones & {"LowerLeg.L", "LowerLeg.R"}:
            return False
        return c.z > bottom

    body_shell = fe.shell(g.body, "brigandine", material, keep, 0.04, relax=25)
    hem(body_shell, 0.008)
    out = [body_shell]
    if studs and g.level == 0:
        bm_src = bmesh.new()
        bm_src.from_mesh(body_shell.data)
        bm_src.transform(body_shell.matrix_world)
        tree = BVHTree.FromBMesh(bm_src)
        bm_src.free()
        bm = bmesh.new()
        chest_top = lm.shoulder_z - 0.06
        z = bottom + 0.03
        row = 0
        while z < chest_top:
            count = 26
            for k in range(count):
                a = 2 * math.pi * (k + 0.5 * (row % 2)) / count
                d = Vector((math.cos(a), math.sin(a), 0.0))
                if abs(d.y) < 0.35:
                    continue  # the sides are laced, no rivets
                origin = Vector((hips.x, hips.y, z)) + d * 0.5
                hit = tree.ray_cast(origin, -d, 0.6)
                if hit[0] is None:
                    continue
                rivet(bm, hit[0], hit[1], 0.0055, 0)
            z += 0.04
            row += 1
        rivets = finish_object("rivets", bm, [g.mat(eq.C_TRIM, BRASS)])
        kd, weights = fe.body_lookup(g.body)
        eq.bind_by(rivets, fe.weights_from(kd, weights))
        out.append(rivets)
    return out


# --- Hidden faces -------------------------------------------------------------------------


def world_tree(obj):
    """BVH of `obj` in world space."""
    from mathutils.bvhtree import BVHTree

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.transform(obj.matrix_world)
    tree = BVHTree.FromBMesh(bm)
    bm.free()
    return tree


def cull_hidden(inner, groups, max_dist=0.06):
    """Delete the faces of `inner` covered in every group of outer pieces.

    `groups`: one list of world-space BVH trees per variant in which `inner` is shown. A
    face is hidden in a group when the rays cast outwards (along its normal) from its
    centre and from near each of its corners all meet, within `max_dist`, the back of one
    of the group's pieces (the ray leaves from inside it). Pieces deforming with the same
    weights stay covered in motion; slits, armholes and open hems let the rays through.
    Returns the number of faces removed.
    """
    if not groups or any(not g for g in groups) or not inner.data.polygons:
        return 0
    mw = inner.matrix_world
    nm = mw.to_3x3().inverted().transposed()
    bm = bmesh.new()
    bm.from_mesh(inner.data)
    bm.normal_update()

    def covered(p, n, trees):
        for tree in trees:
            hit = tree.ray_cast(p + n * 0.0005, n, max_dist)
            if hit[0] is not None and hit[1].dot(n) > 0.0:
                return True
        return False

    doomed = []
    for f in bm.faces:
        n = nm @ f.normal
        if n.length < 1e-6:
            continue
        n.normalize()
        centre = f.calc_center_median()
        pts = [mw @ centre] + [mw @ v.co.lerp(centre, 0.15) for v in f.verts]
        if all(all(covered(p, n, trees) for p in pts) for trees in groups):
            doomed.append(f)
    if doomed:
        bmesh.ops.delete(bm, geom=doomed, context="FACES")
        bmesh.ops.delete(
            bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS"
        )
        bm.to_mesh(inner.data)
    bm.free()
    return len(doomed)


# Weapons and shields register themselves in GEAR (import at the end: they use the helpers).
import battle_fine_weapons  # noqa: E402, F401
