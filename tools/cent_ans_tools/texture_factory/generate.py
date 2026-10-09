"""Local Z-Image rendering with resumable cache and retry (T1c).

Attempt ``n`` (1-based) uses seed ``entry.seed + n - 1`` and is cached as
``<id>_<n>.png``. The manifest records the latest attempt per entry.
"""

from __future__ import annotations

import json
from pathlib import Path

from cent_ans_tools.texture_factory import RAW_ROOT
from cent_ans_tools.texture_factory.catalog import build_prompt, select

MAX_ATTEMPTS = 3


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
    """Render one attempt unless cached; return (seed, ok|failed)."""
    from cent_ans_tools.local_art import render_image

    seed = entry["seed"] + attempt - 1
    path = image_path(document, entry["id"], attempt, raw_dir)
    if path.is_file():
        return seed, "ok"
    side = entry.get("size", document["size"])
    try:
        data = render_image(
            build_prompt(document, entry),
            aspect_ratio="1:1",
            seed=seed,
            runner=runner,
            size=(side, side),
        )
    except Exception as error:  # noqa: BLE001 - one failure must not stop the batch
        print(f"[textures] échec {entry['id']} (essai {attempt}) : {error}")
        return seed, "failed"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return seed, "ok"


def _run_attempt(document, entries, attempts, raw_dir, runner) -> dict:
    manifest = load_manifest(document, raw_dir)
    for entry in entries:
        attempt = attempts(entry)
        seed, status = _render(document, entry, attempt, raw_dir, runner)
        manifest[entry["id"]] = {"attempt": attempt, "seed": seed, "status": status}
        _save_manifest(document, raw_dir, manifest)
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
