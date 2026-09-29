"""Land below sea level cut off from the ocean is lifted above 0 m (lot OM I1)."""

import numpy as np

from cent_ans_tools.geo import terrain


def test_inland_depression_is_lifted_ocean_lowland_is_kept() -> None:
    """Caspian-like basin lifted and continuous; low land on the ocean untouched."""
    height = np.full((20, 40), 50.0, dtype=np.float32)
    land = np.ones((20, 40), dtype=bool)
    # Ocean on the left edge, a polder (-3 m) touching it.
    land[:, :3] = False
    height[:, :3] = -100.0
    height[5:10, 3:8] = -3.0
    # Inland basin: a lake at -40 m ringed by land from -28 m to -1 m.
    height[5:15, 20:35] = -1.0
    height[7:13, 23:32] = -28.0
    height[9:11, 26:29] = -40.0
    land[9:11, 26:29] = False

    out, count = terrain.lift_inland_depressions(height, land, (0, 0))

    assert count == int((land & (height <= 0.0))[:, 10:].sum())
    basin = land & (height <= 0.0)
    basin[:, :10] = False
    assert (out[basin] >= terrain.DEPRESSION_FLOOR_M).all()
    assert (
        out[basin] <= terrain.DEPRESSION_FLOOR_M + terrain.DEPRESSION_SPAN_M + 1e-6
    ).all()
    # Deeper land stays lower: relief order kept inside the basin.
    assert out[7, 23] < out[5, 20]
    # Lake inside the basin and the polder on the ocean are unchanged.
    assert (out[9:11, 26:29] == -40.0).all()
    assert (out[5:10, 3:8] == -3.0).all()
    assert (out[height > 0.0] == height[height > 0.0]).all()


def test_no_depression_returns_input() -> None:
    """Nothing below 0 m inland: same array, zero lifted."""
    height = np.full((4, 4), 10.0, dtype=np.float32)
    land = np.ones((4, 4), dtype=bool)
    land[0, 0] = False
    out, count = terrain.lift_inland_depressions(height, land, (0, 0))
    assert count == 0
    assert (out == height).all()
