"""GA3-S5 probe: realistic campaign-map vegetation from fal.ai (textures vs image-to-3D).

Two routes are compared on the oak, a grass tuft, a bush and a rock:

* route A (textures): ``fal-ai/flux-2`` image of an oak leaf cluster / grass tuft on white,
  ``fal-ai/bria/background/remove`` for the alpha, then the same normalisation as
  ``game/assets/textures/vegetation/build_leaf_cards.py`` (colour divided by its linear mean,
  times 0.5) so the atlas is a drop-in replacement of ``campaign_leaf_cards.png``;
* route B (3D): ``flux-2`` image of the whole object on grey, background removed,
  ``fal-ai/trellis`` (0.02 $), then decimation to the campaign budgets (~250 and ~90 triangles
  for the oak) and an albedo bake onto a small texture.

Steps (raw files in ``~/dev/cent-ans-raw/ga3/s5/``):

    uv run --with fal-client python ga3_vegetation.py fal            # images, alpha, TRELLIS
    uv run --with pillow --with numpy python ga3_vegetation.py atlas  # candidate atlases
    blender --background --python ga3_vegetation.py -- decimate       # route B LODs + bake
    blender --background --python ga3_vegetation.py -- sheet          # comparison renders

Outputs go to ``game/assets/models/vegetation/ga3/`` (candidates only, nothing is wired).
"""

import json
import math
import sys
import urllib.request
from pathlib import Path

RAW = Path.home() / "dev/cent-ans-raw/ga3/s5"
REPO = Path(__file__).resolve().parents[2]
MAIN_REPO = REPO.parent / "game_project"  # FC5 leaf cards / mid trees live on main
OUT = REPO / "game/assets/models/vegetation/ga3"

WHITE = "isolated on a plain pure white seamless studio background, no shadow, no ground, soft even diffuse lighting, photorealistic botanical scan, sharp focus, high detail"
GREY = "isolated on a plain uniform neutral mid-grey studio background, no ground, no grass, no shadow, soft diffuse overcast lighting, photorealistic, sharp focus"

# name -> (prompt, background, route)
IMAGES = {
    "oak_leaves": (
        "Top-down view of a dense rounded cluster of fresh summer pedunculate oak (Quercus robur) leaves "
        "on thin brown twigs, deeply lobed dark green leaves overlapping, the cluster fills most of the frame, "
        "a few gaps between leaves at the edge, " + WHITE,
        "white",
        "A",
    ),
    "grass_tuft": (
        "Side view of a single tuft of wild meadow grass, thin green and straw-yellow blades fanning out from one base, "
        "a few seed heads, the tuft fills the frame, " + WHITE,
        "white",
        "A",
    ),
    "oak_tree": (
        "Full view of a single mature pedunculate oak tree standing alone, short thick gnarled trunk, "
        "broad irregular domed crown of dense summer foliage with visible leaf clumps and sky gaps, "
        "whole tree visible from trunk base to crown top, eye-level three-quarter view, "
        + GREY,
        "grey",
        "B",
    ),
    "bush": (
        "A single rounded wild hawthorn bush, dense small green leaves, a few thin woody stems at the base, "
        "whole bush visible, eye-level view, " + GREY,
        "grey",
        "B",
    ),
    "rock": (
        "A single weathered grey limestone boulder with lichen and small moss patches, roughly rounded, "
        "whole rock visible, three-quarter view from slightly above, " + GREY,
        "grey",
        "B",
    ),
}
TRELLIS_ARGS = {"texture_size": 1024, "mesh_simplify": 0.95, "seed": 1337}


# ---------------------------------------------------------------- fal step (plain Python)
def download(url: str, path: Path) -> None:
    """Fetch ``url`` into ``path``."""
    path.parent.mkdir(parents=True, exist_ok=True)
    urllib.request.urlretrieve(url, path)


def fal_step(only: list[str]) -> None:
    """Generate the source images, remove their background and run TRELLIS on route B."""
    import fal_client

    RAW.mkdir(parents=True, exist_ok=True)
    log = {}
    for name, (prompt, _bg, route) in IMAGES.items():
        if only and name not in only:
            continue
        image = fal_client.subscribe(
            "fal-ai/flux-2",
            arguments={
                "prompt": prompt,
                "image_size": "square_hd",
                "num_images": 1,
                "seed": 1337,
                "output_format": "png",
            },
        )
        src_url = image["images"][0]["url"]
        download(src_url, RAW / f"{name}_src.png")
        cut = fal_client.subscribe(
            "fal-ai/bria/background/remove", arguments={"image_url": src_url}
        )
        cut_url = cut["image"]["url"]
        download(cut_url, RAW / f"{name}_cut.png")
        entry = {"prompt": prompt, "src": src_url, "cut": cut_url}
        if route == "B":
            mesh = fal_client.subscribe(
                "fal-ai/trellis", arguments={"image_url": cut_url, **TRELLIS_ARGS}
            )
            glb_url = mesh["model_mesh"]["url"]
            download(glb_url, RAW / f"{name}_trellis.glb")
            entry["glb"] = glb_url
        log[name] = entry
        print("FAL", name, "ok")
    old = (
        json.loads((RAW / "fal_log.json").read_text())
        if (RAW / "fal_log.json").exists()
        else {}
    )
    old.update(log)
    (RAW / "fal_log.json").write_text(json.dumps(old, indent=1))


# ---------------------------------------------------------------- atlas step (Pillow + numpy)
def atlas_step() -> None:
    """Build candidate atlases with the FC5 normalisation (drop-in for ``campaign_leaf_cards.png``)."""
    import numpy as np
    from PIL import Image

    sys.path.insert(0, str(MAIN_REPO / "game/assets/textures/vegetation"))
    from build_leaf_cards import bleed, linear_to_srgb, srgb_to_linear

    def normalized(path: Path, side: int, crop_to_alpha: bool = True) -> np.ndarray:
        image = Image.open(path).convert("RGBA")
        if crop_to_alpha:
            image = image.crop(
                image.getchannel("A").point(lambda a: 255 if a > 128 else 0).getbbox()
            )
            w, h = image.size
            square = Image.new("RGBA", (max(w, h),) * 2, (0, 0, 0, 0))
            square.paste(
                image, ((max(w, h) - w) // 2, max(w, h) - h)
            )  # keep tuft bases at the bottom
            image = square
        arr = (
            np.asarray(image.resize((side, side), Image.LANCZOS)).astype(np.float32)
            / 255.0
        )
        lin = srgb_to_linear(arr[..., :3])
        opaque = arr[..., 3] > 0.5
        mean = lin[opaque].mean(axis=0)
        print(f"{path.name}: mean linear {mean.round(3)}, coverage {opaque.mean():.2f}")
        return np.concatenate(
            [linear_to_srgb(bleed(lin / mean * 0.5, opaque)), arr[..., 3:4]], axis=-1
        )

    OUT.mkdir(parents=True, exist_ok=True)
    current = (
        np.asarray(
            Image.open(
                MAIN_REPO / "game/assets/textures/vegetation/campaign_leaf_cards.png"
            )
        ).astype(np.float32)
        / 255.0
    )
    leaves = normalized(RAW / "oak_leaves_cut.png", 512)
    atlas = np.concatenate([leaves, current[:, 512:]], axis=1)  # fir half unchanged
    Image.fromarray(np.round(atlas * 255).astype(np.uint8), "RGBA").save(
        OUT / "ga3_leaf_cards.png"
    )
    tuft = normalized(RAW / "grass_tuft_cut.png", 256)
    # the grass tuft of FC5 is not normalised (raw colours): keep the real colours for that one
    raw = Image.open(RAW / "grass_tuft_cut.png").convert("RGBA")
    raw = raw.crop(
        raw.getchannel("A").point(lambda a: 255 if a > 128 else 0).getbbox()
    ).resize((256, 256), Image.LANCZOS)
    arr = np.asarray(raw).astype(np.float32) / 255.0
    arr[..., :3] = bleed(arr[..., :3], arr[..., 3] > 0.5)
    Image.fromarray(np.round(arr * 255).astype(np.uint8), "RGBA").save(
        OUT / "ga3_grass_tuft.png"
    )
    del tuft
    print("OK atlas")


# ---------------------------------------------------------------- Blender steps
# route B: (source glb, output name, voxel divisions, [triangle targets], texture px)
DECIMATE = [
    ("oak_tree", "ga3_oak", 22, [250, 90], 256),
    ("bush", "ga3_bush", 16, [120, 50], 128),
]
PALETTE_OAK = (
    (0.040, 0.068, 0.026),
    (0.118, 0.155, 0.056),
    (0.20, 0.15, 0.10),
)  # VegetationMeshes
CELL_W, CELL_H = 390, 300


def decimate_step(_argv: list[str]) -> None:
    """Route B for crowns: coarse voxel remesh (closes the leaf fragments) then collapse and bake.

    Plain collapse stalls at ~3.3-4.8 k triangles on TRELLIS crowns (hundreds of leaf islands);
    the rock needs no remesh and goes through ``ga3_cleanup.py`` (``--lod0 120``).
    """
    import bpy
    import ga3_cleanup as gc

    args = gc.argparse.Namespace(
        exposure=1.0, gamma=1.0, auto_levels=0.0, normal="off", roughness="off"
    )
    OUT.mkdir(parents=True, exist_ok=True)
    for src, name, divisions, targets, tex in DECIMATE:
        bpy.ops.wm.read_factory_settings(use_empty=True)
        source = gc.import_joined(str(RAW / f"{src}_trellis.glb"))
        raw = gc.triangle_count(source)
        gc.normalise(source, 1.0)
        for level, target in enumerate(targets):
            low = source.copy()
            low.data = source.data.copy()
            low.name = f"{name}_{target}"
            bpy.context.scene.collection.objects.link(low)
            for layer in list(low.data.uv_layers):
                low.data.uv_layers.remove(layer)
            gc.select_only(low)
            remesh = low.modifiers.new("vox", "REMESH")
            remesh.mode = "VOXEL"
            remesh.voxel_size = max(low.dimensions) / divisions
            bpy.ops.object.modifier_apply(modifier=remesh.name)
            gc.collapse_to(low, target)
            gc.bake_maps(source, low, max(64, tex >> level), args)
            gc.export(low, OUT / f"{low.name}.glb")
            d = low.dimensions
            print(
                f"GA3-S5 {low.name}: {gc.triangle_count(low)} tris (raw {raw}), {d.x:.2f} x {d.y:.2f} x {d.z:.2f}"
            )
    print("OK decimate")


def _mat(
    name: str,
    colour=None,
    image=None,
    uv_scale=(1.0, 1.0),
    tint_height=False,
    alpha=False,
    emission=False,
):
    """Principled material: flat colour, image (optionally tinted x2 by the oak palette over height)."""
    import bpy

    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    bsdf = nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 0.9
    if colour is not None:
        bsdf.inputs["Base Color"].default_value = (*colour, 1.0)
    if image is not None:
        tex = nodes.new("ShaderNodeTexImage")
        tex.image = bpy.data.images.load(str(image), check_existing=True)
        mapping = nodes.new("ShaderNodeMapping")
        mapping.inputs["Scale"].default_value = (uv_scale[0], uv_scale[1], 1.0)
        coords = nodes.new("ShaderNodeTexCoord")
        links.new(coords.outputs["UV"], mapping.inputs["Vector"])
        links.new(mapping.outputs["Vector"], tex.inputs["Vector"])
        colour_out = tex.outputs["Color"]
        if tint_height:
            geo = nodes.new("ShaderNodeNewGeometry")
            sep = nodes.new("ShaderNodeSeparateXYZ")
            links.new(geo.outputs["Position"], sep.inputs["Vector"])
            ramp = nodes.new("ShaderNodeMapRange")
            ramp.inputs["From Min"].default_value = 0.3
            ramp.inputs["From Max"].default_value = 1.0
            links.new(sep.outputs["Z"], ramp.inputs["Value"])
            mix = nodes.new("ShaderNodeMix")
            mix.data_type = "RGBA"
            mix.inputs["A"].default_value = (*PALETTE_OAK[0], 1.0)
            mix.inputs["B"].default_value = (*PALETTE_OAK[1], 1.0)
            links.new(ramp.outputs["Result"], mix.inputs["Factor"])
            mul = nodes.new("ShaderNodeVectorMath")
            mul.operation = "MULTIPLY"
            links.new(tex.outputs["Color"], mul.inputs[0])
            links.new(mix.outputs["Result"], mul.inputs[1])
            scale = nodes.new("ShaderNodeVectorMath")
            scale.operation = "SCALE"
            scale.inputs["Scale"].default_value = 2.0
            links.new(mul.outputs["Vector"], scale.inputs[0])
            colour_out = scale.outputs["Vector"]
        links.new(colour_out, bsdf.inputs["Base Color"])
        if alpha:
            # alpha scissor as in foliage.gdshaderinc (card_alpha_cut 0.5)
            cut = nodes.new("ShaderNodeMath")
            cut.operation = "GREATER_THAN"
            cut.inputs[1].default_value = 0.5
            links.new(tex.outputs["Alpha"], cut.inputs[0])
            links.new(cut.outputs["Value"], bsdf.inputs["Alpha"])
    return mat


def _import(path: Path, names: list[str] | None = None) -> list:
    """Import a glb, return its mesh objects (optionally only ``names``), others deleted."""
    import bpy

    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    new = [o for o in bpy.data.objects if o not in before]
    keep = []
    for obj in new:
        if obj.type == "MESH" and (names is None or obj.name.split(".")[0] in names):
            obj.parent = None
            keep.append(obj)
    for obj in new:
        if obj not in keep:
            bpy.data.objects.remove(obj)
    return keep


def _tree_variants() -> dict:
    """Prototype trees, height 1, base at the origin.

    Current FC5 mid, route A (new atlas),
    route A' (impostor quad of the generated oak image), route B (TRELLIS 250 tri).
    """
    import bpy

    main_trees = MAIN_REPO / "game/assets/models/vegetation/campaign_trees.glb"
    bark = _mat("bark", colour=PALETTE_OAK[2])
    protos = {}
    for key, atlas in (
        (
            "current",
            MAIN_REPO / "game/assets/textures/vegetation/campaign_leaf_cards.png",
        ),
        ("A", OUT / "ga3_leaf_cards.png"),
    ):
        crown, trunk = sorted(
            _import(main_trees, ["oak_mid_crown", "oak_mid_trunk"]),
            key=lambda o: o.name,
        )
        crown.data.materials.clear()
        crown.data.materials.append(
            _mat(
                f"cards_{key}",
                image=atlas,
                uv_scale=(0.5, 1.0),
                tint_height=True,
                alpha=True,
            )
        )
        trunk.data.materials.clear()
        trunk.data.materials.append(bark)
        protos[key] = [crown, trunk]
    bpy.ops.mesh.primitive_plane_add(size=1.0)
    quad = bpy.context.active_object
    quad.rotation_euler = (math.pi / 2, 0, 0)
    bpy.ops.object.transform_apply(rotation=True)
    quad.location = (0, 0, 0.5)
    bpy.ops.object.transform_apply(location=True)
    quad.data.materials.append(
        _mat("impostor", image=RAW / "oak_tree_cut.png", alpha=True)
    )
    protos["A_impostor"] = [quad]
    (oak_b,) = _import(OUT / "ga3_oak_250.glb")
    oak_b.scale = (1 / oak_b.dimensions.z,) * 3
    protos["B"] = [oak_b]
    for objs in protos.values():
        for obj in objs:
            obj.hide_render = True
            obj.location.x += 1000
    return protos


def _place(objs: list, loc, rot: float, scale: float, face_camera=None) -> None:
    """Linked copies of a prototype at ``loc``."""
    import bpy

    for obj in objs:
        inst = obj.copy()
        bpy.context.scene.collection.objects.link(inst)
        inst.hide_render = False
        inst.location = (loc[0], loc[1], obj.location.z)
        inst.rotation_euler.z = face_camera if face_camera is not None else rot
        inst.scale = tuple(scale * s for s in obj.scale)


def _render(path: Path, cam_loc, target, lens: float, build) -> None:
    """Render one sheet cell: ground, sun, sky, then ``build()`` adds the subjects."""
    import bpy

    scene = bpy.context.scene
    for obj in list(scene.collection.objects):
        if not obj.hide_render and obj.name not in ("",):
            bpy.data.objects.remove(obj)
    bpy.ops.mesh.primitive_plane_add(size=200.0)
    bpy.context.active_object.data.materials.append(
        _mat("ground", colour=(0.10, 0.11, 0.05))
    )
    sun = bpy.data.lights.new("sun", "SUN")
    sun.energy = 3.5
    sun.angle = math.radians(3)
    sun_obj = bpy.data.objects.new("sun", sun)
    sun_obj.rotation_euler = (math.radians(50), 0, math.radians(35))
    scene.collection.objects.link(sun_obj)
    cam = bpy.data.cameras.new("cam")
    cam.lens = lens
    cam_obj = bpy.data.objects.new("cam", cam)
    scene.collection.objects.link(cam_obj)
    cam_obj.location = cam_loc
    direction = (target[0] - cam_loc[0], target[1] - cam_loc[1], target[2] - cam_loc[2])
    from mathutils import Vector

    cam_obj.rotation_euler = Vector(direction).to_track_quat("-Z", "Y").to_euler()
    scene.camera = cam_obj
    build(math.atan2(-direction[0], direction[1]))
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)


def sheet_step(_argv: list[str]) -> None:
    """Comparison cells for ``docs/img/ga3/s5_vegetation.jpg``.

    Current / A / A' / B, near tree + grove, plus a row of ground clutter (grass current vs A, bush B, rock B).
    """
    import random

    import bpy

    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 24
    scene.cycles.use_denoising = False
    scene.render.resolution_x, scene.render.resolution_y = CELL_W, CELL_H
    scene.render.image_settings.file_format = "PNG"
    world = bpy.data.worlds.new("sky")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (
        0.55,
        0.65,
        0.8,
        1.0,
    )
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.8
    scene.world = world
    protos = _tree_variants()
    cells = RAW / "cells"
    cells.mkdir(parents=True, exist_ok=True)
    order = ["current", "A", "A_impostor", "B"]
    rng = random.Random(7)
    grove = [
        (
            rng.uniform(-2.2, 2.2),
            rng.uniform(-2.2, 2.2),
            rng.uniform(0, 6.28),
            rng.uniform(0.8, 1.15),
        )
        for _ in range(16)
    ]
    for key in order:

        def near(yaw, key=key):
            _place(
                protos[key],
                (0, 0),
                0.6,
                1.0,
                face_camera=yaw if key == "A_impostor" else None,
            )

        _render(cells / f"near_{key}.png", (0.0, -2.6, 1.5), (0, 0, 0.5), 50, near)

        def grove_build(yaw, key=key):
            for x, y, r, s in grove:
                _place(
                    protos[key],
                    (x, y),
                    r,
                    s,
                    face_camera=yaw if key == "A_impostor" else None,
                )

        _render(
            cells / f"grove_{key}.png", (0.0, -9.0, 6.0), (0, 0, 0.3), 45, grove_build
        )
    # ground clutter row: grass tufts as crossed quads (current vs A), bush and rock from route B
    bpy.ops.mesh.primitive_plane_add(size=1.0)
    tuft = bpy.context.active_object
    tuft.rotation_euler = (math.pi / 2, 0, 0)
    bpy.ops.object.transform_apply(rotation=True)
    tuft.location = (0, 0, 0.5)
    bpy.ops.object.transform_apply(location=True)
    tuft.hide_render = True
    tufts = {}
    for key, image in (
        (
            "grass_current",
            MAIN_REPO / "game/assets/textures/vegetation/campaign_grass_tuft.png",
        ),
        ("grass_A", OUT / "ga3_grass_tuft.png"),
    ):
        proto = tuft.copy()
        proto.data = tuft.data.copy()
        proto.data.materials.append(_mat(key, image=image, alpha=True))
        bpy.context.scene.collection.objects.link(proto)
        tufts[key] = [proto]
    (bush,) = _import(OUT / "ga3_bush_120.glb")
    (rock,) = _import(OUT / "ga3_rock_lod0.glb")
    for obj in (bush, rock):
        obj.hide_render = True
    tufts["bush_B"] = [bush]
    tufts["rock_B"] = [rock]
    for key, objs in tufts.items():

        def clutter(yaw, objs=objs, key=key):
            spots = [(rng.uniform(-1.3, 1.3), rng.uniform(-1.0, 1.4)) for _ in range(9)]
            for x, y in spots:
                s = (
                    rng.uniform(0.35, 0.6)
                    if key.startswith("grass")
                    else rng.uniform(0.4, 0.8)
                )
                _place(objs, (x, y), rng.uniform(0, 6.28), s)
                if key.startswith("grass"):
                    _place(objs, (x, y), rng.uniform(0, 6.28) + 1.57, s)

        _render(
            cells / f"clutter_{key}.png", (0.0, -3.4, 1.8), (0, 0.2, 0.2), 45, clutter
        )
    print("OK sheet cells")


def blender_main(argv: list[str]) -> None:
    """Dispatch the Blender sub-steps (``bpy`` only exists inside Blender)."""
    {"decimate": decimate_step, "sheet": sheet_step}[argv[0]](argv[1:])


if __name__ == "__main__":
    if "--" in sys.argv:
        sys.path.insert(0, str(Path(__file__).parent))
        blender_main(sys.argv[sys.argv.index("--") + 1 :])
    elif len(sys.argv) > 1 and sys.argv[1] == "fal":
        fal_step(sys.argv[2:])
    elif len(sys.argv) > 1 and sys.argv[1] == "atlas":
        atlas_step()
    else:
        print(__doc__)
