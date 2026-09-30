"""Lot NT13: contact sheets of the video retargeting (keyframed / CMU / video / source frame).

Reads the renders of ``nt13_video_trial.py -- render DIR`` (``<clip>_<k|m|v>_<i>.png`` and
``frames.json``), grabs the matching source video frames with ffmpeg, and writes one sheet per
clip. Everything stays outside the repository (personal videos)::

    uv run --project tools python tools/video_mocap/contact_sheet.py RENDER_DIR VIDEO_DIR OUT_DIR
"""

import json
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROWS = (
    ("k", "keyframé"),
    ("m", "CMU (NT12)"),
    ("v", "vidéo (NT13)"),
    ("c", "NT13, vue caméra"),
    ("src", "source"),
)
CELL = (270, 360)


def video_frame(video: Path, index: int, out: Path) -> Path | None:
    """Frame `index` of `video` as a PNG (cached in `out`)."""
    path = out / f"{video.stem}_{index:04d}.png"
    if not path.exists():
        subprocess.run(
            [
                "ffmpeg",
                "-v",
                "error",
                "-y",
                "-i",
                str(video),
                "-vf",
                f"select=eq(n\\,{index}),scale=-2:{CELL[1]}",
                "-frames:v",
                "1",
                str(path),
            ],
            check=False,
        )
    return path if path.exists() else None


def fit(img: Image.Image) -> Image.Image:
    """`img` scaled into a cell, centred on a grey background."""
    img = img.convert("RGB")
    img.thumbnail(CELL)
    cell = Image.new("RGB", CELL, (60, 60, 60))
    cell.paste(img, ((CELL[0] - img.width) // 2, (CELL[1] - img.height) // 2))
    return cell


def main() -> int:
    """Write ``nt13_<clip>.png`` for every clip of the render index."""
    render, videos, out = (Path(a) for a in sys.argv[1:4])
    out.mkdir(parents=True, exist_ok=True)
    index = json.loads((render / "frames.json").read_text())
    clips = sorted({key.rsplit("_", 1)[0] for key in index})
    for clip in clips:
        cols = sorted(
            int(k.rsplit("_", 1)[1]) for k in index if k.rsplit("_", 1)[0] == clip
        )
        sheet = Image.new(
            "RGB", (110 + CELL[0] * len(cols), CELL[1] * len(ROWS)), (30, 30, 30)
        )
        draw = ImageDraw.Draw(sheet)
        for r, (tag, label) in enumerate(ROWS):
            draw.text((8, r * CELL[1] + CELL[1] // 2), label, fill=(230, 230, 230))
            for c, i in enumerate(cols):
                if tag == "src":
                    video, frame = index[f"{clip}_{i}"]
                    path = video_frame(videos / f"{video}.MOV", frame, render)
                else:
                    path = render / f"{clip}_{tag}_{i}.png"
                if path and path.exists():
                    sheet.paste(fit(Image.open(path)), (110 + c * CELL[0], r * CELL[1]))
        sheet.save(out / f"nt13_{clip}.png")
        print("OK", out / f"nt13_{clip}.png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
