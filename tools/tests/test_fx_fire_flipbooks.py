"""Validates data/fx/fire_flipbooks.json and the baking of the FA2 flipbook frames."""

import json
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools import vfx_flipbooks

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
SHEETS = ROOT / "game/assets/textures/fx"


def _settings() -> dict:
    return json.loads((DATA / "fx" / "fire_flipbooks.json").read_text(encoding="utf-8"))


def test_fire_flipbooks_match_schema() -> None:
    """The flipbook settings match their schema."""
    schema = json.loads(
        (DATA / "schemas" / "fx_fire_flipbooks.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_settings()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_sheets_are_eight_by_eight() -> None:
    """Both versioned sheets hold 8x8 square frames of the configured size."""
    settings = _settings()
    for name, key in (("flame_flipbook.png", "flame"), ("smoke_flipbook.png", "smoke")):
        side = settings[key]["frame_px"] * vfx_flipbooks.GRID
        assert Image.open(SHEETS / name).size == (side, side), name


def test_flame_frames_are_centred_squares() -> None:
    """A tall source frame lands centred in a square frame, heat in RGB, coverage in A."""
    settings = _settings()["flame"]
    source = np.zeros((32, 16, 4), dtype=np.float32)
    source[8:24, 4:12] = [1.0, 0.6, 0.2, 1.0]
    frames = vfx_flipbooks.flame_sheet_frames([source], settings)
    size = settings["frame_px"]
    assert frames[0].shape == (size, size, 4)
    alpha = frames[0][..., 3]
    assert alpha[size // 2, size // 2] > 0.9
    assert alpha[:, : size // 4].max() < 0.05
    assert alpha[:, -size // 4 :].max() < 0.05


def test_smoke_puff_grows_and_thins() -> None:
    """The puff is smaller and denser when young than when old."""
    settings = _settings()["smoke"]
    source = np.zeros((32, 32, 4), dtype=np.float32)
    source[2:30, 2:30] = [0.5, 0.5, 0.5, 1.0]
    frames = vfx_flipbooks.smoke_sheet_frames([source] * vfx_flipbooks.FRAMES, settings)
    young, old = frames[0][..., 3], frames[-1][..., 3]
    assert (young > 0.1).sum() < (old > 0.1).sum()
    assert young.max() > old.max()


def test_flame_heat_ceiling_and_coverage_gamma() -> None:
    """`heat_max` caps the core heat and `coverage_gamma` < 1 thickens thin tongues."""
    base = {**_settings()["flame"], "base_fade_px": 0}
    source = np.zeros((32, 16, 4), dtype=np.float32)
    source[4:28, 4:12] = [1.0, 1.0, 1.0, 0.25]
    size = base["frame_px"]
    plain = vfx_flipbooks.flame_sheet_frames(
        [source], {**base, "heat_max": 1.0, "coverage_gamma": 1.0}
    )[0]
    tuned = vfx_flipbooks.flame_sheet_frames(
        [source], {**base, "heat_max": 0.5, "coverage_gamma": 0.5}
    )[0]
    centre = (size // 2, size // 2)
    assert abs(plain[centre][0] - 1.0) < 0.02
    assert abs(tuned[centre][0] - 0.5) < 0.02
    assert abs(plain[centre][3] - 0.25) < 0.02
    assert abs(tuned[centre][3] - 0.5) < 0.02


def test_smoke_edge_fade_clears_the_frame_border() -> None:
    """With `edge_fade`, a source filling its frame never reaches the border of the puff."""
    base = _settings()["smoke"]
    source = np.full((32, 32, 4), 0.5, dtype=np.float32)
    source[..., 3] = 1.0
    sources = [source] * vfx_flipbooks.FRAMES
    square = vfx_flipbooks.smoke_sheet_frames(sources, {**base, "edge_fade": 0.0})[-1]
    faded = vfx_flipbooks.smoke_sheet_frames(sources, {**base, "edge_fade": 0.3})[-1]
    border = np.concatenate(
        [faded[0, :, 3], faded[-1, :, 3], faded[:, 0, 3], faded[:, -1, 3]]
    )
    assert square[0, :, 3].max() > 0.1
    assert border.max() < 0.02
    middle = faded.shape[0] // 2
    assert faded[middle, middle, 3] > 0.9 * square[middle, middle, 3]


def test_smoke_density_floor_drops_the_thin_veil() -> None:
    """`density_floor` zeroes a faint veil that `density_gamma` < 1 would lift into a halo."""
    base = {**_settings()["smoke"], "edge_fade": 0.0, "density_gamma": 0.4}
    source = np.full((32, 32, 4), 0.5, dtype=np.float32)
    source[..., 3] = 0.03
    source[8:24, 8:24, 3] = 1.0
    sources = [source] * vfx_flipbooks.FRAMES
    lifted = vfx_flipbooks.smoke_sheet_frames(sources, {**base, "density_floor": 0.0})[
        0
    ]
    floored = vfx_flipbooks.smoke_sheet_frames(
        sources, {**base, "density_floor": 0.05}
    )[0]
    size = lifted.shape[0]
    young = base["young_scale"]
    veil = round(size * (0.5 - young * 0.42))
    centre = size // 2
    assert lifted[centre, veil, 3] > 0.15
    assert floored[centre, veil, 3] < 0.01
    assert floored[centre, centre, 3] > 0.95


def test_baked_smoke_never_reaches_the_frame_rim() -> None:
    """No frame of the versioned smoke sheet is denser than 0.03 on a ring at 90 % of its radius."""
    sheet = (
        np.asarray(Image.open(SHEETS / "smoke_flipbook.png").convert("RGBA"))[..., 3]
        / 255.0
    )
    size = sheet.shape[0] // vfx_flipbooks.GRID
    axis = (np.arange(size) + 0.5) / size * 2.0 - 1.0
    radius = np.sqrt(axis[None, :] ** 2 + axis[:, None] ** 2)
    ring = (radius > 0.88) & (radius < 0.92)
    for index in range(vfx_flipbooks.FRAMES):
        row, column = divmod(index, vfx_flipbooks.GRID)
        frame = sheet[
            row * size : (row + 1) * size, column * size : (column + 1) * size
        ]
        assert frame[ring].max() <= 0.03, index
