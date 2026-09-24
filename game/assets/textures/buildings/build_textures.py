"""Derived building textures (lot BR1).

Run: ``uv run --with pillow python build_textures.py`` in this folder.

* ``lime_plaster_diff.jpg``: lime plaster from Poly Haven ``medieval_wall_01`` (the raw
  albedo is orange-stained; lime wash is whiter, so the colour is pulled 70 % towards its own
  luminance and lifted slightly; the relief and the normal map are kept);
* ``building_albedo_array.jpg``: the albedo layers of the campaign atlas material stacked
  vertically (512 px each, order = ``LAYERS``, same as ``kit_campaign.LAYERS`` and
  ``building_materials.gd``); solid layers are white (their colour is the shader tint).
"""

from pathlib import Path

from PIL import Image, ImageEnhance

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
        path = Path("..") / f"{layer}_diff.jpg" if layer.startswith("battle/") else Path(f"{layer}_diff.jpg")
        image = Image.open(path).convert("RGB").resize((SLICE, SLICE), Image.LANCZOS)
        sheet.paste(image, (0, index * SLICE))
    sheet.save("building_albedo_array.jpg", quality=88, optimize=True)


if __name__ == "__main__":
    main()
