"""Hosted package of the generated 3D models (``game/assets/models/dn``, ADR 0212).

The glb files produced by ``cent-ans dn-ingest`` (about 890 MB) are not versioned in
git: they travel as a package in the GitHub Releases ``models-v<N>`` of the data
repository, exactly like the fine relief (ADR 0077, 0149). This module reuses the
relief machinery (split tar parts, SHA-256 manifest, ranged download, atomic swap,
``gh`` publication) and only adds what is specific to a models tree:

- the content signature (SHA-256 over the relative path and SHA-256 of every file);
- ``dn/package.json``, the installation mark read by ``tools/launch.sh``;
- the tracked ``data/art/dn_models_hosting.json`` (version, signature, URL).

``data/art/dn_manifest.json`` stays tracked by git: it lists what the package holds.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import tarfile
import urllib.request
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

from cent_ans_tools.geo import relief_fetch, relief_pack, relief_update

REPO_DIR = relief_pack.REPO_DIR
MODELS_PARENT = REPO_DIR / "game" / "assets" / "models"
MODELS_DIR = MODELS_PARENT / "dn"
HOSTING_FILE = REPO_DIR / "data" / "art" / "dn_models_hosting.json"
DEFAULT_OUT_DIR = REPO_DIR / "dist" / "models"
ROOT_NAME = "dn"
INSTALLED_MARKER = "package.json"
DEST_ENV_VAR = "CENT_ANS_MODELS_DIR"
TAG_PREFIX = "models-v"
RELEASE_TITLE = "Cent Ans modèles"
RELEASE_NOTES = (
    "Modèles 3D générés (glb LOD0-2 et textures, game/assets/models/dn). Installés par "
    "tools/launch.sh, ou : uv run --project tools cent-ans art models-fetch"
)
#: Godot writes these next to the assets at import (``.import``, ``.uid``, and the textures it
#: extracts from the glb as ``<name>_Image_<n>.jpg``); they are not part of the package.
_SKIPPED_SUFFIXES = (".import", ".uid")
_IMPORT_TEXTURE = re.compile(r"_Image_\d+\.(jpg|png)$")


def _is_import_artifact(name: str) -> bool:
    return name.endswith(_SKIPPED_SUFFIXES) or _IMPORT_TEXTURE.search(name) is not None


_CHUNK = 1024 * 1024

Log = Callable[[str], None]


@dataclass
class UpdateResult:
    """What ``update`` did."""

    packed: bool = False
    published: bool = False
    version: int = 0
    files: int = 0

    @property
    def changed(self) -> bool:
        """The tracked hosting file was rewritten (to commit)."""
        return self.packed


def package_files(models_dir: Path = MODELS_DIR) -> list[Path]:
    """Files that make up the package, sorted (marker and Godot import files excluded)."""
    return sorted(
        path
        for path in models_dir.rglob("*")
        if path.is_file()
        and path.name != INSTALLED_MARKER
        and not _is_import_artifact(path.name)
        and not path.name.startswith(".")
    )


def content_signature(models_dir: Path = MODELS_DIR) -> dict:
    """Fingerprint of the tree: file count, bytes and a SHA-256 over path + file hash."""
    digest = hashlib.sha256()
    files = package_files(models_dir)
    total = 0
    for path in files:
        file_digest = hashlib.sha256()
        with path.open("rb") as handle:
            while chunk := handle.read(_CHUNK):
                file_digest.update(chunk)
        total += path.stat().st_size
        relative = path.relative_to(models_dir).as_posix()
        digest.update(f"{relative}\0{file_digest.hexdigest()}\n".encode())
    return {
        "files": len(files),
        "bytes": total,
        "signature": digest.hexdigest() if files else None,
    }


def load_hosting(hosting_file: Path = HOSTING_FILE) -> dict:
    """Read ``dn_models_hosting.json``."""
    return relief_pack.load_hosting(hosting_file)


def write_installed_marker(models_dir: Path, hosting: dict) -> None:
    """Record in the tree which package version it holds (one key per line, for launch.sh)."""
    marker = {
        "package_name": hosting.get("package_name"),
        "version": int(hosting.get("version", 1)),
        "signature": (hosting.get("content") or {}).get("signature"),
    }
    partial = models_dir / f".{INSTALLED_MARKER}.part"
    partial.write_text(json.dumps(marker, indent=2) + "\n", encoding="utf-8")
    partial.replace(models_dir / INSTALLED_MARKER)


def read_installed_marker(models_dir: Path) -> dict:
    """``dn/package.json`` (empty when absent or unreadable)."""
    try:
        return relief_pack.read_installed_marker(models_dir)
    except (OSError, ValueError):
        return {}


def default_dest() -> Path:
    """Folder holding ``dn/``: ``CENT_ANS_MODELS_DIR`` or ``game/assets/models``."""
    from_env = os.environ.get(DEST_ENV_VAR, "").strip()
    return Path(from_env) if from_env else MODELS_PARENT


def bump_version_if_changed(
    models_dir: Path = MODELS_DIR, hosting_file: Path = HOSTING_FILE
) -> dict:
    """Increment ``version`` and refresh ``content`` when the tree changed; return hosting."""
    hosting = load_hosting(hosting_file)
    current = content_signature(models_dir)
    previous = (hosting.get("content") or {}).get("signature")
    if current["signature"] is not None and current["signature"] != previous:
        hosting["version"] = int(hosting.get("version", 1)) + (
            1 if previous is not None else 0
        )
        hosting["content"] = current
        hosting_file.write_text(
            json.dumps(hosting, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
    return hosting


def _skip_unwanted(info: tarfile.TarInfo) -> tarfile.TarInfo | None:
    name = Path(info.name).name
    if _is_import_artifact(name) or (name.startswith(".") and name != INSTALLED_MARKER):
        return None
    return info


def pack(
    out_dir: Path,
    models_dir: Path = MODELS_DIR,
    hosting_file: Path = HOSTING_FILE,
    max_part_bytes: int = relief_pack.MAX_PART_BYTES,
) -> relief_pack.PackResult:
    """Archive ``models_dir`` into split tar parts plus a manifest (nothing is uploaded)."""
    if not models_dir.is_dir() or not package_files(models_dir):
        raise FileNotFoundError(f"{models_dir} : aucun modèle à empaqueter")
    out_dir = Path(out_dir)
    relief_pack.check_free_space(
        out_dir, sum(p.stat().st_size for p in package_files(models_dir))
    )
    hosting = bump_version_if_changed(models_dir, hosting_file)
    write_installed_marker(models_dir, hosting)
    return relief_pack.archive_tree(
        models_dir,
        ROOT_NAME,
        out_dir,
        str(hosting["package_name"]),
        int(hosting["version"]),
        description="Manifeste du paquet « Cent Ans modèles » (ADR 0212). Concaténer "
        "les parts dans l'ordre reconstitue le flux tar (game/assets/models/dn/).",
        extra={"content": hosting.get("content", {})},
        max_part_bytes=max_part_bytes,
        tar_filter=_skip_unwanted,
    )


def needs_fetch(
    dest: Path | None = None, hosting_file: Path = HOSTING_FILE
) -> tuple[bool, str]:
    """Whether the installed models are older than the hosted package.

    A tree without mark but holding models (ingested here, or checked out from git
    before ADR 0212) is adopted, never overwritten: its mark is written.
    """
    dest_dir = Path(dest) if dest is not None else default_dest()
    hosting = load_hosting(hosting_file)
    version = int(hosting.get("version", 1))
    models_dir = dest_dir / ROOT_NAME
    if not models_dir.is_dir() or not package_files(models_dir):
        return True, "modèles générés absents"
    marker = read_installed_marker(models_dir)
    if "version" in marker:
        installed = int(marker["version"])
        if installed < version:
            return True, f"paquet v{installed} installé, v{version} publié"
        return False, f"paquet v{installed} installé"
    write_installed_marker(models_dir, hosting)
    return False, f"modèles sans marque, adoptés comme paquet v{version}"


def discard_partial_download(
    dest: Path | None = None, hosting_file: Path = HOSTING_FILE
) -> bool:
    """Remove the leftover parts of an interrupted download."""
    dest_dir = Path(dest) if dest is not None else default_dest()
    staging = (
        dest_dir / f".{ROOT_NAME}.download-{load_hosting(hosting_file)['package_name']}"
    )
    if not staging.is_dir():
        return False
    shutil.rmtree(staging, ignore_errors=True)
    return True


@dataclass
class FetchResult:
    """Where the models were installed."""

    dest_dir: Path
    models_dir: Path
    total_bytes: int
    parts: int = 0
    files: list[str] = field(default_factory=list)


def fetch(
    dest: Path | None = None,
    base_url: str | None = None,
    from_dir: Path | None = None,
    hosting_file: Path = HOSTING_FILE,
    log: Log = print,
) -> FetchResult:
    """Install the models into ``dest/dn`` from ``base_url`` or ``from_dir``."""
    dest_dir = Path(dest) if dest is not None else default_dest()
    if from_dir is not None:
        from_dir = Path(from_dir)
        manifest = relief_fetch._read_manifest(from_dir / relief_pack.MANIFEST_NAME)  # noqa: SLF001
        relief_fetch.verify_parts(from_dir, manifest)
        parts = [from_dir / entry["name"] for entry in manifest["parts"]]
    else:
        hosting = load_hosting(hosting_file)
        base = (base_url or hosting["base_url"]).format(version=hosting["version"])
        if not base.endswith("/"):
            base += "/"
        staging = dest_dir / f".{ROOT_NAME}.download-{hosting['package_name']}"
        staging.mkdir(parents=True, exist_ok=True)
        manifest_path = staging / relief_pack.MANIFEST_NAME
        with urllib.request.urlopen(base + relief_pack.MANIFEST_NAME) as response:  # noqa: S310
            manifest_path.write_bytes(response.read())
        manifest = relief_fetch._read_manifest(manifest_path)  # noqa: SLF001
        for entry in manifest["parts"]:
            relief_fetch.download_part(
                base + entry["name"], staging / entry["name"], entry["sha256"], log
            )
        relief_fetch.verify_parts(staging, manifest)
        parts = [staging / entry["name"] for entry in manifest["parts"]]

    models_dir = relief_fetch.extract_atomic(
        parts, dest_dir, log, root=ROOT_NAME, label="Modèles"
    )
    write_installed_marker(
        models_dir,
        {
            "package_name": manifest.get("package_name"),
            "version": manifest.get("version", 1),
            "content": manifest.get("content"),
        },
    )
    if from_dir is None:
        shutil.rmtree(parts[0].parent, ignore_errors=True)
    return FetchResult(
        dest_dir=dest_dir,
        models_dir=models_dir,
        total_bytes=manifest.get("total_bytes", 0),
        parts=len(parts),
    )


def update(
    models_dir: Path = MODELS_DIR,
    hosting_file: Path = HOSTING_FILE,
    out_dir: Path = DEFAULT_OUT_DIR,
    do_publish: bool = True,
    keep_parts: bool = False,
    log: Log = print,
    read_published: Callable[[dict], str | None] | None = None,
    run: relief_update.Runner = relief_update._run,  # noqa: SLF001
) -> UpdateResult:
    """Pack (and publish) the models when the tree differs from the published package."""
    if read_published is None:

        def read_published(hosting: dict) -> str | None:
            return relief_update.published_signature(hosting, section="content")

    result = UpdateResult()
    current = content_signature(models_dir)
    result.files = current["files"]
    hosting = load_hosting(hosting_file)
    result.version = int(hosting.get("version", 1))
    signature = current["signature"]
    if signature is None:
        raise FileNotFoundError(f"{models_dir} : aucun modèle à empaqueter")
    same = (hosting.get("content") or {}).get("signature") == signature
    if same and (not do_publish or read_published(hosting) == signature):
        log(f"Paquet v{result.version} à jour : rien à publier.")
        return result

    packed = pack(out_dir, models_dir=models_dir, hosting_file=hosting_file)
    result.packed = True
    result.version = packed.version
    log(
        f"Paquet v{packed.version} : {len(packed.parts)} part(s), "
        f"{packed.total_bytes / 1e6:.0f} Mo → {packed.out_dir}"
    )
    if not do_publish:
        return result

    hosting = load_hosting(hosting_file)
    relief_update.publish(
        packed,
        hosting,
        run=run,
        log=log,
        tag_prefix=TAG_PREFIX,
        title=RELEASE_TITLE,
        notes=RELEASE_NOTES,
    )
    if read_published(hosting) != signature:
        raise RuntimeError(
            f"Release {TAG_PREFIX}{packed.version} envoyée mais son manifeste ne "
            "correspond pas aux modèles : relancer la commande."
        )
    result.published = True
    log(f"Paquet v{packed.version} publié.")
    if not keep_parts:
        shutil.rmtree(out_dir, ignore_errors=True)
    return result
