"""Sea lanes (lot SL1, ADR 0139): catalogue schema, ports and routed geometry."""

import json
from pathlib import Path

import numpy as np
from PIL import Image

from cent_ans_tools.geo import sea_lanes

DATA = Path(__file__).resolve().parents[2] / "data"


def _catalogue() -> dict:
    return json.loads((DATA / "naval" / "sea_lanes.json").read_text(encoding="utf-8"))


def _settlements() -> dict[str, dict]:
    out = {}
    for path in (DATA / "settlements").glob("prov_*.json"):
        for settlement in json.loads(path.read_text(encoding="utf-8")):
            out[settlement["id"]] = settlement
    return out


def test_lanes_link_known_ports_on_known_seas() -> None:
    """Both ends are port settlements, the sea is named, ids are unique."""
    settlements = _settlements()
    seas = json.loads((DATA / "naval" / "fleets.json").read_text(encoding="utf-8"))[
        "sea_names"
    ]
    ids = [lane["id"] for lane in _catalogue()["lanes"]]
    assert len(ids) == len(set(ids))
    for lane in _catalogue()["lanes"]:
        for end in (lane["from"], lane["to"]):
            assert end in settlements, f"{lane['id']} : {end} inconnu"
            assert settlements[end].get("port"), (
                f"{lane['id']} : {end} n'est pas un port"
            )
        assert lane["sea"] in seas, lane["id"]


def test_geometry_covers_every_lane_and_stays_at_sea() -> None:
    """sea_lanes_px.json traces every lane between its ports, over water.

    Land is allowed within ~110 km of either port (estuaries: Hamburg on the
    Elbe, Bordeaux on the Gironde, London on the Thames).
    """
    geometry = json.loads(
        (DATA / "map" / "sea_lanes_px.json").read_text(encoding="utf-8")
    )
    by_id = {lane["id"]: lane for lane in geometry["lanes"]}
    positions = json.loads(
        (DATA / "map" / "settlements_px.json").read_text(encoding="utf-8")
    )
    mask = np.asarray(Image.open(DATA / "map" / "land_mask.png").convert("L"))
    for lane in _catalogue()["lanes"]:
        traced = by_id.get(lane["id"])
        assert traced is not None, f"{lane['id']} non tracée (cent-ans geo sea-lanes)"
        points = np.array(traced["points"])
        assert np.allclose(points[0], positions[lane["from"]], atol=0.2)
        assert np.allclose(points[-1], positions[lane["to"]], atol=0.2)
        assert traced["length_km"] > 0
        # Sample the polyline; allow land near both ports (estuaries, harbours).
        samples = []
        for a, b in zip(points[:-1], points[1:], strict=True):
            for t in np.linspace(0.0, 1.0, 8, endpoint=False):
                samples.append(a + (b - a) * t)
        samples = np.array(samples)
        far = (np.hypot(*(samples - points[0]).T) > 150) & (
            np.hypot(*(samples - points[-1]).T) > 150
        )
        cols = np.clip(samples[far, 0].astype(int), 0, mask.shape[1] - 1)
        rows = np.clip(samples[far, 1].astype(int), 0, mask.shape[0] - 1)
        land_share = float((mask[rows, cols] > 127).mean()) if far.any() else 0.0
        assert land_share < 0.05, f"{lane['id']} : {land_share:.0%} sur terre"


def test_simplify_keeps_shortcuts_on_water() -> None:
    """A path around an island keeps the point that avoids it."""
    water = np.ones((20, 20), dtype=bool)
    water[8:12, 8:12] = False
    path = np.array([[10.0, 2.0], [14.0, 10.0], [10.0, 18.0]])
    simplified = sea_lanes.simplify(path, water, tolerance=100.0)
    assert len(simplified) == 3
