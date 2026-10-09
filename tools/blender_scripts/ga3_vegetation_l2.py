"""GA3-L2: realistic campaign-map vegetation (production lot, verdict of probe S5).

Steps (raw files in ``~/dev/cent-ans-raw/ga3/l2/``, run from the repository root)::

    uv run --with fal-client python tools/blender_scripts/ga3_vegetation_l2.py fal [names]
    uv run --project tools python tools/blender_scripts/ga3_vegetation_l2.py atlas
    uv run --project tools python tools/blender_scripts/ga3_vegetation_l2.py rocks

* ``fal``: per essence (oak, beech, fir) one ``fal-ai/flux-2`` reference view on grey, then ONE
  ``fal-ai/nano-banana-2/edit`` call turning it into a 4 x 2 turnaround sheet (8 azimuths of the
  same tree, 45 degrees apart) and one ``fal-ai/bria/background/remove`` on the whole sheet.
  A single sheet is cheaper than seven separate edits (0.08 $ instead of 0.56 $ per essence) and
  more coherent: the eight views share one generation (lighting, palette, scale).
  Also a dense grass clump (flux-2 + bria) and two more rocks (flux-2 + bria + ``fal-ai/trellis``).
* ``atlas``: the 3 x 8 impostor grid (256 px cells, same framing as the FC2 atlas: each view is
  fitted into the silhouette box of the matching FC2 row, colours matched to the FC2 row mean in
  linear space), an approximate normal map (crown dome from the distance to the silhouette edge
  plus luminance detail, view frame x right / y up / z to the camera, A = occlusion), the leaf-card
  atlas (FC5 normalisation, fir half kept) and the grass tuft (512 x 256 like FC5, alpha dilated so
  the thin blades survive the 0.5 cut).
* ``rocks``: ``ga3_cleanup.py`` (Blender) on the three TRELLIS rocks, 120 / 60 / 18 triangles,
  1 m long, 256 px albedo.
* ``sheet <species> <glb>`` (lot DN nature): glb -> impostor input. Renders the 8 views of an
  ingested tree glb (``cent-ans dn-ingest ... --class tree``, ``tools/blender_scripts/
  dn_tree_views.py``, flat albedo, 25 degrees) and composes ``<species>_sheet_cut.png`` (4 x 2,
  transparent) in ``RAW``, exactly what ``atlas`` reads: no fal call, no turnaround sheet.
* ``species`` (lot HB4, ADR 0143): compiles ``data/art/tree_species.yaml`` into
  ``data/art/tree_species.json`` (the game does not read YAML). ``atlas`` does it too.

Lot HB4 extends the essences to the catalogue ``data/art/tree_species.yaml``: ``fal`` generates
every species of the catalogue (same prompt frame, seed and chain as the first three), ``atlas``
writes one impostor row per species in catalogue order (rows 0-2 unchanged: oak, beech, fir), a
single shared atlas so the tree draw calls do not grow, and ``board`` a local contact sheet.

Outputs: ``game/assets/textures/vegetation/ga3/`` and ``game/assets/models/vegetation/ga3/``.
Consumed by ``game/scripts/map/ga3_vegetation.gd``.
"""

import json
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

import numpy as np

RAW = Path.home() / "dev/cent-ans-raw/ga3/l2"
BOARD = Path.home() / "dev/cent-ans-raw/ga3/hb4/species_board.jpg"
S5 = Path.home() / "dev/cent-ans-raw/ga3/s5"
REPO = Path(__file__).resolve().parents[2]
VEG = REPO / "game/assets/textures/vegetation"
OUT_TEX = VEG / "ga3"
OUT_MODELS = REPO / "game/assets/models/vegetation/ga3"
SPECIES_YAML = REPO / "data/art/tree_species.yaml"
SPECIES_JSON = REPO / "data/art/tree_species.json"

CELL = 256
VIEWS = 8
CONIFER_ROWS = ("fir",)
ESSENCES = ("oak", "beech", "fir")  # VegetationMeshes.IMPOSTOR_ROWS (FC2 rows)
# Season classes of ``foliage_common.gdshaderinc`` (``foliage_season``); 3 is the hedge.
SEASON_CLASSES = {"oak": 0, "beech": 1, "evergreen": 2, "golden": 4}
ROLES = ("massif", "lisiere", "isole", "verger", "ripisylve", "garrigue")
SHEET_COLS, SHEET_ROWS = 4, 2

GREY = (
    "isolated on a plain uniform neutral mid-grey studio background, no ground, no grass, "
    "no shadow, soft diffuse overcast daylight, photorealistic, sharp focus, high detail"
)
WHITE = (
    "isolated on a plain pure white seamless studio background, no shadow, no ground, "
    "soft even diffuse lighting, photorealistic botanical scan, sharp focus, high detail"
)
TREES = {
    "oak": "a single mature pedunculate oak tree (Quercus robur) standing alone, short thick gnarled "
    "trunk, broad irregular domed crown of dense summer foliage with visible leaf clumps and a few sky gaps",
    "beech": "a single mature European beech tree (Fagus sylvatica) standing alone, smooth silver-grey "
    "trunk, tall dense oval crown of fresh green summer foliage with visible leaf clumps",
    "fir": "a single mature silver fir tree (Abies alba) standing alone, straight conical evergreen, "
    "dense dark green layered branches down to near the ground, narrow pointed top",
}
TREE_VIEW = (
    ", the whole tree visible from trunk base to crown top with a margin, seen from slightly "
    "above at about 25 degrees elevation, "
)
SHRUB_VIEW = (
    ", the whole shrub visible from its base on the ground line to its top with a margin, seen "
    "from slightly above at about 25 degrees elevation, "
)
# Species below this height (map units) are shrubs (maquis, garrigue): shrub framing.
SHRUB_HEIGHT = 0.8
SHEET_PROMPT = (
    "Turnaround reference sheet of the exact same tree as in the input image, shown 8 times in a "
    "grid of 4 columns and 2 rows. Each panel shows the tree rotated a further 45 degrees around "
    "its vertical trunk axis (0, 45, 90, 135, 180, 225, 270, 315 degrees), with the same camera "
    "height and slight downward angle, the same scale, the same soft overcast lighting and the "
    "same colours. Every tree is complete from trunk base to crown top, centred in its panel with "
    "a clear margin; the trees never touch or overlap. Plain uniform neutral mid-grey background, "
    "no ground, no shadow, no text, no labels, no borders, no grid lines."
)
GRASS_PROMPT = (
    "Side view of a single dense bushy clump of wild meadow grass, hundreds of long overlapping "
    "blades fanning out from one base, mostly fresh green with some straw-yellow blades, blades "
    "clearly visible, the clump is wider than tall and fills the whole frame, " + WHITE
)
ROCKS = {
    "rock_b": "A single weathered grey granite boulder, angular with a few flat facets and cracks, "
    "patches of pale yellow-green lichen, whole rock visible, three-quarter view from slightly above, "
    + GREY,
    "rock_c": "A single low flat weathered sandstone rock slab, rounded eroded edges, ochre-grey with "
    "dark moss at the base, whole rock visible, three-quarter view from slightly above, "
    + GREY,
}
TRELLIS_ARGS = {"texture_size": 1024, "mesh_simplify": 0.95, "seed": 1337}
# Prices (fal pricing API, 2026-09-30).
PRICE = {
    "fal-ai/flux-2": 0.012,  # per megapixel
    "fal-ai/nano-banana-2/edit": 0.08,
    "fal-ai/bria/background/remove": 0.018,
    "fal-ai/trellis": 0.02,
}


# ---------------------------------------------------------------- colour helpers
def srgb_to_linear(c: np.ndarray) -> np.ndarray:
    """SRGB [0, 1] to linear."""
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c: np.ndarray) -> np.ndarray:
    """Linear to sRGB [0, 1]."""
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)


def bleed(rgb: np.ndarray, known: np.ndarray, iterations: int = 24) -> np.ndarray:
    """Spread the colours of ``known`` pixels into the others (no dark mipmap fringes)."""
    rgb = rgb.copy()
    known = known.copy()
    h, w = known.shape
    for _ in range(iterations):
        if known.all():
            break
        acc = np.zeros_like(rgb)
        cnt = np.zeros(known.shape, dtype=np.float32)
        pad_rgb = np.pad(rgb * known[..., None], ((1, 1), (1, 1), (0, 0)))
        pad_k = np.pad(known.astype(np.float32), 1)
        for dy, dx in ((0, 1), (2, 1), (1, 0), (1, 2)):
            acc += pad_rgb[dy : dy + h, dx : dx + w]
            cnt += pad_k[dy : dy + h, dx : dx + w]
        grow = (~known) & (cnt > 0)
        rgb[grow] = acc[grow] / cnt[grow][:, None]
        known = known | grow
    if known.any():
        rgb[~known] = rgb[known].mean(axis=0)
    return rgb


# ---------------------------------------------------------------- sheet and cell helpers
def split_sheet(
    rgba: np.ndarray, cols: int = SHEET_COLS, rows: int = SHEET_ROWS
) -> list[np.ndarray]:
    """Cut a background-removed turnaround sheet into ``cols * rows`` tight RGBA crops.

    Connected silhouettes first (robust to panels that are not on an exact grid): the
    ``cols * rows`` largest components, fragments attached to the nearest one, sorted by row then
    column. Falls back to the regular grid when the count does not match.
    """
    from scipy import ndimage

    alpha = rgba[..., 3] > 0.5
    labels, count = ndimage.label(alpha)
    wanted = cols * rows
    crops: list[np.ndarray] = []
    if count >= wanted:
        areas = ndimage.sum(alpha, labels, index=np.arange(1, count + 1))
        order = np.argsort(areas)[::-1]
        big = [int(i) + 1 for i in order[:wanted]]
        if areas[order[wanted - 1]] > 0.15 * areas[order[0]]:
            centres = ndimage.center_of_mass(alpha, labels, big)
            # nearest big component for every labelled pixel (fragments included)
            ys, xs = np.nonzero(labels)
            dist = np.stack([(ys - cy) ** 2 + (xs - cx) ** 2 for cy, cx in centres])
            owner = np.full(labels.shape, -1)
            owner[ys, xs] = np.argmin(dist, axis=0)
            mid_y = np.median([c[0] for c in centres])
            keyed = sorted(
                range(wanted), key=lambda k: (centres[k][0] > mid_y, centres[k][1])
            )
            for k in keyed:
                mask = owner == k
                yy, xx = np.nonzero(mask)
                crop = rgba[yy.min() : yy.max() + 1, xx.min() : xx.max() + 1].copy()
                crop[..., 3] *= mask[yy.min() : yy.max() + 1, xx.min() : xx.max() + 1]
                crops.append(crop)
            return crops
    h, w = alpha.shape
    for r in range(rows):
        for c in range(cols):
            panel = rgba[
                r * h // rows : (r + 1) * h // rows, c * w // cols : (c + 1) * w // cols
            ]
            yy, xx = np.nonzero(panel[..., 3] > 0.5)
            crops.append(panel[yy.min() : yy.max() + 1, xx.min() : xx.max() + 1].copy())
    return crops


def split_sheet_panels(
    rgba: np.ndarray, cols: int = SHEET_COLS, rows: int = SHEET_ROWS
) -> list[np.ndarray]:
    """Cut a sheet by assigning each connected piece to the grid panel of its centre (HB4).

    Robust when thin trunks detach crowns from their base (tall pines): every fragment follows
    its own panel instead of the nearest large component.
    """
    from scipy import ndimage

    alpha = rgba[..., 3] > 0.5
    labels, count = ndimage.label(alpha)
    h, w = alpha.shape
    centres = ndimage.center_of_mass(alpha, labels, np.arange(1, count + 1))
    panel_of = np.full(count + 1, -1)
    for index, (cy, cx) in enumerate(centres, start=1):
        r = min(int(cy / (h / rows)), rows - 1)
        c = min(int(cx / (w / cols)), cols - 1)
        panel_of[index] = r * cols + c
    owner = panel_of[labels]
    crops = []
    for k in range(cols * rows):
        mask = (owner == k) & alpha
        yy, xx = np.nonzero(mask)
        crop = rgba[yy.min() : yy.max() + 1, xx.min() : xx.max() + 1].copy()
        crop[..., 3] *= mask[yy.min() : yy.max() + 1, xx.min() : xx.max() + 1]
        crops.append(crop)
    return crops


def consistent_crops(crops: list[np.ndarray], spread: float = 1.3) -> bool:
    """True when no view is broken (a neighbour's fragment glued above or below a tree).

    Heights stay within ``spread`` of the median and no crop has an empty row band.
    """
    heights = np.array([c.shape[0] for c in crops], dtype=np.float64)
    median = float(np.median(heights))
    if not (np.all(heights <= median * spread) and np.all(heights >= median / spread)):
        return False
    for crop in crops:
        filled = (crop[..., 3] > 0.5).any(axis=1)
        if (~filled).sum() > 0.02 * crop.shape[0]:
            return False
    return True


def drop_specks(crop: np.ndarray, fraction: float = 0.02) -> np.ndarray:
    """Clear detached blobs smaller than ``fraction`` of the main one (ground-shadow remnants)."""
    from scipy import ndimage

    labels, count = ndimage.label(crop[..., 3] > 0.5)
    if count <= 1:
        return crop
    areas = ndimage.sum(np.ones(labels.shape), labels, index=np.arange(1, count + 1))
    keep = np.isin(labels, 1 + np.nonzero(areas >= fraction * areas.max())[0])
    near = ndimage.binary_dilation(keep, iterations=2)
    out = crop.copy()
    out[..., 3] *= near
    yy, xx = np.nonzero(out[..., 3] > 0.02)
    return out[yy.min() : yy.max() + 1, xx.min() : xx.max() + 1]


def silhouette_box(cell: np.ndarray) -> tuple[float, float, float]:
    """(top row, bottom row, foot column) of an RGBA cell's opaque silhouette."""
    ys, xs = np.nonzero(cell[..., 3] > 0.5)
    top, bottom = ys.min(), ys.max()
    foot = xs[ys >= bottom - max(2, (bottom - top) * 0.06)].mean()
    return float(top), float(bottom), float(foot)


def fit_cell(
    crop: np.ndarray, box: tuple[float, float, float], cell: int = CELL
) -> np.ndarray:
    """Scale ``crop`` (tight RGBA) so its silhouette spans ``box`` = (top, bottom, foot x)."""
    from PIL import Image

    top, bottom, foot_x = box
    _, crop_bottom, crop_foot = silhouette_box(crop)
    h, w = crop.shape[:2]
    scale = (bottom - top + 1) / h
    scale = min(scale, (cell - 4) / w)
    nw, nh = max(1, round(w * scale)), max(1, round(h * scale))
    premul = crop.copy()
    premul[..., :3] *= premul[..., 3:4]
    channels = [
        np.asarray(
            Image.fromarray(premul[..., i].astype(np.float32), "F").resize(
                (nw, nh), Image.LANCZOS
            )
        )
        for i in range(4)
    ]
    small = np.clip(np.stack(channels, axis=-1), 0.0, 1.0)
    out = np.zeros((cell, cell, 4), dtype=np.float32)
    y0 = round(bottom - crop_bottom * scale)
    x0 = round(foot_x - crop_foot * scale)
    ys0, xs0 = max(0, -y0), max(0, -x0)
    ys1, xs1 = min(nh, cell - y0), min(nw, cell - x0)
    out[y0 + ys0 : y0 + ys1, x0 + xs0 : x0 + xs1] = small[ys0:ys1, xs0:xs1]
    alpha = out[..., 3:4]
    out[..., :3] = np.where(alpha > 1e-4, out[..., :3] / np.maximum(alpha, 1e-4), 0.0)
    return out


def crown_normals(
    alpha: np.ndarray, lum: np.ndarray, strength: float = 6.0
) -> np.ndarray:
    """Approximate view-frame normals + occlusion for a cut-out tree (RGBA, encoded n*0.5+0.5).

    Height = dome from the distance to the silhouette edge (rounded crown) plus fine luminance
    detail; normal = (-dh/dx, +dh/drow, 1) (image rows go down, the normal's y goes up) with a
    slight sky bias like the FC2 bake. A = occlusion from the luminance (dark hollows).
    """
    from scipy import ndimage

    inside = alpha > 0.5
    dist = ndimage.distance_transform_edt(inside)
    dome = np.sqrt(dist / max(dist.max(), 1.0))
    dome = ndimage.gaussian_filter(dome, 2.0)
    detail = lum - ndimage.gaussian_filter(lum, 3.0)
    height = dome * strength + detail * strength * 1.5
    gy, gx = np.gradient(height)
    n = np.stack([-gx, gy + 0.2, np.ones_like(height)], axis=-1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    ref = np.percentile(lum[inside], 90) if inside.any() else 1.0
    ao = np.clip(0.35 + 0.65 * lum / max(ref, 1e-4), 0.0, 1.0)
    out = np.concatenate([n * 0.5 + 0.5, ao[..., None]], axis=-1)
    out[~inside] = (0.5, 0.5, 1.0, 1.0)
    return out


def match_mean(cells: list[np.ndarray], target_linear: np.ndarray) -> list[np.ndarray]:
    """Scale the linear colours of ``cells`` (sRGB RGBA) so their joint opaque mean is ``target_linear``."""
    lin = [srgb_to_linear(c[..., :3]) for c in cells]
    opaque = np.concatenate(
        [lin_c[c[..., 3] > 0.5] for lin_c, c in zip(lin, cells, strict=True)]
    )
    ratio = target_linear / np.maximum(opaque.mean(axis=0), 1e-4)
    out = []
    for lin_c, c in zip(lin, cells, strict=True):
        rgb = bleed(lin_c * ratio, c[..., 3] > 0.5)
        out.append(np.concatenate([linear_to_srgb(rgb), c[..., 3:4]], axis=-1))
    return out


def fc_row_reference(
    atlas: np.ndarray, row: int
) -> tuple[tuple[float, float, float], np.ndarray]:
    """Mean silhouette box and mean opaque linear colour of one FC2 atlas row."""
    boxes, colours = [], []
    for view in range(VIEWS):
        cell = atlas[row * CELL : (row + 1) * CELL, view * CELL : (view + 1) * CELL]
        boxes.append(silhouette_box(cell))
        colours.append(srgb_to_linear(cell[..., :3])[cell[..., 3] > 0.5].mean(axis=0))
    return tuple(np.mean(boxes, axis=0).tolist()), np.mean(colours, axis=0)


# ---------------------------------------------------------------- species catalogue (HB4)
def load_species(path: Path = SPECIES_YAML) -> dict:
    """The species catalogue (``data/art/tree_species.yaml``)."""
    import yaml

    return yaml.safe_load(path.read_text(encoding="utf-8"))


def is_shrub(species: dict) -> bool:
    """Maquis / garrigue shrub (low plant, shrub framing in the prompt)."""
    return float(species["height"][1]) < SHRUB_HEIGHT


def species_prompt(species: dict) -> str:
    """Reference-view prompt of a species (same frame as the first three essences)."""
    subject = " ".join(str(species["prompt"]).split())
    view = SHRUB_VIEW if is_shrub(species) else TREE_VIEW
    return subject[0].upper() + subject[1:] + view + GREY


def compile_species(catalogue: dict) -> dict:
    """Runtime table (``tree_species.json``): catalogue + atlas ``row`` and ``season_class``.

    Rows follow the catalogue order; the first three must be the FC2 rows (oak, beech, fir) so
    the FC atlas and meshes stay valid fallbacks.
    """
    ids = [s["id"] for s in catalogue["species"]]
    if tuple(ids[:3]) != ESSENCES:
        raise ValueError(f"the first species must be {ESSENCES}, got {ids[:3]}")
    if len(set(ids)) != len(ids):
        raise ValueError("duplicate species id")
    out = {
        "description": "Compilé depuis data/art/tree_species.yaml par "
        "tools/blender_scripts/ga3_vegetation_l2.py (species / atlas). Ne pas éditer.",
        "roles": list(ROLES),
        "seasons": dict(SEASON_CLASSES),
        "distribution": catalogue["distribution"],
        "biomes": catalogue["biomes"],
        "species": [],
    }
    for row, species in enumerate(catalogue["species"]):
        entry = dict(species)
        entry["prompt"] = " ".join(str(species["prompt"]).split())
        entry["row"] = row
        entry["season_class"] = SEASON_CLASSES[species["season"]]
        out["species"].append(entry)
    return out


def species_step() -> None:
    """Write ``data/art/tree_species.json`` from the YAML catalogue."""
    table = compile_species(load_species())
    SPECIES_JSON.write_text(
        json.dumps(table, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"OK species ({len(table['species'])})")


# ---------------------------------------------------------------- fal step
def _download(url: str, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    urllib.request.urlretrieve(url, path)


def fal_step(only: list[str]) -> None:
    """Reference views, turnaround sheets, grass, rocks (skips names already downloaded)."""
    import fal_client

    RAW.mkdir(parents=True, exist_ok=True)
    log_path = RAW / "fal_log.json"
    log = json.loads(log_path.read_text()) if log_path.exists() else {}
    cost = 0.0

    def run(app: str, args: dict, units: float = 1.0) -> dict:
        nonlocal cost
        result = fal_client.subscribe(app, arguments=args)
        cost += PRICE[app] * units
        return result

    def cut(name: str, url: str) -> str:
        out = run("fal-ai/bria/background/remove", {"image_url": url})["image"]["url"]
        _download(out, RAW / f"{name}_cut.png")
        return out

    for species in load_species()["species"]:
        name = species["id"]
        if (only and name not in only) or (RAW / f"{name}_sheet_cut.png").exists():
            continue
        prompt = TREES[name] if name in TREES else species_prompt(species)
        if name in TREES:
            prompt = prompt[0].upper() + prompt[1:] + TREE_VIEW + GREY
        noun = "shrub" if is_shrub(species) else "tree"
        ref = run(
            "fal-ai/flux-2",
            {
                "prompt": prompt,
                "image_size": "square_hd",
                "num_images": 1,
                "seed": int(species.get("seed", 1337)),
                "output_format": "png",
            },
        )["images"][0]["url"]
        _download(ref, RAW / f"{name}_ref.png")
        sheet = run(
            "fal-ai/nano-banana-2/edit",
            {
                "prompt": SHEET_PROMPT.replace(" tree", " " + noun),
                "image_urls": [ref],
                "num_images": 1,
                "aspect_ratio": "16:9",
                "output_format": "png",
            },
        )["images"][0]["url"]
        _download(sheet, RAW / f"{name}_sheet.png")
        log[name] = {
            "prompt": prompt,
            "ref": ref,
            "sheet": sheet,
            "sheet_cut": cut(f"{name}_sheet", sheet),
        }
        print("FAL", name, "ok")
    if (not only or "grass" in only) and not (RAW / "grass_cut.png").exists():
        src = run(
            "fal-ai/flux-2",
            {
                "prompt": GRASS_PROMPT,
                "image_size": {"width": 1024, "height": 512},
                "num_images": 1,
                "seed": 7,
                "output_format": "png",
            },
            0.5,
        )["images"][0]["url"]
        _download(src, RAW / "grass_src.png")
        log["grass"] = {"prompt": GRASS_PROMPT, "src": src, "cut": cut("grass", src)}
        print("FAL grass ok")
    for name, prompt in ROCKS.items():
        if (only and name not in only) or (RAW / f"{name}_trellis.glb").exists():
            continue
        src = run(
            "fal-ai/flux-2",
            {
                "prompt": prompt,
                "image_size": "square_hd",
                "num_images": 1,
                "seed": 1337,
                "output_format": "png",
            },
        )["images"][0]["url"]
        _download(src, RAW / f"{name}_src.png")
        cut_url = cut(name, src)
        glb = run("fal-ai/trellis", {"image_url": cut_url, **TRELLIS_ARGS})[
            "model_mesh"
        ]["url"]
        _download(glb, RAW / f"{name}_trellis.glb")
        log[name] = {"prompt": prompt, "src": src, "cut": cut_url, "glb": glb}
        print("FAL", name, "ok")
    log["_cost_usd"] = round(float(log.get("_cost_usd", 0.0)) + cost, 4)
    log_path.write_text(json.dumps(log, indent=1))
    print(f"COST {cost:.3f} $ (total {log['_cost_usd']} $)")


# ---------------------------------------------------------------- atlas step
def _load(path: Path) -> np.ndarray:
    from PIL import Image

    return np.asarray(Image.open(path).convert("RGBA")).astype(np.float32) / 255.0


def _save(arr: np.ndarray, path: Path) -> None:
    from PIL import Image

    path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(np.round(np.clip(arr, 0, 1) * 255).astype(np.uint8), "RGBA").save(
        path
    )


GREEN_RATIO = {"broadleaf": (1.22, 0.5), "conifer": (1.12, 0.6)}


def foliage_chroma(cells: list[np.ndarray], conifer: bool) -> list[np.ndarray]:
    """Lot DN-FORET: pull the generated textures' beige / grey foliage towards green.

    The baked TRELLIS trees are desaturated (charter ADR 0211, albedo graded on the whole
    model): at campaign scale they read as dead wood. Only the channel ratios of the mean
    opaque linear colour are corrected (green / red at least ``gr``, blue / green at most
    ``bg``); the brightness is brought back by ``match_luminance`` afterwards.
    """
    gr, bg = GREEN_RATIO["conifer" if conifer else "broadleaf"]
    mean = opaque_mean(cells)
    r_gain = min(1.0, (mean[1] / max(mean[0], 1e-4)) / gr)
    b_gain = min(1.0, bg / max(mean[2] / max(mean[1], 1e-4), 1e-4))
    ratio = np.array([r_gain, 1.0, b_gain])
    out = []
    for c in cells:
        lin = srgb_to_linear(c[..., :3]) * ratio
        out.append(np.concatenate([linear_to_srgb(lin), c[..., 3:4]], axis=-1))
    return out


def match_luminance(cells: list[np.ndarray], target_lum: float) -> list[np.ndarray]:
    """Scale ``cells`` (sRGB RGBA) so their joint opaque linear luminance is ``target_lum``.

    Lot HB4: the new species keep their generated hue (olive silver, cypress dark green, birch
    light) and only take the brightness of their family, so they sit with the first three.
    """
    weights = np.array([0.2126, 0.7152, 0.0722])
    lin = [srgb_to_linear(c[..., :3]) for c in cells]
    opaque = np.concatenate(
        [lin_c[c[..., 3] > 0.5] for lin_c, c in zip(lin, cells, strict=True)]
    )
    mean = opaque.mean(axis=0)
    return match_mean(cells, mean * target_lum / max(float(mean @ weights), 1e-4))


def _species_cells(name: str, box: tuple[float, float, float]) -> list[np.ndarray]:
    sheet = _load(RAW / f"{name}_sheet_cut.png")
    crops = [drop_specks(c) for c in split_sheet(sheet)]
    if not consistent_crops(crops):
        crops = [drop_specks(c, 0.005) for c in split_sheet_panels(sheet)]
        print(f"   {name}: panel split (consistent: {consistent_crops(crops)})")
    return [fit_cell(c, box) for c in crops]


def opaque_mean(cells: list[np.ndarray]) -> np.ndarray:
    """Mean opaque linear colour of sRGB RGBA cells."""
    return np.concatenate(
        [srgb_to_linear(c[..., :3])[c[..., 3] > 0.5] for c in cells]
    ).mean(axis=0)


def impostor_atlases(
    species: list[dict] | None = None,
    gains: dict[str, list[float]] | None = None,
) -> tuple[np.ndarray, np.ndarray]:
    """GA3 impostor grid (albedo, normal): one row per catalogue species, FC2 framing.

    Rows 0-2 (oak, beech, fir) are matched to the FC2 row colours as before; the other species
    are framed like their ``frame`` FC2 row and brought to the luminance of their family (GA3 oak
    for broadleaves, GA3 fir for conifers) times their ``tone``.
    """
    if species is None:
        species = load_species()["species"]
    fc = _load(VEG / "campaign_impostors_albedo.png")
    rows = len(species)
    albedo = np.zeros((rows * CELL, VIEWS * CELL, 4), dtype=np.float32)
    normal = np.zeros_like(albedo)
    normal[...] = (0.5, 0.5, 1.0, 1.0)
    weights = np.array([0.2126, 0.7152, 0.0722])
    family_lum: dict[str, float] = {}
    for row, entry in enumerate(species):
        name = entry["id"]
        if row < len(ESSENCES):
            box, colour = fc_row_reference(fc, row)
            lum = float(colour @ weights)
            if entry.get("dn_id"):
                # Lot DN-FORET: the generated model keeps its own hue, only the brightness of
                # the FC2 row so that it sits with the other species.
                cells = match_luminance(
                    foliage_chroma(_species_cells(name, box), name in CONIFER_ROWS), lum
                )
            else:
                cells = match_mean(_species_cells(name, box), colour)
            family_lum["conifer" if name == "fir" else name] = lum
        else:
            box, _ = fc_row_reference(fc, ESSENCES.index(entry["frame"]))
            family = "conifer" if entry["mesh"] == "conifer" else "oak"
            target = family_lum[family] * float(entry.get("tone", 1.0))
            raw = _species_cells(name, box)
            if entry.get("dn_id"):
                raw = foliage_chroma(raw, entry["mesh"] == "conifer")
            cells = match_luminance(raw, target)
        if gains is not None and entry.get("dn_id"):
            # Colour factor of the baked impostor: the near mesh (same texture) applies it too.
            before = opaque_mean(_species_cells(name, box))
            gains[name] = [
                round(float(g), 4) for g in opaque_mean(cells) / np.maximum(before, 1e-4)
            ]
        for view, cell in enumerate(cells):
            y0, x0 = row * CELL, view * CELL
            albedo[y0 : y0 + CELL, x0 : x0 + CELL] = cell
            lum = srgb_to_linear(cell[..., :3]) @ weights
            normal[y0 : y0 + CELL, x0 : x0 + CELL] = crown_normals(cell[..., 3], lum)
        cov = np.mean([(c[..., 3] > 0.5).mean() for c in cells])
        print(f"{row:2d} {name}: box {np.round(box, 1)}, coverage {cov:.3f}")
    return albedo, normal


def sheet_step(name: str, glb: str) -> None:
    """Lot DN nature: 8 Blender views of ``glb`` -> ``RAW/<name>_sheet_cut.png`` (4 x 2 grid)."""
    from PIL import Image

    views_dir = RAW / f"{name}_views"
    subprocess.run(
        [
            "blender",
            "-b",
            "--factory-startup",
            "--python",
            str(REPO / "tools/blender_scripts/dn_tree_views.py"),
            "--",
            glb,
            str(views_dir),
            "768",
        ],
        check=True,
    )
    first = Image.open(views_dir / "view_0.png")
    width, height = first.size
    sheet = Image.new("RGBA", (SHEET_COLS * width, SHEET_ROWS * height), (0, 0, 0, 0))
    for view in range(VIEWS):
        panel = Image.open(views_dir / f"view_{view}.png").convert("RGBA")
        sheet.paste(panel, ((view % SHEET_COLS) * width, (view // SHEET_COLS) * height))
    sheet.save(RAW / f"{name}_sheet_cut.png")
    print(f"OK sheet {name}")


def dn_glb(dn_id: str) -> Path | None:
    """Ingested lod0 glb of a DN tree (``CENT_ANS_MODELS_DIR``, the repo, then the raw TRELLIS glb)."""
    import os

    roots = [
        Path(os.environ.get("CENT_ANS_MODELS_DIR", REPO / "game/assets/models")),
        Path.home() / "dev/game_project/game/assets/models",
    ]
    for root in roots:
        path = root / "dn/vegetation" / f"{dn_id}_lod0.glb"
        if path.exists():
            return path
    raw = sorted((Path.home() / "dev/cent-ans-raw/dn" / dn_id / "3d").glob("*.glb"))
    return raw[0] if raw else None


def sheets_step(only: list[str]) -> None:
    """Lot DN-FORET: bake the 8-view sheet of every species that names a ``dn_id``.

    The previous generated sheet is kept once as ``<name>_sheet_cut_fal.png`` (never overwritten).
    """
    for entry in load_species()["species"]:
        name, dn_id = entry["id"], entry.get("dn_id")
        if not dn_id or (only and name not in only):
            continue
        glb = dn_glb(dn_id)
        if glb is None:
            print(f"MISSING glb for {name} ({dn_id}): sheet kept")
            continue
        old = RAW / f"{name}_sheet_cut.png"
        backup = RAW / f"{name}_sheet_cut_fal.png"
        if old.exists() and not backup.exists():
            shutil.copy2(old, backup)
        sheet_step(name, str(glb))


def board_step() -> None:
    """Local contact sheet of the atlas (one row per species, labelled), for visual review."""
    from PIL import Image, ImageDraw

    species = load_species()["species"]
    atlas = Image.open(OUT_TEX / "ga3_impostors_albedo.png").convert("RGBA")
    cell = CELL // 2
    views = 4  # 0, 90, 180, 270 degrees
    per_col = (len(species) + 1) // 2
    label_w = 150
    width = 2 * (label_w + views * cell)
    sheet = Image.new("RGB", (width, per_col * cell), (128, 128, 128))
    draw = ImageDraw.Draw(sheet)
    for row, entry in enumerate(species):
        col, r = divmod(row, per_col)
        x0 = col * (label_w + views * cell)
        y0 = r * cell
        draw.text((x0 + 6, y0 + cell // 2 - 6), f"{row} {entry['id']}", fill=(0, 0, 0))
        for k in range(views):
            src = atlas.crop(
                (k * 2 * CELL, row * CELL, (k * 2 + 1) * CELL, (row + 1) * CELL)
            ).resize((cell, cell), Image.LANCZOS)
            sheet.paste(src, (x0 + label_w + k * cell, y0), src)
    BOARD.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(BOARD, quality=88)
    print("OK board", BOARD)


def leaf_cards() -> np.ndarray:
    """FC5 leaf-card atlas with the S5 oak cluster in the broadleaf half (fir half kept)."""
    from PIL import Image

    current = _load(VEG / "campaign_leaf_cards.png")
    image = Image.open(S5 / "oak_leaves_cut.png").convert("RGBA")
    image = image.crop(
        image.getchannel("A").point(lambda a: 255 if a > 128 else 0).getbbox()
    )
    side = max(image.size)
    square = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    square.paste(image, ((side - image.size[0]) // 2, (side - image.size[1]) // 2))
    arr = (
        np.asarray(square.resize((512, 512), Image.LANCZOS)).astype(np.float32) / 255.0
    )
    lin = srgb_to_linear(arr[..., :3])
    opaque = arr[..., 3] > 0.5
    leaves = np.concatenate(
        [
            linear_to_srgb(bleed(lin / lin[opaque].mean(axis=0) * 0.5, opaque)),
            arr[..., 3:4],
        ],
        axis=-1,
    )
    return np.concatenate([leaves, current[:, 512:]], axis=1)


def grass_tuft(dilate: int = 1) -> np.ndarray:
    """512 x 256 grass tuft, base at the bottom, alpha dilated so thin blades survive the cut."""
    from PIL import Image
    from scipy import ndimage

    image = Image.open(RAW / "grass_cut.png").convert("RGBA")
    image = image.crop(
        image.getchannel("A").point(lambda a: 255 if a > 96 else 0).getbbox()
    )
    w, h = image.size
    width = max(w, h * 2)
    canvas = Image.new("RGBA", (width, width // 2), (0, 0, 0, 0))
    scale = min(width / w, (width // 2) / h)
    image = image.resize((round(w * scale), round(h * scale)), Image.LANCZOS)
    canvas.paste(image, ((width - image.size[0]) // 2, width // 2 - image.size[1]))
    arr = (
        np.asarray(canvas.resize((512, 256), Image.LANCZOS)).astype(np.float32) / 255.0
    )
    alpha = arr[..., 3]
    if dilate > 0:
        alpha = np.maximum(
            alpha, ndimage.grey_dilation(alpha, size=(1, 2 * dilate + 1)) * 0.9
        )
    arr[..., 3] = alpha
    arr[..., :3] = bleed(arr[..., :3], arr[..., 3] > 0.5)
    return arr


IMPORT_PARAMS = """[params]

compress/mode=2
compress/high_quality=true
compress/lossy_quality=0.7
compress/uastc_level=0
compress/rdo_quality_loss=0.0
compress/hdr_compression=1
compress/normal_map=2
compress/channel_pack=0
mipmaps/generate=true
mipmaps/limit=-1
roughness/mode=0
roughness/src_normal=""
process/channel_remap/red=0
process/channel_remap/green=1
process/channel_remap/blue=2
process/channel_remap/alpha=3
process/fix_alpha_border={border}
process/premult_alpha=false
process/normal_map_invert_y=false
process/hdr_as_srgb=false
process/hdr_clamp_exposure=false
process/size_limit=0
detect_3d/compress_to=0
"""


def _write_import(png: Path, fix_border: bool) -> None:
    """Godot import settings of the FC textures (VRAM, mipmaps), filled in by ``--import``."""
    target = png.with_suffix(".png.import")
    if target.exists():
        return
    body = '[remap]\n\nimporter="texture"\ntype="CompressedTexture2D"\n\n'
    target.write_text(body + IMPORT_PARAMS.format(border=str(fix_border).lower()))


def atlas_step() -> None:
    """Write the four GA3 textures (and their import settings) and the species table."""
    species_step()
    gains: dict[str, list[float]] = {}
    albedo, normal = impostor_atlases(gains=gains)
    (REPO / "data/art/tree_model_gains.json").write_text(
        json.dumps(
            {
                "description": "Facteur de couleur (RVB linéaire) appliqué à la texture du glb DN de chaque essence pour atteindre la luminance de son imposteur (ga3_vegetation_l2.py atlas, lot DN-FORET, ADR 0213). Lu par DnTreeModels.",
                "gains": gains,
            },
            indent=1,
            ensure_ascii=False,
        )
        + "\n",
        encoding="utf-8",
    )
    outputs = {
        "ga3_impostors_albedo.png": (albedo, False),
        "ga3_impostors_normal.png": (normal, False),
        "ga3_leaf_cards.png": (leaf_cards(), True),
        "ga3_grass_tuft.png": (grass_tuft(), True),
    }
    for name, (arr, border) in outputs.items():
        _save(arr, OUT_TEX / name)
        _write_import(OUT_TEX / name, border)
    tuft = outputs["ga3_grass_tuft.png"][0]
    old = _load(VEG / "campaign_grass_tuft.png")
    print(
        f"grass coverage {(tuft[..., 3] > 0.5).mean():.3f} (FC5 {(old[..., 3] > 0.5).mean():.3f})"
    )
    print("OK atlas")


# ---------------------------------------------------------------- rocks step
def rocks_step() -> None:
    """Three TRELLIS rocks through ``ga3_cleanup.py``: 120 / 60 / 18 triangles, 1 m, 256 px.

    The raw TRELLIS albedos are dark (linear mean 0.05-0.09, first in-game capture: black blobs
    on the pale Auvergne ground); ``--exposure`` lifts each one to ~0.17, the terrain rock tint
    (``terrain.gdshader`` ``tint_rock`` 0.19 / 0.17 / 0.15).
    """
    blender = (
        shutil.which("blender") or "/Applications/Blender.app/Contents/MacOS/Blender"
    )
    sources = {
        "ga3_rock_a": (S5 / "rock_trellis.glb", 3.4),
        "ga3_rock_b": (RAW / "rock_b_trellis_s7.glb", 2.0),
        "ga3_rock_c": (RAW / "rock_c_trellis.glb", 2.1),
    }
    for name, (source, exposure) in sources.items():
        cmd = [
            blender, "-b", "--factory-startup", "--python", str(REPO / "tools/blender_scripts/ga3_cleanup.py"), "--",
            str(source), str(OUT_MODELS), name, "--length", "1.0", "--lod0", "120", "--tex", "256", "--normal", "off",
            "--exposure", str(exposure),
        ]  # fmt: skip
        result = subprocess.run(cmd, capture_output=True, text=True, check=False)
        lines = [
            line
            for line in result.stdout.splitlines()
            if "tri" in line.lower() or "error" in line.lower() or "grade" in line
        ]
        print(name, result.returncode, *lines[-6:], sep="\n  ")
    print("OK rocks")


if __name__ == "__main__":
    step = sys.argv[1] if len(sys.argv) > 1 else ""
    if step == "fal":
        fal_step(sys.argv[2:])
    elif step == "atlas":
        atlas_step()
    elif step == "rocks":
        rocks_step()
    elif step == "species":
        species_step()
    elif step == "sheets":
        sheets_step(sys.argv[2:])
    elif step == "board":
        board_step()
    elif step == "sheet" and len(sys.argv) == 4:
        sheet_step(sys.argv[2], sys.argv[3])
    else:
        sys.exit(
            "usage: ga3_vegetation_l2.py fal|atlas|rocks|species|board|sheet <species> <glb>"
        )
