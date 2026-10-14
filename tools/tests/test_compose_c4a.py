"""Couches dérivées de la carte composée indépendantes des provinces (lot CC, C4a)."""

from __future__ import annotations

import json
from pathlib import Path

import jsonschema
import numpy as np
import pytest
from PIL import Image
from shapely.geometry import LineString

from cent_ans_tools.compose import layers, masks, relief
from cent_ans_tools.geo import biomes, freshwater, landcover, navgrid, terrain
from cent_ans_tools.geo.project import grid_from_metadata
from cent_ans_tools.paths import DATA_DIR

MAP_DIR = DATA_DIR / "map"
COMPOSE_DIR = MAP_DIR / "compose"
Image.MAX_IMAGE_PIXELS = None


def _load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


@pytest.fixture(scope="module")
def grid():
    """Grille de la carte composée de ``data/map``."""
    return grid_from_metadata(_load(MAP_DIR / "map.json"))


# -- Carpates murées --------------------------------------------------------------------------

CARPATHIAN_PASSES = ("pass_dukla", "pass_verecke", "pass_predeal", "pass_turnu_rosu")


def test_carpathian_passes_are_documented_and_attached() -> None:
    """Quatre cols des Carpates : source, route, rattachés à la chaîne d'auteur."""
    crossings = {c["id"]: c for c in _load(MAP_DIR / "crossings.json")["crossings"]}
    ridges = _load(COMPOSE_DIR / "ridges.json")["ridges"]
    chain = next(r for r in ridges if r.get("name") == "Carpates")
    assert chain["height_m"] >= 2500.0
    attached = {p["id"] for r in ridges for p in r["passes"]}
    for pass_id in CARPATHIAN_PASSES:
        item = crossings[pass_id]
        assert item["type"] == "pass" and len(item["route"]) >= 3 and item["source"]
        assert pass_id in {p["id"] for p in chain["passes"]}, pass_id
        assert pass_id in attached


def test_carpathians_are_walled_except_at_their_passes() -> None:
    """La règle ``massif_impassable`` mure l'arc des Carpates ; les seules trouées sont les cols."""
    height = terrain.uint16_to_height(terrain.read_png16(MAP_DIR / "heightmap.png"))
    land = np.asarray(Image.open(MAP_DIR / "land_mask.png")) > 0
    scale = navgrid.NAVGRID_SCALE
    rows, cols = height.shape[0] // scale * scale, height.shape[1] // scale * scale
    small = (
        height[:rows, :cols]
        .reshape(rows // scale, scale, cols // scale, scale)
        .mean(axis=(1, 3))
    )
    water = (~land[:rows, :cols]).reshape(
        rows // scale, scale, cols // scale, scale
    ).mean(axis=(1, 3)) > 0.5
    mass = navgrid.massif_mask(
        small, water, navgrid.load_rules()
    ) | navgrid.authored_mass(MAP_DIR, small.shape)
    ridges = _load(COMPOSE_DIR / "ridges.json")["ridges"]
    chain = next(r for r in ridges if r.get("name") == "Carpates")
    line = LineString(chain["points"])
    pass_points = [np.asarray(p["at"]) for p in chain["passes"]]
    gaps = []
    for distance in np.arange(0.0, line.length, 4.0):
        point = line.interpolate(float(distance))
        ci, cj = int(point.x // scale), int(point.y // scale)
        if not mass[cj - 1 : cj + 2, ci - 1 : ci + 2].any():
            gaps.append(np.array([point.x, point.y]))
    assert len(gaps) < 0.1 * line.length / 4.0
    for gap in gaps:  # chaque trouée est un col (marge : largeur du col + une case)
        assert min(float(np.hypot(*(gap - p))) for p in pass_points) < 18.0, gap


# -- Lacs ---------------------------------------------------------------------------------------


def test_flatten_lakes_makes_flat_basins_only_for_inland_water() -> None:
    """Un plan d'eau intérieur devient plat sous sa rive ; la mer reste négative."""
    land = np.ones((60, 80), dtype=bool)
    land[:, :10] = False  # mer à l'ouest (touche le bord)
    land[20:35, 30:50] = False  # lac
    inside = np.ones_like(land)
    h = np.where(land, 120.0 + np.arange(80)[None, :], -50.0).astype(np.float32)
    report = relief.flatten_lakes(h, land, inside)
    assert len(report) == 1
    lake = h[20:35, 30:50]
    assert np.ptp(lake) == 0.0 and 3.0 <= float(lake[0, 0]) <= 160.0
    assert h[:, :10].max() < 0.0
    assert lake[0, 0] < float(np.percentile(h[land], 50))  # sous la rive


def test_fine_lakes_are_back_in_the_composed_mask(grid) -> None:
    """Léman, Constance, Garde, Balaton et Fucino sont de l'eau dans ``land_mask.png``."""
    land = np.asarray(Image.open(MAP_DIR / "land_mask.png")) > 0
    for name, lon, lat in (
        ("Léman", 6.55, 46.45),
        ("Constance", 9.35, 47.6),
        ("Garde", 10.7, 45.65),
        ("Balaton", 17.7, 46.85),
        ("Fucino", 13.55, 41.99),
    ):
        x, y = grid.lonlat_to_pixel(lon, lat)
        row, col = int(y), int(x)
        window = land[row - 4 : row + 5, col - 4 : col + 5]
        assert not window.all(), name


def test_lakes_json_levels_are_above_the_sea() -> None:
    """Chaque lac extrait a un niveau au-dessus de celui de la mer (cuvettes plates de C2)."""
    lakes = _load(MAP_DIR / "lakes.json")["lakes"]
    assert len(lakes) > 100
    assert all(lake["level_m"] > 0.5 for lake in lakes)
    assert {"Ladoga", "Léman", "Constance"} <= {lake["name"] for lake in lakes}


# -- Rasters rééchantillonnés par la déformation -----------------------------------------------


def test_derived_rasters_have_the_composed_size(grid) -> None:
    """Biomes, forêts, zones humides, coast_dist, agri, occlusion : taille du canevas composé."""
    rows, cols = grid.shape
    for name, factor in (
        ("biomes.png", 1),
        ("coast_dist.png", 1),
        ("splat.png", 1),
        ("wetlands.png", 1),
        ("forest_kind.png", 2),
        ("agri_regions.png", 4),
        ("relief_occlusion.png", 2),
    ):
        with Image.open(MAP_DIR / name) as image:
            assert image.size == (cols // factor, rows // factor), name


def test_majority_filter_removes_speckles_and_keeps_regions() -> None:
    """Le filtre majoritaire généralise un raster catégoriel (pixels isolés avalés)."""
    codes = np.ones((40, 40), dtype=np.uint8)
    codes[:, 20:] = 7
    codes[10, 5] = 3  # grain isolé
    codes[20, 30] = 2
    out = biomes.majority_filter(codes, 3, chunk_rows=9)
    assert out[10, 5] == 1 and out[20, 30] == 7
    assert (out[:, :15] == 1).all() and (out[:, 25:] == 7).all()


def test_majority_filter_is_independent_of_the_band_height() -> None:
    """Même résultat quelle que soit la hauteur des bandes (mémoire bornée)."""
    rng = np.random.default_rng(3)
    codes = rng.integers(0, 6, size=(50, 37)).astype(np.uint8)
    assert (
        biomes.majority_filter(codes, 3, chunk_rows=7)
        == biomes.majority_filter(codes, 3, chunk_rows=50)
    ).all()


def test_ellipses_follow_the_local_scale_of_the_warp(grid) -> None:
    """Une ellipse en km réels est tracée à travers la déformation, pas à ``meters_per_px`` fixe."""
    ellipse = {"center": [2.35, 48.85], "radii_km": [60.0, 30.0], "angle_deg": 0.0}
    polygon = landcover.ellipse_polygon(ellipse)
    lon, lat = zip(*polygon, strict=True)
    px, py = grid.lonlat_to_pixel(np.array(lon), np.array(lat))
    cx, cy = grid.lonlat_to_pixel(*ellipse["center"])
    east_px = float(np.hypot(px[0] - cx, py[0] - cy))  # sommet du grand axe
    placed = freshwater._ellipse_px(grid, ellipse["center"], ellipse["radii_km"], 0.0)
    assert placed["radii_px"][0] == pytest.approx(east_px, rel=1e-3)
    nominal = 60.0 * 1000.0 / grid.meters_per_px
    assert placed["radii_px"][0] != pytest.approx(nominal, rel=0.005)
    dist, window = landcover.area_signed_distance({"ellipse": ellipse}, grid)
    assert dist.shape == (
        window[0].stop - window[0].start,
        window[1].stop - window[1].start,
    )
    assert float(dist.max()) > 0.3 * min(placed["radii_px"])


def test_area_off_the_canvas_gives_an_empty_window(grid) -> None:
    """Une zone hors de l'emprise de la déformation (NaN) n'ajoute rien et ne plante pas."""
    ellipse = {"center": [70.0, 60.0], "radii_km": [80.0, 40.0]}
    dist, window = landcover.area_signed_distance({"ellipse": ellipse}, grid)
    assert dist.size == 0 and window[0].stop == window[0].start
    assert landcover.area_min_radius_px({"ellipse": ellipse}, grid) == 0.0


# -- Masques d'auteur ----------------------------------------------------------------------------


def _write_mask(directory: Path, name: str, cells: np.ndarray) -> None:
    (directory / "masks").mkdir(parents=True, exist_ok=True)
    Image.fromarray(cells.astype(np.uint8), mode="L").save(
        directory / "masks" / f"{name}.png"
    )


def test_masks_absent_or_empty_change_nothing(tmp_path: Path) -> None:
    """Pas de fichier, ou fichier vide : aucun masque."""
    assert masks.load_all((64, 96), tmp_path) == {}
    _write_mask(tmp_path, "forest", np.zeros((4, 6)))
    assert masks.load_mask("forest", (64, 96), tmp_path) is None


def test_mask_cells_are_16_pixels_wide(tmp_path: Path) -> None:
    """Une case de 16 px : plein centre, bord doux, taille du canevas."""
    cells = np.zeros((6, 8))
    cells[2:4, 3:5] = 255
    _write_mask(tmp_path, "forest", cells)
    mask = masks.load_mask("forest", (96, 128), tmp_path)
    assert mask.shape == (96, 128) and mask.dtype == np.float32
    assert mask[40, 64] == pytest.approx(1.0)  # centre des cases 2-3 x 3-4
    assert mask[5, 5] == 0.0
    assert 0.0 < mask[40, 44] < 1.0  # bord lissé


def test_masks_override_cover(tmp_path: Path) -> None:
    """Forêt ajoutée, champs et marais retirent la forêt, roche = lande, marais = canal R."""
    shape = (32, 32)
    full = np.ones(shape, dtype=np.float32)
    forest, heath = np.zeros(shape, np.float32), np.zeros(shape, np.float32)
    cover, rock = masks.apply_cover(forest, heath, {"forest": full})
    assert cover.min() == 1.0 and rock.max() == 0.0
    cover, rock = masks.apply_cover(np.ones(shape, np.float32), heath, {"fields": full})
    assert cover.max() == 0.0
    cover, rock = masks.apply_cover(np.ones(shape, np.float32), heath, {"rock": full})
    assert cover.max() == 0.0 and rock.min() == 1.0
    wet = masks.apply_wetlands(np.zeros((*shape, 3), np.float32), {"marsh": full})
    assert wet[..., 0].min() == 1.0 and wet[..., 1].max() == 0.0
    weights = np.zeros((*shape, 4), np.float32)
    weights[..., 2] = 1.0
    farm = masks.apply_splat(weights, {"fields": full})
    assert farm[..., 1].min() == 1.0 and farm[..., 2].max() == 0.0


# -- Horizon : relief réel -------------------------------------------------------------------------


def test_horizon_reads_the_real_relief_not_the_composed_tiles(grid) -> None:
    """Le relief de bataille vient de ``data/map/real`` (719 m par pixel réel), pas de ``height_tiles``."""
    from cent_ans_tools.geo import horizon

    sources = horizon.Sources.__new__(horizon.Sources)
    meta = _load(MAP_DIR / "map.json")
    from cent_ans_tools.geo.project import real_grid_from_metadata

    sources.composed = True
    sources.grid = real_grid_from_metadata(meta)
    sources.bounds = tuple(sources.grid.bounds)
    sources.campaign = grid
    sources._load_real(meta)
    real_meta = _load(MAP_DIR / "real" / "map.json")
    assert sources.fine.shape == tuple(reversed(real_meta["size_px"]))
    assert sources.fine.shape != tuple(
        np.asarray(_load(MAP_DIR / "map.json")["height_tiles"]["size_px"])[::-1]
    )
    # Mont-Blanc : l'altitude réelle (≈ 4 000 m moyennés sur 719 m), pas la crête composée.
    x, y = sources.grid.pixel_to_projected(*sources.grid.lonlat_to_pixel(6.87, 45.83))
    assert 1500.0 < float(sources.fine_height(np.array([x]), np.array([y]))[0]) < 4900.0
    # La forêt (raster composé) est lue à la position composée de chaque échantillon réel.
    sources.forest = np.fromfunction(
        lambda r, c: c / (grid.width_px / 2), (grid.height_px // 2, grid.width_px // 2)
    ).astype(np.float32)
    sources.forest_mpp = 0.0
    paris = grid.lonlat_to_pixel(2.35, 48.85)
    got = float(
        sources.forest_share(np.array([x * 0 + 3750000.0]), np.array([2890000.0]))[0]
    )
    px_c, _ = grid.lonlat_to_pixel(
        *[
            float(v)
            for v in sources.grid.pixel_to_lonlat(
                *sources.grid.projected_to_pixel(3750000.0, 2890000.0)
            )
        ]
    )
    assert got == pytest.approx(float(px_c) / grid.width_px, abs=2e-3)
    assert paris[0] > 0


# -- Orchestrateur et schéma ------------------------------------------------------------------------


def test_layers_refuse_unknown_steps() -> None:
    """Une étape inconnue est refusée avant tout calcul."""
    with pytest.raises(ValueError, match="inconnues"):
        layers.run(["nope"])
    assert list(layers.STEPS)[0] == "coast" and "colormap" in layers.STEPS


def test_generalise_schema_accepts_lakes_fine() -> None:
    """``generalise.json`` (avec ``lakes_fine``) respecte son schéma."""
    jsonschema.validate(
        _load(COMPOSE_DIR / "generalise.json"),
        _load(DATA_DIR / "schemas" / "compose_generalise.schema.json"),
    )


# -- Retrait du calculé et peinture vectorielle (lot CP) ---------------------------------------------


def test_clear_masks_remove_computed_cover(tmp_path: Path) -> None:
    """Un masque ``_clear`` multiplie la couverture calculée par (1 - v) ; absent = aucun effet."""
    shape = (32, 32)
    half = np.full(shape, 0.5, dtype=np.float32)
    full = np.ones(shape, dtype=np.float32)
    forest, heath = np.ones(shape, np.float32), np.ones(shape, np.float32)
    cover, rock = masks.apply_cover(forest, heath, {})
    assert cover.min() == 1.0 and rock.min() == 1.0
    cover, rock = masks.apply_cover(
        forest, heath, {"forest_clear": half, "rock_clear": full}
    )
    assert cover.max() == pytest.approx(0.5) and rock.max() == 0.0
    # Retrait puis ajout : la forêt peinte reprend la place.
    cover, _ = masks.apply_cover(forest, heath, {"forest_clear": full, "forest": half})
    assert cover.min() == pytest.approx(0.5)
    wet = np.zeros((*shape, 3), np.float32)
    wet[..., 0] = 1.0
    wet = masks.apply_wetlands(wet, {"marsh_clear": half})
    assert wet[..., 0].max() == pytest.approx(0.5)
    weights = np.zeros((*shape, 4), np.float32)
    weights[..., 1], weights[..., 2], weights[..., 3] = 0.4, 0.3, 0.3
    out = masks.apply_splat(
        weights, {"forest_clear": full, "fields_clear": half, "rock_clear": half}
    )
    assert out[..., 2].max() == 0.0 and out[..., 1].max() == pytest.approx(0.2)
    assert out[..., 3].max() == pytest.approx(0.15)
    assert out.sum(axis=-1) == pytest.approx(1.0)


def test_brush_and_derived_masks_combine_by_max(tmp_path: Path) -> None:
    """Pinceau (masks/) et dérivé vectoriel (masks/derived/) : max, retrait chargé sous ``_clear``."""
    brush = np.zeros((6, 8))
    brush[1, 1] = 100
    _write_mask(tmp_path, "forest", brush)
    derived = np.zeros((6, 8))
    derived[1, 1], derived[4, 6] = 200, 255
    (tmp_path / "masks" / "derived").mkdir()
    Image.fromarray(derived.astype(np.uint8), mode="L").save(
        tmp_path / "masks" / "derived" / "forest.png"
    )
    Image.fromarray(derived.astype(np.uint8), mode="L").save(
        tmp_path / "masks" / "derived" / "rock_clear.png"
    )
    found = masks.load_all((96, 128), tmp_path)
    assert set(found) == {"forest", "rock_clear"}
    assert (
        found["forest"][24, 24] > 150 / 255
    )  # case (1, 1) : le dérivé (200) l'emporte sur le pinceau (100)
    assert found["forest"][72, 104] > 0.9  # case (4, 6)
