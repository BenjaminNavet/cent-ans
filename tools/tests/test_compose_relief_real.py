"""Tests du relief réel déformé (lot CC CD) : pyramide réelle synthétique et contraintes d'auteur."""

from __future__ import annotations

import json
from pathlib import Path

import jsonschema
import numpy as np

from cent_ans_tools.compose import relief_real as rr
from cent_ans_tools.geo import terrain
from cent_ans_tools.paths import DATA_DIR

COMPOSE_DIR = DATA_DIR / "map" / "compose"


def _write(path: Path, heights: np.ndarray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    terrain.write_png16(terrain.height_to_uint16(heights), path)


def _fake_real(tmp_path: Path) -> rr.RealPyramid:
    """Pyramide réelle : E0 partout (pente en x), E1 seulement sur la tuile (0, 0), plus fine."""
    (tmp_path / "relief_pyramid.json").write_text(
        json.dumps({"height_min_m": -200.0, "height_range_m": 5000.0}), encoding="utf-8"
    )
    x = np.arange(rr.TILE_PX, dtype=np.float32)[None, :].repeat(rr.TILE_PX, axis=0)
    for col in range(2):
        _write(tmp_path / "height" / f"h_{col}_0.png", x * 2.0 + col * 1024.0)
    _write(tmp_path / "pyramid" / "E1" / "0_0.png", x * 2.0 + 500.0)
    return rr.RealPyramid(tmp_path)


def test_relief_real_settings_follow_schema() -> None:
    """``relief_real.json`` est valide."""
    jsonschema.validate(
        json.loads((COMPOSE_DIR / "relief_real.json").read_text("utf-8")),
        json.loads(
            (DATA_DIR / "schemas" / "compose_relief_real.schema.json").read_text(
                "utf-8"
            )
        ),
    )


def test_mosaic_uses_finest_tile_and_falls_back_to_coarser_level(
    tmp_path: Path,
) -> None:
    """Une tuile E1 présente est lue telle quelle ; une absente est comblée par l'étage E0 suréchantillonné."""
    real = _fake_real(tmp_path)
    present = real.mosaic(1, 10, 10, 20, 20)
    assert np.allclose(present[0, 0], 10 * 2.0 + 500.0, atol=0.2)
    assert real.coverage(1, 0, 0, 512, 512) == 1.0
    assert real.coverage(1, 512, 0, 1024, 512) == 0.0
    missing = real.mosaic(1, 600, 10, 610, 20)
    assert np.isfinite(missing).all()
    # E0 : x * 2 ; E1 absent -> même rampe vue depuis E0 (pixel E1 = 1/2 pixel E0)
    assert abs(float(missing[0, 0]) - 2.0 * (600 / 2)) < 6.0


def test_crest_floor_never_exceeds_local_real_peak_or_author_cap() -> None:
    """Le plancher de crête reste sous max(0,85 x sommet réel voisin, plafond d'auteur)."""
    from cent_ans_tools.compose import relief as author_relief

    cfg = json.loads((COMPOSE_DIR / "relief_real.json").read_text("utf-8"))
    shape = (120, 160)
    real_b = np.full(shape, 300.0, dtype=np.float32)
    real_b[:, 80:84] = 2400.0  # un sommet réel net
    author = np.full(shape, 3200.0, dtype=np.float32)
    inp = author_relief.Inputs(
        land=np.ones(shape, bool),
        inside=np.ones(shape, bool),
        real_height=real_b,
        ridges=[
            {
                "id": "m",
                "kind": "massif",
                "points": [(40.0, 60.0), (120.0, 60.0)],
                "width_px": 20.0,
                "height_m": 3200.0,
                "passes": [],
            }
        ],
        rivers=[],
        places=np.zeros((0, 2)),
    )
    floor = rr.crest_floor(inp, author, real_b, cfg)
    c = cfg["crest"]
    bound = max(
        c["floor_real_frac"] * 2400.0,
        min(c["floor_author_frac"] * 3200.0, c["floor_author_cap_m"]),
    )
    assert float(floor.max()) <= bound + 1e-3
    assert float(floor[60, 100]) > 0.0  # sur la polyligne
    assert float(floor[5, 100]) == 0.0  # loin de la polyligne


def _chain_inputs(shape: tuple[int, int]):
    from cent_ans_tools.compose import relief as author_relief

    return author_relief.Inputs(
        land=np.ones(shape, bool),
        inside=np.ones(shape, bool),
        real_height=np.zeros((1, 1), np.float32),
        ridges=[
            {
                "id": "c",
                "kind": "massif",
                "points": [(20.0, 60.0), (140.0, 60.0)],
                "width_px": 20.0,
                "height_m": 2000.0,
                "passes": [],
            }
        ],
        rivers=[],
        places=np.zeros((0, 2)),
    )


def test_chain_gain_amplifies_real_relief_softly_and_leaves_plains() -> None:
    """Le gain croît le relief au-dessus du socle près de la chaîne, sans toucher la plaine ni dépasser ``span``."""
    shape = (120, 160)
    cfg = {
        "chains": {
            "c": {
                "gain": 1.5,
                "socle_m": 300.0,
                "span_m": 1000.0,
                "core_px": 5.0,
                "influence_px": 25.0,
            }
        }
    }
    gain, socle, span = rr.chain_gain_fields(_chain_inputs(shape), shape, cfg)
    assert float(gain[60, 80]) == 0.5  # sur la chaîne : g - 1
    assert float(gain[60 + 40, 80]) == 0.0  # hors de la zone d'influence
    assert 0.0 < float(gain[60 + 15, 80]) < 0.5  # fondu doux
    real = np.full(shape, 1500.0, np.float32)
    out = rr.apply_gain(real, gain, socle, span)
    assert abs(float(out[60, 80]) - (1500.0 + 0.5 * 1000.0)) < 1e-3  # plafonné par span
    plain = rr.apply_gain(np.full(shape, 200.0, np.float32), gain, socle, span)
    assert np.allclose(plain, 200.0)  # sous le socle : identité
    assert not rr.chain_gain_fields(_chain_inputs(shape), shape, {})[0].any()


def test_crest_floor_skips_chains_with_crest_disabled() -> None:
    """``chains.<id>.crest = false`` supprime le mur de crête de cette chaîne."""
    cfg = json.loads((COMPOSE_DIR / "relief_real.json").read_text("utf-8"))
    cfg["chains"] = {"c": {"crest": False}}
    shape = (120, 160)
    real_b = np.full(shape, 300.0, np.float32)
    author = np.full(shape, 2000.0, np.float32)
    floor = rr.crest_floor(_chain_inputs(shape), author, real_b, cfg)
    assert float(floor.max()) == 0.0


def test_tiles_in_bbox_and_swap_dir(tmp_path: Path) -> None:
    """Sélection de tuiles racines d'une région et échange atomique de dossier."""
    shape = (1024, 1536)  # 4 x 6 tuiles racines
    assert len(rr.tiles_in_bbox(shape, None)) == 24
    assert rr.tiles_in_bbox(shape, (300, 10, 500, 20)) == [(1, 0)]
    assert rr.tiles_in_bbox(shape, (256, 256, 257, 257)) == [(1, 1)]
    final, new = tmp_path / "p", tmp_path / "p.new"
    final.mkdir()
    (final / "old.txt").write_text("a")
    new.mkdir()
    (new / "new.txt").write_text("b")
    rr.swap_dir(new, final)
    assert (final / "new.txt").exists() and not (final / "old.txt").exists()
    assert not new.exists() and not (tmp_path / "p.old").exists()
