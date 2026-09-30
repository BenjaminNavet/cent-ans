"""Campaign-map tree impostor atlases (lot FC2).

Run headless (from the repository root)::

    blender --background --python tools/blender_scripts/campaign_tree_impostors.py -- \
        game/assets/textures/vegetation [seed]

Builds richer *source* trees used only for baking (never exported as meshes): a tapered trunk,
limbs, and a crown of textured leaf cards (oak, beech: ``leaf_spray.png``, CC0 procedural; fir:
the CC0 Poly Haven fir twig) coloured with the palette of ``VegetationMeshes.PALETTES``. Each
essence is rendered from ``VIEWS`` azimuths at ``ELEVATION_DEG`` (campaign camera pitch at the
distances where impostors are shown) with an orthographic camera, then assembled into two
atlases (one row per essence in ``ESSENCE_ORDER``, one column per view, ``CELL`` px square):

* ``campaign_impostors_albedo.png``: straight-alpha albedo (sRGB), colours bled into the
  transparent pixels so that mipmaps do not darken the silhouettes;
* ``campaign_impostors_normal.png``: RGB = normal in the view frame of the bake camera
  (x right, y up, z towards the camera) encoded ``n * 0.5 + 0.5``; A = ambient occlusion.

Every render is emission only (no lighting), on a black background, "Raw" view transform into
16-bit PNG: pass 0 = albedo (premultiplied by coverage), pass 1 = encoded normal, pass 2 =
(AO, coverage). Renders are made at ``SUPERSAMPLE`` times the cell size and box-filtered down.

Framing (must match ``campaign_tree_impostor.gdshader``): tree height 1.0, foot at the origin;
a cell covers ``ORTHO`` units square; the foot sits at ``FOOT`` of the cell height from the
bottom, horizontally centred. View k looks from azimuth k / VIEWS * tau (Blender XY, Z up).
Deterministic for a given seed. Prints ``ATLAS <path> <w>x<h>`` and ``OK``.
"""

import math
import random
import struct
import sys
import zlib
from pathlib import Path

import bmesh
import bpy
import numpy as np
from mathutils import Vector

REPO = Path(__file__).resolve().parents[2]
LEAF_TEXTURE = REPO / "game/assets/textures/battle/leaf_spray.png"
FIR_TEXTURE = REPO / "game/assets/third_party/vegetation/fir_tree_01/fir_tree_01_fir_tree_01_twig_diff_alpha.png"

ESSENCE_ORDER = ("oak", "beech", "fir")
VIEWS = 8
ELEVATION_DEG = 35.0
CELL = 256
SUPERSAMPLE = 2
ORTHO = 1.7
CENTER_UP = 0.41  # image centre, along the camera up vector, from the foot
FOOT = 0.5 - CENTER_UP / ORTHO

# Linear albedo, same values as VegetationMeshes.PALETTES (dark leaf, light leaf, bark).
PALETTES = {
    "oak": ((0.040, 0.068, 0.026), (0.118, 0.155, 0.056), (0.20, 0.15, 0.10)),
    "beech": ((0.052, 0.090, 0.030), (0.150, 0.200, 0.064), (0.30, 0.29, 0.26)),
    "fir": ((0.022, 0.058, 0.046), (0.065, 0.120, 0.075), (0.22, 0.14, 0.09)),
}


# --- Geometry -------------------------------------------------------------------------------


class TreeBuilder:
    """Accumulates bark and leaf-card faces with their colour and crown ('radial') direction."""

    def __init__(self, rng: random.Random, palette: tuple) -> None:
        self.rng = rng
        self.palette = palette
        self.bark = bmesh.new()
        self.leaves = bmesh.new()
        self.bark_data: list[tuple] = []
        self.leaf_data: list[tuple] = []

    def cylinder(self, base: Vector, top: Vector, r0: float, r1: float, sides: int = 7) -> None:
        """Tapered open cylinder (bark)."""
        axis = (top - base).normalized()
        ref = Vector((1, 0, 0)) if abs(axis.x) < 0.9 else Vector((0, 1, 0))
        u = axis.cross(ref).normalized()
        w = axis.cross(u)
        rings = []
        for center, r in ((base, r0), (top, r1)):
            rings.append([self.bark.verts.new(center + (u * math.cos(a) + w * math.sin(a)) * r) for a in (i / sides * math.tau for i in range(sides))])
        bark = Vector(self.palette[2])
        for i in range(sides):
            j = (i + 1) % sides
            face = self.bark.faces.new((rings[0][i], rings[0][j], rings[1][j], rings[1][i]))
            self.bark_data.append((face, bark * self.rng.uniform(0.8, 1.1), None))

    def card(self, start: Vector, normal: Vector, along: Vector, width: float, length: float, color: Vector, radial: Vector, droop: float = -1.0) -> None:
        """Leaf card: quad centred on ``start`` or, with ``droop`` >= 0, growing from it."""
        normal = normal.normalized()
        along = (along - normal * along.dot(normal)).normalized()
        side = normal.cross(along).normalized()
        if droop >= 0.0:
            p0, p1 = start, start + along * length + Vector((0, 0, -droop))
        else:
            p0, p1 = start - along * length * 0.5, start + along * length * 0.5
        corners = [p0 - side * width * 0.5, p0 + side * width * 0.5, p1 + side * width * 0.5, p1 - side * width * 0.5]
        face = self.leaves.faces.new([self.leaves.verts.new(c) for c in corners])
        self.leaf_data.append((face, color, radial))

    def leaf_color(self, exposure: float) -> Vector:
        dark, light = Vector(self.palette[0]), Vector(self.palette[1])
        return dark.lerp(light, max(0.0, min(1.0, exposure * 0.8 + 0.2 * self.rng.random())))


def random_unit(rng: random.Random) -> Vector:
    while True:
        v = Vector((rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-1, 1)))
        if 0.05 < v.length <= 1.0:
            return v.normalized()


def broadleaf(builder: TreeBuilder, lumps: list, trunk_top: float, trunk_r: tuple, clusters: int, cards: tuple, card_size: tuple, crown_center: Vector) -> None:
    """Trunk, limbs towards some cluster centres, and clustered leaf cards on the lumps' shells."""
    rng = builder.rng
    builder.cylinder(Vector((0, 0, -0.05)), Vector((0, 0, trunk_top)), trunk_r[0], trunk_r[1], 8)
    total_volume = sum(r.x * r.y * r.z for _, r in lumps)
    points = []
    for center, radii in lumps:
        count = max(4, round(clusters * radii.x * radii.y * radii.z / total_volume))
        for _ in range(count):
            d = random_unit(rng)
            shell = rng.uniform(0.62, 1.0) if rng.random() < 0.8 else rng.uniform(0.25, 0.62)
            points.append(center + Vector((d.x * radii.x, d.y * radii.y, d.z * radii.z)) * shell)
    top = Vector((0, 0, trunk_top))
    for k, point in enumerate(points):
        if k % 3 == 0:  # limbs, seen through the gaps of the crown
            start = top + Vector((0, 0, rng.uniform(-0.12, 0.0)))
            mid = start.lerp(point, 0.55) + Vector((0, 0, 0.04))
            builder.cylinder(start, mid, trunk_r[1] * 0.55, trunk_r[1] * 0.32, 5)
            builder.cylinder(mid, point, trunk_r[1] * 0.32, trunk_r[1] * 0.12, 4)
    for point in points:
        radial = point - crown_center
        outward = radial.normalized()
        # Upper clusters lighter, inner clusters darker.
        exposure = (0.5 + 0.5 * outward.z) * (0.7 + 0.3 * min(1.0, radial.length / 0.4))
        color = builder.leaf_color(exposure)
        for _ in range(rng.randint(*cards)):
            offset = random_unit(rng) * card_size[0] * 0.7
            normal = outward * 0.6 + random_unit(rng) * 0.8 + Vector((0, 0, 0.35))
            size = rng.uniform(*card_size)
            builder.card(point + offset, normal, random_unit(rng), size, size, color * rng.uniform(0.85, 1.12), outward)


def oak(builder: TreeBuilder) -> None:
    """Pedunculate oak: broad irregular crown of five lumps, short thick trunk."""
    lumps = [
        (Vector((0.0, 0.0, 0.72)), Vector((0.36, 0.36, 0.25))),
        (Vector((0.30, 0.05, 0.58)), Vector((0.26, 0.25, 0.2))),
        (Vector((-0.27, 0.17, 0.60)), Vector((0.27, 0.25, 0.2))),
        (Vector((0.02, -0.30, 0.56)), Vector((0.25, 0.24, 0.19))),
        (Vector((-0.12, -0.12, 0.48)), Vector((0.22, 0.22, 0.16))),
    ]
    broadleaf(builder, lumps, 0.42, (0.07, 0.045), 280, (5, 8), (0.09, 0.14), Vector((0, 0, 0.6)))


def beech(builder: TreeBuilder) -> None:
    """Beech: taller rounder crown, slender grey trunk."""
    lumps = [
        (Vector((0.0, 0.0, 0.68)), Vector((0.36, 0.36, 0.31))),
        (Vector((0.04, 0.02, 0.88)), Vector((0.25, 0.25, 0.13))),
        (Vector((0.19, -0.1, 0.52)), Vector((0.22, 0.22, 0.19))),
        (Vector((-0.2, 0.12, 0.52)), Vector((0.22, 0.22, 0.19))),
    ]
    broadleaf(builder, lumps, 0.55, (0.05, 0.03), 250, (5, 8), (0.09, 0.13), Vector((0, 0, 0.66)))


def fir(builder: TreeBuilder) -> None:
    """Silver fir: whorls of drooping fronds (twig cards) along a straight trunk."""
    rng = builder.rng
    builder.cylinder(Vector((0, 0, -0.05)), Vector((0, 0, 0.96)), 0.035, 0.01, 7)
    whorls = 18
    for i in range(whorls):
        t = i / (whorls - 1)
        z = 0.12 + t * 0.8
        radius = 0.3 * (1.0 - t) ** 0.9 + 0.035
        fronds = max(6, round(14 - 7 * t))
        twist = rng.uniform(0, math.tau)
        for k in range(fronds):
            a = twist + k / fronds * math.tau + rng.uniform(-0.2, 0.2)
            out = Vector((math.cos(a), math.sin(a), 0.0))
            length = radius * rng.uniform(0.85, 1.1)
            color = builder.leaf_color(0.35 + 0.65 * t) * rng.uniform(0.85, 1.1)
            normal = Vector((0, 0, 1)) + out * 0.25 + random_unit(rng) * 0.2
            start = Vector((0, 0, z)) + out * 0.01
            width = 0.09 + 0.06 * (1.0 - t)
            builder.card(start, normal, out, width, length, color, (out + Vector((0, 0, 0.5))).normalized(), droop=length * 0.35)
            # Second, shorter layer rotated a little: denser frond.
            out2 = Vector((math.cos(a + 0.18), math.sin(a + 0.18), 0.0))
            builder.card(start + Vector((0, 0, 0.012)), normal, out2, width * 0.8, length * 0.7, color * 1.08, (out2 + Vector((0, 0, 0.6))).normalized(), droop=length * 0.2)
    tip = builder.leaf_color(1.0)
    for k in range(3):
        a = k / 3 * math.pi
        builder.card(Vector((0, 0, 0.9)), Vector((math.cos(a), math.sin(a), 0)), Vector((0, 0, 1)), 0.05, 0.12, tip, Vector((0, 0, 1)))


BUILDERS = {"oak": oak, "beech": beech, "fir": fir}


# --- Materials ------------------------------------------------------------------------------


def srgb_to_linear(c: np.ndarray) -> np.ndarray:
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c: np.ndarray) -> np.ndarray:
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)


def texture_mean(image: bpy.types.Image) -> np.ndarray:
    """Mean linear RGB of the opaque texels (the texture only modulates the palette)."""
    px = np.array(image.pixels[:], dtype=np.float32).reshape(-1, 4)
    opaque = px[px[:, 3] > 0.5, :3]
    return srgb_to_linear(opaque).mean(axis=0) if len(opaque) else np.ones(3)


MODE_NODES: list = []


def _vmath(nodes, op: str):
    node = nodes.new("ShaderNodeVectorMath")
    node.operation = op
    return node


def make_material(name: str, texture: Path | None) -> bpy.types.Material:
    """Emission material: albedo / encoded view normal / (AO, coverage) selected by the mode."""
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    nodes.clear()
    out = nodes.new("ShaderNodeOutputMaterial")
    color_attr = nodes.new("ShaderNodeAttribute")
    color_attr.attribute_name = "albedo"
    albedo = color_attr.outputs["Color"]
    alpha = None
    if texture is not None:
        image = bpy.data.images.load(str(texture), check_existing=True)
        tex = nodes.new("ShaderNodeTexImage")
        tex.image = image
        norm = _vmath(nodes, "MULTIPLY")
        norm.inputs[1].default_value = tuple(float(1.0 / max(m, 1e-3)) for m in texture_mean(image))
        links.new(tex.outputs["Color"], norm.inputs[0])
        mul = _vmath(nodes, "MULTIPLY")
        links.new(norm.outputs[0], mul.inputs[0])
        links.new(albedo, mul.inputs[1])
        albedo = mul.outputs[0]
        alpha = tex.outputs["Alpha"]
    # Normal: rounded crown normal ('radial') plus a little of the card normal; bark: geometric.
    geo = nodes.new("ShaderNodeNewGeometry")
    radial_attr = nodes.new("ShaderNodeAttribute")
    radial_attr.attribute_name = "radial"
    weight_attr = nodes.new("ShaderNodeAttribute")
    weight_attr.attribute_name = "radial_weight"
    unpack = _vmath(nodes, "MULTIPLY_ADD")
    links.new(radial_attr.outputs["Vector"], unpack.inputs[0])
    unpack.inputs[1].default_value = (2.0, 2.0, 2.0)
    unpack.inputs[2].default_value = (-1.0, -1.0, -1.0)
    scaled_geo = _vmath(nodes, "SCALE")
    scaled_geo.inputs["Scale"].default_value = 0.35
    links.new(geo.outputs["Normal"], scaled_geo.inputs[0])
    rounded = _vmath(nodes, "ADD")
    links.new(unpack.outputs[0], rounded.inputs[0])
    links.new(scaled_geo.outputs[0], rounded.inputs[1])
    blend = nodes.new("ShaderNodeMix")
    blend.data_type = "VECTOR"
    links.new(weight_attr.outputs["Fac"], blend.inputs["Factor"])
    links.new(geo.outputs["Normal"], blend.inputs["A"])
    links.new(rounded.outputs[0], blend.inputs["B"])
    normalize = _vmath(nodes, "NORMALIZE")
    links.new(blend.outputs["Result"], normalize.inputs[0])
    to_cam = nodes.new("ShaderNodeVectorTransform")
    to_cam.vector_type = "NORMAL"
    to_cam.convert_from = "WORLD"
    to_cam.convert_to = "CAMERA"
    links.new(normalize.outputs[0], to_cam.inputs[0])
    # Cycles camera space: x right, y up, +z forward (into the scene): z flipped so that the
    # stored normal points towards the camera.
    encode = _vmath(nodes, "MULTIPLY_ADD")
    links.new(to_cam.outputs[0], encode.inputs[0])
    encode.inputs[1].default_value = (0.5, 0.5, -0.5)
    encode.inputs[2].default_value = (0.5, 0.5, 0.5)
    ao = nodes.new("ShaderNodeAmbientOcclusion")
    ao.samples = 16
    ao.inputs["Distance"].default_value = 0.1
    ao_pack = nodes.new("ShaderNodeCombineXYZ")
    links.new(ao.outputs["AO"], ao_pack.inputs["X"])
    ao_pack.inputs["Y"].default_value = 1.0
    mode = nodes.new("ShaderNodeValue")
    MODE_NODES.append(mode)
    selects = []
    for value in (1.0, 2.0):
        cmp = nodes.new("ShaderNodeMath")
        cmp.operation = "COMPARE"
        cmp.inputs[1].default_value = value
        cmp.inputs[2].default_value = 0.1
        links.new(mode.outputs[0], cmp.inputs[0])
        selects.append(cmp)
    pick1 = nodes.new("ShaderNodeMix")
    pick1.data_type = "VECTOR"
    links.new(selects[0].outputs[0], pick1.inputs["Factor"])
    links.new(albedo, pick1.inputs["A"])
    links.new(encode.outputs[0], pick1.inputs["B"])
    pick2 = nodes.new("ShaderNodeMix")
    pick2.data_type = "VECTOR"
    links.new(selects[1].outputs[0], pick2.inputs["Factor"])
    links.new(pick1.outputs["Result"], pick2.inputs["A"])
    links.new(ao_pack.outputs[0], pick2.inputs["B"])
    emit = nodes.new("ShaderNodeEmission")
    links.new(pick2.outputs["Result"], emit.inputs["Color"])
    if alpha is None:
        links.new(emit.outputs[0], out.inputs["Surface"])
        return mat
    cutout = nodes.new("ShaderNodeMath")
    cutout.operation = "GREATER_THAN"
    cutout.inputs[1].default_value = 0.4
    links.new(alpha, cutout.inputs[0])
    transparent = nodes.new("ShaderNodeBsdfTransparent")  # white: the cards behind show through
    mix = nodes.new("ShaderNodeMixShader")
    links.new(cutout.outputs[0], mix.inputs["Fac"])
    links.new(transparent.outputs[0], mix.inputs[1])
    links.new(emit.outputs[0], mix.inputs[2])
    links.new(mix.outputs[0], out.inputs["Surface"])
    return mat


# --- Scene and bake -------------------------------------------------------------------------


def to_object(name: str, bm: bmesh.types.BMesh, data: list, material: bpy.types.Material, uv: bool) -> bpy.types.Object:
    """Mesh object with per-corner 'albedo', 'radial', 'radial_weight' attributes (card UVs)."""
    uv_layer = bm.loops.layers.uv.new("UVMap") if uv else None
    col_layer = bm.loops.layers.float_color.new("albedo")
    rad_layer = bm.loops.layers.float_color.new("radial")
    w_layer = bm.loops.layers.float_color.new("radial_weight")
    corner_uv = ((0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0))
    for face, color, radial in data:
        r = radial if radial is not None else Vector((0, 0, 1))
        weight = 0.0 if radial is None else 1.0
        for k, loop in enumerate(face.loops):
            loop[col_layer] = (color.x, color.y, color.z, 1.0)
            loop[rad_layer] = (r.x * 0.5 + 0.5, r.y * 0.5 + 0.5, r.z * 0.5 + 0.5, 1.0)
            loop[w_layer] = (weight, weight, weight, 1.0)
            if uv_layer is not None:
                loop[uv_layer].uv = corner_uv[k % 4]
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    obj.data.materials.append(material)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def setup_render(scene: bpy.types.Scene) -> None:
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 24
    scene.cycles.use_denoising = False
    scene.cycles.max_bounces = 0
    scene.cycles.transparent_max_bounces = 24
    scene.render.film_transparent = False
    scene.render.filter_size = 1.2
    scene.render.resolution_x = CELL * SUPERSAMPLE
    scene.render.resolution_y = CELL * SUPERSAMPLE
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGB"
    scene.render.image_settings.color_depth = "16"
    scene.view_settings.view_transform = "Raw"
    scene.view_settings.look = "None"
    scene.view_settings.exposure = 0.0
    scene.view_settings.gamma = 1.0
    world = bpy.data.worlds.new("black")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (0, 0, 0, 1)
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.0
    scene.world = world


def place_camera(camera: bpy.types.Object, azimuth: float) -> None:
    """Orthographic camera at ``azimuth``, ``ELEVATION_DEG`` above the horizon, framing the tree."""
    e = math.radians(ELEVATION_DEG)
    h = Vector((math.cos(azimuth), math.sin(azimuth), 0.0))
    up = Vector((-math.sin(e) * h.x, -math.sin(e) * h.y, math.cos(e)))
    direction = Vector((math.cos(e) * h.x, math.cos(e) * h.y, math.sin(e)))
    camera.location = up * CENTER_UP + direction * 10.0
    camera.rotation_euler = (-direction).to_track_quat("-Z", "Y").to_euler()


def render_pass(scene: bpy.types.Scene, mode: int, path: Path) -> np.ndarray:
    """Renders one pass; returns float RGB (rows top-down), box-filtered to the cell size."""
    for node in MODE_NODES:
        node.outputs[0].default_value = float(mode)
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    image = bpy.data.images.load(str(path))
    image.colorspace_settings.name = "Non-Color"
    size = CELL * SUPERSAMPLE
    px = np.array(image.pixels[:], dtype=np.float32).reshape(size, size, 4)[::-1, :, :3]
    bpy.data.images.remove(image)
    s = SUPERSAMPLE
    return px.reshape(CELL, s, CELL, s, 3).mean(axis=(1, 3))


def bleed(rgb: np.ndarray, alpha: np.ndarray, iterations: int = 24) -> np.ndarray:
    """Spreads the colours of covered pixels into transparent ones (mip-friendly edges)."""
    rgb = rgb.copy()
    known = alpha > 0.02
    for _ in range(iterations):
        if known.all():
            break
        acc = np.zeros_like(rgb)
        cnt = np.zeros(alpha.shape, dtype=np.float32)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)):
            shifted_known = np.roll(np.roll(known, dy, axis=0), dx, axis=1)
            acc += np.roll(np.roll(rgb, dy, axis=0), dx, axis=1) * shifted_known[..., None]
            cnt += shifted_known
        grow = (~known) & (cnt > 0)
        rgb[grow] = acc[grow] / cnt[grow][:, None]
        known = known | grow
    if known.any():
        rgb[~known] = rgb[known].mean(axis=0)
    return rgb


def write_png(path: Path, rgba: np.ndarray) -> None:
    """Minimal 8-bit RGBA PNG writer (rows top-down)."""
    data = np.clip(np.round(rgba * 255.0), 0, 255).astype(np.uint8)
    h, w, _ = data.shape
    raw = b"".join(b"\x00" + data[y].tobytes() for y in range(h))

    def chunk(tag: bytes, body: bytes) -> bytes:
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    path.write_bytes(png)


def main() -> None:
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out_dir = Path(argv[0]) if argv else REPO / "game/assets/textures/vegetation"
    seed = int(argv[1]) if len(argv) > 1 else 1337
    only = argv[2].split(",") if len(argv) > 2 else list(ESSENCE_ORDER)  # debug: subset
    out_dir.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    tmp = Path(bpy.app.tempdir or "/tmp") / "fc2_impostors"
    tmp.mkdir(parents=True, exist_ok=True)
    scene = bpy.context.scene
    setup_render(scene)
    camera_data = bpy.data.cameras.new("bake")
    camera_data.type = "ORTHO"
    camera_data.ortho_scale = ORTHO
    camera = bpy.data.objects.new("bake", camera_data)
    scene.collection.objects.link(camera)
    scene.camera = camera
    bark_mat = make_material("bark", None)
    leaf_mat = make_material("leaf", LEAF_TEXTURE)
    needle_mat = make_material("needle", FIR_TEXTURE)
    rows = len(ESSENCE_ORDER)
    albedo_atlas = np.zeros((rows * CELL, VIEWS * CELL, 4), dtype=np.float32)
    normal_atlas = np.zeros((rows * CELL, VIEWS * CELL, 4), dtype=np.float32)
    for row, name in enumerate(ESSENCE_ORDER):
        if name not in only:
            continue
        for obj in list(scene.collection.objects):
            if obj.type == "MESH":
                bpy.data.objects.remove(obj)
        builder = TreeBuilder(random.Random(f"{seed}-{name}-impostor"), PALETTES[name])
        BUILDERS[name](builder)
        triangles = 2 * len(builder.leaf_data) + 2 * len(builder.bark_data)
        to_object(f"{name}_bark", builder.bark, builder.bark_data, bark_mat, False)
        to_object(f"{name}_leaves", builder.leaves, builder.leaf_data, needle_mat if name == "fir" else leaf_mat, True)
        print(f"SOURCE {name} triangles={triangles}")
        for view in range(VIEWS):
            place_camera(camera, view / VIEWS * math.tau)
            albedo_pm = render_pass(scene, 0, tmp / f"{name}_{view}_albedo.png")
            normal_pm = render_pass(scene, 1, tmp / f"{name}_{view}_normal.png")
            ao_cov = render_pass(scene, 2, tmp / f"{name}_{view}_ao.png")
            coverage = np.clip(ao_cov[..., 1], 0.0, 1.0)
            safe = np.maximum(coverage, 1e-4)[..., None]
            albedo = bleed(albedo_pm / safe, coverage)
            normal = bleed(normal_pm / safe, coverage)
            ao = bleed(ao_cov[..., :1] / safe, coverage)[..., 0]
            y0, x0 = row * CELL, view * CELL
            albedo_atlas[y0 : y0 + CELL, x0 : x0 + CELL, :3] = linear_to_srgb(albedo)
            albedo_atlas[y0 : y0 + CELL, x0 : x0 + CELL, 3] = coverage
            normal_atlas[y0 : y0 + CELL, x0 : x0 + CELL, :3] = np.clip(normal, 0.0, 1.0)
            normal_atlas[y0 : y0 + CELL, x0 : x0 + CELL, 3] = np.clip(ao, 0.0, 1.0)
            print(f"VIEW {name} {view} coverage={coverage.mean():.3f}")
    for suffix, atlas in (("albedo", albedo_atlas), ("normal", normal_atlas)):
        path = out_dir / f"campaign_impostors_{suffix}.png"
        write_png(path, atlas)
        print(f"ATLAS {path} {atlas.shape[1]}x{atlas.shape[0]}")
    print(f"FRAMING ortho={ORTHO} foot={FOOT:.4f} elevation={ELEVATION_DEG} views={VIEWS}")
    print("OK")


if __name__ == "__main__":
    main()
