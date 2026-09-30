"""NB: tests of the UI ornament pipeline (no network)."""

import pytest

pytestmark = pytest.mark.skip(reason="NB skeleton: implemented in NB1")


def test_key_out_removes_green() -> None:
    """No green-dominant pixel is left opaque after keying."""


def test_fit_to_piece_matches_kit_size() -> None:
    """Output size equals the kit.json piece size."""


def test_seam_fix_seam_error() -> None:
    """Seam error of the stretched bands is at most 4/255."""


def test_fallback_is_procedural() -> None:
    """A missing NB piece leaves the procedural drawing unchanged."""
