"""VN: missing unit-type miniatures through fal.ai (OpenRouter unavailable).

Same prompt as the encyclopedia miniatures (:func:`cent_ans_tools.entry_art.build_prompt`),
rendered by ``fal-ai/nano-banana-2/edit`` with an existing unit miniature as style reference,
cropped to 640x360 like the others (:func:`cent_ans_tools.entry_art.convert`).

Run with the fal client (``FAL_KEY`` in the environment)::

    uv run --project tools --with fal-client python tools/experiments/vn_unit_art_fal.py RAW_DIR \
        [--only unit_akinci,unit_yaya] [--dry-run]

Raw PNGs stay in ``RAW_DIR`` (reused on a rerun); only missing illustrations are generated.
"""

from __future__ import annotations

import argparse
import json
import urllib.request
from pathlib import Path

from cent_ans_tools import entry_art

ENDPOINT = "fal-ai/nano-banana-2/edit"
PRICE_USD = 0.08  # 1K, catalogue price
# Style references: a mounted and a foot miniature already in the game.
REFERENCE_MOUNTED = entry_art.ILLUSTRATIONS_DIR / "unit_knights.jpg"
REFERENCE_FOOT = entry_art.ILLUSTRATIONS_DIR / "unit_longbowmen.jpg"
REFERENCE_BUILDING = entry_art.ILLUSTRATIONS_DIR / "bld_abbey.jpg"
STYLE_NOTE = (
    "The reference image only shows the painting style to match (Gothic manuscript "
    "miniature technique, palette, gilded diapered background, ink outlines); paint a new "
    "scene with its own subject, do not copy its figures or buildings."
)


def missing_units(category: str = "unit_types") -> list[str]:
    """Entry ids of ``category`` without an illustration."""
    return sorted(
        path.stem
        for path in (entry_art.REPO_DIR / "data" / category).glob("*.json")
        if not (entry_art.ILLUSTRATIONS_DIR / f"{path.stem}.jpg").exists()
    )


def is_mounted(entry: dict) -> bool:
    """Cavalry types get the mounted reference."""
    text = json.dumps(entry).lower()
    return any(
        word in text for word in ("cavalry", "mounted", "horse", "cavalerie", "cheval")
    )


def main() -> None:
    """Generate the missing unit miniatures."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_dir", type=Path)
    parser.add_argument("--only", default="")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--category", default="unit_types", choices=["unit_types", "buildings"]
    )
    args = parser.parse_args()
    ids = missing_units(args.category)
    data_dir = entry_art.REPO_DIR / "data" / args.category
    if args.only:
        ids = [i for i in ids if i in args.only.split(",")]
    print(f"{len(ids)} unit(s): {', '.join(ids)} (~{len(ids) * PRICE_USD:.2f} $)")
    if args.dry_run:
        return
    import fal_client

    args.raw_dir.mkdir(parents=True, exist_ok=True)
    uploaded: dict[Path, str] = {}
    spent = 0.0
    for unit_id in ids:
        raw = args.raw_dir / f"{unit_id}.png"
        entry = json.loads((data_dir / f"{unit_id}.json").read_text())
        if not raw.exists():
            if args.category == "buildings":
                reference = REFERENCE_BUILDING
            else:
                reference = REFERENCE_MOUNTED if is_mounted(entry) else REFERENCE_FOOT
            if reference not in uploaded:
                uploaded[reference] = fal_client.upload_file(str(reference))
            prompt = entry_art.build_prompt(args.category, entry) + "\n" + STYLE_NOTE
            result = fal_client.subscribe(
                ENDPOINT,
                arguments={
                    "prompt": prompt,
                    "image_urls": [uploaded[reference]],
                    "num_images": 1,
                    "aspect_ratio": "16:9",
                    "resolution": "1K",
                    "output_format": "png",
                    "seed": 1337,
                },
                with_logs=False,
            )
            urllib.request.urlretrieve(result["images"][0]["url"], raw)
            spent += PRICE_USD
            print(f"{unit_id}: generated ({spent:.2f} $ so far)")
        out = entry_art.ILLUSTRATIONS_DIR / f"{unit_id}.jpg"
        out.write_bytes(entry_art.convert(raw.read_bytes()))
        print(f"{unit_id}: {out.relative_to(entry_art.REPO_DIR)}")
    print(f"spent ~{spent:.2f} $")


if __name__ == "__main__":
    main()
