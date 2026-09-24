"""Encyclopedia miniatures (units, buildings, technologies, factions) through OpenRouter.

One wide illumination per entry of ``data/unit_types``, ``data/buildings``,
``data/technologies`` and ``data/factions``, shown at the top of its encyclopedia fiche. Prompts are built
from the entry data only (name, equipment, description, historical year); images are
saved as ``game/assets/illustrations/<entry id>.jpg`` (640x360 JPEG, same upward-biased
crop as the event miniatures).

The paid batch reuses :func:`cent_ans_tools.portraits.generate`.
"""

from __future__ import annotations

import json
from functools import partial
from pathlib import Path

from cent_ans_tools import event_art
from cent_ans_tools.portraits import DATA_DIR, REPO_DIR, PortraitJob

ILLUSTRATIONS_DIR = REPO_DIR / "game" / "assets" / "illustrations"
ART_WIDTH = 640
ART_HEIGHT = 360
CATEGORIES = ("unit_types", "buildings", "technologies", "factions")

STYLE = (
    "Style: 14th-century French Gothic manuscript miniature (enluminure). Wide "
    "landscape scene: every figure and important object fits inside the middle 55 % "
    "of the image height, with patterned sky above and ground below (the top and "
    "bottom will be cropped). Flat gilded or diapered azure background, egg tempera "
    "colours, fine black ink outlines, gold leaf highlights, period-accurate "
    "clothing, armour, tools and buildings. No text, no letters, no captions, no "
    "frame, no modern elements."
)

convert = partial(event_art.to_miniature_jpg, width=ART_WIDTH, height=ART_HEIGHT)


def _year(entry: dict) -> str:
    value = entry.get("historical_year")
    if isinstance(value, dict):
        value = value.get("value")
    return str(value or "")[:4]


def build_prompt(category: str, entry: dict) -> str:
    """Miniature prompt for one entry of ``category``, from its data only."""
    name = entry["name"]["display"]
    local = entry["name"].get("local", "")
    label = f"« {name} »" + (f" ({local})" if local and local != name else "")
    description = entry.get("description", "")
    if category == "unit_types":
        lines = [
            f"A company of soldiers of the type {label} in the field, several men "
            "shown together, full length.",
            f"Equipment: {entry.get('equipment', '')}",
            f"Role: {description}",
        ]
    elif category == "buildings":
        lines = [
            f"The building {label} in a 14th-century French town or countryside, "
            "with people at work around it.",
            f"What it is: {description}",
        ]
    elif category == "factions":
        blazon = (entry.get("heraldry") or {}).get("blazon", "")
        city = entry.get("capital_city", "")
        lines = [
            f"The realm {label} in 1337: "
            + (f"a view of its capital {city} with " if city else "")
            + "its people, soldiers and banners.",
            f"Arms on the banners: {blazon}" if blazon else "",
            f"Context: {description}",
        ]
    else:
        year = _year(entry)
        lines = [
            f"A scene illustrating the medieval innovation {label}"
            + (f", around {year}" if year else "")
            + ", shown in use by craftsmen, scholars, physicians or soldiers.",
            f"What it is: {description}",
        ]
    lines.append(STYLE)
    return "\n".join(line for line in lines if line)


def plan(
    data_dir: Path = DATA_DIR,
    out_dir: Path = ILLUSTRATIONS_DIR,
    categories: tuple[str, ...] = CATEGORIES,
    limit: int | None = None,
) -> list[PortraitJob]:
    """Miniatures still missing (idempotence), by category then id, up to ``limit``."""
    jobs = []
    for category in categories:
        for path in sorted((data_dir / category).glob("*.json")):
            out_path = out_dir / f"{path.stem}.jpg"
            if out_path.exists():
                continue
            entry = json.loads(path.read_text(encoding="utf-8"))
            jobs.append(PortraitJob(path.stem, build_prompt(category, entry), out_path))
            if limit is not None and len(jobs) >= limit:
                return jobs
    return jobs
