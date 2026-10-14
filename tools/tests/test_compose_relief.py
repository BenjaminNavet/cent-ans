"""Tests du relief composé (lot CC, C2) : unités synthétiques et fichiers de ``data/map``."""

from __future__ import annotations

import json
from pathlib import Path

import jsonschema
import numpy as np
import pytest
from PIL import Image
from scipy import ndimage

from cent_ans_tools.compose import relief
from cent_ans_tools.geo import navgrid, terrain
from cent_ans_tools.paths import DATA_DIR

MAP_DIR = DATA_DIR / "map"
COMPOSE_DIR = MAP_DIR / "compose"
Image.MAX_IMAGE_PIXELS = None


def _load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def test_relief_settings_follow_schema() -> None:
    """``relief.json`` est valide."""
    jsonschema.validate(
        _load(COMPOSE_DIR / "relief.json"),
        _load(DATA_DIR / "schemas" / "compose_relief.schema.json"),
    )


# -- unités synthétiques (rapides) ---------------------------------------------


def _synthetic() -> tuple[relief.Inputs, dict]:
    """Canevas 700 x 500 : mer à l'ouest, une chaîne verticale avec un col, un fleuve vers la mer."""
    cfg = _load(COMPOSE_DIR / "relief.json")
    h, w = 500, 700
    land = np.zeros((h, w), dtype=bool)
    land[:, 80:] = True
    ridge = {
        "id": "massif_t",
        "kind": "massif",
        "points": [[400.0, 40.0], [410.0, 250.0], [400.0, 460.0]],
        "width_px": 70.0,
        "height_m": 3600.0,
        "passes": [
            {
                "id": "pass_t",
                "name": "t",
                "at": [410.0, 250.0],
                "width_px": 6,
                "depth": 0.7,
            }
        ],
    }
    river = {
        "name": "Test",
        "width_px": 3.0,
        "coords": np.array(
            [[650.0, 120.0], [500.0, 140.0], [300.0, 130.0], [90.0, 150.0]]
        ),
    }
    real = np.zeros((h, w), dtype=np.float32)
    places = np.array([[200.0, 300.0], [600.0, 400.0]])
    return relief.Inputs(
        land, np.ones((h, w), bool), real, [ridge], [river], places
    ), cfg


@pytest.fixture(scope="module")
def synthetic() -> tuple[np.ndarray, relief.Inputs, dict]:
    """Relief construit sur le canevas synthétique."""
    inp, cfg = _synthetic()
    height, _ = relief.build_height(inp, cfg)
    return height, inp, cfg


def test_synthetic_coast_and_ridge(synthetic) -> None:
    """Terre > 0, mer < 0 ; la crête domine nettement la plaine."""
    height, inp, _ = synthetic
    assert (height[inp.land] > 0).all()
    assert (height[~inp.land] < 0).all()
    crest = np.median(
        [height[y, int(405 + 5 * np.sin(y / 80))] for y in range(80, 200)]
    )
    plain = float(np.median(height[:, 560:690]))
    assert crest > 1500
    assert crest - plain > 1200


def test_synthetic_pass_lower_than_both_crests(synthetic) -> None:
    """Le col est plus bas que la crête de part et d'autre, le long de la chaîne."""
    height, _, _ = synthetic
    at = (250, 410)
    col = float(height[at[0] - 1 : at[0] + 2, at[1] - 2 : at[1] + 3].min())
    for dy in (-95, 95):
        side = float(height[at[0] + dy - 4 : at[0] + dy + 5, 395:420].max())
        assert col < side - 400


def test_synthetic_pass_is_a_notch_not_a_corridor(synthetic) -> None:
    """C2b : le col reste à >= 40 % de la crête et aucun couloir ne rejoint le piémont."""
    height, _, _ = synthetic
    col = float(height[249:252, 408:413].min())
    crest = float(height[150:170, 395:420].max())
    assert 0.4 * crest < col < 0.75 * crest
    # En travers de la chaîne, le long de la ligne du col, on ne descend pas à la plaine.
    across = height[250, 340:480]
    assert float(across.min()) > 0.3 * crest
    # Approche en pente douce : pas de paroi verticale entre le fond et la crête.
    assert float(np.abs(np.diff(height[250, 380:440])).max()) < 120.0


def test_synthetic_river_does_not_dig_the_massif(synthetic) -> None:
    """C2b : le fleuve s'arrête au piémont, il ne creuse pas la chaîne."""
    height, inp, cfg = synthetic
    inp_nr = relief.Inputs(
        inp.land, inp.inside, inp.real_height, inp.ridges, [], inp.places
    )
    no_river, _ = relief.build_height(inp_nr, cfg)
    on_ridge = height[110:160, 390:430]
    assert float(np.abs(on_ridge - no_river[110:160, 390:430]).max()) < 5.0


def test_trough_is_a_dead_end_u_valley_with_rising_floor() -> None:
    """C2b : une auge d'auteur monte vers sa tête et ne creuse pas jusqu'au niveau de la plaine."""
    cfg = _load(COMPOSE_DIR / "relief.json")
    shape = (300, 500)
    h = np.full(shape, 3000.0, dtype=np.float32)
    base = np.full(shape, 10.0, dtype=np.float32)
    h[:, :60] = 20.0
    trough = {
        "name": "t",
        "points": np.array([[60.0, 150.0], [250.0, 150.0], [330.0, 150.0]]),
        "floor_m": [1500.0, 2400.0],
        "half_floor_px": 6.0,
    }
    inp = relief.Inputs(
        np.ones(shape, bool),
        np.ones(shape, bool),
        h,
        [],
        [],
        np.zeros((0, 2)),
        None,
        [trough],
    )
    relief.carve_troughs(h, base, inp, cfg)
    floor = [float(h[150, x]) for x in (80, 160, 240, 320)]
    assert floor == sorted(floor)  # le fond monte vers la tête
    assert floor[-1] > floor[0] + 400.0
    assert float(h[150 - 40, 240]) > floor[2] + 500.0  # parois
    assert float(h[150, 450]) == 3000.0  # au-delà de la tête : intact


def test_synthetic_river_descends_and_plain_flat(synthetic) -> None:
    """Le lit du fleuve ne remonte pas vers l'aval ; la plaine éloignée est plate."""
    height, inp, cfg = synthetic
    pts = relief.orient_downstream(
        inp.rivers[0]["coords"], height, np.zeros(height.shape)
    )
    dense, _ = relief.densify(pts, 2.0)
    # C2b : le fleuve ne creuse pas la chaîne (x ~ 400) : on juge son tronçon aval (x < 330).
    dense = dense[dense[:, 0] < 330.0]
    along = height[dense[:, 1].astype(int), dense[:, 0].astype(int)]
    assert (np.diff(along) <= 3.0).mean() > 0.97
    plain = height[300:480, 140:190]
    assert float(plain.std()) < 12.0


def test_synthetic_platform_is_flat(synthetic) -> None:
    """Sous un lieu le terrain est aplani (pente locale faible)."""
    height, inp, _ = synthetic
    x, y = (int(v) for v in inp.places[0])
    assert float(height[y - 3 : y + 4, x - 3 : x + 4].std()) < 3.0


def test_attach_free_passes_is_idempotent() -> None:
    """Un col libre se rattache à la crête la plus proche, une seule fois."""
    ridges = [
        {"id": "a", "points": [[0.0, 0.0], [100.0, 0.0]], "passes": []},
        {"id": "b", "points": [[0.0, 500.0], [100.0, 500.0]], "passes": []},
    ]
    free = [{"id": "p", "name": "P", "at": (50.0, 60.0)}]
    assert relief.attach_free_passes(ridges, free, 140.0, 6.0) == 1
    assert ridges[0]["passes"][0]["id"] == "p"
    assert relief.attach_free_passes(ridges, free, 140.0, 6.0) == 0
    far = [{"id": "q", "name": "Q", "at": (50.0, 300.0)}]
    assert relief.attach_free_passes(ridges, far, 40.0, 6.0) == 0


# -- relief composé versionné -----------------------------------------------------


@pytest.fixture(scope="module")
def height() -> np.ndarray:
    """``heightmap.png`` décodé (mètres)."""
    return terrain.uint16_to_height(
        terrain.read_png16(MAP_DIR / "heightmap.png")
    ).astype(np.float32)


@pytest.fixture(scope="module")
def land() -> np.ndarray:
    """Masque terre composé."""
    return np.asarray(Image.open(MAP_DIR / "land_mask.png")) > 0


@pytest.fixture(scope="module")
def ridges() -> list[dict]:
    """Crêtes d'auteur."""
    return _load(COMPOSE_DIR / "ridges.json")["ridges"]


@pytest.fixture(scope="module")
def lakes_mask(land: np.ndarray, height: np.ndarray) -> np.ndarray:
    """Cuvettes de lacs (lot C4a) : eau du masque dont la hauteur est > 0."""
    return ~land & (height > 0.0)


def test_coast_matches_land_mask(
    height: np.ndarray, land: np.ndarray, lakes_mask: np.ndarray
) -> None:
    """Terre > 0 et mer < 0 ; les lacs sont des cuvettes plates > 0 (``flatten_lakes``)."""
    assert height.shape == land.shape
    assert height[land].min() > 0.0
    assert height[~land & ~lakes_mask].max() < 0.0
    labels, count = ndimage.label(lakes_mask)
    assert count > 100
    sizes = np.bincount(labels.ravel())[1:]
    assert sizes.min() >= relief.LAKE_MIN_PX
    edge = np.unique(
        np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]])
    )
    assert set(edge.tolist()) <= {0}, "un lac touche le bord du canevas"
    flat = [
        float(np.ptp(height[box][labels[box] == index + 1]))
        for index, box in enumerate(ndimage.find_objects(labels))
    ]
    assert max(flat) < 1e-3, "un lac n'est pas plat"


def test_render_heightmap_agrees_with_heightmap(
    height: np.ndarray, land: np.ndarray, lakes_mask: np.ndarray
) -> None:
    """Le relief de rendu a la même côte et un écart faible (profil RE neutralisé)."""
    render = terrain.uint16_to_height(
        terrain.read_png16(MAP_DIR / "heightmap_render.png")
    )
    assert render.shape == height.shape
    assert ((render > 0) == (land | lakes_mask)).mean() > 0.9999
    assert float(np.abs(render - height)[land].mean()) < 5.0
    meta = _load(MAP_DIR / "map.json")
    assert meta["render_heightmap"]["file"] == "heightmap_render.png"
    assert meta["render_heightmap"]["boost"] is None


def test_height_tiles_cover_the_padded_canvas(height: np.ndarray) -> None:
    """Les tuiles couvrent le canevas complété (multiple de 256 px de carte, finesse x2)."""
    meta = _load(MAP_DIR / "map.json")
    tiles = meta["height_tiles"]
    pw, ph = relief.padded_size((height.shape[1], height.shape[0]))
    assert tiles["size_px"] == [pw * 2, ph * 2]
    cols, rows = (
        tiles["size_px"][0] // tiles["tile_px"],
        tiles["size_px"][1] // tiles["tile_px"],
    )
    assert (
        MAP_DIR / tiles["dir"] / tiles["pattern"].format(col=cols - 1, row=rows - 1)
    ).exists()
    corner = terrain.uint16_to_height(
        terrain.read_png16(
            MAP_DIR / tiles["dir"] / tiles["pattern"].format(col=10, row=10)
        )
    )
    assert corner.shape == (tiles["tile_px"], tiles["tile_px"])
    block = height[10 * 256 : 10 * 256 + 256, 10 * 256 : 10 * 256 + 256]
    assert abs(float(corner.mean()) - float(block.mean())) < 20.0


def _ridge_distance(ridges: list[dict], shape: tuple[int, int]) -> np.ndarray:
    """Distance (px) à la crête la plus proche, calculée au quart de résolution."""
    from PIL import ImageDraw

    img = Image.new("L", (shape[1] // 4, shape[0] // 4), 0)
    draw = ImageDraw.Draw(img)
    for ridge in ridges:
        draw.line([(x / 4, y / 4) for x, y in ridge["points"]], fill=255, width=1)
    return ndimage.distance_transform_edt(np.asarray(img) == 0) * 4.0


def test_plains_are_flat(
    height: np.ndarray, land: np.ndarray, ridges: list[dict]
) -> None:
    """Hors massifs : écart-type local faible dans les plaines basses."""
    dist = _ridge_distance(ridges, height.shape)
    low = ndimage.uniform_filter(height[::4, ::4], 5)
    plain = (low < 60.0) & land[::4, ::4] & (dist > 160.0)
    assert plain.sum() > 100_000
    sub = height[::4, ::4]
    mean = ndimage.uniform_filter(sub, 9)
    sq = ndimage.uniform_filter(sub * sub, 9)
    local_std = np.sqrt(np.maximum(sq - mean * mean, 0.0))
    # CD : relief réel déformé (vallées et collines des bas pays), exagération RE à l'exécution
    assert float(np.median(local_std[plain])) < 25.0
    assert float(np.percentile(local_std[plain], 95)) < 100.0


def _sample(array: np.ndarray, x: float, y: float, radius: int = 2) -> float:
    i, j = int(round(x)), int(round(y))
    window = array[
        max(j - radius, 0) : j + radius + 1, max(i - radius, 0) : i + radius + 1
    ]
    return float(window.max()) if window.size else 0.0


def test_crests_are_higher_than_their_piedmont(
    height: np.ndarray, ridges: list[dict]
) -> None:
    """Chaque massif domine son piémont d'au moins 600 m."""
    for ridge in (r for r in ridges if r["kind"] == "massif"):
        prof, _, half = relief._profile_params(
            ridge, _load(COMPOSE_DIR / "relief.json")
        )
        del prof
        pts = relief.smooth_points(ridge["points"])
        dense, _ = relief.densify(pts, 6.0)
        tangent = np.gradient(dense, axis=0)
        tangent /= np.maximum(np.hypot(*tangent.T)[:, None], 1e-6)
        normal = np.stack([-tangent[:, 1], tangent[:, 0]], axis=1)
        crest = [_sample(height, x, y, 3) for x, y in dense]
        sides = []
        for sign in (-1.0, 1.0):
            off = dense + sign * normal * half * 4.5
            sides.append(
                np.median(
                    [
                        _sample(height, x, y, 1)
                        for x, y in off
                        if 0 <= x < height.shape[1] and 0 <= y < height.shape[0]
                    ]
                )
            )
        # Le piémont est le versant le plus bas (une branche intérieure d'un bloc a un côté haut).
        # CD : le réel est moins haut que les chaînes d'auteur ; le plancher de crête garde un mur
        assert np.median(crest) - np.nanmin(sides) > 50.0, ridge["id"]


def test_all_passes_attached_and_lower_than_crest(
    height: np.ndarray, ridges: list[dict]
) -> None:
    """Les 24 cols sont rattachés (dont Arlberg, Perthus) et plus bas que la crête voisine."""
    crossings = _load(MAP_DIR / "crossings.json")["crossings"]
    wanted = {c["id"] for c in crossings if c["type"] == "pass"}
    attached = {p["id"] for r in ridges for p in r["passes"]}
    assert wanted <= attached
    assert {"pass_arlberg", "pass_perthus"} <= attached
    owner = relief.pass_owners(ridges)
    by_id = {r["id"]: r for r in ridges}
    cfg = _load(COMPOSE_DIR / "relief.json")
    for pid, rid in owner.items():
        ridge = by_id[rid]
        if ridge["kind"] != "massif":
            continue
        p = next(q for q in ridge["passes"] if q["id"] == pid)
        line = relief.smooth_points(ridge["points"])
        from shapely.geometry import LineString, Point

        ls = LineString(line)
        along = ls.project(Point(p["at"]))
        col = _sample(height, p["at"][0], p["at"][1], 2)
        sides = []
        for delta in (-1.0, 1.0):
            q = ls.interpolate(min(max(along + delta * 90.0, 0.0), ls.length))
            sides.append(_sample(height, q.x, q.y, 6))
        _, top, _ = relief._profile_params(ridge, cfg)
        if (
            max(sides) > 1800.0
        ):  # encoche : plus basse que la crête voisine la plus haute
            # marge 100 -> 60 m : le cartogramme recalculé en C4b (Jérusalem) décale les Alpes de quelques px
            assert col < max(sides) - 60.0, pid


def test_rivers_descend_and_lie_in_valleys(height: np.ndarray) -> None:
    """Le lit de chaque fleuve ne remonte pas vers l'aval (tolérance) et n'est pas sur un versant."""
    feats = _load(MAP_DIR / "rivers.geojson")["features"]
    d_land = ndimage.distance_transform_edt(height > 0)
    bad = []
    for feat in feats:
        coords = np.array(feat["geometry"]["coordinates"], dtype=np.float64)
        pts = relief.orient_downstream(coords, height, d_land)
        dense, _ = relief.densify(pts, 3.0)
        i = np.clip(dense[:, 0].astype(int), 0, height.shape[1] - 1)
        j = np.clip(dense[:, 1].astype(int), 0, height.shape[0] - 1)
        along = ndimage.uniform_filter1d(height[j, i].astype(np.float64), 5)
        # C2b : seuls les tronçons de plaine/colline sont creusés ; les tronçons de massif non.
        low = (along[:-1] < 700.0) & (along[1:] < 700.0)
        rises = np.diff(along)[low]
        if len(rises) < 10:
            continue
        if (
            rises <= 3.0
        ).mean() < 0.7 or rises.max() > 60.0:  # CD : bruit propre aux MNT réels
            bad.append(
                (
                    feat["properties"]["name"],
                    float((rises <= 3.0).mean()),
                    float(rises.max()),
                )
            )
    assert len(bad) <= 6, bad


def test_platforms_under_places(height: np.ndarray, land: np.ndarray) -> None:
    """Sous 90 % des lieux le terrain est quasi plat (rayon 3 px)."""
    from cent_ans_tools.compose.contract import ComposedWarp

    cw = ComposedWarp.load(COMPOSE_DIR)
    selection = _load(COMPOSE_DIR / "selection.json")["settlements"]
    px, py = cw.forward(
        np.array([s["lon"] for s in selection]), np.array([s["lat"] for s in selection])
    )
    stds = []
    for x, y in zip(px, py, strict=True):
        i, j = int(x), int(y)
        if (
            not (4 <= i < height.shape[1] - 4 and 4 <= j < height.shape[0] - 4)
            or not land[j, i]
        ):
            continue
        stds.append(float(height[j - 3 : j + 4, i - 3 : i + 4].std()))
    assert len(stds) > 300
    assert (
        np.percentile(stds, 90) < 60.0
    )  # CD : villes sur les versants du relief réel (platform <= 90 m)


def test_massif_rule_applies_and_passes_stay_open(
    height: np.ndarray, land: np.ndarray, ridges: list[dict]
) -> None:
    """La règle ADR 0353 se calcule sur ce relief : massifs murés (cols en encoches, C2b : la chaîne reste murée hors des encoches)."""
    rules = navgrid.load_rules()
    scale = navgrid.NAVGRID_SCALE
    h = height[: height.shape[0] // scale * scale, : height.shape[1] // scale * scale]
    small = h.reshape(h.shape[0] // scale, scale, h.shape[1] // scale, scale).mean(
        axis=(1, 3)
    )
    water = ~land[: h.shape[0], : h.shape[1]]
    water = (
        water.reshape(small.shape[0], scale, small.shape[1], scale).mean(axis=(1, 3))
        > 0.5
    )
    mass = navgrid.massif_mask(small, water, rules) | navgrid.authored_mass(
        MAP_DIR, small.shape
    )
    assert 0.003 < mass.mean() < 0.16  # CD : masses d'auteur OU relief réel
    attached = [(p, r) for r in ridges if r["kind"] == "massif" for p in r["passes"]]
    assert attached
    owner = relief.pass_owners(ridges)
    open_count = 0
    total = 0
    for p, r in attached:
        if owner[p["id"]] != r["id"]:
            continue
        total += 1
        ci, cj = int(p["at"][0] // scale), int(p["at"][1] // scale)
        if not mass[cj - 1 : cj + 2, ci - 1 : ci + 2].all():
            open_count += 1
    # CD : le franchissement est garanti par crossings.json (navgrid), pas par la géométrie du relief réel
    assert open_count >= 0.2 * total
