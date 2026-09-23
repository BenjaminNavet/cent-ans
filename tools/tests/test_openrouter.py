"""Tests for the OpenRouter client (model filter and budget guard), HTTP mocked."""

import base64
import json
from decimal import Decimal
from pathlib import Path

import httpx
import pytest

from cent_ans_tools import openrouter
from cent_ans_tools.budget import BudgetExceeded, BudgetLedger

MODELS_PAYLOAD = {
    "data": [
        {
            "id": "vendor/text-only",
            "name": "Text only",
            "architecture": {"output_modalities": ["text"]},
            "pricing": {"prompt": "0.000001", "completion": "0.000002"},
        },
        {
            "id": "vendor/imager",
            "name": "Imager",
            "architecture": {"output_modalities": ["text", "image"]},
            "pricing": {
                "prompt": "0.0000003",
                "completion": "0.0000025",
                "image_output": "0.00003",
            },
        },
        {
            "id": "router/auto",
            "name": "Auto",
            "architecture": {"output_modalities": ["image"]},
            "pricing": {"prompt": "-1", "completion": "-1"},
        },
        {"id": "vendor/no-arch", "name": "No arch", "pricing": {}},
    ]
}


def _client(handler) -> httpx.Client:
    return httpx.Client(transport=httpx.MockTransport(handler))


def test_filter_image_models_keeps_only_image_output() -> None:
    """Only models with "image" in output_modalities survive; -1 prices become None."""
    models = openrouter.filter_image_models(MODELS_PAYLOAD["data"])
    assert [model.id for model in models] == ["vendor/imager", "router/auto"]
    imager, auto = models
    assert imager.image_output == Decimal("0.00003")
    assert auto.prompt is None and auto.image_output is None
    assert auto.estimated_price_per_image() is None


def test_estimated_price_per_image_uses_token_assumptions() -> None:
    """Price = image tokens * image_output + prompt/completion estimates."""
    imager = openrouter.filter_image_models(MODELS_PAYLOAD["data"])[0]
    expected = (
        Decimal("0.00003") * openrouter.IMAGE_OUTPUT_TOKENS
        + Decimal("0.0000003") * openrouter.PROMPT_TOKENS_ESTIMATE
        + Decimal("0.0000025") * openrouter.COMPLETION_TOKENS_ESTIMATE
    )
    assert imager.estimated_price_per_image() == expected


def test_list_image_models_calls_models_endpoint(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """GET /models is called with the bearer token and filtered."""
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")
    seen: dict[str, str] = {}

    def handler(request: httpx.Request) -> httpx.Response:
        seen["url"] = str(request.url)
        seen["auth"] = request.headers["Authorization"]
        return httpx.Response(200, json=MODELS_PAYLOAD)

    with _client(handler) as client:
        models = openrouter.list_image_models(client)
    assert seen == {
        "url": "https://openrouter.ai/api/v1/models",
        "auth": "Bearer test-key",
    }
    assert [model.id for model in models] == ["vendor/imager", "router/auto"]


def test_list_image_models_requires_api_key(monkeypatch: pytest.MonkeyPatch) -> None:
    """Missing OPENROUTER_API_KEY raises before any HTTP call."""
    monkeypatch.delenv("OPENROUTER_API_KEY", raising=False)
    with (
        pytest.raises(RuntimeError, match="OPENROUTER_API_KEY"),
        _client(lambda _: httpx.Response(500)) as client,
    ):
        openrouter.list_image_models(client)


def test_generate_image_refuses_when_budget_exceeded(
    monkeypatch: pytest.MonkeyPatch, budget_file: Path, tmp_path: Path
) -> None:
    """The paid endpoint is never hit when the estimate does not fit under the cap."""
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")
    BudgetLedger(budget_file).add_entry("2026-09-23", "OpenRouter", "fill", 50, 50)
    calls: list[str] = []

    def handler(request: httpx.Request) -> httpx.Response:
        calls.append(request.url.path)
        return httpx.Response(200, json=MODELS_PAYLOAD)

    with _client(handler) as client, pytest.raises(BudgetExceeded):
        openrouter.generate_image(
            "vendor/imager",
            "a knight",
            tmp_path / "out.png",
            budget_path=budget_file,
            client=client,
        )
    assert calls == ["/api/v1/models"]


def test_generate_image_saves_file_and_records_budget(
    monkeypatch: pytest.MonkeyPatch, budget_file: Path, tmp_path: Path
) -> None:
    """A successful (mocked) call writes the image and appends a budget row."""
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")
    png = b"\x89PNG\r\n\x1a\nfake"
    data_url = "data:image/png;base64," + base64.b64encode(png).decode()

    def handler(request: httpx.Request) -> httpx.Response:
        if request.url.path.endswith("/models"):
            return httpx.Response(200, json=MODELS_PAYLOAD)
        body = json.loads(request.read())
        assert body["modalities"] == ["image", "text"]
        assert body["model"] == "vendor/imager"
        return httpx.Response(
            200,
            json={
                "choices": [
                    {
                        "message": {
                            "content": "",
                            "images": [{"image_url": {"url": data_url}}],
                        }
                    }
                ],
                "usage": {"cost": 0.0371},
            },
        )

    out = tmp_path / "img" / "knight.png"
    with _client(handler) as client:
        result = openrouter.generate_image(
            "vendor/imager",
            "a knight",
            out,
            subject="portrait test",
            budget_path=budget_file,
            client=client,
        )
    assert result == out and out.read_bytes() == png
    ledger = BudgetLedger(budget_file)
    assert ledger.entries[-1].subject == "portrait test"
    assert ledger.entries[-1].estimated == Decimal("0.04")
    assert ledger.total() == Decimal("0.04")
