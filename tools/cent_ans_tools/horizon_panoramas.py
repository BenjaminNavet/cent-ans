"""Painted horizon panoramas for the battles (lot EP2, ADR 0032).

Prompts live in ``data/fx/horizon.json`` (``panoramas.<id>.prompt`` inside
``prompt_template``). A paid run asks OpenRouter for one painting per id, keeps
the raw picture (``tools/horizon_raw/<id>.jpg``, so the processing can be tuned
again for free) and records each call in ``docs/budget.md`` (current table).

Processing (free, :func:`process`): the plain sky requested by the prompt is
keyed out column by column (the painted skyline becomes the alpha edge), the
picture is cropped to the band between the highest crest and the bottom, and
saved as ``game/assets/horizon/panoramas/<id>.webp`` (RGBA). The skyline of
every column (0 = top of the band, 1 = bottom) goes to
``game/assets/horizon/panoramas/panoramas.json``: the battle shader stretches
each column so that the painted crest lands on the real skyline.
"""

from __future__ import annotations

import io
import json
from dataclasses import dataclass
from datetime import date
from decimal import Decimal
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools import budget, local_art, openrouter

REPO_DIR = Path(__file__).resolve().parents[2]
DATA_PATH = REPO_DIR / "data" / "fx" / "horizon.json"
RAW_DIR = REPO_DIR / "tools" / "horizon_raw"
OUT_DIR = REPO_DIR / "game" / "assets" / "horizon" / "panoramas"
DEFAULT_MODEL = "openai/gpt-5-image-mini"
#: Pre-call estimate per image (ledger rows of 2026-09-24: 0,045-0,05 $).
ESTIMATE_PER_IMAGE = Decimal("0.05")
LOT_CAP = Decimal("3.00")
# Aspect ratio of a locally generated painting (very wide, sky then crest).
LOCAL_ASPECT = "21:9"
SKYLINE_SAMPLES = 512
OUT_WIDTH = 2048
FEATHER_PX = 2.5


@dataclass
class Band:
    """A processed panorama band."""

    image: Image.Image  # RGBA
    skyline: np.ndarray  # (SKYLINE_SAMPLES,) 0 = top of the band, 1 = bottom


def load_data(path: Path = DATA_PATH) -> dict:
    """The horizon data file."""
    return json.loads(path.read_text(encoding="utf-8"))


def prompt_for(data: dict, panorama_id: str) -> str:
    """Full prompt of one panorama."""
    return data["prompt_template"].replace(
        "{prompt}", data["panoramas"][panorama_id]["prompt"]
    )


def sky_model(rgb: np.ndarray, jump: float = 0.12, spread: float = 0.07) -> np.ndarray:
    """Plain-sky colour of every row (``(h, 3)``).

    The per-row median (robust to crests covering a minority of the columns) is the
    sky while it varies smoothly and the row stays uniform; below the first break
    (the land takes over), the gradient of the last fifth of the sky rows is
    extrapolated linearly.
    """
    h = rgb.shape[0]
    median = ndimage.uniform_filter1d(np.median(rgb, axis=1), size=15, axis=0)
    spread_row = np.median(np.abs(rgb - median[:, None, :]).sum(axis=2), axis=1)
    end = h
    start = max(16, int(0.06 * h))  # the painted canvas is often darker at the very top
    for row in range(start, h):
        step = np.abs(median[row] - median[row - 12]).sum()
        if step > jump or spread_row[row] > spread:
            end = row
            break
    fit_from = max(end - max(end // 5, 8), min(start, end - 2), 0)
    rows = np.arange(fit_from, end)
    model = median.copy()
    if end < h and rows.size >= 2:
        for channel in range(3):
            slope, intercept = np.polyfit(rows, median[fit_from:end, channel], 1)
            below = np.arange(end, h)
            model[end:, channel] = intercept + slope * below
    return np.clip(model, 0.0, 1.0)


def sky_mask(rgb: np.ndarray, threshold: float = 0.08) -> np.ndarray:
    """Per-column skyline row of a painting over a plain sky.

    A pixel belongs to the land when it departs from :func:`sky_model` by more than
    ``threshold`` (RGB distance, 0-1) and stays so for a few rows; the per-column
    crest is then median-smoothed.

    Args:
        rgb: ``float`` array ``(h, w, 3)`` in 0-1.
        threshold: Colour distance separating land from sky.

    Returns:
        ``float`` array ``(w,)``: skyline row of each column.
    """
    h, w, _ = rgb.shape
    model = sky_model(rgb)
    distance = np.sqrt(((rgb - model[:, None, :]) ** 2).sum(axis=2))
    distance = ndimage.uniform_filter(distance, size=(5, 3))
    land = distance > threshold
    # A crest is the first row where the next rows are land as well.
    solid = (
        ndimage.minimum_filter1d(land.astype(np.uint8), size=6, axis=0, origin=-2) > 0
    )
    solid[:4] = False
    first = np.where(solid.any(axis=0), solid.argmax(axis=0), h - 1).astype(np.float64)
    crest = ndimage.median_filter(first, size=9, mode="wrap")
    # Thin upward spikes (keying noise on a painted sky) are clipped to the local crest.
    crest = ndimage.grey_closing(crest, size=15, mode="wrap")
    wide = ndimage.median_filter(crest, size=61, mode="wrap")
    return np.maximum(crest, wide - 0.06 * h)


def process(raw: Image.Image, width: int = OUT_WIDTH, depth: float = 0.16) -> Band:
    """Key the sky out, crop to the land band, compute the skyline profile.

    The band runs from just above the highest crest down to ``depth`` (share of the
    picture height) below the lowest one: only the far distance, never the painted
    foreground.
    """
    rgb = np.asarray(raw.convert("RGB"), dtype=np.float64) / 255.0
    h, w, _ = rgb.shape
    crest = sky_mask(rgb)
    top = int(max(np.floor(crest.min()) - 6, 0))
    bottom = int(min(np.ceil(crest.max() + depth * h), h))
    rows = np.arange(h)[:, None]
    alpha = np.clip((rows - crest[None, :]) / FEATHER_PX + 0.5, 0.0, 1.0)
    rgba = np.dstack([rgb, alpha])[top:bottom]
    band_h = rgba.shape[0]
    # Bleed: sky pixels take the land colour just below the crest so that bilinear
    # filtering never mixes the painted sky into the edge.
    edge = np.clip(np.ceil(crest - top + 1), 0, band_h - 1).astype(int)
    for c in range(3):
        channel = rgba[:, :, c]
        below = channel[edge, np.arange(w)]
        channel[:] = np.where(rgba[:, :, 3] < 0.5, below[None, :], channel)
    out_h = int(round(band_h * width / w))
    image = Image.fromarray(np.uint8(np.clip(rgba, 0, 1) * 255 + 0.5), "RGBA").resize(
        (width, out_h), Image.Resampling.LANCZOS
    )
    samples = np.interp(
        np.linspace(0, w - 1, SKYLINE_SAMPLES),
        np.arange(w),
        (crest - top) / max(band_h - 1, 1),
    )
    return Band(image=image, skyline=np.clip(samples, 0.0, 1.0))


def process_all(
    data: dict | None = None, raw_dir: Path = RAW_DIR, out_dir: Path = OUT_DIR
) -> dict:
    """Process every raw painting present; write the WebP bands and the skyline JSON."""
    data = data or load_data()
    out_dir.mkdir(parents=True, exist_ok=True)
    meta: dict = {
        "description": "Bandes d'horizon peintes (lot EP2) : ligne de crête par colonne (0 = haut de la bande, 1 = bas), produite par `cent-ans horizon-panoramas --process`.",
        "panoramas": {},
    }
    for panorama_id in data["panoramas"]:
        raw_path = raw_dir / f"{panorama_id}.jpg"
        if not raw_path.exists():
            continue
        band = process(Image.open(raw_path))
        band.image.save(out_dir / f"{panorama_id}.webp", "WEBP", quality=86, method=6)
        meta["panoramas"][panorama_id] = {
            "size": list(band.image.size),
            "skyline": [round(float(v), 4) for v in band.skyline],
        }
    (out_dir / "panoramas.json").write_text(json.dumps(meta) + "\n", encoding="utf-8")
    return meta


def generate(
    ids: list[str],
    model: str = DEFAULT_MODEL,
    dry_run: bool = False,
    data: dict | None = None,
    raw_dir: Path = RAW_DIR,
    budget_path: Path | str = budget.DEFAULT_BUDGET_PATH,
) -> list[tuple[str, Decimal]]:
    """Paid: one painting per id (skips ids already in ``raw_dir``); returns costs.

    With the free local model (:data:`local_art.MODEL_ID`, ADR 0190) nothing is estimated,
    checked or recorded in the ledger; the painting is 21:9.
    """
    data = data or load_data()
    local = model == local_art.MODEL_ID
    per_image = Decimal("0") if local else ESTIMATE_PER_IMAGE
    todo = [i for i in ids if not (raw_dir / f"{i}.jpg").exists()]
    estimate = per_image * len(todo)
    if estimate > LOT_CAP:
        raise budget.BudgetExceeded(f"{estimate} $ > plafond du lot {LOT_CAP} $")
    if dry_run:
        for panorama_id in todo:
            print(f"[dry-run] {panorama_id}: {prompt_for(data, panorama_id)}")
        return []
    raw_dir.mkdir(parents=True, exist_ok=True)
    costs = []
    for panorama_id in todo:
        if not local and not budget.check(ESTIMATE_PER_IMAGE, budget_path):
            raise budget.BudgetExceeded("plafond de la session atteint")
        extra = {"image_config": {"aspect_ratio": LOCAL_ASPECT}} if local else {}
        image, actual = openrouter.request_image(
            model, prompt_for(data, panorama_id), max_tokens=4000, **extra
        )
        cost = budget.to_money(actual) if actual is not None else ESTIMATE_PER_IMAGE
        if not local:
            budget.add_entry(
                date.today().isoformat(),
                openrouter.SERVICE_NAME,
                f"EP2 : panorama d'horizon « {panorama_id} » (1 × {model})",
                ESTIMATE_PER_IMAGE,
                cost,
                path=budget_path,
            )
        Image.open(io.BytesIO(image)).convert("RGB").save(
            raw_dir / f"{panorama_id}.jpg", "JPEG", quality=92
        )
        costs.append((panorama_id, cost))
        print(f"{panorama_id}: {cost} $")
    return costs
