"""Alpha cut-outs -> square RGBA cards -> RGBA texture arrays (TX T3, vegetation).

Ground-plant cards and battle-grass cards are cropped to the plant, scaled so the longest side
fills the square cell and stood on its bottom edge (the quad pivot of the game meshes is the
foot of the plant). Leaf sheets keep the whole image. Both get a defringed colour (the cut-out
keeps the light-grey backdrop on soft edges) so mipmaps do not show a halo.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image
from scipy.ndimage import distance_transform_edt, gaussian_filter, minimum_filter

from cent_ans_tools.texture_factory.color import LUMA, srgb_to_linear
from cent_ans_tools.texture_factory.generate import _family_dir, load_manifest
from cent_ans_tools.texture_factory.pack import (
    grid_shape,
    import_file,
    write_texture_import,
)

ALPHA_FLOOR = 24
ALPHA_SOLID = 200
FILL = 0.97
WEBP_QUALITY = 86


def defringe(rgba: np.ndarray, erode: int = 1) -> np.ndarray:
    """Erode the alpha a little and flood the colour of transparent pixels from the nearest solid one."""
    alpha = rgba[..., 3].astype(np.float64) / 255.0
    if erode > 0:
        alpha = minimum_filter(alpha, size=2 * erode + 1)
        alpha = gaussian_filter(alpha, 0.5)
    alpha[alpha * 255 < ALPHA_FLOOR] = 0.0
    solid = alpha * 255 >= ALPHA_SOLID
    out = rgba.copy()
    if solid.any() and not solid.all():
        _, (rows, cols) = distance_transform_edt(~solid, return_indices=True)
        out[..., :3] = rgba[..., :3][rows, cols]
    out[..., 3] = np.clip(np.rint(alpha * 255), 0, 255).astype(np.uint8)
    return out


def plant_bbox(alpha: np.ndarray) -> tuple[int, int, int, int] | None:
    """(left, top, right, bottom) of the pixels above the alpha floor, or None when empty."""
    rows = np.flatnonzero((alpha > ALPHA_FLOOR * 2).any(axis=1))
    cols = np.flatnonzero((alpha > ALPHA_FLOOR * 2).any(axis=0))
    if rows.size == 0 or cols.size == 0:
        return None
    return int(cols[0]), int(rows[0]), int(cols[-1]) + 1, int(rows[-1]) + 1


def make_card(src: Path, size: int, *, crop: bool = True) -> tuple[Image.Image, float]:
    """Square RGBA card of ``size`` px from a cut-out; returns the card and the plant aspect (w/h)."""
    image = Image.open(src).convert("RGBA")
    array = defringe(np.asarray(image))
    image = Image.fromarray(array, "RGBA")
    if not crop:
        return image.resize((size, size), Image.Resampling.LANCZOS), 1.0
    box = plant_bbox(array[..., 3])
    if box is None:
        raise ValueError(f"{src} : découpe vide")
    plant = image.crop(box)
    aspect = plant.width / plant.height
    longest = max(plant.size)
    scale = size * FILL / longest
    plant = plant.resize(
        (max(1, round(plant.width * scale)), max(1, round(plant.height * scale))),
        Image.Resampling.LANCZOS,
    )
    card = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    left = (size - plant.width) // 2
    top = size - plant.height - round(size * (1 - FILL) / 2)
    card.paste(plant, (left, top))
    # Resampling blends colour with the transparent black: flood it again.
    return Image.fromarray(defringe(np.asarray(card), erode=0), "RGBA"), aspect


def mean_linear_luma(card: Image.Image) -> float:
    """Mean linear luminance of the solid pixels of an RGBA card (the game shaders normalise by it)."""
    array = np.asarray(card, dtype=np.float64)
    solid = array[..., 3] > 128
    if not solid.any():
        return 0.2
    return float((srgb_to_linear(array[solid][:, :3] / 255.0) @ LUMA).mean())


def card_entries(
    document: dict[str, Any], pack_spec: dict[str, Any], manifest: dict[str, Any]
) -> list[dict[str, Any]]:
    """Entries of an alpha pack (roles, ok status, ``alpha`` flag), in catalogue order."""
    roles = set(pack_spec["roles"])
    chosen = []
    for entry in document["entries"]:
        record = manifest.get(entry["id"], {})
        if entry["role"] in roles and record.get("status") == "ok" and entry.get("alpha"):
            chosen.append(entry)
    return chosen


def build_card_pack(
    document: dict[str, Any],
    pack_spec: dict[str, Any],
    raw_dir: Path | None = None,
    repo_root: Path | None = None,
    size: int | None = None,
) -> dict[str, Any]:
    """Cut-outs of one pack -> RGBA WebP grid + Godot import + manifest (``size`` -> ``hi_dir``)."""
    from cent_ans_tools.texture_factory.process import REPO_ROOT

    repo_root = repo_root or REPO_ROOT
    manifest = load_manifest(document, raw_dir)
    entries = card_entries(document, pack_spec, manifest)
    if not entries:
        raise ValueError(f"{pack_spec['name']} : aucune entrée retenue")
    layer_size = size or pack_spec["layer_size"]
    crop = pack_spec.get("kind", "cards") in ("cards", "files")
    prefix = pack_spec.get("strip_prefix", "")
    if pack_spec.get("kind") == "files":
        return _write_card_files(document, pack_spec, entries, raw_dir, repo_root, layer_size)
    columns, rows = grid_shape(len(entries))
    grid = Image.new("RGBA", (columns * layer_size, rows * layer_size), (0, 0, 0, 0))
    layers = []
    for layer, entry in enumerate(entries):
        src = _family_dir(document, raw_dir) / "alpha" / f"{entry['id']}.png"
        if not src.is_file():
            raise ValueError(f"{entry['id']} : détourage absent ({src}), lancer `textures alpha`")
        card, aspect = make_card(src, layer_size, crop=crop)
        grid.paste(card, ((layer % columns) * layer_size, (layer // columns) * layer_size))
        layers.append(
            {
                "id": entry["id"].removeprefix(prefix),
                "layer": layer,
                "role": entry["role"],
                "biomes": entry.get("biomes", []),
                **({"species": entry["species"]} if "species" in entry else {}),
                "tile_m": entry["tile_m"],
                "aspect": round(aspect, 3),
            }
        )
    texture_dir = repo_root / "game" / pack_spec["texture_dir"]
    res_dir = "res://" + pack_spec["texture_dir"].rstrip("/") + "/"
    manifest_path = repo_root / pack_spec["manifest"]
    if size:
        texture_dir = texture_dir / pack_spec["hi_dir"]
        res_dir += pack_spec["hi_dir"].rstrip("/") + "/"
        manifest_path = manifest_path.with_name(
            manifest_path.stem + f"_{size}" + manifest_path.suffix
        )
    texture_dir.mkdir(parents=True, exist_ok=True)
    albedo_path = texture_dir / pack_spec["albedo"]
    grid.save(albedo_path, quality=WEBP_QUALITY, method=4)
    imported = Path(f"{albedo_path}.import")
    slicing = f"slices/horizontal={columns}\nslices/vertical={rows}"
    if not imported.exists() or slicing not in imported.read_text():
        imported.write_text(import_file(columns, rows, albedo_path.name, res_dir))
    total = albedo_path.stat().st_size
    max_bytes = pack_spec.get("max_mb", 40) * 1_000_000 if not size else 10**10
    if total > max_bytes:
        raise ValueError(f"tableau trop lourd : {total / 1e6:.1f} Mo > {max_bytes / 1e6:.0f} Mo")
    document_out = {
        "layer_size": layer_size,
        "grid": [columns, rows],
        "albedo": res_dir + pack_spec["albedo"],
        "layers": layers,
    }
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(document_out, indent=1, ensure_ascii=False) + "\n")
    return {"grid": [columns, rows], "bytes": total, "layers": len(layers)}


def _write_card_files(
    document: dict[str, Any],
    pack_spec: dict[str, Any],
    entries: list[dict[str, Any]],
    raw_dir: Path | None,
    repo_root: Path,
    layer_size: int,
) -> dict[str, Any]:
    """One ``<id>.webp`` card per entry under ``texture_dir`` (plain Texture2D imports) + manifest."""
    texture_dir = repo_root / "game" / pack_spec["texture_dir"]
    texture_dir.mkdir(parents=True, exist_ok=True)
    prefix = pack_spec.get("strip_prefix", "")
    res_dir = "res://" + pack_spec["texture_dir"].rstrip("/") + "/"
    layers = []
    total = 0
    for entry in entries:
        src = _family_dir(document, raw_dir) / "alpha" / f"{entry['id']}.png"
        if not src.is_file():
            raise ValueError(f"{entry['id']} : détourage absent ({src}), lancer `textures alpha`")
        card, aspect = make_card(src, layer_size)
        luma = mean_linear_luma(card)
        name = entry["id"].removeprefix(prefix)
        path = texture_dir / f"{name}.webp"
        card.save(path, quality=WEBP_QUALITY, method=4)
        write_texture_import(path, repo_root / "game")
        total += path.stat().st_size
        layers.append(
            {
                "id": name,
                "file": res_dir + f"{name}.webp",
                "role": entry["role"],
                "biomes": entry.get("biomes", []),
                "tile_m": entry["tile_m"],
                "aspect": round(aspect, 3),
                "luma": round(luma, 4),
            }
        )
    manifest_path = repo_root / pack_spec["manifest"]
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(
        json.dumps({"layer_size": layer_size, "cards": layers}, indent=1, ensure_ascii=False) + "\n"
    )
    max_bytes = pack_spec.get("max_mb", 40) * 1_000_000
    if total > max_bytes:
        raise ValueError(f"cartes trop lourdes : {total / 1e6:.1f} Mo > {max_bytes / 1e6:.0f} Mo")
    return {"grid": [len(layers), 1], "bytes": total, "layers": len(layers)}
