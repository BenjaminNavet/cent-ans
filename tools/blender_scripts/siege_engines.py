"""SG2 siege engines: procedural low-poly models with named, pivoted parts for node animation.

Run headless::

    blender --background --python tools/blender_scripts/siege_engines.py -- <out_dir>

Writes ``trebuchet.glb``, ``mangonel.glb``, ``bombard.glb``, ``ram.glb``, ``siege_tower.glb``
and ``manifest.json`` (triangles and pivots per model) to ``<out_dir>``
(``game/assets/models/siege/``). Every moving part is its own node whose origin is its pivot,
so that Godot animates it by rotating or moving the node (``siege_engines_fx.gd``, timings in
``data/fx/siege_engines.json``); nothing is baked as an animation clip. Materials are named only
(``Timber``, ``TimberDark``, ``Iron``, ``Rope``, ``Hide``, ``Stone``): Godot swaps them for its
textured battle materials. Geometry is written in Godot axes (x right, y up, +z forward, towards
the target) and converted to Blender axes (x, -z, y) so that the glTF export lands back on them.

Dimensions (metres) follow 14th-century descriptions: counterweight trebuchet with a 6.4 m axle
and an 8.5 m long arm (Villard de Honnecourt, Viollet-le-Duc), torsion mangonel, a bombard of
about 3 m on its timber bed behind a hinged mantlet, a hide-roofed ram shed on wheels, a siege
tower of 12 m (scaled in height by the renderer to the wall).
"""

import json
import math
import sys
from pathlib import Path

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

MATERIALS = {
    "Timber": (0.36, 0.26, 0.16, 0.9),
    "TimberDark": (0.2, 0.14, 0.09, 0.9),
    "Iron": (0.08, 0.08, 0.085, 0.45),
    "Rope": (0.55, 0.47, 0.33, 1.0),
    "Hide": (0.42, 0.3, 0.18, 0.85),
    "Stone": (0.45, 0.43, 0.4, 0.95),
}

# Trebuchet proportions (shared with the manifest: the renderer reads the pivots from it).
TREB_AXLE = 6.4
TREB_LONG = 8.5
TREB_SHORT = 2.2
TREB_SLING = 4.4
MANGONEL_PIVOT = (0.0, 0.8, -0.6)
MANGONEL_ARM = 2.3
TOWER_HEIGHT = 12.0
TOWER_AXLE = 0.8

# SG3: simplified level of detail (`<name>_lod.glb`): small boxes and short thin beams are
# dropped, prisms and wheels get fewer sides; the named, pivoted nodes stay (animation).
LOD = {"on": False}
LOD_MIN_BOX = 0.4
LOD_THIN = 0.16
LOD_MIN_BEAM = 1.0


def g2b(v) -> Vector:
    """Godot axes (x right, y up, z forward) to Blender axes."""
    return Vector((v[0], -v[2], v[1]))


def material(name: str):
    """Named material shared by every model (colour for the Blender previews only)."""
    mat = bpy.data.materials.get(name)
    if mat is None:
        mat = bpy.data.materials.new(name)
        r, g, b, rough = MATERIALS[name]
        mat.diffuse_color = (r, g, b, 1.0)
        mat.use_nodes = True
        bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
        bsdf.inputs["Base Color"].default_value = (r, g, b, 1.0)
        bsdf.inputs["Roughness"].default_value = rough
        if name == "Iron":
            bsdf.inputs["Metallic"].default_value = 0.8
    return mat


class Part:
    """A mesh node under construction: geometry in its local Godot axes, per-face material."""

    def __init__(self) -> None:
        """Empty geometry."""
        self.bm = bmesh.new()
        self.names: list[str] = []

    def _mat(self, name: str) -> int:
        if name not in self.names:
            self.names.append(name)
        return self.names.index(name)

    def _faces(self, faces, mat: str) -> None:
        index = self._mat(mat)
        for f in faces:
            f.material_index = index

    def box(self, center, size, mat: str = "Timber", rot=None) -> None:
        """Axis-aligned box (``rot``: optional (axis, angle) rotation about its centre)."""
        dims = sorted(size)
        if LOD["on"] and (dims[2] < LOD_MIN_BOX or dims[1] < LOD_THIN):
            return
        geom = bmesh.ops.create_cube(self.bm, size=1.0)
        verts = geom["verts"]
        sx, sy, sz = size
        m = Matrix.Diagonal((sx, sz, sy, 1.0))  # Blender y = -Godot z, z = Godot y
        if rot is not None:
            axis, angle = rot
            m = Matrix.Rotation(angle, 4, g2b(axis).normalized()) @ m
        bmesh.ops.transform(
            self.bm, matrix=Matrix.Translation(g2b(center)) @ m, verts=verts
        )
        self._faces({f for v in verts for f in v.link_faces}, mat)

    def beam(
        self,
        a,
        b,
        r1: float,
        r2: float | None = None,
        sides: int = 6,
        mat: str = "Timber",
        cap: bool = True,
    ) -> None:
        """Tapered prism from ``a`` to ``b`` (Godot points)."""
        r2 = r1 if r2 is None else r2
        pa, pb = g2b(a), g2b(b)
        axis = pb - pa
        length = axis.length
        if LOD["on"]:
            if length < LOD_MIN_BEAM and max(r1, r2) < 0.15:
                return
            sides = max(3, sides // 2)
        geom = bmesh.ops.create_cone(
            self.bm,
            cap_ends=cap,
            cap_tris=False,
            segments=sides,
            radius1=r1,
            radius2=r2,
            depth=length,
        )
        verts = geom["verts"]
        rot = (
            Vector((0, 0, 1))
            .rotation_difference(axis.normalized())
            .to_matrix()
            .to_4x4()
        )
        bmesh.ops.transform(
            self.bm, matrix=Matrix.Translation((pa + pb) * 0.5) @ rot, verts=verts
        )
        self._faces({f for v in verts for f in v.link_faces}, mat)

    def sphere(self, center, radius: float, mat: str = "Stone") -> None:
        """Low icosphere (a stone)."""
        geom = bmesh.ops.create_icosphere(self.bm, subdivisions=1, radius=radius)
        verts = geom["verts"]
        bmesh.ops.transform(
            self.bm, matrix=Matrix.Translation(g2b(center)), verts=verts
        )
        self._faces({f for v in verts for f in v.link_faces}, mat)

    def wheel(
        self, center, radius: float, width: float, mat: str = "TimberDark"
    ) -> None:
        """Solid wheel on an axle along x: rim prism, hub, four spokes on the outer face."""
        x, y, z = center
        self.beam(
            (x - width * 0.5, y, z), (x + width * 0.5, y, z), radius, radius, 10, mat
        )
        side = 1.0 if x >= 0 else -1.0
        self.beam(
            (x, y, z),
            (x + side * (width * 0.5 + 0.12), y, z),
            radius * 0.22,
            radius * 0.22,
            6,
            "Iron",
        )
        for k in range(2):
            angle = k * math.pi * 0.5
            dy, dz = math.sin(angle) * radius * 0.92, math.cos(angle) * radius * 0.92
            self.beam(
                (x + side * width * 0.52, y - dy, z - dz),
                (x + side * width * 0.52, y + dy, z + dz),
                0.06,
                0.06,
                4,
                "Timber",
            )

    def build(self, name: str, parent=None, pivot=(0.0, 0.0, 0.0)):
        """Mesh object ``name`` under ``parent``, its origin (pivot) at ``pivot``."""
        mesh = bpy.data.meshes.new(name)
        self.bm.to_mesh(mesh)
        self.bm.free()
        for mat_name in self.names:
            mesh.materials.append(material(mat_name))
        obj = bpy.data.objects.new(name, mesh)
        bpy.context.scene.collection.objects.link(obj)
        obj.parent = parent
        obj.location = g2b(pivot)
        return obj


def empty(name: str, parent=None, pivot=(0.0, 0.0, 0.0)):
    """Empty node (a pivot) under ``parent``."""
    obj = bpy.data.objects.new(name, None)
    bpy.context.scene.collection.objects.link(obj)
    obj.parent = parent
    obj.location = g2b(pivot)
    return obj


# --- Engines ----------------------------------------------------------------------------


def trebuchet() -> dict:
    """Counterweight trebuchet, cocked: long arm down at the back, sling laid in the trough.

    Nodes: ``Frame``; ``Arm`` (pivot on the axle, rotation about x: 0 = arm level, long end to
    the back; positive raises the long end); under it ``Counterweight`` (hinged at the short end,
    counter-rotated to hang), ``Sling`` (at the long end, rope along its local +z) with ``Stone``
    in the pouch; ``Winch`` (drum at the back, rotation about x).
    """
    root = empty("Trebuchet")
    h = TREB_AXLE
    frame = Part()
    for x in (-1.6, 1.6):
        frame.box((x, 0.2, -2.0), (0.4, 0.4, 10.4), "TimberDark")
        frame.beam((x, 0.4, -2.8), (x, h - 0.2, 0.0), 0.2, 0.17)
        frame.beam((x, 0.4, 2.8), (x, h - 0.2, 0.0), 0.2, 0.17)
        frame.beam((x, 0.4, 0.0), (x, h - 0.2, 0.0), 0.22, 0.2)
        frame.box((x, h, 0.0), (0.55, 0.6, 0.8), "TimberDark")
        frame.box((x, 2.6, 0.0), (0.26, 0.26, 3.4))
        frame.box((x, 4.4, 0.0), (0.24, 0.24, 1.8))
        frame.box((x, 0.9, -6.2), (0.3, 1.4, 0.3))
    for z in (-6.8, -2.8, 0.0, 2.8):
        frame.box((0.0, 0.3, z), (3.8, 0.3, 0.34), "TimberDark")
    for x in (-0.45, 0.0, 0.45):
        frame.box((x, 0.5, -3.6), (0.3, 0.08, 5.6))  # trough planks under the sling
    frame.box((0.0, 2.6, 2.4), (3.2, 0.22, 0.22))
    frame.build("Frame", root)
    winch = Part()
    winch.beam((-1.45, 0.0, 0.0), (1.45, 0.0, 0.0), 0.3, 0.3, 8, "Timber")
    for k in range(3):
        angle = k * math.pi / 3.0
        dy, dz = math.sin(angle) * 1.1, math.cos(angle) * 1.1
        for x in (-1.3, 1.3):
            winch.beam((x, -dy, -dz), (x, dy, dz), 0.05, 0.05, 4, "Timber")
    winch.build("Winch", root, (0.0, 1.5, -6.2))
    arm = empty("Arm", root, (0.0, h, 0.0))
    beam = Part()
    beam.beam(
        (0.0, 0.0, -TREB_LONG), (0.0, 0.0, TREB_SHORT + 0.2), 0.14, 0.26, 6, "Timber"
    )
    beam.beam((-1.9, 0.0, 0.0), (1.9, 0.0, 0.0), 0.12, 0.12, 8, "Iron")
    for z in (-6.0, -3.5, -1.2, 1.2):
        r = 0.2 + 0.1 * (z + TREB_LONG) / TREB_LONG
        beam.beam((0.0, 0.0, z - 0.1), (0.0, 0.0, z + 0.1), r, r, 6, "Iron")
    beam.box((0.0, 0.0, TREB_SHORT), (1.3, 0.3, 0.3), "TimberDark")
    beam.build("ArmBeam", arm)
    counter = empty("Counterweight", arm, (0.0, 0.0, TREB_SHORT))
    box = Part()
    for x in (-0.55, 0.55):
        box.beam((x, 0.0, 0.0), (x, -1.2, 0.0), 0.08, 0.08, 4, "Iron")
    box.box((0.0, -2.0, 0.0), (2.2, 1.7, 1.9), "TimberDark")
    for x in (-1.12, 1.12):
        for y in (-1.4, -2.6):
            box.box((x, y, 0.0), (0.06, 0.14, 1.96), "Iron")
    for i, (x, z) in enumerate(
        ((-0.5, -0.4), (0.4, 0.3), (0.0, 0.1), (-0.3, 0.5), (0.5, -0.5))
    ):
        box.sphere((x, -1.05 + 0.08 * (i % 2), z), 0.34, "Stone")
    box.build("CounterweightBox", counter)
    sling = empty("Sling", arm, (0.0, 0.0, -TREB_LONG))
    rope = Part()
    for x in (-0.14, 0.14):
        rope.beam(
            (0.0, 0.0, 0.0), (x, 0.0, TREB_SLING), 0.035, 0.035, 4, "Rope", cap=False
        )
    rope.box((0.0, -0.18, TREB_SLING), (0.7, 0.12, 0.8), "Hide")
    rope.build("SlingRope", sling)
    stone = Part()
    stone.sphere((0.0, 0.0, 0.0), 0.45, "Stone")
    stone.build("Stone", sling, (0.0, 0.3, TREB_SLING))
    return {
        "axle": [0.0, h, 0.0],
        "long_arm": TREB_LONG,
        "short_arm": TREB_SHORT,
        "sling": TREB_SLING,
    }


def mangonel() -> dict:
    """Torsion mangonel.

    ``Arm`` pivots in the twisted skein (rotation about x: 0 = upright,
    negative = drawn back); ``Stone`` sits in the cup; ``Winch`` at the back.
    """
    root = empty("Mangonel")
    frame = Part()
    for x in (-0.85, 0.85):
        frame.box((x, 0.25, 0.0), (0.26, 0.3, 3.6), "TimberDark")
        frame.beam((x, 0.35, 0.8), (x, 2.0, 0.4), 0.1, 0.09)
        frame.beam((x, 0.35, -0.2), (x, 2.0, 0.4), 0.1, 0.09)
        frame.box((x, 0.8, MANGONEL_PIVOT[2]), (0.3, 0.7, 0.5), "TimberDark")
    for z in (-1.6, 0.0, 1.6):
        frame.box((0.0, 0.25, z), (1.96, 0.22, 0.24), "TimberDark")
    frame.box((0.0, 2.0, 0.4), (2.0, 0.24, 0.24))
    frame.box((0.0, 2.0, 0.28), (0.8, 0.3, 0.1), "Hide")  # padded stop
    frame.beam(
        (-0.75, 0.8, MANGONEL_PIVOT[2]),
        (0.75, 0.8, MANGONEL_PIVOT[2]),
        0.26,
        0.26,
        8,
        "Rope",
    )
    frame.build("Frame", root)
    winch = Part()
    winch.beam((-0.8, 0.0, 0.0), (0.8, 0.0, 0.0), 0.16, 0.16, 8)
    for x in (-0.7, 0.7):
        winch.beam((x, -0.5, 0.0), (x, 0.5, 0.0), 0.04, 0.04, 4)
        winch.beam((x, 0.0, -0.5), (x, 0.0, 0.5), 0.04, 0.04, 4)
    winch.build("Winch", root, (0.0, 0.55, -1.55))
    arm = empty("Arm", root, MANGONEL_PIVOT)
    beam = Part()
    beam.beam((0.0, -0.1, 0.0), (0.0, MANGONEL_ARM, 0.0), 0.1, 0.08, 6, "Timber")
    beam.beam(
        (0.0, MANGONEL_ARM + 0.05, 0.0),
        (0.0, MANGONEL_ARM + 0.25, 0.0),
        0.28,
        0.32,
        8,
        "Hide",
    )
    beam.build("ArmBeam", arm)
    stone = Part()
    stone.sphere((0.0, 0.0, 0.0), 0.26, "Stone")
    stone.build("Stone", arm, (0.0, MANGONEL_ARM + 0.38, 0.0))
    return {"pivot": list(MANGONEL_PIVOT), "arm": MANGONEL_ARM}


def bombard() -> dict:
    """Bombard on its timber bed.

    ``Barrel`` recoils along -z; ``Mantlet`` (hinged at the top of
    its posts, rotation about x) is raised to fire.
    """
    root = empty("Bombard")
    bed = Part()
    bed.box((0.0, 0.3, -0.2), (1.3, 0.45, 4.2), "TimberDark")
    bed.box((0.0, 0.65, -0.2), (1.3, 0.25, 0.25))
    for x in (-0.55, 0.55):
        bed.box((x, 0.7, -0.2), (0.2, 0.35, 4.0))
    bed.box((0.0, 0.75, -2.45), (1.5, 1.1, 0.45), "TimberDark")  # backstop
    for x in (-0.6, 0.6):
        bed.beam(
            (x, -0.4, -2.95), (x, 1.2, -2.6), 0.09, 0.07, 5
        )  # stakes behind the backstop
    for x in (-1.35, 1.35):
        bed.beam((x, 0.0, 2.3), (x, 2.5, 2.3), 0.11, 0.1, 6)
    bed.box((0.0, 2.5, 2.3), (2.9, 0.2, 0.2))
    bed.build("Bed", root)
    barrel = Part()
    barrel.beam((0.0, 0.0, -1.9), (0.0, 0.0, -1.35), 0.3, 0.32, 10, "Iron")
    barrel.beam((0.0, 0.0, -1.35), (0.0, 0.0, 1.5), 0.37, 0.36, 12, "Iron")
    for z in (-1.1, -0.5, 0.1, 0.7, 1.3):
        barrel.beam((0.0, 0.0, z - 0.06), (0.0, 0.0, z + 0.06), 0.42, 0.42, 12, "Iron")
    barrel.beam((0.0, 0.0, 1.49), (0.0, 0.0, 1.52), 0.24, 0.24, 10, "TimberDark")
    barrel.build("BarrelTube", empty("Barrel", root, (0.0, 1.05, 0.0)))
    mantlet = Part()
    mantlet.box((0.0, -1.1, 0.0), (2.5, 2.1, 0.14), "Timber")
    for x in (-0.9, 0.0, 0.9):
        mantlet.box((x, -1.1, -0.1), (0.14, 2.1, 0.08), "TimberDark")
    mantlet.build("MantletBoard", empty("Mantlet", root, (0.0, 2.45, 2.42)))
    return {"barrel": [0.0, 1.05, 0.0], "muzzle_z": 1.5}


def ram() -> dict:
    """Ram shed (« chat ») on four wheels.

    ``BeamPivot`` (ropes under the ridge, rotation about x)
    swings the ``Beam`` with its iron head; ``Wheel_*`` turn about x.
    """
    root = empty("Ram")
    shed = Part()
    for x in (-1.45, 1.45):
        for z in (-3.6, -1.2, 1.2, 3.6):
            shed.beam((x, 0.5, z), (x, 2.4, z), 0.12, 0.11, 5)
        shed.box((x, 0.55, 0.0), (0.26, 0.26, 7.6), "TimberDark")
        shed.box((x, 2.4, 0.0), (0.24, 0.24, 7.6), "TimberDark")
    for z in (-3.6, 3.6):
        shed.box((0.0, 0.55, z), (3.1, 0.24, 0.24), "TimberDark")
    for side in (-1.0, 1.0):
        angle = side * math.radians(52.0)
        shed.box(
            (side * 0.82, 3.1, 0.0), (2.3, 0.14, 8.2), "Hide", rot=((0, 0, 1), -angle)
        )
    shed.box((0.0, 3.95, 0.0), (0.3, 0.3, 8.3), "TimberDark")
    shed.build("Shed", root)
    for i, (x, z) in enumerate(
        ((-1.62, -2.4), (1.62, -2.4), (-1.62, 2.4), (1.62, 2.4))
    ):
        wheel = Part()
        wheel.wheel((0.0, 0.0, 0.0), 0.55, 0.26)
        wheel.build(f"Wheel_{i}", root, (x, 0.55, z))
    pivot = empty("BeamPivot", root, (0.0, 3.2, 0.4))
    beam = Part()
    for z in (-2.4, 2.4):
        beam.beam((0.0, 0.0, z), (0.0, -1.9, z), 0.03, 0.03, 4, "Rope", cap=False)
    beam.beam((0.0, -1.95, -4.4), (0.0, -1.95, 4.4), 0.3, 0.26, 8, "Timber")
    beam.beam((0.0, -1.95, 4.4), (0.0, -1.95, 5.3), 0.33, 0.16, 8, "Iron")
    for z in (-3.0, -0.5, 2.0):
        beam.beam((0.0, -1.95, z - 0.08), (0.0, -1.95, z + 0.08), 0.33, 0.33, 8, "Iron")
    beam.build("Beam", pivot)
    return {"beam_pivot": [0.0, 3.2, 0.4], "beam_drop": 1.95, "wheel_radius": 0.55}


def siege_tower() -> dict:
    """Siege tower (beffroi) 12 m high.

    ``Body`` (scaled in height by the renderer),
    ``BridgePivot`` (hinged at the front, rotation about x: -90° raised, 0 lowered), ``Wheel_*``.
    """
    root = empty("SiegeTower")
    h = TOWER_HEIGHT
    body = Part()
    base = TOWER_AXLE
    for x in (-2.4, 2.4):
        for z in (-2.4, 2.4):
            body.box((x, base + h * 0.5, z), (0.4, h, 0.4), "TimberDark")
    for k in range(1, 5):
        y = base + k * h / 4.0
        for z in (-2.4, 2.4):
            body.box((0.0, y, z), (5.2, 0.28, 0.28), "TimberDark")
        for x in (-2.4, 2.4):
            body.box((x, y, 0.0), (0.28, 0.28, 5.2), "TimberDark")
        body.box((0.0, y - 0.1, 0.0), (4.8, 0.12, 4.8), "Timber")
    body.box((0.0, base + 0.2, 0.0), (5.4, 0.4, 5.4), "TimberDark")
    for x in (-2.45, 2.45):  # cross bracing of the lower storeys (open at the back)
        body.beam(
            (x, base + 0.4, -2.3), (x, base + h * 0.5, 2.3), 0.1, 0.1, 4, "Timber"
        )
        body.beam(
            (x, base + 0.4, 2.3), (x, base + h * 0.5, -2.3), 0.1, 0.1, 4, "Timber"
        )
    # Wet hides over the upper half (front and sides), open planks at the back.
    body.box((0.0, base + h * 0.62, 2.55), (5.0, h * 0.62, 0.1), "Hide")
    for x in (-2.55, 2.55):
        body.box((x, base + h * 0.62, 0.0), (0.1, h * 0.62, 5.0), "Hide")
    body.box((0.0, base + h * 0.22, 2.52), (5.0, h * 0.3, 0.08), "Timber")
    for side in range(4):
        for k in range(3):
            along = -1.7 + k * 1.7
            if side == 0:
                center = (along, base + h + 0.55, 2.45)
            elif side == 1:
                center = (along, base + h + 0.55, -2.45)
            elif side == 2:
                center = (2.45, base + h + 0.55, along)
            else:
                center = (-2.45, base + h + 0.55, along)
            size = (0.9, 1.1, 0.2) if side < 2 else (0.2, 1.1, 0.9)
            body.box(center, size, "Timber")
    body.build("Body", root)
    for i, (x, z) in enumerate(
        ((-2.25, -1.8), (2.25, -1.8), (-2.25, 1.8), (2.25, 1.8))
    ):
        wheel = Part()
        wheel.wheel((0.0, 0.0, 0.0), 0.8, 0.36)
        wheel.build(f"Wheel_{i}", root, (x, TOWER_AXLE, z))
    pivot = empty("BridgePivot", root, (0.0, base + h - 3.2, 2.5))
    bridge = Part()
    bridge.box((0.0, 0.0, 2.0), (3.2, 0.24, 4.0), "Timber")
    for x in (-1.5, 1.5):
        bridge.box((x, 0.12, 2.0), (0.16, 0.2, 4.0), "TimberDark")
    for z in (0.8, 2.0, 3.2):
        bridge.box((0.0, 0.14, z), (3.1, 0.06, 0.14), "TimberDark")
    bridge.build("Bridge", pivot)
    return {
        "height": h,
        "axle": TOWER_AXLE,
        "bridge_pivot": [0.0, base + h - 3.2, 2.5],
        "wheel_radius": 0.8,
    }


BUILDERS = {
    "trebuchet": trebuchet,
    "mangonel": mangonel,
    "bombard": bombard,
    "ram": ram,
    "siege_tower": siege_tower,
}


def clear_scene() -> None:
    """Removes every object and mesh."""
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)
    for mesh in list(bpy.data.meshes):
        bpy.data.meshes.remove(mesh)


def triangles() -> int:
    """Triangles of the current scene."""
    count = 0
    for obj in bpy.data.objects:
        if obj.type == "MESH":
            count += sum(len(p.vertices) - 2 for p in obj.data.polygons)
    return count


def main() -> None:
    """Builds and exports every engine, then the manifest."""
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out = (
        Path(argv[0])
        if argv
        else Path(__file__).resolve().parents[2] / "game/assets/models/siege"
    )
    out.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for name, builder in BUILDERS.items():
        for lod in (False, True):
            LOD["on"] = lod
            clear_scene()
            info = builder()
            if lod:
                manifest[name]["lod_triangles"] = triangles()
            else:
                info["triangles"] = triangles()
                manifest[name] = info
            bpy.ops.object.select_all(action="SELECT")
            suffix = "_lod" if lod else ""
            bpy.ops.export_scene.gltf(
                filepath=str(out / f"{name}{suffix}.glb"),
                export_format="GLB",
                use_selection=True,
                export_yup=True,
                export_apply=False,
                export_normals=True,
                export_texcoords=False,
                export_animations=False,
            )
    LOD["on"] = False
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    print("siege engines:", {k: v["triangles"] for k, v in manifest.items()})


if __name__ == "__main__":
    main()
