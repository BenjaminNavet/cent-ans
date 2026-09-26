"""DA5: ink action icons and medallion buttons (catalogue, plan, image processing)."""

import json
from decimal import Decimal
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator
from PIL import Image, ImageDraw

from cent_ans_tools import ink_icons
from cent_ans_tools.budget import BudgetLedger

ROOT = Path(__file__).resolve().parents[2]


def _catalog() -> dict:
    return ink_icons.load_catalog()


def test_catalog_matches_schema() -> None:
    """The catalogue matches its schema."""
    schema = json.loads(
        (ROOT / "data/schemas/icons_ink.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_catalog()))
    assert not errors, [error.message for error in errors]


def test_ids_and_targets_are_unique() -> None:
    """Ids are unique per kind and a game id is claimed by one image of a kind only."""
    catalog = _catalog()
    for key in ("icons", "medallions"):
        ids = [item["id"] for item in catalog[key]]
        assert len(ids) == len(set(ids)), key
        targets = [t for item in catalog[key] for t in item["targets"]]
        assert len(targets) == len(set(targets)), key


def test_references_and_sources_exist() -> None:
    """Style references and reused validated sources are in the repository."""
    catalog = _catalog()
    for style in ("icon_style", "medallion_style"):
        assert (ROOT / catalog[style]["reference"]).is_file()
    for item in catalog["medallions"]:
        if "source" in item:
            assert (ROOT / item["source"]).is_file()


def test_prompt_ends_with_subject_and_forbids_text() -> None:
    """Prompts are built from the data: shared style block, then the subject."""
    catalog = _catalog()
    entry = ink_icons.entries(catalog, "icon")[0]
    prompt = ink_icons.build_prompt(catalog, entry)
    assert prompt.endswith(entry.subject + ".")
    assert "No text" in prompt


def test_plan_skips_sources_existing_raws_and_filters(tmp_path, monkeypatch) -> None:
    """Idempotent: an entry with a raw file or a validated source is never planned."""
    monkeypatch.setattr(ink_icons, "RAW_DIR", tmp_path)
    catalog = _catalog()
    first = ink_icons.entries(catalog, "icon")[0]
    first.raw_path.parent.mkdir(parents=True)
    first.raw_path.write_bytes(b"x")
    jobs = ink_icons.plan(catalog)
    keys = {job.character_id for job in jobs}
    assert f"icon:{first.id}" not in keys
    assert "medallion:end_turn" not in keys  # validated bell, reused
    assert len(jobs) == len(catalog["icons"]) - 1 + len(catalog["medallions"]) - 1
    only = ink_icons.plan(catalog, only=["medallion:diplomacy", "halt"])
    assert {job.character_id for job in only} == {"medallion:diplomacy", "icon:halt"}
    assert len(ink_icons.plan(catalog, kind="icon", limit=3)) == 3


def _ink_drawing(width: int) -> Image.Image:
    image = Image.new("RGB", (512, 512), (236, 224, 196))
    draw = ImageDraw.Draw(image)
    draw.ellipse((150, 150, 360, 360), outline=(60, 40, 20), width=width)
    draw.line((256, 100, 256, 420), fill=(60, 40, 20), width=width)
    return image


def test_icon_alpha_crop_and_stroke_normalisation() -> None:
    """Parchment becomes transparent, the ink colour is INK, thin and thick strokes converge."""
    widths = []
    for stroke in (6, 30):
        icon, width = ink_icons.process_icon(_ink_drawing(stroke), 128)
        assert icon.size == (128, 128)
        pixels = np.asarray(icon)
        assert pixels[0, 0, 3] == 0
        assert pixels[..., 3].max() > 200
        inked = pixels[pixels[..., 3] > 0][:, :3]
        assert (inked == ink_icons.INK_RGB).all()
        widths.append(width)
    target = ink_icons.TARGET_STROKE_PX
    for width in widths:
        assert target * 0.6 < width < target * 1.6, widths


def test_medallion_background_is_transparent() -> None:
    """The parchment around the disc is cut out; the disc stays opaque."""
    image = Image.new("RGB", (768, 768), (236, 224, 196))
    draw = ImageDraw.Draw(image)
    draw.ellipse(
        (100, 100, 668, 668), fill=(200, 160, 50), outline=(50, 30, 10), width=8
    )
    medallion = ink_icons.process_medallion(image, 256)
    pixels = np.asarray(medallion)
    assert medallion.size == (256, 256)
    assert pixels[2, 2, 3] == 0
    assert pixels[128, 128, 3] == 255


def test_lot_spent_counts_da5_rows_only(tmp_path) -> None:
    """The lot envelope is read from the DA5 rows of the ledger."""
    path = tmp_path / "budget.md"
    path.write_text(
        "# Budget\n\n| Date | Service | Objet | Coût estimé | Coût réel | Cumul |\n|---|---|---|---|---|---|\n"
        "| 2026-09-25 | OpenRouter | DA2 : portraits | 0,28 $ | 0,27 $ | 0,27 $ |\n"
        "| 2026-09-26 | OpenRouter | DA5 : icônes | 0,20 $ | 0,18 $ | 0,45 $ |\n",
        encoding="utf-8",
    )
    assert ink_icons.lot_spent(BudgetLedger(path)) == Decimal("0.18")
