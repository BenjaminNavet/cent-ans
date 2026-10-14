"""Fixture de la vérification croisée Rust/Python de la projection composée (lot CC, C4b).

``uv run --project tools python -m cent_ans_tools.compose.cross_fixture`` réécrit
``core/crates/data-model/tests/fixtures/composed_cross.json`` : 250 points lon/lat -> px composés et
250 px composés -> lon/lat, calculés par ``ComposedGrid`` sur ``data/map/compose``. À relancer à
chaque nouveau cartogramme (``geo compose --rebuild-warp``).
"""

from __future__ import annotations

import json

import numpy as np

from cent_ans_tools.compose.contract import ComposedWarp
from cent_ans_tools.paths import REPO_DIR

FIXTURE = REPO_DIR / "core/crates/data-model/tests/fixtures/composed_cross.json"
COUNT = 250


def write(seed: int = 7) -> dict:
    """Écrit la fixture et la renvoie."""
    cw = ComposedWarp.load()
    rng = np.random.default_rng(seed)
    w, h = cw.size_px
    forward, inverse = [], []
    while len(forward) < COUNT:
        x, y = rng.uniform(40, w - 40), rng.uniform(40, h - 40)
        lon, lat = cw.inverse(x, y)
        if not (np.isfinite(lon) and np.isfinite(lat)):
            continue
        fx, fy = cw.forward(lon, lat)
        forward.append(
            [
                round(float(lon), 6),
                round(float(lat), 6),
                round(float(fx), 3),
                round(float(fy), 3),
            ]
        )
    while len(inverse) < COUNT:
        x, y = rng.uniform(40, w - 40), rng.uniform(40, h - 40)
        lon, lat = cw.inverse(x, y)
        if not (np.isfinite(lon) and np.isfinite(lat)):
            continue
        inverse.append([round(float(x), 3), round(float(y), 3), float(lon), float(lat)])
    doc = {"forward": forward, "inverse": inverse}
    FIXTURE.write_text(json.dumps(doc, separators=(",", ":")) + "\n", encoding="utf-8")
    return doc


if __name__ == "__main__":
    write()
