"""Automatic update of the relief package (ADR 0149): staleness, fetch-if-needed, publish.

No network and no ``gh``: the published signature reader and the command runner
are injected.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from cent_ans_tools.geo import (
    bake_stamp,
    relief_cache,
    relief_fetch,
    relief_pack,
    relief_update,
)
from tests.test_relief_hosting import (
    _write_fake_manifests,
    _write_fake_pyramid,
    _write_hosting,
)

GITHUB_URL = "https://github.com/someone/some-relief/releases/download/v{version}/"


def _hosting(path: Path, version: int = 1, tiers: dict | None = None) -> dict:
    hosting = {
        "package_name": "cent-ans-relief",
        "base_url": GITHUB_URL,
        "version": version,
        "bake": {"pyramid_bake_versions": tiers or {}, "signature": "old"},
    }
    path.write_text(json.dumps(hosting, indent=2))
    return hosting


# ------------------------------------------------------------------- staleness


def test_manifest_bake_versions_follow_the_code() -> None:
    """A bumped ``BAKE_VERSION`` must reach ``relief_pyramid.json`` (else ``--check`` is blind)."""
    manifest = json.loads(
        (relief_pack.MAP_DIR / relief_pack.PYRAMID_MANIFEST).read_text()
    )
    assert manifest["bake_versions"] == relief_update.code_bake_versions(), (
        "lancer : uv run --project tools cent-ans geo relief-update"
    )


# ---------------------------------------------------------------- needs_fetch


def test_needs_fetch_when_the_cache_is_absent(tmp_path: Path) -> None:
    """No ``pyramid/`` at all: download."""
    hosting_file = tmp_path / "relief_hosting.json"
    _hosting(hosting_file, version=3)
    needed, _ = relief_fetch.needs_fetch(tmp_path / "map", hosting_file)
    assert needed


def test_needs_fetch_compares_the_installed_marker(tmp_path: Path) -> None:
    """The marker's version against the hosted one."""
    hosting_file = tmp_path / "relief_hosting.json"
    pyramid = _write_fake_pyramid(tmp_path / "map")
    relief_pack.write_installed_marker(pyramid, _hosting(hosting_file, version=2))
    assert relief_fetch.needs_fetch(tmp_path / "map", hosting_file)[0] is False
    _hosting(hosting_file, version=3)
    assert relief_fetch.needs_fetch(tmp_path / "map", hosting_file)[0] is True


def test_needs_fetch_without_marker_judges_on_bake_stamps(tmp_path: Path) -> None:
    """A pre-ADR 0149 cache with an older tier is behind, and not adopted."""
    hosting_file = tmp_path / "relief_hosting.json"
    _hosting(hosting_file, version=3, tiers={"tier1": 3, "tier3": 6})
    pyramid = _write_fake_pyramid(tmp_path / "map")
    bake_stamp.write(pyramid, "tier1", bake_stamp.TierStamp(3, 0.0, True))
    bake_stamp.write(pyramid, "tier3", bake_stamp.TierStamp(5, 0.0, True))
    assert relief_fetch.needs_fetch(tmp_path / "map", hosting_file)[0] is True
    assert not (pyramid / relief_pack.INSTALLED_MARKER).exists()


def test_needs_fetch_adopts_a_cache_baked_ahead_of_the_package(tmp_path: Path) -> None:
    """A local rebake (even unfinished) newer than the package is never overwritten."""
    hosting_file = tmp_path / "relief_hosting.json"
    _hosting(hosting_file, version=2, tiers={"tier1": 3, "tier3": 5})
    pyramid = _write_fake_pyramid(tmp_path / "map")
    bake_stamp.write(pyramid, "tier1", bake_stamp.TierStamp(3, 0.0, True))
    bake_stamp.write(pyramid, "tier3", bake_stamp.TierStamp(6, 0.0, False))
    assert relief_fetch.needs_fetch(tmp_path / "map", hosting_file)[0] is False
    assert relief_pack.read_installed_marker(pyramid)["version"] == 2


def test_pack_marks_the_cache_and_fetch_installs_the_mark(tmp_path: Path) -> None:
    """The marker travels in the package and survives the install."""
    map_dir = tmp_path / "map"
    _write_fake_manifests(map_dir)
    pyramid = _write_fake_pyramid(map_dir)
    hosting_file = tmp_path / "relief_hosting.json"
    _write_hosting(hosting_file, version=4)
    relief_pack.pack(tmp_path / "pack", map_dir=map_dir, hosting_file=hosting_file)
    assert relief_pack.read_installed_marker(pyramid)["version"] == 4

    result = relief_fetch.fetch(
        dest=tmp_path / "install", from_dir=tmp_path / "pack", log=lambda _: None
    )
    marker = relief_pack.read_installed_marker(result.pyramid_dir)
    assert marker["version"] == 4
    assert marker["signature"] == relief_pack.bake_signature(map_dir)["signature"]
    assert relief_fetch.needs_fetch(tmp_path / "install", hosting_file)[0] is False


def test_discard_partial_download(tmp_path: Path) -> None:
    """Leftover parts of an interrupted download are removed once."""
    hosting_file = tmp_path / "relief_hosting.json"
    _hosting(hosting_file)
    staging = tmp_path / "map" / ".pyramid.download-cent-ans-relief"
    staging.mkdir(parents=True)
    (staging / "cent-ans-relief-v1.part000.tar").write_bytes(b"x")
    assert relief_fetch.discard_partial_download(tmp_path / "map", hosting_file)
    assert not staging.exists()
    assert not relief_fetch.discard_partial_download(tmp_path / "map", hosting_file)


# --------------------------------------------------------------------- update


class _Fixture:
    """A complete fake cache whose bake differs from the hosted package."""

    def __init__(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        self.map_dir = tmp_path / "map"
        _write_fake_manifests(self.map_dir)
        _write_fake_pyramid(self.map_dir)
        self.hosting_file = tmp_path / "relief_hosting.json"
        _write_hosting(self.hosting_file, version=2)
        hosting = json.loads(self.hosting_file.read_text())
        hosting["base_url"] = GITHUB_URL
        hosting["bake"]["signature"] = "old"
        self.hosting_file.write_text(json.dumps(hosting))
        self.out_dir = tmp_path / "dist"
        self.commands: list[list[str]] = []
        self.published: str | None = "old"
        self.stale = False
        self.derived = 0
        monkeypatch.setattr(relief_update, "sync_manifest_versions", lambda _: False)
        monkeypatch.setattr(relief_cache, "check", lambda _map_dir: self)

    # Stand-in for the CacheReport.
    complete = True

    def plan(self) -> list[str]:
        return ["tier3", "hydro", "anchors"] if self.stale else []

    def _rebuild(self, **_: object) -> relief_cache.RebuildResult:
        self.stale = False
        return relief_cache.RebuildResult(
            ran=["tier3", "hydro", "anchors"], report=self
        )

    def _derived(self, _log: object) -> None:
        self.derived += 1

    def _run(self, command: list[str]) -> int:
        self.commands.append(command)
        if command[:3] == ["gh", "release", "view"]:
            return 1  # no such Release yet
        if command[-1].endswith("manifest.json"):
            manifest = json.loads(Path(command[-1]).read_text())
            self.published = manifest["bake"]["signature"]
        return 0

    def update(self, **kwargs: object) -> relief_update.UpdateResult:
        return relief_update.update(
            map_dir=self.map_dir,
            hosting_file=self.hosting_file,
            out_dir=self.out_dir,
            log=lambda _: None,
            rebuild=self._rebuild,
            regenerate_derived=self._derived,
            read_published=lambda _hosting: self.published,
            run=self._run,
            **kwargs,
        )


def test_update_packs_and_publishes_a_changed_bake(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """New signature: version bump, Release created, manifest last; then idempotent."""
    fixture = _Fixture(tmp_path, monkeypatch)
    result = fixture.update()

    assert (result.packed, result.published, result.version) == (True, True, 3)
    assert result.rebaked == [] and fixture.derived == 0
    assert json.loads(fixture.hosting_file.read_text())["version"] == 3
    subcommands = [command[2] for command in fixture.commands]
    assert subcommands[:2] == ["view", "create"]
    assert fixture.commands[1][3:6] == ["v3", "--repo", "someone/some-relief"]
    uploads = [command[-1] for command in fixture.commands if command[2] == "upload"]
    assert uploads[-1].endswith("manifest.json"), "the manifest goes last"
    assert all(name.endswith(".tar") for name in uploads[:-1]) and len(uploads) >= 2
    assert not fixture.out_dir.exists(), "parts removed once published"

    again = fixture.update()
    assert (again.packed, again.published, again.changed) == (False, False, False)


def test_update_rebakes_stale_steps_then_regenerates_derived_data(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Stale tiers are rebaked, towns and landmarks follow."""
    fixture = _Fixture(tmp_path, monkeypatch)
    fixture.stale = True
    result = fixture.update()
    assert result.rebaked == ["tier3", "hydro", "anchors"]
    assert fixture.derived == 1 and result.published


def test_update_without_publish_stops_after_the_pack(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """``--no-publish`` never calls ``gh`` and keeps the parts."""
    fixture = _Fixture(tmp_path, monkeypatch)
    result = fixture.update(do_publish=False)
    assert result.packed and not result.published
    assert fixture.commands == []
    assert (fixture.out_dir / "manifest.json").exists()


def test_update_completes_an_interrupted_publication(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Hosting already bumped but the Release has no manifest: same version, re-sent."""
    fixture = _Fixture(tmp_path, monkeypatch)
    fixture.update(do_publish=False)
    fixture.published = None
    result = fixture.update()
    assert (result.packed, result.published, result.version) == (True, True, 3)


def test_update_fails_when_an_upload_fails(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A failed upload raises and keeps the parts."""
    fixture = _Fixture(tmp_path, monkeypatch)
    monkeypatch.setattr(fixture, "_run", lambda command: int(command[2] == "upload"))
    with pytest.raises(RuntimeError, match="upload"):
        fixture.update()
    assert fixture.out_dir.exists(), "parts kept for the next run"


def test_release_repo_requires_a_github_release_url() -> None:
    """``owner/name`` is read from ``base_url``."""
    assert relief_update.release_repo({"base_url": GITHUB_URL}) == "someone/some-relief"
    with pytest.raises(ValueError, match="base_url"):
        relief_update.release_repo({"base_url": "https://example.org/relief/"})
