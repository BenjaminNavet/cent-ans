"""GA: tileable material pipeline and data/art/materials.yaml."""

import pytest

pytestmark = pytest.mark.skip(reason="GA0 skeleton: enabled in GA1")


def test_make_tileable_edges_match():
    """Opposite edges differ by at most 4/255 after make_tileable."""


def test_derive_maps_shapes_and_channels():
    """Derived maps keep the albedo size and expected channel counts."""


def test_materials_yaml_matches_schema():
    """data/art/materials.yaml validates against materials.schema.json."""
