"""GA3-L5: generated trebuchet and ram cut into the moving parts of the animated siege engines.

Run headless from the repository root::

    blender -b --factory-startup --python tools/blender_scripts/ga3_siege_rig.py -- [OUT_DIR]

Inputs: the cleaned GA3 LODs ``game/assets/models/props_ga/ga3_{trebuchet,ram}_lod{0,2}.glb``
(lot L1/L1b, already UV-mapped and baked) and the procedural rigs of
``siege_engines.py`` (named, pivoted nodes that ``SiegeEnginesFx`` and ``SiegeAssaultFx``
animate). Output (``OUT_DIR``, default ``game/assets/models/siege/``): ``ga3_trebuchet.glb``,
``ga3_trebuchet_lod.glb``, ``ga3_ram.glb``, ``ga3_ram_lod.glb`` with **the same hierarchy and
node names** as the procedural models, and ``ga3_rigs.json`` (pivots, scales, triangles).

Method (ADR 0140, section L5): every face of the GA3 mesh is labelled by its centroid (regions
measured once on LOD0: arm line fitted by PCA, posts, counterweight box, wheels, hanging beam),
then each label is re-expressed in the local frame of the procedural node it dresses, and that
node's mesh data is replaced (UVs and the baked texture are kept, no re-bake):

* Trebuchet: ``Frame`` = GA3 frame, scaled so the post tops (bearing) sit on the procedural
  axle (6.4 m), mirrored lengthwise so its long base lies under the long arm, the two side
  frames spread apart (smooth gap) so the counterweight box swings between the posts;
  ``ArmBeam`` = GA3 arm, uniformly scaled to the procedural arm length with its pivot moved
  along it to the procedural long/short ratio (8.5 / 2.2 m), plus a procedural iron axle;
  ``CounterweightBox`` = GA3 box and hanger, scaled to the procedural box. The GA3 sling bag
  (stones always inside) is dropped: ``Sling``/``Stone`` and the ``Winch`` stay procedural,
  with procedural winch posts added to ``Frame``.
* Ram: ``Shed`` = GA3 shed (a spurious centre wheel under the bed is removed), ``Wheel_i`` =
  the four GA3 wheels on their own centres, ``BeamPivot`` moved to the top of the GA3 hangers
  and ``Beam`` = GA3 beam, iron head and hangers.

Geometry is exported in Godot axes like ``siege_engines.py`` (x right, y up, +z towards the
target). LOD0 feeds the near model, LOD2 the ``_lod`` model (with the procedural parts in their
SG3 simplified form).
"""

import json
import math
import sys
from pathlib import Path

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))
import siege_engines as se  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PROPS = ROOT / "game" / "assets" / "models" / "props_ga"
LODS = (("", 0), ("_lod", 2))  # suffix of the output, GA3 LOD used

# Procedural targets (Godot metres, ``siege_engines.py``).
TREB_ARM_TOTAL = se.TREB_LONG + se.TREB_SHORT + 0.2  # long tip to short tip
BOX_WIDTH = 1.8  # counterweight box across the arm (Godot x)
BOX_DEPTH = 1.9  # along the arm (Godot z)
BOX_BOTTOM = -2.85  # below the hinge, as the procedural box
POST_CLEARANCE = 1.05  # inner face of the posts from the arm plane, after scaling
GAP_BLEND = 0.25  # half-width (GA3 m) of the smooth spread between the side frames
RAM_SCALE = 1.15
RAM_HEAD_Z = 5.3  # tip of the iron head (procedural ram)
LOG_BOTTOM = 1.78  # GA3 m: log faces above the bed top (1.66-1.72)
HANGER_HALF = 0.35  # GA3 m: half-width of a hanger along the ram


# --- Helpers ----------------------------------------------------------------------------


def import_ga3(name: str, lod: int) -> bpy.types.Mesh:
    """Imported GA3 LOD as a standalone mesh in world coordinates (import objects removed)."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(PROPS / f"ga3_{name}_lod{lod}.glb"))
    new = [o for o in bpy.data.objects if o not in before]
    source = next(o for o in new if o.type == "MESH")
    mesh = source.data.copy()
    mesh.transform(source.matrix_world)
    for obj in new:
        bpy.data.objects.remove(obj, do_unlink=True)
    return mesh


def rename_material(mesh: bpy.types.Mesh, name: str) -> None:
    """Deterministic material and texture names (Godot caches siege materials by name)."""
    for mat in mesh.materials:
        mat.name = name
        if mat.node_tree is not None:
            for node in mat.node_tree.nodes:
                if node.type == "TEX_IMAGE" and node.image is not None:
                    node.image.name = f"{name}_{node.label or node.name}".replace(
                        " ", ""
                    )


def centroids(mesh: bpy.types.Mesh) -> list[Vector]:
    """Face centres (world)."""
    return [p.center.copy() for p in mesh.polygons]


def piece(mesh: bpy.types.Mesh, faces: set[int], mapping, target) -> bpy.types.Mesh:
    """Faces ``faces`` of ``mesh`` mapped by ``mapping`` (world to world), in ``target``'s frame."""
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bm.faces.ensure_lookup_table()
    bmesh.ops.delete(
        bm, geom=[f for f in bm.faces if f.index not in faces], context="FACES"
    )
    inverse = target.matrix_world.inverted()
    for vert in bm.verts:
        vert.co = inverse @ mapping(vert.co.copy())
    bm.normal_update()
    out = bpy.data.meshes.new(f"{target.name}_ga3")
    bm.to_mesh(out)
    bm.free()
    for mat in mesh.materials:
        out.materials.append(mat)
    return out


def dress(target, mesh: bpy.types.Mesh) -> None:
    """Replaces the mesh of a procedural node (name, pivot and parent kept)."""
    old = target.data
    target.data = mesh
    if old is not None and old.users == 0:
        bpy.data.meshes.remove(old)


def godot_to_blender(x: float, y: float, z: float) -> Vector:
    """Godot vector to Blender axes (as ``siege_engines.g2b``)."""
    return Vector((x, -z, y))


def percentile(values: list[float], q: float) -> float:
    """Percentile ``q`` (0-1) of ``values`` (nearest rank)."""
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, max(0, int(q * (len(ordered) - 1) + 0.5)))]


def triangles() -> int:
    """Triangles of the scene's mesh objects."""
    return sum(
        sum(len(p.vertices) - 2 for p in o.data.polygons)
        for o in bpy.data.objects
        if o.type == "MESH"
    )


def export(path: Path) -> None:
    """Exports the scene (Godot axes), textures embedded."""
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_yup=True,
        export_apply=False,
        export_normals=True,
        export_texcoords=True,
        export_animations=False,
    )


def reset() -> None:
    """Empty scene, orphan data purged (no ``.001`` duplicates between builds)."""
    se.clear_scene()
    for _ in range(3):
        bpy.ops.outliner.orphans_purge(do_local_ids=True, do_recursive=True)


# --- Trebuchet --------------------------------------------------------------------------


def measure_trebuchet(cs: list[Vector]) -> dict:
    """Arm line (PCA on the side plane), posts and counterweight box of the GA3 trebuchet."""
    a, b = (
        Vector((-5.8, 10.66)),
        Vector((3.86, 4.8)),
    )  # rough arm ends (x, z), LOD0 views

    def seg_dist(p: Vector) -> float:
        ab = b - a
        t = max(0.0, min(1.0, (p - a).dot(ab) / ab.length_squared))
        return (p - (a + ab * t)).length

    sel = [c for c in cs if abs(c.y) < 0.45 and seg_dist(Vector((c.x, c.z))) < 0.6]
    mean = Vector((sum(c.x for c in sel), sum(c.z for c in sel))) / len(sel)
    sxx = sum((c.x - mean.x) ** 2 for c in sel)
    szz = sum((c.z - mean.y) ** 2 for c in sel)
    sxz = sum((c.x - mean.x) * (c.z - mean.y) for c in sel)
    angle = 0.5 * math.atan2(2.0 * sxz, sxx - szz)
    d = Vector((math.cos(angle), math.sin(angle)))
    if d.x < 0.0:
        d = -d
    n = Vector((-d.y, d.x))
    ta, tb = (a - mean).dot(d), (b - mean).dot(d)
    measured = {"mean": mean, "d": d, "n": n, "ta": ta, "tb": tb}
    arm = [c for c in cs if on_arm(c, measured)]
    ts = [(Vector((c.x, c.z)) - mean).dot(d) for c in arm]
    posts = [c for c in cs if 0.45 < abs(c.y) < 1.15 and 4.5 < c.z < 6.8 and c.x > -3.0]
    x_post = percentile([c.x for c in posts], 0.5)
    box = [
        c
        for c in cs
        if c.x > 1.7
        and 1.55 < c.z < 4.95
        and abs(c.y) < 2.1
        and not on_arm(c, measured)
    ]
    measured.update(
        {
            "t_min": percentile(ts, 0.005),
            "t_max": percentile(ts, 0.995),
            "y_arm": percentile([c.y for c in arm], 0.5),
            "x_post": x_post,
            "post_inner": percentile([abs(c.y) for c in posts], 0.1),
            "axle_z": mean.y + (x_post - mean.x) * d.y / d.x,
            "box": (
                min(c.x for c in box),
                max(c.x for c in box),
                min(c.y for c in box),
                max(c.y for c in box),
                min(c.z for c in box),
                max(c.z for c in box),
            ),
        }
    )
    return measured


def arm_coords(c: Vector, m: dict) -> Vector:
    """(along the arm from its centre towards the short end, across it upwards) in GA3 m."""
    p = Vector((c.x, c.z)) - m["mean"]
    return Vector((p.dot(m["d"]), p.dot(m["n"])))


def on_arm(c: Vector, m: dict) -> bool:
    """Face centre on the GA3 throwing arm (beam, iron bands, end hooks)."""
    t, v = arm_coords(c, m)
    return abs(c.y) < 0.55 and abs(v) < 0.55 and m["ta"] - 0.4 < t < m["tb"] + 0.4


def trebuchet(mesh: bpy.types.Mesh, m: dict) -> dict:
    """Dresses the procedural trebuchet rig (built in the scene) with the GA3 pieces."""
    objs = {o.name: o for o in bpy.data.objects}
    bx0, bx1, by0, by1, bz0, bz1 = m["box"]
    labels = {"frame": set(), "arm": set(), "box": set()}
    for i, c in enumerate(centroids(mesh)):
        if on_arm(c, m):
            labels["arm"].add(i)
        elif bx0 - 0.05 <= c.x and bz0 - 0.05 <= c.z <= bz1 + 0.05 and abs(c.y) < 2.1:
            labels["box"].add(i)
        elif c.x < -4.2 and c.z > 3.0:
            continue  # sling bag with its stones: the procedural sling is kept
        else:
            labels["frame"].add(i)

    # Frame: bearings on the procedural axle, long base under the long arm, side frames spread.
    s_f = se.TREB_AXLE / m["axle_z"]
    gap = max(0.0, POST_CLEARANCE / s_f - m["post_inner"])

    def frame_map(p: Vector) -> Vector:
        y = p.y + gap * max(-1.0, min(1.0, p.y / GAP_BLEND))
        return godot_to_blender(-y * s_f, p.z * s_f, -(p.x - m["x_post"]) * s_f)

    dress(objs["Frame"], piece(mesh, labels["frame"], frame_map, objs["Frame"]))
    winch = se.Part()
    for x in (-1.6, 1.6):
        winch.box((x, 0.8, -6.2), (0.3, 1.6, 0.3), "TimberDark")
        winch.box((x, 0.12, -5.5), (0.3, 0.24, 1.7), "TimberDark")
    winch.build("WinchFrame", objs["Trebuchet"])

    # Arm: uniform scale to the procedural length, pivot moved along it (8.5 / 2.2 m).
    s_a = TREB_ARM_TOTAL / (m["t_max"] - m["t_min"])
    t_pivot = m["t_min"] + se.TREB_LONG / s_a
    arm_origin = objs["Arm"].matrix_world.translation.copy()

    def arm_map(p: Vector) -> Vector:
        t, v = arm_coords(p, m)
        g = godot_to_blender((p.y - m["y_arm"]) * s_a, v * s_a, (t - t_pivot) * s_a)
        return arm_origin + g

    dress(objs["ArmBeam"], piece(mesh, labels["arm"], arm_map, objs["ArmBeam"]))
    half = (m["post_inner"] + gap) * s_f + 0.35
    axle = se.Part()
    axle.beam((-half, 0.0, 0.0), (half, 0.0, 0.0), 0.12, 0.12, 8, "Iron")
    axle.build("Axle", objs["Arm"])

    # Counterweight: GA3 box and hanger scaled to the procedural box, hanging from the hinge.
    hinge = objs["Counterweight"].matrix_world.translation.copy()
    sx = BOX_WIDTH / (by1 - by0)
    sz = BOX_DEPTH / (bx1 - bx0)
    sy = (-0.05 - BOX_BOTTOM) / (bz1 - bz0)
    cx, cy = (bx0 + bx1) * 0.5, (by0 + by1) * 0.5

    def box_map(p: Vector) -> Vector:
        return hinge + godot_to_blender(
            (p.y - cy) * sx, (p.z - bz1) * sy - 0.05, (p.x - cx) * sz
        )

    target = objs["CounterweightBox"]
    dress(target, piece(mesh, labels["box"], box_map, target))
    return {
        "frame_scale": round(s_f, 4),
        "side_gap_m": round(gap * s_f, 3),
        "arm_scale": round(s_a, 4),
        "arm_pivot_from_long_tip_m": round((t_pivot - m["t_min"]) * s_a, 3),
        "box_scale": [round(sx, 3), round(sy, 3), round(sz, 3)],
        "faces": {k: len(v) for k, v in labels.items()},
    }


# --- Ram --------------------------------------------------------------------------------


def measure_ram(cs: list[Vector]) -> dict:
    """Spurious centre wheel, the four wheels and the hanging beam of the GA3 ram."""
    low = [c for c in cs if abs(c.y) < 0.7 and c.z < 0.6]
    debris = (min(c.x for c in low) - 0.3, max(c.x for c in low) + 0.3)
    wheels = []
    for guess in (-2.6, 1.4):
        for side in (-1.0, 1.0):
            disc = [
                c
                for c in cs
                if side * c.y > 0.95 and c.z < 1.25 and abs(c.x - guess) < 1.0
            ]
            x0, x1 = min(c.x for c in disc), max(c.x for c in disc)
            r = (x1 - x0) * 0.5
            bottom = min(c.z for c in disc)
            below = [c for c in disc if c.z < bottom + r]
            wheels.append(
                {
                    "x": (x0 + x1) * 0.5,
                    "z": bottom + r,
                    "r": r,
                    "y": (min(c.y for c in below), max(c.y for c in below)),
                }
            )
    # Two hangers (ropes or chains) from the ridge to the log, away from the gable fringes.
    mid = [c for c in cs if abs(c.y) < 0.5 and 2.5 < c.z < 3.5 and abs(c.x) < 3.1]
    split = percentile([c.x for c in mid], 0.5)
    hangers = [
        sum(c.x for c in side) / len(side)
        for side in ([c for c in mid if c.x < split], [c for c in mid if c.x >= split])
    ]
    tops = [
        c.z
        for c in cs
        if abs(c.y) < 0.5
        and c.z < 3.85
        and min(abs(c.x - h) for h in hangers) < HANGER_HALF
    ]
    return {
        "debris": debris,
        "wheels": wheels,
        "hangers": hangers,
        "anchor": ((hangers[0] + hangers[1]) * 0.5, max(tops)),
    }


def on_ram_beam(c: Vector, m: dict) -> bool:
    """Face centre on the GA3 ram log, its iron head or its two hangers."""
    if abs(c.y) < 0.65 and LOG_BOTTOM < c.z < 2.45:
        return True
    near = min(abs(c.x - h) for h in m["hangers"]) < HANGER_HALF
    return near and abs(c.y) < 0.5 and 2.45 <= c.z < 3.85


def ram(mesh: bpy.types.Mesh, m: dict) -> dict:
    """Dresses the procedural ram rig (built in the scene) with the GA3 pieces."""
    objs = {o.name: o for o in bpy.data.objects}
    labels = {"shed": set(), "beam": set()}
    wheel_faces = [set() for _ in m["wheels"]]
    for i, c in enumerate(centroids(mesh)):
        if abs(c.y) < 0.75 and c.z < 1.2 and m["debris"][0] < c.x < m["debris"][1]:
            continue  # spurious centre wheel and post under the bed
        wheel = -1
        for k, w in enumerate(m["wheels"]):
            inside = (c.x - w["x"]) ** 2 + (c.z - w["z"]) ** 2 < (w["r"] * 1.08) ** 2
            if inside and w["y"][0] - 0.06 <= c.y <= w["y"][1] + 0.06:
                wheel = k
        if wheel >= 0:
            wheel_faces[wheel].add(i)
        elif on_ram_beam(c, m):
            labels["beam"].add(i)
        else:
            labels["shed"].add(i)
    if "ground" not in m:  # measured on LOD0, shared by the simplified model
        kept = labels["shed"] | labels["beam"] | set().union(*wheel_faces)
        verts = [mesh.vertices[v].co for i in kept for v in mesh.polygons[i].vertices]
        m["ground"] = min(co.z for co in verts)
        m["head"] = max(co.x for co in verts)
    ground, head = m["ground"], m["head"]
    s = RAM_SCALE
    off = RAM_HEAD_Z / s - head

    def ram_map(p: Vector) -> Vector:
        return godot_to_blender(p.y * s, (p.z - ground) * s, (p.x + off) * s)

    dress(objs["Shed"], piece(mesh, labels["shed"], ram_map, objs["Shed"]))
    wheel_objs = [objs[f"Wheel_{i}"] for i in range(4)]
    used = set()
    for k, w in enumerate(m["wheels"]):
        centre = ram_map(Vector((w["x"], 0.5 * (w["y"][0] + w["y"][1]), w["z"])))
        target = min(
            (o for o in wheel_objs if o.name not in used),
            key=lambda o, c=centre: (o.matrix_world.translation - c).length,
        )
        used.add(target.name)
        target.location = centre  # parent (root) at the origin, unrotated
        bpy.context.view_layer.update()
        dress(target, piece(mesh, wheel_faces[k], ram_map, target))
    anchor = ram_map(Vector((m["anchor"][0], 0.0, m["anchor"][1])))
    objs["BeamPivot"].location = anchor
    bpy.context.view_layer.update()
    dress(objs["Beam"], piece(mesh, labels["beam"], ram_map, objs["Beam"]))
    beam_y = [
        ram_map(mesh.vertices[v].co.copy()).z
        for i in labels["beam"]
        for v in mesh.polygons[i].vertices
    ]
    return {
        "scale": s,
        "beam_pivot": [round(anchor.x, 3), round(anchor.z, 3), round(-anchor.y, 3)],
        "beam_drop": round(anchor.z - percentile(beam_y, 0.02), 3),
        "wheel_radius": round(sum(w["r"] for w in m["wheels"]) / 4.0 * s, 3),
        "faces": {
            "shed": len(labels["shed"]),
            "beam": len(labels["beam"]),
            "wheels": [len(f) for f in wheel_faces],
        },
    }


# --- Main -------------------------------------------------------------------------------


def build(name: str, rig, measure, dresser, out: Path, report: dict) -> None:
    """Both levels of detail of one engine; regions measured on LOD0."""
    measured = None
    for suffix, lod in LODS:
        se.LOD["on"] = suffix != ""
        reset()
        mesh = import_ga3(name, lod)
        rename_material(mesh, "Ga3" + name.capitalize() + ("Lod" if suffix else ""))
        if measured is None:
            measured = measure(centroids(mesh))
        rig()
        info = dresser(mesh, measured)
        bpy.data.meshes.remove(mesh)
        info["triangles"] = triangles()
        report[f"ga3_{name}{suffix}"] = info
        export(out / f"ga3_{name}{suffix}.glb")
    se.LOD["on"] = False


def main() -> None:
    """Builds both rigs at both levels of detail."""
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out = Path(argv[0]) if argv else ROOT / "game" / "assets" / "models" / "siege"
    out.mkdir(parents=True, exist_ok=True)
    report = {}
    build("trebuchet", se.trebuchet, measure_trebuchet, trebuchet, out, report)
    build("ram", se.ram, measure_ram, ram, out, report)
    (out / "ga3_rigs.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print("ga3 siege rigs:", json.dumps(report))


if __name__ == "__main__":
    main()
