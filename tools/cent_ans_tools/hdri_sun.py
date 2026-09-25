"""Locate the sun in equirectangular Radiance HDR skies (lot V3, A1-05).

Prints, for each `.hdr`, the direction of the brightest region (azimuth and elevation in degrees,
Godot convention: azimuth 0 = -Z, growing towards +X) and the mean sky luminance, so that
`data/fx/atmosphere.json` can align the directional light with the sun painted in the panorama.

Usage: uv run --project tools python -m cent_ans_tools.hdri_sun game/assets/third_party/skies/*/*.hdr
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import numpy as np


def read_hdr(path: Path) -> np.ndarray:
    """Reads a Radiance RGBE file (new-style RLE or flat) into a float32 (H, W, 3) array."""
    data = path.read_bytes()
    header_end = data.index(b"\n\n") + 2
    line_end = data.index(b"\n", header_end)
    resolution = data[header_end:line_end].decode()
    match = re.match(r"-Y (\d+) \+X (\d+)", resolution)
    if match is None:
        raise ValueError(f"unsupported resolution line: {resolution}")
    height, width = int(match.group(1)), int(match.group(2))
    buffer = np.frombuffer(data, dtype=np.uint8, offset=line_end + 1)
    rgbe = np.zeros((height, width, 4), dtype=np.uint8)
    position = 0
    for row in range(height):
        if buffer[position] == 2 and buffer[position + 1] == 2:
            position += 4
            for channel in range(4):
                column = 0
                while column < width:
                    count = int(buffer[position])
                    position += 1
                    if count > 128:
                        count -= 128
                        rgbe[row, column : column + count, channel] = buffer[position]
                        position += 1
                    else:
                        rgbe[row, column : column + count, channel] = buffer[
                            position : position + count
                        ]
                        position += count
                    column += count
        else:
            rgbe[row] = buffer[position : position + width * 4].reshape(width, 4)
            position += width * 4
    exponent = rgbe[..., 3].astype(np.int32)
    scale = np.where(exponent > 0, np.ldexp(1.0, exponent - 136), 0.0)
    return (rgbe[..., :3].astype(np.float64) * scale[..., None]).astype(np.float32)


def sun_direction(image: np.ndarray) -> tuple[float, float, float]:
    """Returns (azimuth_deg, elevation_deg, peak_to_mean ratio) of the brightest blob."""
    height, width, _ = image.shape
    luminance = image @ np.array([0.2126, 0.7152, 0.0722], dtype=np.float32)
    upper = luminance[: height // 2]
    row, column = np.unravel_index(np.argmax(upper), upper.shape)
    elevation = 90.0 - (row + 0.5) / height * 180.0
    # Godot panorama: u = 0.5 looks towards -Z; u grows towards +X (azimuth east of north).
    azimuth = ((column + 0.5) / width - 0.5) * 360.0
    ratio = float(upper.max() / max(upper.mean(), 1e-6))
    return azimuth % 360.0, elevation, ratio


def main(paths: list[str]) -> None:
    """Prints the sun direction of every HDR file given on the command line."""
    for name in paths:
        image = read_hdr(Path(name))
        azimuth, elevation, ratio = sun_direction(image)
        mean = float(image[: image.shape[0] // 2].mean())
        sunny = "sun" if ratio > 50.0 else "diffuse"
        print(
            f"{Path(name).stem}: azimuth {azimuth:6.1f}  elevation {elevation:5.1f}  peak/mean {ratio:8.1f} ({sunny})  mean {mean:.3f}"
        )


if __name__ == "__main__":
    main(sys.argv[1:])
