"""Tests for the province step: id encoding and neighbour extraction (no network)."""

import numpy as np

from cent_ans_tools.geo import provinces


def test_id_encoding_round_trip() -> None:
    """R holds the low byte, G the high byte, B is zero; decoding inverts it."""
    labels = np.array([[0, 1, 255], [256, 300, 65535]], dtype=np.uint16)
    rgb = provinces.encode_ids(labels)
    assert rgb.dtype == np.uint8 and rgb.shape == (2, 3, 3)
    assert rgb[0, 1].tolist() == [1, 0, 0]
    assert rgb[0, 2].tolist() == [255, 0, 0]
    assert rgb[1, 0].tolist() == [0, 1, 0]
    assert rgb[1, 1].tolist() == [44, 1, 0]
    assert np.all(rgb[..., 2] == 0)
    np.testing.assert_array_equal(provinces.decode_ids(rgb), labels)


def _synthetic_labels() -> np.ndarray:
    """Three provinces: 1 | 2 side by side, 3 below, sea (0) around."""
    labels = np.zeros((10, 10), dtype=np.uint16)
    labels[1:5, 1:5] = 1
    labels[1:5, 5:9] = 2
    labels[5:9, 1:9] = 3
    return labels


def test_land_neighbours_from_adjacent_pixels() -> None:
    """Provinces sharing a border are mutual neighbours; sea is ignored."""
    graph = provinces.land_neighbours(_synthetic_labels())
    assert graph == {1: {2, 3}, 2: {1, 3}, 3: {1, 2}}


def test_land_neighbours_ignore_short_contacts() -> None:
    """A single-pixel (corner-like) contact is dropped below the threshold."""
    labels = np.zeros((6, 6), dtype=np.uint16)
    labels[1:3, 1:3] = 1
    labels[2, 3] = 2
    labels[3:5, 3:5] = 2
    graph = provinces.land_neighbours(labels, min_shared_px=3)
    assert graph == {}
    graph = provinces.land_neighbours(labels, min_shared_px=1)
    assert graph == {1: {2}, 2: {1}}


def test_sea_neighbours_across_a_strait() -> None:
    """Two islands closer than the radius are sea neighbours, not land neighbours."""
    labels = np.zeros((20, 40), dtype=np.uint16)
    labels[5:15, 2:10] = 1
    labels[5:15, 20:28] = 2
    labels[5:15, 30:38] = 3
    graph = provinces.sea_neighbours(labels, {}, radius_px=12.0, step=1)
    assert graph == {1: {2}, 2: {1, 3}, 3: {2}}
    # Provinces already adjacent by land are never listed as sea neighbours.
    graph = provinces.sea_neighbours(labels, {2: {3}, 3: {2}}, radius_px=12.0, step=1)
    assert graph == {1: {2}, 2: {1}}


def test_vectorise_and_capital_snapping() -> None:
    """Regions become polygons in pixel corners; a capital in the sea snaps to its province."""
    labels = _synthetic_labels()
    geometries = provinces.vectorise(labels, tolerance=0.0, min_area=1.0)
    assert set(geometries) == {1, 2, 3}
    assert geometries[1].bounds == (1.0, 1.0, 5.0, 5.0)
    assert geometries[3].area == 32.0
    inside, moved = provinces.snap_capital(labels, 1, 2.5, 2.5)
    assert inside == (2.5, 2.5) and not moved
    snapped, moved = provinces.snap_capital(labels, 2, 9.7, 0.2)
    assert moved and labels[int(snapped[1]), int(snapped[0])] == 2


def test_assignable_land_drops_large_seedless_landmass() -> None:
    """A big landmass without seed is excluded, a small island is kept."""
    land = np.zeros((20, 20), dtype=bool)
    land[1:9, 1:9] = True  # seeded continent
    land[12:19, 1:19] = True  # large seedless landmass
    land[2:4, 15:17] = True  # small seedless island
    sources = np.array([[4, 4]])
    kept = provinces.assignable_land(land, sources, max_seedless_px=20)
    assert kept[4, 4] and kept[2, 15]
    assert not kept[15, 10]
