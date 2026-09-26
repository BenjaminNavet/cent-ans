"""Battle captures and mean HSV saturation of the 3D scene (DA6, DA7b; art bible § 3.3).

The art bible caps the mean HSV saturation of terrain and buildings at 35 % in full daylight.
This tool reproduces the DA6 measurement: battle screenshots at 1600 × 900 for a fixed set of
views, then the mean HSV saturation of the 3D area below the horizon (the banner, the log and
the bottom bars are left out).

Usage:
    uv run --project tools python -m cent_ans_tools.scene_saturation measure docs/img/da6/*.jpg
    uv run --project tools python -m cent_ans_tools.scene_saturation capture OUT_DIR \
        [--prefix apres_] [--views automne,bocage] [--extra=--no-da6]
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image

REPO_ROOT = Path(__file__).resolve().parents[2]
CAPTURE_SIZE = (1600, 900)
BIBLE_MAX_SATURATION = 0.35

# Battle options (after `--`) of each DA6 view, in the order of the DA6 table.
VIEWS: dict[str, list[str]] = {
    "closeup": ["--closeup", "--shot-at=45"],
    "foot": ["--standard-shot=foot"],
    "haute": ["--shot-at=40"],
    "ligne": ["--standard-shot=line"],
    "hiver": ["--season=winter", "--closeup", "--shot-at=45"],
    "automne": ["--season=autumn", "--closeup", "--shot-at=45"],
    "bocage": ["--terrain=bocage", "--closeup", "--shot-at=45"],
    "bois": ["--camera=200,230,45,250", "--shot-at=45", "--no-hud"],
    "bois_hiver": [
        "--season=winter",
        "--camera=200,230,45,250",
        "--shot-at=45",
        "--no-hud",
    ],
}


@dataclass(frozen=True)
class SaturationStats:
    """Mean and 90th percentile HSV saturation (0-1) and mean HSV value of the measured area."""

    mean: float
    p90: float
    value: float


def measured_pixels(rgb: np.ndarray) -> np.ndarray:
    """Pixels (N, 3) of the 3D area below the horizon of a 1600 × 900 battle capture.

    Two rectangles, as in DA6: the centre-right field (rows 260-650, columns 560-1600) and a
    band just below the horizon (rows 180-260, columns 0-1240). Other sizes are rescaled.
    """
    height, width = rgb.shape[:2]
    sy, sx = height / CAPTURE_SIZE[1], width / CAPTURE_SIZE[0]

    def box(y0: int, y1: int, x0: int, x1: int) -> np.ndarray:
        return rgb[int(y0 * sy) : int(y1 * sy), int(x0 * sx) : int(x1 * sx)].reshape(
            -1, 3
        )

    return np.concatenate([box(260, 650, 560, 1600), box(180, 260, 0, 1240)])


def saturation_stats(rgb: np.ndarray) -> SaturationStats:
    """HSV saturation statistics of an RGB image (uint8 or float in 0-1)."""
    pixels = measured_pixels(rgb).astype(float)
    if rgb.dtype == np.uint8:
        pixels /= 255.0
    high = pixels.max(axis=1)
    low = pixels.min(axis=1)
    saturation = np.where(high > 0, (high - low) / np.maximum(high, 1e-6), 0.0)
    return SaturationStats(
        float(saturation.mean()),
        float(np.percentile(saturation, 90)),
        float(high.mean()),
    )


def measure_file(path: Path) -> SaturationStats:
    """Statistics of one capture file."""
    return saturation_stats(np.array(Image.open(path).convert("RGB")))


def capture(
    out_dir: Path, views: list[str], prefix: str, extra: list[str], godot: str = "godot"
) -> list[Path]:
    """Captures each view with the battle scene (PNG), then writes a JPEG next to it."""
    out_dir.mkdir(parents=True, exist_ok=True)
    written = []
    for view in views:
        png = (out_dir / f"{prefix}{view}.png").resolve()
        command = [
            godot,
            "--resolution",
            f"{CAPTURE_SIZE[0]}x{CAPTURE_SIZE[1]}",
            "--path",
            str(REPO_ROOT / "game"),
            "res://scenes/battle/battle.tscn",
            "--",
            *VIEWS[view],
            *extra,
            f"--screenshot={png}",
        ]
        subprocess.run(
            command, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        )
        if not png.exists():
            print(f"{view}: no capture", file=sys.stderr)
            continue
        jpg = png.with_suffix(".jpg")
        Image.open(png).convert("RGB").save(jpg, quality=88)
        png.unlink()
        written.append(jpg)
    return written


def report(paths: list[Path]) -> int:
    """Prints one line per capture; returns the number of captures above the bible cap."""
    over = 0
    for path in paths:
        stats = measure_file(path)
        flag = " > 35 %" if stats.mean > BIBLE_MAX_SATURATION else ""
        print(
            f"{path.name}: sat moy {stats.mean * 100:.1f} %, p90 {stats.p90 * 100:.1f} %, val moy {stats.value:.2f}{flag}"
        )
        over += stats.mean > BIBLE_MAX_SATURATION
    return over


def main(argv: list[str] | None = None) -> int:
    """Command line entry point."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    measure_parser = sub.add_parser("measure", help="measure captures")
    measure_parser.add_argument("files", nargs="+", type=Path)
    capture_parser = sub.add_parser(
        "capture", help="capture the DA6 views, then measure them"
    )
    capture_parser.add_argument("out_dir", type=Path)
    capture_parser.add_argument("--prefix", default="")
    capture_parser.add_argument("--views", default=",".join(VIEWS))
    capture_parser.add_argument(
        "--extra", action="append", default=[], help="extra battle option"
    )
    capture_parser.add_argument("--godot", default="godot")
    args = parser.parse_args(argv)
    if args.command == "measure":
        report(args.files)
        return 0
    views = [view for view in args.views.split(",") if view]
    unknown = [view for view in views if view not in VIEWS]
    if unknown:
        parser.error(f"unknown views: {', '.join(unknown)}")
    report(capture(args.out_dir, views, args.prefix, args.extra, args.godot))
    return 0


if __name__ == "__main__":
    sys.exit(main())
