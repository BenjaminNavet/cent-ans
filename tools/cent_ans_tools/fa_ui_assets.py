"""Interface ornaments cut from public-domain works (lot FA5).

Cuts illuminated initials, border ornaments, wax seals and materials out of open-access
photographs (Cleveland Museum of Art CC0, Metropolitan Museum Open Access, ambientCG and Poly
Haven CC0) and writes small derivatives into `game/assets/ui/fa/`:

- `initials/<id>.png`: an initial with its painted field (`framed`) or lifted off the parchment
  (`matte`), used by `Lettrine` when the first letter of a window title matches.
- `ornaments/<id>.png`: foliage sprays and bars with the parchment removed (soft alpha).
- `seals/<id>.png`: wax seals cut off the museum backdrop, with a soft drop shadow.
- `materials/<id>.png`: seamless leather, brass, wood and velvet tiles.
- `SOURCE.md`: object, institution, date, URL and licence of every file.

Every cut (source, box, size, matte settings) lives in `data/ui/fa_ui_assets.json`; nothing is
hard-coded here. The raw photographs stay outside the repository.

Usage: uv run --project tools python -m cent_ans_tools.fa_ui_assets [raw_dir] [output_dir]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[2]
CATALOGUE = ROOT / "data" / "ui" / "fa_ui_assets.json"
DEFAULT_OUT = ROOT / "game" / "assets" / "ui" / "fa"
DEFAULT_RAW = Path.home() / "dev" / "cent-ans-raw" / "fa" / "ui"
KINDS = ("initials", "ornaments", "seals", "materials")
LUMA = np.array([0.299, 0.587, 0.114], dtype=np.float32)


def load_catalogue(path: Path = CATALOGUE) -> dict:
    """The cut catalogue (sources and cuts of every kind)."""
    return json.loads(path.read_text(encoding="utf-8"))


def crop_source(raw_dir: Path, catalogue: dict, cut: dict) -> np.ndarray:
    """The boxed region of a cut's source photograph, float RGB in [0, 1], oriented."""
    source = catalogue["sources"][cut["source"]]
    with Image.open(raw_dir / source["file"]) as photograph:
        image = photograph.convert("RGB")
        if "box" in cut:
            image = image.crop(tuple(cut["box"]))
    region = np.asarray(image, dtype=np.float32) / 255.0
    return orient(region, cut)


def orient(region: np.ndarray, cut: dict) -> np.ndarray:
    """Applies the cut's quarter-turn rotation (counter-clockwise) and horizontal flip."""
    turns = int(cut.get("rotate", 0)) // 90
    if turns:
        region = np.rot90(region, turns)
    if cut.get("flip_h", False):
        region = region[:, ::-1]
    return np.ascontiguousarray(region)


def smoothstep(low: float, high: float, value: np.ndarray) -> np.ndarray:
    """Hermite ramp from 0 at `low` to 1 at `high`."""
    ramp = np.clip((value - low) / max(high - low, 1e-6), 0.0, 1.0)
    return ramp * ramp * (3.0 - 2.0 * ramp)


def local_background(rgb: np.ndarray, matte: dict) -> np.ndarray:
    """Per-pixel colour of the support (parchment, museum backdrop) behind the subject.

    The support is the light, broad part of the region: its reference colour is the median of the
    pixels above a luminance percentile, and its local shade (stains, lighting) is a normalised
    blur of the pixels close to that reference.
    """
    luminance = rgb @ LUMA
    light = luminance >= np.percentile(
        luminance, float(matte.get("background_percentile", 60))
    )
    reference = np.median(rgb[light], axis=0)
    close = (np.linalg.norm(rgb - reference, axis=2) < float(matte["low"])).astype(
        np.float32
    )
    sigma = float(matte.get("background_blur_px", 24))
    weight = ndimage.gaussian_filter(close, sigma)
    blurred = np.stack(
        [
            ndimage.gaussian_filter(rgb[..., channel] * close, sigma)
            for channel in range(3)
        ],
        axis=2,
    )
    known = weight > 0.05
    background = np.broadcast_to(reference, rgb.shape).copy()
    background[known] = blurred[known] / weight[known][:, None]
    return background


def matte_alpha(rgb: np.ndarray, background: np.ndarray, matte: dict) -> np.ndarray:
    """Soft coverage of the subject: colour distance to the support, cleaned and feathered."""
    distance = np.linalg.norm(rgb - background, axis=2)
    alpha = smoothstep(float(matte["low"]), float(matte["high"]), distance)
    solid = alpha > 0.5
    if "ellipse" in matte:
        centre_x, centre_y, radius_x, radius_y = (
            float(value) for value in matte["ellipse"]
        )
        rows, columns = np.mgrid[0 : rgb.shape[0], 0 : rgb.shape[1]]
        inside = ((columns - centre_x) / radius_x) ** 2 + (
            (rows - centre_y) / radius_y
        ) ** 2
        alpha = alpha * (1.0 - smoothstep(0.92, 1.0, inside))
        solid &= inside <= 1.0
    fill_holes = int(matte.get("fill_holes_px", 0))
    if fill_holes:
        # Highlights as pale as the support, enclosed by paint or wax, belong to the subject.
        holes = ndimage.binary_fill_holes(solid) & ~solid
        labels, count = ndimage.label(holes)
        if count:
            areas = ndimage.sum_labels(holes, labels, index=np.arange(1, count + 1))
            small = np.isin(labels, np.flatnonzero(areas <= fill_holes) + 1)
            alpha = np.where(small, 1.0, alpha)
            solid |= small
    min_blob = int(matte.get("min_blob_px", 0))
    if min_blob or matte.get("keep_largest", False):
        reach = ndimage.binary_dilation(solid, iterations=int(matte.get("join_px", 2)))
        labels, count = ndimage.label(reach)
        if count:
            areas = ndimage.sum_labels(solid, labels, index=np.arange(1, count + 1))
            if matte.get("keep_largest", False):
                kept = labels == (int(np.argmax(areas)) + 1)
            else:
                kept = np.isin(labels, np.flatnonzero(areas >= min_blob) + 1)
            alpha = alpha * kept
    feather = float(matte.get("feather_px", 0.0))
    if feather > 0.0:
        alpha = np.minimum(alpha, ndimage.gaussian_filter(alpha, feather) * 1.15)
    return np.clip(alpha, 0.0, 1.0).astype(np.float32)


def decontaminate(
    rgb: np.ndarray, alpha: np.ndarray, background: np.ndarray
) -> np.ndarray:
    """Removes the support's colour from half-covered edge pixels."""
    coverage = np.maximum(alpha, 0.25)[..., None]
    return np.clip((rgb - (1.0 - coverage) * background) / coverage, 0.0, 1.0)


def lift(region: np.ndarray, matte: dict) -> np.ndarray:
    """A region lifted off its support: float RGBA, straight alpha."""
    background = local_background(region, matte)
    alpha = matte_alpha(region, background, matte)
    return np.dstack([decontaminate(region, alpha, background), alpha])


def trim(rgba: np.ndarray, margin: int = 2) -> np.ndarray:
    """Crops transparent borders, keeping `margin` pixels."""
    rows = np.flatnonzero(rgba[..., 3].max(axis=1) > 0.02)
    columns = np.flatnonzero(rgba[..., 3].max(axis=0) > 0.02)
    if rows.size == 0 or columns.size == 0:
        return rgba
    top, bottom = max(rows[0] - margin, 0), min(rows[-1] + margin + 1, rgba.shape[0])
    left, right = (
        max(columns[0] - margin, 0),
        min(columns[-1] + margin + 1, rgba.shape[1]),
    )
    return rgba[top:bottom, left:right]


def fit(rgba: np.ndarray, size: int) -> np.ndarray:
    """Resizes so that the longest side is `size` (premultiplied Lanczos, never enlarges)."""
    height, width = rgba.shape[:2]
    scale = min(1.0, size / max(height, width))
    target = (max(1, round(width * scale)), max(1, round(height * scale)))
    premultiplied = rgba.copy()
    premultiplied[..., :3] *= premultiplied[..., 3:4]
    channels = [
        np.asarray(
            Image.fromarray(premultiplied[..., channel], mode="F").resize(
                target, Image.LANCZOS
            ),
            dtype=np.float32,
        )
        for channel in range(4)
    ]
    resized = np.clip(np.dstack(channels), 0.0, 1.0)
    coverage = np.maximum(resized[..., 3:4], 1e-4)
    resized[..., :3] = np.clip(resized[..., :3] / coverage, 0.0, 1.0)
    return resized


def soften_edges(rgba: np.ndarray, radius: float) -> np.ndarray:
    """Fades the alpha towards the borders of a framed cut over `radius` pixels."""
    if radius <= 0.0:
        return rgba
    height, width = rgba.shape[:2]
    rows = np.minimum(np.arange(height), np.arange(height)[::-1])[:, None]
    columns = np.minimum(np.arange(width), np.arange(width)[::-1])[None, :]
    edge = np.clip((np.minimum(rows, columns) + 0.5) / radius, 0.0, 1.0)
    softened = rgba.copy()
    softened[..., 3] *= edge
    return softened


def grade(rgb: np.ndarray, cut: dict) -> np.ndarray:
    """Tone settings of a cut: wax or material tint, saturation, gain."""
    graded = rgb
    if "tint" in cut:
        # Recolours by luminance: the relief of the wax stays, its hue becomes the tint's.
        tint = np.asarray(cut["tint"], dtype=np.float32) / 255.0
        luminance = (graded @ LUMA)[..., None]
        recoloured = np.clip(tint * luminance / max(float(tint @ LUMA), 1e-3), 0.0, 1.0)
        strength = float(cut.get("tint_strength", 1.0))
        graded = graded * (1.0 - strength) + recoloured * strength
    saturation = float(cut.get("saturation", 1.0))
    if saturation != 1.0:
        luminance = (graded @ LUMA)[..., None]
        graded = luminance + (graded - luminance) * saturation
    return np.clip(graded * float(cut.get("gain", 1.0)), 0.0, 1.0)


def drop_shadow(rgba: np.ndarray, shadow: dict) -> np.ndarray:
    """Pads the cut and bakes a soft shadow under it."""
    blur = float(shadow["blur_px"])
    offset_x, offset_y = (int(value) for value in shadow["offset_px"])
    pad = int(np.ceil(blur * 2.5)) + max(abs(offset_x), abs(offset_y))
    padded = np.pad(rgba, ((pad, pad), (pad, pad), (0, 0)))
    cast = ndimage.gaussian_filter(
        np.roll(padded[..., 3], (offset_y, offset_x), axis=(0, 1)), blur
    ) * float(shadow["opacity"])
    alpha = padded[..., 3]
    out_alpha = alpha + cast * (1.0 - alpha)
    colour = padded[..., :3] * (alpha / np.maximum(out_alpha, 1e-4))[..., None]
    return np.dstack([colour, out_alpha])


def cut_initial(region: np.ndarray, cut: dict) -> np.ndarray:
    """An initial: with its painted field (`framed`) or lifted off the parchment (`matte`)."""
    if cut["mode"] == "matte":
        return fit(trim(lift(region, cut["matte"])), int(cut["size"]))
    opaque = np.dstack([region, np.ones(region.shape[:2], dtype=np.float32)])
    return soften_edges(fit(opaque, int(cut["size"])), float(cut.get("edge_px", 1.0)))


def fade_ends(rgba: np.ndarray, width: int) -> np.ndarray:
    """Fades the left and right ends of a band cut straight through over `width` pixels."""
    if width <= 0:
        return rgba
    columns = np.arange(rgba.shape[1], dtype=np.float32)
    ramp = smoothstep(0.0, 1.0, np.minimum(columns, columns[::-1]) / float(width))
    faded = rgba.copy()
    faded[..., 3] *= ramp[None, :]
    return faded


def cut_ornament(region: np.ndarray, cut: dict) -> np.ndarray:
    """A border ornament lifted off the parchment."""
    ornament = fit(trim(lift(region, cut["matte"])), int(cut["size"]))
    return fade_ends(ornament, int(cut.get("fade_ends_px", 0)))


def cut_seal(region: np.ndarray, cut: dict) -> np.ndarray:
    """A wax seal off its backdrop, toned, with its drop shadow."""
    lifted = trim(lift(region, cut["matte"]))
    lifted[..., :3] = grade(lifted[..., :3], cut)
    seal = fit(lifted, int(cut["size"]))
    return drop_shadow(seal, cut["shadow"]) if "shadow" in cut else seal


def cut_material(region: np.ndarray, cut: dict) -> np.ndarray:
    """A seamless material tile (the sources tile already), toned."""
    size = int(cut["size"])
    image = Image.fromarray((region * 255.0 + 0.5).astype(np.uint8)).resize(
        (size, size), Image.LANCZOS
    )
    rgb = grade(np.asarray(image, dtype=np.float32) / 255.0, cut)
    return np.dstack([rgb, np.ones((size, size), dtype=np.float32)])


CUTTERS = {
    "initials": cut_initial,
    "ornaments": cut_ornament,
    "seals": cut_seal,
    "materials": cut_material,
}


def to_image(rgba: np.ndarray, opaque: bool = False) -> Image.Image:
    """Float RGBA to an 8-bit image (RGB when `opaque`)."""
    image = Image.fromarray(
        (np.clip(rgba, 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8), "RGBA"
    )
    return image.convert("RGB") if opaque else image


def sources_markdown(catalogue: dict) -> str:
    """`SOURCE.md`: provenance and licence of every generated file."""
    lines = [
        "# Sources des ornements d'interface (lot FA5)",
        "",
        "Fichiers produits par `tools/cent_ans_tools/fa_ui_assets.py` depuis le catalogue",
        "`data/ui/fa_ui_assets.json`. Ce sont des dérivés découpés et réduits ; les photographies",
        "d'origine ne sont pas dans le dépôt.",
        "",
        "| Fichier | Objet | Institution | Lieu, date | Licence | URL |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for kind in KINDS:
        for cut in catalogue.get(kind, []):
            source = catalogue["sources"][cut["source"]]
            lines.append(
                f"| `{kind}/{cut['id']}.png` | {source['object']} | {source['institution']} "
                f"| {source['origin']} | {source['licence']} | {source['url']} |"
            )
    return "\n".join(lines) + "\n"


def build(raw_dir: Path = DEFAULT_RAW, out_dir: Path = DEFAULT_OUT) -> list[Path]:
    """Cuts every catalogue entry and writes the PNG files and `SOURCE.md`."""
    catalogue = load_catalogue()
    written: list[Path] = []
    for kind in KINDS:
        for cut in catalogue.get(kind, []):
            rgba = CUTTERS[kind](crop_source(raw_dir, catalogue, cut), cut)
            path = out_dir / kind / f"{cut['id']}.png"
            path.parent.mkdir(parents=True, exist_ok=True)
            to_image(rgba, opaque=kind == "materials").save(path, optimize=True)
            written.append(path)
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "SOURCE.md").write_text(sources_markdown(catalogue), encoding="utf-8")
    return written


def main() -> None:
    """Command-line entry point: optional raw directory, then output directory."""
    raw_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_RAW
    out_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_OUT
    for path in build(raw_dir, out_dir):
        print(f"{path.relative_to(out_dir)} {path.stat().st_size // 1024} ko")


if __name__ == "__main__":
    main()
