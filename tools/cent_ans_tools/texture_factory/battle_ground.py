"""Battle-ground data derived from the catalogue and the map (TX lot T2c, ADR 0240).

Two files are written by ``cent-ans textures battle-data``:

* ``data/fx/battle_province_biomes.json``: biome (1..14) of every province, read at its
  ``seed_lonlat`` (``capital_lonlat`` otherwise) in ``data/map/biomes.png``; a sea pixel
  takes the nearest land pixel.
* the ``materials`` section of ``data/fx/battle_ground_layers.json``: role -> biome ->
  material id, exactly what the per-biome packs of ``ground_battle.yaml`` contain.
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image
from pyproj import Transformer

from cent_ans_tools.paths import REPO_DIR

PROVINCES_DIR = REPO_DIR / "data" / "provinces"
MAP_PATH = REPO_DIR / "data" / "map" / "map.json"
BIOMES_PATH = REPO_DIR / "data" / "map" / "biomes.png"
PROVINCE_BIOMES_PATH = REPO_DIR / "data" / "fx" / "battle_province_biomes.json"
LAYERS_PATH = REPO_DIR / "data" / "fx" / "battle_ground_layers.json"

# Battle role (shader contract) -> catalogue role of ground_battle.yaml.
CATALOGUE_ROLE = {
    "grass": "grass",
    "meadow": "meadow",
    "forest": "understory",
    "dirt": "dirt",
    "mud": "mud",
    "pebbles": "gravel",
    "rock": "rock",
    "plough": "plowed",
    "snow": "snow",
    "flowering_meadow": "flower_meadow",
    "trodden_grass": "trampled_grass",
    "stubble": "stubble",
    "fresh_plough": "fresh_plow",
}


def province_biomes() -> dict[str, int]:
    """Biome of each province at its seed point (nearest land pixel when at sea)."""
    map_doc = json.loads(MAP_PATH.read_text(encoding="utf-8"))
    transformer = Transformer.from_crs("EPSG:4326", map_doc["crs"], always_xy=True)
    west, _south, _east, north = map_doc["bounds_projected"]
    mpp = map_doc["meters_per_px"]
    raster = np.asarray(Image.open(BIOMES_PATH))
    height, width = raster.shape
    land_y, land_x = np.nonzero(raster)
    result: dict[str, int] = {}
    for path in sorted(PROVINCES_DIR.glob("prov_*.json")):
        geo = json.loads(path.read_text(encoding="utf-8")).get("geo", {})
        lonlat = geo.get("seed_lonlat") or geo.get("capital_lonlat")
        if not lonlat:
            continue
        x, y = transformer.transform(*lonlat)
        px = min(max(int((x - west) / mpp * width / map_doc["size_px"][0]), 0), width - 1)
        py = min(max(int((north - y) / mpp * height / map_doc["size_px"][1]), 0), height - 1)
        biome = int(raster[py, px])
        if biome == 0:
            nearest = int(np.argmin((land_x - px) ** 2 + (land_y - py) ** 2))
            biome = int(raster[land_y[nearest], land_x[nearest]])
        result[path.stem] = biome
    return result


def materials(document: dict, repo_root: Path = REPO_DIR) -> dict[str, dict[str, str]]:
    """Role -> biome -> material id, read from the per-biome pack manifests."""
    table: dict[str, dict[str, str]] = {role: {} for role in CATALOGUE_ROLE}
    inverse = {value: key for key, value in CATALOGUE_ROLE.items()}
    for spec in document["packs"]:
        if "biome" not in spec:
            continue
        manifest = json.loads((repo_root / spec["manifest"]).read_text(encoding="utf-8"))
        for layer in manifest["layers"]:
            table[inverse[layer["role"]]][str(spec["biome"])] = layer["id"]
    return table


def write_layers(document: dict, path: Path = LAYERS_PATH) -> dict:
    """Rewrite the ``materials`` section of the battle layers file; returns the document."""
    data = json.loads(path.read_text(encoding="utf-8"))
    data["materials"] = materials(document)
    path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    return data
