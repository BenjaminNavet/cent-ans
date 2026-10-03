"""Construit les rameaux feuillus des arbres de bataille à partir de vraies feuilles (lot FA1).

Usage : uv run --with pillow --with numpy --with scipy python build_fa_leaf_sprays.py [dossier brut]

Lit `data/art/battle_tree_leaves.json` : pour chaque essence, un atlas de feuilles photographiées
ambientCG (CC0 1.0), téléchargé dans le dossier brut (`~/dev/cent-ans-raw/fa/ambientcg` par
défaut, hors dépôt) s'il n'y est pas. Les feuilles sont détourées par la carte d'opacité, ramenées
à la couleur moyenne du rameau dessiné (les teintes de saison de `BattleTrees` restent justes) et
posées le long de brindilles ramifiées.

Produit, à côté de ce script :
- `leaf_spray_<essence>.png` : rameau feuillu de l'essence ;
- `dead_leaves_oak.png` : feuilles de chêne sèches sur `twig_spray.png` (chêne marcescent).
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
CATALOGUE = HERE.parents[3] / "data" / "art" / "battle_tree_leaves.json"
DEFAULT_RAW = Path.home() / "dev" / "cent-ans-raw" / "fa" / "ambientcg"
MIN_LEAF_PIXELS = 4000
LUMA = np.array([0.3, 0.59, 0.11])


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


def _cut_leaves(raw: Path, asset: str, axis: str, source_url: str) -> list[Image.Image]:
    """Feuilles détourées de l'atlas, grand axe vertical."""
    color_path, opacity_path = _atlas_maps(raw, asset, source_url)
    color = np.array(Image.open(color_path).convert("RGB"))
    opacity = np.array(Image.open(opacity_path).convert("L"))
    labels = ndimage.label(opacity > 128)[0]
    leaves = []
    for index, box in enumerate(ndimage.find_objects(labels)):
        own = labels[box] == index + 1
        if own.sum() < MIN_LEAF_PIXELS:
            continue
        alpha = np.where(own, opacity[box], 0).astype(np.uint8)
        leaf = Image.fromarray(np.dstack([color[box], alpha]), "RGBA")
        leaves.append(leaf.rotate(90, expand=True) if axis == "horizontal" else leaf)
    return leaves


def _tone_leaves(
    leaves: list[Image.Image], target: np.ndarray, hue_pull: float
) -> list[Image.Image]:
    """Ramène la couleur moyenne des feuilles vers `target` (détail de la photo gardé)."""
    pixels = np.concatenate(
        [np.array(leaf)[..., :3][np.array(leaf)[..., 3] > 128] for leaf in leaves]
    ).astype(np.float32)
    mean = pixels.mean(axis=0)
    luma_gain = float(target @ LUMA) / float(mean @ LUMA)
    gain = (target / mean) ** hue_pull * luma_gain ** (1.0 - hue_pull)
    toned = []
    for leaf in leaves:
        arr = np.array(leaf).astype(np.float32)
        arr[..., :3] = (arr[..., :3] * gain).clip(0, 255)
        toned.append(Image.fromarray(arr.astype(np.uint8), "RGBA"))
    return toned


def _stamp(
    image: Image.Image,
    leaf: Image.Image,
    x: float,
    y: float,
    length: float,
    angle: float,
    shade: float,
) -> None:
    """Pose une feuille de `length` px, pied en (x, y), pointe vers `angle` (rad, repère image)."""
    scale = length / leaf.height
    small = leaf.resize(
        (max(2, round(leaf.width * scale)), max(2, round(length))),
        Image.Resampling.LANCZOS,
    )
    arr = np.array(small).astype(np.float32)
    arr[..., :3] = (arr[..., :3] * shade).clip(0, 255)
    # Feuille dans l'atlas : pointe en haut. Rotation autour du centre, puis décalage du pied.
    turned = Image.fromarray(arr.astype(np.uint8), "RGBA").rotate(
        -math.degrees(angle) - 90.0, expand=True, resample=Image.Resampling.BICUBIC
    )
    cx = x + math.cos(angle) * length * 0.5
    cy = y + math.sin(angle) * length * 0.5
    image.alpha_composite(
        turned, (round(cx - turned.width / 2), round(cy - turned.height / 2))
    )


def _twigs(
    draw: ImageDraw.ImageDraw, rng: random.Random, size: int
) -> list[tuple[float, float, float]]:
    """Brindilles ramifiées partant du pied de la carte ; rend (x, y, cap) de leurs points."""
    points = []
    unit = size / 512

    def grow(x: float, y: float, angle: float, length: float, depth: int) -> None:
        steps = 6
        for step in range(steps):
            nx = x + math.cos(angle) * length / steps
            ny = y + math.sin(angle) * length / steps
            draw.line(
                (x, y, nx, ny),
                fill=(62, 52, 42, 255),
                width=max(1, round(depth * 1.5 * unit)),
            )
            points.append((nx, ny, angle))
            x, y = nx, ny
            angle += rng.uniform(-0.18, 0.18)
            if depth > 0 and step in (1, 3, 4) and rng.random() < 0.85:
                side = rng.choice((-1, 1))
                grow(
                    x,
                    y,
                    angle + side * rng.uniform(0.5, 0.95),
                    length * rng.uniform(0.45, 0.62),
                    depth - 1,
                )

    for k in range(5):
        grow(
            size * 0.5 + rng.uniform(-30, 30) * unit,
            size * 0.98,
            -math.pi / 2 + rng.uniform(-0.15, 0.15) + (k - 2) * 0.3,
            size * rng.uniform(0.55, 0.68),
            3,
        )
    return points


def _bleed(image: Image.Image) -> Image.Image:
    """Étend la couleur des feuilles sous les pixels transparents (pas de franges aux mipmaps)."""
    arr = np.array(image)
    opaque = arr[..., 3] > 0
    nearest = ndimage.distance_transform_edt(
        ~opaque, return_distances=False, return_indices=True
    )
    arr[..., :3] = arr[..., :3][nearest[0], nearest[1]]
    return Image.fromarray(arr, "RGBA")


def _recentre(image: Image.Image, target: np.ndarray) -> Image.Image:
    """Ramène la couleur moyenne de la carte finie (brindilles et ombres comprises) à `target`."""
    arr = np.array(image).astype(np.float32)
    opaque = arr[..., 3] > 127
    arr[..., :3] = (arr[..., :3] * (target / arr[opaque][:, :3].mean(axis=0))).clip(
        0, 255
    )
    return Image.fromarray(arr.astype(np.uint8), "RGBA")


def _coverage(image: Image.Image) -> float:
    return float((np.array(image)[..., 3] > 127).mean())


def leaf_spray(species: str, entry: dict, catalogue: dict, raw: Path) -> None:
    """Rameau feuillu d'une essence : feuilles réelles au bout et le long des brindilles."""
    size = int(catalogue["size"])
    unit = size / 1024
    rng = random.Random(f"fa1/{species}")
    target = np.array(catalogue["target_rgb"], dtype=np.float32) * np.array(
        entry["tone"]
    )
    leaves = _tone_leaves(
        _cut_leaves(raw, entry["asset"], entry["axis"], catalogue["source_url"]),
        target,
        float(catalogue["hue_pull"]),
    )
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    points = _twigs(ImageDraw.Draw(image), rng, size)
    center = size / 2
    leaf_px = float(entry["leaf_px"]) * unit
    margin = leaf_px * 0.6
    placed = 0
    budget = 1600
    while _coverage(image) < float(catalogue["coverage"]) and placed < budget:
        for _ in range(40):
            tx, ty, heading = rng.choice(points)
            spread = rng.uniform(0.0, 22.0) * unit * math.sqrt(rng.random())
            around = rng.random() * math.tau
            x = tx + math.cos(around) * spread
            y = ty + math.sin(around) * spread
            radius = math.hypot(x - center, y - center * 0.9) / (center * 0.95)
            # La feuille part de la brindille, vers l'avant ou sur les côtés.
            angle = heading + rng.choice((-1, 1)) * rng.uniform(0.2, 1.5)
            tip_x = x + math.cos(angle) * leaf_px
            tip_y = y + math.sin(angle) * leaf_px
            if radius > 0.92 or not (
                margin < min(x, tip_x)
                and max(x, tip_x) < size - margin
                and margin < min(y, tip_y)
                and max(y, tip_y) < size - margin
            ):
                continue
            # Feuilles du dessous (posées d'abord) et cœur plus sombres, bord et haut plus clairs.
            depth = min(placed / (budget * 0.5), 1.0)
            shade = 0.55 + 0.22 * depth + 0.2 * radius + (center - y) / size * 0.15
            shade += rng.uniform(-0.12, 0.12)
            _stamp(
                image,
                rng.choice(leaves),
                x,
                y,
                leaf_px * rng.uniform(0.8, 1.2),
                angle,
                shade,
            )
            placed += 1
    _bleed(_recentre(image, target)).save(HERE / f"leaf_spray_{species}.png")
    print(
        f"leaf_spray_{species}.png : {placed} feuilles, couverture {_coverage(image):.2f}"
    )


def dead_leaves(entry: dict, catalogue: dict, raw: Path) -> None:
    """Feuilles de chêne sèches clairsemées sur les ramilles nues (`twig_spray.png`)."""
    size = int(catalogue["size"])
    unit = size / 1024
    rng = random.Random("fa1/dead")
    leaves = _tone_leaves(
        _cut_leaves(raw, entry["asset"], entry["axis"], catalogue["source_url"]),
        np.array(entry["target_rgb"], dtype=np.float32),
        float(catalogue["hue_pull"]),
    )
    image = (
        Image.open(HERE / "twig_spray.png")
        .convert("RGBA")
        .resize((size, size), Image.Resampling.LANCZOS)
    )
    ys, xs = np.nonzero(np.array(image)[..., 3] > 127)
    leaf_px = float(entry["leaf_px"]) * unit
    for _ in range(int(entry["count"])):
        pick = rng.randrange(len(xs))
        # Les feuilles sèches pendent : pointe vers le bas, plus ou moins.
        angle = math.pi / 2 + rng.uniform(-1.1, 1.1)
        x = float(np.clip(xs[pick], leaf_px, size - leaf_px))
        y = float(np.clip(ys[pick], leaf_px, size - leaf_px * 1.5))
        _stamp(
            image,
            rng.choice(leaves),
            x,
            y,
            leaf_px * rng.uniform(0.75, 1.2),
            angle,
            rng.uniform(0.75, 1.15),
        )
    _bleed(image).save(HERE / "dead_leaves_oak.png")
    print(f"dead_leaves_oak.png : couverture {_coverage(image):.2f}")


if __name__ == "__main__":
    raw_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_RAW
    document = json.loads(CATALOGUE.read_text(encoding="utf-8"))
    for name, species_entry in document["species"].items():
        leaf_spray(name, species_entry, document, raw_dir)
    dead_leaves(document["dead"], document, raw_dir)
