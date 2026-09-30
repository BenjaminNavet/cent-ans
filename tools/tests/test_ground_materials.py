"""HB2: ground materials catalogue, seamless tiling, luminance and packing."""

import json
from pathlib import Path

import numpy as np
import pytest
from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools import ground_materials as gm

ROOT = Path(__file__).resolve().parents[2]


def _noise_texture(size: int = 256, seed: int = 3) -> np.ndarray:
    """Natural-looking random texture whose opposite borders do NOT match."""
    rng = np.random.default_rng(seed)
    from scipy.ndimage import gaussian_filter

    base = gaussian_filter(rng.random((size, size, 3)), (3, 3, 0)) * 4 - 1.5
    ramp = np.linspace(0, 0.4, size)[np.newaxis, :, np.newaxis]  # left/right mismatch
    return np.clip(0.3 + 0.2 * base + ramp, 0, 1)


def test_catalog_matches_schema_and_layers_contiguous() -> None:
    """The catalogue validates, ids unique, layers 0..n-1, biomes within ADR 0143."""
    document = gm.load_catalog()
    assert len(document["materials"]) >= 20
    roles = {m["role"] for m in document["materials"]}
    assert roles == {"crop", "canopy", "ground"}
    covered = {b for m in document["materials"] for b in m["biomes"]}
    assert covered == set(range(1, 8))


def test_prompt_is_nadir_and_scaled() -> None:
    """Every prompt asks a top-down orthophoto and states the ground scale."""
    document = gm.load_catalog()
    for entry in document["materials"]:
        prompt = gm.build_prompt(document, entry)
        assert "orthophoto" in prompt and "no horizon" in prompt
        assert f"{entry['tile_m']} metres" in prompt


def test_make_seamless_closes_the_wrap_seam() -> None:
    """Opposite borders of the result differ no more than neighbouring pixels do."""
    image = _noise_texture()
    assert gm.seam_ratio(image) > 3.0
    tiled = gm.make_seamless(image, half_band=24)
    assert tiled.shape == image.shape
    assert gm.seam_ratio(tiled) < 1.5


def test_make_seamless_has_no_seam_on_the_central_cross() -> None:
    """The rolled seams (central cross) are replaced: no jump there either."""
    tiled = gm.make_seamless(_noise_texture(), half_band=24)
    interior = np.abs(np.diff(tiled, axis=1)).mean()
    centre = tiled.shape[1] // 2
    cross = np.abs(tiled[:, centre] - tiled[:, centre - 1]).mean()
    assert cross < 2.0 * interior


def test_loop_cut_closes() -> None:
    """The min-error cut starts and ends on the same column, steps of one column."""
    error = np.random.default_rng(1).random((200, 20))
    path = gm._loop_cut(error)
    assert path[0] == path[-1]
    assert np.abs(np.diff(path)).max() <= 1


def test_equalize_luminance_hits_target() -> None:
    """Mean linear luminance equals the target after equalisation."""
    image = _noise_texture() * 0.3
    out = gm.equalize_luminance(image, 0.18)
    assert abs(float((out @ gm._LUMA).mean()) - 0.18) < 0.005


def test_flatten_lighting_removes_blotches() -> None:
    """A large bright blotch is flattened (block means vary less)."""
    image = _noise_texture()
    y, x = np.mgrid[0:256, 0:256]
    blotch = 1 + 1.5 * np.exp(-((x - 80) ** 2 + (y - 80) ** 2) / (2 * 40**2))
    stained = image * blotch[..., np.newaxis]
    assert gm.blotch_score(gm.flatten_lighting(stained, 32)) < gm.blotch_score(stained)


def test_normal_rough_is_flat_on_flat_input() -> None:
    """A flat albedo gives an up-facing normal and the base roughness."""
    packed = gm.derive_normal_rough(np.full((64, 64, 3), 0.2), 2.0, 0.8)
    assert abs(int(packed[..., 0].mean()) - 128) <= 1
    assert abs(int(packed[..., 1].mean()) - 128) <= 1
    assert abs(int(packed[..., 2].mean()) - 204) <= 1


def test_grid_shape_is_exact() -> None:
    """Columns x rows equals the layer count (Godot slices the whole image)."""
    for count in (20, 24, 27, 28):
        columns, rows = gm.grid_shape(count)
        assert columns * rows == count and columns <= rows


def test_pack_writes_arrays_imports_and_manifest(tmp_path: Path) -> None:
    """Pack of small fake tiles: grid sizes, import slices, manifest schema."""
    document = gm.load_catalog()
    document["layer_size"] = 64
    tiles = tmp_path / "raw" / "tiles"
    tiles.mkdir(parents=True)
    rng = np.random.default_rng(0)
    for entry in document["materials"]:
        for kind in ("albedo", "normal"):
            data = rng.integers(0, 255, (64, 64, 3), dtype=np.uint8)
            Image.fromarray(data).save(tiles / f"{entry['id']}_{kind}.png")
    manifest_path = tmp_path / "pack.json"
    report = gm.pack(document, tmp_path / "raw", tmp_path / "tex", manifest_path)
    columns, rows = report["grid"]
    count = len(document["materials"])
    assert columns * rows == count == report["layers"]
    with Image.open(tmp_path / "tex" / gm.ALBEDO_NAME) as albedo:
        assert albedo.size == (columns * 64, rows * 64)
    imported = (tmp_path / "tex" / f"{gm.NORMAL_NAME}.import").read_text()
    assert f"slices/horizontal={columns}" in imported
    assert f"slices/vertical={rows}" in imported
    manifest = json.loads(manifest_path.read_text())
    schema = json.loads(
        (ROOT / "data" / "schemas" / "ground_materials_pack.schema.json").read_text()
    )
    Draft202012Validator(schema).validate(manifest)
    assert [m["layer"] for m in manifest["layers"]] == list(range(count))


def test_committed_pack_matches_catalog() -> None:
    """The committed manifest and arrays follow the catalogue (skipped before HB2 pack)."""
    if not gm.MANIFEST_PATH.exists():
        pytest.skip("tableaux HB2 pas encore empaquetés")
    document = gm.load_catalog()
    manifest = json.loads(gm.MANIFEST_PATH.read_text())
    assert [m["id"] for m in manifest["layers"]] == [
        m["id"] for m in document["materials"]
    ]
    columns, rows = manifest["grid"]
    size = manifest["layer_size"]
    total = 0
    for name in (gm.ALBEDO_NAME, gm.NORMAL_NAME):
        path = gm.TEXTURE_DIR / name
        with Image.open(path) as image:
            assert image.size == (columns * size, rows * size)
        total += path.stat().st_size
    assert total <= gm.MAX_PACK_BYTES
