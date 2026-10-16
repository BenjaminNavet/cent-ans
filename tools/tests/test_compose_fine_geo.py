"""Fine geography carried into the composed frame (lot Bevy geofine2)."""

from __future__ import annotations

import numpy as np
import pytest

from cent_ans_tools.compose import fine_geo
from cent_ans_tools.compose.contract import ComposedWarp
from cent_ans_tools.geo import fine_tiles

W, H, F = 512, 384, 8


def affine_warp(scale: float = 0.75, shift=(-40.0, -20.0)) -> ComposedWarp:
    """Forward warp ``x' = scale * x + shift`` on a 512 x 384 real crop."""
    ny, nx = H // F, W // F
    jj, ii = np.mgrid[0:ny, 0:nx]
    rx, ry = (ii + 0.5) * F, (jj + 0.5) * F
    fwd = np.stack([scale * rx + shift[0], scale * ry + shift[1]], axis=-1)
    size = (int(W * scale), int(H * scale))
    cy, cx = np.mgrid[0 : size[1] // F, 0 : size[0] // F]
    inv = np.stack(
        [((cx + 0.5) * F - shift[0]) / scale, ((cy + 0.5) * F - shift[1]) / scale],
        axis=-1,
    )
    return ComposedWarp(
        {}, (0, 0, W, H), F, size, fwd.astype("<f4"), inv.astype("<f4"), {}
    )


def test_densify_caps_segment_length() -> None:
    xy = np.array([[0.0, 0.0], [20.0, 0.0]])
    out, att = fine_geo.densify(xy, np.array([[0.0], [10.0]]), step=8.0)
    assert len(out) == 4
    assert np.max(np.hypot(*np.diff(out, axis=0).T)) <= 8.0
    assert att[-1, 0] == pytest.approx(10.0)
    assert att[1, 0] == pytest.approx(10.0 / 3)


def test_warp_polyline_is_the_composition_transform() -> None:
    warp = affine_warp()
    xy = np.array([[100.0, 100.0], [140.0, 120.0]])
    runs = fine_geo.warp_polyline(warp, xy, np.array([[5.0, 2.0], [7.0, 4.0]]))
    assert len(runs) == 1
    out, att = runs[0]
    np.testing.assert_allclose(out[0], [0.75 * 100 - 40, 0.75 * 100 - 20], atol=1e-3)
    np.testing.assert_allclose(out[-1], [0.75 * 140 - 40, 0.75 * 120 - 20], atol=1e-3)
    assert att[0, 0] == pytest.approx(5.0) and att[-1, 1] == pytest.approx(4.0)


def test_warp_polyline_splits_outside_the_crop() -> None:
    warp = affine_warp()
    xy = np.array([[100.0, 100.0], [100.0, 200.0], [W + 50.0, 200.0], [100.0, 300.0]])
    runs = fine_geo.warp_polyline(warp, xy, np.zeros((4, 2)))
    assert len(runs) == 2


def test_warp_layer_retiles(tmp_path) -> None:
    warp = affine_warp(scale=1.0, shift=(70.0, 0.0))
    lines = fine_tiles.TileLines()
    xy = np.array([[10.0, 10.0], [50.0, 10.0]])
    lines.add(7, fine_tiles.FLAG_TIDAL, xy, np.array([1.0, 2.0]), np.array([30.0, 30.0]))
    src = tmp_path / "E2"
    src.mkdir()
    (src / "0_0.bin").write_bytes(
        fine_tiles.encode(fine_tiles.LAYER_RIVERS, 2, 0, 0, lines)
    )
    tiles, stats = fine_geo.warp_layer(warp, src, fine_tiles.LAYER_RIVERS)
    # Real x 10..50 -> composed 80..120: tile column 1 (64..128).
    assert set(tiles) == {(1, 0)}
    piece = tiles[(1, 0)]
    assert piece.features == [7] and piece.flags == [fine_tiles.FLAG_TIDAL]
    np.testing.assert_allclose(piece.xy[0][0], [80.0, 10.0], atol=1e-3)
    assert stats.lines_in == 1 and stats.points_dropped == 0


def test_anchors_keep_offsets_and_follow_rules() -> None:
    warp = affine_warp()
    real_anchors = {
        "settlements": {
            "set_a": {"px": [110.0, 100.0], "z": 50.0, "moved_m": 7000.0},
            "set_b": {"px": [200.0, 200.0], "z": 10.0},
        },
        "hamlets": {"fields": ["x", "y", "z", "moved_m"], "items": [[300.0, 100.0, 5.0, 0.0]]},
        "crossings": [
            {"id": "br", "px": [120.0, 160.0], "snapped": True, "dir": [1.0, 0.0]},
        ],
    }
    real_rules = {
        "settlements": {"set_a": [100.0, 100.0], "set_b": [200.0, 200.0]},
        "hamlets": [{"name": "H", "province": "p", "px": [300.0, 100.0]}],
    }
    composed = {
        # set_b was moved by the editor: its anchor follows the rule position.
        "settlements": {"set_a": [35.0, 55.0], "set_b": [111.0, 133.0]},
        "hamlets": [
            {"name": "H", "province": "p", "px": [185.0, 55.0]},
            {"name": "New", "province": "p", "px": [10.0, 10.0]},
        ],
        "crossings": [{"id": "br", "px": [0.0, 0.0]}, {"id": "zz", "px": [1.0, 2.0]}],
    }
    out, res = fine_geo.warp_anchors(
        warp, real_anchors, real_rules, composed, height=lambda x, y: 42.0
    )
    np.testing.assert_allclose(out["settlements"]["set_a"]["px"], [35.0 + 7.5, 55.0])
    assert out["settlements"]["set_a"]["moved_m"] == 7000.0
    np.testing.assert_allclose(out["settlements"]["set_b"]["px"], [111.0, 133.0])
    assert res.anchor_offset_px["median"] == pytest.approx(3.75)
    assert out["hamlets"]["items"][0][:2] == [185.0, 55.0]
    assert out["hamlets"]["items"][1] == [10.0, 10.0, 42.0, 0.0]
    assert res.hamlets_matched == 1
    br = out["crossings"][0]
    np.testing.assert_allclose(br["px"], [0.75 * 120 - 40, 0.75 * 160 - 20], atol=1e-3)
    np.testing.assert_allclose(br["dir"], [1.0, 0.0], atol=1e-4)
    assert out["crossings"][1]["snapped"] is False
    assert res.crossings_missing == ["zz"]
