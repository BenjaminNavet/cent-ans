"""Bake versions of the relief cache and valley floors (lot SZ2, ADR 0036)."""

import json
import os
import time
from pathlib import Path

import numpy as np
import pytest

from cent_ans_tools.geo import bake_stamp, relief_cache, relief_shade

# ------------------------------------------------------------------ valley floor


def test_valley_floor_is_monotone_and_close_to_the_real_height() -> None:
    """Floor = max(0.5, 0.85 h, h - 2 m): non-decreasing, within 2 m above ~13 m."""
    heights = np.linspace(0.0, 400.0, 4001, dtype=np.float32)
    floor = relief_shade.valley_floor(heights)
    assert np.all(np.diff(floor) >= 0.0)
    assert floor[0] == pytest.approx(relief_shade.MIN_LAND_M)
    high = heights > 14.0
    np.testing.assert_allclose(
        floor[high], heights[high] - relief_shade.VALLEY_DIG_MAX_M, atol=1e-3
    )
    low = (heights > 1.0) & (heights < 13.0)
    np.testing.assert_allclose(
        floor[low], relief_shade.VALLEY_KEEP * heights[low], atol=1e-3
    )


def test_boost_no_longer_digs_an_incised_valley() -> None:
    """Seine-like valley (floor 12 m, plateaux 130 m): the floor stays within 2 m.

    Before SZ2 the σ 5 km base (≈ 100 m) dug it by 0.8 x 88 m, below sea level,
    then clamped it to 0.5 m. Hills still grow.
    """
    size = 128
    height = np.full((size, size), 130.0, dtype=np.float32)
    height[:, 60:68] = 12.0
    height[20:24, 20:24] = 180.0
    land = np.ones(height.shape, dtype=bool)
    boosted = relief_shade.boost_relief(height, land, 500.0)
    valley = boosted[:, 60:68]
    assert valley.min() >= 12.0 - relief_shade.VALLEY_DIG_MAX_M - 1e-3
    assert valley.max() <= 12.0 + 1e-3
    assert boosted[21, 21] > 180.0 + 20.0


def test_pyramid_and_detail_share_the_valley_floor() -> None:
    """E1-E4 (pyramid.boost_with_base) and E5-E7 (detail_dem.apply_boost) agree."""
    from cent_ans_tools.geo import detail_dem, pyramid

    heights = np.array([[3.0, 12.0, 55.0, 92.0, 150.0]], dtype=np.float32)
    base = np.full_like(heights, 110.0)
    pyr = pyramid.boost_with_base(heights, base)
    det = detail_dem.apply_boost(heights, base)
    np.testing.assert_allclose(pyr, det, atol=1e-4)
    np.testing.assert_allclose(
        pyr[0, :4], relief_shade.valley_floor(heights[0, :4]), atol=1e-4
    )


# ------------------------------------------------------------------- bake stamps


def test_begin_finish_and_resume(tmp_path: Path) -> None:
    """A stale tier starts a forced bake; an interrupted one resumes from its start."""
    root = tmp_path / "pyramid"
    started = bake_stamp.begin(root, "tier1", 2, force=False)
    assert started is not None  # no stamp: stale, rebake everything
    assert bake_stamp.stale_tiers(root, {"tier1": 2}) == ["tier1"]
    # Interrupted, then resumed: same threshold, not a new start.
    assert bake_stamp.begin(root, "tier1", 2, force=False) == started
    bake_stamp.finish(root, "tier1", 2, started)
    assert bake_stamp.stale_tiers(root, {"tier1": 2}) == []
    assert bake_stamp.begin(root, "tier1", 2, force=False) is None
    # A newer code version makes it stale again; force always restarts.
    assert bake_stamp.stale_tiers(root, {"tier1": 3, "tier2": 1}) == ["tier1", "tier2"]
    assert bake_stamp.begin(root, "tier1", 2, force=True) >= started
    data = json.loads((root / bake_stamp.STAMP_FILE).read_text())
    assert data["tier1"]["complete"] is False


def test_needs_rebake_uses_the_threshold(tmp_path: Path) -> None:
    """Missing tiles always, older ones only below a threshold."""
    tile = tmp_path / "t.png"
    assert bake_stamp.needs_rebake(tile, None)
    tile.write_bytes(b"x")
    now = time.time()
    os.utime(tile, (now - 100.0, now - 100.0))
    assert not bake_stamp.needs_rebake(tile, None)
    assert bake_stamp.needs_rebake(tile, now - 10.0)
    assert not bake_stamp.needs_rebake(tile, now - 200.0)


def test_relief_cache_reports_a_stale_tier(tmp_path: Path) -> None:
    """Every tile present but tier 2 baked by an older version: rerun from tier 2."""
    map_dir = tmp_path / "map"
    map_dir.mkdir()
    levels = [
        {"level": level, "tiles_rle": [{"row": 1, "runs": [[2, 1]]}]}
        for level in range(1, 8)
    ]
    (map_dir / "relief_pyramid.json").write_text(
        json.dumps(
            {
                "dir": "pyramid",
                "levels": levels,
                "bake_versions": {"tier1": 2, "tier2": 2, "tier3": 5},
            }
        )
    )
    for level in range(1, 8):
        path = map_dir / "pyramid" / f"E{level}" / "2_1.png"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"x")
    root = map_dir / "pyramid"
    bake_stamp.finish(root, "tier1", 2, 1.0)
    bake_stamp.finish(root, "tier2", 1, 1.0)
    bake_stamp.finish(root, "tier3", 5, 1.0)
    report = relief_cache.check(map_dir, tmp_path / "raw")
    assert report.stale == ["tier2"]
    assert not report.complete
    assert report.plan()[:2] == ["tier2", "tier3"]
    assert "périmée" in "\n".join(report.lines())
