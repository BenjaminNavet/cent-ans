"""Before/after previews of lot ZG5a (``docs/img/zg5a/``).

Shaded relief of one pyramid level around a place, with the Natural Earth rivers
of ``rivers.geojson`` (red, before) and the fine network of ``rivers_fine.json``
(blue, after), optionally the draped roads (brown) and the fine anchors.
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from pyproj import Transformer

from cent_ans_tools.geo import fine_relief, fine_tiles, pyramid

BEFORE = (220, 40, 40)
AFTER = (30, 90, 230)
ROAD = (150, 90, 30)


def hillshade(heights: np.ndarray, pixel_m: float) -> np.ndarray:
    """Grey shading (0-255) lit from the north-west, with a light hypsometric tint."""
    gy, gx = np.gradient(np.nan_to_num(heights, nan=0.0), pixel_m)
    slope = np.pi / 2.0 - np.arctan(np.hypot(gx, gy) * 2.0)
    aspect = np.arctan2(-gx, gy)
    azimuth, altitude = np.radians(315.0), np.radians(45.0)
    shade = np.sin(altitude) * np.sin(slope) + np.cos(altitude) * np.cos(
        slope
    ) * np.cos(azimuth - aspect)
    return np.clip(shade * 255.0, 0, 255)


def render(
    map_dir: Path,
    lonlat: tuple[float, float],
    half_km: float,
    level: int,
    out: Path,
    roads: bool = False,
) -> Path:
    """Write one preview image.

    Args:
        map_dir: ``data/map``.
        lonlat: Centre.
        half_km: Half side of the square.
        level: Pyramid level of the shaded relief (finest available used under it).
        out: Output image.
        roads: Also draw the draped roads of ``fine_anchors.json``.
    """
    bounds = pyramid.map_bounds(map_dir)
    relief = fine_relief.FineRelief(map_dir, bounds, max_level=level)
    cx, cy = Transformer.from_crs("EPSG:4326", "EPSG:3035", always_xy=True).transform(
        *lonlat
    )
    pixel = relief.pixel_m(level)
    size = int(2 * half_km * 1000 / pixel)
    xs = cx - half_km * 1000 + (np.arange(size) + 0.5) * pixel
    ys = cy + half_km * 1000 - (np.arange(size) + 0.5) * pixel
    gx, gy = np.meshgrid(xs, ys)
    heights = relief.sample(gx, gy)
    shade = hillshade(heights, pixel)
    lo, hi = np.nanpercentile(heights, [2, 98])
    tint = np.clip((heights - lo) / max(hi - lo, 1.0), 0, 1)
    rgb = np.stack(
        [
            shade * (0.75 + 0.25 * tint),
            shade * (0.8 + 0.15 * tint),
            shade * (0.7 + 0.1 * tint),
        ],
        axis=-1,
    )
    rgb[heights <= 0.5] = [150, 180, 205]
    image = Image.fromarray(np.nan_to_num(rgb).astype(np.uint8), "RGB")
    draw = ImageDraw.Draw(image)
    mpp = (bounds[2] - bounds[0]) / pyramid.frame_width_units(bounds)
    x0 = (cx - half_km * 1000 - bounds[0]) / mpp
    y0 = (bounds[3] - (cy + half_km * 1000)) / mpp
    scale = mpp / pixel  # image pixels per world unit

    def to_image(units: np.ndarray) -> list[tuple[float, float]]:
        return [((u - x0) * scale, (v - y0) * scale) for u, v in units]

    rivers = json.loads((map_dir / "rivers.geojson").read_text(encoding="utf-8"))
    for feature in rivers["features"]:
        geometry = feature["geometry"]
        parts = (
            [geometry["coordinates"]]
            if geometry["type"] == "LineString"
            else geometry["coordinates"]
        )
        for part in parts:
            draw.line(to_image(np.asarray(part)), fill=BEFORE, width=2)
    side = fine_tiles.tile_units(fine_tiles.TILE_LEVEL)
    tile_dir = map_dir / pyramid.PYRAMID_DIR_NAME
    layers = [("hydro_fine", AFTER)]
    if roads:
        layers.append(("roads_fine", ROAD))
    extent_units = 2 * half_km * 1000 / mpp
    for sub, colour in layers:
        for col in range(int(x0 // side), int((x0 + extent_units) // side) + 1):
            for row in range(int(y0 // side), int((y0 + extent_units) // side) + 1):
                path = tile_dir / sub / f"E{fine_tiles.TILE_LEVEL}" / f"{col}_{row}.bin"
                if not path.exists():
                    continue
                for line in fine_tiles.decode(path.read_bytes())["lines"]:
                    width_px = max(1, int(round(float(np.median(line["w"])) / pixel)))
                    draw.line(
                        to_image(line["xy"]), fill=colour, width=min(width_px, 12)
                    )
    out.parent.mkdir(parents=True, exist_ok=True)
    image.save(out, quality=88)
    return out


PLACES = {
    "rouen": ((1.07, 49.44), 9.0, 6),
    "orleans": ((1.905, 47.90), 9.0, 6),
    "bordeaux": ((-0.56, 44.85), 10.0, 5),
    "londres": ((-0.10, 51.505), 9.0, 6),
}


def render_all(map_dir: Path, out_dir: Path, roads: bool = True) -> list[Path]:
    """Previews of :data:`PLACES`, at E4 and at the detail level."""
    written = []
    for name, (lonlat, half_km, level) in PLACES.items():
        written.append(
            render(map_dir, lonlat, half_km, 4, out_dir / f"{name}_E4.jpg", roads)
        )
        written.append(
            render(
                map_dir,
                lonlat,
                half_km / 3.0,
                level,
                out_dir / f"{name}_E{level}.jpg",
                roads,
            )
        )
    return written
