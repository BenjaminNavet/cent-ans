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
import os
import shutil
import tarfile
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from urllib.error import HTTPError

from cent_ans_tools.geo import bake_stamp, relief_pack

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
    from_env = os.environ.get(DEST_ENV_VAR, "").strip()
    if from_env:
        return Path(from_env)
    return MAP_DIR


def needs_fetch(
    dest: Path | None = None, hosting_file: Path = HOSTING_FILE
) -> tuple[bool, str]:
    """Whether the installed cache is older than the hosted package (ADR 0149).

    The cache carries the package version it holds (``pyramid/package.json``,
    :func:`relief_pack.write_installed_marker`). A cache without marker (installed
    or baked before ADR 0149) is judged on its bake stamps: behind when a tier is
    missing or older than the package's; otherwise it is adopted (marker written),
    so that a cache rebaked locally ahead of the package is never overwritten.

    Returns:
        ``(needed, reason)``, the reason in French for the CLI.
    """
    dest_dir = Path(dest) if dest is not None else default_dest(hosting_file)
    hosting = relief_pack.load_hosting(hosting_file)
    version = int(hosting.get("version", 1))
    pyramid_dir = dest_dir / "pyramid"
    if not pyramid_dir.is_dir():
        return True, "cache de relief absent"
    marker = relief_pack.read_installed_marker(pyramid_dir)
    if "version" in marker:
        installed = int(marker["version"])
        if installed < version:
            return True, f"paquet v{installed} installé, v{version} publié"
        return False, f"paquet v{installed} installé"
    stamps = bake_stamp.read(pyramid_dir)
    expected = (hosting.get("bake") or {}).get("pyramid_bake_versions") or {}
    behind = [
        tier
        for tier, wanted in sorted(expected.items())
        if tier not in stamps or stamps[tier].version < int(wanted)
    ]
    if behind:
        return True, f"cuisson antérieure au paquet v{version} ({', '.join(behind)})"
    relief_pack.write_installed_marker(pyramid_dir, hosting)
    return False, f"cache sans marque, au moins aussi récent que le paquet v{version}"


def discard_partial_download(
    dest: Path | None = None, hosting_file: Path = HOSTING_FILE
) -> bool:
    """Remove the leftover parts of an interrupted download (cache already current)."""
    dest_dir = Path(dest) if dest is not None else default_dest(hosting_file)
    hosting = relief_pack.load_hosting(hosting_file)
    staging = dest_dir / f".pyramid.download-{hosting['package_name']}"
    if not staging.is_dir():
        return False
    shutil.rmtree(staging, ignore_errors=True)
    return True


def _read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _read_manifest(path: Path) -> dict:
    """Read a package manifest, rejecting part names that could escape their folder."""
    manifest = _read_json(path)
    for entry in manifest["parts"]:
        name = entry["name"]
        if (
            not isinstance(name, str)
            or name in ("", ".", "..")
            or Path(name).name != name
            or "\\" in name
        ):
            raise ValueError(f"nom de part invalide dans le manifeste : {name!r}")
    return manifest


def download_part(url: str, dest: Path, expected_sha256: str, log=print) -> None:  # noqa: ANN001
    """Fetch one part with HTTP Range resume; verify its SHA-256 when done."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    existing = dest.stat().st_size if dest.exists() else 0
    digest = hashlib.sha256()
    mode = "wb"
    if existing:
        with dest.open("rb") as already:
            while chunk := already.read(CHUNK_BYTES):
                digest.update(chunk)
        mode = "r+b"
    request = urllib.request.Request(url)
    if existing:
        request.add_header("Range", f"bytes={existing}-")
    try:
        response = urllib.request.urlopen(request)  # noqa: S310
    except HTTPError as error:
        if error.code == 416:  # Range not satisfiable: already complete.
            response = None
        else:
            raise
    if response is not None:
        resumed = existing > 0 and response.status == 206
        if existing and not resumed:
            # The server ignored the Range request: restart from scratch.
            digest = hashlib.sha256()
            mode = "wb"
        with dest.open(mode) as out:
            if mode == "r+b":
                out.seek(existing)
            while chunk := response.read(CHUNK_BYTES):
                out.write(chunk)
                digest.update(chunk)
        response.close()
        log(f"{dest.name} : {dest.stat().st_size / 1e6:.1f} Mo")
    actual = digest.hexdigest()
    if actual != expected_sha256:
        dest.unlink(missing_ok=True)
        raise ValueError(
            f"{dest.name} : somme de contrôle invalide (attendu {expected_sha256}, obtenu {actual})"
        )


class _ChainedReader:
    """Sequential, read-only view over several files, for streaming tar extraction."""

    def __init__(self, paths: list[Path]) -> None:
        self._paths = list(paths)
        self._index = 0
        self._current = self._paths[0].open("rb") if self._paths else None

    def read(self, size: int = -1) -> bytes:
        if self._current is None:
            return b""
        data = self._current.read(size)
        if data:
            return data
        self._current.close()
        self._index += 1
        if self._index >= len(self._paths):
            self._current = None
            return b""
        self._current = self._paths[self._index].open("rb")
        return self.read(size)


def verify_parts(parts_dir: Path, manifest: dict) -> None:
    """Raise ``ValueError`` if any part's SHA-256 does not match the manifest."""
    for entry in manifest["parts"]:
        path = parts_dir / entry["name"]
        if not path.exists():
            raise ValueError(f"{path} : part manquante")
        digest = hashlib.sha256()
        with path.open("rb") as handle:
            while chunk := handle.read(CHUNK_BYTES):
                digest.update(chunk)
        actual = digest.hexdigest()
        if actual != entry["sha256"]:
            raise ValueError(
                f"{path.name} : somme de contrôle invalide (attendu {entry['sha256']}, obtenu {actual})"
            )
        if path.stat().st_size != entry["bytes"]:
            raise ValueError(
                f"{path.name} : taille inattendue ({path.stat().st_size} au lieu de {entry['bytes']})"
            )


def extract_atomic(
    parts: list[Path],
    dest_dir: Path,
    log=print,  # noqa: ANN001
    root: str = "pyramid",
    label: str = "Relief",
) -> Path:
    """Stream-extract the chained parts into ``dest_dir``, swapped in atomically.

    ``root`` is the single folder at the archive's root (``pyramid`` for the relief,
    ``dn`` for the generated models package, ADR 0212).
    """
    dest_dir.mkdir(parents=True, exist_ok=True)
    tmp_dir = dest_dir / f".{root}.tmp-{os.getpid()}"
    if tmp_dir.exists():
        shutil.rmtree(tmp_dir)
    tmp_dir.mkdir()
    reader = _ChainedReader(parts)
    with tarfile.open(fileobj=reader, mode="r|") as tar:
        tar.extractall(tmp_dir, filter="data")  # noqa: S202
    extracted = tmp_dir / root
    if not extracted.is_dir():
        shutil.rmtree(tmp_dir)
        raise ValueError(f"Archive invalide : aucun dossier « {root} » à la racine")
    final = dest_dir / root
    old = None
    if final.exists():
        old = dest_dir / f".{root}.old-{os.getpid()}"
        if old.exists():
            shutil.rmtree(old)
        os.rename(final, old)
    os.rename(extracted, final)
    shutil.rmtree(tmp_dir, ignore_errors=True)
    if old is not None:
        shutil.rmtree(old, ignore_errors=True)
    log(f"{label} installé : {final}")
    return final


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
    dest_dir = Path(dest) if dest is not None else default_dest(hosting_file)
    if from_dir is not None:
        from_dir = Path(from_dir)
        manifest = _read_manifest(from_dir / relief_pack.MANIFEST_NAME)
        verify_parts(from_dir, manifest)
        parts = [from_dir / entry["name"] for entry in manifest["parts"]]
    else:
        hosting = relief_pack.load_hosting(hosting_file)
        base = (base_url or hosting["base_url"]).format(version=hosting["version"])
        if not base.endswith("/"):
            base += "/"
        staging = dest_dir / f".pyramid.download-{hosting['package_name']}"
        staging.mkdir(parents=True, exist_ok=True)
        manifest_path = staging / relief_pack.MANIFEST_NAME
        with urllib.request.urlopen(base + relief_pack.MANIFEST_NAME) as response:  # noqa: S310
            manifest_path.write_bytes(response.read())
        manifest = _read_manifest(manifest_path)
        for entry in manifest["parts"]:
            download_part(
                base + entry["name"], staging / entry["name"], entry["sha256"], log
            )
        verify_parts(staging, manifest)
        parts = [staging / entry["name"] for entry in manifest["parts"]]

    pyramid_dir = extract_atomic(parts, dest_dir, log)
    relief_pack.write_installed_marker(pyramid_dir, manifest)
    if from_dir is None:
        shutil.rmtree(parts[0].parent, ignore_errors=True)
    return FetchResult(
        dest_dir=dest_dir,
        pyramid_dir=pyramid_dir,
        total_bytes=manifest.get("total_bytes", 0),
        parts=len(parts),
    )
