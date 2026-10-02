"""Construit l'atlas des touffes d'herbe des batailles à partir de vrais brins (lot FA7).

Usage : uv run --with pillow --with numpy --with scipy python build_fa_grass.py [dossier brut]

Lit `data/art/battle_grass.json` : les atlas de brins et d'herbes photographiés ambientCG
(CC0 1.0) de `sources`, téléchargés dans le dossier brut (`~/dev/cent-ans-raw/fa/ambientcg` par
défaut, hors dépôt) s'ils n'y sont pas. Chaque brin est détouré par la carte d'opacité, ramené à
une luminance commune, mis à l'échelle en mètres, penché et courbé, puis posé sur l'un des pieds
de la touffe. Chaque touffe de `variants` occupe une case de l'atlas.

La carte finie est ramenée (en linéaire) à une couleur moyenne neutre de luminance
`render.tex_lum` : elle ne porte que les écarts de teinte et de clarté entre brins (brin vert,
épi paille, feuille jaunie) ; la couleur vient du sol sous la touffe (`battle_grass.gdshader`).

Produit, à côté de ce script : `grass_tufts.png`.
"""

import io
import json
import math
import random
import sys
import urllib.request
import zipfile
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

HERE = Path(__file__).parent
CATALOGUE = HERE.parents[3] / "data" / "art" / "battle_grass.json"
DEFAULT_RAW = Path.home() / "dev" / "cent-ans-raw" / "fa" / "ambientcg"
OUTPUT = "grass_tufts.png"
LUMA = np.array([0.3, 0.59, 0.11], dtype=np.float32)
OPACITY_CUT = 60


def _atlas_maps(raw: Path, asset: str, source_url: str) -> tuple[Path, Path]:
    """Chemins des cartes couleur et opacité de `asset` (téléchargées au besoin)."""
    folder = raw / asset
    if not list(folder.glob("*_Color.jpg")):
        folder.mkdir(parents=True, exist_ok=True)
        request = urllib.request.Request(
            source_url.format(asset=asset), headers={"User-Agent": "cent-ans-game/1.0"}
        )
        with urllib.request.urlopen(request, timeout=180) as response:
            zipfile.ZipFile(io.BytesIO(response.read())).extractall(folder)
    return next(folder.glob("*_Color.jpg")), next(folder.glob("*_Opacity.jpg"))


def _to_linear(srgb: np.ndarray) -> np.ndarray:
    """Couleurs sRGB 0-255 vers linéaire 0-1."""
    c = srgb.astype(np.float32) / 255.0
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def _to_srgb(linear: np.ndarray) -> np.ndarray:
    """Couleurs linéaires 0-1 vers sRGB 0-255."""
    c = np.clip(linear, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055) * 255.0


def cut_pieces(raw: Path, source: dict, source_url: str) -> list[np.ndarray]:
    """Brins détourés d'un atlas : RGBA flottant (couleur prémultipliée), pied en bas.

    Chaque brin est ramené à une luminance moyenne de 1 (fois `gain`) : aucune source n'est plus
    sombre qu'une autre, seule sa teinte la distingue.
    """
    color_path, opacity_path = _atlas_maps(raw, source["asset"], source_url)
    color = np.array(Image.open(color_path).convert("RGB"))
    opacity = np.array(Image.open(opacity_path).convert("L"))
    labels = ndimage.label(opacity > OPACITY_CUT)[0]
    pieces = []
    for index, box in enumerate(ndimage.find_objects(labels)):
        own = labels[box] == index + 1
        if own.sum() < int(source.get("min_pixels", 3000)):
            continue
        alpha = np.where(own, opacity[box], 0).astype(np.float32) / 255.0
        rgb = color[box].astype(np.float32)
        if source["axis"] == "horizontal":
            alpha = np.rot90(alpha)
            rgb = np.rot90(rgb)
        height, width = alpha.shape
        if width / height > float(source.get("max_aspect", 10.0)):
            continue
        pieces.append((rgb, alpha))
    weights = np.concatenate([a[a > 0.5] for _, a in pieces])
    pixels = np.concatenate([rgb[a > 0.5] for rgb, a in pieces])
    mean_luma = float((pixels @ LUMA).sum() / len(weights))
    gain = float(source.get("gain", 1.0)) / mean_luma
    tone = np.array(source.get("tone", [1.0, 1.0, 1.0]), dtype=np.float32)
    return [
        np.dstack([rgb * gain * tone * alpha[..., None], alpha]) for rgb, alpha in pieces
    ]


def bend(piece: np.ndarray, length: int, width_scale: float, lean: float, curve: float):
    """Brin mis à `length` px de haut, penché (`lean`, tangente) et courbé (`curve`).

    Rend (image RGBA prémultipliée, abscisse du pied dans cette image).
    """
    src_h, src_w = piece.shape[:2]
    width = max(2, round(src_w * length / src_h * width_scale))
    scaled = np.dstack(
        [
            np.array(
                Image.fromarray(piece[..., k]).resize(
                    (width, length), Image.Resampling.LANCZOS
                )
            )
            for k in range(4)
        ]
    ).clip(0.0, None)
    t = 1.0 - (np.arange(length) + 0.5) / length  # 0 au pied, 1 à la pointe
    shift = length * (lean * t + curve * t * t)
    low = math.floor(float(shift.min()))
    out_w = width + math.ceil(float(shift.max())) - low + 2
    rows = np.repeat(np.arange(length, dtype=np.float32)[:, None], out_w, axis=1)
    cols = np.arange(out_w, dtype=np.float32)[None, :] - (shift[:, None] - low)
    bent = np.dstack(
        [
            ndimage.map_coordinates(
                scaled[..., k], [rows, cols], order=1, mode="constant", cval=0.0
            )
            for k in range(4)
        ]
    )
    return bent, -low + width * 0.5


def _paste(canvas: np.ndarray, stamp: np.ndarray, left: int, top: int) -> None:
    """Pose `stamp` (prémultiplié) sur `canvas` (prémultiplié), rogné aux bords."""
    h, w = stamp.shape[:2]
    x0, y0 = max(left, 0), max(top, 0)
    x1, y1 = min(left + w, canvas.shape[1]), min(top + h, canvas.shape[0])
    if x1 <= x0 or y1 <= y0:
        return
    part = stamp[y0 - top : y1 - top, x0 - left : x1 - left]
    region = canvas[y0:y1, x0:x1]
    region[...] = part + region * (1.0 - part[..., 3:4])


def _flower(rng: random.Random, flower: dict, ppm: float) -> np.ndarray:
    """Petite fleur dessinée (capitule de quelques pixels) au bout d'une tige fine."""
    radius = float(flower["radius_m"]) * ppm
    stem = round(rng.uniform(*flower["height_m"]) * ppm)
    size = math.ceil(radius * 2 + 4)
    image = Image.new("RGBA", (size, stem + size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    cx = size / 2
    stem_rgb = tuple(int(v) for v in flower["stem_rgb"])
    draw.line(
        (cx, size / 2, cx, stem + size),
        fill=(*stem_rgb, 255),
        width=max(2, round(radius * 0.22)),
    )
    petal = tuple(int(v) for v in flower["petal_rgb"])
    heart = tuple(int(v) for v in flower["heart_rgb"])
    petals = int(flower["petals"])
    for k in range(petals):
        angle = k / petals * math.tau + rng.uniform(-0.1, 0.1)
        px = cx + math.cos(angle) * radius * 0.6
        py = size / 2 + math.sin(angle) * radius * 0.45
        r = radius * rng.uniform(0.36, 0.48)
        shade = rng.uniform(0.82, 1.0)
        draw.ellipse(
            (px - r, py - r * 0.75, px + r, py + r * 0.75),
            fill=(*(int(v * shade) for v in petal), 255),
        )
    r = radius * float(flower["heart"])
    draw.ellipse((cx - r, size / 2 - r * 0.75, cx + r, size / 2 + r * 0.75), fill=(*heart, 255))
    arr = np.array(image).astype(np.float32)
    alpha = arr[..., 3:4] / 255.0
    return np.dstack([arr[..., :3] / float(flower["luma_ref"]) * alpha, alpha])


def tuft(variant: dict, pieces: dict[str, list[np.ndarray]], catalogue: dict, seed: str):
    """Compose une touffe : RGBA prémultiplié à l'échelle de travail (`pixels_per_metre`)."""
    atlas = catalogue["atlas"]
    ppm = float(atlas["pixels_per_metre"])
    rng = random.Random(seed)
    width = round(float(catalogue["render"]["card"]["width_m"]) * ppm)
    height = round(float(variant["height_m"]) * ppm)
    canvas = np.zeros((height, width, 4), dtype=np.float32)
    span = float(variant["foot_span"])
    feet = [
        width * (0.5 + span * ((k + 0.5) / int(variant["feet"]) - 0.5) * 2.0 * 0.5)
        + rng.uniform(-0.03, 0.03) * width
        for k in range(int(variant["feet"]))
    ]
    stamps = []
    for layer in variant["layers"]:
        for _ in range(int(layer["count"])):
            foot = rng.choice(feet)
            offset = rng.gauss(0.0, float(layer["foot_sigma_m"]) * ppm)
            length = round(min(rng.uniform(*layer["length_m"]) * ppm, height * 0.98))
            # Les brins s'écartent du pied (éventail modéré) et retombent dans le même sens.
            fan = offset / max(length, 1.0) * float(layer["fan"])
            lean = fan + rng.uniform(-1.0, 1.0) * float(layer["lean"])
            curve = (
                math.copysign(1.0, lean if rng.random() < 0.8 else -lean)
                * rng.uniform(0.0, 1.0)
                * float(layer["curve"])
            )
            # Pointe gardée dans la carte : sinon le brin est redressé.
            tip = foot + offset + (lean + curve) * length
            margin = width * 0.03
            if not margin < tip < width - margin:
                lean, curve = -lean * 0.3, -curve * 0.3
            tone = rng.uniform(*layer["tone"])
            stamps.append((length, layer, foot + offset, lean, curve, tone))
    # Les longs brins derrière (posés d'abord, un peu plus sombres), les courts devant.
    stamps.sort(key=lambda s: -s[0])
    for rank, (length, layer, base_x, lean, curve, tone) in enumerate(stamps):
        depth = rank / max(len(stamps) - 1, 1)
        shade = tone * (float(variant["back_shade"]) + (1.0 - float(variant["back_shade"])) * depth)
        piece = rng.choice(pieces[layer["source"]])
        if rng.random() < 0.5:
            piece = piece[:, ::-1]
        bent, foot_x = bend(piece, length, float(layer["width_scale"]), lean, curve)
        bent = bent.copy()
        bent[..., :3] *= shade
        _paste(canvas, bent, round(base_x - foot_x), height - length)
    for flower in variant.get("flowers", []):
        kind = catalogue["flowers"][flower["kind"]]
        for _ in range(int(flower["count"])):
            stamp = _flower(rng, kind, ppm)
            x = rng.uniform(0.2, 0.8) * width
            _paste(canvas, stamp, round(x - stamp.shape[1] / 2), height - stamp.shape[0])
    return canvas


def build(catalogue: dict, raw: Path) -> Image.Image:
    """Atlas complet, neutre en moyenne, de luminance linéaire moyenne `render.tex_lum`."""
    atlas = catalogue["atlas"]
    columns, rows = int(atlas["columns"]), int(atlas["rows"])
    cell_w, cell_h, pad = int(atlas["cell_width"]), int(atlas["cell_height"]), int(atlas["padding"])
    pieces = {
        name: cut_pieces(raw, source, catalogue["source_url"])
        for name, source in catalogue["sources"].items()
    }
    sheet = np.zeros((rows * cell_h, columns * cell_w, 4), dtype=np.float32)
    for index, variant in enumerate(catalogue["variants"]):
        canvas = tuft(variant, pieces, catalogue, f"fa7/{variant['name']}")
        inner = (cell_w - 2 * pad, cell_h - 2 * pad)
        small = np.dstack(
            [
                np.array(Image.fromarray(canvas[..., k]).resize(inner, Image.Resampling.LANCZOS))
                for k in range(4)
            ]
        ).clip(0.0, None)
        x = (index % columns) * cell_w + pad
        y = (index // columns) * cell_h + pad
        sheet[y : y + inner[1], x : x + inner[0]] = small
        # Marge du bas : le pied est prolongé (les mipmaps ne creusent pas la base de la touffe).
        sheet[y + inner[1] : y + inner[1] + pad, x : x + inner[0]] = small[-1:]
        coverage = float((small[..., 3] > 0.5).mean())
        print(f"{variant['name']} : {len(variant['layers'])} couches, couverture {coverage:.2f}")
    alpha = sheet[..., 3].clip(0.0, 1.0)
    rgb = sheet[..., :3] / np.maximum(alpha, 1e-4)[..., None]
    # Luminance relative (1 = brin moyen) vers linéaire, puis moyenne neutre de luminance cible.
    linear = _to_linear((rgb * float(atlas["work_grey"])).clip(0, 255))
    mean = (linear * alpha[..., None]).sum(axis=(0, 1)) / alpha.sum()
    target = float(catalogue["render"]["tex_lum"])
    pull = float(atlas["neutral_pull"])
    gain = (target / mean) ** pull * (target / float(mean @ LUMA)) ** (1.0 - pull)
    linear *= gain
    linear *= target / float(((linear @ LUMA) * alpha).sum() / alpha.sum())
    out = np.dstack([_to_srgb(linear), alpha * 255.0]).round().clip(0, 255).astype(np.uint8)
    # Couleur des brins étendue sous les pixels transparents (pas de franges aux mipmaps).
    nearest = ndimage.distance_transform_edt(
        out[..., 3] == 0, return_distances=False, return_indices=True
    )
    out[..., :3] = out[..., :3][nearest[0], nearest[1]]
    return Image.fromarray(out, "RGBA")


if __name__ == "__main__":
    raw_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_RAW
    document = json.loads(CATALOGUE.read_text(encoding="utf-8"))
    build(document, raw_dir).save(HERE / OUTPUT)
    print(f"{OUTPUT} écrit")
