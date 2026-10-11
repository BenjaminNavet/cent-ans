"""Prepare the painted council figures (RG13-V23, option 2).

Reads the chosen cut-out of each council figure from ~/dev/cent-ans-raw/council, crops it to the
alpha bounding box (so the quad height equals the figure height), resizes to <= 1024 px high and
writes game/assets/illustrations/council/council_<role>.png.
Usage: uv run --with pillow python tools/experiments/council_painted_prep.py
"""

import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
RAW = Path.home() / "dev" / "cent-ans-raw" / "council"
OUT = ROOT / "game" / "assets" / "illustrations" / "council"
MAX_HEIGHT = 1024


def main() -> None:
    """Crop and resize every figure cut-out."""
    catalog = json.loads((ROOT / "data" / "art" / "council_catalog.json").read_text())
    OUT.mkdir(parents=True, exist_ok=True)
    for item in catalog["items"]:
        if item["kind"] != "figure":
            continue
        seed = item["result"]["image_seed_chosen"]
        source = RAW / item["id"] / "cut" / f"s{seed}.png"
        image = Image.open(source).convert("RGBA")
        box = image.getchannel("A").point(lambda value: 255 if value > 16 else 0).getbbox()
        image = image.crop(box)
        if image.height > MAX_HEIGHT:
            width = round(image.width * MAX_HEIGHT / image.height)
            image = image.resize((width, MAX_HEIGHT), Image.LANCZOS)
        image.save(OUT / f"{item['id']}.png", optimize=True)
        print(item["id"], image.size)


if __name__ == "__main__":
    main()
