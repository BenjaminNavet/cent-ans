"""Generated models package (ADR 0212): pack, fetch, staleness, update; no network, no ``gh``."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from cent_ans_tools import models_package
from cent_ans_tools.geo import relief_pack
from tests.conftest import DATA, assert_matches_schema
from tests.test_relief_hosting import http_server  # noqa: F401  (fixture)

GITHUB_URL = "https://github.com/someone/some-data/releases/download/models-v{version}/"


def _tree(root: Path, payload: bytes = b"g") -> Path:
    models = root / "dn"
    (models / "buildings").mkdir(parents=True)
    (models / "buildings" / "a_lod0.glb").write_bytes(payload * 5000)
    (models / "buildings" / "a_lod0_Image_0.jpg").write_bytes(
        b"j" * 3000
    )  # Godot extract
    (models / "buildings" / "a_tex.jpg").write_bytes(b"t" * 3000)
    (models / "buildings" / "a_lod0.glb.import").write_text("godot")  # never packed
    return models


def _hosting(path: Path, version: int = 1, base_url: str = GITHUB_URL) -> Path:
    path.write_text(
        json.dumps(
            {
                "package_name": "cent-ans-models",
                "base_url": base_url,
                "version": version,
                "manifest_name": "manifest.json",
            }
        )
    )
    return path


def _pack(tmp_path: Path, payload: bytes = b"g") -> tuple[Path, Path, Path]:
    models = _tree(tmp_path / "src", payload)
    hosting = _hosting(tmp_path / "hosting.json")
    out = tmp_path / "out"
    models_package.pack(
        out, models_dir=models, hosting_file=hosting, max_part_bytes=4096
    )
    return models, hosting, out


def test_shipped_hosting_file_matches_its_schema() -> None:
    """The tracked hosting file validates."""
    assert_matches_schema(
        DATA / "art" / "dn_models_hosting.json", "art_dn_models_hosting.schema.json"
    )


def test_pack_splits_and_skips_godot_files(tmp_path: Path) -> None:
    """Parts are split, the manifest carries the content signature, .import is left out."""
    models, hosting, out = _pack(tmp_path)
    manifest = json.loads((out / "manifest.json").read_text())
    assert len(manifest["parts"]) >= 2
    assert manifest["content"]["files"] == 2  # glb + real jpg; import artefacts skipped
    assert (
        manifest["content"]["signature"]
        == models_package.content_signature(models)["signature"]
    )
    assert json.loads(hosting.read_text())["version"] == 1
    assert (models / "package.json").exists()


def test_fetch_from_dir_installs_without_import_files(tmp_path: Path) -> None:
    """A local fetch rebuilds the tree and writes the installation mark."""
    models, hosting, out = _pack(tmp_path)
    dest = tmp_path / "dest"
    result = models_package.fetch(
        dest=dest, from_dir=out, hosting_file=hosting, log=lambda *_: None
    )
    assert (dest / "dn" / "buildings" / "a_lod0.glb").read_bytes() == b"g" * 5000
    assert not (dest / "dn" / "buildings" / "a_lod0.glb.import").exists()
    assert models_package.read_installed_marker(result.models_dir)["version"] == 1
    needed, _ = models_package.needs_fetch(dest, hosting)
    assert not needed


def test_fetch_over_http(tmp_path: Path, http_server) -> None:  # noqa: ANN001, F811
    """Fake server: manifest and parts are downloaded and verified."""
    served, base_url = http_server
    _, hosting, out = _pack(tmp_path)
    for item in out.iterdir():
        (served / item.name).write_bytes(item.read_bytes())
    dest = tmp_path / "dest"
    models_package.fetch(
        dest=dest, base_url=base_url, hosting_file=hosting, log=lambda *_: None
    )
    assert (dest / "dn" / "buildings" / "a_lod0.glb").exists()
    assert not list(dest.glob(".dn.download-*"))


def test_fetch_rejects_a_corrupt_part(tmp_path: Path) -> None:
    """A part that does not match its SHA-256 aborts and leaves nothing installed."""
    _, hosting, out = _pack(tmp_path)
    first = sorted(out.glob("*.tar"))[0]
    first.write_bytes(b"x" + first.read_bytes()[1:])
    dest = tmp_path / "dest"
    with pytest.raises(ValueError, match="somme de contrôle"):
        models_package.fetch(
            dest=dest, from_dir=out, hosting_file=hosting, log=lambda *_: None
        )
    assert not (dest / "dn").exists()


def test_needs_fetch_cases(tmp_path: Path) -> None:
    """Absent: fetch. Behind: fetch. Marker-less models: adopted. Current: nothing."""
    hosting = _hosting(tmp_path / "hosting.json", version=3)
    dest = tmp_path / "dest"
    assert models_package.needs_fetch(dest, hosting)[0]
    models = _tree(dest)
    needed, reason = models_package.needs_fetch(dest, hosting)
    assert not needed and "adoptés" in reason
    assert models_package.read_installed_marker(models)["version"] == 3
    models_package.write_installed_marker(models, {"version": 2})
    assert models_package.needs_fetch(dest, hosting)[0]
    models_package.write_installed_marker(models, {"version": 3})
    assert not models_package.needs_fetch(dest, hosting)[0]


def test_update_no_publish_then_noop(tmp_path: Path) -> None:
    """Packs once; a second run on the unchanged tree has nothing to do."""
    models = _tree(tmp_path / "src")
    hosting = _hosting(tmp_path / "hosting.json")
    out = tmp_path / "out"
    first = models_package.update(
        models, hosting, out, do_publish=False, log=lambda *_: None
    )
    assert first.packed and not first.published
    second = models_package.update(
        models, hosting, out, do_publish=False, log=lambda *_: None
    )
    assert not second.packed


def test_update_publishes_with_gh_and_bumps_on_change(tmp_path: Path) -> None:
    """Publication order and tag; a changed tree bumps the version."""
    models = _tree(tmp_path / "src")
    hosting = _hosting(tmp_path / "hosting.json")
    commands: list[list[str]] = []

    def run(command: list[str]) -> int:
        commands.append(command)
        return 1 if command[:3] == ["gh", "release", "view"] else 0

    def published(hosting_dict: dict) -> str | None:
        return json.loads(hosting.read_text())["content"]["signature"]

    models_package.update(
        models,
        hosting,
        tmp_path / "out",
        do_publish=True,
        log=lambda *_: None,
        read_published=published,
        run=run,
    )
    create = next(c for c in commands if c[:3] == ["gh", "release", "create"])
    assert create[3] == "models-v1"
    assert commands[-1][-1].endswith("manifest.json")
    (models / "buildings" / "b_lod0.glb").write_bytes(b"n" * 100)
    result = models_package.update(
        models, hosting, tmp_path / "out", do_publish=False, log=lambda *_: None
    )
    assert result.version == 2


def test_relief_pack_untouched_by_the_refactor() -> None:
    """The relief pack still exposes the shared writer under its public name."""
    assert relief_pack.SplitWriter is not None
