"""Derived building textures (lot BR1; 2k bump and half-timber variant lot GA5).

Run: ``uv run --with pillow --with numpy python build_textures.py`` in this folder.

* ``lime_plaster_diff.jpg``: lime plaster from Poly Haven ``medieval_wall_01`` (the raw
  albedo is orange-stained; lime wash is whiter, so the colour is pulled 70 % towards its own
  luminance and lifted slightly; the relief and the normal map are kept);
* ``building_albedo_array.jpg``: the albedo layers of the campaign atlas material stacked
  vertically (512 px each, order = ``LAYERS``, same as ``kit_export.ATLAS_LAYERS`` and
  ``building_materials.gd``); solid layers are white (their colour is the shader tint);
* ``building_normal_array.jpg``: the matching OpenGL normal maps (solid layers flat).
* ``timber_frame_{diff,nor,rough}.jpg`` (lot GA5): half-timbered wall (torchis/colombage) —
  procedural beam lattice (studs, rails, diagonal braces) composited over ``lime_plaster``
  (daub infill) and ``rough_wood`` (timber), both already CC0 Poly Haven sources used elsewhere
  in this kit; no new external asset, deterministic (seeded). Not part of ``LAYERS``/the atlas
  (see ``docs/wip/ga.md`` GA5 for why: the atlas layer index is baked into exported `.glb`
  vertex colours by `tools/blender_scripts/kit_export.py`, so adding a layer there needs a
  Blender-side change and re-export, out of scope for this lot). Available as a standalone
  ``BuildingMaterials`` entry (``TimberFrame``) for future wiring.
"""

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageEnhance, ImageFilter

LAYERS = [
    "lime_plaster",
    "stone_wall",
    "rustic_stone_wall",
    "battle/castle_wall_varriation",
    "rough_wood",
    "weathered_brown_planks",
    "weathered_brown_planks",
    "clay_roof_tiles_03",
    "roof_tiles_14",
    "battle/roof_slates_02",
    "battle/thatch_roof_angled",
    None,  # Window
    None,  # Iron
    None,  # Canvas
]
SLICE = 512


def main() -> None:
    """Write ``lime_plaster_diff.jpg`` from ``medieval_wall_01_diff.jpg``."""
    src = Image.open("medieval_wall_01_diff.jpg").convert("RGB")
    grey = ImageEnhance.Color(src).enhance(0.3)
    lifted = ImageEnhance.Brightness(grey).enhance(1.04)
    soft = ImageEnhance.Contrast(lifted).enhance(0.8)
    soft.save("lime_plaster_diff.jpg", quality=88, optimize=True)
    sheet = Image.new("RGB", (SLICE, SLICE * len(LAYERS)), "white")
    for index, layer in enumerate(LAYERS):
        if layer is None:
            continue
        path = (
            Path("..") / f"{layer}_diff.jpg"
            if layer.startswith("battle/")
            else Path(f"{layer}_diff.jpg")
        )
        image = Image.open(path).convert("RGB").resize((SLICE, SLICE), Image.LANCZOS)
        sheet.paste(image, (0, index * SLICE))
    sheet.save("building_albedo_array.jpg", quality=88, optimize=True)
    normals = Image.new("RGB", (SLICE, SLICE * len(LAYERS)), (128, 128, 255))
    for index, layer in enumerate(LAYERS):
        if layer is None:
            continue
        name = "medieval_wall_01" if layer == "lime_plaster" else layer
        path = (
            Path("..") / f"{name}_nor.jpg"
            if name.startswith("battle/")
            else Path(f"{name}_nor.jpg")
        )
        image = Image.open(path).convert("RGB").resize((SLICE, SLICE), Image.LANCZOS)
        normals.paste(image, (0, index * SLICE))
    normals.save("building_normal_array.jpg", quality=90, optimize=True)


def _beam_mask(size: int) -> Image.Image:
    """Half-timber lattice (studs, rails, one diagonal brace per panel), soft edges, L mode."""
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    stud_w = size * 0.05
    rail_h = size * 0.05
    stud_x = [size * 0.10, size * 0.5, size * 0.90]
    rail_y = [size * 0.06, size * 0.5, size * 0.94]
    for x in stud_x:
        draw.rectangle([x - stud_w / 2, 0, x + stud_w / 2, size], fill=255)
    for y in rail_y:
        draw.rectangle([0, y - rail_h / 2, size, y + rail_h / 2], fill=255)
    brace_w = size * 0.045
    panels = [
        (stud_x[0], rail_y[0], stud_x[1], rail_y[1]),
        (stud_x[1], rail_y[1], stud_x[2], rail_y[2]),
    ]
    for x0, y0, x1, y1 in panels:
        for sign in (1, -1):
            cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
            length = ((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5 / 2
            angle = np.arctan2(y1 - y0, (x1 - x0) * sign)
            dx, dy = np.cos(angle) * length, np.sin(angle) * length
            perp = brace_w / 2
            px, py = -dy / length * perp, dx / length * perp
            draw.polygon(
                [
                    (cx - dx + px, cy - dy + py),
                    (cx + dx + px, cy + dy + py),
                    (cx + dx - px, cy + dy - py),
                    (cx - dx - px, cy - dy - py),
                ],
                fill=200,
            )
    return mask.filter(ImageFilter.GaussianBlur(size * 0.004))


def timber_frame(size: int = 2048) -> None:
    """Half-timbered wall (torchis/colombage, lot GA5): beam lattice over plaster infill."""
    plaster = Image.open("lime_plaster_diff.jpg").convert("RGB").resize((size, size), Image.LANCZOS)
    wood = Image.open("rough_wood_diff.jpg").convert("RGB").resize((size, size), Image.LANCZOS)
    # Timber darkened and desaturated (aged oak, not fresh planking).
    wood = ImageEnhance.Brightness(ImageEnhance.Color(wood).enhance(0.6)).enhance(0.55)
    mask = np.asarray(_beam_mask(size)).astype(np.float32) / 255.0
    diff = np.asarray(plaster).astype(np.float32) * (1 - mask[..., None]) + np.asarray(
        wood
    ).astype(np.float32) * mask[..., None]
    Image.fromarray(diff.clip(0, 255).astype(np.uint8)).save(
        "timber_frame_diff.jpg", quality=88, optimize=True
    )

    nor_size = size // 2
    plaster_nor = (
        Image.open("medieval_wall_01_nor.jpg").convert("RGB").resize((nor_size, nor_size), Image.LANCZOS)
    )
    wood_nor = Image.open("rough_wood_nor.jpg").convert("RGB").resize((nor_size, nor_size), Image.LANCZOS)
    nor_mask = np.asarray(_beam_mask(nor_size)).astype(np.float32) / 255.0
    nor = np.asarray(plaster_nor).astype(np.float32) * (1 - nor_mask[..., None]) + np.asarray(
        wood_nor
    ).astype(np.float32) * nor_mask[..., None]
    Image.fromarray(nor.clip(0, 255).astype(np.uint8)).save(
        "timber_frame_nor.jpg", quality=90, optimize=True
    )

    plaster_rough = (
        Image.open("medieval_wall_01_rough.jpg")
        .convert("L")
        .resize((nor_size, nor_size), Image.LANCZOS)
    )
    wood_rough = Image.open("rough_wood_rough.jpg").convert("L").resize((nor_size, nor_size), Image.LANCZOS)
    rough = np.asarray(plaster_rough).astype(np.float32) * (1 - nor_mask) + np.asarray(
        wood_rough
    ).astype(np.float32) * nor_mask
    Image.fromarray(rough.clip(0, 255).astype(np.uint8)).save(
        "timber_frame_rough.jpg", quality=90, optimize=True
    )


if __name__ == "__main__":
    main()
    timber_frame()
