"""Tests for the terrain shader rasters (splatmap, border and coast distances)."""

import numpy as np

from cent_ans_tools.geo import splat


def _toy_map() -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """64² map: sea on the left, lowland province 1, mountain province 2 on the right."""
    size = 64
    cols = np.arange(size)[None, :].repeat(size, axis=0)
    height = np.where(cols < 10, -50.0, 50.0 + np.maximum(cols - 36, 0) * 150.0)
    land = cols >= 10
    ids = np.where(cols < 10, 0, np.where(cols < 36, 1, 2)).astype(np.uint16)
    return height.astype(np.float32), land, ids


def test_splat_weights_are_normalised_and_zero_at_sea() -> None:
    """Weights sum to 1 on land and are 0 at sea; the encoded PNG sums to 255."""
    height, land, ids = _toy_map()
    weights = splat.compute_splat(
        height, land, ids, {1: "plains", 2: "mountains"}, 1000.0
    )
    assert weights.shape == (64, 64, 4)
    np.testing.assert_allclose(weights[land].sum(axis=-1), 1.0, atol=1e-5)
    assert np.all(weights[~land] == 0.0)
    encoded = splat.encode_splat(weights)
    assert encoded.dtype == np.uint8
    assert np.all(encoded[land].astype(int).sum(axis=-1) == 255)
    assert np.all(encoded[~land] == 0)


def test_splat_puts_fields_in_plains_and_rock_on_peaks() -> None:
    """Farmland dominates the low plain; rock dominates the high mountain."""
    height, land, ids = _toy_map()
    weights = splat.compute_splat(
        height, land, ids, {1: "plains", 2: "mountains"}, 1000.0
    )
    plain = weights[:, 14:30].mean(axis=(0, 1))
    peak = weights[:, 56:].mean(axis=(0, 1))
    assert plain[1] > plain[3] and plain[1] > 0.3
    assert peak[3] > peak[1] and peak[3] > 0.5


def test_splat_is_deterministic() -> None:
    """Same inputs and seed give the same splatmap."""
    height, land, ids = _toy_map()
    a = splat.compute_splat(height, land, ids, {1: "forest"}, 1000.0, seed=7)
    b = splat.compute_splat(height, land, ids, {1: "forest"}, 1000.0, seed=7)
    np.testing.assert_array_equal(a, b)


def test_border_distance_is_small_on_borders_and_grows_inside() -> None:
    """The border between provinces 1 and 2 is at 0 px; coasts are not borders."""
    _, land, ids = _toy_map()
    signed = splat.compute_border_dist(ids, land)
    assert signed.shape == (64, 64, splat.BORDER_CHANNELS)
    dist = np.abs(signed).min(axis=-1)
    assert dist[32, 35] < 1.0 and dist[32, 36] < 1.0
    assert dist[32, 25] > 9.0
    # La côte (colonne 10) n'est pas une frontière de province.
    assert dist[32, 10] > 20.0
    # Au moins un canal change de signe à la frontière : zéro sous-pixel entre 35 et 36.
    assert np.any(np.sign(signed[32, 35]) != np.sign(signed[32, 36]))
    midpoint = np.abs(0.5 * (signed[32, 35] + signed[32, 36])).min()
    assert midpoint < 0.1
    decoded = splat.decode_border_dist(splat.encode_border_dist(signed))
    assert decoded[32, 20] > decoded[32, 33]


def test_region_colouring_separates_neighbours() -> None:
    """Adjacent regions never share a colour."""
    ids = np.array(
        [[1, 1, 2, 2], [1, 3, 3, 2], [4, 3, 3, 5], [4, 4, 5, 5]], dtype=np.uint16
    )
    labels = splat.region_labels(ids, np.ones(ids.shape, dtype=bool))
    colours = splat.colour_regions(labels)
    for a, b in [(1, 2), (1, 3), (2, 3), (3, 4), (3, 5), (4, 5), (1, 4), (2, 5)]:
        assert colours[a] != colours[b]
    assert max(colours.values()) < 2**splat.BORDER_CHANNELS


def test_coast_distance_is_signed() -> None:
    """Positive on land, negative at sea, 128 at the shore once encoded."""
    _, land, _ = _toy_map()
    signed = splat.compute_coast_dist(~land)
    assert signed[32, 30] > 0 > signed[32, 2]
    encoded = splat.encode_coast_dist(signed)
    assert abs(int(encoded[32, 10]) - 128) <= splat.COAST_DIST_SCALE
    assert encoded[32, 40] > 128 > encoded[32, 5]


def test_pack_normal_rough_keeps_xy_and_stores_roughness_in_blue() -> None:
    """Normal X/Y stay in R/G, roughness replaces the (rebuilt) Z in B."""
    from cent_ans_tools.geo import textures

    normal = np.zeros((2, 2, 3), dtype=np.uint8)
    normal[..., 0], normal[..., 1], normal[..., 2] = 120, 130, 255
    rough = np.full((2, 2), 200, dtype=np.uint8)
    packed = textures.pack_normal_rough(normal, rough)
    assert packed.shape == (2, 2, 3)
    assert np.all(packed[..., 0] == 120) and np.all(packed[..., 1] == 130)
    assert np.all(packed[..., 2] == 200)
    assert list(textures.LAYERS)[:4] == ["grass", "farmland", "forest", "rock"]
