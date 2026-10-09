"""TX texture factory (ADR 0236): skeleton tests, enabled lot by lot (T1b-T1e)."""

from __future__ import annotations

import pytest

from cent_ans_tools import texture_factory


def test_families_declared() -> None:
    """The five catalogue families of the spec are declared."""
    assert len(texture_factory.FAMILIES) == 5


@pytest.mark.skip(reason="T1b : portage de seamless")
def test_seamless_edges_match() -> None:
    """A seamless tile has a seam error below the threshold."""
