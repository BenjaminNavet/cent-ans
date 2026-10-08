"""Procedural water detail textures: shape, periodicity, normals."""

import numpy as np

from cent_ans_tools import water_procedural as wp


def test_build_is_deterministic_and_well_formed():
    """Same params give the same bytes; normals point up; foam coverage is sparse."""
    normal, albedo = wp.build(wp.MATERIALS["river_large"], 64)
    again, _ = wp.build(wp.MATERIALS["river_large"], 64)
    assert normal.shape == (64, 64, 3) and albedo.shape == (64, 64, 4)
    assert np.array_equal(normal, again)
    assert (normal[..., 2] > 128).all()
    assert 0.0 < (albedo[..., 3] > 76).mean() < 0.3


def test_field_is_periodic():
    """Wrap-around gradient has no seam: opposite edges are as smooth as the interior."""
    field = wp.spectral_field(64, 4.0, 2.0, 1)
    seam = np.abs(field[:, 0] - field[:, -1]).mean()
    inner = np.abs(field[:, 31] - field[:, 32]).mean()
    assert seam < inner * 3
