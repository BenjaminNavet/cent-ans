"""Portolan ornaments of the campaign parchment view, cut out of public-domain charts (lot FA6).

The far view of the campaign map (camera above 1200) is drawn as an illuminated portolan. Its sea
ornaments (compass rose, ships, sea monsters) used to be drawn by code; this tool cuts the real
ones out of scans of period charts (Catalan Atlas, 1375) and writes them as transparent PNGs:

- the vellum is removed (`key`): the local paper colour is estimated, a pixel's opacity is its
  colour distance to that paper, and its colour is un-mixed from the paper so that only the ink
  and the paint remain;
- painted figures keep their opaque body (a `mask` without `key`: polygon with holes);
- the blue wave pattern of the Atlas seas can be rejected (`reject_blue`), except where blue is
  painted on purpose (`reject_except`).

Every crop, mask and threshold lives in `data/map/parchment_ornaments.json` (no coordinate in this
file). Raw scans stay outside the repository; missing ones are downloaded from Wikimedia Commons.

Usage: uv run --project tools python -m cent_ans_tools.parchment_ornaments [raw_dir] [output_dir]
    [--preview=<png>]  (contact sheet over the parchment colours, to judge the cut-outs)
"""

from __future__ import annotations

import json
import sys
import urllib.parse
import urllib.request
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[2]
SETTINGS = ROOT / "data" / "map" / "parchment_ornaments.json"
DEFAULT_RAW = Path.home() / "dev" / "cent-ans-raw" / "fa" / "campaign"
USER_AGENT = "cent-ans-game/1.0 (parchment ornaments; public-domain scans)"

Image.MAX_IMAGE_PIXELS = None


def load_settings(path: Path = SETTINGS) -> dict:
    """The ornament catalogue (sources, cut-outs, display hints)."""
    return json.loads(path.read_text(encoding="utf-8"))


def source_image(raw_dir: Path, source: dict) -> Image.Image:
    """A raw scan as RGB (downloaded from its `download_url` if missing)."""
    path = raw_dir / source["file"]
    if not path.is_file():
        path.parent.mkdir(parents=True, exist_ok=True)
        request = urllib.request.Request(
            source["download_url"], headers={"User-Agent": USER_AGENT}
        )
        with urllib.request.urlopen(request, timeout=600) as response:
            path.write_bytes(response.read())
    return Image.open(path).convert("RGB")


def smoothstep(low: float, high: float, value: np.ndarray) -> np.ndarray:
    """Hermite step between `low` and `high`."""
    t = np.clip((value - low) / max(high - low, 1e-6), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def shape_mask(
    shape: dict, origin: tuple[int, int], size: tuple[int, int]
) -> np.ndarray:
    """Coverage in [0, 1] of a `disk` or a `polygon` (with holes) given in source pixels."""
    width, height = size
    canvas = Image.new("L", (width, height), 0)
    draw = ImageDraw.Draw(canvas)
    if shape["shape"] == "polygon":
        points = [(x - origin[0], y - origin[1]) for x, y in shape["points"]]
        draw.polygon(points, fill=255)
        for hole in shape.get("holes", []):
            draw.polygon([(x - origin[0], y - origin[1]) for x, y in hole], fill=0)
    else:
        cx, cy = shape["center"][0] - origin[0], shape["center"][1] - origin[1]
        radius = shape["radius"]
        draw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), fill=255)
    feather = float(shape.get("feather", 0.0))
    if feather > 0.0:
        # Erode by the feather first so that the soft edge stays inside the shape.
        canvas = canvas.filter(ImageFilter.MinFilter(2 * int(feather) + 1))
        canvas = canvas.filter(ImageFilter.GaussianBlur(feather))
    return np.asarray(canvas, dtype=np.float32) / 255.0


def paper_estimate(rgb: np.ndarray, radius: int, percentile: float) -> np.ndarray:
    """Local vellum colour: a high percentile of each channel over a wide window, smoothed."""
    step = max(radius // 8, 1)
    small = rgb[::step, ::step]
    window = max(2 * radius // step + 1, 3)
    paper = np.stack(
        [
            ndimage.percentile_filter(small[..., c], percentile, size=window)
            for c in range(3)
        ],
        axis=-1,
    )
    paper = ndimage.gaussian_filter(paper, sigma=(window / 3.0, window / 3.0, 0.0))
    zoom = (rgb.shape[0] / paper.shape[0], rgb.shape[1] / paper.shape[1], 1.0)
    return ndimage.zoom(paper, zoom, order=1)[: rgb.shape[0], : rgb.shape[1]]


def ink_key(
    rgb: np.ndarray, key: dict, origin: tuple[int, int]
) -> tuple[np.ndarray, np.ndarray]:
    """Opacity and un-mixed colour of what is drawn on the vellum."""
    paper = paper_estimate(
        rgb, int(key.get("paper_radius", 40)), float(key.get("percentile", 80.0))
    )
    # Only what is darker than the paper on some channel: highlights of the skin are not ink.
    difference = np.sqrt((np.minimum(rgb - paper, 0.0) ** 2).sum(axis=-1))
    alpha = smoothstep(float(key["low"]), float(key["high"]), difference)
    if "reject_blue" in key:
        # Blue wave pattern of the seas: bluer than the paper it is painted on.
        low, high = key["reject_blue"]
        blueness = (rgb[..., 2] - rgb[..., :2].mean(axis=-1)) - (
            paper[..., 2] - paper[..., :2].mean(axis=-1)
        )
        reject = smoothstep(float(low), float(high), blueness)
        if "reject_except" in key:  # painted blue to keep (points of the rose)
            size = (rgb.shape[1], rgb.shape[0])
            reject = reject * (1.0 - shape_mask(key["reject_except"], origin, size))
        alpha = alpha * (1.0 - reject)
    alpha = np.clip(alpha * float(key.get("ink_gain", 1.0)), 0.0, 1.0)
    safe = np.maximum(alpha, 0.08)[..., None]
    colour = np.clip(paper + (rgb - paper) / safe, 0.0, 1.0)
    return alpha, colour


def cut_out(image: Image.Image, ornament: dict) -> Image.Image:
    """One ornament as RGBA, scaled so that its longest side is `size`."""
    x0, y0, x1, y1 = ornament["crop"]
    crop = image.crop((x0, y0, x1, y1))
    rgb = np.asarray(crop, dtype=np.float32) / 255.0
    size = crop.size
    alpha = np.ones(rgb.shape[:2], dtype=np.float32)
    colour = rgb
    if "key" in ornament:
        alpha, colour = ink_key(rgb, ornament["key"], (x0, y0))
    if "mask" in ornament:
        alpha = alpha * shape_mask(ornament["mask"], (x0, y0), size)
    tone = np.asarray(ornament.get("tone", [1.0, 1.0, 1.0]), dtype=np.float32)
    saturation = float(ornament.get("saturation", 1.0))
    luma = colour @ np.array([0.3, 0.59, 0.11], dtype=np.float32)
    colour = luma[..., None] + (colour - luma[..., None]) * saturation
    contrast = float(ornament.get("contrast", 1.0))
    colour = np.clip(((colour - 0.5) * contrast + 0.5) * tone, 0.0, 1.0)
    alpha = np.clip(alpha * float(ornament.get("opacity", 1.0)), 0.0, 1.0)
    rgba = np.concatenate([colour, alpha[..., None]], axis=-1)
    out = Image.fromarray((rgba * 255.0 + 0.5).astype(np.uint8), "RGBA")
    scale = float(ornament["size"]) / max(out.size)
    target = (max(round(out.width * scale), 1), max(round(out.height * scale), 1))
    # Premultiplied resize: transparent pixels must not bleed their colour into the edges.
    premultiplied = np.asarray(out, dtype=np.float32) / 255.0
    premultiplied[..., :3] *= premultiplied[..., 3:4]
    resized = np.stack(
        [
            np.asarray(
                Image.fromarray(premultiplied[..., c]).resize(target, Image.LANCZOS)
            )
            for c in range(4)
        ],
        axis=-1,
    )
    resized = np.clip(resized, 0.0, 1.0)
    resized[..., :3] /= np.maximum(resized[..., 3:4], 1e-3)
    resized = np.clip(resized, 0.0, 1.0)
    return Image.fromarray((resized * 255.0 + 0.5).astype(np.uint8), "RGBA")


def source_notes(settings: dict) -> str:
    """`SOURCE.md` of the output folder: one entry per produced file."""
    lines = [
        "# Ornements de portulan de la vue parchemin (lot FA6)",
        "",
        "Fichiers produits par `tools/cent_ans_tools/parchment_ornaments.py` depuis des",
        "numérisations d'œuvres du domaine public (découpes : `data/map/parchment_ornaments.json`).",
        "Les numérisations brutes ne sont pas versionnées.",
        "",
    ]
    entries = [(o["file"], o["source"], o["note"]) for o in settings["ornaments"]]
    for file, source_id, note in entries:
        source = settings["sources"][source_id]
        lines += [
            f"## `{file}`",
            "",
            f"- Sujet : {note}",
            f"- Œuvre : {source['work']}",
            f"- Auteur : {source['author']} ; date : {source['date']}",
            f"- Institution : {source['institution']}",
            f"- Fichier source : [{source['title']}]({source['page_url']})",
            f"- Licence : {source['licence']}",
            "",
        ]
    return "\n".join(lines)


def preview(images: dict[str, Image.Image], colours: list[list[int]]) -> Image.Image:
    """Contact sheet of the cut-outs over the given background colours."""
    cell = 384
    sheet = Image.new("RGB", (cell * len(images), cell * len(colours)))
    for row, colour in enumerate(colours):
        for column, image in enumerate(images.values()):
            tile = Image.new("RGBA", (cell, cell), (*colour, 255))
            thumb = image.copy()
            thumb.thumbnail((cell - 16, cell - 16), Image.LANCZOS)
            offset = ((cell - thumb.width) // 2, (cell - thumb.height) // 2)
            tile.alpha_composite(thumb, offset)
            sheet.paste(tile.convert("RGB"), (column * cell, row * cell))
    return sheet


def commons_licence(title: str) -> dict:
    """Licence fields of a Wikimedia Commons file (used by hand to check `sources`)."""
    query = urllib.parse.urlencode(
        {
            "action": "query",
            "prop": "imageinfo",
            "iiprop": "extmetadata",
            "titles": title,
            "format": "json",
        }
    )
    request = urllib.request.Request(
        f"https://commons.wikimedia.org/w/api.php?{query}",
        headers={"User-Agent": USER_AGENT},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        pages = json.loads(response.read())["query"]["pages"]
    metadata = next(iter(pages.values()))["imageinfo"][0]["extmetadata"]
    return {
        key: metadata.get(key, {}).get("value", "")
        for key in ("LicenseShortName", "Copyrighted", "Artist", "DateTimeOriginal")
    }


def build(raw_dir: Path, out_dir: Path, preview_path: Path | None = None) -> None:
    """Cuts every ornament of the catalogue and writes `SOURCE.md`."""
    settings = load_settings()
    out_dir.mkdir(parents=True, exist_ok=True)
    scans: dict[str, Image.Image] = {}
    produced: dict[str, Image.Image] = {}

    def scan(source_id: str) -> Image.Image:
        if source_id not in scans:
            scans[source_id] = source_image(raw_dir, settings["sources"][source_id])
        return scans[source_id]

    for ornament in settings["ornaments"]:
        image = cut_out(scan(ornament["source"]), ornament)
        image.save(out_dir / ornament["file"], optimize=True)
        produced[ornament["file"]] = image
        print(f"wrote {ornament['file']} {image.size}")
    (out_dir / "SOURCE.md").write_text(source_notes(settings), encoding="utf-8")
    if preview_path is not None:
        preview(produced, settings["preview_colours"]).save(preview_path)
        print(f"wrote {preview_path}")


def main(argv: list[str]) -> None:
    """Command line entry point."""
    options = [arg for arg in argv if arg.startswith("--")]
    paths = [arg for arg in argv if not arg.startswith("--")]
    settings = load_settings()
    raw_dir = Path(paths[0]) if paths else DEFAULT_RAW
    out_dir = Path(paths[1]) if len(paths) > 1 else ROOT / settings["output_dir"]
    preview_path = None
    for option in options:
        if option.startswith("--preview="):
            preview_path = Path(option.removeprefix("--preview="))
        elif option == "--licences":
            for source in settings["sources"].values():
                print(source["title"], commons_licence(source["title"]))
            return
    build(raw_dir, out_dir, preview_path)


if __name__ == "__main__":
    main(sys.argv[1:])
