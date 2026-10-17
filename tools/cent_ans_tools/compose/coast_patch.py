"""Reporte une retouche de côte sur le relief déjà fabriqué (lot mapgeo11).

Une retouche d'auteur de la côte (``generalise.json`` : ``inlets``, ``headlands``) change
``land_mask.png``. Le relief composé (``height/`` = E0, ``pyramid/E1-E3``) vient d'une chaîne lourde
(``compose-relief-real``, 15-30 min, 8 Go) dont les réglages ont dérivé depuis la dernière
fabrication : la rejouer entière changerait tout le relief. Ce module ne touche que les pixels de
relief autour de la différence entre l'ancien et le nouveau masque de terre :

- terre devenue eau : la hauteur passe sous le niveau de la mer, en pente douce depuis la nouvelle
  rive (``wet_slope_m_per_px``, plancher ``wet_floor_m``) ;
- eau devenue terre : la hauteur monte au-dessus de la mer (``dry_slope_m_per_px``, plafond) ;
- terre voisine d'une zone creusée : la hauteur est plafonnée par une rampe depuis la rive, pour
  ne pas laisser de falaise le long du nouveau rivage.

Les ombrages et ``heightmap*.png`` se refont ensuite par ``geo compose-relief-real --map-files-only``
(à partir des tuiles E0 retouchées et du nouveau masque).
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.geo import terrain

Image.MAX_IMAGE_PIXELS = None

TILE_PX = 512


@dataclass(frozen=True)
class PatchRules:
    """Profil de hauteur autour de la nouvelle rive (pixels composés = 719 m nominaux)."""

    wet_slope_m_per_px: float = 10.0
    wet_floor_m: float = -70.0
    dry_slope_m_per_px: float = 8.0
    dry_cap_m: float = 40.0
    ramp_base_m: float = 8.0
    ramp_slope_m_per_px: float = 25.0
    ramp_reach_px: float = 6.0
    margin_px: int = 48


def change_masks(
    old_land: np.ndarray, new_land: np.ndarray
) -> tuple[np.ndarray, np.ndarray]:
    """``(carved, added)`` : terre devenue eau, eau devenue terre (booléens)."""
    return old_land & ~new_land, ~old_land & new_land


def changed_box(
    carved: np.ndarray, added: np.ndarray, margin: int
) -> tuple[int, int, int, int] | None:
    """Boîte ``(x0, y0, x1, y1)`` en pixels composés autour des changements, ou ``None``."""
    ys, xs = np.nonzero(carved | added)
    if len(xs) == 0:
        return None
    h, w = carved.shape
    return (
        max(int(xs.min()) - margin, 0),
        max(int(ys.min()) - margin, 0),
        min(int(xs.max()) + margin + 1, w),
        min(int(ys.max()) + margin + 1, h),
    )


class CoastField:
    """Champs composés (distance signée à la nouvelle rive, voisinage des zones creusées)."""

    def __init__(
        self, old_land: np.ndarray, new_land: np.ndarray, rules: PatchRules
    ) -> None:
        """Calcule les champs sur la boîte des changements (une seule fois)."""
        self.rules = rules
        carved, added = change_masks(old_land, new_land)
        box = changed_box(carved, added, rules.margin_px)
        self.box = box
        if box is None:
            return
        x0, y0, x1, y1 = box
        sl = (slice(y0, y1), slice(x0, x1))
        new = new_land[sl]
        self.old = old_land[sl]
        self.new = new
        self.carved = carved[sl]
        self.added = added[sl]
        # distance signée (> 0 sur la terre, < 0 dans l'eau), en pixels composés
        self.signed = (
            ndimage.distance_transform_edt(new)
            - ndimage.distance_transform_edt(~new)
        ).astype(np.float32)
        self.near_carved = ndimage.distance_transform_edt(~self.carved).astype(
            np.float32
        )

    def tile_box(self, scale: int) -> tuple[int, int, int, int] | None:
        """Boîte des changements en pixels fins d'un niveau de ``scale`` px par pixel composé."""
        if self.box is None:
            return None
        x0, y0, x1, y1 = self.box
        return x0 * scale, y0 * scale, x1 * scale, y1 * scale

    def patch_window(
        self, heights: np.ndarray, fx0: int, fy0: int, scale: int
    ) -> np.ndarray:
        """Retouche une fenêtre de hauteurs (m) dont le coin est ``(fx0, fy0)`` en pixels fins."""
        if self.box is None:
            return heights
        bx, by, _, _ = self.box
        h, w = heights.shape
        # centre de chaque pixel fin dans le repère des pixels composés (centres en entiers)
        u = (np.arange(fx0, fx0 + w) + 0.5) / scale - 0.5 - bx
        v = (np.arange(fy0, fy0 + h) + 0.5) / scale - 0.5 - by
        gv, gu = np.meshgrid(v, u, indexing="ij")
        coords = np.stack([gv, gu])
        inside = (
            (gv >= 0)
            & (gu >= 0)
            & (gv <= self.signed.shape[0] - 1)
            & (gu <= self.signed.shape[1] - 1)
        )
        if not inside.any():
            return heights
        signed = ndimage.map_coordinates(self.signed, coords, order=1, mode="nearest")
        near = ndimage.map_coordinates(self.near_carved, coords, order=1, mode="nearest")
        nearest = (
            np.rint(coords).astype(np.int64).clip(0, None)
        )  # pixel composé le plus proche
        ny = nearest[0].clip(0, self.old.shape[0] - 1)
        nx = nearest[1].clip(0, self.old.shape[1] - 1)
        was_land = self.old[ny, nx]
        was_sea = ~was_land
        rules = self.rules
        out = heights.astype(np.float32).copy()
        wet = np.maximum(signed * rules.wet_slope_m_per_px - 0.5, rules.wet_floor_m)
        carve_zone = inside & was_land & (signed < 0.0)
        out = np.where(carve_zone, np.minimum(out, wet), out)
        dry = np.minimum(
            signed * rules.dry_slope_m_per_px + 1.0, rules.dry_cap_m
        ).astype(np.float32)
        add_zone = inside & was_sea & (signed > 0.0)
        out = np.where(add_zone, np.maximum(out, dry), out)
        ramp = rules.ramp_base_m + signed * rules.ramp_slope_m_per_px
        ramp_zone = (
            inside
            & was_land
            & (signed >= 0.0)
            & (signed < rules.ramp_reach_px)
            & (near < rules.ramp_reach_px)
        )
        out = np.where(ramp_zone, np.minimum(out, ramp), out)
        return out


def inlet_windows(
    cw, shapes: list[dict], margin_px: int = 60
) -> list[tuple[int, int, int, int]]:
    """Fenêtres ``(x0, y0, x1, y1)`` (pixels composés) autour de chaque estuaire ou baie d'auteur."""
    windows = []
    for item in shapes:
        lon, lat = np.array(item["path"], dtype=float).T
        x, y = cw.forward(lon, lat)
        reach = max(item["widths_km"]) / 0.719 / 2.0
        pad = margin_px + reach
        windows.append(
            (
                int(np.floor(x.min() - pad)),
                int(np.floor(y.min() - pad)),
                int(np.ceil(x.max() + pad)),
                int(np.ceil(y.max() + pad)),
            )
        )
    return windows


def splice_land(
    old_land: np.ndarray, new_land: np.ndarray, windows: list[tuple[int, int, int, int]]
) -> np.ndarray:
    """Masque ancien, sauf dans les fenêtres où l'on prend le nouveau (dérive de lissage ailleurs ignorée)."""
    out = old_land.copy()
    h, w = old_land.shape
    for x0, y0, x1, y1 in windows:
        sl = (slice(max(y0, 0), min(y1, h)), slice(max(x0, 0), min(x1, w)))
        out[sl] = new_land[sl]
    return out


def splice_coastline(old_doc: dict, new_doc: dict, windows: list[tuple[int, int, int, int]]) -> dict:
    """Côtes : les anneaux inchangés restent ; l'anneau modifié = ancien hors fenêtres + nouveau dedans."""
    from shapely.geometry import Polygon, box
    from shapely.ops import unary_union

    cut = unary_union([box(*w) for w in windows]) if windows else Polygon()
    old_rings = {
        json.dumps(f["geometry"]["coordinates"]): f for f in old_doc["features"]
    }
    out = json.loads(json.dumps(new_doc))
    features = []
    for feat in new_doc["features"]:
        key = json.dumps(feat["geometry"]["coordinates"])
        if key in old_rings or not windows:
            features.append(feat)
            continue
        new_poly = Polygon(feat["geometry"]["coordinates"]).buffer(0)
        # l'ancien anneau qui recouvre le plus le nouveau
        best = max(
            (f for f in old_doc["features"] if len(f["geometry"]["coordinates"]) > 3),
            key=lambda f: Polygon(f["geometry"]["coordinates"])
            .buffer(0)
            .intersection(new_poly)
            .area,
        )
        old_poly = Polygon(best["geometry"]["coordinates"]).buffer(0)
        merged = old_poly.difference(cut).union(new_poly.intersection(cut))
        parts = [merged] if merged.geom_type == "Polygon" else list(merged.geoms)
        ring = max(parts, key=lambda p: p.area).exterior
        coords = [[round(x, 1), round(y, 1)] for x, y in ring.coords]
        features.append({**feat, "geometry": {"type": "LineString", "coordinates": coords}})
    out["features"] = features
    return out


def _read_tile(path: Path) -> np.ndarray | None:
    if not path.exists():
        return None
    return np.asarray(Image.open(path)).astype(np.uint16)


def patch_tiles(
    field: CoastField, tiles_dir: Path, pattern: str, scale: int, dry_run: bool = False
) -> int:
    """Retouche les tuiles 512 px d'un niveau ; renvoie le nombre de tuiles réécrites."""
    box = field.tile_box(scale)
    if box is None:
        return 0
    x0, y0, x1, y1 = box
    done = 0
    for row in range(y0 // TILE_PX, (y1 - 1) // TILE_PX + 1):
        for col in range(x0 // TILE_PX, (x1 - 1) // TILE_PX + 1):
            path = tiles_dir / pattern.format(col=col, row=row)
            tile = _read_tile(path)
            if tile is None:
                continue
            metres = terrain.uint16_to_height(tile)
            patched = field.patch_window(metres, col * TILE_PX, row * TILE_PX, scale)
            encoded = terrain.height_to_uint16(patched)
            if np.array_equal(encoded, tile):
                continue
            done += 1
            if not dry_run:
                terrain.write_png16(encoded, path)
    return done


def splice(old_land_path: Path, old_coast_path: Path, map_dir: Path, windows_cfg: list[dict]) -> dict:
    """Remplace ``land_mask.png`` et ``coastline.geojson`` (sortie de ``geo compose``) par l'ancien état
    hors des fenêtres autour des estuaires d'auteur ; renvoie un rapport."""
    from cent_ans_tools.compose.contract import COMPOSE_DIR, ComposedWarp

    cw = ComposedWarp.load(COMPOSE_DIR)
    windows = inlet_windows(cw, windows_cfg)
    old_land = np.asarray(Image.open(old_land_path)) > 0
    new_land = np.asarray(Image.open(map_dir / "land_mask.png")) > 0
    merged = splice_land(old_land, new_land, windows)
    Image.fromarray(merged.astype(np.uint8) * 255, mode="L").save(
        map_dir / "land_mask.png", optimize=True
    )
    old_doc = json.loads(old_coast_path.read_text(encoding="utf-8"))
    new_doc = json.loads((map_dir / "coastline.geojson").read_text(encoding="utf-8"))
    doc = splice_coastline(old_doc, new_doc, windows)
    (map_dir / "coastline.geojson").write_text(
        json.dumps(doc, separators=(",", ":")) + "\n", encoding="utf-8"
    )
    return {"windows": windows, "px_changed_vs_old": int((merged != old_land).sum())}


def patch_image(field: CoastField, path: Path, dry_run: bool = False) -> bool:
    """Retouche un PNG 16 bits à la résolution de la carte (``heightmap.png``, ``heightmap_render.png``)."""
    box = field.box
    if box is None or not path.exists():
        return False
    x0, y0, x1, y1 = box
    image = np.asarray(Image.open(path)).astype(np.uint16)
    window = image[y0:y1, x0:x1]
    patched = terrain.height_to_uint16(
        field.patch_window(terrain.uint16_to_height(window), x0, y0, 1)
    )
    if np.array_equal(patched, window):
        return False
    if not dry_run:
        image[y0:y1, x0:x1] = patched
        terrain.write_png16(image, path)
    return True


def apply(
    old_land_path: Path,
    new_land_path: Path,
    map_dir: Path,
    pyramid_dir: Path | None,
    rules: PatchRules | None = None,
    dry_run: bool = False,
) -> dict:
    """Retouche E0 (``map_dir/height``) et, si donnée, la pyramide E1-E3 ; renvoie un rapport."""
    rules = rules or PatchRules()
    old_land = np.asarray(Image.open(old_land_path)) > 0
    new_land = np.asarray(Image.open(new_land_path)) > 0
    field = CoastField(old_land, new_land, rules)
    carved, added = change_masks(old_land, new_land)
    report: dict = {
        "carved_px": int(carved.sum()),
        "added_px": int(added.sum()),
        "box_px": field.box,
        "tiles": {},
    }
    meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))["height_tiles"]
    report["tiles"]["E0"] = patch_tiles(
        field, map_dir / meta["dir"], meta["pattern"], 2, dry_run
    )
    report["images"] = {
        name: patch_image(field, map_dir / name, dry_run)
        for name in ("heightmap.png", "heightmap_render.png")
    }
    if pyramid_dir is not None:
        for level, scale in ((1, 4), (2, 8), (3, 16)):
            report["tiles"][f"E{level}"] = patch_tiles(
                field,
                pyramid_dir / f"E{level}",
                "{col}_{row}.png",
                scale,
                dry_run,
            )
    return report
