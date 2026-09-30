"""Campaign biome map (chantier HB, lot HB1): legend, rules, smoothing, real map."""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest
import yaml
from jsonschema import Draft202012Validator

from cent_ans_tools.geo import biomes, colormap

MPP = 718.9765625

OCEANIC, CONTINENTAL, MEDITERRANEAN, STEPPE, BOREAL, MOUNTAIN, SEMI_ARID = range(1, 8)


@pytest.fixture(scope="module")
def legend() -> dict:
    """The repository legend (validated by :func:`biomes.load_legend`)."""
    return biomes.load_legend()


def _code(legend: dict, name: str) -> int:
    return legend["source"]["codes"].index(name) + 1


def _inputs(
    legend: dict, code: str | None, rows: int = 40, cols: int = 40, **overrides
) -> biomes.BiomeInputs:
    """Flat land at 100 m, 47° N 8° W, Atlantic sea on the left border (column 0)."""
    water = np.zeros((rows, cols), dtype=bool)
    water[:, 0] = True
    values = {
        "land": ~water,
        "water": water,
        "height_m": np.full((rows, cols), 100.0, dtype=np.float32),
        "lon": np.full((rows, cols), -8.0, dtype=np.float32),
        "lat": np.full((rows, cols), 47.0, dtype=np.float32),
        "coast_km": np.full((rows, cols), 45.0, dtype=np.float32),
        "dryness": np.zeros((rows, cols), dtype=np.float32),
        "meters_per_px": MPP,
        "koppen": None
        if code is None
        else np.full((rows, cols), _code(legend, code), dtype=np.uint8),
    }
    values.update(overrides)
    return biomes.BiomeInputs(**values)


def _raw(legend: dict, code: str, **overrides) -> np.ndarray:
    return biomes.classify_koppen(_inputs(legend, code, **overrides), legend)


def _grid(value: float, rows: int = 40, cols: int = 40) -> np.ndarray:
    return np.full((rows, cols), value, dtype=np.float32)


# ---------------------------------------------------------------------------
# Legend
# ---------------------------------------------------------------------------


def test_legend_matches_schema() -> None:
    """``biomes.yaml`` validates against ``biomes.schema.json``."""
    schema = json.loads(biomes.SCHEMA_PATH.read_text(encoding="utf-8"))
    document = yaml.safe_load(biomes.LEGEND_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(document))
    assert not errors, [error.message for error in errors]


def test_frozen_indices(legend: dict) -> None:
    """Indices read by other lots never move."""
    assert biomes.indices(legend) == {
        "sea": 0,
        "oceanic": 1,
        "continental": 2,
        "mediterranean": 3,
        "steppe": 4,
        "boreal": 5,
        "mountain": 6,
        "semi_arid": 7,
    }


def test_moved_index_is_rejected(tmp_path: Path) -> None:
    """Swapping two indices fails validation."""
    document = yaml.safe_load(biomes.LEGEND_PATH.read_text(encoding="utf-8"))
    document["classes"][1]["index"] = 2
    path = tmp_path / "biomes.yaml"
    path.write_text(yaml.safe_dump(document, allow_unicode=True), encoding="utf-8")
    with pytest.raises(ValueError, match="classes"):
        biomes.load_legend(path)


def test_koppen_lut(legend: dict) -> None:
    """Every Köppen class maps to a land biome; no data maps to sea."""
    lut = biomes.koppen_lut(legend)
    assert lut[0] == 0
    codes = legend["source"]["codes"]
    assert all(1 <= lut[i + 1] <= 7 for i in range(len(codes)))
    assert lut[_code(legend, "Cfb")] == OCEANIC
    assert lut[_code(legend, "Csa")] == MEDITERRANEAN
    assert lut[_code(legend, "Dfc")] == BOREAL
    assert lut[_code(legend, "ET")] == MOUNTAIN
    assert lut[_code(legend, "BSk")] == SEMI_ARID


# ---------------------------------------------------------------------------
# Rules (synthetic rasters)
# ---------------------------------------------------------------------------


def test_oceanic_only_near_the_atlantic(legend: dict) -> None:
    """Cfb next to the Atlantic is oceanic, Cfb 300 km inland is continental."""
    raw = _raw(legend, "Cfb", cols=440, lon=_grid(-8.0, cols=440), lat=_grid(47, 40, 440))
    assert raw[20, 5] == OCEANIC
    assert raw[20, 430] == CONTINENTAL


def test_dry_classes_steppe_north_semi_arid_south(legend: dict) -> None:
    """BSk is steppe north of ``steppe_min_lat``, semi-arid south (inland)."""
    lat = _grid(48.0)
    lat[20:] = 39.0
    raw = _raw(legend, "BSk", lat=lat, lon=_grid(30.0))
    assert raw[5, 20] == STEPPE
    assert raw[30, 20] == SEMI_ARID


def test_coastal_mediterranean_steppe_stays_mediterranean(legend: dict) -> None:
    """BSh on the Attic shore is mediterranean; BWh (desert) is not."""
    common = {"lat": _grid(38.0), "lon": _grid(23.7), "coast_km": _grid(5.0)}
    assert _raw(legend, "BSh", **common)[20, 20] == MEDITERRANEAN
    assert _raw(legend, "BWh", **common)[20, 20] == SEMI_ARID


def test_dry_continental_becomes_steppe(legend: dict) -> None:
    """Dfa of the Pontic steppe (dryness) is steppe; Dfb of southern Finland is boreal."""
    assert _raw(legend, "Dfa", dryness=_grid(0.9), lon=_grid(40.0))[20, 20] == STEPPE
    assert _raw(legend, "Dfb", lat=_grid(61.0), lon=_grid(25.0))[20, 20] == BOREAL


def test_altitude_makes_mountain(legend: dict) -> None:
    """Above the threshold everything humid is mountain; lowland tundra is boreal."""
    height = _grid(100.0)
    height[:, 20:] = 2000.0
    raw = _raw(legend, "Cfb", height_m=height, lon=_grid(8.0))
    assert raw[10, 30] == MOUNTAIN
    assert raw[10, 10] != MOUNTAIN
    assert _raw(legend, "ET", lon=_grid(8.0))[10, 10] == BOREAL


def test_fallback_rules(legend: dict) -> None:
    """Without Köppen: oceanic west, mediterranean south, boreal north, mountain high."""
    rows, cols = 60, 440
    lat = _grid(47.0, rows, cols)
    lat[20:40] = 40.0
    lat[40:] = 62.0
    height = _grid(100.0, rows, cols)
    height[:, 400:] = 2500.0
    inputs = _inputs(
        legend,
        None,
        rows,
        cols,
        lat=lat,
        lon=_grid(-8.0, rows, cols),
        height_m=height,
        coast_km=_grid(45.0, rows, cols),
        dryness=_grid(0.0, rows, cols),
    )
    raw = biomes.classify_fallback(inputs, legend)
    assert raw[10, 5] == OCEANIC
    assert raw[10, 300] == CONTINENTAL
    assert raw[30, 300] == MEDITERRANEAN
    assert raw[50, 300] == BOREAL
    assert raw[10, 420] == MOUNTAIN
    final = biomes.classify(inputs, legend)
    assert final.dtype == np.uint8 and final.max() <= 7
    assert (final[:, 0] == 0).all() and (final[:, 1:] > 0).all()


def test_smoothing_removes_isolated_pixels() -> None:
    """A single odd pixel and a small speckle vanish; big regions stay."""
    raw = np.full((80, 80), CONTINENTAL, dtype=np.uint8)
    raw[:, 40:] = OCEANIC
    raw[20, 20] = STEPPE
    raw[60:62, 10:12] = MEDITERRANEAN
    out = biomes.smooth(raw, 7, sigma_px=2.0, min_px=50)
    assert set(np.unique(out).tolist()) == {CONTINENTAL, OCEANIC}
    assert out[40, 5] == CONTINENTAL and out[40, 75] == OCEANIC


def test_classify_is_deterministic(legend: dict) -> None:
    """Same inputs, same bytes."""
    rng = np.random.default_rng(7)
    codes = rng.integers(1, 31, size=(40, 40)).astype(np.uint8)
    first = biomes.classify(_inputs(legend, "Cfb", koppen=codes.copy()), legend)
    second = biomes.classify(_inputs(legend, "Cfb", koppen=codes.copy()), legend)
    assert first.tobytes() == second.tobytes()
    assert first.max() <= 7


# ---------------------------------------------------------------------------
# Real map (skipped when absent)
# ---------------------------------------------------------------------------

PLACES = {
    "Bretagne": (-3.2, 48.2, OCEANIC),
    "Irlande": (-8.0, 53.0, OCEANIC),
    "Beauce": (1.8, 48.1, CONTINENTAL),
    "Bourgogne": (4.8, 47.2, CONTINENTAL),
    "Provence": (5.6, 43.5, MEDITERRANEAN),
    "Andalousie": (-5.2, 37.6, MEDITERRANEAN),
    "Athènes": (23.75, 38.0, MEDITERRANEAN),
    "Péloponnèse": (22.4, 37.5, MEDITERRANEAN),
    "Dalmatie": (16.3, 43.7, MEDITERRANEAN),
    "Pouilles": (16.9, 41.0, MEDITERRANEAN),
    "Ukraine du sud": (33.5, 46.6, STEPPE),
    "Rostov": (39.7, 47.4, STEPPE),
    "Volgograd": (44.5, 48.7, STEPPE),
    "Kazakhstan ouest": (51.5, 49.5, STEPPE),
    "Finlande": (25.5, 61.5, BOREAL),
    "Carélie": (33.0, 62.5, BOREAL),
    "Alpes bernoises": (8.0, 46.5, MOUNTAIN),
    "Mont-Blanc": (6.9, 45.9, MOUNTAIN),
    "Aragon": (-0.9, 41.6, SEMI_ARID),
    "Manche (Espagne)": (-1.9, 39.0, SEMI_ARID),
    "Anatolie centrale": (33.0, 39.0, SEMI_ARID),
}


@pytest.fixture(scope="module")
def real_map() -> tuple[np.ndarray, object]:
    """``biomes.png`` and its grid, or skip."""
    from cent_ans_tools.geo.project import grid_from_metadata

    biome = biomes.read_biomes()
    if biome is None:
        pytest.skip("data/map/biomes.png absent")
    meta = json.loads((biomes.MAP_DIR / "map.json").read_text(encoding="utf-8"))
    return biome, grid_from_metadata(meta)


def test_real_map_shape_and_indices(real_map: tuple) -> None:
    """Map-sized, indices 0-7, every land biome present."""
    biome, grid = real_map
    assert biome.shape == grid.shape == (6144, 7168)
    assert biome.max() <= 7
    assert set(np.unique(biome).tolist()) == set(range(8))


@pytest.mark.parametrize("place", sorted(PLACES))
def test_real_map_places(real_map: tuple, place: str) -> None:
    """Reference places carry their expected biome."""
    biome, grid = real_map
    lon, lat, expected = PLACES[place]
    x, y = grid.lonlat_to_pixel(lon, lat)
    assert biome[int(y), int(x)] == expected, place


def test_real_map_has_no_isolated_pixels(real_map: tuple) -> None:
    """No land pixel is alone in its biome among land neighbours (islets excepted)."""
    from scipy import ndimage

    biome, _ = real_map
    land = ndimage.uniform_filter((biome > 0).astype(np.float32), 3, mode="nearest")
    lonely = np.zeros(biome.shape, dtype=bool)
    for index in range(1, 8):
        mask = biome == index
        same = ndimage.uniform_filter(mask.astype(np.float32), 3, mode="nearest")
        # Seul de son biome (lui compris) avec au moins un voisin terrestre d'un autre.
        lonely |= mask & (same * 9.0 < 1.5) & ((land - same) * 9.0 > 0.5)
    assert int(lonely.sum()) == 0


# ---------------------------------------------------------------------------
# Colour map palettes by biome
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def style() -> dict:
    """The repository colour-map style."""
    return colormap.load_style()


def _cm_inputs(
    rows: int, cols: int, biome: int, splat_channel: int, **extra
) -> colormap.ColormapInputs:
    splat = np.zeros((rows, cols, 4), dtype=np.uint8)
    splat[..., splat_channel] = 255
    values = {
        "land": np.ones((rows, cols), dtype=bool),
        "splat": splat,
        "height_m": _grid(100.0, rows, cols),
        "coast_dist_px": _grid(60.0, rows, cols),
        "wetlands": np.zeros((rows, cols, 3), dtype=np.uint8),
        "conifer": _grid(0.0, rows, cols),
        "lon": _grid(23.0, rows, cols),
        "lat": _grid(38.0, rows, cols),
        "meters_per_px": MPP,
        "biomes": np.full((rows, cols), biome, dtype=np.uint8),
    }
    values.update(extra)
    return colormap.ColormapInputs(**values)


def _dist(image: np.ndarray, color: str) -> float:
    delta = image.astype(np.float32) / 255.0 - colormap.hex_color(color)
    return float(np.abs(delta).sum(-1).mean())


def test_mediterranean_palette_is_garrigue(style: dict) -> None:
    """Mediterranean (Attica, Peloponnese): scrub dominates, no dense broadleaf forest."""
    spec = style["biomes"]["classes"]["mediterranean"]
    assert spec["scrub"] >= 0.6 and spec["forest_keep"] <= 0.5
    image = colormap.bake(_cm_inputs(32, 32, MEDITERRANEAN, 2), style)
    scrub = spec["palette"]["scrub"]
    oak = style["palette"]["forest_broadleaf"]
    assert _dist(image, scrub) < _dist(image, oak)


def test_steppe_forests_only_in_valleys(style: dict) -> None:
    """Steppe forest weight opens into grass, except along rivers."""
    rows, cols = 32, 32
    river = _grid(50.0, rows, cols)
    river[:, :8] = 0.0
    inputs = _cm_inputs(rows, cols, STEPPE, 2, river_km=river, lat=_grid(48.0, 32, 32))
    image = colormap.bake(inputs, style)
    spec = style["biomes"]["classes"]["steppe"]["palette"]
    valley, open_land = image[:, :12], image[:, 40:]
    forest = spec["forest_broadleaf"]
    assert _dist(valley, forest) < _dist(open_land, forest)
    assert _dist(open_land, spec["grassland"]) < 0.25


def test_biome_bake_is_band_independent(style: dict) -> None:
    """Two biomes side by side: same bytes whatever the band height."""
    rows, cols = 48, 64
    biome = np.full((rows, cols), OCEANIC, dtype=np.uint8)
    biome[:, 32:] = MEDITERRANEAN
    settlements = np.array([[20.0, 20.0], [44.0, 30.0]])
    kwargs = {"biomes": biome, "settlements": settlements}
    first = colormap.bake(_cm_inputs(rows, cols, OCEANIC, 1, **kwargs), style, 32)
    second = colormap.bake(_cm_inputs(rows, cols, OCEANIC, 1, **kwargs), style, 17)
    assert first.tobytes() == second.tobytes()
