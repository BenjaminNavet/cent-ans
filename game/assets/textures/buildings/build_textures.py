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
  in this kit; no new external asset, deterministic (seeded). Lot TF: layer 15 of ``LAYERS``
  (``TimberFrameFar``), on framed walls of the ``low`` detail kit that has no modelled studs.
* ``timber_daub_diff.jpg`` (lot TF): daub infill (torchis) of half-timbered panels, layer 14
  (``TimberFrame``), under the modelled beams of the ``high`` detail kit; normals and roughness
  are those of ``medieval_wall_01``. Layers appended after the solid ones so the indices baked
  into older `.glb` files stay valid.
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
    "timber_daub",  # TimberFrame (lot TF: daub infill under the modelled beams)
    "timber_frame",  # TimberFrameFar (lot TF: painted lattice, low detail without beams)
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
        image = (
            Image.open(path)
            .convert("RGB")
            .resize((SLICE, SLICE), Image.Resampling.LANCZOS)
        )
        sheet.paste(image, (0, index * SLICE))
    sheet.save("building_albedo_array.jpg", quality=88, optimize=True)
    normals = Image.new("RGB", (SLICE, SLICE * len(LAYERS)), (128, 128, 255))
    for index, layer in enumerate(LAYERS):
        if layer is None:
            continue
        name = "medieval_wall_01" if layer in ("lime_plaster", "timber_daub") else layer
        path = (
            Path("..") / f"{name}_nor.jpg"
            if name.startswith("battle/")
            else Path(f"{name}_nor.jpg")
        )
        image = (
            Image.open(path)
            .convert("RGB")
            .resize((SLICE, SLICE), Image.Resampling.LANCZOS)
        )
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


def timber_daub(size: int = 2048) -> None:
    """Daub infill of half-timbered panels (torchis, lot TF): no painted beams.

    Lime plaster warmed towards ochre (clay under a worn lime wash), low-frequency earthy
    blotches and short straw flecks; relief and normals stay those of ``medieval_wall_01``.
    Deterministic (seeded), derived from CC0 sources only.
    """
    rng = np.random.default_rng(1337)
    base = np.asarray(
        Image.open("lime_plaster_diff.jpg")
        .convert("RGB")
        .resize((size, size), Image.Resampling.LANCZOS)
    ).astype(np.float32)
    warm = base * np.array([1.02, 0.99, 0.92], dtype=np.float32)
    # Earthy blotches where the lime wash wore off (tileable: blurred wrapped noise).
    noise = rng.random((size // 64, size // 64)).astype(np.float32)
    blot = Image.fromarray((noise * 255).astype(np.uint8)).resize(
        (size, size), Image.Resampling.BICUBIC
    )
    blot_arr = np.asarray(blot.filter(ImageFilter.GaussianBlur(size * 0.01))).astype(
        np.float32
    )
    earth = np.clip((blot_arr / 255.0 - 0.55) * 2.2, 0.0, 1.0)[..., None] * 0.18
    ochre = np.array([168.0, 140.0, 100.0], dtype=np.float32)
    daub = warm * (1 - earth) + ochre * earth
    image = Image.fromarray(daub.clip(0, 255).astype(np.uint8))
    draw = ImageDraw.Draw(image)
    for _ in range(size // 2):
        x, y = rng.random(2) * size
        angle = rng.random() * np.pi
        length = size * (0.004 + rng.random() * 0.01)
        dx, dy = np.cos(angle) * length, np.sin(angle) * length
        shade = int(165 + rng.random() * 40)
        draw.line(
            [(x, y), (x + dx, y + dy)],
            fill=(shade + 12, shade + 4, int(shade * 0.78)),
            width=max(1, size // 1024),
        )
    image.save("timber_daub_diff.jpg", quality=88, optimize=True)


def timber_frame(size: int = 2048) -> None:
    """Half-timbered wall (torchis/colombage, lot GA5): beam lattice over plaster infill."""
    plaster = (
        Image.open("lime_plaster_diff.jpg")
        .convert("RGB")
        .resize((size, size), Image.Resampling.LANCZOS)
    )
    wood = (
        Image.open("rough_wood_diff.jpg")
        .convert("RGB")
        .resize((size, size), Image.Resampling.LANCZOS)
    )
    # Timber darkened and desaturated (aged oak, not fresh planking).
    wood = ImageEnhance.Brightness(ImageEnhance.Color(wood).enhance(0.6)).enhance(0.55)
    mask = np.asarray(_beam_mask(size)).astype(np.float32) / 255.0
    diff = (
        np.asarray(plaster).astype(np.float32) * (1 - mask[..., None])
        + np.asarray(wood).astype(np.float32) * mask[..., None]
    )
    Image.fromarray(diff.clip(0, 255).astype(np.uint8)).save(
        "timber_frame_diff.jpg", quality=88, optimize=True
    )

    nor_size = size // 2
    plaster_nor = (
        Image.open("medieval_wall_01_nor.jpg")
        .convert("RGB")
        .resize((nor_size, nor_size), Image.Resampling.LANCZOS)
    )
    wood_nor = (
        Image.open("rough_wood_nor.jpg")
        .convert("RGB")
        .resize((nor_size, nor_size), Image.Resampling.LANCZOS)
    )
    nor_mask = np.asarray(_beam_mask(nor_size)).astype(np.float32) / 255.0
    nor = (
        np.asarray(plaster_nor).astype(np.float32) * (1 - nor_mask[..., None])
        + np.asarray(wood_nor).astype(np.float32) * nor_mask[..., None]
    )
    Image.fromarray(nor.clip(0, 255).astype(np.uint8)).save(
        "timber_frame_nor.jpg", quality=90, optimize=True
    )

    plaster_rough = (
        Image.open("medieval_wall_01_rough.jpg")
        .convert("L")
        .resize((nor_size, nor_size), Image.Resampling.LANCZOS)
    )
    wood_rough = (
        Image.open("rough_wood_rough.jpg")
        .convert("L")
        .resize((nor_size, nor_size), Image.Resampling.LANCZOS)
    )
    rough = (
        np.asarray(plaster_rough).astype(np.float32) * (1 - nor_mask)
        + np.asarray(wood_rough).astype(np.float32) * nor_mask
    )
    Image.fromarray(rough.clip(0, 255).astype(np.uint8)).save(
        "timber_frame_rough.jpg", quality=90, optimize=True
    )


if __name__ == "__main__":
    timber_daub()
    main()
    timber_frame()
