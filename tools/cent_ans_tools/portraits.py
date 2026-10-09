"""Character portraits generated through OpenRouter, in one illuminated-manuscript style.

For every historical character of ``data/characters`` alive in 1337 or born before
1400, a prompt is built from the data (name, age in 1337, rank, house, faction arms,
traits, description). Images are centre-cropped and resized to 256x256 PNG in
``game/assets/portraits/<id>.png``.

Safety rails:

- idempotent: an existing portrait is never regenerated;
- before each paid call, the cumulative spend of the batch plus the next estimate
  must fit both the task envelope (``envelope``, 10 $ by default) and the global cap
  of ``docs/budget.md`` (:func:`cent_ans_tools.budget.check`), otherwise
  :class:`~cent_ans_tools.budget.BudgetExceeded` is raised;
- the batch is recorded as **one** row of ``docs/budget.md`` (estimated and real
  cost), even when it stops early on an error;
- ``dry_run`` prints prompts and the estimate without any network call.
"""

from __future__ import annotations

import io
import json
from collections.abc import Callable
from dataclasses import dataclass
from datetime import date
from decimal import ROUND_UP, Decimal
from pathlib import Path

import httpx
from PIL import Image

from cent_ans_tools import budget, openrouter
from cent_ans_tools.budget import DEFAULT_BUDGET_PATH, BudgetExceeded, to_money
from cent_ans_tools.paths import REPO_DIR

DATA_DIR = REPO_DIR / "data"
PORTRAITS_DIR = REPO_DIR / "game" / "assets" / "portraits"

DEFAULT_MODEL = "openai/gpt-5-image-mini"
DEFAULT_ENVELOPE = Decimal("10.00")
# Per-image USD prices used offline by --dry-run and as a floor for the pre-call
# estimate: token-based listing (`cent-ans models list`, 2026-09-23), raised to the
# real cost observed for gpt-5-image-mini (0.0455 $ per portrait, not 0.0113 $).
KNOWN_PRICES = {
    "openai/gpt-5-image-mini": Decimal("0.0455"),
    "google/gemini-2.5-flash-image": Decimal("0.0390"),
    "google/gemini-3.1-flash-lite-image": Decimal("0.0389"),
    "local/z-image-turbo": Decimal("0"),  # free, on this machine (ADR 0190)
}
PORTRAIT_SIZE = 256
# Output token cap per call (image ~1 056-1 290 tokens + reasoning); bounds the reservation.
MAX_TOKENS = 4096
START_YEAR = 1337

# Attempts per image before skipping it (refusals, text-only answers, timeouts).
MAX_ATTEMPTS = 3

STYLE = (
    "Style: 14th-century French Gothic manuscript illumination (enluminure gothique "
    "française du XIVe siècle). Three-quarter bust portrait, flat gilded gold and "
    "azure blue patterned background, thin painted parchment frame, egg tempera "
    "colours, fine black ink outlines, subtle gold leaf highlights. Period-accurate "
    "1330s-1360s clothing and headwear. Square composition, the face centred in the "
    "upper half. No text, no letters, no captions, no modern elements."
)

ROLE_LABELS = {
    "ruler": "ruling sovereign",
    "consort": "royal consort",
    "heir": "heir",
    "prince": "prince of the blood",
    "regent": "regent",
    "commander": "military commander",
    "noble": "high noble",
    "prelate": "high prelate",
    "claimant": "claimant to a throne",
    "burgher": "rich burgher and town leader",
    "exile": "exiled lord",
}


@dataclass
class PortraitJob:
    """One character to paint."""

    character_id: str
    prompt: str
    out_path: Path
    # DA2: optional reference picture sent with the prompt (portrait to age).
    reference: Path | None = None


def _year(value: str | None) -> int | None:
    if not value:
        return None
    try:
        return int(str(value)[:4])
    except ValueError:
        return None


def _load_dir(directory: Path) -> dict[str, dict]:
    return {
        path.stem: json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(directory.glob("*.json"))
    }


def is_eligible(character: dict) -> bool:
    """Historical, born before 1400, not dead before the 1337 start."""
    if not character.get("historical", False):
        return False
    birth = _year((character.get("birth") or {}).get("value"))
    death = _year((character.get("death") or {}).get("value"))
    if birth is None or birth >= 1400:
        return False
    return death is None or death >= START_YEAR


def build_prompt(
    character: dict,
    factions: dict[str, dict],
    traits: dict[str, dict],
    depicted: str | None = None,
) -> str:
    """Portrait prompt built from the character data only (no hard-coded people).

    ``depicted`` replaces the age sentence (DA2 aged variants: "depicted at about 62").
    """
    name = character["name"]["display"]
    local = character["name"].get("local", "")
    birth = _year(character["birth"]["value"]) or START_YEAR
    age = START_YEAR - birth
    if depicted is not None:
        age_text = depicted
    elif age >= 18:
        age_text = f"aged {age} in 1337"
    elif age >= 0:
        age_text = f"aged {age} in 1337, depicted as a young adult of about 20"
    else:
        age_text = f"born in {birth}, depicted as a young adult of about 20"
    sex = "woman" if character.get("sex") == "female" else "man"
    role = ROLE_LABELS.get(character.get("role", ""), character.get("role", "noble"))
    titles = [entry["title"] for entry in character.get("titles", [])][:2]
    faction = factions.get(character.get("faction", ""), {})
    heraldry = faction.get("heraldry", {})
    trait_names = [
        traits[trait]["name"]["display"]
        for trait in character.get("traits", [])
        if trait in traits
    ]
    lines = [
        f"Portrait of {name}"
        + (f" ({local})" if local and local != name else "")
        + f", a {sex} {age_text}.",
        f"Rank: {role}" + (f" — {', '.join(titles)}" if titles else "") + ".",
    ]
    if character.get("house"):
        lines.append(f"House: {character['house']}.")
    if heraldry.get("blazon"):
        lines.append(
            f"Heraldry worn on the clothing or a small shield: {heraldry['blazon']}"
        )
    if trait_names:
        lines.append(
            f"Personality to convey in the expression: {', '.join(trait_names)}."
        )
    if character.get("description"):
        lines.append(f"Context: {character['description']}")
    lines.append(STYLE)
    return "\n".join(lines)


def plan(
    data_dir: Path = DATA_DIR, out_dir: Path = PORTRAITS_DIR, limit: int | None = None
) -> list[PortraitJob]:
    """Portraits still missing (idempotence), in character id order, up to ``limit``."""
    characters = _load_dir(data_dir / "characters")
    factions = _load_dir(data_dir / "factions")
    traits = _load_dir(data_dir / "traits")
    jobs = []
    for character_id, character in characters.items():
        out_path = out_dir / f"{character_id}.png"
        if not is_eligible(character) or out_path.exists():
            continue
        jobs.append(
            PortraitJob(
                character_id, build_prompt(character, factions, traits), out_path
            )
        )
        if limit is not None and len(jobs) >= limit:
            break
    return jobs


def to_portrait_png(image_bytes: bytes, size: int = PORTRAIT_SIZE) -> bytes:
    """Centre-crop to a square (biased to the top for tall images) and resize."""
    image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    width, height = image.size
    side = min(width, height)
    left = (width - side) // 2
    top = 0 if height > width else (height - side) // 2
    square = image.crop((left, top, left + side, top + side))
    buffer = io.BytesIO()
    square.resize((size, size), Image.Resampling.LANCZOS).save(
        buffer, "PNG", optimize=True
    )
    return buffer.getvalue()


def price_per_image(model: str, client: httpx.Client | None = None) -> Decimal:
    """Unrounded estimated USD price of one image with ``model``."""
    if model in KNOWN_PRICES and KNOWN_PRICES[model] == 0:
        return Decimal("0")
    for candidate in openrouter.list_image_models(client):
        if candidate.id == model:
            price = candidate.estimated_price_per_image()
            if price is not None:
                return price
    raise ValueError(f"Modèle d'image inconnu ou sans tarif : {model}")


@dataclass
class BatchResult:
    """Outcome of a portrait batch."""

    written: list[Path]
    estimated: Decimal
    actual: Decimal


def generate(
    jobs: list[PortraitJob],
    model: str = DEFAULT_MODEL,
    *,
    envelope: Decimal = DEFAULT_ENVELOPE,
    budget_path: Path | str = DEFAULT_BUDGET_PATH,
    budget_session: str | None = None,
    client: httpx.Client | None = None,
    subject: str | None = None,
    on_progress: Callable[[PortraitJob, Decimal], None] | None = None,
    convert: Callable[[bytes], bytes] = to_portrait_png,
    image_config: dict[str, str] | None = None,
) -> BatchResult:
    """Paid batch: generate each job, guarded by the envelope and the global cap.

    ``convert`` turns the raw model image into the saved file (portrait crop by
    default; :mod:`cent_ans_tools.event_art` passes its wide miniature crop).
    ``budget_session`` names the ``docs/budget.md`` heading that funds this batch
    (see :meth:`cent_ans_tools.budget.BudgetLedger.find_session`); left out, the
    ledger row goes to the last table in the file, as before. A caller whose
    envelope is not always the last section (several pipelines share one, and a
    later, unrelated section can be appended after it) must pass it explicitly.
    ``image_config`` (e.g. ``{"aspect_ratio": "16:9"}``) goes to every request.
    """
    unit = max(price_per_image(model, client), KNOWN_PRICES.get(model, Decimal("0")))
    written: list[Path] = []
    failed: list[tuple[PortraitJob, str]] = []
    last_error: Exception | None = None
    spent = Decimal("0")
    estimated = Decimal("0")
    try:
        for job in jobs:
            if spent + unit > envelope:
                raise BudgetExceeded(
                    f"Enveloppe de {envelope} $ atteinte ({spent:.4f} $ dépensés)"
                )
            if not budget.check(to_money(spent + unit), budget_path):
                raise BudgetExceeded(
                    "Le plafond global de docs/budget.md serait dépassé"
                )
            extra: dict = {"image_config": image_config} if image_config else {}
            if job.reference is not None:
                extra["images"] = [job.reference.read_bytes()]
            image = None
            for _attempt in range(MAX_ATTEMPTS):
                estimated += unit
                try:
                    image, cost = openrouter.request_image(
                        model, job.prompt, client, max_tokens=MAX_TOKENS, **extra
                    )
                except openrouter.ImageExtractionError as exc:
                    # A refused or text-only answer may be billed: record its real cost.
                    spent += exc.cost if exc.cost is not None else unit
                    last_error = exc
                    continue
                except httpx.HTTPError as exc:
                    last_error = exc
                    continue
                break
            if image is None:
                failed.append((job, str(last_error)))
                continue
            spent += cost if cost is not None else unit
            job.out_path.parent.mkdir(parents=True, exist_ok=True)
            job.out_path.write_bytes(convert(image))
            written.append(job.out_path)
            if on_progress is not None:
                on_progress(job, spent)
    finally:
        if spent > 0:
            budget.add_entry(
                date.today().isoformat(),
                openrouter.SERVICE_NAME,
                subject or f"Portraits M10 ({len(written)} × {model})",
                estimated.quantize(Decimal("0.01"), rounding=ROUND_UP),
                spent.quantize(Decimal("0.01"), rounding=ROUND_UP),
                path=budget_path,
                session=budget_session,
            )
    for job, error in failed:
        print(f"Échec après {MAX_ATTEMPTS} essais : {job.out_path.name} ({error})")
    return BatchResult(written, estimated, spent)
