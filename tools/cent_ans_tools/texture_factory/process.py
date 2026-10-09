"""Raw images -> tileable tiles -> texture arrays, per pack of a catalogue (TX T2b3).

A catalogue lists its ``packs`` (name, roles, file names, layer sizes); every entry whose
role belongs to a pack and whose manifest status is ``ok`` becomes one layer, from the attempt
the manifest retains. Flagged entries are left out: consumers fall back to the parent biome.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image

from cent_ans_tools.texture_factory.color import linear_to_srgb, srgb_to_linear
from cent_ans_tools.texture_factory.generate import (
    _family_dir,
    image_path,
    load_manifest,
)
from cent_ans_tools.texture_factory.pack import pack
from cent_ans_tools.texture_factory.pbr import (
    derive_normal_rough,
    equalize_luminance,
    flatten_gradients,
    flatten_lighting,
)
from cent_ans_tools.texture_factory.seamless import make_seamless

REPO_ROOT = Path(__file__).resolve().parents[3]
PROCESSING_DEFAULTS = {
    "blend_width": 128,
    "feather": 8.0,
    "target_luminance": 0.18,
    "height_strength": 2.0,
    "roughness": 0.85,
}


def processing(document: dict[str, Any]) -> dict[str, Any]:
    """Processing parameters of the catalogue over the defaults."""
    return {**PROCESSING_DEFAULTS, **document.get("processing", {})}


def pack_entries(
    document: dict[str, Any], pack_spec: dict[str, Any], manifest: dict[str, Any]
) -> list[dict[str, Any]]:
    """Entries of one pack, in catalogue order, with their retained attempt."""
    roles = set(pack_spec["roles"])
    chosen = []
    for entry in document["entries"]:
        record = manifest.get(entry["id"], {})
        if entry["role"] in roles and record.get("status") == "ok":
            chosen.append({**entry, "attempt": record["attempt"]})
    return chosen


def tile_dir(document: dict[str, Any], size: int, raw_dir: Path | None = None) -> Path:
    """Directory of the processed tiles of one layer size."""
    return _family_dir(document, raw_dir) / f"tiles_{size}"


def process_entry(
    document: dict[str, Any],
    entry: dict[str, Any],
    size: int,
    raw_dir: Path | None = None,
) -> Path:
    """Raw image -> ``<id>_albedo.png`` + ``<id>_normal.png`` (size²); cached."""
    tiles = tile_dir(document, size, raw_dir)
    albedo_path = tiles / f"{entry['id']}_albedo.png"
    normal_path = tiles / f"{entry['id']}_normal.png"
    source_path = image_path(document, entry["id"], entry["attempt"], raw_dir)
    if (
        albedo_path.is_file()
        and normal_path.is_file()
        and albedo_path.stat().st_mtime >= source_path.stat().st_mtime
    ):
        return albedo_path
    params = processing(document)
    source = Image.open(source_path).convert("RGB")
    linear = srgb_to_linear(np.asarray(source, dtype=np.float64) / 255.0)
    scale = source.width / 1024
    linear = flatten_gradients(linear, sigma=source.width / 6)
    tiled = make_seamless(
        linear, int(params["blend_width"] * scale), feather=params["feather"] * scale
    )
    tiled = flatten_lighting(tiled, sigma=source.width / 8)
    tiled = equalize_luminance(tiled, params["target_luminance"])
    albedo = Image.fromarray(
        np.clip(np.rint(linear_to_srgb(tiled) * 255), 0, 255).astype(np.uint8)
    )
    if albedo.width != size:
        albedo = albedo.resize((size, size), Image.Resampling.LANCZOS)
    normal = derive_normal_rough(
        srgb_to_linear(np.asarray(albedo) / 255.0),
        entry.get("height_strength", params["height_strength"]),
        params["roughness"],
    )
    tiles.mkdir(parents=True, exist_ok=True)
    albedo.save(albedo_path)
    Image.fromarray(normal).save(normal_path)
    return albedo_path


def build_pack(
    document: dict[str, Any],
    pack_spec: dict[str, Any],
    raw_dir: Path | None = None,
    repo_root: Path = REPO_ROOT,
    size: int | None = None,
) -> dict[str, Any]:
    """Process and pack the entries of one pack; writes the arrays and the manifest.

    ``size`` overrides the pack's ``layer_size`` (normals at half size) and writes the
    arrays under the pack's ``hi_dir`` (outside the repository budget) instead.
    """
    manifest = load_manifest(document, raw_dir)
    entries = pack_entries(document, pack_spec, manifest)
    if not entries:
        raise ValueError(f"{pack_spec['name']} : aucune entrée retenue")
    layer_size = size or pack_spec["layer_size"]
    normal_size = layer_size // 2 if size else pack_spec["normal_size"]
    prefix = pack_spec.get("strip_prefix", "")
    materials = []
    for layer, entry in enumerate(entries):
        process_entry(document, entry, layer_size, raw_dir)
        materials.append(
            {
                "id": entry["id"],
                "name": entry["id"].removeprefix(prefix),
                "layer": layer,
                "role": entry["role"],
                "biomes": entry.get("biomes", []),
                "tile_m": entry["tile_m"],
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
    # The packer reads ``<raw_dir>/tiles``: point it at this size's tiles.
    staging = tile_dir(document, layer_size, raw_dir).parent / f"pack_{layer_size}"
    staging.mkdir(parents=True, exist_ok=True)
    link = staging / "tiles"
    if not link.exists():
        link.symlink_to(tile_dir(document, layer_size, raw_dir))
    result = pack(
        {"materials": materials, "layer_size": layer_size, "normal_size": normal_size},
        staging,
        texture_dir,
        manifest_path,
        albedo_name=pack_spec["albedo"],
        normal_name=pack_spec["normal"],
        res_dir=res_dir,
        max_bytes=pack_spec.get("max_mb", 40) * 1_000_000 if not size else 10**10,
    )
    _rename_ids(manifest_path, {m["id"]: m["name"] for m in materials})
    return result


def _rename_ids(manifest_path: Path, names: dict[str, str]) -> None:
    """Manifest ids without the pack prefix (the names the game data use)."""
    data = json.loads(manifest_path.read_text())
    for layer in data["layers"]:
        layer["id"] = names[layer["id"]]
    manifest_path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")


def build_packs(
    document: dict[str, Any],
    names: list[str] | None = None,
    raw_dir: Path | None = None,
    repo_root: Path = REPO_ROOT,
    size: int | None = None,
) -> dict[str, dict[str, Any]]:
    """Every (or the named) pack of a catalogue."""
    specs = document.get("packs", [])
    unknown = set(names or []) - {spec["name"] for spec in specs}
    if unknown:
        raise ValueError(f"paquets inconnus : {sorted(unknown)}")
    return {
        spec["name"]: build_pack(document, spec, raw_dir, repo_root, size)
        for spec in specs
        if not names or spec["name"] in names
    }
