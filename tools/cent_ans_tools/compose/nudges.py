"""Retouches d'auteur de la position composée des lieux (lot CC, CE).

``data/map/compose/place_nudges.json`` (schéma ``compose_place_nudges``) associe l'``id`` d'un lieu
de ``selection.json`` à une position cible en pixels composés. Le ``lonlat`` réel ne change JAMAIS :
seule la position composée (``forward(lon, lat)``) est remplacée. Tous les consommateurs de
positions composées (relief, couches dérivées) passent par :func:`place_positions`.
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from cent_ans_tools.paths import DATA_DIR

COMPOSE_DIR = DATA_DIR / "map" / "compose"
NUDGES_FILE = "place_nudges.json"


def load_nudges(compose_dir: Path = COMPOSE_DIR) -> dict[str, tuple[float, float]]:
    """``id -> (x, y)`` ; fichier absent = aucune retouche."""
    path = compose_dir / NUDGES_FILE
    if not path.exists():
        return {}
    doc = json.loads(path.read_text(encoding="utf-8"))
    return {n["id"]: (float(n["target_px"][0]), float(n["target_px"][1])) for n in doc["nudges"]}


def place_positions(
    cw,  # noqa: ANN001 - ComposedWarp
    settlements: list[dict],
    nudges: dict[str, tuple[float, float]] | None = None,
) -> np.ndarray:
    """Positions composées ``(n, 2)`` des lieux, retouches d'auteur appliquées."""
    lon = np.array([s["lon"] for s in settlements], dtype=np.float64)
    lat = np.array([s["lat"] for s in settlements], dtype=np.float64)
    px, py = cw.forward(lon, lat)
    out = np.stack([np.asarray(px, dtype=np.float64), np.asarray(py, dtype=np.float64)], axis=1)
    for index, item in enumerate(settlements):
        if nudges and item["id"] in nudges:
            out[index] = nudges[item["id"]]
    return out
