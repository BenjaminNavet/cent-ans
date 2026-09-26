"""Widths of anchored rivers along the anchor chain (lot ZG7a, ADR 0036)."""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest

DATA = Path(__file__).resolve().parents[2] / "data"


def _chain_model():  # noqa: ANN202
    """Three anchors along the x axis (upstream to downstream), 6 km search."""
    from cent_ans_tools.geo import hydro_fine

    return hydro_fine.WidthModel(
        by_order={1: 1.5, 4: 9.0, 8: 120.0},
        exponent=0.7,
        search_m=6000.0,
        rivers=[
            {
                "id": "seine",
                "anchors": [
                    (0.0, 0.0, 100.0),
                    (100_000.0, 0.0, 200.0),
                    (200_000.0, 0.0, 400.0),
                ],
            }
        ],
        names={"seine": 0},
    )


def test_anchor_chain_widths_ignore_stroke_boundaries() -> None:
    """A stroke far from every anchor still gets the chain's widths.

    Before, the Seine between Elbeuf and Rouen (no anchor within 6 km) fell back
    to its Strahler width (50 m) and each stroke shrank towards its first
    vertex as if it were the source (4.5 m just upstream of Rouen).
    """
    model = _chain_model()
    points = np.column_stack(
        [np.linspace(50_000.0, 150_000.0, 11), np.full(11, 3000.0)]
    )
    widths = model.river_widths("la Seine", points)
    assert widths is not None
    assert widths[0] == pytest.approx(100.0 * 2**0.5, rel=1e-3)
    assert widths[5] == pytest.approx(200.0, rel=1e-3)
    assert widths[-1] == pytest.approx(200.0 * 2**0.5, rel=1e-3)
    assert np.all(np.diff(widths) > 0.0)


def test_anchor_chain_upstream_and_homonyms_fall_back() -> None:
    """Upstream of the first anchor without a nearby one, and far off the chain: NaN."""
    from cent_ans_tools.geo import hydro_fine

    model = _chain_model()
    upstream = np.column_stack([np.linspace(-80_000.0, -40_000.0, 5), np.zeros(5)])
    assert np.all(np.isnan(model.river_widths("Seine", upstream)))
    homonym = np.column_stack([np.linspace(0.0, 50_000.0, 5), np.full(5, 90_000.0)])
    assert np.all(np.isnan(model.river_widths("Seine", homonym)))
    # Source decay kept upstream of the first anchor when it is within reach.
    near = np.column_stack([np.linspace(-5000.0, 0.0, 6), np.zeros(6)])
    decay = model.river_widths("Seine", near)
    assert decay[-1] == pytest.approx(100.0, rel=1e-3)
    assert decay[0] < 100.0
    fused = hydro_fine.stroke_widths(
        model, "Seine", upstream, np.full(5, 4), np.full(5, np.nan), np.full(5, np.nan)
    )
    assert np.all(fused == 9.0)


def test_width_anchors_listed_upstream_to_downstream() -> None:
    """The chain rule needs anchors in flow order: widths never shrink by half downstream."""
    widths = json.loads(
        (DATA / "map" / "river_widths.json").read_text(encoding="utf-8")
    )
    for river in widths["rivers"]:
        values = [a["width_m"] for a in river["anchors"]]
        for up, down in zip(values, values[1:], strict=False):
            assert down >= up * 0.5, river["id"]
