"""OpenRouter client for image generation, guarded by the budget ledger.

Every paid call goes through :func:`generate_image`, which refuses to run when the
estimated price would push ``docs/budget.md`` above the cap, and records the
entry afterwards. Listing models (``GET /models``) is free.
"""

from __future__ import annotations

import base64
import os
import re
from datetime import date
from decimal import Decimal
from pathlib import Path
from typing import Any

import httpx
from pydantic import BaseModel

from cent_ans_tools import budget
from cent_ans_tools.budget import DEFAULT_BUDGET_PATH, BudgetExceeded, to_money

API_BASE = "https://openrouter.ai/api/v1"
SERVICE_NAME = "OpenRouter"

# Token budget assumed for one 1024x1024 image (Gemini bills ~1290 output tokens per
# image, OpenAI ~1056 at medium quality). Used only to compute the pre-call estimate.
IMAGE_OUTPUT_TOKENS = 1290
PROMPT_TOKENS_ESTIMATE = 300
COMPLETION_TOKENS_ESTIMATE = 100


class ImageModel(BaseModel):
    """An OpenRouter model able to output images, with its per-token pricing (USD)."""

    id: str
    name: str
    prompt: Decimal | None = None
    completion: Decimal | None = None
    image_output: Decimal | None = None

    def estimated_price_per_image(self) -> Decimal | None:
        """Estimated USD cost of one image (see ``IMAGE_OUTPUT_TOKENS``)."""
        if self.image_output is None:
            return None
        cost = self.image_output * IMAGE_OUTPUT_TOKENS
        cost += (self.prompt or 0) * PROMPT_TOKENS_ESTIMATE
        cost += (self.completion or 0) * COMPLETION_TOKENS_ESTIMATE
        return cost


def _price(value: Any) -> Decimal | None:
    if value is None:
        return None
    price = Decimal(str(value))
    return None if price < 0 else price


def _headers() -> dict[str, str]:
    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        raise RuntimeError("OPENROUTER_API_KEY manquant dans l'environnement")
    return {"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"}


def filter_image_models(models: list[dict[str, Any]]) -> list[ImageModel]:
    """Keep models whose ``architecture.output_modalities`` contains ``"image"``."""
    result: list[ImageModel] = []
    for model in models:
        modalities = (model.get("architecture") or {}).get("output_modalities") or []
        if "image" not in modalities:
            continue
        pricing = model.get("pricing") or {}
        result.append(
            ImageModel(
                id=model["id"],
                name=model.get("name", model["id"]),
                prompt=_price(pricing.get("prompt")),
                completion=_price(pricing.get("completion")),
                image_output=_price(pricing.get("image_output")),
            )
        )
    return result


def list_image_models(client: httpx.Client | None = None) -> list[ImageModel]:
    """Free call: fetch ``GET /models`` and return the image-capable ones with pricing."""
    own_client = client is None
    client = client or httpx.Client(timeout=30)
    try:
        response = client.get(f"{API_BASE}/models", headers=_headers())
        response.raise_for_status()
        return filter_image_models(response.json()["data"])
    finally:
        if own_client:
            client.close()


def estimate_price(model: str, client: httpx.Client | None = None) -> Decimal:
    """Estimated USD price for one image with ``model`` (rounded up to the cent)."""
    for candidate in list_image_models(client):
        if candidate.id == model:
            price = candidate.estimated_price_per_image()
            if price is not None:
                return price.quantize(Decimal("0.01"), rounding="ROUND_UP")
    raise ValueError(f"Modèle d'image inconnu ou sans tarif : {model}")


def _extract_image(payload: dict[str, Any]) -> bytes:
    message = payload["choices"][0]["message"]
    for image in message.get("images") or []:
        url = image.get("image_url", {}).get("url", "")
        match = re.match(r"data:image/\w+;base64,(.+)", url, re.DOTALL)
        if match:
            return base64.b64decode(match.group(1))
    raise RuntimeError("Aucune image base64 dans la réponse OpenRouter")


def request_image(
    model: str,
    prompt: str,
    client: httpx.Client | None = None,
    max_tokens: int | None = None,
) -> tuple[bytes, Decimal | None]:
    """Paid call without budget bookkeeping: return (image bytes, ``usage.cost`` or None).

    Callers must check and record the budget themselves (see :func:`generate_image`
    or ``cent_ans_tools.portraits``, which records one row per batch).
    """
    body = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "modalities": ["image", "text"],
        "usage": {"include": True},
    }
    if max_tokens is not None:
        # Caps the credit OpenRouter reserves up front (402 on low key limits).
        body["max_tokens"] = max_tokens
    own_client = client is None
    client = client or httpx.Client(timeout=180)
    try:
        response = client.post(
            f"{API_BASE}/chat/completions", headers=_headers(), json=body
        )
        response.raise_for_status()
        payload = response.json()
    finally:
        if own_client:
            client.close()
    cost = (payload.get("usage") or {}).get("cost")
    return _extract_image(payload), (Decimal(str(cost)) if cost is not None else None)


def generate_image(
    model: str,
    prompt: str,
    out_path: Path | str,
    *,
    subject: str | None = None,
    budget_path: Path | str = DEFAULT_BUDGET_PATH,
    client: httpx.Client | None = None,
) -> Path:
    """Generate one image (paid), guarded by the budget, and save it to ``out_path``.

    Raises:
        BudgetExceeded: if the estimated price would exceed the cap.
    """
    estimated = estimate_price(model, client)
    if not budget.check(estimated, budget_path):
        raise BudgetExceeded(
            f"Estimation {estimated} $ + cumul {budget.total(budget_path)} $ dépasse le plafond"
        )

    image, actual_raw = request_image(model, prompt, client)
    actual = to_money(actual_raw) if actual_raw is not None else estimated
    budget.add_entry(
        date.today().isoformat(),
        SERVICE_NAME,
        subject or f"image {model}",
        estimated,
        actual,
        path=budget_path,
    )

    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(image)
    return out_path
