"""Éditeur de carte local (lot CC, CE) : serveur HTTP de la bibliothèque standard + page unique.

Les couches éditables sont les fichiers d'auteur de ``data/map/compose`` :

- ``ridges.json`` : crêtes, cols (schéma ``compose_ridges``) ;
- ``pins.json`` : épingles de la déformation (schéma ``compose_pins``) ;
- ``place_nudges.json`` : position composée des lieux (schéma ``compose_place_nudges``) ;
- ``masks/{forest,rock,fields,marsh}.png`` : masques peints (format : ``docs/editeur-carte.md``).

Toute écriture valide d'abord contre le schéma (plus des contrôles de cohérence) puis remplace le
fichier de façon atomique. Le serveur n'écoute que sur 127.0.0.1 ; les requêtes qui modifient
quelque chose doivent porter l'en-tête ``X-Cent-Ans-Editor``.
"""

from __future__ import annotations

import io
import json
import os
import subprocess
import sys
import threading
import time
import traceback
from dataclasses import dataclass, field
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

import jsonschema
import numpy as np
from PIL import Image

from cent_ans_tools.paths import DATA_DIR, REPO_DIR

COMPOSE_DIR = DATA_DIR / "map" / "compose"
MAP_DIR = DATA_DIR / "map"
SCHEMA_DIR = DATA_DIR / "schemas"
STATIC_DIR = Path(__file__).resolve().parent / "editor_static"
CACHE_DIR = Path.home() / "dev" / "cent-ans-raw" / "compose" / "editor"
GUARD_HEADER = "X-Cent-Ans-Editor"
MAX_BODY = 64 * 1024 * 1024
#: Pixels composés par pixel de masque (canevas 7080 x 6104 -> 443 x 382).
MASK_CELL_PX = 16
MASK_NAMES = ("forest", "rock", "fields", "marsh")
BACKGROUND_WIDTH = 2400
PREVIEW_MARGIN = 260
PREVIEW_MAX_SIDE = 2200

#: couche -> (fichier, schéma, indentation JSON (None = compact))
LAYERS: dict[str, tuple[str, str, int | None]] = {
    "ridges": ("ridges.json", "compose_ridges.schema.json", None),
    "pins": ("pins.json", "compose_pins.schema.json", 2),
    "nudges": ("place_nudges.json", "compose_place_nudges.schema.json", 2),
}


class EditorError(Exception):
    """Erreur de requête : ``status`` HTTP et liste de messages en français."""

    def __init__(self, status: int, errors: list[str]) -> None:
        """Mémorise le statut et les messages."""
        super().__init__("; ".join(errors))
        self.status = status
        self.errors = errors


def atomic_write(path: Path, data: bytes) -> None:
    """Écrit ``data`` dans un fichier voisin puis le renomme sur ``path`` (jamais de fichier tronqué)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    try:
        with open(tmp, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp, path)
    except BaseException:
        tmp.unlink(missing_ok=True)
        raise


def dump_json(doc: dict, indent: int | None) -> bytes:
    """Sérialise comme les fichiers d'auteur (UTF-8, fin de ligne, compact si ``indent`` est None)."""
    if indent is None:
        text = json.dumps(doc, ensure_ascii=False, separators=(",", ":"))
    else:
        text = json.dumps(doc, ensure_ascii=False, indent=indent)
    return (text + "\n").encode("utf-8")


@dataclass
class Job:
    """Tâche de fond (recompilation de la carte)."""

    state: str = "idle"  # idle | running | done | failed
    step: str = ""
    log: list[str] = field(default_factory=list)
    started: float = 0.0
    finished: float = 0.0

    def as_dict(self, tail: int = 400) -> dict:
        """Vue JSON : état, étape, dernières lignes du journal."""
        end = self.finished or time.time()
        return {
            "state": self.state,
            "step": self.step,
            "seconds": round(end - self.started, 1) if self.started else 0.0,
            "log": self.log[-tail:],
        }


class Editor:
    """État du serveur : chemins, caches, tâche de fond. Toute la logique métier est ici."""

    def __init__(
        self,
        compose_dir: Path = COMPOSE_DIR,
        map_dir: Path = MAP_DIR,
        cache_dir: Path = CACHE_DIR,
        schema_dir: Path = SCHEMA_DIR,
    ) -> None:
        """Fixe les dossiers (injectables pour les tests)."""
        self.compose_dir = compose_dir
        self.map_dir = map_dir
        self.cache_dir = cache_dir
        self.schema_dir = schema_dir
        self.lock = threading.RLock()
        self.job = Job()
        self.recompile_steps: list[list[str]] | None = None
        self._inputs = None
        self._cw = None
        self._settlements: list[dict] | None = None
        self._names: dict[str, str] | None = None
        self._preview_seq = 0

    # -- couches JSON -------------------------------------------------------
    def layer_path(self, name: str) -> Path:
        """Fichier d'une couche (404 si le nom est inconnu)."""
        if name not in LAYERS:
            raise EditorError(HTTPStatus.NOT_FOUND, [f"Couche inconnue : {name}"])
        return self.compose_dir / LAYERS[name][0]

    def read_layer(self, name: str) -> dict:
        """Contenu d'une couche ; ``place_nudges`` absent = document vide."""
        path = self.layer_path(name)
        if not path.exists() and name == "nudges":
            return {"version": 1, "nudges": []}
        return json.loads(path.read_text(encoding="utf-8"))

    def size_px(self) -> tuple[int, int]:
        """Taille du canevas composé (``ridges.json``)."""
        w, h = self.read_layer("ridges")["size_px"]
        return int(w), int(h)

    def settlements(self) -> list[dict]:
        """Lieux de ``selection.json`` (id, lon, lat, province, kind)."""
        if self._settlements is None:
            doc = json.loads((self.compose_dir / "selection.json").read_text("utf-8"))
            self._settlements = doc["settlements"]
        return self._settlements

    def place_names(self) -> dict[str, str]:
        """Nom d'affichage des lieux, lu dans ``data/settlements`` (repli sur l'identifiant)."""
        if self._names is None:
            names: dict[str, str] = {}
            folder = self.compose_dir.parent.parent / "settlements"
            for path in sorted(folder.glob("*.json")) if folder.is_dir() else []:
                try:
                    for item in json.loads(path.read_text("utf-8")):
                        names[item["id"]] = item["name"]["display"]
                except (ValueError, KeyError, TypeError):
                    continue
            self._names = names
        return self._names

    def validate(self, name: str, doc: dict) -> list[str]:
        """Erreurs de schéma et de cohérence (liste vide = valide)."""
        schema = json.loads(
            (self.schema_dir / LAYERS[name][1]).read_text(encoding="utf-8")
        )
        validator = jsonschema.Draft202012Validator(schema)
        errors = [
            f"{'/'.join(str(p) for p in e.absolute_path) or '(racine)'} : {e.message}"
            for e in sorted(
                validator.iter_errors(doc), key=lambda e: list(e.absolute_path)
            )
        ]
        if errors:
            return errors[:20]
        return getattr(self, f"_check_{name}")(doc)

    def _check_ridges(self, doc: dict) -> list[str]:
        errors: list[str] = []
        width, height = doc["size_px"]
        ids = [r["id"] for r in doc["ridges"]]
        errors += [
            f"Crête en double : {i}"
            for i in sorted({i for i in ids if ids.count(i) > 1})
        ]
        for ridge in doc["ridges"]:
            for x, y in ridge["points"]:
                if not (-50 <= x <= width + 50 and -50 <= y <= height + 50):
                    errors.append(
                        f"{ridge['id']} : sommet hors de la carte ({x:.0f}, {y:.0f})"
                    )
                    break
        return errors

    def _check_pins(self, doc: dict) -> list[str]:
        ids = [p["id"] for p in doc["pins"]]
        return [
            f"Épingle en double : {i}"
            for i in sorted({i for i in ids if ids.count(i) > 1})
        ]

    def _check_nudges(self, doc: dict) -> list[str]:
        known = {s["id"] for s in self.settlements()}
        ids = [n["id"] for n in doc["nudges"]]
        errors = [
            f"Lieu inconnu de selection.json : {i}" for i in ids if i not in known
        ]
        errors += [
            f"Retouche en double : {i}"
            for i in sorted({i for i in ids if ids.count(i) > 1})
        ]
        return errors

    def _merge_ridges(self, new: dict) -> dict:
        """Marque ``authored`` toute crête modifiée ou créée ; note les crêtes générées supprimées."""
        path = self.layer_path("ridges")
        old = json.loads(path.read_text("utf-8")) if path.exists() else {"ridges": []}
        before = {r["id"]: r for r in old["ridges"]}
        for ridge in new["ridges"]:
            previous = before.get(ridge["id"])
            if not ridge.get("authored") and (previous is None or previous != ridge):
                ridge["authored"] = True
        kept = {r["id"] for r in new["ridges"]}
        deleted = set(old.get("deleted_ids", [])) - kept
        deleted |= {
            i for i, r in before.items() if i not in kept and not r.get("authored")
        }
        if deleted:
            new["deleted_ids"] = sorted(deleted)
        else:
            new.pop("deleted_ids", None)
        if old.get("chains_seeded") or any(r.get("authored") for r in new["ridges"]):
            new["chains_seeded"] = True
        return new

    def save_layer(self, name: str, doc: dict) -> dict:
        """Valide puis écrit atomiquement ; ``422`` avec la liste des erreurs sinon."""
        path = self.layer_path(name)
        if not isinstance(doc, dict):
            raise EditorError(
                HTTPStatus.UNPROCESSABLE_ENTITY, ["Document JSON attendu."]
            )
        with self.lock:
            errors = self.validate(name, doc)
            if errors:
                raise EditorError(HTTPStatus.UNPROCESSABLE_ENTITY, errors)
            if name == "ridges":
                doc = self._merge_ridges(doc)
            atomic_write(path, dump_json(doc, LAYERS[name][2]))
        return doc

    # -- masques ------------------------------------------------------------
    def mask_shape(self) -> tuple[int, int]:
        """``(largeur, hauteur)`` des masques."""
        w, h = self.size_px()
        return -(-w // MASK_CELL_PX), -(-h // MASK_CELL_PX)

    def mask_path(self, name: str) -> Path:
        """Fichier d'un masque (404 si inconnu)."""
        if name not in MASK_NAMES:
            raise EditorError(HTTPStatus.NOT_FOUND, [f"Masque inconnu : {name}"])
        return self.compose_dir / "masks" / f"{name}.png"

    def read_mask(self, name: str) -> bytes:
        """PNG du masque ; un masque jamais peint est renvoyé vide (noir)."""
        path = self.mask_path(name)
        if path.exists():
            return path.read_bytes()
        buffer = io.BytesIO()
        Image.new("L", self.mask_shape(), 0).save(buffer, "PNG")
        return buffer.getvalue()

    COVER_CHANNELS = {  # couche -> (fichier de map_dir, canal)
        "forest": ("splat.png", 2),
        "rock": ("splat.png", 3),
        "fields": ("splat.png", 1),
        "marsh": ("wetlands.png", 0),
    }

    def cover_png(self, name: str) -> bytes:
        """Couverture calculée actuelle de la couche (splat / zones humides) réduite à la grille des masques.

        PNG 8 bits à niveaux de gris, mis en cache (recalculé quand le fichier source change) ; noir si
        la cuisson n'existe pas encore.
        """
        if name not in self.COVER_CHANNELS:
            raise EditorError(HTTPStatus.NOT_FOUND, [f"Couche inconnue : {name}"])
        filename, channel = self.COVER_CHANNELS[name]
        source = self.map_dir / filename
        cache = self.cache_dir / f"cover_{name}.png"
        with self.lock:
            if not source.exists():
                buffer = io.BytesIO()
                Image.new("L", self.mask_shape(), 0).save(buffer, "PNG")
                return buffer.getvalue()
            if cache.exists() and cache.stat().st_mtime >= source.stat().st_mtime:
                return cache.read_bytes()
            Image.MAX_IMAGE_PIXELS = None
            with Image.open(source) as image:
                band = image.getchannel(channel)
                small = band.resize(self.mask_shape(), Image.Resampling.BOX)
            buffer = io.BytesIO()
            small.save(buffer, "PNG", optimize=True)
            self.cache_dir.mkdir(parents=True, exist_ok=True)
            atomic_write(cache, buffer.getvalue())
            return buffer.getvalue()

    def save_mask(self, name: str, data: bytes) -> dict:
        """Valide (PNG 8 bits à niveaux de gris, bonne taille) puis écrit atomiquement."""
        path = self.mask_path(name)
        try:
            image = Image.open(io.BytesIO(data))
            image.load()
        except Exception as exc:  # noqa: BLE001 - toute image illisible est une erreur de requête
            raise EditorError(
                HTTPStatus.UNPROCESSABLE_ENTITY, [f"PNG illisible : {exc}"]
            ) from exc
        if image.mode in ("RGB", "RGBA"):
            if image.mode == "RGBA":
                black = Image.new("RGBA", image.size, (0, 0, 0, 255))
                image = Image.alpha_composite(black, image)
            image = image.convert("L")
        if image.mode != "L":
            raise EditorError(
                HTTPStatus.UNPROCESSABLE_ENTITY,
                [f"Masque en niveaux de gris 8 bits attendu, reçu {image.mode}."],
            )
        if image.size != self.mask_shape():
            raise EditorError(
                HTTPStatus.UNPROCESSABLE_ENTITY,
                [f"Taille attendue {self.mask_shape()}, reçue {image.size}."],
            )
        buffer = io.BytesIO()
        image.save(buffer, "PNG", optimize=True)
        with self.lock:
            atomic_write(path, buffer.getvalue())
        return {"ok": True, "size": list(image.size)}

    # -- géométrie, lieux ---------------------------------------------------
    def warp(self):  # noqa: ANN201
        """Contrat de déformation (chargé une fois)."""
        if self._cw is None:
            from cent_ans_tools.compose.contract import ComposedWarp

            self._cw = ComposedWarp.load(self.compose_dir)
        return self._cw

    def places(self) -> list[dict]:
        """Lieux : position composée de base (``x``, ``y``), position retouchée éventuelle."""
        from cent_ans_tools.compose import nudges as nudge_mod

        base = nudge_mod.place_positions(self.warp(), self.settlements())
        moved = nudge_mod.load_nudges(self.compose_dir)
        names = self.place_names()
        out = []
        for s, (x, y) in zip(self.settlements(), base, strict=True):
            out.append(
                {
                    "id": s["id"],
                    "name": names.get(s["id"], s["id"]),
                    "kind": s["kind"],
                    "province": s["province"],
                    "lon": s["lon"],
                    "lat": s["lat"],
                    "x": round(float(x), 1),
                    "y": round(float(y), 1),
                }
            )
        del moved
        return out

    def geometry(self) -> dict:
        """Côtes et fleuves en polylignes de pixels composés (une décimale)."""

        def lines(path: Path) -> list[list[list[float]]]:
            if not path.exists():
                return []
            out = []
            for feat in json.loads(path.read_text("utf-8"))["features"]:
                geom = feat["geometry"]
                parts = (
                    geom["coordinates"]
                    if geom["type"] == "MultiLineString"
                    else [geom["coordinates"]]
                )
                out += [[[round(x, 1), round(y, 1)] for x, y in part] for part in parts]
            return out

        return {
            "coast": lines(self.map_dir / "coastline.geojson"),
            "rivers": lines(self.map_dir / "rivers.geojson"),
        }

    def lonlat(self, x: float, y: float) -> list[float]:
        """Longitude/latitude réelles d'un point composé (pour poser une épingle)."""
        lon, lat = self.warp().inverse(x, y)
        return [round(float(lon), 4), round(float(lat), 4)]

    def forward(self, lonlats: list) -> list[list[float]]:
        """Positions composées actuelles de points réels ``[lon, lat]`` (flèches des épingles)."""
        cw = self.warp()
        out = []
        for lon, lat in lonlats:
            x, y = cw.forward(float(lon), float(lat))
            out.append([round(float(x), 1), round(float(y), 1)])
        return out

    # -- rendu --------------------------------------------------------------
    def background_png(self) -> bytes:
        """Rendu ombré de ``heightmap.png`` (~2400 px), recalculé quand le relief change."""
        from cent_ans_tools.compose import relief
        from cent_ans_tools.geo import terrain

        source = self.map_dir / "heightmap.png"
        cache = self.cache_dir / "background.png"
        with self.lock:
            if cache.exists() and cache.stat().st_mtime >= source.stat().st_mtime:
                return cache.read_bytes()
            Image.MAX_IMAGE_PIXELS = None
            meta = json.loads((self.map_dir / "map.json").read_text("utf-8"))
            raw = terrain.read_png16(source).astype(np.float32)
            lo, hi = meta["height_min_m"], meta["height_max_m"]
            height = lo + raw * (hi - lo) / 65535.0
            buffer = io.BytesIO()
            relief.hillshade(height, BACKGROUND_WIDTH).save(buffer, "PNG")
            self.cache_dir.mkdir(parents=True, exist_ok=True)
            atomic_write(cache, buffer.getvalue())
            return buffer.getvalue()

    def background_info(self) -> dict:
        """Taille et version (mtime du relief) du fond, pour l'invalidation du cache navigateur."""
        source = self.map_dir / "heightmap.png"
        w, h = self.size_px()
        return {
            "url": "/api/background.png",
            "version": int(source.stat().st_mtime) if source.exists() else 0,
            "width_px": w,
            "height_px": h,
        }

    def base_inputs(self):  # noqa: ANN201
        """Entrées du relief lues une fois (terre, relief réel déformé, fleuves) ; lent la 1re fois."""
        from cent_ans_tools.compose import relief

        with self.lock:
            if self._inputs is None:
                cfg = relief.load_config()
                self._inputs = (relief.load_inputs(self.warp(), cfg), cfg)
            return self._inputs

    def render_preview(self, window: list[float], ridges: dict, nudges: dict) -> dict:
        """Relief de la zone ``window`` (px composés) avec les crêtes/retouches non enregistrées.

        Approximation volontaire : le relief est calculé sur la fenêtre élargie d'une marge, donc
        le bruit diffère légèrement de la compilation complète. ``compose-relief`` fait foi.
        """
        from cent_ans_tools.compose import nudges as nudge_mod
        from cent_ans_tools.compose import relief

        errors = self.validate("ridges", ridges) + self.validate("nudges", nudges)
        if errors:
            raise EditorError(HTTPStatus.UNPROCESSABLE_ENTITY, errors)
        t0 = time.time()
        inputs, cfg = self.base_inputs()
        height_full, width_full = inputs.land.shape
        x0, y0, x1, y1 = (int(round(v)) for v in window)
        x0, y0 = max(x0, 0), max(y0, 0)
        x1, y1 = min(x1, width_full), min(y1, height_full)
        if x1 - x0 < 16 or y1 - y0 < 16:
            raise EditorError(HTTPStatus.BAD_REQUEST, ["Fenêtre trop petite."])
        if max(x1 - x0, y1 - y0) > PREVIEW_MAX_SIDE:
            raise EditorError(
                HTTPStatus.BAD_REQUEST,
                [f"Fenêtre trop grande (max {PREVIEW_MAX_SIDE} px) : zoomer."],
            )
        cx0, cy0 = (
            max(x0 - PREVIEW_MARGIN, 0) // 2 * 2,
            max(y0 - PREVIEW_MARGIN, 0) // 2 * 2,
        )
        cx1, cy1 = (
            min(x1 + PREVIEW_MARGIN, width_full),
            min(y1 + PREVIEW_MARGIN, height_full),
        )
        places = nudge_mod.place_positions(
            self.warp(),
            self.settlements(),
            {n["id"]: tuple(n["target_px"]) for n in nudges["nudges"]},
        )
        inp = crop_inputs(inputs, ridges["ridges"], places, (cx0, cy0, cx1, cy1))
        height, _ = relief.build_height(inp, cfg)
        sub = np.ascontiguousarray(height[y0 - cy0 : y1 - cy0, x0 - cx0 : x1 - cx0])
        image = relief.hillshade(sub, min(x1 - x0, 1600))
        self._preview_seq += 1
        name = f"preview_{self._preview_seq % 8}.png"
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        image.save(self.cache_dir / name)
        return {
            "url": f"/api/previews/{name}?n={self._preview_seq}",
            "window": [x0, y0, x1, y1],
            "seconds": round(time.time() - t0, 1),
        }

    def preview_file(self, name: str) -> bytes:
        """PNG d'un aperçu déjà rendu (noms ``preview_N.png`` seulement)."""
        if not (
            name.startswith("preview_") and name.endswith(".png") and "/" not in name
        ):
            raise EditorError(HTTPStatus.NOT_FOUND, ["Aperçu inconnu."])
        path = self.cache_dir / name
        if not path.exists():
            raise EditorError(HTTPStatus.NOT_FOUND, ["Aperçu absent."])
        return path.read_bytes()

    # -- recompilation ------------------------------------------------------
    def pins_changed_since_warp(self) -> bool:
        """Vrai si ``pins.json`` est plus récent que ``warp.json`` (le cartogramme doit être refait)."""
        pins, warp = self.compose_dir / "pins.json", self.compose_dir / "warp.json"
        return (
            pins.exists()
            and warp.exists()
            and pins.stat().st_mtime > warp.stat().st_mtime
        )

    def start_recompile(self) -> dict:
        """Lance ``geo compose``, ``compose-relief``, ``compose paint`` puis ``geo compose-layers --only landcover,colormap`` en tâche de fond (409 si déjà en cours)."""
        with self.lock:
            if self.job.state == "running":
                raise EditorError(
                    HTTPStatus.CONFLICT, ["Une recompilation est déjà en cours."]
                )
            steps = self.recompile_steps
            if steps is None:
                base = [sys.executable, "-m", "cent_ans_tools.cli", "geo"]
                compose = [*base, "compose"]
                if self.pins_changed_since_warp():
                    compose.append("--rebuild-warp")
                steps = [
                    compose,
                    [*base, "compose-relief"],
                    [
                        sys.executable,
                        "-m",
                        "cent_ans_tools.cli",
                        "compose",
                        "paint",
                    ],
                    [*base, "compose-layers", "--only", "landcover,colormap"],
                ]
            self.job = Job(state="running", started=time.time())
            threading.Thread(
                target=self._run_steps, args=(self.job, steps), daemon=True
            ).start()
            return self.job.as_dict()

    def _run_steps(self, job: Job, steps: list[list[str]]) -> None:
        ok = True
        for command in steps:
            job.step = " ".join(command[3:] if len(command) > 3 else command)
            job.log.append(f"$ {' '.join(command)}")
            try:
                process = subprocess.Popen(
                    command,
                    cwd=REPO_DIR,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    bufsize=1,
                )
                assert process.stdout is not None
                for line in process.stdout:
                    job.log.append(line.rstrip())
                code = process.wait()
            except OSError as exc:
                job.log.append(f"Échec du lancement : {exc}")
                code = 1
            if code != 0:
                job.log.append(f"Étape en échec (code {code}).")
                ok = False
                break
        with self.lock:
            self._inputs = None  # la terre et le relief ont pu changer
            job.state = "done" if ok else "failed"
            job.finished = time.time()

    # -- vue d'ensemble -----------------------------------------------------
    def state(self) -> dict:
        """Tout ce dont la page a besoin au démarrage."""
        w, h = self.size_px()
        masks = {n: {"exists": self.mask_path(n).exists()} for n in MASK_NAMES}
        mw, mh = self.mask_shape()
        return {
            "size_px": [w, h],
            "meters_per_px": 719.0,
            "layers": {name: self.read_layer(name) for name in LAYERS},
            "places": self.places(),
            "masks": masks,
            "mask_cell_px": MASK_CELL_PX,
            "mask_size": [mw, mh],
            "background": self.background_info(),
            "job": self.job.as_dict(),
            "pins_need_warp": self.pins_changed_since_warp(),
            **self.geometry(),
        }


def crop_inputs(
    inputs, ridges: list[dict], places: np.ndarray, window: tuple[int, int, int, int]
):  # noqa: ANN001, ANN201
    """Sous-ensemble ``Inputs`` du relief sur ``window`` (coordonnées décalées vers l'origine)."""
    from cent_ans_tools.compose.relief import Inputs

    x0, y0, x1, y1 = window
    shift = np.array([x0, y0], dtype=np.float64)

    def meets(points: np.ndarray, margin: float = 0.0) -> bool:
        lo, hi = points.min(axis=0), points.max(axis=0)
        return bool(
            hi[0] >= x0 - margin
            and lo[0] <= x1 + margin
            and hi[1] >= y0 - margin
            and lo[1] <= y1 + margin
        )

    kept = []
    for ridge in ridges:
        pts = np.asarray(ridge["points"], dtype=np.float64)
        if not meets(pts, ridge["width_px"] * 2):
            continue
        copy = dict(ridge)
        copy["points"] = (pts - shift).tolist()
        copy["passes"] = [
            {**p, "at": [p["at"][0] - x0, p["at"][1] - y0]} for p in ridge["passes"]
        ]
        kept.append(copy)
    rivers = []
    for river in inputs.rivers:
        if meets(river["coords"]):
            rivers.append({**river, "coords": river["coords"] - shift})
    inside_places = places[
        (places[:, 0] >= x0)
        & (places[:, 0] < x1)
        & (places[:, 1] >= y0)
        & (places[:, 1] < y1)
    ]
    return Inputs(
        inputs.land[y0:y1, x0:x1],
        inputs.inside[y0:y1, x0:x1],
        inputs.real_height[y0:y1, x0:x1],
        kept,
        rivers,
        inside_places - shift,
        [
            {**item, "poly": item["poly"] - shift}
            for item in (inputs.plateaus or [])
            if meets(item["poly"], item["edge_px"] * 3.0)
        ],
        [
            {**item, "points": item["points"] - shift}
            for item in (inputs.troughs or [])
            if meets(item["points"], 300.0)
        ],
    )


# ----------------------------------------------------------------------------
# HTTP
# ----------------------------------------------------------------------------

JSON = "application/json; charset=utf-8"


def route(editor: Editor, method: str, url: str, body: bytes) -> tuple[int, str, bytes]:
    """Aiguille une requête ; renvoie ``(statut, type de contenu, corps)``."""
    parsed = urlparse(url)
    path = parsed.path.rstrip("/") or "/"
    query = parse_qs(parsed.query)

    def reply(obj, status: int = 200) -> tuple[int, str, bytes]:
        return status, JSON, json.dumps(obj, ensure_ascii=False).encode("utf-8")

    def json_body():  # noqa: ANN202
        try:
            return json.loads(body.decode("utf-8"))
        except ValueError as exc:
            raise EditorError(
                HTTPStatus.BAD_REQUEST, [f"JSON illisible : {exc}"]
            ) from exc

    write = method in ("POST", "PUT")
    parts = path.strip("/").split("/")
    if method == "GET":
        if path == "/":
            return (
                200,
                "text/html; charset=utf-8",
                (STATIC_DIR / "index.html").read_bytes(),
            )
        if path == "/api/state":
            return reply(editor.state())
        if path == "/api/places":
            return reply(editor.places())
        if path == "/api/geometry":
            return reply(editor.geometry())
        if path == "/api/job":
            return reply(editor.job.as_dict())
        if path == "/api/background.png":
            return 200, "image/png", editor.background_png()
        if path == "/api/lonlat":
            return reply(editor.lonlat(float(query["x"][0]), float(query["y"][0])))
        if len(parts) == 3 and parts[:2] == ["api", "layers"]:
            return reply(editor.read_layer(parts[2]))
        if (
            len(parts) == 3
            and parts[:2] == ["api", "masks"]
            and parts[2].endswith(".png")
        ):
            return 200, "image/png", editor.read_mask(parts[2][:-4])
        if (
            len(parts) == 3
            and parts[:2] == ["api", "cover"]
            and parts[2].endswith(".png")
        ):
            return 200, "image/png", editor.cover_png(parts[2][:-4])
        if len(parts) == 3 and parts[:2] == ["api", "previews"]:
            return 200, "image/png", editor.preview_file(parts[2])
    elif write:
        if len(parts) == 3 and parts[:2] == ["api", "layers"]:
            doc = editor.save_layer(parts[2], json_body())
            return reply({"ok": True, "layer": doc})
        if (
            len(parts) == 3
            and parts[:2] == ["api", "masks"]
            and parts[2].endswith(".png")
        ):
            return reply(editor.save_mask(parts[2][:-4], body))
        if path == "/api/preview":
            data = json_body()
            return reply(
                editor.render_preview(data["window"], data["ridges"], data["nudges"])
            )
        if path == "/api/forward":
            return reply(editor.forward(json_body()))
        if path == "/api/recompile":
            return reply(editor.start_recompile())
    raise EditorError(HTTPStatus.NOT_FOUND, [f"Route inconnue : {method} {path}"])


def make_handler(editor: Editor) -> type[BaseHTTPRequestHandler]:
    """Classe de gestionnaire liée à ``editor``."""

    class Handler(BaseHTTPRequestHandler):
        def _serve(self, method: str) -> None:
            try:
                if method in ("POST", "PUT") and not self.headers.get(GUARD_HEADER):
                    raise EditorError(
                        HTTPStatus.FORBIDDEN, [f"En-tête {GUARD_HEADER} requis."]
                    )
                length = int(self.headers.get("Content-Length") or 0)
                if length > MAX_BODY:
                    raise EditorError(
                        HTTPStatus.REQUEST_ENTITY_TOO_LARGE, ["Corps trop gros."]
                    )
                body = self.rfile.read(length) if length else b""
                status, ctype, payload = route(editor, method, self.path, body)
            except EditorError as exc:
                status, ctype = exc.status, JSON
                payload = json.dumps(
                    {"ok": False, "errors": exc.errors}, ensure_ascii=False
                ).encode("utf-8")
            except (KeyError, ValueError, TypeError) as exc:
                status, ctype = HTTPStatus.BAD_REQUEST, JSON
                payload = json.dumps(
                    {"ok": False, "errors": [f"Requête invalide : {exc!r}"]}
                ).encode("utf-8")
            except Exception as exc:  # noqa: BLE001 - le serveur ne doit pas tomber sur une erreur de calcul
                traceback.print_exc()
                status, ctype = HTTPStatus.INTERNAL_SERVER_ERROR, JSON
                payload = json.dumps(
                    {"ok": False, "errors": [f"Erreur interne : {exc!r}"]}
                ).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(payload)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(payload)

        def do_GET(self) -> None:  # noqa: N802, D102
            self._serve("GET")

        def do_POST(self) -> None:  # noqa: N802, D102
            self._serve("POST")

        def do_PUT(self) -> None:  # noqa: N802, D102
            self._serve("PUT")

        def log_message(self, fmt: str, *args) -> None:  # noqa: ANN002, D102
            pass

    return Handler


def make_server(editor: Editor, port: int = 8766) -> ThreadingHTTPServer:
    """Serveur lié à 127.0.0.1 (port 0 = libre)."""
    return ThreadingHTTPServer(("127.0.0.1", port), make_handler(editor))
