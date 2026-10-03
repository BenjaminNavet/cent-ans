"""Data staging of an exported build with a tiny fake relief cache (lot ZG7b)."""

import json
from pathlib import Path

import pytest

from cent_ans_tools import export_data


def _fake_repo(root: Path, linked: bool = False) -> Path:
    """``data/`` with a schema, a rules file and a one-tile pyramid (maybe a symlink)."""
    repo = root / "repo"
    (repo / "data" / "schemas").mkdir(parents=True)
    (repo / "data" / "schemas" / "x.schema.json").write_text("{}")
    (repo / "data" / "rules").mkdir()
    (repo / "data" / "rules" / "r.json").write_text("{}")
    (repo / "data" / "retinue.json").write_text("{}")
    map_dir = repo / "data" / "map"
    map_dir.mkdir()
    (map_dir / "map.json").write_text("{}")
    (map_dir / "relief_pyramid.json").write_text(
        json.dumps(
            {"levels": [{"level": 1, "tiles_rle": [{"row": 0, "runs": [[0, 1]]}]}]}
        )
    )
    cache = (root / "shared_cache") if linked else (map_dir / "pyramid")
    (cache / "E1").mkdir(parents=True)
    (cache / "E1" / "0_0.png").write_bytes(b"tile")
    if linked:
        (map_dir / "pyramid").symlink_to(cache, target_is_directory=True)
    return repo


def _resources(root: Path) -> Path:
    return root / "export" / "Cent Ans.app" / "Contents" / "Resources"


def test_bundle_puts_the_relief_inside_the_app(tmp_path: Path) -> None:
    """Default: data/ without schemas, relief cache in Resources/data/map/pyramid."""
    repo = _fake_repo(tmp_path)
    result = export_data.stage(_resources(tmp_path), "bundle", repo_dir=repo)
    data = _resources(tmp_path) / "data"
    assert not (data / "schemas").exists()
    assert (data / "rules" / "r.json").exists() and (data / "retinue.json").exists()
    assert (data / "map" / "map.json").exists()
    assert (data / "map" / "pyramid" / "E1" / "0_0.png").read_bytes() == b"tile"
    assert result.relief_dir == data / "map" / "pyramid"
    assert result.relief_bytes == 4
    assert result.report.complete is False  # no fine rivers/roads listed in the fake


def test_external_puts_the_relief_next_to_the_app(tmp_path: Path) -> None:
    """``external``: ``Cent Ans relief/pyramid`` beside the .app, nothing inside it."""
    repo = _fake_repo(tmp_path)
    result = export_data.stage(_resources(tmp_path), "external", repo_dir=repo)
    sibling = tmp_path / "export" / "Cent Ans relief" / "pyramid"
    assert result.relief_dir == sibling
    assert (sibling / "E1" / "0_0.png").exists()
    assert not (_resources(tmp_path) / "data" / "map" / "pyramid").exists()


def test_none_leaves_the_relief_out(tmp_path: Path) -> None:
    """``none``: light build."""
    repo = _fake_repo(tmp_path)
    result = export_data.stage(_resources(tmp_path), "none", repo_dir=repo)
    assert result.relief_dir is None
    assert not (_resources(tmp_path) / "data" / "map" / "pyramid").exists()
    assert not (tmp_path / "export" / "Cent Ans relief").exists()


def test_symlinked_cache_is_copied_as_files(tmp_path: Path) -> None:
    """Worktree layout: data/map/pyramid is a link; the export holds real files."""
    repo = _fake_repo(tmp_path, linked=True)
    export_data.stage(_resources(tmp_path), "bundle", repo_dir=repo)
    pyramid = _resources(tmp_path) / "data" / "map" / "pyramid"
    assert not pyramid.is_symlink()
    tile = pyramid / "E1" / "0_0.png"
    assert tile.is_file() and not tile.is_symlink()


def test_restage_replaces_previous_data(tmp_path: Path) -> None:
    """A second staging does not nest or keep stale files."""
    repo = _fake_repo(tmp_path)
    export_data.stage(_resources(tmp_path), "bundle", repo_dir=repo)
    stale = _resources(tmp_path) / "data" / "stale.json"
    stale.write_text("{}")
    export_data.stage(_resources(tmp_path), "bundle", repo_dir=repo)
    assert not stale.exists()
    assert not (_resources(tmp_path) / "data" / "map" / "pyramid" / "pyramid").exists()


def test_unknown_mode_is_rejected(tmp_path: Path) -> None:
    """Only bundle / external / none."""
    with pytest.raises(ValueError):
        export_data.stage(_resources(tmp_path), "pck", repo_dir=_fake_repo(tmp_path))


def test_windows_folder_gets_data_and_relief_beside_the_exe(tmp_path: Path) -> None:
    """Windows (ADR 0087): ``data/`` and ``Cent Ans relief/`` next to ``Cent Ans.exe``."""
    repo = _fake_repo(tmp_path)
    folder = tmp_path / "export" / "windows"
    result = export_data.stage(
        folder, "external", repo_dir=repo, external_parent=folder
    )
    assert (folder / "data" / "rules" / "r.json").is_file()
    assert (folder / "Cent Ans relief" / "pyramid" / "E1" / "0_0.png").is_file()
    assert result.relief_dir == folder / "Cent Ans relief" / "pyramid"


def test_unused_relief_shade_bands_are_left_out_when_bc5_is_complete(
    tmp_path: Path,
) -> None:
    """The PNG bands are dead weight once the BC5 parts listed in map.json exist."""
    repo = _fake_repo(tmp_path)
    map_dir = repo / "data" / "map"
    shade = {
        "bc5": {"pattern": "relief_shade_bc5_{part}.bin", "part_bytes": [1, 1]},
        "bands": {"pattern": "relief_shade_{band}.png", "count": 2},
    }
    (map_dir / "map.json").write_text(json.dumps({"relief_shade": shade}))
    (map_dir / "relief_shade_0.png").write_bytes(b"png")
    (map_dir / "relief_shade_1.png").write_bytes(b"png")
    (map_dir / "heightmap_render.png").write_bytes(b"png")
    (map_dir / "relief_shade_bc5_0.bin").write_bytes(b"bc5")
    # One BC5 part missing: the fallback bands must stay.
    export_data.stage(_resources(tmp_path), "none", repo_dir=repo)
    data_map = _resources(tmp_path) / "data" / "map"
    assert (data_map / "relief_shade_0.png").exists()
    (map_dir / "relief_shade_bc5_1.bin").write_bytes(b"bc5")
    export_data.stage(_resources(tmp_path), "none", repo_dir=repo)
    assert not (data_map / "relief_shade_0.png").exists()
    assert not (data_map / "relief_shade_1.png").exists()
    assert (data_map / "relief_shade_bc5_0.bin").exists()
    assert (data_map / "heightmap_render.png").exists()
