"""RG14: local resource icons (Z-Image Turbo via mflux, rembg cut-out, 128/64 px PNG).

Free and local only (ADR 0190). Three seeds per resource under ``tools/gpu_lock.sh``::

    tools/gpu_lock.sh uv run --project tools --with rembg --with onnxruntime python tools/experiments/resource_icons.py generate
    uv run --project tools --with rembg --with onnxruntime python tools/experiments/resource_icons.py cut
    uv run --project tools python tools/experiments/resource_icons.py contact
    uv run --project tools python tools/experiments/resource_icons.py install

``choices.json`` in the work directory maps an id to the chosen seed index (0-2).
``install`` writes ``game/assets/icons/entity/<id>.png`` (128 px) and
``game/assets/icons/resources/<id>_64.png`` and registers the ids in ``entity/index.json``,
which is where ``IconLibrary`` resolves them (entity > ink > SVG).
"""

from __future__ import annotations

import argparse
import io
import json
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
WORK = Path.home() / "dev" / "cent-ans-raw" / "icons-rg14"
MODEL = Path.home() / "models" / "mflux" / "z-image-turbo-q8"
ENTITY_DIR = ROOT / "game" / "assets" / "icons" / "entity"
SMALL_DIR = ROOT / "game" / "assets" / "icons" / "resources"
SEEDS = (1401, 1402, 1403)
SIZE = 768
STEPS = 8

STYLE_PREFIX = "Medieval illuminated manuscript miniature painting of a single object, "
STYLE_SUFFIX = (
    ", centered, bold readable silhouette, rich saturated gouache colours with fine dark ink "
    "outline, gold leaf highlights, isolated on a plain flat pale cream background, "
    "no text, no letters, no frame, no border, no shadow on the ground"
)

# id -> subject
SUBJECTS = {
    "res_wheat": "a golden wheat sheaf tied with a red cord",
    "res_wine": "a small oak wine barrel with a purple grape bunch on top",
    "res_wool": "a bundle of white raw fleece tied with string",
    "res_cloth": "a bolt of rolled red and blue woven cloth with a folded end",
    "res_fish": "two silver herrings crossed",
    "res_iron": "a stack of dark grey iron bars",
    "res_tin": "a pile of pale silver tin ingots",
    "res_salt": "a heap of white salt crystals in a small open wooden bowl",
    "res_stone": "a stack of cut pale limestone blocks",
    "res_wood": "a bundle of cut timber logs bound with rope",
    "hud_treasury": "a pile of gold coins with a fleur-de-lis stamped on the top coin and a gold ingot",
}


def prompt_for(resource_id: str) -> str:
    """Full prompt of one resource."""
    return STYLE_PREFIX + SUBJECTS[resource_id] + STYLE_SUFFIX


def raw_path(resource_id: str, index: int) -> Path:
    """Raw render of seed ``index``."""
    return WORK / "raw" / f"{resource_id}_{index}.png"


def cut_path(resource_id: str, index: int) -> Path:
    """Transparent cut-out of seed ``index``."""
    return WORK / "cut" / f"{resource_id}_{index}.png"


def generate() -> None:
    """Three mflux renders per resource (cached)."""
    (WORK / "raw").mkdir(parents=True, exist_ok=True)
    for resource_id in SUBJECTS:
        for index, seed in enumerate(SEEDS):
            output = raw_path(resource_id, index)
            if output.exists():
                continue
            subprocess.run(
                [
                    "mflux-generate-z-image-turbo", "--model", str(MODEL),
                    "--base-model", "z-image-turbo", "--prompt", prompt_for(resource_id),
                    "--steps", str(STEPS), "--seed", str(seed),
                    "--width", str(SIZE), "--height", str(SIZE), "--output", str(output),
                ],
                check=True,
                capture_output=True,
            )  # fmt: skip
            print("rendered", output.name, flush=True)


def square_crop(image: Image.Image, fill: float = 0.92) -> Image.Image:
    """Tight crop of the opaque part, centred in a square with ``fill`` of the side."""
    box = image.getchannel("A").point(lambda v: 255 if v > 24 else 0).getbbox()
    if box is None:
        return image
    cropped = image.crop(box)
    side = int(max(cropped.size) / fill)
    canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    canvas.paste(cropped, ((side - cropped.width) // 2, (side - cropped.height) // 2))
    return canvas


def cut() -> None:
    """Rembg cut-out of every render (cached)."""
    from rembg import new_session, remove

    session = new_session("isnet-general-use")
    (WORK / "cut").mkdir(parents=True, exist_ok=True)
    for resource_id in SUBJECTS:
        for index in range(len(SEEDS)):
            output = cut_path(resource_id, index)
            if output.exists() or not raw_path(resource_id, index).exists():
                continue
            source = Image.open(raw_path(resource_id, index)).convert("RGB")
            result = remove(source, session=session)
            if not isinstance(result, Image.Image):
                result = Image.open(io.BytesIO(result))
            square_crop(result.convert("RGBA")).save(output)
            print("cut", output.name, flush=True)


def load_choices() -> dict[str, int]:
    """Chosen seed index per id."""
    path = WORK / "choices.json"
    return json.loads(path.read_text()) if path.exists() else {}


def contact() -> None:
    """Contact sheet: one row per resource, 3 draws + the chosen one at 128 and 32 px."""
    choices = load_choices()
    cell = 192
    sheet = Image.new("RGB", (cell * 5 + 160, cell * len(SUBJECTS)), (225, 210, 180))
    draw = ImageDraw.Draw(sheet)
    for row, resource_id in enumerate(SUBJECTS):
        draw.text((6, row * cell + 6), resource_id, fill=(40, 25, 10))
        for index in range(len(SEEDS)):
            path = cut_path(resource_id, index)
            if not path.exists():
                continue
            icon = Image.open(path).resize((cell - 8, cell - 8), Image.LANCZOS)
            x = 160 + index * cell
            sheet.paste(icon, (x + 4, row * cell + 4), icon)
            mark = "CHOIX" if choices.get(resource_id) == index else str(index)
            draw.text((x + 6, row * cell + 6), mark, fill=(150, 0, 0))
        if resource_id in choices:
            icon = Image.open(cut_path(resource_id, choices[resource_id]))
            for slot, size in ((3, 128), (4, 32)):
                small = icon.resize((size, size), Image.LANCZOS)
                x = 160 + slot * cell + (cell - size) // 2
                sheet.paste(small, (x, row * cell + (cell - size) // 2), small)
    sheet.save(WORK / "contact.jpg", quality=88)
    print("contact ->", WORK / "contact.jpg")


def install() -> None:
    """Write the chosen icons in the game and register them in the entity index."""
    choices = load_choices()
    ENTITY_DIR.mkdir(parents=True, exist_ok=True)
    SMALL_DIR.mkdir(parents=True, exist_ok=True)
    index_path = ENTITY_DIR / "index.json"
    index = json.loads(index_path.read_text())
    for resource_id, seed_index in choices.items():
        icon = Image.open(cut_path(resource_id, seed_index)).convert("RGBA")
        icon.resize((128, 128), Image.LANCZOS).save(ENTITY_DIR / f"{resource_id}.png")
        icon.resize((64, 64), Image.LANCZOS).save(SMALL_DIR / f"{resource_id}_64.png")
        index["icons"][resource_id] = f"{resource_id}.png"
    index["icons"] = dict(sorted(index["icons"].items()))
    index_path.write_text(json.dumps(index, indent=1, ensure_ascii=False) + "\n")
    print("installed", len(choices))


def main() -> None:
    """Command line entry."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("step", choices=("generate", "cut", "contact", "install"))
    args = parser.parse_args()
    {"generate": generate, "cut": cut, "contact": contact, "install": install}[
        args.step
    ]()


if __name__ == "__main__":
    main()
