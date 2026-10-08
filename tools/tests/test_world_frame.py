"""Frame of the relief cache: manifest origin, cache origin, frame file."""

import json
from pathlib import Path

from cent_ans_tools.geo import world_frame


def test_manifest_origin(tmp_path: Path) -> None:
    """``root_origin_tiles`` of the manifest, ``(0, 0)`` without manifest."""
    assert world_frame.manifest_origin(tmp_path) == (0, 0)
    (tmp_path / "relief_pyramid.json").write_text(
        json.dumps({"root_origin_tiles": [0, 5]})
    )
    assert world_frame.manifest_origin(tmp_path) == (0, 5)


def test_cache_origin_and_frame_file(tmp_path: Path) -> None:
    """A cache with tiles and no frame.json is legacy; frame.json wins; empty is None."""
    cache = tmp_path / "pyramid"
    (cache / "E1").mkdir(parents=True)
    (cache / "E1" / "0_0.png").write_bytes(b"x")
    assert world_frame.cache_origin(cache) == world_frame.LEGACY_ORIGIN
    world_frame.write_frame(cache, (0, 0))
    assert world_frame.cache_origin(cache) == (0, 0)
    assert world_frame.cache_origin(tmp_path / "empty") is None
