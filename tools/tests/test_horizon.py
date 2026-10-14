"""Battle horizon (lot EP2): data schema, baked relief tiles, panorama keying."""

import json
import math
from pathlib import Path

import numpy as np
from PIL import Image

from cent_ans_tools import horizon_panoramas
from cent_ans_tools.geo import horizon

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
RELIEF = ROOT / "game" / "assets" / "horizon" / "relief"
PANORAMAS = ROOT / "game" / "assets" / "horizon" / "panoramas"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_horizon_references_exist() -> None:
    """Rules name known panoramas and provinces; every panorama band is present."""
    data = _load("fx/horizon.json")
    provinces = {p.stem for p in (DATA / "provinces").glob("prov_*.json")}
    for rule in data["rules"]:
        assert rule["panorama"] in data["panoramas"], rule
        for province in rule.get("provinces", []):
            assert province in provinces, province
    assert "provinces" not in data["rules"][-1], "the last rule must be a catch-all"
    meta = json.loads((PANORAMAS / "panoramas.json").read_text(encoding="utf-8"))
    for panorama_id, entry in data["panoramas"].items():
        path = ROOT / "game" / entry["path"].removeprefix("res://")
        assert path.is_file(), path
        assert (
            len(meta["panoramas"][panorama_id]["skyline"])
            == horizon_panoramas.SKYLINE_SAMPLES
        )


def test_every_province_has_a_tile() -> None:
    """Each province has a decodable tile listed in the index, and the total stays small."""
    index = json.loads((RELIEF / "index.json").read_text(encoding="utf-8"))
    provinces = {p.stem for p in (DATA / "provinces").glob("prov_*.json")}
    assert provinces == set(index["provinces"])
    total = sum(p.stat().st_size for p in RELIEF.glob("*.bin"))
    assert total < 90e3 * len(provinces)  # ≈ 82 ko par province (OM2 : ~420 provinces)
    tile = horizon.decode_tile((RELIEF / "prov_bearn.bin").read_bytes())
    assert tile["n"] == horizon.TILE_N
    # Béarn: the Pyrenees stand to the south, far above the plain to the north.
    south = tile["skyline_deg"][horizon.PROFILE_COUNT // 2]
    north = tile["skyline_deg"][0]
    assert south > 2.0 > north


def test_coastal_tile_sees_the_sea() -> None:
    """A coastal province keeps the sea inside its tile and knows where it lies."""
    index = json.loads((RELIEF / "index.json").read_text(encoding="utf-8"))
    entry = index["provinces"]["prov_ponthieu"]
    assert entry["sea_share"] > 0.05
    assert entry["coast_bearing_deg"] is not None
    # The Channel lies west of Ponthieu.
    assert 200.0 < entry["coast_bearing_deg"] < 340.0


def test_encode_decode_round_trip() -> None:
    """Heights (quarter metre), classes and profiles survive the binary format."""
    n = horizon.TILE_N
    rng = np.random.default_rng(1)
    tile = horizon.HorizonTile(
        province="prov_test",
        lon=0.0,
        lat=45.0,
        ref_m=12.5,
        heights_m=rng.uniform(-50, 3000, (n, n)).astype(np.float32),
        classes=rng.integers(0, 256, (n, n)).astype(np.uint8),
        skyline_deg=rng.uniform(-1, 8, horizon.PROFILE_COUNT).astype(np.float32),
        skyline_dist_m=rng.uniform(12000, 150000, horizon.PROFILE_COUNT).astype(
            np.float32
        ),
        sea_share=rng.uniform(0, 1, horizon.PROFILE_COUNT).astype(np.float32),
        coast_bearing_deg=None,
    )
    decoded = horizon.decode_tile(horizon.encode_tile(tile))
    assert np.abs(decoded["heights"] - tile.heights_m).max() <= 0.125 + 1e-6
    assert (decoded["classes"] == tile.classes).all()
    assert np.abs(decoded["skyline_deg"] - tile.skyline_deg).max() <= 0.005 + 1e-6
    assert decoded["ref"] == 12.5


def test_skyline_profile_sees_a_ridge() -> None:
    """A 1000 m wall 20 km east appears at about atan(975/20000) in that azimuth."""

    def height_at(east: np.ndarray, north: np.ndarray) -> np.ndarray:
        return np.where((east > 19_900) & (east < 20_500), 1000.0, 0.0)

    angle, dist, _ = horizon.skyline_profile(height_at, 25.0, count=8, step_m=100.0)
    expected = math.degrees(
        math.atan2(975.0 - horizon.curvature_drop(20_000.0), 20_000.0)
    )
    assert abs(angle[2] - expected) < 0.1  # azimuth 90° = east
    assert abs(dist[2] - 20_000.0) < 200.0
    assert angle[0] < 0.0


def test_coast_bearing_points_to_the_sea() -> None:
    """Sea on the western half gives a bearing of about 270°."""
    classes = np.zeros((41, 41), dtype=np.uint8)
    classes[:, :15] = horizon.SEA_CLASS
    bearing = horizon.coast_bearing(classes)
    assert bearing is not None and abs(bearing - 270.0) < 1.0


def test_sky_keying_finds_the_crest() -> None:
    """A dark hill under a plain gradient sky is keyed out at its crest."""
    h, w = 200, 300
    rows = np.arange(h)[:, None, None]
    sky = np.broadcast_to(0.8 + rows / h * 0.1, (h, w, 3)).copy()
    crest = (120 + 20 * np.sin(np.arange(w) / 30.0)).astype(int)
    for x in range(w):
        sky[crest[x] :, x] = (0.25, 0.3, 0.2)
    image = Image.fromarray(np.uint8(sky * 255))
    found = horizon_panoramas.sky_mask(np.asarray(image, dtype=float) / 255.0)
    assert np.abs(found - crest).mean() < 4.0
    band = horizon_panoramas.process(image, width=300)
    alpha = np.asarray(band.image)[:, :, 3]
    assert alpha[0].max() == 0 and alpha[-1].min() == 255
