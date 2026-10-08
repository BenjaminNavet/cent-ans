"""Chronicle event miniatures generated through OpenRouter.

One wide illumination per event of ``data/events`` (historical, random and chained),
shown as a banner at the top of the chronicle window. The prompt is built from the
event data only (title, period text, date, place); images are centre-cropped to a
16:9 band and saved as ``game/assets/events/<event id>.jpg`` (768x432, JPEG).

The paid batch reuses :func:`cent_ans_tools.portraits.generate` (idempotence, task
envelope, global cap of ``docs/budget.md``, one budget row per batch).
"""

from __future__ import annotations

import io
import json
from pathlib import Path

from PIL import Image

from cent_ans_tools.portraits import DATA_DIR, REPO_DIR, PortraitJob

EVENTS_ART_DIR = REPO_DIR / "game" / "assets" / "events"
ART_WIDTH = 768
ART_HEIGHT = 432
# Share of the cropped height taken above the band (heads sit high in miniatures).
TOP_BIAS = 0.35
JPEG_QUALITY = 85

STYLE = (
    "Style: 14th-15th-century French Gothic manuscript miniature (enluminure de "
    "chronique, in the manner of Froissart's Chronicles and the Grandes Chroniques "
    "de France). Wide landscape scene: every figure, head and important action fits "
    "inside the middle 55 % of the image height, with patterned sky above and ground "
    "below (the top and bottom will be cropped); figures shown whole and not too "
    "large. Flat "
    "gilded or diapered azure background sky, egg tempera colours, fine black ink "
    "outlines, gold leaf highlights, period-accurate clothing, armour and buildings. "
    "No text, no letters, no captions, no frame, no modern elements."
)


def _load_dir(directory: Path) -> dict[str, dict]:
    return {
        path.stem: json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(directory.glob("*.json"))
    }


def _place(event: dict, provinces: dict[str, dict]) -> str | None:
    province_id = (event.get("scope") or {}).get("province")
    province = provinces.get(province_id or "")
    if not province:
        return None
    name = province.get("name")
    return name.get("display") if isinstance(name, dict) else name


def build_prompt(event: dict, provinces: dict[str, dict]) -> str:
    """Miniature prompt built from the event data only (no hard-coded scenes)."""
    lines = [f"Illustrated chronicle scene: « {event['title']} »."]
    year = str((event.get("historical_date") or {}).get("value", ""))[:4]
    place = _place(event, provinces)
    if year or place:
        lines.append(
            "Setting: "
            + ", ".join(part for part in (place, year) if part)
            + " (Hundred Years' War era)."
        )
    elif event.get("kind") == "random":
        lines.append("Setting: a typical scene of 14th-century France or England.")
    lines.append(f"What happens (French chronicle text): {event['text']}")
    lines.append(STYLE)
    return "\n".join(lines)


def plan(
    data_dir: Path = DATA_DIR, out_dir: Path = EVENTS_ART_DIR, limit: int | None = None
) -> list[PortraitJob]:
    """Miniatures still missing (idempotence), in event id order, up to ``limit``."""
    events = _load_dir(data_dir / "events")
    provinces = _load_dir(data_dir / "provinces")
    jobs = []
    for event_id, event in events.items():
        out_path = out_dir / f"{event_id}.jpg"
        if out_path.exists():
            continue
        jobs.append(PortraitJob(event_id, build_prompt(event, provinces), out_path))
        if limit is not None and len(jobs) >= limit:
            break
    return jobs


def to_miniature_jpg(
    image_bytes: bytes,
    width: int = ART_WIDTH,
    height: int = ART_HEIGHT,
    inset: float = 0.0,
) -> bytes:
    """Crop to the ``width``:``height`` ratio (biased upwards) and resize, as JPEG.

    ``inset`` (fraction of each side) is cut first, to drop a painted frame.
    """
    image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    if inset:
        margin_x = round(image.width * inset)
        margin_y = round(image.height * inset)
        image = image.crop(
            (margin_x, margin_y, image.width - margin_x, image.height - margin_y)
        )
    source_width, source_height = image.size
    target_ratio = width / height
    if source_width / source_height > target_ratio:
        crop_width, crop_height = round(source_height * target_ratio), source_height
    else:
        crop_width, crop_height = source_width, round(source_width / target_ratio)
    left = (source_width - crop_width) // 2
    top = round((source_height - crop_height) * TOP_BIAS)
    band = image.crop((left, top, left + crop_width, top + crop_height))
    buffer = io.BytesIO()
    band.resize((width, height), Image.Resampling.LANCZOS).save(
        buffer, "JPEG", quality=JPEG_QUALITY, optimize=True
    )
    return buffer.getvalue()
