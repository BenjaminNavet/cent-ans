"""Tests de contrat de la carte composée (lot CC, C1) sur les fichiers de ``data/map``."""

from __future__ import annotations

import json
from pathlib import Path

import jsonschema
import numpy as np
import pytest
from PIL import Image
from scipy import ndimage

from cent_ans_tools.compose.contract import ComposedWarp, bilinear
from cent_ans_tools.geo import terrain
from cent_ans_tools.geo.project import (
    ComposedGrid,
    MapGrid,
    grid_from_metadata,
    real_grid_from_metadata,
)
from cent_ans_tools.paths import DATA_DIR

MAP_DIR = DATA_DIR / "map"
COMPOSE_DIR = MAP_DIR / "compose"
SCHEMAS = DATA_DIR / "schemas"
Image.MAX_IMAGE_PIXELS = None


def _load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def _validate(instance, schema_name: str) -> None:
    schema = _load(SCHEMAS / f"{schema_name}.schema.json")
    jsonschema.validate(instance, schema)


@pytest.fixture(scope="module")
def warp() -> ComposedWarp:
    """Déformation relue depuis le contrat versionné."""
    return ComposedWarp.load(COMPOSE_DIR)


@pytest.fixture(scope="module")
def land() -> np.ndarray:
    """Masque terre composé (booléen)."""
    return np.asarray(Image.open(MAP_DIR / "land_mask.png")) > 0


# -- format de la déformation -------------------------------------------------


def test_warp_json_is_valid_and_matches_files(warp: ComposedWarp) -> None:
    """``warp.json`` suit son schéma et décrit exactement les fichiers binaires."""
    doc = _load(COMPOSE_DIR / "warp.json")
    _validate(doc, "compose_warp")
    assert doc["factor"] == 8
    for key in ("forward", "inverse"):
        entry = doc[key]
        size = (COMPOSE_DIR / entry["file"]).stat().st_size
        assert size == int(np.prod(entry["shape"])) * 4
    x0, y0, x1, y1 = doc["crop_box_px"]
    assert doc["size_px"] == [x1 - x0, y1 - y0]
    assert doc["forward"]["shape"][:2] == [(y1 - y0) // 8, (x1 - x0) // 8]
    assert doc["inverse"]["shape"][:2] == [
        doc["size_px"][1] // 8,
        doc["size_px"][0] // 8,
    ]
    assert doc["params"]["strength"] == 0.5
    assert doc["params"]["sigma_km"] == 200.0
    assert doc["params"]["sea_density"] == 1.0
    assert warp.forward_grid.dtype == np.dtype("<f4")


def test_forward_inverse_round_trip_under_one_pixel(warp: ComposedWarp) -> None:
    """composé -> réel -> composé rend le point à moins d'un pixel (hors bord de 40 px)."""
    rng = np.random.default_rng(3)
    w, h = warp.size_px
    x = rng.uniform(40, w - 40, 200_000)
    y = rng.uniform(40, h - 40, 200_000)
    rx, ry = warp.inverse_px(x, y)
    fx, fy = warp.forward_px(rx, ry)
    error = np.hypot(fx - x, fy - y)
    assert np.isfinite(error).all()
    assert np.percentile(error, 99.9) < 1.0
    assert error.mean() < 0.1


def test_out_of_extent_is_nan(warp: ComposedWarp) -> None:
    """Hors emprise (réel ou composé) : NaN."""
    x0, y0, x1, y1 = warp.crop_box_px
    fx, fy = warp.forward_px(
        np.array([x0 - 5.0, x1 + 5.0]), np.array([y0 + 100.0, y1 + 5.0])
    )
    assert np.isnan(fx).all() and np.isnan(fy).all()
    w, h = warp.size_px
    rx, _ = warp.inverse_px(np.array([-1.0, w + 1.0]), np.array([10.0, 10.0]))
    assert np.isnan(rx).all()


def test_bilinear_is_exact_on_nodes_and_linear() -> None:
    """L'interpolation retrouve les nœuds et prolonge linéairement au-delà."""
    jj, ii = np.mgrid[0:5, 0:6].astype(float)
    grid = np.stack([2.0 * ii + jj, ii - 3.0 * jj], axis=-1)
    u = np.array([0.0, 2.5, 5.0, 5.4, -0.3])
    v = np.array([0.0, 1.5, 4.0, 4.2, -0.2])
    out = bilinear(grid, u, v)
    assert np.allclose(out[..., 0], 2.0 * u + v)
    assert np.allclose(out[..., 1], u - 3.0 * v)


def test_pins_move_the_pinned_point() -> None:
    """Une épingle d'auteur ramène le point réel sur sa cible."""
    from cent_ans_tools.compose.compile import apply_pins
    from cent_ans_tools.compose.warp import Warp

    warp = Warp.identity()
    px, py = warp.grid.lonlat_to_pixel(2.35, 48.85)
    before = np.array([float(v) for v in warp.forward_px(px, py)])
    target = before + np.array([60.0, -40.0])
    apply_pins(
        warp,
        [
            {
                "id": "t",
                "lonlat": [2.35, 48.85],
                "target_px": target.tolist(),
                "radius_km": 120,
            }
        ],
    )
    after = np.array([float(v) for v in warp.forward_px(px, py)])
    assert np.hypot(*(after - target)) < 2.0


def test_pins_file_is_valid() -> None:
    """``pins.json`` et ``generalise.json`` suivent leurs schémas."""
    _validate(_load(COMPOSE_DIR / "pins.json"), "compose_pins")
    _validate(_load(COMPOSE_DIR / "generalise.json"), "compose_generalise")


# -- grille composée ----------------------------------------------------------


def test_map_json_is_composed_and_valid() -> None:
    """``map.json`` : CRS composé, taille du canevas, ``real_grid`` de l'ancienne carte."""
    meta = _load(MAP_DIR / "map.json")
    _validate(meta, "map")
    assert meta["crs"] == "composed"
    assert meta["size_px"] == _load(COMPOSE_DIR / "warp.json")["size_px"]
    assert meta["real_grid"]["size_px"] == [7168, 6144]
    assert meta["real_grid"]["crs"] == "EPSG:3035"
    grid = grid_from_metadata(meta)
    assert isinstance(grid, ComposedGrid)
    assert grid.shape == (meta["size_px"][1], meta["size_px"][0])
    real = real_grid_from_metadata(meta)
    assert isinstance(real, MapGrid) and not isinstance(real, ComposedGrid)
    assert real.shape == (6144, 7168)
    assert grid.real.bounds == real.bounds
    with pytest.raises(NotImplementedError):
        _ = grid.transform


def test_composed_grid_projection_round_trip() -> None:
    """lon/lat -> px composés -> lon/lat (aller-retour) et échelle de ``scaled``."""
    grid = grid_from_metadata(_load(MAP_DIR / "map.json"))
    lon = np.array([2.35, -0.12, 12.5, 4.4, 28.97, -3.7])
    lat = np.array([48.85, 51.5, 41.9, 51.2, 41.0, 40.4])
    px, py = grid.lonlat_to_pixel(lon, lat)
    assert np.isfinite(px).all()
    assert ((px > 0) & (px < grid.width_px) & (py > 0) & (py < grid.height_px)).all()
    blon, blat = grid.pixel_to_lonlat(px, py)
    assert np.allclose(blon, lon, atol=0.01) and np.allclose(blat, lat, atol=0.01)
    double = grid.scaled(2.0)
    dx, dy = double.lonlat_to_pixel(lon, lat)
    assert np.allclose(dx, 2 * px) and np.allclose(dy, 2 * py)
    assert double.meters_per_px == pytest.approx(grid.meters_per_px / 2)
    # Paris est au nord-ouest de Rome, à l'ouest de Constantinople
    assert px[0] < px[2] < px[4] and py[0] < py[2]


# -- terre, côtes -------------------------------------------------------------


def _inland_water(land: np.ndarray) -> np.ndarray:
    """Plans d'eau intérieurs (non reliés au bord du canevas) : lacs fins du lot C4a, absents des côtes."""
    labels, _ = ndimage.label(~land)
    edge = np.unique(
        np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]])
    )
    return (labels > 0) & ~np.isin(labels, edge)


def test_land_mask_is_consistent_with_coastline(land: np.ndarray) -> None:
    """Le remplissage pair-impair des anneaux de la côte redonne le masque."""
    from rasterio import features
    from shapely.geometry import Polygon

    geo = _load(MAP_DIR / "coastline.geojson")
    h, w = land.shape
    filled = np.zeros(land.shape, dtype=bool)
    for feat in geo["features"]:
        ring = feat["geometry"]["coordinates"]
        assert ring[0] == ring[-1]
        mask = (
            features.rasterize([(Polygon(ring), 1)], out_shape=(h, w), dtype=np.uint8)
            > 0
        )
        filled ^= mask
    # Les lacs (C4a : lacs fins percés après la côte ; grands lacs : trous des anneaux) ne sont pas
    # la côte : on compare la mer seulement.
    keep = ~_inland_water(land)
    filled, land = filled & keep, land & keep
    union = (filled | land).sum()
    assert (filled & land).sum() / union > 0.999
    assert 0.2 < land.mean() < 0.7


def test_land_mask_has_png_contract(land: np.ndarray) -> None:
    """8 bits, 0 / 255, à la taille du canevas."""
    img = Image.open(MAP_DIR / "land_mask.png")
    assert img.mode == "L"
    assert set(np.unique(np.asarray(img)).tolist()) <= {0, 255}
    meta = _load(MAP_DIR / "map.json")
    assert img.size == tuple(meta["size_px"])


def test_no_tiny_islands_and_no_round_discs() -> None:
    """Aucune masse sous l'aire minimale ; les îles gardées ne sont pas des disques."""
    from shapely.geometry import Polygon

    cfg = _load(COMPOSE_DIR / "generalise.json")
    km2 = 0.7189765625**2
    floor = cfg["land"]["min_kept_island_km2"] * 0.85
    geo = _load(MAP_DIR / "coastline.geojson")
    for feat in geo["features"]:
        poly = Polygon(feat["geometry"]["coordinates"])
        # Depuis C4a, les lacs fins (>= lakes_fine.min_km2) sont des anneaux légitimes, parfois ronds :
        # seuls les anneaux plus petits que la moitié de ce seuil sont jugés comme îlots.
        if poly.area * km2 < min(cfg["land"]["min_lake_km2"], cfg["lakes_fine"]["min_km2"]) * 0.5:
            # petit anneau : île (ou lac) minimal, jamais un disque parfait
            compact = 4 * np.pi * poly.area / max(poly.length**2, 1e-9)
            assert poly.area * km2 >= floor or compact < 0.9
            assert compact < 0.88, "îlot rond (disque) détecté"


def test_seas_are_connected(land: np.ndarray, warp: ComposedWarp) -> None:
    """Atlantique, Méditerranée, Adriatique, Égée, mer Noire et Baltique : une seule mer."""
    grid = grid_from_metadata(_load(MAP_DIR / "map.json"))
    labels, _ = ndimage.label(~land)
    spots = {
        "Atlantique": (-9.0, 45.0),
        "Méditerranée": (5.0, 40.0),
        "Adriatique": (15.5, 43.0),
        "Égée": (25.0, 37.5),
        "Mer Noire": (34.0, 43.5),
        "Baltique": (19.5, 58.0),
        "Mer du Nord": (3.0, 55.0),
    }
    found = {}
    for name, (lon, lat) in spots.items():
        x, y = grid.lonlat_to_pixel(lon, lat)
        assert not land[int(y), int(x)], f"{name} tombe sur la terre"
        found[name] = labels[int(y), int(x)]
    assert len(set(found.values())) == 1, found


# -- fleuves ------------------------------------------------------------------


def test_rivers_are_continuous_and_on_land(land: np.ndarray) -> None:
    """≈ 40 fleuves, un seul trait continu par nom, sur la terre, avec largeur et rang."""
    geo = _load(MAP_DIR / "rivers.geojson")
    feats = geo["features"]
    assert 35 <= len(feats) <= 50
    names = [f["properties"]["name"] for f in feats]
    assert len(set(names)) == len(names), "un fleuve en plusieurs tronçons"
    for key in ("Loire", "Seine", "Rhône", "Rhine", "Danube", "Thames", "Po", "Ebro"):
        assert key in names
    h, w = land.shape
    for feat in feats:
        assert feat["geometry"]["type"] == "LineString"
        props = feat["properties"]
        assert props["width_px"] > 0 and props["scalerank"] is not None
        pts = np.array(feat["geometry"]["coordinates"])
        assert len(pts) >= 5
        gaps = np.hypot(*np.diff(pts, axis=0).T)
        assert gaps.max() < 25.0, f"{props['name']} discontinu"
        assert (pts[:, 0] >= 0).all() and (pts[:, 0] <= w).all()
        assert (pts[:, 1] >= 0).all() and (pts[:, 1] <= h).all()
        xi = np.clip(pts[:, 0].astype(int), 0, w - 1)
        yi = np.clip(pts[:, 1].astype(int), 0, h - 1)
        assert land[yi, xi].mean() > 0.9, f"{props['name']} hors de la terre"


# -- relief provisoire --------------------------------------------------------


def test_heightmap_matches_land_mask(land: np.ndarray) -> None:
    """Même encodage que l'ancien relief ; terre > 0 m, eau < 0 m."""
    meta = _load(MAP_DIR / "map.json")
    encoded = terrain.read_png16(MAP_DIR / "heightmap.png")
    assert encoded.shape == land.shape
    height = terrain.uint16_to_height(encoded)
    assert (
        meta["height_min_m"] == terrain.HEIGHT_MIN_M
        and meta["height_max_m"] == terrain.HEIGHT_MAX_M
    )
    assert (height[land] > 0).mean() > 0.999
    sea = ~land & ~_inland_water(
        land
    )  # les lacs sont des cuvettes plates au-dessus de 0 (C2)
    assert (height[sea] < 0).mean() > 0.999
    assert height.max() > 3000  # les Alpes sont là


# -- crêtes -------------------------------------------------------------------


def test_ridges_file() -> None:
    """``ridges.json`` suit son schéma, reste dans le canevas et garde les cols."""
    doc = _load(COMPOSE_DIR / "ridges.json")
    _validate(doc, "compose_ridges")
    w, h = doc["size_px"]
    assert len(doc["ridges"]) >= 50
    for ridge in doc["ridges"]:
        pts = np.array(ridge["points"])
        assert (pts >= 0).all() and (pts[:, 0] <= w).all() and (pts[:, 1] <= h).all()
        assert ridge["height_m"] > 0
    ids = [p["id"] for r in doc["ridges"] for p in r["passes"]]
    assert len(ids) >= 20
    assert any(r["kind"] == "massif" for r in doc["ridges"])
