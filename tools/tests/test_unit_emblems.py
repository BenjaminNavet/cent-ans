"""Unit emblems (lot OMR R5): local sources of the miniatures of units without illustration."""

import json
from pathlib import Path

import numpy as np

from cent_ans_tools import unit_emblems

ROOT = Path(__file__).resolve().parents[2]


def test_emblem_is_gold_on_azure() -> None:
    """Square RGB picture: azure ground in the corner, gold drawing somewhere in the middle."""
    picture = unit_emblems.emblem("unit_teutonic_knights", size=128)
    assert picture.size == (128, 128) and picture.mode == "RGB"
    pixels = np.asarray(picture, dtype=np.int32)
    corner = pixels[4, 4]
    assert corner[2] > corner[0], corner  # blue ground
    middle = pixels[32:96, 32:96].reshape(-1, 3)
    gold = (middle[:, 0] > 150) & (middle[:, 0] > middle[:, 2] + 40)
    assert gold.mean() > 0.05


def test_every_emblem_unit_is_catalogued() -> None:
    """Each emblem has a game-icons icon and is the source of its entity miniature."""
    icons = json.loads((ROOT / "game/assets/icons/icons.json").read_text("utf-8"))[
        "icons"
    ]
    catalog = json.loads((ROOT / "data/ui/entity_icons.json").read_text("utf-8"))
    sources = {item["id"]: item.get("source") for item in catalog["icons"]}
    for unit_id in unit_emblems.UNITS:
        assert unit_id in icons, unit_id
        assert sources.get(unit_id) == f"tools/emblem_src/{unit_id}.png", unit_id
        assert (ROOT / "tools/emblem_src" / f"{unit_id}.png").exists(), unit_id
