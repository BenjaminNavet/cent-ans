"""SVG-only icons rasterised to ink PNGs for Bevy."""

import json

import numpy as np
from PIL import Image

from cent_ans_tools import svg_png
from cent_ans_tools.ink_icons import INK_RGB


def test_render_ink_is_128_ink_coloured_with_alpha() -> None:
    """A rendered icon is 128 px, ink-coloured, alpha carries the drawing."""
    image = svg_png.render_ink(svg_png.ICONS_DIR / "delapouite-knight-banner.svg")
    assert image.size == (128, 128)
    data = np.asarray(image)
    alpha = data[..., 3]
    assert alpha.max() == 255 and alpha.mean() > 5
    assert (data[alpha > 0][:, :3] == INK_RGB).all()


def test_no_svg_only_icon_left_after_build() -> None:
    """Committed assets: every id of icons.json has a PNG family, the ruins marker exists."""
    assert svg_png.svg_only_icons() == {}
    index = json.loads((svg_png.INK_DIR / "index.json").read_text(encoding="utf-8"))[
        "icons"
    ]
    for icon_id in ("skill_banneret", "skill_sapeur", "marker_ruins"):
        assert Image.open(svg_png.INK_DIR / index[icon_id]).size == (128, 128)
