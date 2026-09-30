"""GA3-L3 battle figures: A-pose reference sheet -> cut-out views -> TRELLIS on fal.ai.

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
4. ``multi.glb`` (default, ``--model multi``): ``fal-ai/trellis/multi`` (0.02 $) on the
   A-pose views (``--views front,back``: the generated profile often holds something);
   ``trellis.glb`` (``--model trellis``, 0.02 $) on ``front.png``; ``trellis2.glb``
   (``--model trellis2``, 0.30 $, lot L3a comparison only: the player keeps to the cheap
   models) at 1024 on ``front.png`` (2048 texture).

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
PRICES = {"sheet": 0.12, "cut": 0.018, "trellis2": 0.30, "trellis": 0.02, "multi": 0.02}
VIEW_NAMES = ("front", "side", "back")  # left to right on the sheet
TRELLIS_ARGS = {"texture_size": 2048, "mesh_simplify": 0.9}
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
    # L3b: infantry_0 (men-at-arms on foot). No hose key: plate legs; the steel stays steel.
    "man_at_arms": COMMON
    + "Clothing changes: the short tight padded jupon (coat armour) worn over the coat of plates "
    "keeps its cut, padding and dirt but is plain saturated vivid pure green wool without any "
    "heraldic charge, stripe or badge (the livery colour); it is the only green item. All the "
    "armour stays plain grey steel, not painted: pointed bascinet with the visor raised and the "
    "mail aventail, mail sleeves, plate vambraces, couters, gauntlets, cuisses, poleyns, greaves "
    "and sabatons. There is no blue item. Keep the low sword belt with an empty scabbard at the "
    "left hip and a rondel dagger at the right hip; no sword drawn, no lance, no shield.",
    # L3b: archer_2 (Genoese crossbowmen); archer_1 and archer_4 share the crossbow.
    "crossbowman": COMMON
    + "Clothing changes: the livery tabard over the padded aketon keeps its cut and dirt but is "
    "plain saturated vivid pure green wool, one single colour, no stripes and no second colour "
    "(the livery colour); the woollen hose are saturated vivid pure blue. The green tabard and "
    "the blue hose are the only green and blue items; the aketon sleeves keep their natural "
    "undyed linen colour. Keep the steel kettle hat, the mail, the leather belt with the "
    "spanning hook, the boots. Remove the pavise, the crossbow and the quiver of bolts "
    "entirely: nothing on the back, nothing at the hips except the belt hook and a sheathed "
    "dagger.",
    # L3b: infantry_1 (Flemish pikemen, sergeants); infantry_4 shares the pike.
    "sergeant": COMMON
    + "Clothing changes: the simple surcoat keeps its cut and dirt but is plain saturated vivid "
    "pure green wool, one single colour (the livery colour); the woollen hose are saturated vivid "
    "pure blue. The green surcoat and the blue hose are the only green and blue items; the "
    "quilted gambeson sleeves keep their natural undyed colour. Keep the open bascinet, the mail "
    "collar, the leather gloves, the leather belt with a purse and a sheathed dagger, the "
    "leather boots. No polearm, no guisarme, no weapon in the hands.",
    # L3b: infantry_5 (goedendag militia, Brabançons); infantry_2 (urban militia) next.
    "militia": COMMON
    + "Clothing changes: over the padded jack he wears a short open-sided livery tabard of plain "
    "saturated vivid pure green wool, one single colour (the livery colour); the woollen hose are "
    "saturated vivid pure blue. The green tabard and the blue hose are the only green and blue "
    "items; the padded jack sleeves and the woollen hood keep their natural undyed colours. Keep "
    "the iron kettle hat, the mail gorget, the leather belt with a purse and a sheathed dagger, "
    "the boots. No goedendag, no club, no buckler, no shield: nothing held and nothing on the "
    "back.",
    # L3c: the knight's rider (cavalry_0), generated dismounted and set back on the fine
    # FG4 horse by ``ga3_figures.py`` (cavalry rig). Plate legs: no hose key.
    "knight": COMMON
    + "The source shows the knight mounted: show the same knight dismounted, standing on his "
    "own feet, without the horse, the saddle, the lance or the shield. Clothing changes: the "
    "short tight padded jupon (coat armour) worn over the plate keeps its cut, padding and dirt "
    "but is plain saturated vivid pure green wool without any heraldic charge, lion, lily, "
    "quartering or stripe (the livery colour); it is the only green item. All the armour stays "
    "plain grey steel, not painted: rounded bascinet with the mail aventail, spaulders, plate "
    "vambraces, couters, gauntlets, cuisses, poleyns, greaves and sabatons with spurs. There is "
    "no blue item and no red item. Keep the knightly belt low on the hips, without any sword, "
    "scabbard or dagger hanging from it; no lance, no shield, nothing on the back. Legs slightly "
    "apart, clear gap between the thighs.",
}
# SR3 source sheet per unit when it is not ``<unit>.png``.
SOURCES = {"knight": "knight_mounted.png"}
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


def view_crop(
    cut: Path, out: Path, index: int = 0, size: int = 1536
) -> tuple[int, int, int, int]:
    """Figure `index` (left to right) of the cut sheet (alpha column runs), square canvas."""
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
    x0, x1 = runs[index]
    rows = np.nonzero(alpha[:, x0:x1].any(axis=1))[0]
    y0, y1 = int(rows[0]), int(rows[-1]) + 1
    crop = Image.fromarray(rgba[y0:y1, x0:x1])
    side = int(max(crop.width, crop.height) * 1.08)
    canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    canvas.paste(crop, ((side - crop.width) // 2, (side - crop.height) // 2))
    canvas.resize((size, size), Image.LANCZOS).save(out)
    print(f"VIEW {index} runs={runs} crop=({x0},{y0})-({x1},{y1})")
    return x0, y0, x1, y1


def main() -> None:
    """Run the cached stages for one unit."""
    parser = argparse.ArgumentParser()
    parser.add_argument("raw", type=Path)
    parser.add_argument("--unit", required=True, choices=sorted(UNITS))
    parser.add_argument("--attempt", type=int, default=1)
    parser.add_argument("--seed", type=int, default=1337)
    parser.add_argument(
        "--model", choices=("multi", "trellis", "trellis2"), default="multi"
    )
    parser.add_argument("--views", default="front,back")
    args = parser.parse_args()
    name = args.unit if args.attempt == 1 else f"{args.unit}_{args.attempt}"
    out = args.raw / name
    out.mkdir(parents=True, exist_ok=True)
    (out / "prompt.txt").write_text(UNITS[args.unit])
    sheet = out / "sheet.png"
    if not sheet.exists():
        src = fal_client.upload_file(
            str(SR3 / SOURCES.get(args.unit, f"{args.unit}.png"))
        )
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
    for index, view in enumerate(VIEW_NAMES):
        if not (out / f"{view}.png").exists():
            view_crop(cut, out / f"{view}.png", index)
    front = out / "front.png"
    if args.model == "multi":
        views = args.views.split(",")
        glb = out / f"multi_{'_'.join(views)}.glb"
        if not glb.exists():
            urls = [fal_client.upload_file(str(out / f"{v}.png")) for v in views]
            res = run(
                "fal-ai/trellis/multi",
                {
                    "image_urls": urls,
                    "multiimage_algo": "stochastic",
                    "seed": args.seed,
                    **TRELLIS_ARGS,
                },
                out,
                "multi",
            )
            download(res["model_mesh"]["url"], glb)
    elif args.model == "trellis":
        glb = out / "trellis.glb"
        if not glb.exists():
            url = fal_client.upload_file(str(front))
            res = run(
                "fal-ai/trellis",
                {"image_url": url, "seed": args.seed, **TRELLIS_ARGS},
                out,
                "trellis",
            )
            download(res["model_mesh"]["url"], glb)
    else:
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
