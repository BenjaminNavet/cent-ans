"""TP: mflux Z-Image ControlNet with a ready-made depth map (no DepthPro re-estimation).

mflux always re-estimates depth from the ``depth:`` control image (DepthPro, +1.9 GB). Our control
image already is an exact depth render, so this launcher passes it through unchanged. Run it with
the mflux tool interpreter: ``~/.local/share/uv/tools/mflux/bin/python retex_mflux.py <CLI args>``.
"""

import sys

from mflux.models.z_image.cli.z_image_turbo_generate_controlnet import main
from mflux.models.z_image.variants.controlnet.control_types import ControlType
from mflux.models.z_image.variants.controlnet.controlnet_util import (
    ZImageControlnetUtil,
)

_original_preprocess = ZImageControlnetUtil._preprocess


def _preprocess(img, control_type):
    """Depth hints are used as given; other modalities keep mflux's preprocessing."""
    if control_type == ControlType.depth:
        return img.convert("RGB")
    return _original_preprocess(img, control_type)


ZImageControlnetUtil._preprocess = staticmethod(_preprocess)

if __name__ == "__main__":
    sys.argv[0] = "mflux-generate-z-image-controlnet"
    sys.exit(main())
