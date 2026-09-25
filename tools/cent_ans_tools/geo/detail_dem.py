"""Relief pyramid, tier 3: national DTMs on the detail zones (lot ZG3, ADR 0036).

Reads ``data/map/detail_zones.json``, fetches bare-earth DTMs (IGN RGE ALTI,
Environment Agency LiDAR, AHN, DHM Vlaanderen, Wallonie) into
``tools/geo/raw/detail/``, erases modern features (motorways, railways,
quarries, reservoirs from OpenStreetMap masks, Laplace infill), then bakes
levels E5-E7 of the pyramid (see :mod:`cent_ans_tools.geo.pyramid`) and
updates ``data/map/relief_pyramid.json``.
"""

from __future__ import annotations


def build(zone_ids: tuple[str, ...] = (), force: bool = False) -> None:
    """Bake E5-E7 for the given zones (all when empty)."""
    raise NotImplementedError("lot ZG3")
