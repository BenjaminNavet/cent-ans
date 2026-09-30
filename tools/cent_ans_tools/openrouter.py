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
# Output tokens per Gemini ``image_config.image_size`` tier (Google's published counts;
# the ledger keeps the billed ``usage.cost``, these only feed the pre-call estimate).
IMAGE_SIZE_TOKENS = {"512": 747, "1K": 1120, "2K": 1680, "4K": 2520}


class ImageModel(BaseModel):
    """An OpenRouter model able to output images, with its per-token pricing (USD)."""

    id: str
    name: str
    prompt: Decimal | None = None
    completion: Decimal | None = None
    image_output: Decimal | None = None

    def estimated_price_per_image(
        self, image_size: str | None = None
    ) -> Decimal | None:
        """Estimated USD cost of one image (``IMAGE_SIZE_TOKENS`` or ``IMAGE_OUTPUT_TOKENS``)."""
        if self.image_output is None:
            return None
        tokens = IMAGE_SIZE_TOKENS[image_size] if image_size else IMAGE_OUTPUT_TOKENS
        cost = self.image_output * tokens
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


def estimate_price(
    model: str, client: httpx.Client | None = None, image_size: str | None = None
) -> Decimal:
    """Estimated USD price for one image with ``model`` (rounded up to the cent)."""
    for candidate in list_image_models(client):
        if candidate.id == model:
            price = candidate.estimated_price_per_image(image_size)
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


class ImageExtractionError(RuntimeError):
    """Raised when a billed OpenRouter response has no image (e.g. a refusal).

    ``cost`` carries ``usage.cost`` read from that same response, if any, so
    callers can still record the spend even though no image was produced.
    """

    def __init__(self, message: str, cost: Decimal | None):
        """Wrap ``message`` and keep the already-billed ``cost``, if known."""
        super().__init__(message)
        self.cost = cost


def request_image(
    model: str,
    prompt: str,
    client: httpx.Client | None = None,
    max_tokens: int | None = None,
    images: list[bytes] | None = None,
    image_config: dict[str, str] | None = None,
    seed: int | None = None,
) -> tuple[bytes, Decimal | None]:
    """Paid call without budget bookkeeping: return (image bytes, ``usage.cost`` or None).

    Callers must check and record the budget themselves (see :func:`generate_image`
    or ``cent_ans_tools.portraits``, which records one row per batch). ``images``
    are optional reference pictures (PNG or JPEG bytes) sent with the prompt, e.g.
    the existing portrait of a character to age (lot DA2). ``image_config`` (Gemini:
    ``aspect_ratio``, ``image_size``) and ``seed`` are sent only when given.
    """
    content: str | list[dict[str, Any]] = prompt
    if images:
        content = [{"type": "text", "text": prompt}]
        for image in images:
            kind = "jpeg" if image[:2] == b"\xff\xd8" else "png"
            encoded = base64.b64encode(image).decode()
            content.append(
                {
                    "type": "image_url",
                    "image_url": {"url": f"data:image/{kind};base64,{encoded}"},
                }
            )
    body = {
        "model": model,
        "messages": [{"role": "user", "content": content}],
        "modalities": ["image", "text"],
        "usage": {"include": True},
    }
    if max_tokens is not None:
        # Caps the credit OpenRouter reserves up front (402 on low key limits).
        body["max_tokens"] = max_tokens
    if image_config:
        body["image_config"] = image_config
    if seed is not None:
        body["seed"] = seed
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
    cost_raw = (payload.get("usage") or {}).get("cost")
    cost = Decimal(str(cost_raw)) if cost_raw is not None else None
    try:
        image = _extract_image(payload)
    except RuntimeError as exc:
        raise ImageExtractionError(str(exc), cost) from exc
    return image, cost


def generate_image(
    model: str,
    prompt: str,
    out_path: Path | str,
    *,
    subject: str | None = None,
    budget_path: Path | str = DEFAULT_BUDGET_PATH,
    client: httpx.Client | None = None,
    images: list[bytes] | None = None,
    image_config: dict[str, str] | None = None,
    seed: int | None = None,
    cap: Decimal | None = None,
) -> Path:
    """Generate one image (paid), guarded by the budget, and save it to ``out_path``.

    ``images``, ``image_config`` and ``seed`` go to :func:`request_image`; ``cap``
    replaces the ledger's default cap for the current session (e.g. NB: 10 $).

    Raises:
        BudgetExceeded: if the estimated price would exceed the cap.
    """
    image_size = (image_config or {}).get("image_size")
    estimated = estimate_price(model, client, image_size)
    ledger = (
        budget.BudgetLedger(budget_path, cap)
        if cap
        else budget.BudgetLedger(budget_path)
    )
    if not ledger.check(estimated):
        raise BudgetExceeded(
            f"Estimation {estimated} $ + cumul {budget.total(budget_path)} $ dépasse le plafond"
        )

    spent: Decimal | None = None
    try:
        image, actual_raw = request_image(
            model, prompt, client, images=images, image_config=image_config, seed=seed
        )
        spent = to_money(actual_raw) if actual_raw is not None else estimated
    except ImageExtractionError as exc:
        if exc.cost is not None:
            spent = to_money(exc.cost)
        raise
    finally:
        if spent is not None and spent > 0:
            budget.add_entry(
                date.today().isoformat(),
                SERVICE_NAME,
                subject or f"image {model}",
                estimated,
                spent,
                path=budget_path,
            )

    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(image)
    return out_path
