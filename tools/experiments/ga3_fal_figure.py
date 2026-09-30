"""GA3-L3 battle figures: A-pose reference sheet -> cut-out front view -> TRELLIS 2 on fal.ai.

Run with the fal client (``FAL_KEY`` in the environment)::

    uv run --with fal-client --with pillow --with numpy python \
        tools/experiments/ga3_fal_figure.py RAW_DIR --unit longbowman [--attempt 2]

Stages, each cached in ``RAW_DIR/<unit>[_<attempt>]/`` (an existing output is never paid twice):

1. ``sheet.png``: ``fal-ai/nano-banana-2/edit`` (2K) from the SR3 sheet
   ``~/dev/cent-ans-raw/sr3/<unit>.png``: same man, strict A-pose, empty hands, nothing held,
   three views (front, left profile, back). The tintable zones are painted in key colours
   (``KEYS``: livery garment saturated green, hose saturated blue) so that
   ``ga3_figures.py`` can segment them on the generated albedo.
2. ``sheet_cut.png``: ``fal-ai/bria/background/remove`` on the whole sheet.
3. ``front.png``: the leftmost figure of the cut sheet (alpha columns), padded square.
4. ``trellis2.glb``: ``fal-ai/trellis-2`` at 1024 on ``front.png`` (2048 texture).

Raw responses are kept as ``result_<stage>.json``; the estimated cost of every paid call is
appended to ``RAW_DIR/costs.json``.
"""

import argparse
import json
import urllib.request
from pathlib import Path

import fal_client
import numpy as np
from PIL import Image

SR3 = Path.home() / "dev/cent-ans-raw/sr3"
# Catalogue prices (USD): nano-banana-2 edit at 2K (1.5 x 0.08), bria, trellis-2 at 1024.
PRICES = {"sheet": 0.12, "cut": 0.018, "trellis2": 0.30}
COMMON = (
    "Edit this photographic character reference sheet. Keep the same man, the same face, the "
    "same headgear, belt, boots and materials, the same realistic museum-reenactor photographic "
    "style with wear and dirt, soft even studio light and a plain uniform neutral mid-grey "
    "background. Show him exactly three times side by side at full length, evenly spaced and not "
    "overlapping, all at the same scale with the feet on the same line: front view on the left, "
    "left side view (strict profile) in the middle, back view on the right. Pose in every view: "
    "strict A-pose, standing straight, feet shoulder-width apart, both arms straight and held away "
    "from the body at about 40 degrees, palms facing the thighs, hands open and empty with the "
    "fingers together, clearly separated from the body. Nothing is held in the hands and nothing "
    "crosses the body: no weapon, no bow, no shield, no buckler, no polearm in the hands. "
    "No ground shadow, no text, no labels, no frame. "
)
UNITS = {
    "longbowman": COMMON
    + "Clothing changes: the padded quilted jack keeps its cut, quilting and dirt but is made of "
    "saturated vivid pure green wool (the livery colour); the woollen hose are saturated vivid pure "
    "blue. The green jack and the blue hose are the only green and blue items. Keep the steel "
    "kettle hat, the leather belt, the brown leather boots; keep at the belt only a leather arrow "
    "bag with a sheaf of arrows hanging at the right hip and a sheathed dagger behind the left hip.",
}
TRELLIS2_ARGS = {
    "resolution": 1024,
    "texture_size": 2048,
    "decimation_target": 100000,
    "remesh": True,
}
COST_LOG: list[dict] = []


def run(endpoint: str, arguments: dict, out_dir: Path, stage: str) -> dict:
    """Call ``endpoint`` synchronously, keep the raw response and log its cost."""
    result = fal_client.subscribe(endpoint, arguments=arguments, with_logs=False)
    (out_dir / f"result_{stage}.json").write_text(json.dumps(result, indent=1))
    COST_LOG.append({"dir": out_dir.name, "stage": stage, "usd": PRICES[stage]})
    return result


def download(url: str, path: Path) -> None:
    """Fetch ``url`` into ``path``."""
    urllib.request.urlretrieve(url, path)


def front_crop(cut: Path, out: Path, size: int = 1536) -> tuple[int, int, int, int]:
    """Leftmost figure of the cut sheet (alpha column runs), centred on a square canvas."""
    rgba = np.asarray(Image.open(cut).convert("RGBA"))
    alpha = rgba[..., 3] > 32
    cols = alpha.sum(axis=0) > 2
    runs, start = [], None
    for x, on in enumerate(cols):
        if on and start is None:
            start = x
        elif not on and start is not None:
            runs.append((start, x))
            start = None
    if start is not None:
        runs.append((start, len(cols)))
    runs = [r for r in runs if r[1] - r[0] > 0.03 * len(cols)]
    x0, x1 = runs[0]
    rows = np.nonzero(alpha[:, x0:x1].any(axis=1))[0]
    y0, y1 = int(rows[0]), int(rows[-1]) + 1
    crop = Image.fromarray(rgba[y0:y1, x0:x1])
    side = int(max(crop.width, crop.height) * 1.08)
    canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    canvas.paste(crop, ((side - crop.width) // 2, (side - crop.height) // 2))
    canvas.resize((size, size), Image.LANCZOS).save(out)
    print(f"FRONT runs={runs} crop=({x0},{y0})-({x1},{y1})")
    return x0, y0, x1, y1


def main() -> None:
    """Run the cached stages for one unit."""
    parser = argparse.ArgumentParser()
    parser.add_argument("raw", type=Path)
    parser.add_argument("--unit", required=True, choices=sorted(UNITS))
    parser.add_argument("--attempt", type=int, default=1)
    parser.add_argument("--seed", type=int, default=1337)
    args = parser.parse_args()
    name = args.unit if args.attempt == 1 else f"{args.unit}_{args.attempt}"
    out = args.raw / name
    out.mkdir(parents=True, exist_ok=True)
    (out / "prompt.txt").write_text(UNITS[args.unit])
    sheet = out / "sheet.png"
    if not sheet.exists():
        src = fal_client.upload_file(str(SR3 / f"{args.unit}.png"))
        res = run(
            "fal-ai/nano-banana-2/edit",
            {
                "prompt": UNITS[args.unit],
                "image_urls": [src],
                "num_images": 1,
                "aspect_ratio": "16:9",
                "resolution": "2K",
                "output_format": "png",
                "seed": args.seed + args.attempt - 1,
            },
            out,
            "sheet",
        )
        download(res["images"][0]["url"], sheet)
    cut = out / "sheet_cut.png"
    if not cut.exists():
        url = fal_client.upload_file(str(sheet))
        res = run("fal-ai/bria/background/remove", {"image_url": url}, out, "cut")
        download(res["image"]["url"], cut)
    front = out / "front.png"
    if not front.exists():
        front_crop(cut, front)
    glb = out / "trellis2.glb"
    if not glb.exists() and not (out / "STOP").exists():
        url = fal_client.upload_file(str(front))
        res = run(
            "fal-ai/trellis-2",
            {"image_url": url, "seed": args.seed, **TRELLIS2_ARGS},
            out,
            "trellis2",
        )
        download(res["model_glb"]["url"], glb)
    log_path = args.raw / "costs.json"
    log = json.loads(log_path.read_text()) if log_path.exists() else []
    log.extend(COST_LOG)
    log_path.write_text(json.dumps(log, indent=1))
    print("COST", sum(c["usd"] for c in COST_LOG), "total", sum(c["usd"] for c in log))


if __name__ == "__main__":
    main()
