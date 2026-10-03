"""RC5: water detail materials (config, budget cap, dry run) and data/fx/water_detail.json."""

import json
from decimal import Decimal
from pathlib import Path

import numpy as np
import pytest
import yaml
from jsonschema import Draft202012Validator
from PIL import Image
from typer.testing import CliRunner

from cent_ans_tools import budget, cli, material_gen, openrouter

ROOT = Path(__file__).resolve().parents[2]
WATER_YAML = ROOT / "data" / "art" / "water_materials.yaml"
IDS = ["sea", "ocean", "river_large", "river_small"]
RC_SECTION = "Fleuves et rivières RC"


def _validate(schema_name: str, document: object) -> None:
    schema = json.loads(
        (ROOT / "data" / "schemas" / schema_name).read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(document))
    assert not errors, [error.message for error in errors]


def test_water_materials_yaml_matches_schema():
    """water_materials.yaml validates and lists the four expected materials."""
    document = material_gen.load_materials(WATER_YAML)
    _validate("water_materials.schema.json", document)
    assert [entry["id"] for entry in document["materials"]] == IDS
    assert document["model"] == "google/gemini-3.1-flash-image"
    assert (document["size"], document["tile_size"]) == (1024, 512)
    assert document["budget"]["cap"] == 5.0


def test_water_detail_json_matches_schema():
    """data/fx/water_detail.json validates and covers every water material."""
    document = json.loads(
        (ROOT / "data" / "fx" / "water_detail.json").read_text(encoding="utf-8")
    )
    _validate("fx_water_detail.schema.json", document)
    assert sorted(document["materials"]) == sorted(IDS)


def test_repo_ledger_has_rc_section():
    """docs/budget.md keeps the RC section with its 5 $ envelope.

    It was the last table while RC5 ran; later chantiers append their own sections after it,
    and a new RC5 run must first reopen its section at the end (the paid-call guard checks it).
    """
    ledger = budget.BudgetLedger()
    assert "5 $" in (ledger.find_session(RC_SECTION).title or "")


def _ledger(tmp_path: Path, spent: str = "0,00") -> Path:
    path = tmp_path / "budget.md"
    path.write_text(
        f"# Budget\n\n## {RC_SECTION} (30/09) — plafond propre de 5 $\n\n"
        "| Date | Service | Objet | Coût estimé | Coût réel | Cumul RC |\n"
        "|---|---|---|---|---|---|\n"
        f"| 2026-09-30 | OpenRouter | RC5 : x | 0,00 $ | {spent} $ | {spent} $ |\n",
        encoding="utf-8",
    )
    return path


def _config(tmp_path: Path) -> Path:
    path = tmp_path / "water.yaml"
    path.write_text(
        yaml.safe_dump(
            {
                "model": "test/model",
                "size": 256,
                "tile_size": 64,
                "budget": {
                    "section": RC_SECTION,
                    "lot": "RC5",
                    "cap": 5.0,
                    "estimate_per_image": 0.078,
                },
                "materials": [
                    {"id": "sea", "prompt": "x" * 30, "blend_width": 16},
                    {"id": "river_large", "prompt": "y" * 30, "blend_width": 16},
                ],
            }
        ),
        encoding="utf-8",
    )
    return path


def _png_bytes(tmp_path: Path) -> bytes:
    path = tmp_path / "fake.png"
    ramp = np.tile(np.linspace(0, 255, 256), (256, 1)).astype(np.uint8)
    Image.fromarray(np.stack([ramp] * 3, axis=-1)).save(path)
    return path.read_bytes()


def test_plan_counts_only_missing_raw_images(tmp_path):
    """The dry run prices each missing image and skips those with a raw file."""
    out = tmp_path / "out"
    out.mkdir()
    (out / "sea_raw.png").write_bytes(b"x")
    report = material_gen.plan(None, out, materials_path=_config(tmp_path))
    assert [item["reuse"] for item in report["items"]] == [True, False]
    assert report["total"] == Decimal("0.078")
    assert report["cap"] == Decimal("5.0")


def test_cli_dry_run_makes_no_call(tmp_path, monkeypatch):
    """--dry-run prints prompts and cost without touching the API or the ledger."""

    def boom(*args, **kwargs):
        raise AssertionError("appel payant en dry-run")

    monkeypatch.setattr(openrouter, "request_image", boom)
    monkeypatch.setattr(openrouter, "estimate_price", boom)
    result = CliRunner().invoke(
        cli.app,
        [
            "assets", "materials", "--config", str(_config(tmp_path)),
            "--out", str(tmp_path / "out"), "--dry-run",
        ],
    )  # fmt: skip
    assert result.exit_code == 0, result.output
    assert "coût estimé 0.156 $" in result.output
    assert "xxxxx" in result.output


def test_cli_dry_run_over_envelope_fails(tmp_path):
    """An --envelope below the estimate makes the dry run fail."""
    result = CliRunner().invoke(
        cli.app,
        [
            "assets", "materials", "--config", str(_config(tmp_path)),
            "--out", str(tmp_path / "out"), "--dry-run", "--envelope", "0.1",
        ],
    )  # fmt: skip
    assert result.exit_code == 1


def test_generate_records_in_rc_section_with_mocked_client(tmp_path, monkeypatch):
    """A mocked OpenRouter call lands in the RC ledger section and yields the tile."""
    png = _png_bytes(tmp_path)
    monkeypatch.setattr(openrouter, "estimate_price", lambda *a, **k: Decimal("0.078"))
    monkeypatch.setattr(
        openrouter, "request_image", lambda *a, **k: (png, Decimal("0.078"))
    )
    ledger = _ledger(tmp_path)
    albedo = material_gen.generate(
        "sea",
        tmp_path / "out",
        materials_path=_config(tmp_path),
        budget_path=ledger,
    )
    assert albedo.name == "sea_albedo.png"
    assert (tmp_path / "out" / "sea_normal.png").exists()
    entries = budget.BudgetLedger(ledger).current_session.entries
    assert entries[-1].subject.startswith("RC5 : matière sea")
    assert budget.total(ledger) == Decimal("0.08")


def test_generate_refuses_above_section_cap(tmp_path, monkeypatch):
    """No call when the section total plus the estimate exceeds the 5 $ cap."""
    monkeypatch.setattr(
        openrouter,
        "request_image",
        lambda *a, **k: pytest.fail("appel payant au-delà du plafond"),
    )
    with pytest.raises(budget.BudgetExceeded):
        material_gen.generate(
            "sea",
            tmp_path / "out",
            materials_path=_config(tmp_path),
            budget_path=_ledger(tmp_path, spent="4,95"),
        )


def test_generate_envelope_lowers_cap(tmp_path, monkeypatch):
    """--envelope below the section cap stops the run earlier."""
    monkeypatch.setattr(
        openrouter,
        "request_image",
        lambda *a, **k: pytest.fail("appel payant au-delà de l'enveloppe"),
    )
    with pytest.raises(budget.BudgetExceeded):
        material_gen.generate(
            "sea",
            tmp_path / "out",
            materials_path=_config(tmp_path),
            budget_path=_ledger(tmp_path, spent="0,50"),
            envelope=Decimal("0.55"),
        )


def test_generate_refuses_wrong_last_section(tmp_path):
    """The configured section must be the last one of the ledger."""
    ledger = tmp_path / "budget.md"
    ledger.write_text(
        "# Budget\n\n## Autre chantier\n\n"
        "| Date | Service | Objet | Coût estimé | Coût réel | Cumul |\n"
        "|---|---|---|---|---|---|\n"
        "| 2026-09-28 | — | x | 0,00 $ | 0,00 $ | 0,00 $ |\n",
        encoding="utf-8",
    )
    with pytest.raises(RuntimeError, match=RC_SECTION):
        material_gen.generate(
            "sea",
            tmp_path / "out",
            materials_path=_config(tmp_path),
            budget_path=ledger,
        )
