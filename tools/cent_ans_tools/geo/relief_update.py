"""Keep the hosted fine relief package in step with the code (ADR 0149).

One command, ``cent-ans geo relief-update``, chains what used to be a list of
manual gestures, each step skipped when it has nothing to do:

1. the manifest's ``bake_versions`` follow the bake versions of the code
   (:func:`code_bake_versions`), so that a bumped ``BAKE_VERSION`` makes the cache
   stale without anyone editing ``relief_pyramid.json``;
2. the stale or missing steps are rebaked (:mod:`relief_cache`), then the data
   derived from the fine relief (towns, landmark cities) is regenerated;
3. when the bake signature differs from the published package, the cache is
   packed (:mod:`relief_pack`, which bumps the package version) and published as
   a GitHub Release of the data repository named by ``relief_hosting.json``.

The player side is :func:`relief_fetch.needs_fetch`, called by ``tools/launch.sh``.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import urllib.request
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from urllib.error import HTTPError, URLError

from cent_ans_tools.geo import relief_cache, relief_pack

MAP_DIR = relief_pack.MAP_DIR
HOSTING_FILE = relief_pack.HOSTING_FILE
DEFAULT_OUT_DIR = relief_pack.REPO_DIR / "dist" / "relief"
RELEASE_NOTES = (
    "Cache de relief fin (pyramide + fleuves et routes fins). Installé par "
    "tools/launch.sh, ou : uv run --project tools cent-ans geo relief-fetch"
)
_GITHUB_RELEASES = re.compile(r"github\.com/([^/]+/[^/]+)/releases/download/")

Log = Callable[[str], None]
#: ``(command) -> exit code`` (``gh``), replaced in tests.
Runner = Callable[[list[str]], int]


@dataclass
class UpdateResult:
    """What ``update`` did."""

    synced_versions: bool = False
    rebaked: list[str] = field(default_factory=list)
    packed: bool = False
    published: bool = False
    version: int = 0

    @property
    def changed(self) -> bool:
        """Something was written (tracked data files are then to commit)."""
        return self.synced_versions or bool(self.rebaked) or self.packed


def code_bake_versions() -> dict[str, int]:
    """Bake version of each pyramid tier, as the code would bake it now."""
    from cent_ans_tools.geo import detail_dem, pyramid

    return {**pyramid.TIER_VERSIONS, "tier3": detail_dem.BAKE_VERSION}


def sync_manifest_versions(map_dir: Path = MAP_DIR) -> bool:
    """Write the code's bake versions into ``relief_pyramid.json``; ``True`` if changed."""
    from cent_ans_tools.geo import pyramid

    path = map_dir / relief_pack.PYRAMID_MANIFEST
    before = path.read_text(encoding="utf-8")
    pyramid.set_manifest_bake_versions(map_dir, code_bake_versions())
    return path.read_text(encoding="utf-8") != before


def release_repo(hosting: dict) -> str:
    """``owner/name`` of the GitHub repository behind ``base_url``."""
    match = _GITHUB_RELEASES.search(str(hosting.get("base_url", "")))
    if match is None:
        raise ValueError(
            "relief_hosting.json : base_url n'est pas une Release GitHub, "
            "publication automatique impossible (relancer avec --no-publish)"
        )
    return match.group(1)


def published_signature(hosting: dict) -> str | None:
    """Bake signature of the published package (``None``: version not published)."""
    base = str(hosting["base_url"]).format(version=hosting["version"])
    url = base.rstrip("/") + "/" + relief_pack.MANIFEST_NAME
    try:
        with urllib.request.urlopen(url, timeout=30) as response:  # noqa: S310
            manifest = json.loads(response.read())
    except HTTPError as error:
        if error.code == 404:
            return None
        raise
    except URLError as error:
        raise OSError(f"{url} injoignable : {error.reason}") from error
    return (manifest.get("bake") or {}).get("signature")


def _run(command: list[str]) -> int:
    return subprocess.run(command, check=False).returncode  # noqa: S603


def publish(
    result: relief_pack.PackResult, hosting: dict, run: Runner = _run, log: Log = print
) -> None:
    """Create the Release ``v<version>`` (or complete it) with the parts and the manifest.

    The manifest goes last: it is what :func:`published_signature` and
    ``relief-fetch`` read first, so a Release interrupted mid-upload stays
    "not published" and the next run completes it.
    """
    repo = release_repo(hosting)
    tag = f"v{result.version}"
    files = [str(path) for path in result.parts]
    if run(["gh", "release", "view", tag, "--repo", repo, "--json", "tagName"]) != 0:
        log(f"Création de la Release {tag} sur {repo}…")
        create = ["gh", "release", "create", tag, "--repo", repo]
        create += ["--title", f"Cent Ans relief {tag}", "--notes", RELEASE_NOTES]
        if run(create) != 0:
            raise RuntimeError(f"gh release create {tag} a échoué (voir ci-dessus)")
    upload = ["gh", "release", "upload", tag, "--repo", repo, "--clobber"]
    for path in [*files, str(result.manifest_path)]:
        log(f"Envoi de {Path(path).name}…")
        if run([*upload, path]) != 0:
            raise RuntimeError(f"gh release upload {Path(path).name} a échoué")


def _regenerate_derived(log: Log) -> None:
    """Data read from the fine relief: town footprints, landmark city waters."""
    from cent_ans_tools.geo import landmarks_v2, towns

    log(towns.build(log=log).summary())
    log(landmarks_v2.build(log=log).summary())


def update(
    map_dir: Path = MAP_DIR,
    hosting_file: Path = HOSTING_FILE,
    out_dir: Path = DEFAULT_OUT_DIR,
    do_publish: bool = True,
    keep_parts: bool = False,
    workers: int | None = None,
    log: Log = print,
    rebuild: Callable[..., relief_cache.RebuildResult] = relief_cache.rebuild,
    regenerate_derived: Callable[[Log], None] = _regenerate_derived,
    read_published: Callable[[dict], str | None] = published_signature,
    run: Runner = _run,
) -> UpdateResult:
    """Rebake what the code made stale, then pack and publish if the bake changed.

    Args:
        map_dir: ``data/map``.
        hosting_file: ``relief_hosting.json`` (version bumped by the pack).
        out_dir: Where the parts are written before the upload.
        do_publish: Upload the package (``False``: stop after packing).
        keep_parts: Keep ``out_dir`` after a successful upload.
        workers: Processes of the bake steps (``None``: each step's default).
        log: Progress lines (French).
        rebuild: Bake runner (tests).
        regenerate_derived: Derived data runner (tests).
        read_published: Published signature reader (tests).
        run: ``gh`` runner (tests).
    """
    result = UpdateResult()
    result.synced_versions = sync_manifest_versions(map_dir)
    if result.synced_versions:
        log("Versions de cuisson du manifeste alignées sur le code.")

    report = relief_cache.check(map_dir)
    if report.plan():
        rebuilt = rebuild(map_dir=map_dir, workers=workers, log=log)
        if not rebuilt.report.complete:
            raise RuntimeError(
                "Cache de relief encore incomplet après recuisson : relancer la "
                "commande, elle reprend où elle s'est arrêtée."
            )
        result.rebaked = rebuilt.ran
        regenerate_derived(log)
    else:
        log("Cache de relief à jour avec le code : rien à recuire.")

    signature = relief_pack.bake_signature(map_dir)["signature"]
    hosting = relief_pack.load_hosting(hosting_file)
    result.version = int(hosting.get("version", 1))
    same_bake = (hosting.get("bake") or {}).get("signature") == signature
    if same_bake and (not do_publish or read_published(hosting) == signature):
        log(f"Paquet v{result.version} à jour : rien à publier.")
        return result

    packed = relief_pack.pack(out_dir, map_dir=map_dir, hosting_file=hosting_file)
    result.packed = True
    result.version = packed.version
    log(
        f"Paquet v{packed.version} : {len(packed.parts)} part(s), "
        f"{packed.total_bytes / 1e9:.2f} Go → {packed.out_dir}"
    )
    if not do_publish:
        return result

    hosting = relief_pack.load_hosting(hosting_file)
    publish(packed, hosting, run=run, log=log)
    if read_published(hosting) != signature:
        raise RuntimeError(
            f"Release v{packed.version} envoyée mais son manifeste ne correspond pas "
            "au cache : relancer la commande."
        )
    result.published = True
    log(f"Paquet v{packed.version} publié.")
    if not keep_parts:
        shutil.rmtree(out_dir, ignore_errors=True)
    return result
