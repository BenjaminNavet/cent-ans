"""Derived building textures (lot BR1): lime plaster from Poly Haven ``medieval_wall_01``.

Run: ``uv run --with pillow python build_textures.py`` in this folder. The raw Poly Haven
albedo is orange-stained; lime wash is whiter, so the colour is pulled 70 % towards its own
luminance and lifted slightly (the relief and the normal map are kept).
"""

from PIL import Image, ImageEnhance


def main() -> None:
    """Write ``lime_plaster_diff.jpg`` from ``medieval_wall_01_diff.jpg``."""
    src = Image.open("medieval_wall_01_diff.jpg").convert("RGB")
    grey = ImageEnhance.Color(src).enhance(0.3)
    lifted = ImageEnhance.Brightness(grey).enhance(1.04)
    soft = ImageEnhance.Contrast(lifted).enhance(0.8)
    soft.save("lime_plaster_diff.jpg", quality=88, optimize=True)


if __name__ == "__main__":
    main()
