"""Codex miniatures (places, battles, daily life, institutions...) through OpenRouter.

A Codex entry whose ``entity`` already has an image (event miniature, encyclopedia
illustration or portrait) reuses it in game; only the other entries get their own
miniature, saved as ``game/assets/illustrations/<cdx id>.jpg`` (640x360 JPEG, same crop
as the encyclopedia illustrations). Prompts are built from the entry data only (title,
category, period, summary without the ``[[links]]`` markup).

The paid batch reuses :func:`cent_ans_tools.portraits.generate`.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from cent_ans_tools import entry_art
from cent_ans_tools.portraits import DATA_DIR, REPO_DIR, PortraitJob

GAME_ASSETS_DIR = REPO_DIR / "game" / "assets"
# Generation order: most visual families first, so a short budget covers them.
# Characters (portrait style) and plants (herbarium) are left out.
CATEGORIES = (
    "lieu",
    "bataille",
    "guerre",
    "evenement",
    "vie_quotidienne",
    "societe",
    "economie",
    "institution",
    "religion",
    "savoir",
    "medecine",
    "dynastie",
    "cuisine",
    "recette",
    "ingredient",
)
# Where an entity image may already live, relative to game/assets.
ENTITY_IMAGES = ("events/{}.jpg", "illustrations/{}.jpg", "portraits/{}.png")

_LINK = re.compile(r"\[\[[^\]|]+\|([^\]]+)\]\]|\[\[([^\]]+)\]\]")

convert = entry_art.convert


def plain_text(text: str) -> str:
    """Codex text without the ``[[id|label]]`` link markup (label kept)."""
    return _LINK.sub(lambda match: match.group(1) or match.group(2), text)


def has_entity_image(entry: dict, assets_dir: Path = GAME_ASSETS_DIR) -> bool:
    """True when the entry's game entity already has an image the Codex can reuse."""
    entity = entry.get("entity", "")
    return bool(entity) and any(
        (assets_dir / pattern.format(entity)).exists() for pattern in ENTITY_IMAGES
    )


def build_prompt(entry: dict) -> str:
    """Miniature prompt for one Codex entry, from its data only."""
    era = entry.get("era") or {}
    start, end = str(era.get("from", "")), str(era.get("to", ""))
    period = start if start == end or not end else f"{start}-{end}"
    lines = [
        f"A scene illustrating « {entry['title']} » (French encyclopedia topic, "
        f"category {entry['category'].replace('_', ' ')}"
        + (f", period {period}" if period else "")
        + "), in the world of the Hundred Years' War.",
        f"Subject: {plain_text(entry.get('summary', ''))}",
        entry_art.STYLE,
    ]
    return "\n".join(lines)


def plan(
    data_dir: Path = DATA_DIR,
    assets_dir: Path = GAME_ASSETS_DIR,
    categories: tuple[str, ...] = CATEGORIES,
    limit: int | None = None,
) -> list[PortraitJob]:
    """Missing Codex miniatures, by category priority then id, up to ``limit``."""
    entries = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted((data_dir / "codex").glob("cdx_*.json"))
    ]
    jobs = []
    for category in categories:
        for entry in entries:
            if entry["category"] != category or has_entity_image(entry, assets_dir):
                continue
            out_path = assets_dir / "illustrations" / f"{entry['id']}.jpg"
            if out_path.exists():
                continue
            jobs.append(PortraitJob(entry["id"], build_prompt(entry), out_path))
            if limit is not None and len(jobs) >= limit:
                return jobs
    return jobs
