"""Tests for portrait prompts, planning, budget envelope and dry-run (HTTP mocked)."""

import base64
import io
import json
from decimal import Decimal
from pathlib import Path

import httpx
import pytest
from PIL import Image
from typer.testing import CliRunner

from cent_ans_tools import cli, portraits
from cent_ans_tools.budget import BudgetExceeded, BudgetLedger

MODELS_PAYLOAD = {
    "data": [
        {
            "id": "vendor/imager",
            "name": "Imager",
            "architecture": {"output_modalities": ["image"]},
            "pricing": {"prompt": "0", "completion": "0", "image_output": "0.00003"},
        }
    ]
}


def _png(width: int = 64, height: int = 96) -> bytes:
    buffer = io.BytesIO()
    Image.new("RGB", (width, height), (200, 30, 30)).save(buffer, "PNG")
    return buffer.getvalue()


def _handler(calls: list[str], cost: float = 0.04):
    data_url = "data:image/png;base64," + base64.b64encode(_png()).decode()

    def handler(request: httpx.Request) -> httpx.Response:
        calls.append(request.url.path)
        if request.url.path.endswith("/models"):
            return httpx.Response(200, json=MODELS_PAYLOAD)
        body = json.loads(request.read())
        assert body["max_tokens"] == portraits.MAX_TOKENS
        return httpx.Response(
            200,
            json={
                "choices": [
                    {"message": {"images": [{"image_url": {"url": data_url}}]}}
                ],
                "usage": {"cost": cost},
            },
        )

    return handler


def test_prompt_uses_character_data() -> None:
    """The prompt carries name, age in 1337, rank, house, blazon, traits and style."""
    character = json.loads(
        (portraits.DATA_DIR / "characters" / "chr_edward_iii.json").read_text()
    )
    factions = portraits._load_dir(portraits.DATA_DIR / "factions")
    traits = portraits._load_dir(portraits.DATA_DIR / "traits")
    prompt = portraits.build_prompt(character, factions, traits)
    assert "Édouard III" in prompt
    assert "aged 25 in 1337" in prompt
    assert "Roi d'Angleterre" in prompt
    assert "Plantagenêt" in prompt
    assert "léopards" in prompt
    assert traits["trait_ambitious"]["name"]["display"] in prompt
    assert "enluminure gothique" in prompt


def test_eligibility_rules() -> None:
    """Dead before 1337, born after 1400 or fictional characters are skipped."""
    base = {"historical": True, "birth": {"value": "1300"}}
    assert portraits.is_eligible(base)
    assert not portraits.is_eligible({**base, "death": {"value": "1330"}})
    assert not portraits.is_eligible({**base, "birth": {"value": "1401"}})
    assert not portraits.is_eligible({**base, "historical": False})


def test_plan_is_idempotent(tmp_path: Path) -> None:
    """Existing portraits are not planned again; limit is honoured."""
    jobs = portraits.plan(out_dir=tmp_path, limit=3)
    assert len(jobs) == 3
    jobs[0].out_path.write_bytes(b"done")
    again = portraits.plan(out_dir=tmp_path, limit=3)
    assert jobs[0].character_id not in {job.character_id for job in again}


def test_generate_records_one_batch_row(
    budget_file: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Two images -> two 256x256 PNGs and a single ledger row with the real cost."""
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")
    calls: list[str] = []
    jobs = portraits.plan(out_dir=tmp_path, limit=2)
    rows_before = len(BudgetLedger(budget_file).entries)
    with httpx.Client(transport=httpx.MockTransport(_handler(calls))) as client:
        result = portraits.generate(
            jobs, "vendor/imager", budget_path=budget_file, client=client
        )
    assert len(result.written) == 2
    assert Image.open(result.written[0]).size == (256, 256)
    ledger = BudgetLedger(budget_file)
    assert ledger.entries[-1].actual == Decimal("0.08")
    assert len([e for e in ledger.entries if not e.is_placeholder]) <= rows_before + 1
    assert calls.count("/api/v1/chat/completions") == 2


def test_generate_refuses_beyond_envelope(
    budget_file: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """The envelope stops the batch before the paid call that would overflow it."""
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")
    calls: list[str] = []
    jobs = portraits.plan(out_dir=tmp_path, limit=5)
    with (
        httpx.Client(transport=httpx.MockTransport(_handler(calls))) as client,
        pytest.raises(BudgetExceeded),
    ):
        portraits.generate(
            jobs,
            "vendor/imager",
            envelope=Decimal("0.09"),
            budget_path=budget_file,
            client=client,
        )
    assert calls.count("/api/v1/chat/completions") == 2
    assert BudgetLedger(budget_file).entries[-1].actual == Decimal("0.08")


def test_generate_refuses_beyond_global_cap(
    budget_file: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """With the ledger full, no paid call is made."""
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")
    BudgetLedger(budget_file).add_entry("2026-09-23", "OpenRouter", "fill", 50, 50)
    calls: list[str] = []
    jobs = portraits.plan(out_dir=tmp_path, limit=1)
    with (
        httpx.Client(transport=httpx.MockTransport(_handler(calls))) as client,
        pytest.raises(BudgetExceeded),
    ):
        portraits.generate(
            jobs, "vendor/imager", budget_path=budget_file, client=client
        )
    assert "/api/v1/chat/completions" not in calls


def test_dry_run_makes_no_network_call(monkeypatch: pytest.MonkeyPatch) -> None:
    """--dry-run prints prompts and an offline estimate without touching httpx."""

    def forbidden(*_args, **_kwargs):
        raise AssertionError("network call during dry-run")

    monkeypatch.setattr(httpx.Client, "send", forbidden)
    result = CliRunner().invoke(
        cli.app, ["assets", "portraits", "--dry-run", "--limit", "2"]
    )
    assert result.exit_code == 0, result.output
    assert "aucun appel réseau" in result.output
    assert "Style" in result.output
