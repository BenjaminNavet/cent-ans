"""Relief cache moved into the world frame (lot OMR R7, ADR 0119)."""

import json
from pathlib import Path

import numpy as np

from cent_ans_tools.geo import fine_tiles, world_frame


def _cafv(col: int, row: int, xy: np.ndarray) -> bytes:
    lines = fine_tiles.TileLines()
    lines.add(7, 64, xy, np.array([1.0, 2.0]), np.array([3.0, 4.0]))
    return fine_tiles.encode(fine_tiles.LAYER_ROADS, 2, col, row, lines)


def test_shift_cafv_moves_address_and_vertices() -> None:
    """A CAFV tile shifted by (0, 5) root tiles: +20 E2 rows, +1280 units in y."""
    xy = np.array([[100.5, 10.25], [120.0, 30.0]])
    data = world_frame.shift_cafv(_cafv(1, 0, xy), (0, 5))
    tile = fine_tiles.decode(data)
    assert (tile["col"], tile["row"]) == (1, 20)
    line = tile["lines"][0]
    assert line["feature"] == 7 and line["flags"] == 64
    np.testing.assert_allclose(line["xy"], xy + [0.0, 1280.0])
    np.testing.assert_allclose(line["z"], [1.0, 2.0])
    np.testing.assert_allclose(line["w"], [3.0, 4.0])


def test_reframe_cache_links_tiles_to_world_addresses(tmp_path: Path) -> None:
    """Relief tiles are hard-linked at row + 5·2^k, CAFV tiles rewritten."""
    src, dst = tmp_path / "src", tmp_path / "dst"
    for level, name in ((1, "3_2.png"), (4, "40_7.png")):
        path = src / f"E{level}" / name
        path.parent.mkdir(parents=True)
        path.write_bytes(b"tile")
    fine = src / "roads_fine" / "E2" / "5_6.bin"
    fine.parent.mkdir(parents=True)
    fine.write_bytes(_cafv(5, 6, np.array([[330.0, 400.0], [331.0, 401.0]])))
    (src / "roads_fine" / "features.json").write_text("{}")
    (src / "bake.json").write_text("{}")
    counts = world_frame.reframe_cache(src, dst, (0, 5), log=lambda _: None)
    assert counts == {"relief": 2, "cafv": 1}
    assert (dst / "E1" / "3_12.png").stat().st_ino == (
        src / "E1" / "3_2.png"
    ).stat().st_ino
    assert (dst / "E4" / "40_87.png").exists()
    moved = fine_tiles.decode((dst / "roads_fine" / "E2" / "5_26.bin").read_bytes())
    assert moved["row"] == 26
    assert (dst / "roads_fine" / "features.json").exists()
    assert (dst / "bake.json").exists()
    assert (src / "E1" / "3_2.png").exists()  # the source cache is untouched


def test_reframe_in_place_and_frame_file(tmp_path: Path) -> None:
    """In place, tiles are renamed farthest row first (no overwrite)."""
    cache = tmp_path / "pyramid"
    for row in range(0, 12):
        path = cache / "E1" / f"0_{row}.png"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(str(row).encode())
    assert world_frame.cache_origin(cache) == world_frame.LEGACY_ORIGIN
    world_frame.reframe_cache(cache, cache, (0, 5), log=lambda _: None)
    world_frame.write_frame(cache, (0, 0))
    rows = sorted(int(p.stem.split("_")[1]) for p in (cache / "E1").glob("*.png"))
    assert rows == list(range(10, 22))
    assert (cache / "E1" / "0_15.png").read_bytes() == b"5"
    assert world_frame.cache_origin(cache) == (0, 0)
    assert world_frame.cache_origin(tmp_path / "empty") is None


def test_shift_manifests(tmp_path: Path) -> None:
    """Manifests: tiles_rle and fine tile indexes shifted, origin set to [0, 0]."""
    level = {"level": 2, "tiles_rle": [{"row": 1, "runs": [[3, 2]]}]}
    (tmp_path / "relief_pyramid.json").write_text(
        '{\n  "root_origin_tiles": [0, 5],\n  "levels": [\n    '
        + json.dumps(level)
        + '\n  ],\n  "cache": {}\n}'
    )
    (tmp_path / "rivers_fine.json").write_text(
        json.dumps({"tiles": [{"col": 1, "row": 2, "sha1": "abc"}]}, indent=1) + "\n"
    )
    (tmp_path / "fine_anchors.json").write_text(
        json.dumps({"roads": {"tiles": [{"col": 4, "row": 0}]}}, separators=(",", ":"))
    )
    world_frame.shift_manifests(tmp_path, (0, 5))
    manifest = json.loads((tmp_path / "relief_pyramid.json").read_text())
    assert manifest["root_origin_tiles"] == [0, 0]
    assert manifest["levels"][0]["tiles_rle"] == [{"row": 21, "runs": [[3, 2]]}]
    rivers = json.loads((tmp_path / "rivers_fine.json").read_text())
    assert rivers["tiles"][0]["row"] == 22
    assert (tmp_path / "rivers_fine.json").read_text().startswith("{\n ")
    roads = json.loads((tmp_path / "fine_anchors.json").read_text())
    assert roads["roads"]["tiles"][0] == {"col": 4, "row": 20}
