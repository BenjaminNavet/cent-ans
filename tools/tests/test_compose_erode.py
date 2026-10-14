"""Tests de l'érosion du relief composé (lot CC CD) sur de petites grilles synthétiques."""

from __future__ import annotations

import json

import jsonschema
import numpy as np

from cent_ans_tools.compose import erode
from cent_ans_tools.paths import DATA_DIR

COMPOSE_DIR = DATA_DIR / "map" / "compose"


def _cfg() -> dict:
    cfg = json.loads((COMPOSE_DIR / "erode.json").read_text("utf-8"))
    cfg["erosion"]["iterations"] = 24
    cfg["erosion"]["block_px"] = 320
    cfg["erosion"]["halo_px"] = 32
    return cfg


def _massif(size: int = 96) -> tuple[np.ndarray, np.ndarray]:
    """Plaine à l'ouest (mer au bord), massif lisse à l'est (hauteur max 3200 m)."""
    y, x = np.mgrid[0:size, 0:size].astype(np.float32)
    ridge = (
        3200.0
        * np.exp(-(((x - 0.7 * size) / 18.0) ** 2))
        * (0.8 + 0.2 * np.sin(y / 9.0))
    )
    plain = 20.0 + 0.2 * x
    h = (plain + ridge).astype(np.float32)
    land = np.ones(h.shape, dtype=bool)
    h[:, :6] = -30.0
    land[:, :6] = False
    return h, land


def test_erode_settings_follow_schema() -> None:
    """``erode.json`` est valide."""
    jsonschema.validate(
        json.loads((COMPOSE_DIR / "erode.json").read_text("utf-8")),
        json.loads(
            (DATA_DIR / "schemas" / "compose_erode.schema.json").read_text("utf-8")
        ),
    )


def test_priority_flood_drains_every_pit() -> None:
    """Après remplissage, aucune cuvette : chaque pixel intérieur a un voisin plus bas."""
    rng = np.random.default_rng(1)
    h = rng.uniform(0, 50, (60, 60)).astype(np.float32)
    seeds = np.zeros(h.shape, dtype=bool)
    seeds[0, :] = seeds[-1, :] = seeds[:, 0] = seeds[:, -1] = True
    filled = erode.priority_flood(h, seeds, 0.05)
    assert (filled >= h).all()
    rec = erode._receivers(filled, seeds)
    inner = rec.reshape(60, 60)[1:-1, 1:-1]
    assert (inner >= 0).all()


def test_global_noise_is_seamless_between_windows() -> None:
    """Le bruit est une fonction des pixels globaux : deux fenêtres se recollent à l'identique."""
    whole = erode.value_noise_global(40, 80, 100, 50, 9.0, 3)
    left = erode.value_noise_global(40, 30, 100, 50, 9.0, 3)
    right = erode.value_noise_global(40, 50, 130, 50, 9.0, 3)
    assert np.array_equal(whole[:, :30], left)
    assert np.array_equal(whole[:, 30:], right)


def test_erosion_carves_ravines_and_keeps_sea_and_plain() -> None:
    """Le massif gagne du relief fin ; la mer et la plaine éloignée restent intactes."""
    cfg = _cfg()
    cfg["work_scale"] = 2
    base, land = _massif()
    result = erode.erode_canvas(
        base, land, cfg, [], workers=1, block=192, halo=32, log=lambda m: None
    )
    h = result["h"]
    scale = 2
    before = erode.upsample_window(
        base, 0, 0, base.shape[1] * scale, base.shape[0] * scale, scale
    )
    assert h.shape == before.shape
    sea = before < 0
    assert np.allclose(h[sea], before[sea], atol=1e-3)
    plain = (
        (before < 150.0)
        & (np.arange(h.shape[1])[None, :] < 40)
        & (np.arange(h.shape[1])[None, :] > 16)
    )
    assert np.abs(h - before)[plain].max() < 3.0
    mountain = before > 1500.0

    def roughness(array: np.ndarray) -> float:
        return float(
            np.abs(np.gradient(np.gradient(array, axis=1), axis=1))[mountain].mean()
        )

    assert roughness(h) > 1.5 * roughness(before)
    assert float(result["inc"].astype(np.float32).max()) > 20.0
    assert (h[~sea] >= erode.LAND_MIN_M - 1e-3).all()


def test_anchor_keeps_low_frequency_relief() -> None:
    """La moyenne lissée du relief érodé reste proche de celle du relief d'auteur."""
    from scipy import ndimage

    cfg = _cfg()
    cfg["work_scale"] = 2
    base, land = _massif()
    h = erode.erode_canvas(
        base, land, cfg, [], workers=1, block=192, halo=32, log=lambda m: None
    )["h"]
    before = erode.upsample_window(base, 0, 0, base.shape[1] * 2, base.shape[0] * 2, 2)
    low_after = ndimage.gaussian_filter(h, 10)
    low_before = ndimage.gaussian_filter(before, 10)
    assert np.abs(low_after - low_before)[before > 500].mean() < 60.0


def test_rle_rows_roundtrip() -> None:
    """Le codage par plages restitue exactement l'ensemble de tuiles."""
    tiles = {(0, 0), (1, 0), (2, 0), (5, 0), (3, 2)}
    rows = erode.rle_rows(tiles)
    back = {
        (c, r["row"])
        for r in rows
        for start, n in r["runs"]
        for c in range(start, start + n)
    }
    assert back == tiles
