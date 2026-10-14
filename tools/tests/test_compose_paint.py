"""Peinture vectorielle régionale (lot CP) : schéma, rastérisation, écriture dérivée, aperçu, recompilation."""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

from cent_ans_tools.compose import editor, masks, paint
from cent_ans_tools.compose.contract import ComposedWarp
from cent_ans_tools.paths import DATA_DIR

REAL_COMPOSE = DATA_DIR / "map" / "compose"
SHAPE = (382, 443)


def _doc(*strokes: dict) -> dict:
    return {"strokes": list(strokes)}


def _stroke(**over) -> dict:
    base = {
        "id": "t1",
        "layer": "forest",
        "mode": "add",
        "strength": 1.0,
        "feather_km": 0,
        "note": "essai",
        "coords": "px",
        "polygon": [[160, 160], [480, 160], [480, 480], [160, 480]],
    }
    return {**base, **over}


def test_schema_accepts_and_rejects() -> None:
    """Un trait valide passe ; couche inconnue, polygone court ou note absente échouent."""
    paint.validate(_doc(_stroke()))
    for bad in (
        _stroke(layer="water"),
        _stroke(polygon=[[0, 0], [1, 1]]),
        _stroke(strength=2),
        {k: v for k, v in _stroke().items() if k != "note"},
    ):
        with pytest.raises(paint.PaintError):
            paint.validate(_doc(bad))


def test_rasterize_px_square_hard_edge() -> None:
    """Carré de 20 x 20 cases px : plein dedans, 0 dehors, taille de la grille."""
    grid = paint.rasterize_polygon(
        np.array([[160, 160], [480, 160], [480, 480], [160, 480]], float), SHAPE
    )
    assert grid.shape == SHAPE
    assert grid[15, 15] == 1.0 and grid[5, 5] == 0.0 and grid[40, 40] == 0.0
    assert grid.sum() == pytest.approx(400, abs=20)


def test_feather_softens_edge() -> None:
    """Avec un bord adouci : 0,5 sur le contour, rampe de part et d'autre, centre plein."""
    square = np.array([[160, 160], [480, 160], [480, 480], [160, 480]], float)
    grid = paint.rasterize_polygon(square, SHAPE, feather_px=64.0)
    assert grid[15, 15] == pytest.approx(1.0)
    assert 0.0 < grid[8, 15] < 0.5 < grid[11, 15] < 1.0  # contour à la ligne 10
    assert grid[3, 15] == 0.0


def test_lonlat_polygon_lands_on_known_place() -> None:
    """Un polygone lon/lat autour de Paris couvre la position composée de Paris, pas celle de Rome."""
    warp = ComposedWarp.load(REAL_COMPOSE)
    stroke = _stroke(
        coords="lonlat",
        feather_km=5,
        polygon=[[1.8, 48.4], [2.9, 48.4], [2.9, 49.3], [1.8, 49.3]],
    )
    grids = paint.rasterize_all([stroke], SHAPE, warp, 719.0)
    grid = grids["forest"]
    px, py = (float(v) for v in warp.forward(2.35, 48.85))
    assert grid[int(py // 16), int(px // 16)] == pytest.approx(1.0)
    rx, ry = (float(v) for v in warp.forward(12.5, 41.9))
    assert grid[int(ry // 16), int(rx // 16)] == 0.0
    assert 20 < grid.sum() < 400  # ~ 80 x 100 km en cases de 11,5 km


def test_modes_strength_and_max(tmp_path: Path) -> None:
    """Add / clear vont dans des grilles distinctes, strength module, deux traits se combinent au max."""
    strokes = [
        _stroke(id="a", strength=0.4),
        _stroke(id="b", strength=0.7, polygon=[[240, 240], [400, 240], [400, 400]]),
        _stroke(id="c", mode="clear", layer="marsh"),
    ]
    grids = paint.rasterize_all(strokes, SHAPE, None, 719.0)
    assert set(grids) == {"forest", "marsh_clear"}
    assert grids["forest"].max() == pytest.approx(0.7)
    assert grids["forest"][12, 12] == pytest.approx(0.4)


def test_write_derived_and_combine_with_brush(tmp_path: Path) -> None:
    """La commande écrit masks/derived/, ne touche pas au pinceau, et load_all prend le max des deux."""
    compose = tmp_path
    (compose / "paint").mkdir()
    (compose / "masks").mkdir()
    (compose / "ridges.json").write_text(json.dumps({"size_px": [7080, 6104]}))
    brush = np.zeros(SHAPE[::1], np.uint8)
    brush[2, 2] = 128
    Image.fromarray(brush, mode="L").save(compose / "masks" / "forest.png")
    (compose / "paint" / "ouest.json").write_text(
        json.dumps(
            _doc(
                _stroke(),
                _stroke(id="t2", layer="fields", mode="clear", strength=0.5),
            )
        )
    )
    counts = paint.write_derived(compose)
    assert counts == {"forest": 1, "fields_clear": 1}
    derived = compose / "masks" / "derived"
    assert (derived / "forest.png").exists() and (derived / "fields_clear.png").exists()
    with Image.open(compose / "masks" / "forest.png") as image:  # pinceau intact
        assert np.asarray(image)[2, 2] == 128
    found = masks.load_all((SHAPE[0] * 16, SHAPE[1] * 16), compose)
    assert found["forest"][200, 200] == pytest.approx(
        1.0
    )  # dérivé (255) > pinceau (128 en case 2,2)
    assert 0.4 < found["forest"][40, 40] < 0.6  # hors trait : le pinceau seul
    assert found["fields_clear"].max() == pytest.approx(0.5, abs=0.01)
    # Plus de trait : le dérivé disparaît, le pinceau reste.
    (compose / "paint" / "ouest.json").unlink()
    paint.write_derived(compose)
    assert not list(derived.glob("*.png"))
    assert set(masks.load_all((SHAPE[0] * 16, SHAPE[1] * 16), compose)) == {"forest"}


def test_duplicate_ids_rejected(tmp_path: Path) -> None:
    """Deux traits de même id dans un fichier : erreur."""
    (tmp_path / "x.json").write_text(json.dumps(_doc(_stroke(), _stroke())))
    with pytest.raises(paint.PaintError):
        paint.load_strokes(tmp_path)


def test_preview_image(tmp_path: Path) -> None:
    """L'aperçu fait 443 x 382, couleur de la couche dans le trait, fond de terre ailleurs."""
    compose = tmp_path / "c"
    (compose / "paint").mkdir(parents=True)
    (compose / "ridges.json").write_text(json.dumps({"size_px": [7080, 6104]}))
    (compose / "paint" / "est.json").write_text(json.dumps(_doc(_stroke())))
    land = Image.new("L", (708, 610), 255)
    land_path = tmp_path / "land.png"
    land.save(land_path)
    out = paint.preview(tmp_path / "out.png", compose, land_path)
    with Image.open(out) as image:
        assert image.size == (443, 382) and image.mode == "RGB"
        pixels = np.asarray(image)
    assert tuple(pixels[15, 15]) == paint.PREVIEW_COLORS["forest"]
    assert tuple(pixels[100, 100]) == (150, 150, 150)


def test_default_recompile_steps_include_paint(tmp_path: Path, monkeypatch) -> None:
    """Le bouton Recompiler enchaîne compose, relief, paint, compose-layers (surchargeable)."""
    ed = editor.Editor(compose_dir=tmp_path, map_dir=tmp_path, cache_dir=tmp_path)
    captured = {}
    monkeypatch.setattr(
        editor.threading,
        "Thread",
        lambda target, args, daemon: type(
            "T", (), {"start": lambda self: captured.setdefault("steps", args[1])}
        )(),
    )
    ed.start_recompile()
    steps = [" ".join(c[3:]) for c in captured["steps"]]
    assert steps == [
        "geo compose",
        "geo compose-relief",
        "compose paint",
        "geo compose-layers --only landcover,colormap",
    ]


def test_cover_png_reduces_channel(tmp_path: Path) -> None:
    """Le calque de couverture réduit le canal du splat à la grille des masques."""
    (tmp_path / "ridges.json").write_text(json.dumps({"size_px": [64, 48]}))
    splat = np.zeros((48, 64, 4), np.uint8)
    splat[..., 2] = 255
    Image.fromarray(splat, mode="RGBA").save(tmp_path / "splat.png")
    ed = editor.Editor(
        compose_dir=tmp_path, map_dir=tmp_path, cache_dir=tmp_path / "cache"
    )
    import io

    forest = np.asarray(Image.open(io.BytesIO(ed.cover_png("forest"))))
    assert forest.shape == (3, 4) and forest.min() == 255
    assert np.asarray(Image.open(io.BytesIO(ed.cover_png("rock")))).max() == 0
    marsh = np.asarray(Image.open(io.BytesIO(ed.cover_png("marsh"))))  # pas de wetlands
    assert marsh.max() == 0


class _NanWarp:
    """Cartogramme factice : lon > 100 est hors domaine (NaN)."""

    def forward(self, lon, lat):
        lon = np.asarray(lon, float)
        lat = np.asarray(lat, float)
        return np.where(lon > 100, np.nan, lon * 16), np.where(
            lon > 100, np.nan, lat * 16
        )


def test_out_of_domain_vertices_are_dropped_with_warning() -> None:
    """Un sommet NaN est ignoré (avertissement), un trait à < 3 sommets aussi ; jamais de plantage."""
    good = _stroke(
        id="g",
        coords="lonlat",
        polygon=[[10, 10], [30, 10], [30, 30], [10, 30], [200, 5]],
        region="ouest",
    )
    short = _stroke(
        id="s", coords="lonlat", polygon=[[10, 10], [30, 10], [200, 30]], region="ouest"
    )
    with pytest.warns(UserWarning) as record:
        grids = paint.rasterize_all([good, short], SHAPE, _NanWarp(), 1.0)
    messages = " ".join(str(w.message) for w in record)
    assert "ouest.json trait g : sommet 4" in messages
    assert "trait s" in messages and "trait ignoré" in messages
    assert grids["forest"].sum() > 0
