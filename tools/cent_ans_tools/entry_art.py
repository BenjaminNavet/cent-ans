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
import zlib
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
    "frame, no border, no modern elements."
)

# Faction miniatures: landscape and buildings of the capital province, one scene per
# faction (stable choice by id) so that 150 realms do not all show the same procession.
TERRAIN_SCENERY = {
    "hills": "rolling hills",
    "plains": "a wide open plain",
    "mountains": "high snow-capped mountains with steep valleys and torrents",
    "forest": "a dense dark forest",
    "steppe": "a vast treeless grassy steppe under a huge sky",
    "heath": "open windswept heathland",
    "marsh": "marshes, reed beds and channels",
    "desert": "an arid desert with palm groves and sand",
    "bocage": "small fields enclosed by hedgerows",
}
CLIMATE_SCENERY = {
    "mediterranean": "olive trees, cypresses and vines in warm light",
    "arid": "dry ochre earth and sparse vegetation",
    "oceanic": "green wet meadows",
    "steppe": "tall dry grass",
    "mountain": "pine woods and alpine pastures",
}
RELIGION_BUILDINGS = {
    "rel_orthodox": "Byzantine-style domed churches",
    "rel_armenian": "Armenian stone churches with conical domes",
    "rel_islam": "mosques with minarets",
    "rel_judaism": "a synagogue among the houses",
    "rel_pagan": "wooden sanctuaries and sacred groves",
}
DEFAULT_RELIGION_BUILDINGS = "Gothic church spires"
FACTION_SCENES = (
    "a procession with banners entering the capital",
    "market day in the capital, merchants and townsfolk around the stalls",
    "the realm's army encamped with tents and horses before the capital",
    "the ruler holding court under an open arcade, the capital beyond",
    "peasants harvesting the fields around the capital",
    "riders and hunters crossing the countryside towards the capital",
    "masons repairing the walls of the capital while guards watch",
)
HARBOUR_SCENE = "the harbour of the capital, ships unloading on the quay"

# Z-Image paints a thin gold frame despite "no frame" (ADR 0190): cut it off.
FRAME_INSET = 0.02
convert = partial(
    event_art.to_miniature_jpg,
    width=ART_WIDTH,
    height=ART_HEIGHT,
    inset=FRAME_INSET,
)


def _year(entry: dict) -> str:
    value = entry.get("historical_year")
    if isinstance(value, dict):
        value = value.get("value")
    return str(value or "")[:4]


def faction_setting(faction_id: str, province: dict | None) -> str:
    """Scene and scenery of a faction miniature, from its capital province data."""
    province = province or {}
    scenes = FACTION_SCENES + ((HARBOUR_SCENE,) if province.get("coastal") else ())
    scene = scenes[zlib.crc32(faction_id.encode("utf-8")) % len(scenes)]
    scenery = [
        TERRAIN_SCENERY.get(province.get("terrain", ""), ""),
        CLIMATE_SCENERY.get(province.get("climate", ""), ""),
        RELIGION_BUILDINGS.get(
            province.get("religion", ""), DEFAULT_RELIGION_BUILDINGS
        ),
    ]
    rivers = province.get("rivers") or []
    if rivers:
        scenery.append(f"the river {rivers[0]}")
    if province.get("coastal"):
        scenery.append("the sea shore")
    lines = [f"Scene: {scene}."]
    lines.append(
        "Landscape and buildings: " + ", ".join(part for part in scenery if part) + "."
    )
    lines.append(
        "Clothing, arms and architecture of this region in 1337, not of France "
        "unless the realm is French."
    )
    return "\n".join(lines)


def build_prompt(category: str, entry: dict, province: dict | None = None) -> str:
    """Miniature prompt for one entry of ``category``, from its data only.

    ``province`` is the capital province of a faction (scenery and scene choice).
    """
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
            f"The realm {label} in 1337"
            + (f", around its capital {city}" if city else "")
            + ", with its people, soldiers and banners.",
            faction_setting(entry.get("id", name), province),
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
            province_path = data_dir / "provinces" / f"{entry.get('capital', '')}.json"
            province = (
                json.loads(province_path.read_text(encoding="utf-8"))
                if category == "factions" and province_path.is_file()
                else None
            )
            jobs.append(
                PortraitJob(
                    path.stem, build_prompt(category, entry, province), out_path
                )
            )
            if limit is not None and len(jobs) >= limit:
                return jobs
    return jobs
