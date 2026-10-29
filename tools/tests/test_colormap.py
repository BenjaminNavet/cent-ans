"""Campaign ground colour map (chantier SS, lot SS1): painter, style, BC1 chain."""

from __future__ import annotations

import colorsys
import json
import zlib
from pathlib import Path

import numpy as np
import pytest
import yaml
from PIL import Image

from cent_ans_tools.codex import schema_validator
from cent_ans_tools.geo import block_compress, colormap

MPP = 718.9765625


@pytest.fixture(scope="module")
def style() -> dict:
    """The repository style (validated by :func:`colormap.load_style`)."""
    return colormap.load_style()


def _inputs(rows: int = 48, cols: int = 64, **overrides) -> colormap.ColormapInputs:
    """Flat farmland at 100 m, 47° N, everything else empty."""
    splat = np.zeros((rows, cols, 4), dtype=np.uint8)
    splat[..., 1] = 255
    values = {
        "land": np.ones((rows, cols), dtype=bool),
        "splat": splat,
        "height_m": np.full((rows, cols), 100.0, dtype=np.float32),
        "coast_dist_px": np.full((rows, cols), 60.0, dtype=np.float32),
        "wetlands": np.zeros((rows, cols, 3), dtype=np.uint8),
        "conifer": np.zeros((rows, cols), dtype=np.float32),
        "lon": np.full((rows, cols), 2.0, dtype=np.float32),
        "lat": np.full((rows, cols), 47.0, dtype=np.float32),
        "meters_per_px": MPP,
    }
    values.update(overrides)
    return colormap.ColormapInputs(**values)


def _distance(image: np.ndarray, color: str) -> np.ndarray:
    return np.abs(image.astype(np.float32) / 255.0 - colormap.hex_color(color)).sum(-1)


def test_repository_style_matches_schema() -> None:
    """``colormap_style.yaml`` validates against ``colormap_style.schema.json``."""
    validator = schema_validator(
        colormap.SCHEMA_PATH.parents[1], colormap.SCHEMA_PATH.name
    )
    document = yaml.safe_load(colormap.STYLE_PATH.read_text(encoding="utf-8"))
    errors = list(validator.iter_errors(document))
    assert not errors, [error.message for error in errors]


def test_invalid_style_is_rejected(tmp_path: Path) -> None:
    """A colour that is not ``#rrggbb`` fails validation."""
    document = yaml.safe_load(colormap.STYLE_PATH.read_text(encoding="utf-8"))
    document["palette"]["road"] = "beige"
    path = tmp_path / "style.yaml"
    path.write_text(yaml.safe_dump(document), encoding="utf-8")
    with pytest.raises(ValueError, match="palette/road"):
        colormap.load_style(path)


def test_bake_is_deterministic_and_band_independent(style: dict) -> None:
    """Same inputs, same bytes, whatever the band height."""
    settlements = np.array([[20.0, 20.0], [40.0, 30.0]])
    roads = [("main", np.array([[2.0, 10.0], [60.0, 14.0]]))]
    towns = [(30.0, 30.0, 40.0)]
    first = colormap.bake(
        _inputs(settlements=settlements, roads=roads, towns=towns), style, 32
    )
    second = colormap.bake(
        _inputs(settlements=settlements, roads=roads, towns=towns), style, 17
    )
    assert first.shape == (96, 128, 3)
    assert first.tobytes() == second.tobytes()


def test_road_is_painted_on_its_line(style: dict) -> None:
    """A horizontal road at y = 24.25 map px paints texture row 48 in dirt colour."""
    rows, cols = 48, 64
    road = [("main", np.array([[4.0, 24.25], [60.0, 24.25]]))]
    with_road = colormap.bake(_inputs(rows, cols, roads=road), style)
    without = colormap.bake(_inputs(rows, cols), style)
    color = style["palette"]["road"]
    on_line = _distance(with_road[48, 20:100], color)
    before = _distance(without[48, 20:100], color)
    far = _distance(with_road[20, 20:100], color)
    assert on_line.mean() < 0.5 * before.mean()
    assert on_line.mean() < 0.5 * far.mean()
    # Loin du tracé, rien ne change.
    assert np.array_equal(with_road[:40], without[:40])


def test_lake_has_a_shore_and_the_sea_does_not(style: dict) -> None:
    """Inland water gets a shore ring; water touching the map edge (sea) does not."""
    rows, cols = 48, 64
    land = np.ones((rows, cols), dtype=bool)
    land[:, :6] = False  # mer à l'ouest (touche le bord)
    land[20:28, 36:46] = False  # lac intérieur
    coast = np.full((rows, cols), 60.0, dtype=np.float32)
    coast[:, :12] = np.arange(12, dtype=np.float32) - 5.5
    image = colormap.bake(_inputs(rows, cols, land=land, coast_dist_px=coast), style)
    shore = style["palette"]["lake_shore"]
    # Anneau d'un texel autour du lac (texture = 2 x carte, lac en 40..56 x 72..92).
    ring = np.zeros((rows * 2, cols * 2), dtype=bool)
    ring[39:57, 71] = ring[39:57, 92] = True
    ring[39, 71:93] = ring[56, 71:93] = True
    sea_edge = np.zeros_like(ring)
    sea_edge[:, 12] = True
    assert _distance(image[ring], shore).mean() < 0.25
    assert _distance(image[sea_edge], shore).mean() > 0.35
    # L'eau du lac n'est pas peinte en terre.
    centre = image[46:50, 80:84]
    assert (
        _distance(centre, style["palette"]["lake_deep"]).mean()
        < _distance(centre, style["palette"]["farmland"]).mean()
    )


def test_modern_reservoir_is_painted_as_land() -> None:
    """A lake holding a ``modern_reservoirs`` point is not a lake."""
    land = np.ones((48, 64), dtype=bool)
    land[20:28, 36:46] = False
    lake, reservoir = colormap.classify_water(land, np.array([[40.0, 24.0]]), 10_000)
    assert not lake.any()
    assert reservoir.sum() == 80
    lake, reservoir = colormap.classify_water(land, np.zeros((0, 2)), 10_000)
    assert lake.sum() == 80 and not reservoir.any()


def test_farm_blocks_near_settlement_not_on_rock(style: dict) -> None:
    """Farmland near a village is a patchwork of crop colours; bare rock is not."""
    rows, cols = 64, 64
    settlements = np.array([[16.0, 32.0], [48.0, 32.0]])
    splat = np.zeros((rows, cols, 4), dtype=np.uint8)
    splat[:, :32, 1] = 255  # cultures à l'ouest
    splat[:, 32:, 3] = 255  # roche à l'est
    height = np.full((rows, cols), 100.0, dtype=np.float32)
    height[:, 32:] = 2000.0
    image = colormap.bake(
        _inputs(rows, cols, splat=splat, height_m=height, settlements=settlements),
        style,
    )
    mosaic = style["mosaic"]
    crops = [e["color"] for e in mosaic["crops"] + mosaic["pastures"]]

    def crop_share(patch: np.ndarray) -> float:
        nearest = np.min([_distance(patch, c) for c in crops], axis=0)
        return float((nearest < 0.12).mean())

    farm = image[40:88, 8:56]
    rock = image[40:88, 72:90]  # clear of the village ring of the eastern place
    assert crop_share(farm) > 0.4
    # Contraste de bloc à bloc sur les cultures, aplat sur la roche.
    farm_std = farm.reshape(-1, 3).std(axis=0).mean()
    rock_std = rock.reshape(-1, 3).std(axis=0).mean()
    assert farm_std > 3 * rock_std


def test_bc1_chain_size_matches_godot(tmp_path: Path) -> None:
    """BC1 + mipmaps has the size ``Image.create_from_data`` expects; parts add up."""
    rng = np.random.default_rng(3)
    image = rng.integers(0, 255, size=(40, 56, 3), dtype=np.uint8)
    (tmp_path / "map.json").write_text("{}", encoding="utf-8")
    paths, meta = colormap.write_colormap(image, tmp_path)
    expected = 0
    width, height = 56, 40
    while True:
        expected += ((width + 3) // 4) * ((height + 3) // 4) * 8
        if (width, height) == (1, 1):
            break
        width, height = max(1, width // 2), max(1, height // 2)
    assert meta["bytes"] == expected == sum(meta["part_bytes"])
    assert meta["format"] == "dxt1" and meta["mipmaps"] is True
    assert meta["size_px"] == [56, 40]
    raw = b"".join(zlib.decompress(p.read_bytes()) for p in paths)
    assert len(raw) == expected
    level0 = block_compress.decode_bc1(raw[: 14 * 10 * 8], 40, 56)
    assert level0.shape == (40, 56, 3)
    block_compress.update_map_json(tmp_path, colormap.MAP_KEY, meta)
    stored = json.loads((tmp_path / "map.json").read_text(encoding="utf-8"))
    assert stored["colormap"]["bc1"]["pattern"] == "colormap_bc1_{part}.bin"


def test_full_size_chain_bytes() -> None:
    """The real 14336 x 12288 texture: 8 bytes per 4 x 4 block over the whole chain."""
    assert colormap.mip_chain_bytes(14336, 12288, 8) == 117_440_544


def test_preview_width(tmp_path: Path) -> None:
    """The preview is resized to the requested width."""
    image = np.zeros((96, 128, 3), dtype=np.uint8)
    path = colormap.write_preview(image, tmp_path / "p.jpg", width=24)
    with Image.open(path) as preview:
        assert preview.size == (24, 18)


def _varied_inputs(rows: int = 70, cols: int = 90) -> colormap.ColormapInputs:
    """Hills, a lake, sea on the left, biomes, rivers, forests: every whole-map field is exercised."""
    rng = np.random.default_rng(4)
    yy, xx = np.mgrid[0:rows, 0:cols]
    splat = np.zeros((rows, cols, 4), dtype=np.uint8)
    farm = 140 + (60 * np.sin(xx / 6.0)).astype(np.int64)
    splat[..., 1] = farm
    splat[..., 2] = 255 - farm
    land = xx > 14
    land &= ((yy - 35) ** 2 + (xx - 55) ** 2) > 49  # an inland lake
    height = (300 + 900 * np.sin(xx / 17.0) * np.cos(yy / 13.0)).astype(np.float32)
    biomes = np.where(xx < 45, 1, 3).astype(np.uint8)
    biomes[~land & (xx <= 14)] = 0
    return _inputs(
        rows,
        cols,
        land=land,
        splat=splat,
        height_m=height,
        coast_dist_px=np.where(land, 20.0, -20.0).astype(np.float32),
        conifer=rng.uniform(0, 1, (rows, cols)).astype(np.float32),
        lon=(2.0 + xx * 0.01).astype(np.float32),
        lat=(44.0 + yy * 0.02).astype(np.float32),
        settlements=np.array([[20.0, 20.0], [70.0, 50.0]]),
        roads=[("main", np.array([[16.0, 10.0], [88.0, 60.0]]))],
        towns=[(70.0, 50.0, 60.0)],
        biomes=biomes,
        river_km=np.abs(yy - 30).astype(np.float32),
    )


def test_tiled_bake_equals_single_band(style: dict) -> None:
    """Lot CC CS: tiling (any band height, any halo cut) never changes a byte of the colour map."""
    whole = colormap.bake(_varied_inputs(), style, 1024)
    for band_rows in (24, 50, 70):
        tiled = colormap.bake(_varied_inputs(), style, band_rows)
        np.testing.assert_array_equal(tiled, whole)


# --- VIS-R3a : hiérarchie de valeur, classes régionales, noyaux villageois ----------------------

#: Biomes where fields sit on a green meadow background: forest < pré < labour < chaume holds
#: strictly. The dry biomes keep a bright, burnt pasture and only need the forests to stay darkest.
DRY_BIOMES = {"mediterranean", "steppe", "semi_arid", "desert", "aegean_maquis"}
LABOUR = {"labour", "jachere", "essart"}
CHAUME = {"ble", "orge", "seigle_avoine", "seigle", "avoine", "millet"}


def _luma(color: np.ndarray) -> float:
    """Luma (Rec. 709 weights on sRGB values, as the visual audit measures it)."""
    return float(0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2])


def _class_means(spec: dict, palette: dict, mosaic: dict) -> dict[str, float | None]:
    """Mean luma of forest, pré, labour, chaume for a biome spec, crop contrast applied."""
    pal = {**palette, **spec.get("palette", {})}
    forest = np.mean(
        [
            _luma(colormap.hex_color(pal[k]))
            for k in ("forest_broadleaf", "forest_conifer")
        ]
    )
    crops = spec.get("crops", mosaic["crops"])
    pastures = spec.get("pastures", mosaic["pastures"])
    crop_colors, _ = colormap._weighted(crops, mosaic["crop_contrast"])
    pasture_colors, _ = colormap._weighted(pastures, mosaic["pasture_contrast"])

    def mean(names: set[str]) -> float | None:
        picked = [
            (e["weight"], _luma(c))
            for e, c in zip(crops, crop_colors, strict=False)
            if e["name"] in names
        ]
        if not picked:
            return None
        return sum(w * v for w, v in picked) / sum(w for w, _ in picked)

    meadow = [
        (e["weight"], _luma(c)) for e, c in zip(pastures, pasture_colors, strict=False)
    ] + [
        (e["weight"], _luma(c))
        for e, c in zip(crops, crop_colors, strict=False)
        if e["name"] == "pre"
    ]
    return {
        "forest": float(forest),
        "pre": sum(w * v for w, v in meadow) / sum(w for w, _ in meadow),
        "labour": mean(LABOUR),
        "chaume": mean(CHAUME),
    }


def test_value_hierarchy_forest_meadow_plough_stubble(style: dict) -> None:
    """Mean luma of the classes is strictly ordered: forêt < pré < labour < chaume (bible, VIS-R3a)."""
    specs = {"default": {}, **style["biomes"]["classes"]}
    for name in style["regional"]:
        specs[name] = colormap.regional_spec(style, name)
    for name, spec in specs.items():
        means = _class_means(spec, style["palette"], style["mosaic"])
        open_land = [v for k, v in means.items() if k != "forest" and v is not None]
        assert means["forest"] < min(open_land) - 0.05, (name, means)
        if name in DRY_BIOMES or spec.get("base") in DRY_BIOMES:
            continue
        ordered = [
            means[k]
            for k in ("forest", "pre", "labour", "chaume")
            if means[k] is not None
        ]
        assert all(a < b for a, b in zip(ordered, ordered[1:], strict=False)), (
            name,
            means,
        )


def test_global_palette_ladder_and_dirt_roads(style: dict) -> None:
    """Regional base: conifer < broadleaf < grassland < farmland; roads are a dirt tone, not a ribbon."""
    luma = {k: _luma(colormap.hex_color(v)) for k, v in style["palette"].items()}
    assert (
        luma["forest_conifer"]
        < luma["forest_broadleaf"]
        < luma["grassland"]
        < luma["farmland"]
    )
    road = colormap.hex_color(style["palette"]["road"])
    assert np.abs(road - np.array([0.55, 0.48, 0.36])).max() < 0.02


def test_regional_class_replaces_biomes_where_its_mask_is_full(style: dict) -> None:
    """A regional mask takes the weight of the biomes inside it; the weights still sum to 1."""
    biomes = np.full((512, 512), 1, dtype=np.uint8)
    step = style["biomes"]["weight_step"]
    mask = np.zeros(biomes[::step, ::step].shape, dtype=np.float32)
    mid = mask.shape[0] // 2
    mask[mid - 30 : mid + 30, mid - 30 : mid + 30] = 1.0
    specs, weights, _ = colormap.biome_weights(biomes, style, MPP, [("beauce", mask)])
    assert specs[-1]["name"] == "beauce"
    assert np.allclose(weights.sum(axis=0), 1.0, atol=1e-4)
    assert weights[-1][mid, mid] > 0.9
    assert weights[-1][0, 0] < 0.05
    assert specs[-1]["crops"][0]["name"] == "ble"
    assert specs[-1]["openfield"] > 1.0


def test_regional_palettes_are_distinct(style: dict) -> None:
    """Meseta is ochre-yellow, bocage deep green, Landes dark: distinct land colours."""
    hsv = {}
    for name in ("bocage", "beauce", "meseta", "landes"):
        spec = colormap.regional_spec(style, name)
        hsv[name] = colorsys.rgb_to_hsv(
            *colormap.hex_color(spec["palette"]["grassland"])
        )
    assert hsv["meseta"][1] > 0.35
    assert 0.10 < hsv["meseta"][0] < 0.19
    assert hsv["bocage"][0] > hsv["meseta"][0] + 0.03
    landes = colormap.hex_color(style["regional"]["landes"]["palette"]["grassland"])
    meseta = colormap.hex_color(style["regional"]["meseta"]["palette"]["grassland"])
    assert _luma(landes) < _luma(meseta)


def test_village_kernel_is_painted_around_an_indexed_place(style: dict) -> None:
    """An indexed place paints a warm core and a ring of gardens and orchards, locally."""
    rows, cols = 48, 64
    inputs = _inputs(rows, cols, settlements=np.array([[32.0, 24.0]]))
    plain = {k: v for k, v in style.items() if k != "villages"}
    base = colormap.bake(inputs, plain)
    village = colormap.bake(inputs, style)
    changed = np.abs(village.astype(int) - base.astype(int)).sum(-1) > 0
    ys, xs = np.nonzero(changed)
    assert len(ys) > 10
    # The change stays within the ring of the place (texture px = 2 x map px).
    assert np.hypot(xs - 64, ys - 48).max() < 12
    core = village[47:50, 63:66].astype(np.float32) / 255.0
    core_color = colormap.hex_color(style["palette"]["village_core"])
    assert np.abs(core - core_color).sum(-1).min() < 0.45
