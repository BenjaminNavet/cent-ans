"""Download and install the fine relief cache (lot SZ7, ADR 0077).

Fetches the parts produced by :mod:`cent_ans_tools.geo.relief_pack` (from an
HTTP base URL, resuming with plain ``Range`` requests via the stdlib
``urllib``, or from a local directory with ``--from-dir``), verifies their
SHA-256 against the manifest, then extracts the reassembled tar stream
atomically into the destination's relief folder without ever materialising
the full multi-gigabyte tar on disk.
"""

from __future__ import annotations

import hashlib
import json
import tarfile
import urllib.request
from dataclasses import dataclass
from pathlib import Path

from cent_ans_tools.geo import relief_pack

REPO_DIR = relief_pack.REPO_DIR
MAP_DIR = relief_pack.MAP_DIR
HOSTING_FILE = relief_pack.HOSTING_FILE
DEST_ENV_VAR = "CENT_ANS_RELIEF_DIR"
CHUNK_BYTES = 4 * 1024 * 1024


@dataclass
class FetchResult:
    """Where the relief cache was installed and how much moved."""

    dest_dir: Path
    pyramid_dir: Path
    total_bytes: int
    parts: int


def default_dest(hosting_file: Path = HOSTING_FILE) -> Path:
    """Resolve the destination folder like ``MapPaths.relief_root_for`` would.

    Order: ``CENT_ANS_RELIEF_DIR``, else ``data/map`` (the pyramid lands at
    ``data/map/pyramid``, the first place the game looks).
    """
    raise NotImplementedError


def download_part(url: str, dest: Path, expected_sha256: str, log=print) -> None:  # noqa: ANN001
    """Fetch one part with HTTP Range resume; verify its SHA-256 when done."""
    raise NotImplementedError


class _ChainedReader:
    """Sequential, read-only view over several files, for streaming tar extraction."""

    def __init__(self, paths: list[Path]) -> None:
        raise NotImplementedError

    def read(self, size: int = -1) -> bytes:
        raise NotImplementedError


def verify_parts(parts_dir: Path, manifest: dict) -> None:
    """Raise ``ValueError`` if any part's SHA-256 does not match the manifest."""
    raise NotImplementedError


def extract_atomic(parts: list[Path], dest_dir: Path, log=print) -> Path:  # noqa: ANN001
    """Stream-extract the chained parts into ``dest_dir``, swapped in atomically."""
    raise NotImplementedError


def fetch(
    dest: Path | None = None,
    base_url: str | None = None,
    from_dir: Path | None = None,
    hosting_file: Path = HOSTING_FILE,
    log=print,  # noqa: ANN001
) -> FetchResult:
    """Install the relief cache into ``dest`` from ``base_url`` or ``from_dir``.

    Downloads (or copies) the manifest and every part, verifies checksums,
    extracts atomically, then leaves cache verification to the caller
    (``cent-ans geo relief-all --check``).
    """
    raise NotImplementedError
