"""GPU-ready relief shade: BC5 (RGTC RG) with its full mipmap chain (lot OMR-R2).

The campaign map's ``relief_shade`` raster (14336 x 12288 LA8, ADR 0115) was
decoded from PNG bands at every launch, stacked, mipmapped and uploaded as LA8:
470 MB of texture plus ≈ 1 GB of transient buffers. This module writes the same
two channels (R = L detail, G = A occlusion) block-compressed in BC5, one byte
per pixel, with every mip level precomputed in Godot's layout, so that the game
only reads bytes and hands them to ``Image.create_from_data`` (235 MB, no
decoding, no mipmap pass).

The byte stream is split into zlib-compressed parts (``relief_shade_bc5_<i>.bin``,
each well below GitHub's 50 MB warning), recorded under
``map.json.relief_shade.bc5``. The PNG bands stay the master copy (and the
fallback of ``ReliefLandcover``).

Encoding: per 4 x 4 block and channel, BC4 with endpoints at the block's max and
min (eight-level mode), each pixel to the nearest level: the error is at most
range / 14 (below one level in the smooth blocks, which are almost all of them).
Mip levels follow Godot (``w >> 1`` down to 1 x 1, 2 x 2 box ``(a+b+c+d+2) >> 2``
like ``Image.generate_mipmaps``) and each level is padded to whole blocks.
"""

from __future__ import annotations

import json
import zlib
from dataclasses import dataclass
from pathlib import Path

import numpy as np

BC5_STEM = "relief_shade_bc5"
#: Largest raw part before zlib (parts are equal; ≈ 15-20 MB per file once compressed).
PART_RAW_BYTES = 32 * 1024 * 1024
#: Rows encoded at once (bounds the numpy temporaries on the 14336-wide raster).
ENCODE_ROWS = 512
ZLIB_LEVEL = 9


@dataclass(frozen=True)
class Bc5Result:
    """Written parts and their metadata (``map.json.relief_shade.bc5``)."""

    paths: list[Path]
    meta: dict


def mip_chain(image: np.ndarray) -> list[np.ndarray]:
    """Levels of ``image`` (H x W x C uint8) down to 1 x 1, as Godot sizes them."""
    levels = [image]
    while levels[-1].shape[0] > 1 or levels[-1].shape[1] > 1:
        levels.append(_downsample(levels[-1]))
    return levels


def _downsample(level: np.ndarray) -> np.ndarray:
    height, width = level.shape[:2]
    out_h, out_w = max(1, height >> 1), max(1, width >> 1)
    # Axe de taille 1 : dupliqué (moyenne de deux fois le même texel).
    src = level
    if height == 1:
        src = np.concatenate([src, src], axis=0)
    if width == 1:
        src = np.concatenate([src, src], axis=1)
    src = src[: out_h * 2, : out_w * 2].astype(np.uint16)
    total = src[0::2, 0::2] + src[1::2, 0::2] + src[0::2, 1::2] + src[1::2, 1::2]
    return ((total + 2) >> 2).astype(np.uint8)


def level_bytes(width: int, height: int) -> int:
    """BC5 bytes of one level (padded to 4 x 4 blocks, 16 bytes per block)."""
    return ((width + 3) // 4) * ((height + 3) // 4) * 16


def chain_bytes(width: int, height: int) -> int:
    """BC5 bytes of the whole mip chain of a ``width`` x ``height`` image."""
    total = 0
    while True:
        total += level_bytes(width, height)
        if width == 1 and height == 1:
            return total
        width, height = max(1, width >> 1), max(1, height >> 1)


def encode_bc4(channel: np.ndarray) -> np.ndarray:
    """BC4 blocks (``(H/4) x (W/4) x 8`` uint8) of a channel padded to whole blocks."""
    height, width = channel.shape
    pad_h, pad_w = (-height) % 4, (-width) % 4
    if pad_h or pad_w:
        channel = np.pad(channel, ((0, pad_h), (0, pad_w)), mode="edge")
    rows, cols = channel.shape[0] // 4, channel.shape[1] // 4
    blocks = (
        channel.reshape(rows, 4, cols, 4).transpose(0, 2, 1, 3).reshape(rows, cols, 16)
    )
    hi = blocks.max(axis=-1)
    lo = blocks.min(axis=-1)
    span = (hi.astype(np.int32) - lo)[..., None]
    # Position sur le segment r0 = max (0) → r1 = min (7) ; 0 si le bloc est plat.
    offset = hi[..., None].astype(np.int32) - blocks
    position = np.where(span > 0, (offset * 14 + span) // np.maximum(span * 2, 1), 0)
    position = np.clip(position, 0, 7).astype(np.uint64)
    # Codes BC4 (r0 > r1) : 0 = r0, 1 = r1, 2..7 = (6 r0 + r1) / 7 … (r0 + 6 r1) / 7.
    codes = np.where(position == 0, 0, np.where(position == 7, 1, position + 1)).astype(
        np.uint64
    )
    flat = span[..., 0] == 0
    codes[flat] = 0
    shifts = (np.arange(16, dtype=np.uint64) * 3)[None, None, :]
    bits = np.bitwise_or.reduce(codes << shifts, axis=-1)
    out = np.empty((rows, cols, 8), dtype=np.uint8)
    out[..., 0] = hi
    out[..., 1] = lo
    for k in range(6):
        out[..., 2 + k] = ((bits >> np.uint64(8 * k)) & np.uint64(0xFF)).astype(
            np.uint8
        )
    return out


def encode_bc5(level: np.ndarray) -> bytes:
    """BC5 bytes of an H x W x 2 uint8 level (R block then G block, blocks row-major)."""
    height = level.shape[0]
    chunks = []
    for top in range(0, height, ENCODE_ROWS):
        part = level[top : top + ENCODE_ROWS]
        red = encode_bc4(part[..., 0])
        green = encode_bc4(part[..., 1])
        chunks.append(np.concatenate([red, green], axis=-1).tobytes())
    return b"".join(chunks)


def decode_bc4(blocks: np.ndarray, height: int, width: int) -> np.ndarray:
    """Inverse of :func:`encode_bc4` (eight-level mode and flat blocks), for tests."""
    r0 = blocks[..., 0].astype(np.float64)
    r1 = blocks[..., 1].astype(np.float64)
    bits = np.zeros(blocks.shape[:2], dtype=np.uint64)
    for k in range(6):
        bits |= blocks[..., 2 + k].astype(np.uint64) << np.uint64(8 * k)
    codes = np.stack(
        [(bits >> np.uint64(3 * i)) & np.uint64(7) for i in range(16)], axis=-1
    )
    codes = codes.astype(np.int64)
    weight = np.where(codes == 0, 0, np.where(codes == 1, 7, codes - 1))
    values = (r0[..., None] * (7 - weight) + r1[..., None] * weight) / 7.0
    rows, cols = blocks.shape[:2]
    image = (
        values.reshape(rows, cols, 4, 4)
        .transpose(0, 2, 1, 3)
        .reshape(rows * 4, cols * 4)
    )
    return image[:height, :width]


def encode_chain(image: np.ndarray) -> bytes:
    """Whole BC5 mip chain of an H x W x 2 image, in Godot's ``RGTC_RG`` layout."""
    return b"".join(encode_bc5(level) for level in mip_chain(image))


def write_parts(image: np.ndarray, map_dir: Path, stem: str = BC5_STEM) -> Bc5Result:
    """Encode ``image`` and write its zlib parts; returns the ``bc5`` metadata."""
    height, width = image.shape[:2]
    data = encode_chain(image)
    expected = chain_bytes(width, height)
    if len(data) != expected:
        raise ValueError(f"BC5 chain is {len(data)} bytes, expected {expected}")
    for old in map_dir.glob(stem + "_*.bin"):
        old.unlink()
    paths: list[Path] = []
    sizes: list[int] = []
    # Parts de taille égale (au plus PART_RAW_BYTES), pas de reliquat minuscule en fin.
    count = -(-len(data) // PART_RAW_BYTES)
    part = -(-len(data) // count)
    for index, start in enumerate(range(0, len(data), part)):
        raw = data[start : start + part]
        path = map_dir / f"{stem}_{index}.bin"
        path.write_bytes(zlib.compress(raw, ZLIB_LEVEL))
        paths.append(path)
        sizes.append(len(raw))
    meta = {
        "format": "rgtc_rg",
        "size_px": [width, height],
        "mipmaps": True,
        "bytes": len(data),
        "compression": "deflate",
        "pattern": stem + "_{part}.bin",
        "part_bytes": sizes,
    }
    return Bc5Result(paths=paths, meta=meta)


def update_map_json(map_dir: Path, meta: dict) -> None:
    """Record ``meta`` as ``map.json.relief_shade.bc5``."""
    path = map_dir / "map.json"
    metadata = json.loads(path.read_text(encoding="utf-8"))
    metadata.setdefault("relief_shade", {})["bc5"] = meta
    path.write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")


def read_png_bands(map_dir: Path) -> np.ndarray:
    """The LA8 shade raster stacked from ``map.json.relief_shade.bands``."""
    from PIL import Image

    metadata = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    bands = metadata["relief_shade"]["bands"]
    parts = []
    for index in range(int(bands["count"])):
        name = bands["pattern"].replace("{band}", str(index))
        with Image.open(map_dir / name) as band:
            parts.append(np.asarray(band.convert("LA")))
    return np.concatenate(parts, axis=0)


def build_from_bands(map_dir: Path) -> Bc5Result:
    """``geo relief-shade-bc5``: BC5 parts from the PNG bands already in ``map_dir``."""
    result = write_parts(read_png_bands(map_dir), map_dir)
    update_map_json(map_dir, result.meta)
    return result
