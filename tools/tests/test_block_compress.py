"""BC5 relief shade (lot OMR-R2): Godot layout, BC4 error bound, parts."""

import json
import zlib

import numpy as np

from cent_ans_tools.geo import block_compress as bc5


def test_chain_bytes_matches_godot_rgtc_rg_layout():
    """Chain size equals Godot's RGTC_RG image size."""
    # Image.create_empty(14336, 12288, true, FORMAT_RGTC_RG).get_data_size() (Godot 4.7).
    assert bc5.chain_bytes(14336, 12288) == 234881088
    assert (
        len(bc5.mip_chain(np.zeros((12288 // 64, 14336 // 64, 2), np.uint8))) == 14 - 6
    )


def test_mip_chain_sizes_and_box_filter():
    """Levels halve down to 1 x 1 with a rounded 2 x 2 mean."""
    image = np.arange(6 * 7 * 2, dtype=np.uint8).reshape(6, 7, 2)
    sizes = [level.shape[:2] for level in bc5.mip_chain(image)]
    assert sizes == [(6, 7), (3, 3), (1, 1)]
    level1 = bc5.mip_chain(image)[1]
    block = image[0:2, 0:2, 0].astype(int)
    assert level1[0, 0, 0] == (block.sum() + 2) >> 2


def test_bc4_error_is_bounded_by_block_range():
    """Decoded texels stay within range / 14 of the source."""
    rng = np.random.default_rng(3)
    smooth = np.cumsum(np.cumsum(rng.normal(0, 1, (64, 64)), axis=0), axis=1)
    channel = np.clip(128 + smooth, 0, 255).astype(np.uint8)
    decoded = bc5.decode_bc4(bc5.encode_bc4(channel), 64, 64)
    blocks = channel.reshape(16, 4, 16, 4).transpose(0, 2, 1, 3).reshape(16, 16, 16)
    span = (blocks.max(-1) - blocks.min(-1).astype(int)).repeat(4, 0).repeat(4, 1)
    assert np.all(np.abs(decoded - channel) <= span / 14.0 + 1e-9)
    flat = np.full((8, 8), 77, np.uint8)
    assert np.array_equal(bc5.decode_bc4(bc5.encode_bc4(flat), 8, 8), flat)


def test_encode_pads_partial_blocks():
    """Partial blocks are padded to whole 4 x 4 blocks."""
    level = np.full((6, 7, 2), 200, np.uint8)
    assert len(bc5.encode_bc5(level)) == bc5.level_bytes(7, 6) == 2 * 2 * 16


def test_write_relief_parts_roundtrip(tmp_path, monkeypatch):
    """Parts decompress to the chain recorded in map.json."""
    monkeypatch.setattr(bc5, "PART_RAW_BYTES", 1000)
    (tmp_path / "map.json").write_text(
        json.dumps({"relief_shade": {"detail_scale_m": 1.5}})
    )
    image = np.random.default_rng(1).integers(0, 255, (40, 56, 2), dtype=np.uint8)
    result = bc5.write_relief_bc5(image, tmp_path)
    bc5.update_map_json(tmp_path, bc5.RELIEF_KEY, result.meta)
    meta = json.loads((tmp_path / "map.json").read_text())["relief_shade"]["bc5"]
    assert meta["size_px"] == [56, 40] and meta["format"] == "rgtc_rg"
    raw = b"".join(zlib.decompress(p.read_bytes()) for p in result.paths)
    assert len(raw) == meta["bytes"] == bc5.chain_bytes(56, 40)
    assert [len(zlib.decompress(p.read_bytes())) for p in result.paths] == meta[
        "part_bytes"
    ]
    assert len(set(meta["part_bytes"])) <= 2
    assert raw == bc5.encode_chain(image)


def test_bc1_single_channel_masks_and_black_blocks():
    """Sparse wetland masks: black blocks exact, one-channel ramps within a quantum."""
    image = np.zeros((16, 20, 3), np.uint8)
    ramp = np.linspace(0, 230, 8).astype(np.uint8)
    image[4:12, 4:12, 0] = ramp[None, :]
    image[8:16, 12:20, 2] = 191
    decoded = bc5.decode_bc1(bc5.encode_bc1(image), 16, 20)
    assert decoded.shape == (16, 20, 3)
    assert np.all(decoded[0:4] == 0)
    # Quatre niveaux par bloc sur une rampe de 4 texels (≈ 100 niveaux) + quantification 565.
    assert np.abs(decoded - image).max() <= 24
    assert np.abs(decoded - image).mean() < 2.0
    assert len(bc5.encode_bc1(image)) == 4 * 5 * 8


def test_write_wetlands_meta(tmp_path):
    """Wetlands parts and their ``wetlands_gpu`` entry."""
    (tmp_path / "map.json").write_text("{}")
    image = np.zeros((8, 8, 3), np.uint8)
    result = bc5.write_wetlands_bc1(image, tmp_path)
    bc5.update_map_json(tmp_path, bc5.WETLANDS_KEY, result.meta)
    meta = json.loads((tmp_path / "map.json").read_text())["wetlands_gpu"]
    assert (
        meta["format"] == "dxt1" and meta["bytes"] == 2 * 2 * 8 and not meta["mipmaps"]
    )
