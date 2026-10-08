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

Free local 2D stages (ADR 0190): ``--sheet-backend local`` makes stage 1 with Z-Image Turbo
(mflux, img2img from the SR3 sheet at ``--strength``, 3:2, white background; see
``ga3_local.py``) and ``--cut-backend local`` makes stage 2 with ``rembg`` (``isnet-general-use``,
weights in ``~/.rembg/models``). Only TRELLIS then stays paid; local stages add nothing to costs.json::

    uv run --with rembg --with onnxruntime --with fal-client --with pillow --with numpy \
        python tools/experiments/ga3_fal_figure.py RAW_DIR --unit longbowman \
        --sheet-backend local --cut-backend local

Raw responses are kept as ``result_<stage>.json``; the estimated cost of every paid call is
appended to ``RAW_DIR/costs.json``.
"""

import argparse
import json
import urllib.request
from pathlib import Path

import ga3_local
import numpy as np
from PIL import Image

try:
    import fal_client
except ImportError:  # only the paid stages need it (local 2D stages, tests)
    fal_client = None

SR3 = Path.home() / "dev/cent-ans-raw/sr3"
# Catalogue prices (USD): nano-banana-2 edit at 2K (1.5 x 0.08), bria, trellis-2 at 1024.
PRICES = {
    "sheet": 0.12,
    "sheet1k": 0.08,
    "cut": 0.018,
    "trellis2": 0.30,
    "trellis": 0.02,
    "multi": 0.02,
}
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
# L4: the remaining recipes are edits of an L3 A-pose sheet (same layout, pose and scale):
# only the kit changes. Parent unit per derived unit (its ``sheet.png`` under ``L3_RAW``).
L3_RAW = Path.home() / "dev/cent-ans-raw/ga3/l3"
EDIT = (
    "Edit this photographic A-pose character reference sheet (three full-length views of the "
    "same man: front on the left, left profile in the middle, back on the right). Keep exactly "
    "the same layout, the same pose (strict A-pose, arms straight and away from the body, hands "
    "open and empty, nothing held, nothing crossing the body), the same position and scale of "
    "each figure with the feet on the same line, the same plain uniform neutral mid-grey "
    "background, the same soft studio light and the same realistic museum-reenactor "
    "photographic style with wear and dirt. No ground shadow, no text. "
)
KEY_RULE = (
    " The saturated vivid pure green garment is the only green item and the saturated vivid "
    "pure blue hose are the only blue items; nothing else is green or blue."
)
DERIVED = {
    # infantry_7: English retinue men-at-arms (pollaxe).
    "retinue": (
        "man_at_arms",
        EDIT
        + "Change the soldier into an English retinue man-at-arms of about 1415: full "
        "harness of plain grey steel (breastplate, complete arm and leg harness, gauntlets, "
        "sabatons), a short tight padded jupon of plain saturated vivid pure green wool over "
        "the breastplate without any charge or stripe (the livery colour), a visored bascinet "
        "with the visor raised and a steel bevor at the chin instead of the mail aventail. "
        "Remove the sword and its scabbard entirely; only a rondel dagger at the right hip. "
        "A different face: a man in his thirties with a short brown beard. There is no blue "
        "item; the green jupon is the only green item.",
    ),
    # infantry_8: routiers (sword and rondache).
    "routier": (
        "man_at_arms",
        EDIT
        + "Change the soldier into a mercenary routier of the 1360s with mismatched gear: "
        "remove all the plate armour of the arms and legs and the sword and scabbard; he wears "
        "a riveted brigandine covered with plain saturated vivid pure green cloth (the livery "
        "colour, rows of brass rivet heads) over a mail shirt whose short mail sleeves show "
        "over brown padded sleeves, an open bascinet without aventail over a dark brown woollen "
        "hood whose cape lies on the shoulders, leather gloves, saturated vivid pure blue "
        "woollen hose, worn brown leather boots, a leather belt with a sheathed dagger. A "
        "scarred unshaven face." + KEY_RULE,
    ),
    # infantry_2: urban militia (spear, bill).
    "urban_militia": (
        "militia",
        EDIT
        + "Change the soldier into a town militiaman: a plain undyed linen tunic with long "
        "sleeves under a short open-sided livery tabard of plain saturated vivid pure green "
        "wool, saturated vivid pure blue woollen hose, a round red-brown felt cap instead of "
        "the helmet, no mail gorget, no hood, bare hands, a leather belt with a purse and a "
        "sheathed knife, simple brown leather ankle shoes. A round-faced man with a brown "
        "moustache." + KEY_RULE,
    ),
    # infantry_3: Welsh spearmen (spear, small buckler).
    "welsh_spearman": (
        "militia",
        EDIT
        + "Change the soldier into a Welsh spearman: a knee-length woollen tunic divided "
        "vertically in two colours, the half on the wearer's right plain saturated vivid pure "
        "green (the livery colour) and the half on the wearer's left natural undyed off-white; "
        "bare legs with bare skin from the knee down and simple low leather shoes; a small "
        "round grey felt cap, no helmet, no mail, no tabard, no hood; a leather belt with a "
        "long knife. A lean dark-haired man with a black moustache. The green half of the "
        "tunic is the only green item; there is no blue item.",
    ),
    # infantry_4: Scottish schiltron (long spear, targe on the back).
    "schiltron": (
        "sergeant",
        EDIT
        + "Change the soldier into a Scottish spearman of the schiltron: a thick brown "
        "quilted padded jack reaching mid-thigh with its brown quilted sleeves, a short "
        "sleeveless tabard of plain saturated vivid pure green wool over it (the livery "
        "colour), a flat dark brown woollen bonnet instead of the bascinet, no mail, leather "
        "gloves, saturated vivid pure blue woollen hose, brown leather boots, a leather belt "
        "with a sheathed dirk. A fair-haired man with a short fair beard." + KEY_RULE,
    ),
    # infantry_6: coutiliers (coustille).
    "coutilier": (
        "militia",
        EDIT
        + "Change the soldier into a French coutilier of about 1450: a riveted brigandine "
        "covered with plain saturated vivid pure green cloth (the livery colour) with rows of "
        "rivet heads, padded sleeves with mail at the elbows, a steel sallet with a short tail "
        "and a steel bevor at the chin instead of the kettle hat and the mail gorget, no hood, "
        "leather gloves, saturated vivid pure blue woollen hose, brown leather boots, a leather "
        "belt with a sheathed dagger. A clean-shaven face." + KEY_RULE,
    ),
    # archer_1: crossbowmen (crossbow, bolt case).
    "plain_crossbowman": (
        "crossbowman",
        EDIT
        + "Change the livery tabard and the aketon into one padded quilted gambeson "
        "reaching mid-thigh, body and sleeves all of plain saturated vivid pure green cloth "
        "(the livery colour); an open bascinet without aventail instead of the kettle hat, no "
        "mail; keep the saturated vivid pure blue hose, the leather belt with the spanning "
        "hook and the sheathed dagger, the boots. A man with a thin brown beard."
        + KEY_RULE,
    ),
    # archer_4: Gascon crossbowmen (crossbow, bolt case, pavise).
    "gascon_crossbowman": (
        "crossbowman",
        EDIT
        + "Change the soldier into a Gascon crossbowman: a haubergeon (mail shirt) with "
        "long mail sleeves reaching the wrists, worn under a short livery tabard of plain "
        "saturated vivid pure green wool, an open bascinet with a mail aventail instead of the "
        "kettle hat; keep the saturated vivid pure blue hose, the leather belt with the "
        "spanning hook and a sheathed dagger, the boots. A dark-eyed man with a black "
        "moustache." + KEY_RULE,
    ),
    # cavalry_3: ordonnance gendarmes (lance), rider dismounted.
    "gendarme": (
        "knight",
        EDIT
        + "Change the knight into a French ordonnance gendarme of about 1450: polished "
        "bright white plate harness (full armour, plain steel, no fabric on the arms and legs), "
        "over the breastplate a short sleeveless open-sided livery huque of plain saturated "
        "vivid pure green cloth reaching the upper thighs, a steel sallet with the visor raised "
        "and a steel bevor at the chin instead of the bascinet and the aventail. No sword, no "
        "scabbard, no dagger, nothing held. There is no blue item and no red item; the green "
        "huque is the only green item. Legs slightly apart, clear gap between the thighs.",
    ),
    # standard_1: mounted standard bearer (pole), rider dismounted.
    "standard_bearer": (
        "knight",
        EDIT
        + "Change the knight into a young squire, the standard bearer: the same plain grey "
        "steel harness and the same plain saturated vivid pure green jupon (the livery colour, "
        "no charge), an open bascinet without visor and without aventail but with a mail "
        "collar, a clean-shaven young face with brown hair showing. No sword, no scabbard, no "
        "dagger, nothing held. There is no blue item and no red item; the green jupon is the "
        "only green item. Legs slightly apart, clear gap between the thighs.",
    ),
}
UNITS.update({name: prompt for name, (_parent, prompt) in DERIVED.items()})
# L4: face variants. An edit of the unit's own A-pose sheet that changes the head only; the
# body stays identical so that ``ga3_figures.py`` grafts the new head on the shared body.
VARIANT = (
    "Edit this photographic A-pose character reference sheet. Change ONLY the head of the man, "
    "consistently in all three views: {}. Everything from the collar down stays exactly "
    "identical: same body, same clothing, armour and colours (the saturated green and blue "
    "garments unchanged), same belt, same boots, same hands, same A-pose, same position and "
    "scale of each figure with the feet on the same line, same plain mid-grey background, same "
    "photographic style. Nothing green or blue on the head. Nothing held in the hands."
)
VARIANTS = {
    "longbowman": [
        "an older man of about fifty with a short grey beard and deeply weathered tanned skin, "
        "wearing a round brown felt cap instead of the steel kettle hat",
    ],
    "man_at_arms": [
        "a younger man with a thick black moustache and olive skin, wearing the same pointed "
        "bascinet with the visor removed and the same mail aventail",
    ],
    "crossbowman": [
        "a dark-haired man with a full black beard and olive Mediterranean skin, wearing an "
        "open steel bascinet instead of the kettle hat, over the same mail collar",
    ],
    "sergeant": [
        "a red-haired man with a ginger beard and pale freckled skin, wearing a steel kettle "
        "hat instead of the bascinet, over the same mail collar",
    ],
    "militia": [
        "a bald older man with grey stubble and weathered skin, bare-headed without any helmet",
    ],
    "knight": [
        "a clean-shaven young man with a fair complexion, wearing a rounded bascinet with a "
        "closed pointed visor (hounskull) over the same mail aventail",
    ],
    "retinue": [
        "an older man with a grey beard, wearing a steel sallet with a short tail instead of "
        "the bascinet, over the same bevor",
    ],
    "routier": [
        "a bald man with a black beard, without the bascinet: only the dark brown hood worn up "
        "on the head",
    ],
    "urban_militia": [
        "a thin older man with a long grey beard, wearing an open steel kettle hat instead of "
        "the felt cap",
    ],
    "welsh_spearman": [
        "a young red-haired man with a short red beard, bare-headed without the cap",
    ],
    "schiltron": [
        "a dark-haired man with a black beard, wearing an open steel bascinet instead of the "
        "bonnet",
    ],
    "coutilier": [
        "a man with a brown moustache, wearing a steel sallet without the bevor (the chin "
        "bare above the same collar)",
    ],
    "plain_crossbowman": [
        "a clean-shaven man with tanned skin, wearing a steel kettle hat instead of the "
        "bascinet",
    ],
    "gascon_crossbowman": [
        "an older man with a grey moustache and weathered skin, wearing a steel kettle hat "
        "instead of the bascinet, over the same mail aventail",
    ],
    "gendarme": [
        "an older man with a grey moustache and a weathered face, the same sallet with the "
        "visor raised over the same bevor",
    ],
    "standard_bearer": [
        "a man with a short brown beard, wearing a steel kettle hat instead of the bascinet, "
        "over the same mail collar",
    ],
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


def figure_box(cut: Path) -> dict:
    """Front figure of the cut sheet: top and feet rows over the sheet height (L4).

    The head variants and the derived units keep the pose and the scale of their source
    sheet: ``ga3_figures.py`` scales their helmet height by the ratio of the figure heights.
    """
    rgba = np.asarray(Image.open(cut).convert("RGBA"))
    alpha = rgba[..., 3] > 32
    cols = alpha.sum(axis=0) > 2
    # First run of columns wider than 3 % of the sheet (the front view; crumbs skipped).
    x0 = x1 = None
    for x, on in enumerate(list(cols) + [False]):
        if on and x0 is None:
            x0 = x
        elif not on and x0 is not None:
            if x - x0 > 0.03 * len(cols):
                x1 = x
                break
            x0 = None
    rows = np.nonzero(alpha[:, x0:x1].any(axis=1))[0]
    h = alpha.shape[0]
    return {"top": float(rows[0]) / h, "feet": float(rows[-1] + 1) / h, "height": h}


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
    parser.add_argument(
        "--variant",
        type=int,
        default=1,
        help="L4 face variant k >= 2: head-only edit of the unit's own sheet (VARIANTS)",
    )
    parser.add_argument("--sheet-backend", choices=ga3_local.BACKENDS, default="fal")
    parser.add_argument("--cut-backend", choices=ga3_local.BACKENDS, default="fal")
    parser.add_argument(
        "--strength",
        type=float,
        default=ga3_local.SHEET_STRENGTH,
        help="local sheet: influence of the source image (img2img)",
    )
    parser.add_argument("--resolution", choices=("1K", "2K"), default="2K")
    args = parser.parse_args()
    name = args.unit if args.attempt == 1 else f"{args.unit}_{args.attempt}"
    if args.variant > 1:
        name = f"{args.unit}_v{args.variant}" + (
            "" if args.attempt == 1 else f"_{args.attempt}"
        )
        prompt = VARIANT.format(VARIANTS[args.unit][args.variant - 2])
        own = L3_RAW if (L3_RAW / args.unit / "sheet.png").exists() else args.raw
        source = own / args.unit / "sheet.png"
    elif args.unit in DERIVED:
        prompt = UNITS[args.unit]
        source = L3_RAW / DERIVED[args.unit][0] / "sheet.png"
    else:
        prompt = UNITS[args.unit]
        source = SR3 / SOURCES.get(args.unit, f"{args.unit}.png")
    out = args.raw / name
    out.mkdir(parents=True, exist_ok=True)
    (out / "prompt.txt").write_text(prompt)
    (out / "source.txt").write_text(str(source))
    sheet = out / "sheet.png"
    if args.sheet_backend == "local":
        ga3_local.render_local(
            ga3_local.local_sheet_prompt(prompt),
            sheet,
            reference=source,
            aspect_ratio=ga3_local.SHEET_ASPECT,
            seed=args.seed + args.attempt - 1,
            strength=args.strength,
        )
    elif not sheet.exists():
        src = fal_client.upload_file(str(source))
        res = run(
            "fal-ai/nano-banana-2/edit",
            {
                "prompt": prompt,
                "image_urls": [src],
                "num_images": 1,
                "aspect_ratio": "16:9",
                "resolution": args.resolution,
                "output_format": "png",
                "seed": args.seed + args.attempt - 1,
            },
            out,
            "sheet" if args.resolution == "2K" else "sheet1k",
        )
        download(res["images"][0]["url"], sheet)
    cut = out / "sheet_cut.png"
    if args.cut_backend == "local":
        ga3_local.cut_local(sheet, cut)
    elif not cut.exists():
        url = fal_client.upload_file(str(sheet))
        res = run("fal-ai/bria/background/remove", {"image_url": url}, out, "cut")
        download(res["image"]["url"], cut)
    box = out / "box.json"
    if not box.exists():
        box.write_text(json.dumps(figure_box(cut)))
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
