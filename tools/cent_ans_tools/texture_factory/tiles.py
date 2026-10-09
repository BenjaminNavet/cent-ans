"""Individual albedo + normal JPEG tiles per entry (TX water surfaces)."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image

from cent_ans_tools.texture_factory.pack import write_texture_import

#: Below this mean slope the derived normal is considered flat (silty rivers): no normal file.
FLAT_NORMAL_SLOPE = 0.045


def plain_normal(derived: np.ndarray) -> tuple[Image.Image, float]:
    """Normal map (blue = up) from a derived normal + roughness tile; returns it and its mean slope."""
    xy = derived[..., :2].astype(np.float64) / 255.0 * 2 - 1
    slope = float(np.sqrt((xy**2).sum(-1)).mean())
    z = np.sqrt(np.clip(1 - (xy**2).sum(-1), 0, 1))
    packed = np.concatenate([xy * 0.5 + 0.5, z[..., np.newaxis] * 0.5 + 0.5], axis=-1)
    image = Image.fromarray(np.clip(np.rint(packed * 255), 0, 255).astype(np.uint8))
    return image, slope


def build_tile_files(
    document: dict[str, Any],
    pack_spec: dict[str, Any],
    raw_dir: Path | None = None,
    repo_root: Path | None = None,
) -> dict[str, Any]:
    """Seamless albedo + plain normal JPEG per entry under ``texture_dir``.

    Entries whose derived normal is almost flat (the two silty rivers) get no normal file and are
    marked ``flat`` in the manifest: the game keeps the procedural water for them.
    """
    from cent_ans_tools.texture_factory.generate import load_manifest
    from cent_ans_tools.texture_factory.process import (
        REPO_ROOT,
        pack_entries,
        process_entry,
        tile_dir,
    )

    repo_root = repo_root or REPO_ROOT
    manifest = load_manifest(document, raw_dir)
    entries = pack_entries(document, pack_spec, manifest)
    if not entries:
        raise ValueError(f"{pack_spec['name']} : aucune entrée retenue")
    size = pack_spec["layer_size"]
    game_dir = repo_root / "game"
    texture_dir = game_dir / pack_spec["texture_dir"]
    texture_dir.mkdir(parents=True, exist_ok=True)
    res_dir = "res://" + pack_spec["texture_dir"].rstrip("/") + "/"
    records, total = [], 0
    for entry in entries:
        process_entry(document, entry, size, raw_dir)
        tiles = tile_dir(document, size, raw_dir)
        derived = np.asarray(
            Image.open(tiles / f"{entry['id']}_normal.png").convert("RGB")
        )
        normal, slope = plain_normal(derived)
        flat = slope < FLAT_NORMAL_SLOPE
        albedo_path = texture_dir / f"{entry['id']}_albedo.jpg"
        Image.open(tiles / f"{entry['id']}_albedo.png").convert("RGB").save(
            albedo_path, quality=pack_spec.get("albedo_quality", 82), subsampling=2
        )
        write_texture_import(albedo_path, game_dir)
        total += albedo_path.stat().st_size
        record = {
            "id": entry["id"],
            "role": entry["role"],
            "biomes": entry.get("biomes", []),
            "tile_m": entry["tile_m"],
            "slope": round(slope, 4),
            "flat": flat,
            "albedo": res_dir + f"{entry['id']}_albedo.jpg",
        }
        if not flat:
            normal_path = texture_dir / f"{entry['id']}_normal.jpg"
            normal.save(
                normal_path, quality=pack_spec.get("normal_quality", 90), subsampling=0
            )
            write_texture_import(normal_path, game_dir, normal=True)
            total += normal_path.stat().st_size
            record["normal"] = res_dir + f"{entry['id']}_normal.jpg"
        records.append(record)
    manifest_path = repo_root / pack_spec["manifest"]
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(
        json.dumps({"layer_size": size, "layers": records}, indent=1, ensure_ascii=False)
        + "\n"
    )
    if total > pack_spec.get("max_mb", 40) * 1_000_000:
        raise ValueError(f"textures d'eau trop lourdes : {total / 1e6:.1f} Mo")
    return {"grid": [len(records), 1], "bytes": total, "layers": len(records)}
