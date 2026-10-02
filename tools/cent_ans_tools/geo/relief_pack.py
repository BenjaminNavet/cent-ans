"""Package the fine relief cache for hosting (lot SZ7, ADR 0077).

``data/map/pyramid/`` (tiles E1-E7, ``hydro_fine/``, ``roads_fine/``, ~2.8 GB, hors
git) is archived as an uncompressed ``.tar`` stream, split into parts under
:data:`MAX_PART_BYTES`, alongside a JSON manifest (version, parts, SHA-256 per
part and global, source credits). Nothing is uploaded here: publishing the
result is the job of :mod:`cent_ans_tools.geo.relief_update` (ADR 0149).

No compression: a spot check (``docs/wip/sz7-hebergement-relief.md``) found zstd
-19 saves ~0% on the already-DEFLATE-compressed PNG tiles and on the packed
binary CAFV tiles, so the extra CPU time and dependency are not worth it.
"""

from __future__ import annotations

import hashlib
import json
import shutil
import tarfile
from dataclasses import dataclass, field
from datetime import UTC, datetime
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

PYRAMID_MANIFEST = "relief_pyramid.json"
RIVERS_MANIFEST = "rivers_fine.json"
ANCHORS_MANIFEST = "fine_anchors.json"
#: ``pyramid/package.json``: which package version the cache on disk holds (ADR 0149).
INSTALLED_MARKER = "package.json"


@dataclass
class PackResult:
    """Where the pack landed and what it contains."""

    out_dir: Path
    manifest_path: Path
    parts: list[Path] = field(default_factory=list)
    total_bytes: int = 0
    version: int = 1


def _read_json(path: Path) -> dict:
    if not path.exists():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    return data if isinstance(data, dict) else {}


def bake_signature(map_dir: Path = MAP_DIR) -> dict:
    """Combined bake fingerprint of the pyramid, the fine rivers and the fine roads.

    The pyramid (tiers 1-3) already carries an authoritative bake version per
    tier (lot SZ2, :mod:`cent_ans_tools.geo.bake_stamp`): ``relief_pyramid.json``
    ``bake_versions``. The fine rivers and roads have no such version yet, only
    their own ``generated_at``. The combination is hashed (SHA-256 of the
    canonical JSON) into ``signature``, which changes whenever any of the three
    is rebaked (ADR 0077: "toute recuisson... incrémente la version").
    """
    pyramid = _read_json(map_dir / PYRAMID_MANIFEST)
    bake_versions = pyramid.get("bake_versions") or {}
    rivers_generated_at = _read_json(map_dir / RIVERS_MANIFEST).get("generated_at")
    roads_generated_at = _read_json(map_dir / ANCHORS_MANIFEST).get("generated_at")
    pyramid_bake_versions = {str(k): int(v) for k, v in sorted(bake_versions.items())}
    payload = {
        "pyramid_bake_versions": pyramid_bake_versions,
        "rivers_generated_at": rivers_generated_at,
        "roads_generated_at": roads_generated_at,
    }
    has_data = bool(pyramid_bake_versions) or rivers_generated_at or roads_generated_at
    signature = (
        hashlib.sha256(json.dumps(payload, sort_keys=True).encode("utf-8")).hexdigest()
        if has_data
        else None
    )
    return {**payload, "signature": signature}


def read_installed_marker(pyramid_dir: Path) -> dict:
    """``pyramid/package.json`` (empty when absent or unreadable)."""
    try:
        return _read_json(pyramid_dir / INSTALLED_MARKER)
    except (OSError, ValueError):
        return {}


def write_installed_marker(pyramid_dir: Path, hosting: dict) -> None:
    """Record in the cache the package version it holds (ADR 0149).

    Written by ``pack`` before archiving (so every package carries it), by
    ``fetch`` after installing, and when a cache without marker is adopted.
    ``tools/launch.sh`` reads its ``version`` line: keep one key per line.
    """
    marker = {
        "package_name": hosting.get("package_name"),
        "version": int(hosting.get("version", 1)),
        "signature": (hosting.get("bake") or {}).get("signature"),
    }
    partial = pyramid_dir / f".{INSTALLED_MARKER}.part"
    partial.write_text(json.dumps(marker, indent=2) + "\n", encoding="utf-8")
    partial.replace(pyramid_dir / INSTALLED_MARKER)


def load_hosting(hosting_file: Path = HOSTING_FILE) -> dict:
    """Read ``relief_hosting.json`` (created by SZ7's skeleton commit)."""
    if not hosting_file.exists():
        raise FileNotFoundError(
            f"{hosting_file} : absent (attendu, voir data/schemas/relief_hosting.schema.json)"
        )
    return _read_json(hosting_file)


def bump_version_if_rebaked(
    map_dir: Path = MAP_DIR, hosting_file: Path = HOSTING_FILE
) -> dict:
    """Increment ``version`` and update ``bake`` in-place if the cache changed.

    Writes the updated ``relief_hosting.json`` back to disk and returns it.
    """
    hosting = load_hosting(hosting_file)
    current = bake_signature(map_dir)
    previous_signature = (hosting.get("bake") or {}).get("signature")
    if current["signature"] is not None and current["signature"] != previous_signature:
        hosting["version"] = int(hosting.get("version", 1)) + (
            1 if previous_signature is not None else 0
        )
        hosting["bake"] = current
        hosting_file.write_text(
            json.dumps(hosting, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
    return hosting


def extract_credits(credits_file: Path = CREDITS_FILE) -> str:
    """Text of the "Données géographiques" section of ``CREDITS.md``."""
    if not credits_file.exists():
        return ""
    lines = credits_file.read_text(encoding="utf-8").splitlines()
    try:
        start = next(
            i for i, line in enumerate(lines) if line.strip() == CREDITS_HEADING
        )
    except StopIteration:
        return ""
    end = len(lines)
    for i in range(start + 1, len(lines)):
        if lines[i].startswith("## "):
            end = i
            break
    return "\n".join(lines[start:end]).strip()


def check_free_space(target_dir: Path, estimated_bytes: int) -> None:
    """Raise ``OSError`` if ``target_dir``'s filesystem lacks room for the pack."""
    target_dir.mkdir(parents=True, exist_ok=True)
    needed = int(estimated_bytes * DISK_MARGIN)
    free = shutil.disk_usage(target_dir).free
    if free < needed:
        raise OSError(
            f"Espace disque insuffisant dans {target_dir} : {free / 1e9:.2f} Go libres, "
            f"{needed / 1e9:.2f} Go nécessaires (paquet {estimated_bytes / 1e9:.2f} Go × "
            f"{DISK_MARGIN} de marge)."
        )


def _tree_bytes(path: Path) -> int:
    return sum(p.stat().st_size for p in path.rglob("*") if p.is_file())


class _SplitWriter:
    """File-like sink for :mod:`tarfile`'s stream mode, rolling to new parts.

    Splits the raw tar byte stream on :data:`MAX_PART_BYTES` boundaries,
    independent of tar member boundaries (like ``split(1)``): reassembly is
    just concatenating the parts back, in order, before feeding them to a
    streaming tar reader.

    The threshold is only checked between the writes :mod:`tarfile` itself
    issues (its stream mode writes fixed ``RECORDSIZE`` blocks, 10 KiB by
    default), so a part can exceed ``max_bytes`` by up to one such block.
    Negligible at the real ~1.9 GiB scale.
    """

    def __init__(
        self, out_dir: Path, package: str, version: int, max_bytes: int
    ) -> None:
        self._out_dir = out_dir
        self._package = package
        self._version = version
        self._max_bytes = max_bytes
        self._index = 0
        self._current_size = 0
        self._current_file = None
        self.parts: list[Path] = []
        self.part_sha256: list[str] = []
        self._part_digest = hashlib.sha256()
        self.global_sha256 = hashlib.sha256()
        self.total_bytes = 0
        self._open_next()

    def _part_path(self) -> Path:
        name = PART_NAME.format(
            package=self._package, version=self._version, index=self._index
        )
        return self._out_dir / name

    def _open_next(self) -> None:
        if self._current_file is not None:
            self._current_file.close()
            self.part_sha256.append(self._part_digest.hexdigest())
        path = self._part_path()
        self._current_file = path.open("wb")
        self.parts.append(path)
        self._current_size = 0
        self._part_digest = hashlib.sha256()
        self._index += 1

    def write(self, data: bytes) -> int:
        if self._current_size >= self._max_bytes:
            self._open_next()
        self._current_file.write(data)
        self._part_digest.update(data)
        self.global_sha256.update(data)
        self._current_size += len(data)
        self.total_bytes += len(data)
        return len(data)

    def close(self) -> None:
        if self._current_file is not None:
            self._current_file.close()
            self.part_sha256.append(self._part_digest.hexdigest())
            self._current_file = None


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
    pyramid_dir = pyramid_dir or (map_dir / "pyramid")
    if not pyramid_dir.is_dir():
        raise FileNotFoundError(
            f"{pyramid_dir} : cache de relief absent, rien à empaqueter"
        )
    out_dir = Path(out_dir)
    estimated = _tree_bytes(pyramid_dir)
    check_free_space(out_dir, estimated)

    hosting = bump_version_if_rebaked(map_dir, hosting_file)
    version = int(hosting["version"])
    package = str(hosting["package_name"])
    write_installed_marker(pyramid_dir, hosting)

    writer = _SplitWriter(out_dir, package, version, max_part_bytes)
    with tarfile.open(fileobj=writer, mode="w|") as tar:
        tar.add(pyramid_dir, arcname="pyramid")
    writer.close()

    parts_info = [
        {"name": path.name, "bytes": path.stat().st_size, "sha256": sha}
        for path, sha in zip(writer.parts, writer.part_sha256, strict=True)
    ]
    manifest = {
        "description": "Manifeste du paquet « Cent Ans relief » (lot SZ7, ADR 0077). "
        "Concaténer les parts dans l'ordre reconstitue le flux tar (data/map/pyramid/).",
        "package_name": package,
        "version": version,
        "generated_at": datetime.now(UTC).isoformat(timespec="seconds"),
        "total_bytes": writer.total_bytes,
        "sha256": writer.global_sha256.hexdigest(),
        "bake": hosting.get("bake", {}),
        "parts": parts_info,
        "credits": extract_credits(credits_file),
    }
    manifest_path = out_dir / MANIFEST_NAME
    manifest_path.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return PackResult(
        out_dir=out_dir,
        manifest_path=manifest_path,
        parts=writer.parts,
        total_bytes=writer.total_bytes,
        version=version,
    )
