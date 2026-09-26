"""DA5b: entity icons as painted miniatures (catalogue, plan, crop, frame)."""

import json
from decimal import Decimal
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator
from PIL import Image, ImageDraw

from cent_ans_tools import entity_icons
from cent_ans_tools.budget import BudgetLedger

ROOT = Path(__file__).resolve().parents[2]


def _catalog() -> dict:
    return entity_icons.load_catalog()


def test_catalog_matches_schema() -> None:
    """The catalogue matches its schema."""
    schema = json.loads(
        (ROOT / "data/schemas/entity_icons.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_catalog()))
    assert not errors, [error.message for error in errors]


def test_ids_targets_and_sources() -> None:
    """Unique ids and targets; every derived source and the reference exist."""
    catalog = _catalog()
    items = entity_icons.entries(catalog)
    ids = [entry.id for entry in items]
    assert len(ids) == len(set(ids))
    targets = [t for entry in items for t in entry.targets]
    assert len(targets) == len(set(targets))
    for entry in items:
        if entry.source:
            assert (ROOT / entry.source).is_file(), entry.source
    assert (ROOT / catalog["generation"]["reference"]).is_file()


def test_every_game_entity_with_an_illustration_is_derived() -> None:
    """Reuse first: a unit, building or technology with an illustration is never generated."""
    catalog = _catalog()
    by_id = {entry.id: entry for entry in entity_icons.entries(catalog)}
    for folder in ("unit_types", "buildings", "technologies"):
        for path in (ROOT / "data" / folder).glob("*.json"):
            illustration = ROOT / "game/assets/illustrations" / f"{path.stem}.jpg"
            assert path.stem in by_id, path.stem
            if illustration.exists():
                assert not by_id[path.stem].generated, path.stem


def test_prompt_ends_with_the_style_block() -> None:
    """Prompts are built from the data: lead and subject, then the common style block."""
    catalog = _catalog()
    entry = next(e for e in entity_icons.entries(catalog) if e.generated)
    prompt = entity_icons.build_prompt(catalog, entry)
    assert entry.subject in prompt
    assert prompt.endswith(catalog["generation"]["prompt"])
    assert "no text" in prompt


def test_plan_skips_derived_and_existing_raws(tmp_path, monkeypatch) -> None:
    """Only generated entries without a raw file are planned; ``only`` and ``limit`` filter."""
    monkeypatch.setattr(entity_icons, "RAW_DIR", tmp_path)
    catalog = _catalog()
    generated = [e for e in entity_icons.entries(catalog) if e.generated]
    jobs = entity_icons.plan(catalog)
    assert len(jobs) == len(generated)
    generated[0].raw_path.write_bytes(b"x")
    assert len(entity_icons.plan(catalog)) == len(generated) - 1
    assert [
        j.character_id for j in entity_icons.plan(catalog, only=[generated[1].id])
    ] == [generated[1].id]
    assert len(entity_icons.plan(catalog, limit=2)) == 2


def test_auto_crop_finds_the_subject_and_is_deterministic() -> None:
    """A detailed figure on a flat azure ground draws the square onto it."""
    image = Image.new("RGB", (640, 360), (40, 70, 150))
    draw = ImageDraw.Draw(image)
    for i in range(0, 120, 6):  # striped, colourful "figure" near the right edge
        draw.rectangle((470 + i // 2, 100 + i, 560, 106 + i), fill=(200, 60 + i, 30))
    crop = entity_icons.auto_crop(image, 0.6)
    assert crop.x > 0.6
    assert crop == entity_icons.auto_crop(image, 0.6)
    square = entity_icons.cut_square(image, crop)
    assert square.size == (216, 216)


def test_compose_frames_with_gold_and_transparent_corners() -> None:
    """The framed miniature has transparent corners, a gold fillet and the painting inside."""
    catalog = _catalog()
    picture = Image.new("RGB", (300, 300), (20, 160, 40))
    icon = entity_icons.compose(picture, catalog)
    assert icon.size == (catalog["size"], catalog["size"])
    pixels = np.asarray(icon)
    assert pixels[0, 0, 3] == 0  # rounded corner
    centre = pixels[64, 64]
    assert centre[1] > 120 and centre[3] == 255  # painting
    gold = pixels[64, 3]  # inside the gold fillet
    assert gold[0] > gold[2] + 40


def test_lot_spent_counts_only_da5b_rows(tmp_path) -> None:
    """The lot envelope ignores the DA5 rows of the same section."""
    ledger_path = tmp_path / "budget.md"
    ledger_path.write_text(
        "# Budget\n\nPlafond : 50 $\n\n## Session\n\n"
        "| Date | Service | Objet | Coût estimé | Coût réel | Cumul |\n"
        "|---|---|---|---|---|---|\n"
        "| 2026-09-26 | OpenRouter | DA5 : icônes | 1,00 $ | 1,00 $ | 1,00 $ |\n"
        "| 2026-09-26 | OpenRouter | DA5b : miniatures | 0,50 $ | 0,40 $ | 1,40 $ |\n",
        encoding="utf-8",
    )
    assert entity_icons.lot_spent(BudgetLedger(ledger_path)) == Decimal("0.40")
