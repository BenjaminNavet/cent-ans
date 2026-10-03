"""Valley occlusion bake (lot RV-D): synthetic valley, summit, plain and encoding."""

import json

import numpy as np
from PIL import Image

from cent_ans_tools.geo import relief_occlusion, terrain

MPP = 719.0


def _grid(size: int = 96) -> tuple[np.ndarray, np.ndarray]:
    coords = (np.arange(size) - size / 2.0) * MPP
    return np.meshgrid(coords, coords)


def test_v_valley_bottom_is_occluded() -> None:
    """V-shaped valley: the floor sees a raised horizon, the rims an open one."""
    x, _ = _grid()
    height = 500.0 + np.abs(x) * 0.08  # 8 % slopes on both sides
    occ = relief_occlusion.compute_occlusion(height, MPP)
    mid = occ.shape[0] // 2
    floor = occ[mid, mid]
    rim = occ[mid, mid + 25]
    assert floor > 0.02
    assert floor > rim


def test_summit_is_open() -> None:
    """Isolated cone: the summit has a falling horizon (negative occlusion)."""
    x, y = _grid()
    height = np.maximum(2000.0 - np.hypot(x, y) * 0.1, 200.0)
    occ = relief_occlusion.compute_occlusion(height, MPP)
    mid = occ.shape[0] // 2
    assert occ[mid, mid] < -0.02
    assert occ[mid, mid] == occ.min() or occ[mid, mid] < 0.5 * occ.min()


def test_plain_is_neutral() -> None:
    """Flat plain: zero occlusion everywhere, encoded 128."""
    height = np.full((64, 64), 120.0, dtype=np.float32)
    occ = relief_occlusion.compute_occlusion(height, MPP)
    assert np.allclose(occ, 0.0, atol=1e-6)
    encoded, _ = relief_occlusion.encode_occlusion(occ, np.ones_like(occ, dtype=bool))
    assert np.all(encoded == 128)


def test_encoding_sign_and_sea() -> None:
    """Valley above 128, crest below, sea forced to 128."""
    occ = np.array([[0.2, -0.2, 0.5]], dtype=np.float32)
    land = np.array([[True, True, False]])
    encoded, gain = relief_occlusion.encode_occlusion(occ, land, gain=2.0)
    assert gain == 2.0
    assert encoded[0, 0] > 128 > encoded[0, 1]
    assert encoded[0, 2] == 128


def test_build_writes_half_resolution(tmp_path) -> None:
    """End to end on a tiny map: L8 PNG at half the map grid."""
    x, _ = _grid(64)
    height = 300.0 + np.abs(x) * 0.05
    height[:, :4] = -50.0  # a strip of sea
    Image.fromarray(terrain.height_to_uint16(height)).save(tmp_path / "heightmap.png")
    (tmp_path / "map.json").write_text(
        json.dumps({"meters_per_px": MPP}), encoding="utf-8"
    )
    result = relief_occlusion.build(tmp_path, radii_m=(3000.0, 8000.0))
    with Image.open(result.path) as image:
        assert image.mode == "L"
        assert image.size == (32, 32)
        data = np.asarray(image)
    assert np.all(data[:, 0] == 128)
    assert data[16, 16] > 128
