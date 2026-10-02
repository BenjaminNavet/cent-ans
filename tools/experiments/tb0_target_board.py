"""TB0: target board, our campaign captures repainted towards the Thrones of Britannia look.

Each tier (wide, medium, close, plus a winter medium) sends one of our own captures to
``fal-ai/nano-banana-2/edit`` with two Thrones of Britannia references that only carry the
rendering style. The repaint keeps our geography and camera, so it can serve as the success
criterion of the later TB lots.

Run with the fal client (``FAL_KEY`` in the environment)::

    uv run --project tools --with fal-client python tools/experiments/tb0_target_board.py \
        CACHE_DIR [--only wide,close] [--dry-run]

``CACHE_DIR`` holds ``ours/`` (captures of ``game/tests/ss_shot.gd``) and ``refs/`` (third-party
references, never versioned). Raw PNGs stay in ``CACHE_DIR/repaint`` (reused on a rerun); the
board images go to ``docs/img/tb/``.
"""

from __future__ import annotations

import argparse
import subprocess
import urllib.request
from dataclasses import dataclass
from pathlib import Path

ENDPOINT = "fal-ai/nano-banana-2/edit"
PRICE_USD = 0.08  # 1K, catalogue price
REPO_DIR = Path(__file__).resolve().parents[2]
BOARD_DIR = REPO_DIR / "docs" / "img" / "tb"
BOARD_WIDTH = 1280

COMMON_PROMPT = (
    "The FIRST image is a screenshot of our own strategy game campaign map: 14th-century "
    "France and its neighbours. Repaint it as the target look of that same map. Keep exactly "
    "the same camera, coastline, rivers, relief, town positions, army figures and heraldic "
    "banners; do not move or invent geography. The OTHER images are only style references "
    "(Total War Saga: Thrones of Britannia campaign map): take their rendering, never their "
    "geography, interface panels, minimap, logos or watermarks. Target rendering: restrained "
    "semi-realistic painted realism, natural golden light, muted earth colours, soft cloud "
    "shadows on the ground, deep grey-blue sea with pale shallows and a thin foam line along "
    "the coasts, forests as dense dark masses, a visible patchwork of fields and pastures, "
    "towns as small clusters of real buildings with walls for the large ones. Keep the image "
    "calm and readable: thin subdued faction borders instead of thick bright ribbons, no "
    "milky fog over the land, few small icons, no interface."
)


@dataclass(frozen=True)
class Tier:
    """One image of the board: our capture, its style references and the tier wording."""

    name: str
    capture: str
    references: tuple[str, ...]
    note: str


TIERS = (
    Tier(
        "wide",
        "avant-sol-1100.png",
        ("steam-05.jpg", "steam-07.jpg"),
        "Wide view of the whole theatre in summer: the land reads as green and gold "
        "regions under a clear sky, lands outside our sight are slightly darker and "
        "desaturated, not white.",
    ),
    Tier(
        "medium",
        "avant-sol-400.png",
        ("steam-07.jpg", "yt-a-02.jpg"),
        "Medium view in summer: ripening wheat, green pastures, dark forests, each town "
        "a recognisable cluster of roofs.",
    ),
    Tier(
        "medium-winter",
        "avant-sol-400.png",
        ("yt-a-13.jpg", "steam-07.jpg"),
        "Same medium view in deep winter: snow covers fields, roofs and forest tops, bare "
        "brown woods, rivers dark and cold, the sea steel grey, low pale light.",
    ),
    Tier(
        "close",
        "avant-sol-35.png",
        ("yt-a-15.jpg", "yt-a-10.jpg"),
        "Close view in summer: the town shows individual timber and stone houses, a "
        "church, walls, and outside the walls farms, a mill and worked fields; grass and "
        "crops have fine texture.",
    ),
)


def main() -> None:
    """Generate the missing repaints and write the board images."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cache_dir", type=Path)
    parser.add_argument("--only", default="")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    tiers = [
        tier for tier in TIERS if not args.only or tier.name in args.only.split(",")
    ]
    raw_dir = args.cache_dir / "repaint"
    missing = [tier for tier in tiers if not (raw_dir / f"{tier.name}.png").exists()]
    print(f"{len(missing)} repaint(s) to generate (~{len(missing) * PRICE_USD:.2f} $)")
    if args.dry_run:
        return
    raw_dir.mkdir(parents=True, exist_ok=True)
    BOARD_DIR.mkdir(parents=True, exist_ok=True)
    uploaded: dict[Path, str] = {}
    for tier in tiers:
        raw = raw_dir / f"{tier.name}.png"
        if not raw.exists():
            import fal_client

            sources = [args.cache_dir / "ours" / tier.capture]
            sources += [args.cache_dir / "refs" / name for name in tier.references]
            for source in sources:
                if source not in uploaded:
                    uploaded[source] = fal_client.upload_file(str(source))
            result = fal_client.subscribe(
                ENDPOINT,
                arguments={
                    "prompt": f"{COMMON_PROMPT}\n{tier.note}",
                    "image_urls": [uploaded[source] for source in sources],
                    "num_images": 1,
                    "aspect_ratio": "16:9",
                    "resolution": "1K",
                    "output_format": "png",
                    "seed": 1337,
                },
                with_logs=False,
            )
            urllib.request.urlretrieve(result["images"][0]["url"], raw)
            print(f"{tier.name}: generated")
        board_image = BOARD_DIR / f"cible-{tier.name}.jpg"
        subprocess.run(
            ["ffmpeg", "-v", "error", "-y", "-i", str(raw)]
            + ["-vf", f"scale={BOARD_WIDTH}:-2", "-q:v", "3", str(board_image)],
            check=True,
        )
        print(f"{tier.name}: {board_image.relative_to(REPO_DIR)}")


if __name__ == "__main__":
    main()
