"""Z-Image rendering with resumable cache and retry (T1c, fal backend TX).

Attempt ``n`` (1-based) uses seed ``entry.seed + n - 1`` and is cached as
``<id>_<n>.png``. The manifest records the latest attempt per entry.

Backend (catalogue ``backend``, default ``fal``): ``fal`` calls Z-Image Turbo on
fal.ai in parallel at the requested size (native 2048 allowed, 0.005 $/Mpx);
``local`` renders with mflux one image at a time (ADR 0190). A fal failure is
recorded as ``failed`` and never retried locally in the same run.
"""

from __future__ import annotations

import json
import threading
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from cent_ans_tools.texture_factory import RAW_ROOT
from cent_ans_tools.texture_factory.catalog import build_prompt, select

MAX_ATTEMPTS = 3
FAL_MODEL = "fal-ai/z-image/turbo"
FAL_PRICE_PER_MPX = 0.005  # USD, fal pricing page 2026-10-09
FAL_WORKERS = 8


def backend(document: dict) -> str:
    """Rendering backend of a catalogue (``fal`` unless declared ``local``)."""
    return document.get("backend", "fal")


def entry_side(document: dict, entry: dict) -> int:
    """Rendered side in pixels of one entry."""
    return entry.get("size", document["size"])


def estimate_cost(document: dict, entries: list[dict]) -> float:
    """USD cost of one fal attempt per entry (0 for the local backend)."""
    if backend(document) != "fal":
        return 0.0
    pixels = sum(entry_side(document, entry) ** 2 for entry in entries)
    return round(pixels / 1e6 * FAL_PRICE_PER_MPX, 4)


def fal_render(prompt: str, seed: int, side: int) -> bytes:
    """One Z-Image Turbo call on fal.ai; returns the PNG bytes."""
    import urllib.request

    import fal_client  # only needed for paid calls

    result = fal_client.subscribe(
        FAL_MODEL,
        arguments={
            "prompt": prompt,
            "image_size": {"width": side, "height": side},
            "seed": seed,
            "num_inference_steps": 8,
            "enable_safety_checker": False,
            "output_format": "png",
        },
    )
    with urllib.request.urlopen(result["images"][0]["url"], timeout=120) as response:
        return response.read()


def _family_dir(document: dict, raw_dir: Path | None) -> Path:
    return Path(raw_dir) if raw_dir is not None else RAW_ROOT / document["family"]


def load_manifest(document: dict, raw_dir: Path | None = None) -> dict:
    """Manifest ``{id: {attempt, seed, status}}`` (empty if none yet)."""
    path = _family_dir(document, raw_dir) / "manifest.json"
    return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}


def _save_manifest(document: dict, raw_dir: Path | None, manifest: dict) -> None:
    path = _family_dir(document, raw_dir) / "manifest.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(manifest, indent=2, sort_keys=True), encoding="utf-8")


def image_path(
    document: dict, entry_id: str, attempt: int, raw_dir: Path | None = None
) -> Path:
    """Cache path of attempt ``attempt`` of ``entry_id``."""
    return _family_dir(document, raw_dir) / f"{entry_id}_{attempt}.png"


def _render(document, entry, attempt, raw_dir, runner) -> tuple[int, str]:
    """Render one attempt unless cached; return (seed, ok|failed).

    ``runner`` replaces the mflux subprocess (local) or ``fal_render`` (fal) in tests.
    """
    seed = entry["seed"] + attempt - 1
    path = image_path(document, entry["id"], attempt, raw_dir)
    if path.is_file():
        return seed, "ok"
    side = entry_side(document, entry)
    prompt = build_prompt(document, entry)
    try:
        if backend(document) == "fal":
            data = (runner or fal_render)(prompt, seed, side)
        else:
            from cent_ans_tools.local_art import render_image

            data = render_image(
                prompt, aspect_ratio="1:1", seed=seed, runner=runner, size=(side, side)
            )
    except Exception as error:  # noqa: BLE001 - one failure must not stop the batch
        print(f"[textures] échec {entry['id']} (essai {attempt}) : {error}")
        return seed, "failed"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return seed, "ok"


def _run_attempt(document, entries, attempts, raw_dir, runner) -> dict:
    manifest = load_manifest(document, raw_dir)
    lock = threading.Lock()

    def run_one(entry: dict) -> None:
        attempt = attempts(entry)
        seed, status = _render(document, entry, attempt, raw_dir, runner)
        with lock:
            manifest[entry["id"]] = {"attempt": attempt, "seed": seed, "status": status}
            _save_manifest(document, raw_dir, manifest)

    workers = FAL_WORKERS if backend(document) == "fal" else 1
    with ThreadPoolExecutor(max_workers=workers) as pool:
        list(pool.map(run_one, entries))
    return manifest


def generate(
    document: dict,
    only: list[str] | None = None,
    raw_dir: Path | None = None,
    runner=None,
) -> dict:
    """Attempt 1 (catalogue seed) for each selected entry; cached images are kept."""
    return _run_attempt(document, select(document, only), lambda _e: 1, raw_dir, runner)


def retry_flagged(
    document: dict,
    failed_ids: list[str],
    raw_dir: Path | None = None,
    runner=None,
) -> dict:
    """One retry step for entries that failed rendering or the checks.

    Each id moves to its next attempt (seed + 1, then seed + 2: attempts 2 and 3).
    An id already at attempt 3 becomes ``flagged`` (manual review) and is not rendered.
    The caller re-runs its checks and calls again with the ids still failing.
    """
    manifest = load_manifest(document, raw_dir)
    retry = []
    for entry in select(document, list(failed_ids)):
        if manifest.get(entry["id"], {}).get("attempt", 1) >= MAX_ATTEMPTS:
            manifest[entry["id"]]["status"] = "flagged"
        else:
            retry.append(entry)
    _save_manifest(document, raw_dir, manifest)
    return _run_attempt(
        document,
        retry,
        lambda e: manifest.get(e["id"], {}).get("attempt", 1) + 1,
        raw_dir,
        runner,
    )
