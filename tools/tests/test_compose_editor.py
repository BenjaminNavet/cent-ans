"""Tests de l'éditeur de carte (lot CC, CE) : API, écriture atomique, retouches de lieux, crêtes d'auteur."""

from __future__ import annotations

import copy
import io
import json
import shutil
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

from cent_ans_tools.compose import compile as composer
from cent_ans_tools.compose import editor, nudges, relief, ridges_author
from cent_ans_tools.paths import DATA_DIR

REAL_COMPOSE = DATA_DIR / "map" / "compose"
GUARD = {editor.GUARD_HEADER: "1"}


def _load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


@pytest.fixture
def workdir(tmp_path: Path) -> Path:
    """Copie des fichiers d'auteur (sans la déformation) : les tests n'écrivent jamais dans le dépôt."""
    compose = tmp_path / "compose"
    compose.mkdir()
    for name in (
        "ridges.json",
        "pins.json",
        "selection.json",
        "place_nudges.json",
        "generalise.json",
    ):
        shutil.copyfile(REAL_COMPOSE / name, compose / name)
    return tmp_path


@pytest.fixture
def server(workdir: Path):
    """Serveur réel sur un port libre, dossiers temporaires."""
    ed = editor.Editor(
        compose_dir=workdir / "compose",
        map_dir=workdir / "map",
        cache_dir=workdir / "cache",
    )
    srv = editor.make_server(ed, 0)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    base = f"http://127.0.0.1:{srv.server_address[1]}"

    def call(method: str, path: str, body=None, headers=None, raw: bytes | None = None):
        data = (
            raw
            if raw is not None
            else (None if body is None else json.dumps(body).encode())
        )
        req = urllib.request.Request(
            base + path, data=data, method=method, headers=headers or {}
        )
        try:
            with urllib.request.urlopen(req) as resp:
                return resp.status, resp.read()
        except urllib.error.HTTPError as err:
            return err.code, err.read()

    yield ed, call
    srv.shutdown()


# -- lecture des couches ----------------------------------------------------------


def test_get_layers_and_page(server) -> None:
    """Chaque couche est servie telle que sur disque ; la page s'ouvre."""
    ed, call = server
    for name in ("ridges", "pins", "nudges"):
        status, body = call("GET", f"/api/layers/{name}")
        assert status == 200
        assert json.loads(body) == ed.read_layer(name)
    status, body = call("GET", "/")
    assert status == 200 and "Éditeur de carte".encode() in body
    assert call("GET", "/api/layers/inconnue")[0] == 404
    assert call("GET", "/api/nimportequoi")[0] == 404


def test_missing_nudges_file_reads_empty(server) -> None:
    """Sans ``place_nudges.json`` la couche est vide (pas d'erreur)."""
    ed, call = server
    (ed.compose_dir / "place_nudges.json").unlink()
    status, body = call("GET", "/api/layers/nudges")
    assert status == 200 and json.loads(body)["nudges"] == []


# -- sauvegarde -------------------------------------------------------------------


def test_save_roundtrip_without_loss(server) -> None:
    """Relire puis enregistrer ne perd ni ne change rien, idempotent."""
    ed, call = server
    path = ed.compose_dir / "ridges.json"
    original = _load(path)
    status, body = call("POST", "/api/layers/ridges", original, GUARD)
    assert status == 200, body
    saved = _load(path)
    assert saved["chains_seeded"] is True
    assert saved == original
    first = path.read_bytes()
    call("POST", "/api/layers/ridges", _load(path), GUARD)
    assert path.read_bytes() == first
    for name, file in (("pins", "pins.json"), ("nudges", "place_nudges.json")):
        before = _load(ed.compose_dir / file)
        assert call("POST", f"/api/layers/{name}", before, GUARD)[0] == 200
        assert _load(ed.compose_dir / file) == before


def test_save_rejects_invalid_and_keeps_file(server) -> None:
    """Schéma ou cohérence violés : 422, messages, fichier intact."""
    ed, call = server
    path = ed.compose_dir / "ridges.json"
    before = path.read_bytes()
    doc = _load(path)
    bad = copy.deepcopy(doc)
    bad["ridges"][0]["height_m"] = "haut"
    status, body = call("POST", "/api/layers/ridges", bad, GUARD)
    assert status == 422 and json.loads(body)["errors"]
    dup = copy.deepcopy(doc)
    dup["ridges"].append(copy.deepcopy(dup["ridges"][0]))
    assert call("POST", "/api/layers/ridges", dup, GUARD)[0] == 422
    outside = copy.deepcopy(doc)
    outside["ridges"][0]["points"][0] = [99999, 5]
    assert call("POST", "/api/layers/ridges", outside, GUARD)[0] == 422
    assert path.read_bytes() == before
    unknown = {"version": 1, "nudges": [{"id": "set_inconnu", "target_px": [1, 2]}]}
    status, body = call("POST", "/api/layers/nudges", unknown, GUARD)
    assert status == 422 and "inconnu" in body.decode()
    assert call("POST", "/api/layers/pins", {"version": 1}, GUARD)[0] == 422
    assert call("POST", "/api/layers/pins", None, GUARD, raw=b"{pas du json")[0] == 400


def test_writes_need_the_guard_header(server) -> None:
    """Sans l'en-tête, une page tierce ne peut rien écrire."""
    ed, call = server
    before = (ed.compose_dir / "pins.json").read_bytes()
    assert call("POST", "/api/layers/pins", {"version": 1, "pins": []})[0] == 403
    assert (ed.compose_dir / "pins.json").read_bytes() == before


def test_atomic_write_survives_failure(tmp_path: Path, monkeypatch) -> None:
    """Un échec en cours d'écriture laisse l'ancien fichier et aucun résidu."""
    target = tmp_path / "x.json"
    target.write_bytes(b"ancien\n")

    def boom(fd: int) -> None:
        raise OSError("disque plein")

    monkeypatch.setattr(editor.os, "fsync", boom)
    with pytest.raises(OSError):
        editor.atomic_write(target, b"nouveau\n")
    assert target.read_bytes() == b"ancien\n"
    assert list(tmp_path.iterdir()) == [target]
    monkeypatch.undo()
    editor.atomic_write(target, b"nouveau\n")
    assert target.read_bytes() == b"nouveau\n"


def test_edited_ridge_becomes_authored_and_deletion_is_remembered(server) -> None:
    """Une crête générée modifiée devient d'auteur ; supprimée, elle est notée dans ``deleted_ids``."""
    ed, call = server
    path = ed.compose_dir / "ridges.json"
    doc = _load(path)
    generated = [r for r in doc["ridges"] if not r.get("authored")]
    assert len(generated) >= 2
    moved, removed = generated[0], generated[1]
    moved["points"][0][0] += 25.0
    doc["ridges"] = [r for r in doc["ridges"] if r["id"] != removed["id"]]
    doc["ridges"].append(
        {
            "id": "user_001",
            "kind": "hills",
            "points": [[10, 10], [200, 90]],
            "width_px": 30,
            "height_m": 500,
            "passes": [],
        }
    )
    assert call("POST", "/api/layers/ridges", doc, GUARD)[0] == 200
    saved = _load(path)
    by_id = {r["id"]: r for r in saved["ridges"]}
    assert by_id[moved["id"]]["authored"] is True
    assert by_id["user_001"]["authored"] is True
    assert saved["deleted_ids"] == [removed["id"]]
    # recréée : n'est plus supprimée
    saved["ridges"].append(removed)
    assert call("POST", "/api/layers/ridges", saved, GUARD)[0] == 200
    assert "deleted_ids" not in _load(path)


# -- masques ----------------------------------------------------------------------


def _png(size: tuple[int, int], mode: str = "L", value=255) -> bytes:
    buffer = io.BytesIO()
    Image.new(mode, size, value).save(buffer, "PNG")
    return buffer.getvalue()


def test_masks_blank_save_and_validate(server) -> None:
    """Masque vierge à la demande ; PNG gris de la bonne taille accepté ; le reste refusé."""
    ed, call = server
    shape = ed.mask_shape()
    assert shape == (443, 382)
    status, body = call("GET", "/api/masks/forest.png")
    blank = Image.open(io.BytesIO(body))
    assert (
        status == 200
        and blank.size == shape
        and blank.mode == "L"
        and blank.getextrema() == (0, 0)
    )
    assert not (ed.compose_dir / "masks" / "forest.png").exists()
    assert (
        call("PUT", "/api/masks/forest.png", headers=GUARD, raw=_png(shape))[0] == 200
    )
    saved = Image.open(ed.compose_dir / "masks" / "forest.png")
    assert saved.mode == "L" and saved.getextrema() == (255, 255)
    assert (
        call(
            "PUT",
            "/api/masks/rock.png",
            headers=GUARD,
            raw=_png(shape, "RGB", (7, 7, 7)),
        )[0]
        == 200
    )
    assert Image.open(ed.compose_dir / "masks" / "rock.png").getextrema() == (7, 7)
    assert (
        call("PUT", "/api/masks/marsh.png", headers=GUARD, raw=_png((10, 10)))[0] == 422
    )
    assert (
        call("PUT", "/api/masks/fields.png", headers=GUARD, raw=b"pas un png")[0] == 422
    )
    assert call("PUT", "/api/masks/lave.png", headers=GUARD, raw=_png(shape))[0] == 404
    assert call("PUT", "/api/masks/forest.png", raw=_png(shape))[0] == 403
    assert call("GET", "/api/masks/..%2Fridges.png")[0] == 404


# -- tâche de fond ------------------------------------------------------------------


def _wait(call, state: str, timeout: float = 20.0) -> dict:
    end = time.time() + timeout
    while time.time() < end:
        job = json.loads(call("GET", "/api/job")[1])
        if job["state"] == state:
            return job
        time.sleep(0.05)
    raise AssertionError(f"état {state} non atteint")


def test_recompile_job_runs_steps_and_logs(server) -> None:
    """Les étapes tournent en tâche de fond, le journal est lisible, un second départ est refusé."""
    ed, call = server
    ed.recompile_steps = [
        ["python3", "-c", "import time; time.sleep(0.4); print('étape un')"],
        ["python3", "-c", "print('étape deux')"],
    ]
    assert call("POST", "/api/recompile", {}, GUARD)[0] == 200
    assert call("POST", "/api/recompile", {}, GUARD)[0] == 409
    job = _wait(call, "done")
    assert "étape un" in job["log"] and "étape deux" in job["log"]


def test_recompile_job_stops_on_failure(server) -> None:
    """Une étape en échec arrête la chaîne."""
    ed, call = server
    ed.recompile_steps = [
        ["python3", "-c", "raise SystemExit(3)"],
        ["python3", "-c", "print('jamais')"],
    ]
    call("POST", "/api/recompile", {}, GUARD)
    job = _wait(call, "failed")
    assert "jamais" not in job["log"]


# -- retouches de lieux -------------------------------------------------------------


class _FakeWarp:
    """Déformation de test : ``forward(lon, lat) = (10 lon, 10 lat)``."""

    def forward(self, lon, lat):
        return np.asarray(lon) * 10.0, np.asarray(lat) * 10.0


SETTLEMENTS = [
    {"id": "set_a", "lon": 1.0, "lat": 2.0},
    {"id": "set_b", "lon": 3.0, "lat": 4.0},
]


def test_place_positions_apply_nudges_and_keep_lonlat() -> None:
    """La position composée est remplacée, le lonlat des données reste intact."""
    before = copy.deepcopy(SETTLEMENTS)
    plain = nudges.place_positions(_FakeWarp(), SETTLEMENTS)
    moved = nudges.place_positions(_FakeWarp(), SETTLEMENTS, {"set_b": (7.5, 8.5)})
    assert plain.tolist() == [[10.0, 20.0], [30.0, 40.0]]
    assert moved.tolist() == [[10.0, 20.0], [7.5, 8.5]]
    assert before == SETTLEMENTS


def test_nudges_file_follows_schema_and_loads(tmp_path: Path) -> None:
    """Le fichier d'auteur du dépôt est valide ; ``load_nudges`` lit un fichier posé."""
    ed = editor.Editor()
    assert ed.validate("nudges", _load(REAL_COMPOSE / "place_nudges.json")) == []
    (tmp_path / "place_nudges.json").write_text(
        json.dumps({"version": 1, "nudges": [{"id": "set_a", "target_px": [3, 4]}]}),
        encoding="utf-8",
    )
    assert nudges.load_nudges(tmp_path) == {"set_a": (3.0, 4.0)}
    assert nudges.load_nudges(tmp_path / "absent") == {}


def test_relief_inputs_use_nudges(workdir: Path, monkeypatch) -> None:
    """``relief.load_inputs`` place les plateformes sous la position retouchée."""
    from cent_ans_tools.compose.contract import ComposedWarp

    if not (DATA_DIR / "map" / "land_mask.png").exists():
        pytest.skip("carte composée absente")
    compose = workdir / "compose"
    cw = ComposedWarp.load(REAL_COMPOSE)
    selection = _load(compose / "selection.json")["settlements"]
    target = (1234.5, 2345.5)
    (compose / "place_nudges.json").write_text(
        json.dumps(
            {
                "version": 1,
                "nudges": [{"id": selection[7]["id"], "target_px": list(target)}],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(relief, "COMPOSE_DIR", compose)
    monkeypatch.setattr(composer, "COMPOSE_DIR", compose)
    inputs = relief.load_inputs(cw, relief.load_config(REAL_COMPOSE / "relief.json"))
    base = nudges.place_positions(cw, selection)
    assert inputs.places[7].tolist() == list(target)
    others = np.delete(np.arange(len(selection)), 7)
    assert np.allclose(inputs.places[others], base[others])


# -- crêtes d'auteur ----------------------------------------------------------------


def test_authored_ridges_survive_apply_and_are_seeded_once() -> None:
    """``ridges_author.apply`` n'écrase pas les crêtes d'auteur éditées et n'amorce qu'une fois."""
    doc = {"ridges": [], "size_px": [10, 10], "version": 1}
    report = ridges_author.apply(doc, _FakeWarp(), [])
    assert report["chains"] == len(ridges_author.CHAINS) and doc["chains_seeded"]
    doc["ridges"][0]["points"][0] = [1.0, 2.0]
    doc["ridges"][0]["height_m"] = 1234.0
    edited = copy.deepcopy(doc)
    ridges_author.apply(doc, _FakeWarp(), [])
    assert doc == edited
    doc["ridges"] = []  # tout supprimé par l'auteur : ne revient pas
    ridges_author.apply(doc, _FakeWarp(), [])
    assert doc["ridges"] == []


def test_apply_keeps_user_hills_that_the_filter_would_drop() -> None:
    """Les collines générées en boucle sont filtrées, celles d'auteur jamais."""
    loop = [[0, 0], [50, 0], [50, 50], [0, 50], [0, 1]]
    base = {
        "kind": "hills",
        "points": loop,
        "width_px": 10,
        "height_m": 300,
        "passes": [],
    }
    doc = {
        "chains_seeded": True,
        "ridges": [
            {**base, "id": "hills_01"},
            {**base, "id": "user_001", "authored": True},
        ],
    }
    ridges_author.apply(doc, _FakeWarp(), [])
    assert [r["id"] for r in doc["ridges"]] == ["user_001"]


def test_compile_keeps_authored_ridges(tmp_path: Path) -> None:
    """La recompilation reporte les crêtes d'auteur et respecte ``deleted_ids``."""
    old = {
        "ridges": [
            {
                "id": "massif_01",
                "kind": "massif",
                "points": [[0, 0], [1, 1]],
                "width_px": 9,
                "height_m": 100,
                "passes": [],
                "authored": True,
            },
            {
                "id": "chain_01",
                "kind": "massif",
                "points": [[0, 0], [5, 5]],
                "width_px": 9,
                "height_m": 100,
                "passes": [],
                "authored": True,
            },
            {
                "id": "massif_02",
                "kind": "massif",
                "points": [[0, 0], [2, 2]],
                "width_px": 9,
                "height_m": 100,
                "passes": [],
            },
        ],
        "deleted_ids": ["massif_03"],
        "chains_seeded": True,
    }
    path = tmp_path / "ridges.json"
    path.write_text(json.dumps(old), encoding="utf-8")
    fresh = {
        "ridges": [
            {
                "id": f"massif_0{i}",
                "kind": "massif",
                "points": [[0, 0], [9, 9]],
                "width_px": 1,
                "height_m": 1,
                "passes": [],
            }
            for i in (1, 2, 3)
        ]
    }
    assert composer.keep_authored_ridges(fresh, path) == 2
    ids = [r["id"] for r in fresh["ridges"]]
    assert ids == [
        "massif_02",
        "massif_01",
        "chain_01",
    ]  # 03 supprimée, 01 remplacée par la version d'auteur
    assert fresh["ridges"][1]["points"] == [[0, 0], [1, 1]]
    assert fresh["deleted_ids"] == ["massif_03"] and fresh["chains_seeded"] is True


def test_real_ridges_file_is_idempotent_under_author_pass() -> None:
    """Sur le vrai fichier, la passe d'auteur ne change plus rien (les chaînes sont la source)."""
    doc = _load(REAL_COMPOSE / "ridges.json")
    before = json.dumps(doc["ridges"], sort_keys=True)
    ridges_author.apply(doc, _FakeWarp(), [])
    assert json.dumps(doc["ridges"], sort_keys=True) == before


# -- état complet et aperçu (données réelles) ----------------------------------------


@pytest.fixture(scope="module")
def real_editor(tmp_path_factory) -> editor.Editor:
    """Éditeur sur les vraies données (cache temporaire)."""
    if (
        not (DATA_DIR / "map" / "heightmap.png").exists()
        or not (REAL_COMPOSE / "warp.json").exists()
    ):
        pytest.skip("carte composée absente")
    return editor.Editor(cache_dir=tmp_path_factory.mktemp("cache"))


def test_state_lists_places_geometry_and_masks(real_editor) -> None:
    """``/api/state`` : lieux nommés, côtes, fleuves, masques."""
    state = real_editor.state()
    assert len(state["places"]) == 400 and state["mask_size"] == [443, 382]
    assert (
        state["coast"]
        and state["rivers"]
        and set(state["masks"]) == set(editor.MASK_NAMES)
    )
    assert any(p["name"] != p["id"] for p in state["places"])
    lonlat = real_editor.lonlat(3500.0, 3000.0)
    forward = real_editor.forward([lonlat])[0]
    assert abs(forward[0] - 3500.0) < 3 and abs(forward[1] - 3000.0) < 3


def test_preview_reflects_unsaved_ridge_edit(real_editor) -> None:
    """Relever une crête non enregistrée change l'aperçu de sa zone, sans rien écrire."""
    ridges, pins = real_editor.read_layer("ridges"), real_editor.read_layer("nudges")
    target = next(
        r for r in ridges["ridges"] if r["kind"] == "massif" and r["id"] == "chain_01"
    )
    cx, cy = np.mean(target["points"], axis=0)
    window = [cx - 300, cy - 300, cx + 300, cy + 300]
    disk_before = (REAL_COMPOSE / "ridges.json").read_bytes()
    first = real_editor.render_preview(window, ridges, pins)
    edited = copy.deepcopy(ridges)
    next(r for r in edited["ridges"] if r["id"] == "chain_01")["height_m"] = 4500.0
    second = real_editor.render_preview(window, edited, pins)
    img_a = np.asarray(
        Image.open(real_editor.cache_dir / first["url"].split("/")[-1].split("?")[0]),
        dtype=int,
    )
    img_b = np.asarray(
        Image.open(real_editor.cache_dir / second["url"].split("/")[-1].split("?")[0]),
        dtype=int,
    )
    assert img_a.shape == img_b.shape and np.abs(img_a - img_b).sum() > 0
    assert (REAL_COMPOSE / "ridges.json").read_bytes() == disk_before
    with pytest.raises(editor.EditorError):
        real_editor.render_preview([0, 0, 5000, 5000], ridges, pins)
