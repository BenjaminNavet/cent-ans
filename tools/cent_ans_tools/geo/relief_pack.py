"""Package the fine relief cache for hosting (lot SZ7, ADR 0077).

``data/map/pyramid/`` (tiles E1-E7, ``hydro_fine/``, ``roads_fine/``, ~2.8 GB, hors
git) is archived as an uncompressed ``.tar`` stream, split into parts under
:data:`MAX_PART_BYTES`, alongside a JSON manifest (version, parts, SHA-256 per
part and global, source credits). Nothing is uploaded here: publishing the
result is a separate, player-approved step (see ``docs/geo.md``).

No compression: a spot check (``docs/wip/sz7-hebergement-relief.md``) found zstd
-19 saves ~0% on the already-DEFLATE-compressed PNG tiles and on the packed
binary CAFV tiles, so the extra CPU time and dependency are not worth it.
"""

from __future__ import annotations

import hashlib
import json
import shutil
import tarfile
from dataclasses import dataclass
from pathlib import Path

REPO_DIR = Path(__file__).resolve().parents[3]
MAP_DIR = REPO_DIR / "data" / "map"
PYRAMID_DIR = MAP_DIR / "pyramid"
HOSTING_FILE = MAP_DIR / "relief_hosting.json"
CREDITS_FILE = REPO_DIR / "CREDITS.md"
CREDITS_HEADING = "## Données géographiques"

#: Strictly under 2 GiB (GitHub Releases' per-file limit); leaves margin.
MAX_PART_BYTES = int(1.9 * 1024**3)
#: Safety margin required free on top of the estimated archive size.
DISK_MARGIN = 1.1
PART_NAME = "{package}-v{version}.part{index:03d}.tar"
MANIFEST_NAME = "manifest.json"


@dataclass
class PackResult:
    """Where the pack landed and what it contains."""

    out_dir: Path
    manifest_path: Path
    parts: list[Path]
    total_bytes: int
    version: int


def bake_signature(map_dir: Path = MAP_DIR) -> dict:
    """Read the three fine-relief manifests and compute a combined signature.

    Returns a dict with each manifest's own ``generated_at`` plus a SHA-256
    ``signature`` over their concatenated bytes: it changes whenever any of
    them changes, i.e. whenever the pyramid, the fine rivers or the fine
    roads are rebaked (ADR 0077: "toute recuisson... incrémente la version").
    """
    raise NotImplementedError


def load_hosting(hosting_file: Path = HOSTING_FILE) -> dict:
    """Read ``relief_hosting.json`` (created by SZ7's skeleton commit)."""
    raise NotImplementedError


def bump_version_if_rebaked(map_dir: Path = MAP_DIR, hosting_file: Path = HOSTING_FILE) -> dict:
    """Increment ``version`` and update ``bake`` in-place if the cache changed.

    Writes the updated ``relief_hosting.json`` back to disk and returns it.
    """
    raise NotImplementedError


def extract_credits(credits_file: Path = CREDITS_FILE) -> str:
    """Text of the "Données géographiques" section of ``CREDITS.md``."""
    raise NotImplementedError


def check_free_space(target_dir: Path, estimated_bytes: int) -> None:
    """Raise ``OSError`` if ``target_dir``'s filesystem lacks room for the pack."""
    raise NotImplementedError


def pack(
    out_dir: Path,
    map_dir: Path = MAP_DIR,
    pyramid_dir: Path | None = None,
    hosting_file: Path = HOSTING_FILE,
    credits_file: Path = CREDITS_FILE,
    max_part_bytes: int = MAX_PART_BYTES,
) -> PackResult:
    """Stream ``pyramid_dir`` into split, uncompressed tar parts plus a manifest.

    Args:
        out_dir: Destination folder (created if missing); checked for free
            space before anything is written.
        map_dir: ``data/map`` (for the fine-relief manifests' signature).
        pyramid_dir: Source tree (default: ``map_dir / "pyramid"``).
        hosting_file: ``relief_hosting.json`` (read and updated in place).
        credits_file: ``CREDITS.md`` (geo section copied into the manifest).
        max_part_bytes: Split threshold.
    """
    raise NotImplementedError
